//! # quorum-circuit — the Quorum threshold statement
//!
//! Pure, deterministic logic evaluated inside the Risc0 guest (and mirrored by
//! the host for tests). It proves:
//!
//! > *"M **distinct** members of the committed set `member_root` approved
//! > proposal `proposal_id` under constitution version `V`; the required
//! > threshold is met; and the proposed action respects its own policy
//! > (tier cap / non-noop rotation / valid threshold change)."*
//!
//! Member secrets are **witness inputs only** — they never appear in the
//! journal. The journal publishes the nullifiers (for on-chain double-vote
//! prevention) and the action summary (which the gate executes). This is the
//! exact `ProofGate` pattern: verify in the guest, gate on-chain.

use quorum_core::merkle::{leaf_hash, node_hash};
use quorum_core::nullifier::{derive_nullifier, member_commitment};
use serde::{Deserialize, Serialize};

/// 32-byte digest.
pub type Digest32 = [u8; 32];

/// Hard cap on approvals per proof (matches `MAX_MEMBERS`).
pub const MAX_APPROVALS: usize = 10;

/// The proposed action (public by design — LP-0002 hides identity/vote, not content).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionData {
    /// A treasury transfer under a spending tier.
    Transfer {
        /// Recipient account.
        recipient: Digest32,
        /// Amount (LEZ base units).
        amount: u64,
        /// Spending tier id.
        tier_id: u8,
        /// Tier per-operation cap — the guest enforces `amount <= cap`.
        tier_max_amount: u64,
    },
    /// Shielded member-set rotation.
    RotateMembers {
        /// New member root.
        new_member_root: Digest32,
        /// New member count.
        new_member_count: u8,
    },
    /// Constitution threshold change.
    ChangeThreshold {
        /// New default threshold (must be `>= 1`).
        new_threshold: u8,
    },
}

/// A single member's approval — the member's secret is private.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberApprovalWitness {
    /// The member's identity secret (private; derives the commitment + nullifier).
    pub member_secret: [u8; 32],
    /// Leaf position in the member Merkle tree.
    pub leaf_index: usize,
    /// Sibling hashes from leaf to root.
    pub siblings: Vec<Digest32>,
}

/// Everything the guest needs to evaluate one threshold proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdWitness {
    /// The constitution's current member root.
    pub member_root: Digest32,
    /// Approvals required for this action (tier threshold or default threshold).
    pub required_threshold: u8,
    /// The M approvals being aggregated into one proof.
    pub approvals: Vec<MemberApprovalWitness>,
    /// The action being gated.
    pub action: ActionData,
    /// Proposal id (binds nullifiers and prevents cross-proposal replay).
    pub proposal_id: u64,
    /// Constitution version (binds to a specific member set).
    pub constitution_version: u32,
}

/// Public outputs committed by the guest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThresholdJournal {
    /// Member root the approvals were checked against.
    pub member_root: Digest32,
    /// Proposal id.
    pub proposal_id: u64,
    /// Constitution version.
    pub constitution_version: u32,
    /// Required threshold (gate re-checks against constitution state).
    pub required_threshold: u8,
    /// Number of distinct approvals in the proof.
    pub approval_count: u8,
    /// Nullifiers of the approving members (on-chain double-vote prevention).
    pub nullifiers: Vec<Digest32>,
    /// The gated action.
    pub action: ActionData,
}

/// Deterministic circuit errors (codes `3001`–`3008`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CircuitError {
    /// `required_threshold` is zero.
    ZeroThreshold = 3001,
    /// More approvals than `MAX_APPROVALS`.
    TooManyApprovals = 3002,
    /// Fewer approvals than the required threshold.
    ThresholdNotMet = 3003,
    /// The same member approved twice (same secret → same nullifier).
    DuplicateNullifier = 3004,
    /// A member commitment is not in the supplied `member_root`.
    InvalidMembership = 3005,
    /// Transfer amount exceeds the tier cap.
    AmountExceedsCap = 3006,
    /// Rotation to the same root is a no-op.
    NoopRotation = 3007,
    /// Threshold change to zero (or beyond member count — gate checks count).
    InvalidThresholdChange = 3008,
}

impl CircuitError {
    /// Deterministic code.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }

    /// Description.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::ZeroThreshold => "required threshold is zero",
            Self::TooManyApprovals => "too many approvals in one proof",
            Self::ThresholdNotMet => "approval count below required threshold",
            Self::DuplicateNullifier => "same member approved twice",
            Self::InvalidMembership => "member commitment not in member root",
            Self::AmountExceedsCap => "transfer amount exceeds tier cap",
            Self::NoopRotation => "rotation to the same member root",
            Self::InvalidThresholdChange => "invalid threshold change",
        }
    }
}

impl core::fmt::Display for CircuitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{}] {}", self.code(), self.description())
    }
}

impl std::error::Error for CircuitError {}

/// Evaluates the threshold statement.
///
/// # Errors
/// - [`CircuitError::ZeroThreshold`] if `required_threshold == 0`.
/// - [`CircuitError::TooManyApprovals`] beyond `MAX_APPROVALS`.
/// - [`CircuitError::ThresholdNotMet`] if approvals `< required_threshold`.
/// - [`CircuitError::InvalidMembership`] for any approval not in `member_root`.
/// - [`CircuitError::DuplicateNullifier`] if a member approved twice.
/// - [`CircuitError::AmountExceedsCap`] / [`CircuitError::NoopRotation`] /
///   [`CircuitError::InvalidThresholdChange`] for invalid actions.
///
/// # Panics
/// Never in practice: `approval_count` is `MAX_APPROVALS`-capped and always
/// fits in a `u8`.
#[must_use = "the circuit result must be checked"]
pub fn evaluate(witness: &ThresholdWitness) -> Result<ThresholdJournal, CircuitError> {
    if witness.required_threshold == 0 {
        return Err(CircuitError::ZeroThreshold);
    }
    if witness.approvals.len() > MAX_APPROVALS {
        return Err(CircuitError::TooManyApprovals);
    }
    if witness.approvals.len() < usize::from(witness.required_threshold) {
        return Err(CircuitError::ThresholdNotMet);
    }

    let mut nullifiers = Vec::with_capacity(witness.approvals.len());
    for approval in &witness.approvals {
        let commitment = member_commitment(&approval.member_secret);
        let mut result = leaf_hash(&commitment);
        let mut level_index = approval.leaf_index;
        for sibling in &approval.siblings {
            result = if level_index & 1 == 0 {
                node_hash(&result, sibling)
            } else {
                node_hash(sibling, &result)
            };
            level_index >>= 1;
        }
        if result != witness.member_root {
            return Err(CircuitError::InvalidMembership);
        }
        nullifiers.push(derive_nullifier(
            &approval.member_secret,
            witness.proposal_id,
            witness.constitution_version,
        ));
    }

    let mut sorted = nullifiers.clone();
    sorted.sort_unstable();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(CircuitError::DuplicateNullifier);
    }

    match &witness.action {
        ActionData::Transfer {
            amount,
            tier_max_amount,
            ..
        } if *amount > *tier_max_amount => {
            return Err(CircuitError::AmountExceedsCap);
        }
        ActionData::RotateMembers {
            new_member_root, ..
        } if *new_member_root == witness.member_root => {
            return Err(CircuitError::NoopRotation);
        }
        ActionData::ChangeThreshold { new_threshold } if *new_threshold == 0 => {
            return Err(CircuitError::InvalidThresholdChange);
        }
        _ => {}
    }

    Ok(ThresholdJournal {
        member_root: witness.member_root,
        proposal_id: witness.proposal_id,
        constitution_version: witness.constitution_version,
        required_threshold: witness.required_threshold,
        approval_count: u8::try_from(witness.approvals.len())
            .expect("approval count fits in u8: capped by MAX_APPROVALS"),
        nullifiers,
        action: witness.action.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quorum_core::merkle::MemberTree;
    use quorum_core::nullifier::member_commitment;

    fn secrets(n: usize) -> Vec<[u8; 32]> {
        (0..n).map(|i| [i as u8; 32]).collect()
    }

    fn two_of_three_witness(amount: u64) -> ThresholdWitness {
        let secrets = secrets(3);
        let commitments: Vec<Digest32> = secrets.iter().map(member_commitment).collect();
        let tree = MemberTree::new(&commitments);
        let approval_for = |secret: [u8; 32]| {
            let commitment = member_commitment(&secret);
            let p = tree.proof_for(&commitment).expect("member proof");
            MemberApprovalWitness {
                member_secret: secret,
                leaf_index: p.leaf_index,
                siblings: p.siblings,
            }
        };
        ThresholdWitness {
            member_root: tree.root(),
            required_threshold: 2,
            approvals: vec![approval_for(secrets[0]), approval_for(secrets[1])],
            action: ActionData::Transfer {
                recipient: [9; 32],
                amount,
                tier_id: 1,
                tier_max_amount: 1_000,
            },
            proposal_id: 7,
            constitution_version: 1,
        }
    }

    #[test]
    fn accepts_valid_2_of_3_transfer() {
        let witness = two_of_three_witness(500);
        let journal = evaluate(&witness).unwrap();
        assert_eq!(journal.approval_count, 2);
        assert_eq!(journal.required_threshold, 2);
        assert_eq!(journal.nullifiers.len(), 2);
        assert_ne!(journal.nullifiers[0], journal.nullifiers[1]);
    }

    #[test]
    fn journal_does_not_expose_secrets() {
        let witness = two_of_three_witness(500);
        let journal = evaluate(&witness).unwrap();
        let json = serde_json::to_string(&journal).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let fields = value.as_object().unwrap();
        assert!(!fields.contains_key("approvals"));
        assert!(!fields.contains_key("member_secret"));
        // No full 64-char secret hex string may appear anywhere in the JSON.
        for approval in &witness.approvals {
            let full = approval
                .member_secret
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            assert!(!json.contains(&full));
        }
    }

    #[test]
    fn rejects_under_threshold() {
        let mut witness = two_of_three_witness(500);
        witness.approvals.pop();
        assert_eq!(
            evaluate(&witness).unwrap_err(),
            CircuitError::ThresholdNotMet
        );
    }

    #[test]
    fn rejects_duplicate_member() {
        let mut witness = two_of_three_witness(500);
        witness.approvals[1] = witness.approvals[0].clone();
        assert_eq!(
            evaluate(&witness).unwrap_err(),
            CircuitError::DuplicateNullifier
        );
    }

    #[test]
    fn rejects_wrong_membership_path() {
        let mut witness = two_of_three_witness(500);
        witness.member_root[0] ^= 1;
        assert_eq!(
            evaluate(&witness).unwrap_err(),
            CircuitError::InvalidMembership
        );
    }

    #[test]
    fn rejects_amount_above_cap() {
        assert_eq!(
            evaluate(&two_of_three_witness(1_001)).unwrap_err(),
            CircuitError::AmountExceedsCap
        );
    }

    #[test]
    fn rejects_noop_rotation() {
        let mut witness = two_of_three_witness(500);
        witness.action = ActionData::RotateMembers {
            new_member_root: witness.member_root,
            new_member_count: 3,
        };
        assert_eq!(evaluate(&witness).unwrap_err(), CircuitError::NoopRotation);
    }

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(CircuitError::DuplicateNullifier.code(), 3004);
        assert_eq!(CircuitError::InvalidMembership.code(), 3005);
        let s = CircuitError::ThresholdNotMet.to_string();
        assert!(s.contains("3003"));
    }

    #[test]
    fn secret_binding_changes_nullifier_across_proposals() {
        let mut a = two_of_three_witness(500);
        let mut b = two_of_three_witness(500);
        b.proposal_id = 8;
        let ja = evaluate(&a).unwrap();
        let jb = evaluate(&b).unwrap();
        assert_ne!(ja.nullifiers, jb.nullifiers);
        a.proposal_id = 8;
        assert_eq!(evaluate(&a).unwrap().nullifiers, jb.nullifiers);
    }
}
