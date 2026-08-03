//! The multisig **constitution**: threshold, shielded member set, tiers.
//!
//! The constitution is the on-chain-visible artifact of a Quorum instance.
//! It deliberately leaks only the *shape* of governance (threshold, member
//! count, tier limits) — never the member identities, which live behind
//! `member_root`.

use crate::{error::Result, Commitment, QuorumError};

/// Hard cap on members (matches the public `PoC` and keeps circuits small).
pub const MAX_MEMBERS: u8 = 10;

/// Hard cap on spending tiers.
pub const MAX_TIERS: u8 = 8;

/// Zero commitment (unset sentinel).
pub const ZERO_COMMITMENT: Commitment = [0u8; 32];

/// A per-category spending policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpendingTier {
    /// Stable tier id referenced by proposals.
    pub id: u8,
    /// Committed category label (e.g. `H("ops")`). Never plaintext.
    pub label: Commitment,
    /// Approvals required for this tier (`1..=member_count`).
    pub threshold: u8,
    /// Per-operation amount cap.
    pub max_amount: u64,
}

/// The evolving governance rule-set of a Quorum instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constitution {
    /// Constitution version; incremented on every rotation / threshold change.
    pub version: u32,
    /// Default threshold M (used for governance actions).
    pub threshold: u8,
    /// Number of members N (coarse; reveals count only, never identities).
    pub member_count: u8,
    /// Merkle root over member commitments — the shielded member set.
    pub member_root: Commitment,
    /// Optional per-category policies.
    pub tiers: Vec<SpendingTier>,
}

impl SpendingTier {
    /// Validates a tier in isolation.
    ///
    /// # Errors
    /// - [`QuorumError::InvalidConstitution`] if the label is unset.
    /// - [`QuorumError::ThresholdOutOfRange`] if the threshold is 0 or exceeds `member_count`.
    pub fn validate(&self, member_count: u8) -> Result<()> {
        if self.label == ZERO_COMMITMENT {
            return Err(QuorumError::InvalidConstitution);
        }
        if self.threshold == 0 || self.threshold > member_count {
            return Err(QuorumError::ThresholdOutOfRange);
        }
        Ok(())
    }
}

impl Constitution {
    /// Creates a constitution and validates it.
    ///
    /// # Errors
    /// Returns any [`QuorumError`] from [`Constitution::validate`].
    pub fn new(
        threshold: u8,
        member_count: u8,
        member_root: Commitment,
        tiers: Vec<SpendingTier>,
    ) -> Result<Self> {
        let c = Self {
            version: 1,
            threshold,
            member_count,
            member_root,
            tiers,
        };
        c.validate()?;
        Ok(c)
    }

    /// Validates all constitutional invariants.
    ///
    /// # Errors
    /// - [`QuorumError::InvalidConstitution`] for malformed versions/tier lists.
    /// - [`QuorumError::ThresholdOutOfRange`] for invalid thresholds or member counts.
    /// - [`QuorumError::InvalidMemberRoot`] if the member root is unset.
    /// - [`QuorumError::DuplicateTierId`] for reused tier ids.
    pub fn validate(&self) -> Result<()> {
        if self.version == 0 {
            return Err(QuorumError::InvalidConstitution);
        }
        if self.threshold == 0 || self.threshold > MAX_MEMBERS {
            return Err(QuorumError::ThresholdOutOfRange);
        }
        if self.member_count < self.threshold || self.member_count > MAX_MEMBERS {
            return Err(QuorumError::ThresholdOutOfRange);
        }
        if self.member_root == ZERO_COMMITMENT {
            return Err(QuorumError::InvalidMemberRoot);
        }
        if self.tiers.len() > usize::from(MAX_TIERS) {
            return Err(QuorumError::InvalidConstitution);
        }
        let mut seen = [false; MAX_TIERS as usize];
        for tier in &self.tiers {
            tier.validate(self.member_count)?;
            let idx = usize::from(tier.id);
            if idx >= usize::from(MAX_TIERS) || seen[idx] {
                return Err(QuorumError::DuplicateTierId);
            }
            seen[idx] = true;
        }
        Ok(())
    }

    /// Returns a tier by id.
    ///
    /// # Errors
    /// [`QuorumError::TierNotFound`] if no tier has this id.
    pub fn tier(&self, id: u8) -> Result<&SpendingTier> {
        self.tiers
            .iter()
            .find(|t| t.id == id)
            .ok_or(QuorumError::TierNotFound)
    }

    /// Rotates the member set (shielded membership change).
    ///
    /// `new_root` is the Merkle root over the *new* member commitments.
    /// The old root is retired atomically: this returns a new constitution
    /// with `version + 1` — a removed member's proof no longer has a path in
    /// the new tree, so their key is provably dead.
    ///
    /// # Errors
    /// - [`QuorumError::RotationNoop`] if the root did not change.
    /// - [`QuorumError::RotationWouldBreakThreshold`] if the new set is
    ///   smaller than the threshold.
    /// - [`QuorumError::ThresholdOutOfRange`] if the new count exceeds `MAX_MEMBERS`.
    /// - Any [`QuorumError`] from [`Constitution::validate`] on the result.
    pub fn rotate(&self, new_root: Commitment, new_member_count: u8) -> Result<Self> {
        if new_root == self.member_root && new_member_count == self.member_count {
            return Err(QuorumError::RotationNoop);
        }
        if new_member_count < self.threshold {
            return Err(QuorumError::RotationWouldBreakThreshold);
        }
        if new_member_count > MAX_MEMBERS {
            return Err(QuorumError::ThresholdOutOfRange);
        }
        let mut next = self.clone();
        next.version = self.version.saturating_add(1);
        next.member_root = new_root;
        next.member_count = new_member_count;
        // Tier thresholds must still be satisfiable after the change.
        next.validate()?;
        Ok(next)
    }

    /// Returns the updated constitution after a threshold change.
    ///
    /// # Errors
    /// [`QuorumError::ThresholdOutOfRange`] if the new threshold is 0 or
    /// exceeds the member count.
    pub fn with_threshold(&self, new_threshold: u8) -> Result<Self> {
        if new_threshold == 0 || new_threshold > self.member_count {
            return Err(QuorumError::ThresholdOutOfRange);
        }
        let mut next = self.clone();
        next.version = self.version.saturating_add(1);
        next.threshold = new_threshold;
        Ok(next)
    }
}

/// A shorthand for the transfer proposal used in demos (2-of-3 ops tier).
#[doc(hidden)]
#[must_use]
pub fn demo_tier_ops() -> SpendingTier {
    let mut label = ZERO_COMMITMENT;
    label[0] = b'o';
    label[1] = b'p';
    label[2] = b's';
    SpendingTier {
        id: 1,
        label,
        threshold: 2,
        max_amount: 1_000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nullifier::member_commitment;
    use crate::DOMAIN_TAG;
    use sha2::Digest;

    fn root(secrets: &[[u8; 32]]) -> Commitment {
        // Chunk 2 wires the real Merkle tree; for core tests, a domain-separated
        // commitment over the sorted leaf commitments suffices.
        let mut leaves: Vec<Commitment> = secrets.iter().map(member_commitment).collect();
        leaves.sort_unstable();
        let mut h = sha2::Sha256::new();
        h.update(DOMAIN_TAG);
        h.update(b"/test-root");
        for l in &leaves {
            h.update(l);
        }
        h.finalize().into()
    }

    #[allow(clippy::cast_possible_truncation)] // test helper: n < 256 always
    fn secrets(n: usize) -> Vec<[u8; 32]> {
        (0..n).map(|i| [i as u8; 32]).collect()
    }

    #[test]
    fn valid_constitution() {
        let r = root(&secrets(3));
        let c = Constitution::new(2, 3, r, vec![]).unwrap();
        assert_eq!(c.version, 1);
        assert!(c.validate().is_ok());
    }

    #[test]
    fn rejects_threshold_beyond_members() {
        let r = root(&secrets(2));
        assert_eq!(
            Constitution::new(3, 2, r, vec![]),
            Err(QuorumError::ThresholdOutOfRange)
        );
    }

    #[test]
    fn rejects_zero_member_root() {
        assert_eq!(
            Constitution::new(2, 3, ZERO_COMMITMENT, vec![]),
            Err(QuorumError::InvalidMemberRoot)
        );
    }

    #[test]
    fn rotation_increments_version_and_retires_old_root() {
        let old = root(&secrets(3));
        let c = Constitution::new(2, 3, old, vec![]).unwrap();
        let new_root = root(&[secrets(4), vec![[42u8; 32]]].concat());
        let next = c.rotate(new_root, 4).unwrap();
        assert_eq!(next.version, 2);
        assert_ne!(next.member_root, c.member_root);
        // Old root no longer valid: a member of the old set has no path in new tree.
        assert_ne!(next.member_root, old);
    }

    #[test]
    fn rotation_noop_rejected() {
        let r = root(&secrets(3));
        let c = Constitution::new(2, 3, r, vec![]).unwrap();
        assert_eq!(c.rotate(r, 3), Err(QuorumError::RotationNoop));
    }

    #[test]
    fn rotation_cannot_break_threshold() {
        let r = root(&secrets(3));
        let c = Constitution::new(2, 3, r, vec![]).unwrap();
        assert_eq!(
            c.rotate(root(&secrets(1)), 1),
            Err(QuorumError::RotationWouldBreakThreshold)
        );
    }

    #[test]
    fn tier_invariants_enforced() {
        let r = root(&secrets(3));
        let c = Constitution::new(2, 3, r, vec![]).unwrap();
        let bad = SpendingTier {
            id: 1,
            label: ZERO_COMMITMENT,
            threshold: 2,
            max_amount: 100,
        };
        let with_bad = Constitution::new(2, 3, r, vec![bad]).unwrap_err();
        assert_eq!(with_bad, QuorumError::InvalidConstitution);
        let _ = c;
    }

    #[test]
    fn tier_lookup() {
        let r = root(&secrets(3));
        let c = Constitution::new(2, 3, r, vec![demo_tier_ops()]).unwrap();
        assert!(c.tier(1).is_ok());
        assert_eq!(c.tier(9), Err(QuorumError::TierNotFound));
    }
}
