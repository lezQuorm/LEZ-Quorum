//! # quorum-sdk — client-side toolkit
//!
//! Everything a shielded member needs: create a member set, open proposals,
//! generate **client-side approval proofs** (real mode for evidence, dev mode
//! for fast tests), aggregate approvals, and apply actions — mirroring the
//! on-chain gate state machine so the full flow is verifiable offline first.
//!
//! Privacy: member secrets are written only to `Member` values and (in the
//! CLI) to `mode 600` secret files. They never enter proofs' journals.

use quorum_circuit::{
    evaluate, ActionData, MemberApprovalWitness, ThresholdJournal, ThresholdWitness,
};
use quorum_core::merkle::MemberTree;
use quorum_core::nullifier::member_commitment;
use quorum_gate_core::{
    apply_action, apply_approved_claim, check_claim, ConstitutionState, OnChainThresholdJournal,
    ProposalState, ProposalStatus, TierPolicy,
};
use quorum_prover::{dev_mode_status, prove, DevModeStatus, QuorumProof};
use rand::RngCore;
use serde::{Deserialize, Serialize};
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
    /// An approval proof was bound to a different proposal.
    #[error("proof proposal mismatch")]
    ProofProposalMismatch,
}

/// A shielded member: identity secret + derived commitment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Member {
    /// Canonical index in the member set.
    pub index: usize,
    /// Identity secret — NEVER commit this to a public file.
    pub secret: [u8; 32],
}

impl Member {
    /// The member's identity commitment (safe to publish).
    #[must_use]
    pub fn commitment(&self) -> [u8; 32] {
        member_commitment(&self.secret)
    }
}

/// A set of members and the shielded Merkle root over their commitments.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberSet {
    /// Members (secrets included; keep this structure out of public evidence).
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
    /// # Panics
    /// If `member` is not part of `self` (its commitment is not a leaf).
    #[must_use]
    pub fn approval_witness(
        &self,
        member: &Member,
        _proposal_id: u64,
        _constitution_version: u32,
    ) -> MemberApprovalWitness {
        let proof = MemberTree::new(
            &self
                .members
                .iter()
                .map(Member::commitment)
                .collect::<Vec<_>>(),
        )
        .proof_for(&member.commitment())
        .expect("member is in the set");
        MemberApprovalWitness {
            member_secret: member.secret,
            leaf_index: proof.leaf_index,
            siblings: proof.siblings,
        }
    }
}

/// Builds an approval witness from a member set's commitments and one member's
/// secret (the CLI stores only commitments + each member's own secret file).
///
/// # Panics
/// If `member_secret` is not committed in `commitments`.
#[must_use]
pub fn approval_witness_for(
    commitments: &[[u8; 32]],
    member_secret: &[u8; 32],
    _proposal_id: u64,
    _constitution_version: u32,
) -> MemberApprovalWitness {
    let tree = MemberTree::new(commitments);
    let commitment = member_commitment(member_secret);
    let proof = tree.proof_for(&commitment).expect("member is in the set");
    MemberApprovalWitness {
        member_secret: *member_secret,
        leaf_index: proof.leaf_index,
        siblings: proof.siblings,
    }
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
        Ok(Self {
            constitution: ConstitutionState::new(
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
    /// # Errors
    /// [`SdkError::Gate`] if the action is invalid for this constitution.
    pub fn propose(&mut self, action: ActionData) -> Result<u64, SdkError> {
        let threshold = self.constitution.required_threshold(&action)?;
        let id = self.constitution.proposal_counter;
        let proposal = ProposalState::new(id, self.constitution.version, threshold, action);
        self.proposals.push(proposal);
        self.constitution.proposal_counter = self.constitution.proposal_counter.saturating_add(1);
        Ok(id)
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
                proposal_id,
                self.constitution.version,
            )],
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

    /// Rejects a proposal.
    ///
    /// # Errors
    /// [`SdkError::Gate`] if the proposal is not active.
    pub fn reject(&mut self, proposal_id: u64) -> Result<(), SdkError> {
        let proposal = self
            .proposals
            .get_mut(
                usize::try_from(proposal_id)
                    .map_err(|_| SdkError::ProposalNotFound(proposal_id))?,
            )
            .ok_or(SdkError::ProposalNotFound(proposal_id))?;
        proposal.status = ProposalStatus::Rejected;
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
        let w = set.approval_witness(member, 1, 1);
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
            all.iter().map(member_commitment).collect()
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
        let w = MemberSet::from_secrets(&[old.secret]).approval_witness(old, 99, 2);
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
