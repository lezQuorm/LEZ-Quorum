//! Client API for member sets, proposals, proofs, and rotations.
//!
//! Member secrets remain in client state and are not written to proof journals.

use quorum_circuit::{
    evaluate, ActionData, MemberApprovalWitness, ThresholdJournal, ThresholdWitness,
};
use quorum_core::merkle::MemberTree;
use quorum_core::nullifier::member_commitment_for_credential;
use quorum_gate_core::{
    apply_action, apply_approved_claim, check_claim, ConstitutionState, OnChainThresholdJournal,
    ProposalState, ProposalStatus, TierPolicy,
};
use quorum_prover::{dev_mode_status, prove, DevModeStatus, QuorumProof};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Errors produced by the SDK.
#[derive(Debug, Error)]
pub enum SdkError {
    /// Randomness source failed.
    #[error("randomness failed: {0}")]
    Rng(String),
    /// Constitution/proposal validation failed.
    #[error("gate: {0}")]
    Gate(#[from] quorum_gate_core::GateError),
    /// Proof generation/verification failed.
    #[error("prover: {0}")]
    Prover(#[from] quorum_prover::ProverError),
    /// Receipt encoding failed.
    #[error("receipt encode: {0}")]
    ReceiptEncode(String),
    /// Receipt decoding failed.
    #[error("receipt decode: {0}")]
    ReceiptDecode(String),
    /// The proposal does not exist.
    #[error("proposal {0} not found")]
    ProposalNotFound(u64),
    /// Member index out of range.
    #[error("member index {0} out of range")]
    MemberOutOfRange(usize),
    /// Member secret is not committed in the active member set.
    #[error("member is not in the active member set")]
    MemberNotInSet,
    /// An approval proof was bound to a different proposal.
    #[error("proof proposal mismatch")]
    ProofProposalMismatch,
}

/// Derives the valid ML-KEM viewing public key paired with a member nullifier secret.
///
/// Domain-separated seeds keep fixture and CLI credentials reproducible without
/// requiring ML-KEM key generation inside the threshold guest.
#[must_use]
pub fn viewing_public_key_for_secret(
    secret: &[u8; 32],
) -> [u8; quorum_core::VIEWING_PUBLIC_KEY_LEN] {
    fn seed(secret: &[u8; 32], label: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"quorum/v1/viewing-key/");
        hasher.update(label);
        hasher.update(secret);
        hasher.finalize().into()
    }

    let d = seed(secret, b"d");
    let z = seed(secret, b"z");
    let key = lee_core::encryption::ViewingPublicKey::from_seed(&d, &z);
    let mut bytes = [0_u8; quorum_core::VIEWING_PUBLIC_KEY_LEN];
    bytes.copy_from_slice(key.to_bytes());
    bytes
}

/// A shielded member: identity secret + derived commitment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    /// Canonical index in the member set.
    pub index: usize,
    /// Identity secret.
    pub secret: [u8; 32],
    /// LEZ regular-private-account identifier controlled by `secret`.
    #[serde(default)]
    pub account_identifier: u128,
}

impl Member {
    /// The member's identity commitment (safe to publish).
    #[must_use]
    pub fn commitment(&self) -> [u8; 32] {
        member_commitment_for_credential(
            &self.secret,
            &viewing_public_key_for_secret(&self.secret),
            self.account_identifier,
        )
    }

    /// LEZ v0.2.2 private account id controlled by this member.
    #[must_use]
    pub fn account_id(&self) -> [u8; 32] {
        lez_compat::private_account_id(
            &self.secret,
            &viewing_public_key_for_secret(&self.secret),
            self.account_identifier,
        )
    }
}

/// A set of members and the shielded Merkle root over their commitments.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberSet {
    /// Members, including private identity material.
    pub members: Vec<Member>,
    /// Merkle root over member commitments — the only public artifact.
    pub root: [u8; 32],
}

impl MemberSet {
    /// Creates a member set from explicit secrets (deterministic, for tests).
    #[must_use]
    pub fn from_secrets(secrets: &[[u8; 32]]) -> Self {
        let members: Vec<Member> = secrets
            .iter()
            .enumerate()
            .map(|(index, secret)| Member {
                index,
                secret: *secret,
                account_identifier: 0,
            })
            .collect();
        let commitments: Vec<[u8; 32]> = members.iter().map(Member::commitment).collect();
        Self {
            root: MemberTree::new(&commitments).root(),
            members,
        }
    }

    /// Generates a fresh random member set.
    ///
    /// # Errors
    /// [`SdkError::Rng`] if the OS RNG fails.
    #[must_use]
    pub fn generate(count: usize) -> Self {
        let mut secrets = Vec::with_capacity(count);
        for _ in 0..count {
            let mut secret = [0_u8; 32];
            rand::rngs::OsRng.fill_bytes(&mut secret);
            secrets.push(secret);
        }
        Self::from_secrets(&secrets)
    }

    /// Returns a member by index.
    ///
    /// # Errors
    /// [`SdkError::MemberOutOfRange`] for an out-of-range index.
    pub fn member(&self, index: usize) -> Result<&Member, SdkError> {
        self.members
            .get(index)
            .ok_or(SdkError::MemberOutOfRange(index))
    }

    /// Builds a membership-proven approval witness for `member`.
    ///
    /// # Errors
    /// [`SdkError::MemberNotInSet`] if `member` is not part of this set.
    pub fn approval_witness(
        &self,
        member: &Member,
        _proposal_id: u64,
        _constitution_version: u32,
    ) -> Result<MemberApprovalWitness, SdkError> {
        let proof = MemberTree::new(
            &self
                .members
                .iter()
                .map(Member::commitment)
                .collect::<Vec<_>>(),
        )
        .proof_for(&member.commitment())
        .ok_or(SdkError::MemberNotInSet)?;
        Ok(MemberApprovalWitness {
            member_secret: member.secret,
            viewing_public_key: viewing_public_key_for_secret(&member.secret),
            account_identifier: member.account_identifier,
            leaf_index: proof.leaf_index,
            siblings: proof.siblings,
        })
    }
}

/// Builds an approval witness from a member set's commitments and one member's
/// secret (the CLI stores only commitments + each member's own secret file).
///
/// # Errors
/// [`SdkError::MemberNotInSet`] if the secret is not committed in the set.
pub fn approval_witness_for(
    commitments: &[[u8; 32]],
    member_secret: &[u8; 32],
    account_identifier: u128,
    _proposal_id: u64,
    _constitution_version: u32,
) -> Result<MemberApprovalWitness, SdkError> {
    let tree = MemberTree::new(commitments);
    let viewing_public_key = viewing_public_key_for_secret(member_secret);
    let commitment =
        member_commitment_for_credential(member_secret, &viewing_public_key, account_identifier);
    let proof = tree
        .proof_for(&commitment)
        .ok_or(SdkError::MemberNotInSet)?;
    Ok(MemberApprovalWitness {
        member_secret: *member_secret,
        viewing_public_key,
        account_identifier,
        leaf_index: proof.leaf_index,
        siblings: proof.siblings,
    })
}

/// A local mirror of the on-chain multisig state (deterministic, serde-able).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Multisig {
    /// The constitution.
    pub constitution: ConstitutionState,
    /// Proposals (nullifier sets are the restart-safe source of truth).
    pub proposals: Vec<ProposalState>,
}

impl Multisig {
    /// Creates a multisig from a constitution.
    ///
    /// # Errors
    /// [`SdkError::Gate`] if the constitution is invalid.
    pub fn create(
        threshold: u8,
        member_set: &MemberSet,
        tiers: Vec<TierPolicy>,
    ) -> Result<Self, SdkError> {
        Self::create_with_account_id(member_set.root, threshold, member_set, tiers)
    }

    /// Creates a multisig mirror bound to a known on-chain account id.
    ///
    /// # Errors
    /// [`SdkError::Gate`] if the constitution is invalid.
    pub fn create_with_account_id(
        account_id: [u8; 32],
        threshold: u8,
        member_set: &MemberSet,
        tiers: Vec<TierPolicy>,
    ) -> Result<Self, SdkError> {
        Ok(Self {
            constitution: ConstitutionState::new(
                account_id,
                threshold,
                u8::try_from(member_set.members.len())
                    .map_err(|_| SdkError::Rng("member count overflow".into()))?,
                member_set.root,
                tiers,
            )?,
            proposals: Vec::new(),
        })
    }

    /// Opens a proposal; returns its id.
    ///
    /// The tier amount cap is *constitution policy*, never a caller-supplied
    /// value: for `Transfer` actions the caller-provided `tier_max_amount` is
    /// replaced with the authoritative cap from the constitution's tier state
    /// (mirroring the on-chain `check_claim` 4011 guard).
    ///
    /// # Errors
    /// [`SdkError::Gate`] if the action is invalid for this constitution.
    pub fn propose(&mut self, action: ActionData) -> Result<u64, SdkError> {
        let action = match action {
            ActionData::Transfer {
                recipient,
                amount,
                tier_id,
                ..
            } => ActionData::Transfer {
                recipient,
                amount,
                tier_id,
                tier_max_amount: self.constitution.tier(tier_id)?.max_amount,
            },
            other => other,
        };
        let threshold = self.constitution.required_threshold(&action)?;
        let id = self.constitution.proposal_counter;
        let proposal = ProposalState::new(
            self.constitution.multisig_id,
            id,
            self.constitution.version,
            threshold,
            action,
        );
        self.proposals.push(proposal);
        self.constitution.proposal_counter = self
            .constitution
            .proposal_counter
            .checked_add(1)
            .ok_or(quorum_gate_core::GateError::InvalidConstitution)?;
        Ok(id)
    }

    /// Aggregated approval: M distinct members in **one** threshold proof
    /// (`required_threshold = M`), producing a single on-chain claim.
    ///
    /// This uses the same guest and image ID while producing one
    /// receipt instead of M correlated receipts. `commitments` must be the
    /// member commitments the constitution was created with; `members` are
    /// the approving members (their secrets stay client-side, never in the
    /// journal). In real mode (`RISC0_DEV_MODE=0`) a succinct proof is
    /// produced; in dev mode a fast mock proof (tests/CI).
    ///
    /// # Errors
    /// Any [`SdkError`] variant, including a duplicate member in `members`
    /// (same secret -> same nullifier -> [`quorum_circuit::CircuitError::DuplicateNullifier`]).
    pub fn approve_many(
        &mut self,
        proposal_id: u64,
        commitments: &[[u8; 32]],
        members: &[&Member],
    ) -> Result<QuorumProof, SdkError> {
        if members.is_empty() {
            return Err(SdkError::Rng(
                "approve_many requires at least one member".into(),
            ));
        }
        let proposal_idx =
            usize::try_from(proposal_id).map_err(|_| SdkError::ProposalNotFound(proposal_id))?;
        let action = {
            let proposal = self
                .proposals
                .get(proposal_idx)
                .ok_or(SdkError::ProposalNotFound(proposal_id))?;
            if proposal.status != ProposalStatus::Active {
                return Err(SdkError::Gate(
                    quorum_gate_core::GateError::ProposalNotActive,
                ));
            }
            proposal.action.clone()
        };

        let approvals: Vec<MemberApprovalWitness> = members
            .iter()
            .map(|member| {
                approval_witness_for(
                    commitments,
                    &member.secret,
                    member.account_identifier,
                    proposal_id,
                    self.constitution.version,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let witness = ThresholdWitness {
            member_root: self.constitution.member_root,
            required_threshold: u8::try_from(approvals.len())
                .map_err(|_| SdkError::Rng("too many approvals in one proof".into()))?,
            approvals,
            action,
            proposal_id,
            constitution_version: self.constitution.version,
        };
        // Sanity: the witness must satisfy the statement before any proving.
        evaluate(&witness).map_err(quorum_prover::ProverError::InvalidWitness)?;

        let proof = prove_witness(&witness)?;
        if proof.journal.proposal_id != proposal_id {
            return Err(SdkError::ProofProposalMismatch);
        }

        let proposal = self
            .proposals
            .get_mut(proposal_idx)
            .ok_or(SdkError::ProposalNotFound(proposal_id))?;
        let onchain_journal = OnChainThresholdJournal::from(&proof.journal);
        let check = check_claim(&self.constitution, proposal, &onchain_journal)?;
        apply_approved_claim(proposal, &check)?;
        Ok(proof)
    }

    /// Generates a member's approval proof and applies it to the local mirror.
    ///
    /// `commitments` must be the member commitments the constitution was
    /// created with (safe to store alongside state; they are public). In real
    /// mode (`RISC0_DEV_MODE=0`) a succinct proof is produced; in dev mode a
    /// fast mock proof (tests/CI).
    ///
    /// # Errors
    /// Any [`SdkError`] variant.
    pub fn approve(
        &mut self,
        proposal_id: u64,
        commitments: &[[u8; 32]],
        member: &Member,
    ) -> Result<QuorumProof, SdkError> {
        let proposal = self
            .proposals
            .get_mut(
                usize::try_from(proposal_id)
                    .map_err(|_| SdkError::ProposalNotFound(proposal_id))?,
            )
            .ok_or(SdkError::ProposalNotFound(proposal_id))?;

        let witness = ThresholdWitness {
            member_root: self.constitution.member_root,
            required_threshold: 1,
            approvals: vec![approval_witness_for(
                commitments,
                &member.secret,
                member.account_identifier,
                proposal_id,
                self.constitution.version,
            )?],
            action: proposal.action.clone(),
            proposal_id,
            constitution_version: self.constitution.version,
        };
        // Sanity: the witness must satisfy the statement before any proving.
        evaluate(&witness).map_err(quorum_prover::ProverError::InvalidWitness)?;

        let proof = prove_witness(&witness)?;
        if proof.journal.proposal_id != proposal_id {
            return Err(SdkError::ProofProposalMismatch);
        }

        let onchain_journal = OnChainThresholdJournal::from(&proof.journal);
        let check = check_claim(&self.constitution, proposal, &onchain_journal)?;
        apply_approved_claim(proposal, &check)?;
        Ok(proof)
    }

    /// Applies a proposal's action once the threshold is met.
    ///
    /// # Errors
    /// [`SdkError::Gate`] if the proposal is not active or below threshold.
    pub fn execute(&mut self, proposal_id: u64) -> Result<(), SdkError> {
        let proposal = self
            .proposals
            .get_mut(
                usize::try_from(proposal_id)
                    .map_err(|_| SdkError::ProposalNotFound(proposal_id))?,
            )
            .ok_or(SdkError::ProposalNotFound(proposal_id))?;
        if proposal.status != ProposalStatus::Active {
            return Err(SdkError::Gate(
                quorum_gate_core::GateError::ProposalNotActive,
            ));
        }
        if !proposal.threshold_met() {
            return Err(SdkError::Gate(
                quorum_gate_core::GateError::ProposalNotActive,
            ));
        }
        apply_action(&mut self.constitution, proposal)?;
        proposal.status = ProposalStatus::Executed;
        Ok(())
    }
}

/// Proves a witness in the current mode (dev → fast mock; real → succinct).
fn prove_witness(witness: &ThresholdWitness) -> Result<QuorumProof, SdkError> {
    match dev_mode_status()? {
        DevModeStatus::Disabled => Ok(prove(witness)?),
        DevModeStatus::Enabled => {
            let expected = evaluate(witness).map_err(quorum_prover::ProverError::InvalidWitness)?;
            let env = risc0_zkvm::ExecutorEnv::builder()
                .write(witness)
                .map_err(|e| SdkError::ReceiptEncode(e.to_string()))?
                .build()
                .map_err(|e| SdkError::ReceiptEncode(e.to_string()))?;
            let info = risc0_zkvm::default_prover()
                .prove_with_opts(
                    env,
                    quorum_threshold_methods::THRESHOLD_ELF,
                    &risc0_zkvm::ProverOpts::succinct(),
                )
                .map_err(|e| SdkError::ReceiptEncode(e.to_string()))?;
            let journal = info
                .receipt
                .journal
                .decode::<ThresholdJournal>()
                .map_err(|e| SdkError::ReceiptDecode(e.to_string()))?;
            if journal != expected {
                return Err(SdkError::ReceiptDecode(
                    "journal mismatch in dev proof".into(),
                ));
            }
            let receipt = bincode::serialize(&info.receipt)
                .map_err(|e| SdkError::ReceiptEncode(e.to_string()))?;
            Ok(QuorumProof { journal, receipt })
        }
    }
}

#[cfg(test)]
#[allow(clippy::cast_possible_truncation)]
mod tests {
    use super::*;

    fn secrets(n: usize) -> Vec<[u8; 32]> {
        (0..n).map(|i| [i as u8; 32]).collect()
    }

    fn transfer() -> ActionData {
        ActionData::Transfer {
            recipient: [9; 32],
            amount: 500,
            tier_id: 1,
            tier_max_amount: 1_000,
        }
    }

    #[test]
    fn viewing_key_and_account_id_are_deterministic_and_bound() {
        let secret = [7_u8; 32];
        let other_secret = [8_u8; 32];
        let viewing_public_key = viewing_public_key_for_secret(&secret);
        assert_eq!(viewing_public_key, viewing_public_key_for_secret(&secret));
        assert_ne!(
            viewing_public_key,
            viewing_public_key_for_secret(&other_secret)
        );

        let member = Member {
            index: 0,
            secret,
            account_identifier: 9,
        };
        assert_eq!(
            member.account_id(),
            lez_compat::private_account_id(&secret, &viewing_public_key, 9)
        );
    }

    #[test]
    fn member_set_root_is_canonical_and_order_independent() {
        let a = MemberSet::from_secrets(&secrets(3));
        let b = MemberSet::from_secrets(&[secrets(3)[2], secrets(3)[0], secrets(3)[1]]);
        assert_eq!(a.root, b.root);
        assert_eq!(a.members.len(), 3);
    }

    #[test]
    fn approval_witness_proves_membership() {
        let set = MemberSet::from_secrets(&secrets(3));
        let member = set.member(1).unwrap();
        let w = set.approval_witness(member, 1, 1).unwrap();
        let witness = ThresholdWitness {
            member_root: set.root,
            required_threshold: 1,
            approvals: vec![w],
            action: transfer(),
            proposal_id: 1,
            constitution_version: 1,
        };
        assert!(evaluate(&witness).is_ok());
    }

    fn commitments_of(set: &MemberSet) -> Vec<[u8; 32]> {
        set.members.iter().map(Member::commitment).collect()
    }

    #[test]
    fn full_2_of_3_flow_with_dev_proofs() {
        let set = MemberSet::from_secrets(&secrets(3));
        let tiers = vec![TierPolicy {
            id: 1,
            threshold: 2,
            max_amount: 1_000,
        }];
        let mut multisig = Multisig::create(2, &set, tiers).unwrap();
        let commitments = commitments_of(&set);
        let id = multisig.propose(transfer()).unwrap();

        // First approval only.
        let proof1 = multisig
            .approve(id, &commitments, set.member(0).unwrap())
            .unwrap();
        assert_eq!(proof1.journal.approval_count, 1);
        assert!(multisig.proposals[id as usize].nullifiers.len() == 1);
        assert!(!multisig.proposals[id as usize].threshold_met());

        // Second approval reaches threshold.
        let proof2 = multisig
            .approve(id, &commitments, set.member(1).unwrap())
            .unwrap();
        assert_ne!(proof1.journal.nullifiers, proof2.journal.nullifiers);
        assert!(multisig.proposals[id as usize].threshold_met());

        multisig.execute(id).unwrap();
        assert_eq!(
            multisig.proposals[id as usize].status,
            ProposalStatus::Executed
        );
    }

    #[test]
    fn rotation_flow_updates_constitution() {
        let set = MemberSet::from_secrets(&secrets(3));
        let tiers = vec![TierPolicy {
            id: 1,
            threshold: 2,
            max_amount: 1_000,
        }];
        let mut multisig = Multisig::create(2, &set, tiers).unwrap();

        let newcomer = [42; 32];
        let new_commitments: Vec<[u8; 32]> = {
            let mut all = secrets(3);
            all[2] = newcomer;
            all.iter()
                .map(|secret| {
                    member_commitment_for_credential(
                        secret,
                        &viewing_public_key_for_secret(secret),
                        0,
                    )
                })
                .collect()
        };
        let new_root = MemberTree::new(&new_commitments).root();
        let action = ActionData::RotateMembers {
            new_member_root: new_root,
            new_member_count: 3,
        };
        let id = multisig.propose(action).unwrap();
        let commitments = commitments_of(&set);

        multisig
            .approve(id, &commitments, set.member(0).unwrap())
            .unwrap();
        multisig
            .approve(id, &commitments, set.member(1).unwrap())
            .unwrap();
        multisig.execute(id).unwrap();

        assert_eq!(multisig.constitution.version, 2);
        assert_eq!(multisig.constitution.member_root, new_root);
        // Old member (index 2) is gone: their proof fails against the new root.
        let old_set = MemberSet::from_secrets(&secrets(3));
        let old = old_set.member(2).unwrap();
        let w = MemberSet::from_secrets(&[old.secret])
            .approval_witness(old, 99, 2)
            .unwrap();
        let bad = ThresholdWitness {
            member_root: new_root,
            required_threshold: 1,
            approvals: vec![w],
            action: transfer(),
            proposal_id: 99,
            constitution_version: 2,
        };
        assert_eq!(
            evaluate(&bad).unwrap_err(),
            quorum_circuit::CircuitError::InvalidMembership
        );
    }

    #[test]
    fn propose_forces_constitution_tier_cap() {
        let set = MemberSet::from_secrets(&secrets(3));
        let mut multisig = Multisig::create(
            2,
            &set,
            vec![TierPolicy {
                id: 1,
                threshold: 2,
                max_amount: 1_000,
            }],
        )
        .unwrap();
        // Caller tries to inflate the cap — the SDK replaces it with the
        // constitution's authoritative cap.
        let id = multisig
            .propose(ActionData::Transfer {
                recipient: [9; 32],
                amount: 500,
                tier_id: 1,
                tier_max_amount: 999_999,
            })
            .unwrap();
        match &multisig.proposals[id as usize].action {
            ActionData::Transfer {
                tier_max_amount, ..
            } => assert_eq!(*tier_max_amount, 1_000),
            other => panic!("expected transfer action, got {other:?}"),
        }
    }

    #[test]
    fn aggregated_approval_reaches_threshold_in_one_proof() {
        let set = MemberSet::from_secrets(&secrets(3));
        let mut multisig = Multisig::create(
            2,
            &set,
            vec![TierPolicy {
                id: 1,
                threshold: 2,
                max_amount: 1_000,
            }],
        )
        .unwrap();
        let commitments = commitments_of(&set);
        let id = multisig.propose(transfer()).unwrap();

        let members: Vec<&Member> = vec![set.member(0).unwrap(), set.member(1).unwrap()];
        let proof = multisig.approve_many(id, &commitments, &members).unwrap();

        // ONE receipt proves BOTH approvals.
        assert_eq!(proof.journal.approval_count, 2);
        assert_eq!(proof.journal.required_threshold, 2);
        assert_eq!(proof.journal.nullifiers.len(), 2);
        assert!(multisig.proposals[id as usize].threshold_met());
        assert_eq!(multisig.proposals[id as usize].nullifiers.len(), 2);

        multisig.execute(id).unwrap();
        assert_eq!(
            multisig.proposals[id as usize].status,
            ProposalStatus::Executed
        );
    }

    #[test]
    fn aggregated_approval_rejects_duplicate_member() {
        let set = MemberSet::from_secrets(&secrets(3));
        let mut multisig = Multisig::create(
            2,
            &set,
            vec![TierPolicy {
                id: 1,
                threshold: 2,
                max_amount: 1_000,
            }],
        )
        .unwrap();
        let commitments = commitments_of(&set);
        let id = multisig.propose(transfer()).unwrap();

        let members: Vec<&Member> = vec![set.member(0).unwrap(), set.member(0).unwrap()];
        assert!(matches!(
            multisig.approve_many(id, &commitments, &members),
            Err(SdkError::Prover(
                quorum_prover::ProverError::InvalidWitness(
                    quorum_circuit::CircuitError::DuplicateNullifier
                )
            ))
        ));
    }

    #[test]
    fn double_vote_rejected() {
        let set = MemberSet::from_secrets(&secrets(3));
        let mut multisig = Multisig::create(
            2,
            &set,
            vec![TierPolicy {
                id: 1,
                threshold: 2,
                max_amount: 1_000,
            }],
        )
        .unwrap();
        let commitments = commitments_of(&set);
        let id = multisig.propose(transfer()).unwrap();
        multisig
            .approve(id, &commitments, set.member(0).unwrap())
            .unwrap();
        assert!(matches!(
            multisig.approve(id, &commitments, set.member(0).unwrap()),
            Err(SdkError::Gate(
                quorum_gate_core::GateError::DuplicateNullifier
            ))
        ));
    }
}
