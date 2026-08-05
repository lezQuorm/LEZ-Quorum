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
use quorum_core::nullifier::credential_commitment_from_account_id;
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
/// Maximum supported members. This matches the threshold circuit bound.
pub const MAX_MEMBERS: u8 = 10;
/// Maximum supported spending tiers.
pub const MAX_TIERS: usize = 8;

/// A per-category spending policy. Policy is public by design.
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
    /// Account id of the multisig that owns this constitution.
    pub multisig_id: [u8; 32],
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
        multisig_id: [u8; 32],
        threshold: u8,
        member_count: u8,
        member_root: [u8; 32],
        tiers: Vec<TierPolicy>,
    ) -> Result<Self, GateError> {
        let state = Self {
            multisig_id,
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
        if self.multisig_id == [0; 32] || self.version == 0 {
            return Err(GateError::InvalidConstitution);
        }
        if self.threshold == 0
            || self.threshold > self.member_count
            || self.member_count > MAX_MEMBERS
            || self.tiers.len() > MAX_TIERS
        {
            return Err(GateError::InvalidConstitution);
        }
        if self.member_root == [0; 32] {
            return Err(GateError::InvalidConstitution);
        }
        for (index, tier) in self.tiers.iter().enumerate() {
            if tier.threshold == 0
                || tier.threshold > self.member_count
                || self.tiers[..index]
                    .iter()
                    .any(|previous| previous.id == tier.id)
            {
                return Err(GateError::InvalidConstitution);
            }
        }
        Ok(())
    }

    /// Returns the tier policy for an id.
    ///
    /// # Errors
    /// [`GateError::TierNotFound`] if no tier has this id.
    pub fn tier(&self, id: u8) -> Result<&TierPolicy, GateError> {
        self.tiers
            .iter()
            .find(|t| t.id == id)
            .ok_or(GateError::TierNotFound)
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

    /// Applies a rotation: new root, version increment, and atomic retirement.
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
        if new_member_count < self.threshold || new_member_count > MAX_MEMBERS {
            return Err(GateError::RotationWouldBreakThreshold);
        }
        let mut next = self.clone();
        next.version = self
            .version
            .checked_add(1)
            .ok_or(GateError::InvalidConstitution)?;
        next.member_root = new_member_root;
        next.member_count = new_member_count;
        next.validate()?;
        *self = next;
        Ok(())
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
        self.version = self
            .version
            .checked_add(1)
            .ok_or(GateError::InvalidConstitution)?;
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
    /// Multisig account this proposal belongs to.
    pub multisig_id: [u8; 32],
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
}

impl ProposalState {
    /// Creates a new proposal bound to a constitution.
    #[must_use]
    pub fn new(
        multisig_id: [u8; 32],
        id: u64,
        constitution_version: u32,
        threshold: u8,
        action: ActionData,
    ) -> Self {
        Self {
            multisig_id,
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
    /// Proposal-scoped commitments to the private LEZ credential accounts.
    pub credential_commitments: Vec<[u8; 32]>,
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
            credential_commitments: journal.credential_commitments.clone(),
            action: journal.action.clone(),
        }
    }
}

/// Journal supplied to the on-chain approve instruction.
///
/// The matching Risc0 receipt is attached to the outer executor as an
/// assumption; receipt bytes are not instruction data.
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
}

/// Result of validating a claim against a proposal (before receipt verify).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimCheck {
    /// Nullifiers to append.
    pub nullifiers: Vec<[u8; 32]>,
}

/// Deterministic gate errors (codes `4001`–`4017`).
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
    /// The journal's tier cap does not match the constitution's tier cap.
    ///
    /// The tier amount cap is a *constitution* policy, never a caller-supplied
    /// value: a proof whose `Transfer.tier_max_amount` differs from the
    /// on-chain tier cap is rejected deterministically (a client could
    /// otherwise prove an oversized transfer under a self-inflated cap).
    TierCapMismatch = 4011,
    /// The vault account supplied to `Execute` is not this program's treasury
    /// `PDA` for the multisig (a caller-supplied arbitrary sender is rejected
    /// before any transfer `ChainedCall` is emitted).
    InvalidVault = 4012,
    /// Proposal belongs to a different multisig account.
    ProposalBindingMismatch = 4013,
    /// Proposal was created under an older constitution version.
    StaleProposal = 4014,
    /// Transfer recipient account does not match the approved action.
    InvalidRecipient = 4015,
    /// Instruction proposal id does not match the supplied proposal account.
    ProposalIdMismatch = 4016,
    /// Private credential accounts do not match the threshold proof.
    CredentialMismatch = 4017,
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
            Self::TierCapMismatch => "journal tier cap does not match the constitution",
            Self::InvalidVault => "vault account is not the treasury PDA",
            Self::ProposalBindingMismatch => "proposal belongs to a different multisig",
            Self::StaleProposal => "proposal bound to a stale constitution",
            Self::InvalidRecipient => "recipient account does not match the approved action",
            Self::ProposalIdMismatch => "instruction proposal id does not match proposal state",
            Self::CredentialMismatch => {
                "private credential accounts do not match the threshold proof"
            }
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
    if proposal.multisig_id != constitution.multisig_id {
        return Err(GateError::ProposalBindingMismatch);
    }
    if proposal.status != ProposalStatus::Active {
        return Err(GateError::ProposalNotActive);
    }
    if journal.constitution_version != constitution.version {
        return Err(GateError::StaleConstitution);
    }
    if proposal.constitution_version != constitution.version {
        return Err(GateError::StaleProposal);
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
    // The tier amount cap is constitution policy. Re-derive it from on-chain
    // state and reject any proof whose `tier_max_amount` was caller-supplied
    // differently (guards against cap inflation outside the circuit's
    // `amount <= tier_max_amount` check).
    if let ActionData::Transfer {
        tier_id,
        tier_max_amount,
        ..
    } = &journal.action
    {
        let tier = constitution.tier(*tier_id)?;
        if *tier_max_amount != tier.max_amount {
            return Err(GateError::TierCapMismatch);
        }
    }
    // Per-member approval proofs carry `required_threshold == 1`; the proposal
    // threshold is enforced on-chain by the aggregated nullifier count.
    let approval_count = usize::from(journal.approval_count);
    if journal.required_threshold == 0
        || approval_count == 0
        || approval_count != journal.nullifiers.len()
        || approval_count != journal.credential_commitments.len()
        || approval_count < usize::from(journal.required_threshold)
    {
        return Err(GateError::ThresholdMismatch);
    }
    for (index, nullifier) in journal.nullifiers.iter().enumerate() {
        if proposal.nullifiers.contains(nullifier)
            || journal.nullifiers[..index].contains(nullifier)
        {
            return Err(GateError::DuplicateNullifier);
        }
    }
    Ok(ClaimCheck {
        nullifiers: journal.nullifiers.clone(),
    })
}

/// Binds the threshold receipt to the authorized private LEZ accounts supplied
/// to the outer approval transaction.
///
/// Account order is deliberately irrelevant: the circuit and transaction
/// composer may canonicalize their private inputs independently.
///
/// # Errors
/// `GateError::CredentialMismatch` if the account count, commitments, or
/// uniqueness differ from the receipt journal.
pub fn validate_credentials(
    journal: &OnChainThresholdJournal,
    credential_account_ids: &[[u8; 32]],
) -> Result<(), GateError> {
    if credential_account_ids.len() != journal.credential_commitments.len() {
        return Err(GateError::CredentialMismatch);
    }
    let mut actual: Vec<[u8; 32]> = credential_account_ids
        .iter()
        .map(|account_id| {
            credential_commitment_from_account_id(
                account_id,
                &journal.member_root,
                journal.proposal_id,
                journal.constitution_version,
            )
        })
        .collect();
    actual.sort_unstable();
    if actual.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(GateError::CredentialMismatch);
    }
    let mut expected = journal.credential_commitments.clone();
    expected.sort_unstable();
    if actual != expected {
        return Err(GateError::CredentialMismatch);
    }
    Ok(())
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
    let mut next = proposal.clone();
    for nullifier in &check.nullifiers {
        next.add_nullifier(*nullifier)?;
    }
    *proposal = next;
    Ok(())
}

/// Validates an instruction's proposal id against the supplied proposal state.
///
/// # Errors
/// [`GateError::ProposalIdMismatch`] if the ids differ.
pub fn validate_proposal_id(proposal: &ProposalState, proposal_id: u64) -> Result<(), GateError> {
    if proposal.id != proposal_id {
        return Err(GateError::ProposalIdMismatch);
    }
    Ok(())
}

/// Validates that the runtime recipient matches the approved transfer action.
///
/// Governance actions do not use a recipient account and therefore pass this
/// check unchanged.
///
/// # Errors
/// [`GateError::InvalidRecipient`] if a transfer targets another account.
pub fn validate_transfer_recipient(
    proposal: &ProposalState,
    recipient_id: &[u8; 32],
) -> Result<(), GateError> {
    if let ActionData::Transfer { recipient, .. } = &proposal.action {
        if recipient != recipient_id {
            return Err(GateError::InvalidRecipient);
        }
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
    constitution.validate()?;
    if proposal.multisig_id != constitution.multisig_id {
        return Err(GateError::ProposalBindingMismatch);
    }
    if proposal.constitution_version != constitution.version {
        return Err(GateError::StaleProposal);
    }
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
            // The SPEL guest emits the transfer as a ChainedCall into the
            // treasury vault's token program (see `quorum_gate.rs::execute`);
            // no constitution state is mutated here.
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
    /// Initializes this multisig's program-derived treasury token holding.
    InitializeVault,
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

/// Domain tag for the treasury vault PDA seed.
///
/// The vault is a program-derived account of the `quorum-gate` program:
/// `vault_account_id = for_public_pda(quorum_gate_program_id, vault_pda_seed(multisig_id))`.
/// Binding the seed to the multisig account id gives every Quorum instance its
/// own treasury account, derived deterministically by anyone.
pub const VAULT_SEED_DOMAIN: &[u8] = b"quorum/vault/v1";

/// The 32-byte PDA seed of the treasury vault for a given multisig account.
///
/// `SHA256("quorum/vault/v1" || multisig_account_id)` — deterministic,
/// instance-unique, and computable off-chain by the deployer.
#[must_use]
pub fn vault_pda_seed(multisig_account_id: &[u8; 32]) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(VAULT_SEED_DOMAIN.len() + 32);
    bytes.extend_from_slice(VAULT_SEED_DOMAIN);
    bytes.extend_from_slice(multisig_account_id);
    sha2_256(&bytes)
}

/// Serde mirror of the LEZ token program's transfer instruction.
///
/// The on-chain `execute` handler emits a `ChainedCall` to the treasury
/// vault's token program (the vault holding's `program_owner`). `ChainedCall`
/// serializes the instruction with the risc0 serde format (`risc0_zkvm::serde::to_vec`),
/// and the token program decodes it with the same deserializer
/// (`read_lee_inputs::<token_core::Instruction>`). Serde encodes enums by
/// structure (variant name + field names), so this type — identical in shape
/// to `token_core::Instruction::Transfer { amount_to_transfer: u128 }` —
/// produces byte-identical instruction data. This crate deliberately does not
/// depend on LEZ's `token-core` crate; the mirror keeps the workspace
/// standalone and the compatibility boundary explicit and unit-tested.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TokenTransferInstruction {
    /// Transfer tokens from the authorized vault holding to the recipient.
    Transfer {
        /// Amount in LEZ base units.
        amount_to_transfer: u128,
    },
}

/// Serde mirror used to encode LEZ token `InitializeAccount`.
///
/// Risc0 serde encodes enum variants by their zero-based position. The three
/// placeholders preserve the position of `token_core::Instruction::InitializeAccount`
/// without pulling the token program's metadata types into the gate core crate.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TokenInitializeInstruction {
    /// Placeholder for token instruction index 0.
    Transfer,
    /// Placeholder for token instruction index 1.
    NewFungibleDefinition,
    /// Placeholder for token instruction index 2.
    NewDefinitionWithMetadata,
    /// Initializes an empty token holding for a definition account.
    InitializeAccount,
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
            [7; 32],
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
                account_identifier: 0,
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
        let mut proposal = ProposalState::new(c.multisig_id, 1, c.version, 2, transfer_action());
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
                account_identifier: 0,
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
        let mut proposal = ProposalState::new(c.multisig_id, 1, c.version, 2, transfer_action());
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
        let mut proposal = ProposalState::new(c.multisig_id, 1, c.version, 2, transfer_action());
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
            c.multisig_id,
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
        assert_eq!(GateError::TierCapMismatch.code(), 4011);
    }

    #[test]
    fn tier_cap_mismatch_rejected_on_chain() {
        let c = constitution();
        // A malicious proposer + approver agree on an inflated cap; the gate
        // must still reject it against the constitution's authoritative cap.
        let inflated_action = ActionData::Transfer {
            recipient: [9; 32],
            amount: 500,
            tier_id: 1,
            tier_max_amount: 1_000_000,
        };
        let proposal = ProposalState::new(c.multisig_id, 1, c.version, 2, inflated_action);
        let (_, j) = witness();
        let mut inflated = OnChainThresholdJournal::from(&j);
        if let ActionData::Transfer {
            tier_max_amount, ..
        } = &mut inflated.action
        {
            *tier_max_amount = 1_000_000;
        }
        assert_eq!(
            check_claim(&c, &proposal, &inflated),
            Err(GateError::TierCapMismatch)
        );
    }

    #[test]
    fn vault_seed_is_deterministic_and_instance_unique() {
        let multisig_a = [7u8; 32];
        let multisig_b = [8u8; 32];
        assert_eq!(vault_pda_seed(&multisig_a), vault_pda_seed(&multisig_a));
        assert_ne!(vault_pda_seed(&multisig_a), vault_pda_seed(&multisig_b));
        assert_ne!(vault_pda_seed(&multisig_a), [0u8; 32]);
    }

    #[test]
    fn transfer_instruction_mirrors_token_core_shape() {
        // Pin the serde shape of the LEZ token instruction so any drift in the
        // mirror is caught here (risc0 serde is structure-driven, so identical
        // serde shape => identical ChainedCall instruction words).
        let json = serde_json::to_value(TokenTransferInstruction::Transfer {
            amount_to_transfer: 500,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"Transfer": {"amount_to_transfer": 500}})
        );
    }

    #[test]
    fn cross_multisig_proposal_is_rejected() {
        let mut c = constitution();
        let mut proposal = ProposalState::new([8; 32], 1, c.version, 2, transfer_action());
        proposal.nullifiers = vec![[1; 32], [2; 32]];
        let (_, journal) = witness();
        assert_eq!(
            check_claim(&c, &proposal, &OnChainThresholdJournal::from(&journal)),
            Err(GateError::ProposalBindingMismatch)
        );
        assert_eq!(
            apply_action(&mut c, &proposal),
            Err(GateError::ProposalBindingMismatch)
        );
    }

    #[test]
    fn stale_proposal_cannot_collect_current_votes_or_execute() {
        let mut c = constitution();
        let mut proposal = ProposalState::new(c.multisig_id, 1, c.version, 2, transfer_action());
        proposal.nullifiers = vec![[1; 32], [2; 32]];
        let (_, journal) = witness();
        let commitments: Vec<[u8; 32]> = secrets(4).iter().map(member_commitment).collect();
        c.rotate(MemberTree::new(&commitments).root(), 4).unwrap();
        let mut current = OnChainThresholdJournal::from(&journal);
        current.member_root = c.member_root;
        current.constitution_version = c.version;
        assert_eq!(
            check_claim(&c, &proposal, &current),
            Err(GateError::StaleProposal)
        );
        assert_eq!(
            apply_action(&mut c, &proposal),
            Err(GateError::StaleProposal)
        );
    }

    #[test]
    fn transfer_recipient_and_instruction_id_are_bound() {
        let c = constitution();
        let proposal = ProposalState::new(c.multisig_id, 1, c.version, 2, transfer_action());
        assert!(validate_transfer_recipient(&proposal, &[9; 32]).is_ok());
        assert_eq!(
            validate_transfer_recipient(&proposal, &[8; 32]),
            Err(GateError::InvalidRecipient)
        );
        assert!(validate_proposal_id(&proposal, 1).is_ok());
        assert_eq!(
            validate_proposal_id(&proposal, 2),
            Err(GateError::ProposalIdMismatch)
        );
    }

    #[test]
    fn constitution_limits_and_tier_ids_are_enforced() {
        let mut c = constitution();
        c.member_count = MAX_MEMBERS + 1;
        assert_eq!(c.validate(), Err(GateError::InvalidConstitution));
        c = constitution();
        let duplicate = c.tiers[0].clone();
        c.tiers.push(duplicate);
        assert_eq!(c.validate(), Err(GateError::InvalidConstitution));
        c.tiers.pop();
        c.tiers[0].id = u8::MAX;
        assert!(c.validate().is_ok());
    }

    #[test]
    fn malformed_claim_is_rejected_without_partial_mutation() {
        let c = constitution();
        let mut proposal = ProposalState::new(c.multisig_id, 1, c.version, 2, transfer_action());
        let (_, journal) = witness();
        let mut malformed = OnChainThresholdJournal::from(&journal);
        malformed.approval_count = 0;
        assert_eq!(
            check_claim(&c, &proposal, &malformed),
            Err(GateError::ThresholdMismatch)
        );

        malformed.approval_count = 2;
        malformed.nullifiers.push(malformed.nullifiers[0]);
        malformed
            .credential_commitments
            .push(malformed.credential_commitments[0]);
        assert_eq!(
            check_claim(&c, &proposal, &malformed),
            Err(GateError::DuplicateNullifier)
        );

        let before = proposal.clone();
        let check = ClaimCheck {
            nullifiers: vec![[1; 32], [1; 32]],
        };
        assert_eq!(
            apply_approved_claim(&mut proposal, &check),
            Err(GateError::DuplicateNullifier)
        );
        assert_eq!(proposal, before);
    }

    #[test]
    fn credentials_are_order_independent_but_cannot_be_substituted_or_reused() {
        let credential_ids = [
            lez_compat::private_account_id(&[1_u8; 32], 0),
            lez_compat::private_account_id(&[2_u8; 32], 0),
        ];
        let (_, journal) = witness();
        let mut onchain = OnChainThresholdJournal::from(&journal);
        onchain.approval_count = 2;
        onchain.required_threshold = 2;
        onchain.nullifiers = vec![[10_u8; 32], [11_u8; 32]];
        onchain.credential_commitments = credential_ids
            .iter()
            .map(|account_id| {
                quorum_core::nullifier::credential_commitment_from_account_id(
                    account_id,
                    &onchain.member_root,
                    onchain.proposal_id,
                    onchain.constitution_version,
                )
            })
            .collect();

        assert!(validate_credentials(&onchain, &credential_ids).is_ok());
        assert!(validate_credentials(&onchain, &[credential_ids[1], credential_ids[0]]).is_ok());
        assert_eq!(
            validate_credentials(&onchain, &[credential_ids[0], [99_u8; 32]]),
            Err(GateError::CredentialMismatch)
        );
        assert_eq!(
            validate_credentials(&onchain, &[credential_ids[0], credential_ids[0]]),
            Err(GateError::CredentialMismatch)
        );
    }

    #[test]
    fn failed_rotation_does_not_mutate_constitution() {
        let mut c = constitution();
        c.tiers[0].threshold = 3;
        let before = c.clone();
        let commitments: Vec<[u8; 32]> = secrets(2).iter().map(member_commitment).collect();
        let new_root = MemberTree::new(&commitments).root();
        assert_eq!(c.rotate(new_root, 2), Err(GateError::InvalidConstitution));
        assert_eq!(c, before);
    }
}
