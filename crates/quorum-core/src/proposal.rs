//! Proposal state machine.
//!
//! A proposal accumulates **nullifiers** (one per approving member) rather than
//! identities. The on-chain verifier only ever sees the nullifier set; the ZK
//! threshold circuit proves the nullifiers are real, distinct, and bound to
//! members of the current member root.

use crate::{AccountId, Commitment, Constitution, Nullifier, QuorumError, Result};

/// What a proposal wants to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalKind {
    /// A treasury transfer under a spending tier.
    Transfer {
        /// Recipient of the shielded transfer.
        recipient: AccountId,
        /// Amount (LEZ base units).
        amount: u64,
        /// Spending tier governing threshold + cap.
        tier_id: u8,
    },
    /// Private member-set rotation.
    RotateMembers {
        /// New Merkle root over the new member commitments.
        new_member_root: Commitment,
        /// New member count.
        new_member_count: u8,
    },
    /// Constitution threshold change.
    ChangeThreshold {
        /// New default threshold.
        new_threshold: u8,
    },
}

/// Proposal lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalStatus {
    /// Collecting approvals.
    Active,
    /// Threshold met and action applied.
    Executed,
    /// Rejected (by veto or explicit rejection).
    Rejected,
    /// Cancelled before completion.
    Cancelled,
}

/// A single proposal, owned by no one — its state is fully public-safe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    /// Monotonic proposal id.
    pub id: u64,
    /// The proposed action.
    pub kind: ProposalKind,
    /// Constitution version this proposal is evaluated against.
    pub constitution_version: u32,
    /// Nullifiers of members who approved (never identities).
    pub approvals: Vec<Nullifier>,
    /// Lifecycle state.
    pub status: ProposalStatus,
    /// Block/epoch timestamp at creation.
    pub created_at: u64,
}

impl Proposal {
    /// Creates a new active proposal.
    #[must_use]
    pub fn new(id: u64, kind: ProposalKind, constitution_version: u32, created_at: u64) -> Self {
        Self {
            id,
            kind,
            constitution_version,
            approvals: Vec::new(),
            status: ProposalStatus::Active,
            created_at,
        }
    }

    /// The number of distinct approvals this proposal kind requires.
    ///
    /// Transfer proposals use the tier threshold; governance actions
    /// (rotation, threshold change) use the constitution's default threshold.
    ///
    /// # Errors
    /// [`QuorumError::TierNotFound`] for a transfer referencing an unknown tier.
    pub fn required_threshold(&self, constitution: &Constitution) -> Result<u8> {
        match &self.kind {
            ProposalKind::Transfer { tier_id, .. } => Ok(constitution.tier(*tier_id)?.threshold),
            ProposalKind::RotateMembers { .. } | ProposalKind::ChangeThreshold { .. } => {
                Ok(constitution.threshold)
            }
        }
    }

    /// Validates the proposal against a constitution *before* collecting votes.
    ///
    /// # Errors
    /// - [`QuorumError::InvalidMemberRoot`] if the constitution version differs.
    /// - [`QuorumError::TierNotFound`] / [`QuorumError::AmountExceedsTierCap`]
    ///   for invalid transfers.
    /// - [`QuorumError::RotationNoop`] / [`QuorumError::RotationWouldBreakThreshold`]
    ///   for invalid rotations.
    /// - [`QuorumError::ThresholdOutOfRange`] for invalid threshold changes.
    pub fn validate_against(&self, constitution: &Constitution) -> Result<()> {
        if self.constitution_version != constitution.version {
            return Err(QuorumError::InvalidMemberRoot);
        }
        match &self.kind {
            ProposalKind::Transfer {
                amount, tier_id, ..
            } => {
                let tier = constitution.tier(*tier_id)?;
                if *amount > tier.max_amount {
                    return Err(QuorumError::AmountExceedsTierCap);
                }
            }
            ProposalKind::RotateMembers {
                new_member_root,
                new_member_count,
            } => {
                let _ = constitution.rotate(*new_member_root, *new_member_count)?;
            }
            ProposalKind::ChangeThreshold { new_threshold } => {
                let _ = constitution.with_threshold(*new_threshold)?;
            }
        }
        Ok(())
    }

    /// Records an approval nullifier.
    ///
    /// # Errors
    /// - [`QuorumError::ProposalNotActive`] if the proposal is not active.
    /// - [`QuorumError::DuplicateNullifier`] on double-vote.
    pub fn add_approval(&mut self, nullifier: Nullifier) -> Result<()> {
        if self.status != ProposalStatus::Active {
            return Err(QuorumError::ProposalNotActive);
        }
        if self.approvals.contains(&nullifier) {
            return Err(QuorumError::DuplicateNullifier);
        }
        self.approvals.push(nullifier);
        Ok(())
    }

    /// Whether the current approval count meets the required threshold.
    ///
    /// # Errors
    /// [`QuorumError::ProposalNotActive`] if the proposal is not active.
    pub fn threshold_met(&self, constitution: &Constitution) -> Result<bool> {
        if self.status != ProposalStatus::Active {
            return Err(QuorumError::ProposalNotActive);
        }
        Ok(self.approvals.len() >= usize::from(self.required_threshold(constitution)?))
    }

    /// Marks the proposal rejected.
    ///
    /// # Errors
    /// [`QuorumError::ProposalNotActive`] if the proposal is not active.
    pub fn reject(&mut self) -> Result<()> {
        if self.status != ProposalStatus::Active {
            return Err(QuorumError::ProposalNotActive);
        }
        self.status = ProposalStatus::Rejected;
        Ok(())
    }

    /// Marks the proposal executed.
    ///
    /// # Errors
    /// [`QuorumError::ProposalNotActive`] if the proposal is not active.
    pub fn execute(&mut self) -> Result<()> {
        if self.status != ProposalStatus::Active {
            return Err(QuorumError::ProposalNotActive);
        }
        self.status = ProposalStatus::Executed;
        Ok(())
    }

    /// Cancels the proposal.
    ///
    /// # Errors
    /// [`QuorumError::ProposalNotActive`] if the proposal is not active.
    pub fn cancel(&mut self) -> Result<()> {
        if self.status != ProposalStatus::Active {
            return Err(QuorumError::ProposalNotActive);
        }
        self.status = ProposalStatus::Cancelled;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Constitution;

    fn root(secrets: &[[u8; 32]]) -> Commitment {
        crate::merkle::member_root(secrets)
    }

    fn two_of_three() -> Constitution {
        let r = root(&[[1u8; 32], [2u8; 32], [3u8; 32]]);
        let tier = crate::constitution::demo_tier_ops();
        Constitution::new(2, 3, r, vec![tier]).unwrap()
    }

    #[test]
    fn transfer_requires_tier_threshold_and_cap() {
        let c = two_of_three();
        let p = Proposal::new(
            1,
            ProposalKind::Transfer {
                recipient: [9u8; 32],
                amount: 500,
                tier_id: 1,
            },
            c.version,
            0,
        );
        assert!(p.validate_against(&c).is_ok());
        assert_eq!(p.required_threshold(&c).unwrap(), 2);

        let over = Proposal::new(
            2,
            ProposalKind::Transfer {
                recipient: [9u8; 32],
                amount: 5_000,
                tier_id: 1,
            },
            c.version,
            0,
        );
        assert_eq!(
            over.validate_against(&c),
            Err(QuorumError::AmountExceedsTierCap)
        );
    }

    #[test]
    fn double_vote_rejected() {
        let c = two_of_three();
        let mut p = Proposal::new(
            1,
            ProposalKind::Transfer {
                recipient: [9u8; 32],
                amount: 100,
                tier_id: 1,
            },
            c.version,
            0,
        );
        let n = crate::derive_nullifier(&[1u8; 32], p.id, c.version);
        p.add_approval(n).unwrap();
        assert_eq!(p.add_approval(n), Err(QuorumError::DuplicateNullifier));
    }

    #[test]
    fn threshold_met_after_enough_approvals() {
        let c = two_of_three();
        let mut p = Proposal::new(
            1,
            ProposalKind::Transfer {
                recipient: [9u8; 32],
                amount: 100,
                tier_id: 1,
            },
            c.version,
            0,
        );
        for secret in [[1u8; 32], [2u8; 32]] {
            p.add_approval(crate::derive_nullifier(&secret, p.id, c.version))
                .unwrap();
        }
        assert!(p.threshold_met(&c).unwrap());
    }

    #[test]
    fn lifecycle_guards() {
        let c = two_of_three();
        let mut p = Proposal::new(
            1,
            ProposalKind::ChangeThreshold { new_threshold: 3 },
            c.version,
            0,
        );
        p.execute().unwrap();
        assert_eq!(p.status, ProposalStatus::Executed);
        assert_eq!(
            p.add_approval(crate::derive_nullifier(&[1u8; 32], 1, 1)),
            Err(QuorumError::ProposalNotActive)
        );
    }

    #[test]
    fn rotation_proposal_validates() {
        let c = two_of_three();
        let new_root = root(&[[1u8; 32], [2u8; 32], [4u8; 32]]);
        let p = Proposal::new(
            1,
            ProposalKind::RotateMembers {
                new_member_root: new_root,
                new_member_count: 3,
            },
            c.version,
            0,
        );
        assert!(p.validate_against(&c).is_ok());
        let bad = Proposal::new(
            2,
            ProposalKind::RotateMembers {
                new_member_root: root(&[[1u8; 32]]),
                new_member_count: 1,
            },
            c.version,
            0,
        );
        assert_eq!(
            bad.validate_against(&c),
            Err(QuorumError::RotationWouldBreakThreshold)
        );
    }
}
