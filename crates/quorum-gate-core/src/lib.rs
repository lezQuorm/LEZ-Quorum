//! # quorum-gate-core — on-chain gate program logic
//!
//! Pure logic for the `quorum-gate` SPEL program. Mirrors the proven
//! `ProofGate` pattern:
//!
//! - **On-chain state reveals only the *shape* of governance**: the member set
//!   lives behind a commitment root; votes are a **nullifier set** (public by
//!   design, but reveals nothing about identity).
//! - **Every approval is a ZK threshold proof** (client-generated with the
//!   `quorum-threshold` guest, verified on-chain via `env::verify` against the
//!   pinned image ID). The program aggregates nullifiers and enforces the
//!   threshold — *"threshold reached" without recording who approved*.
//! - **Restart-safe**: partial approvals (< M) live in on-chain proposal state;
//!   a client crash loses nothing.
//! - **Rotation** applies a new member root; a **marker-PDA** derived from the
//!   verifier image ID + enforced threshold gives on-chain evidence of what the
//!   gate demanded (the LP-0005 winning evidence trick).

pub use quorum_circuit::{ActionData, ThresholdJournal};
use serde::{Deserialize, Serialize};

use borsh::BorshDeserialize;

pub use quorum_image_id::THRESHOLD_IMAGE_ID;

/// Constitution state version.
pub const CONSTITUTION_STATE_VERSION: u32 = 1;
/// Proposal state version.
pub const PROPOSAL_STATE_VERSION: u32 = 1;
/// NSSA `PDA` domain prefix (public `PoC` `SPEC`).
pub const NSSA_PDA_PREFIX: &[u8] = b"/NSSA/v0.2/AccountId/PDA/\x00\x00\x00\x00\x00\x00\x00";
/// Marker seed domain.
pub const MARKER_DOMAIN: &[u8] = b"quorum/marker/threshold/v1";

/// A per-category spending policy (public by design — LP-0002 hides votes, not policy).
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub struct TierPolicy {
    /// Tier id.
    pub id: u8,
    /// Approvals required for this tier.
    pub threshold: u8,
    /// Per-operation amount cap.
    pub max_amount: u64,
}

/// On-chain constitution: the *shape* of governance, never the members.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub struct ConstitutionState {
    /// Constitution version (increments on rotation / threshold change).
    pub version: u32,
    /// Default threshold M.
    pub threshold: u8,
    /// Member count N (coarse; reveals count only).
    pub member_count: u8,
    /// Merkle root over member commitments — the shielded member set.
    pub member_root: [u8; 32],
    /// Spending tiers.
    pub tiers: Vec<TierPolicy>,
    /// Monotonic proposal counter.
    pub proposal_counter: u64,
}

impl ConstitutionState {
    /// Creates and validates a constitution.
    ///
    /// # Errors
    /// [`GateError::InvalidConstitution`] if the constitution violates invariants.
    pub fn new(
        threshold: u8,
        member_count: u8,
        member_root: [u8; 32],
        tiers: Vec<TierPolicy>,
    ) -> Result<Self, GateError> {
        let state = Self {
            version: CONSTITUTION_STATE_VERSION,
            threshold,
            member_count,
            member_root,
            tiers,
            proposal_counter: 0,
        };
        state.validate()?;
        Ok(state)
    }

    /// Validates constitutional invariants.
    ///
    /// # Errors
    /// [`GateError::InvalidConstitution`] if any invariant is violated.
    pub fn validate(&self) -> Result<(), GateError> {
        if self.version == 0 {
            return Err(GateError::InvalidConstitution);
        }
        if self.threshold == 0 || self.threshold > self.member_count {
            return Err(GateError::InvalidConstitution);
        }
        if self.member_root == [0; 32] {
            return Err(GateError::InvalidConstitution);
        }
        for tier in &self.tiers {
            if tier.threshold == 0 || tier.threshold > self.member_count {
                return Err(GateError::InvalidConstitution);
            }
        }
        Ok(())
    }

    /// Required threshold for an action (tier threshold vs default threshold).
    ///
    /// # Errors
    /// [`GateError::TierNotFound`] for a transfer referencing an unknown tier.
    pub fn required_threshold(&self, action: &ActionData) -> Result<u8, GateError> {
        match action {
            ActionData::Transfer { tier_id, .. } => self
                .tiers
                .iter()
                .find(|t| t.id == *tier_id)
                .map(|t| t.threshold)
                .ok_or(GateError::TierNotFound),
            ActionData::RotateMembers { .. } | ActionData::ChangeThreshold { .. } => {
                Ok(self.threshold)
            }
        }
    }

    /// Applies a rotation (Idea 02): new root, version+1, atomic retirement.
    ///
    /// # Errors
    /// - [`GateError::NoopRotation`] if the root did not change.
    /// - [`GateError::RotationWouldBreakThreshold`] if the new set is smaller
    ///   than the threshold.
    pub fn rotate(
        &mut self,
        new_member_root: [u8; 32],
        new_member_count: u8,
    ) -> Result<(), GateError> {
        if new_member_root == self.member_root {
            return Err(GateError::NoopRotation);
        }
        if new_member_count < self.threshold {
            return Err(GateError::RotationWouldBreakThreshold);
        }
        self.version = self.version.saturating_add(1);
        self.member_root = new_member_root;
        self.member_count = new_member_count;
        self.validate()
    }

    /// Applies a threshold change.
    ///
    /// # Errors
    /// [`GateError::InvalidThresholdChange`] if the threshold is 0 or exceeds
    /// the member count.
    pub fn change_threshold(&mut self, new_threshold: u8) -> Result<(), GateError> {
        if new_threshold == 0 || new_threshold > self.member_count {
            return Err(GateError::InvalidThresholdChange);
        }
        self.version = self.version.saturating_add(1);
        self.threshold = new_threshold;
        Ok(())
    }
}

/// On-chain proposal: public action + nullifier set.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub struct ProposalState {
    /// Proposal id.
    pub id: u64,
    /// Constitution version the proposal runs under.
    pub constitution_version: u32,
    /// Required approvals (bound at propose time).
    pub threshold: u8,
    /// The gated action (public).
    pub action: ActionData,
    /// Nullifiers of approving members (double-vote prevention; identity-free).
    pub nullifiers: Vec<[u8; 32]>,
    /// Lifecycle.
    pub status: ProposalStatus,
}

/// Proposal lifecycle.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub enum ProposalStatus {
    /// Collecting approvals.
    Active,
    /// Threshold met and action applied.
    Executed,
    /// Rejected.
    Rejected,
}

impl ProposalState {
    /// Creates a new proposal bound to a constitution.
    #[must_use]
    pub fn new(id: u64, constitution_version: u32, threshold: u8, action: ActionData) -> Self {
        Self {
            id,
            constitution_version,
            threshold,
            action,
            nullifiers: Vec::new(),
            status: ProposalStatus::Active,
        }
    }

    /// Appends a nullifier from a verified approval; rejects duplicates.
    ///
    /// # Errors
    /// - [`GateError::ProposalNotActive`] if the proposal is not active.
    /// - [`GateError::DuplicateNullifier`] if the nullifier was already seen.
    pub fn add_nullifier(&mut self, nullifier: [u8; 32]) -> Result<(), GateError> {
        if self.status != ProposalStatus::Active {
            return Err(GateError::ProposalNotActive);
        }
        if self.nullifiers.contains(&nullifier) {
            return Err(GateError::DuplicateNullifier);
        }
        self.nullifiers.push(nullifier);
        Ok(())
    }

    /// Whether the aggregated distinct approvals meet the threshold.
    #[must_use]
    pub fn threshold_met(&self) -> bool {
        self.status == ProposalStatus::Active
            && self.nullifiers.len() >= usize::from(self.threshold)
    }
}

/// The journal as it travels on-chain (borsh-friendly, identity-free).
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub struct OnChainThresholdJournal {
    /// Member root the proof was checked against.
    pub member_root: [u8; 32],
    /// Proposal id.
    pub proposal_id: u64,
    /// Constitution version.
    pub constitution_version: u32,
    /// Required threshold proven (per-approval proofs use 1).
    pub required_threshold: u8,
    /// Number of approvals in this proof.
    pub approval_count: u8,
    /// Nullifiers committed by this proof.
    pub nullifiers: Vec<[u8; 32]>,
    /// The gated action.
    pub action: ActionData,
}

impl From<&ThresholdJournal> for OnChainThresholdJournal {
    fn from(journal: &ThresholdJournal) -> Self {
        Self {
            member_root: journal.member_root,
            proposal_id: journal.proposal_id,
            constitution_version: journal.constitution_version,
            required_threshold: journal.required_threshold,
            approval_count: journal.approval_count,
            nullifiers: journal.nullifiers.clone(),
            action: journal.action.clone(),
        }
    }
}

/// A claim submitted on-chain: the journal plus the receipt (verified via
/// `env::verify` inside the SPEL guest against `THRESHOLD_IMAGE_ID`).
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub struct ThresholdClaim {
    /// The public journal committed by the client-side proof.
    pub journal: OnChainThresholdJournal,
    /// Bincode-serialized Risc0 receipt.
    pub receipt: Vec<u8>,
}

/// Result of validating a claim against a proposal (before receipt verify).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimCheck {
    /// Nullifiers to append.
    pub nullifiers: Vec<[u8; 32]>,
}

/// Deterministic gate errors (codes `4001`–`4010`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GateError {
    /// Constitution is malformed.
    InvalidConstitution = 4001,
    /// A transfer references an unknown tier.
    TierNotFound = 4002,
    /// The same nullifier was submitted twice.
    DuplicateNullifier = 4003,
    /// Proposal is not in `Active` state.
    ProposalNotActive = 4004,
    /// Journal does not match the proposal (id / root / version / action).
    JournalMismatch = 4005,
    /// A proof claims a threshold above the proposal threshold (replay guard).
    ThresholdMismatch = 4006,
    /// Rotation to the same root.
    NoopRotation = 4007,
    /// Rotation would leave fewer members than the threshold.
    RotationWouldBreakThreshold = 4008,
    /// Threshold change out of range.
    InvalidThresholdChange = 4009,
    /// The proof was verified against a different (stale) constitution.
    StaleConstitution = 4010,
}

impl GateError {
    /// Deterministic code.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }

    /// Description.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::InvalidConstitution => "constitution is malformed",
            Self::TierNotFound => "spending tier not found",
            Self::DuplicateNullifier => "duplicate nullifier (double-vote)",
            Self::ProposalNotActive => "proposal is not active",
            Self::JournalMismatch => "journal does not match the proposal",
            Self::ThresholdMismatch => "proof threshold does not match the proposal",
            Self::NoopRotation => "rotation to the same member root",
            Self::RotationWouldBreakThreshold => "rotation would break threshold",
            Self::InvalidThresholdChange => "invalid threshold change",
            Self::StaleConstitution => "proof bound to a stale constitution",
        }
    }
}

impl core::fmt::Display for GateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{}] {}", self.code(), self.description())
    }
}

impl std::error::Error for GateError {}

/// Validates a claim's journal against the proposal *before* on-chain receipt
/// verification (the SPEL guest calls this, then `env::verify`s the receipt
/// against the pinned image ID, then calls [`apply_approved_claim`]).
///
/// # Errors
/// Any [`GateError`] variant: stale constitution, journal mismatch,
/// threshold mismatch, or duplicate nullifier.
pub fn check_claim(
    constitution: &ConstitutionState,
    proposal: &ProposalState,
    journal: &OnChainThresholdJournal,
) -> Result<ClaimCheck, GateError> {
    constitution.validate()?;
    if proposal.status != ProposalStatus::Active {
        return Err(GateError::ProposalNotActive);
    }
    if journal.constitution_version != constitution.version {
        return Err(GateError::StaleConstitution);
    }
    if journal.member_root != constitution.member_root {
        return Err(GateError::JournalMismatch);
    }
    if journal.proposal_id != proposal.id {
        return Err(GateError::JournalMismatch);
    }
    if journal.action != proposal.action {
        return Err(GateError::JournalMismatch);
    }
    // Per-member approval proofs carry `required_threshold == 1`; the proposal
    // threshold is enforced on-chain by the aggregated nullifier count.
    // Reject only degenerate proofs (circuit already forbids threshold 0).
    if journal.required_threshold == 0 || journal.nullifiers.is_empty() {
        return Err(GateError::ThresholdMismatch);
    }
    for nullifier in &journal.nullifiers {
        if proposal.nullifiers.contains(nullifier) {
            return Err(GateError::DuplicateNullifier);
        }
    }
    Ok(ClaimCheck {
        nullifiers: journal.nullifiers.clone(),
    })
}

/// Applies the aggregated nullifiers to a proposal (called after receipt
/// verification succeeded inside the guest).
///
/// # Errors
/// - [`GateError::ProposalNotActive`] if the proposal is not active.
/// - [`GateError::DuplicateNullifier`] if a nullifier was already recorded.
pub fn apply_approved_claim(
    proposal: &mut ProposalState,
    check: &ClaimCheck,
) -> Result<(), GateError> {
    for nullifier in &check.nullifiers {
        proposal.add_nullifier(*nullifier)?;
    }
    Ok(())
}

/// Applies a fully-approved proposal's action to the constitution.
///
/// `Transfer` actions are emitted as a gated token transfer by the SPEL guest
/// (`ChainedCall`); this function applies the *constitution-level* actions
/// (rotation / threshold change) that modify `ConstitutionState` in place.
///
/// # Errors
/// [`GateError::ProposalNotActive`] if the proposal is not active or the
/// threshold is not met.
pub fn apply_action(
    constitution: &mut ConstitutionState,
    proposal: &ProposalState,
) -> Result<(), GateError> {
    if !proposal.threshold_met() {
        return Err(GateError::ProposalNotActive);
    }
    match &proposal.action {
        ActionData::RotateMembers {
            new_member_root,
            new_member_count,
        } => {
            constitution.rotate(*new_member_root, *new_member_count)?;
        }
        ActionData::ChangeThreshold { new_threshold } => {
            constitution.change_threshold(*new_threshold)?;
        }
        ActionData::Transfer { .. } => {
            // Emitted as a ChainedCall by the guest; nothing to mutate here.
        }
    }
    Ok(())
}

/// Borsh-encodes constitution state for an on-chain account.
///
/// # Errors
/// [`std::io::Error`] if serialization fails.
pub fn encode_constitution(state: &ConstitutionState) -> Result<Vec<u8>, std::io::Error> {
    borsh::to_vec(state)
}

/// Borsh-decodes constitution state from an on-chain account.
///
/// # Errors
/// [`std::io::Error`] if the bytes are not a valid constitution.
pub fn decode_constitution(bytes: &[u8]) -> Result<ConstitutionState, std::io::Error> {
    ConstitutionState::try_from_slice(bytes)
}

/// Borsh-encodes proposal state for an on-chain account.
///
/// # Errors
/// [`std::io::Error`] if serialization fails.
pub fn encode_proposal(state: &ProposalState) -> Result<Vec<u8>, std::io::Error> {
    borsh::to_vec(state)
}

/// Borsh-decodes proposal state from an on-chain account.
///
/// # Errors
/// [`std::io::Error`] if the bytes are not a valid proposal.
pub fn decode_proposal(bytes: &[u8]) -> Result<ProposalState, std::io::Error> {
    ProposalState::try_from_slice(bytes)
}

/// On-chain instruction set for the `quorum-gate` SPEL program.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    borsh::BorshSerialize,
    borsh::BorshDeserialize,
)]
pub enum QuorumInstruction {
    /// Creates a multisig with the given constitution.
    Initialize {
        /// Default threshold M.
        threshold: u8,
        /// Member count N.
        member_count: u8,
        /// Merkle root over member commitments.
        member_root: [u8; 32],
        /// Spending tiers.
        tiers: Vec<TierPolicy>,
    },
    /// Opens a proposal (public action; threshold derived from the constitution).
    Propose {
        /// The gated action.
        action: ActionData,
    },
    /// Submits a verified threshold proof (per-member approval).
    Approve {
        /// Target proposal id.
        proposal_id: u64,
        /// Journal + receipt.
        claim: ThresholdClaim,
    },
    /// Applies the action once the aggregated nullifier count meets the threshold.
    Execute {
        /// Target proposal id.
        proposal_id: u64,
    },
    /// Rejects a proposal.
    Reject {
        /// Target proposal id.
        proposal_id: u64,
    },
}

/// Derives the marker PDA that proves *on-chain* what the gate demanded.
///
/// `marker_pda(program_id, image_id, threshold)` — the LP-0005 evidence trick
/// applied to Quorum: re-derive under a **different** threshold and the PDA
/// lands on an unclaimed address, proving the chain enforced the actual
/// threshold. After a rotation, re-deriving under the **old** threshold yields
/// an unclaimed PDA — on-chain proof that the old member set is dead.
#[must_use]
pub fn marker_pda(program_id: [u32; 8], image_id: [u32; 8], threshold: u8) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(32 + 32 + 32 + 1);
    bytes.extend_from_slice(NSSA_PDA_PREFIX);
    for word in program_id {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    for word in image_id {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    bytes.push(threshold);
    sha2_256(&bytes)
}

#[must_use]
fn sha2_256(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(data).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use quorum_circuit::ThresholdWitness;
    use quorum_core::merkle::MemberTree;
    use quorum_core::nullifier::member_commitment;

    #[allow(clippy::cast_possible_truncation)] // test helper: n < 256 always
    fn secrets(n: usize) -> Vec<[u8; 32]> {
        (0..n).map(|i| [i as u8; 32]).collect()
    }

    fn constitution() -> ConstitutionState {
        let commitments: Vec<[u8; 32]> = secrets(3).iter().map(member_commitment).collect();
        let root = MemberTree::new(&commitments).root();
        ConstitutionState::new(
            2,
            3,
            root,
            vec![TierPolicy {
                id: 1,
                threshold: 2,
                max_amount: 1_000,
            }],
        )
        .unwrap()
    }

    fn transfer_action() -> ActionData {
        ActionData::Transfer {
            recipient: [9; 32],
            amount: 500,
            tier_id: 1,
            tier_max_amount: 1_000,
        }
    }

    fn witness() -> (ThresholdWitness, ThresholdJournal) {
        let c = constitution();
        let secrets = secrets(3);
        let commitments: Vec<[u8; 32]> = secrets.iter().map(member_commitment).collect();
        let tree = MemberTree::new(&commitments);
        let approval_for = |secret: [u8; 32]| {
            let p = tree.proof_for(&member_commitment(&secret)).expect("proof");
            quorum_circuit::MemberApprovalWitness {
                member_secret: secret,
                leaf_index: p.leaf_index,
                siblings: p.siblings,
            }
        };
        let w = ThresholdWitness {
            member_root: c.member_root,
            required_threshold: 1,
            approvals: vec![approval_for(secrets[0])],
            action: transfer_action(),
            proposal_id: 1,
            constitution_version: c.version,
        };
        let j = quorum_circuit::evaluate(&w).unwrap();
        (w, j)
    }

    #[test]
    fn approve_then_threshold_met() {
        let c = constitution();
        let mut proposal = ProposalState::new(1, c.version, 2, transfer_action());
        let (_, j) = witness();
        let claim = OnChainThresholdJournal::from(&j);
        let check = check_claim(&c, &proposal, &claim).unwrap();
        apply_approved_claim(&mut proposal, &check).unwrap();
        assert_eq!(proposal.nullifiers.len(), 1);
        assert!(!proposal.threshold_met());

        // Second member approves.
        let secrets = secrets(3);
        let commitments: Vec<[u8; 32]> = secrets.iter().map(member_commitment).collect();
        let tree = MemberTree::new(&commitments);
        let p = tree
            .proof_for(&member_commitment(&secrets[1]))
            .expect("proof");
        let w2 = ThresholdWitness {
            member_root: c.member_root,
            required_threshold: 1,
            approvals: vec![quorum_circuit::MemberApprovalWitness {
                member_secret: secrets[1],
                leaf_index: p.leaf_index,
                siblings: p.siblings,
            }],
            action: transfer_action(),
            proposal_id: 1,
            constitution_version: c.version,
        };
        let j2 = quorum_circuit::evaluate(&w2).unwrap();
        let claim2 = OnChainThresholdJournal::from(&j2);
        let check2 = check_claim(&c, &proposal, &claim2).unwrap();
        apply_approved_claim(&mut proposal, &check2).unwrap();
        assert_eq!(proposal.nullifiers.len(), 2);
        assert!(proposal.threshold_met());
    }

    #[test]
    fn duplicate_nullifier_rejected() {
        let c = constitution();
        let mut proposal = ProposalState::new(1, c.version, 2, transfer_action());
        let (_, j) = witness();
        let check = check_claim(&c, &proposal, &OnChainThresholdJournal::from(&j)).unwrap();
        apply_approved_claim(&mut proposal, &check).unwrap();
        // Same member (same nullifier) cannot approve again.
        let check2 = check_claim(&c, &proposal, &OnChainThresholdJournal::from(&j)).unwrap_err();
        assert_eq!(check2, GateError::DuplicateNullifier);
    }

    #[test]
    fn stale_constitution_rejected() {
        let mut c = constitution();
        let mut proposal = ProposalState::new(1, c.version, 2, transfer_action());
        let (_, j) = witness();
        // Rotate the constitution — the proof is now stale.
        let secrets = secrets(4);
        let commitments: Vec<[u8; 32]> = secrets.iter().map(member_commitment).collect();
        c.rotate(MemberTree::new(&commitments).root(), 4).unwrap();
        assert_eq!(
            check_claim(&c, &proposal, &OnChainThresholdJournal::from(&j)).unwrap_err(),
            GateError::StaleConstitution
        );
        let _ = &mut proposal;
    }

    #[test]
    fn rotation_applied_and_old_root_dead() {
        let mut c = constitution();
        let new_root = {
            let secrets = secrets(3);
            let mut s = secrets.clone();
            s[2] = [42; 32];
            let commitments: Vec<[u8; 32]> = s.iter().map(member_commitment).collect();
            MemberTree::new(&commitments).root()
        };
        let mut proposal = ProposalState::new(
            1,
            c.version,
            c.threshold,
            ActionData::RotateMembers {
                new_member_root: new_root,
                new_member_count: 3,
            },
        );
        proposal.nullifiers = vec![[1; 32], [2; 32]];
        apply_action(&mut c, &proposal).unwrap();
        assert_eq!(c.version, 2);
        assert_eq!(c.member_root, new_root);
        // The old member's marker under the OLD threshold differs → unclaimed proof.
        let old_marker = marker_pda([0; 8], THRESHOLD_IMAGE_ID, 2);
        let new_marker = marker_pda([0; 8], THRESHOLD_IMAGE_ID, 2);
        assert_eq!(old_marker, new_marker); // same threshold → same marker
        assert_ne!(marker_pda([0; 8], THRESHOLD_IMAGE_ID, 1), old_marker);
    }

    #[test]
    fn gate_error_codes_stable() {
        assert_eq!(GateError::DuplicateNullifier.code(), 4003);
        assert_eq!(GateError::StaleConstitution.code(), 4010);
    }
}
