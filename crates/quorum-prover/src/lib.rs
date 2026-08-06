//! Host-side proving and verification for Quorum threshold proofs.
//!
//! Mirrors the proven `ProofGate` prover: strict `RISC0_DEV_MODE` handling,
//! host-side statement evaluation first, succinct proofs via the default
//! prover, receipt verification against the pinned image ID, and bincode
//! serialization for transport.

use std::env;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use quorum_circuit::{evaluate, CircuitError, ThresholdJournal, ThresholdWitness};
use quorum_threshold_methods::{THRESHOLD_ELF, THRESHOLD_ID};
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, Receipt};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

/// A threshold proof ready for transport/verification.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuorumProof {
    /// Public journal committed by the guest.
    pub journal: ThresholdJournal,
    /// Bincode-serialized Risc0 receipt, base64 in JSON.
    #[serde(with = "receipt_base64")]
    pub receipt: Vec<u8>,
}

/// `RISC0_DEV_MODE` state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DevModeStatus {
    /// Real proving (`0`/unset).
    Disabled,
    /// Mock proving (`1`) — never acceptable for evidence.
    Enabled,
}

#[derive(Debug, Error)]
pub enum ProverError {
    #[error("witness does not satisfy the threshold statement: {0}")]
    InvalidWitness(#[from] CircuitError),
    #[error("RISC0_DEV_MODE must be 0 or unset for a real Quorum proof")]
    DevModeEnabled,
    #[error("RISC0_DEV_MODE has unsupported value '{0}'")]
    InvalidDevMode(String),
    #[error("failed to prepare Risc0 executor input: {0}")]
    ExecutorInput(String),
    #[error("Risc0 proof generation failed: {0}")]
    Proving(String),
    #[error("Risc0 receipt verification failed: {0}")]
    ReceiptVerification(String),
    #[error("Risc0 journal decoding failed: {0}")]
    JournalDecode(String),
    #[error("Risc0 receipt encoding failed: {0}")]
    ReceiptEncode(String),
    #[error("Risc0 receipt decoding failed: {0}")]
    ReceiptDecode(String),
    #[error("guest journal did not match the host-evaluated statement")]
    JournalMismatch,
}

/// Reads and validates `RISC0_DEV_MODE`.
///
/// # Errors
/// [`ProverError::InvalidDevMode`] for unsupported values.
pub fn dev_mode_status() -> Result<DevModeStatus, ProverError> {
    match env::var("RISC0_DEV_MODE") {
        Err(env::VarError::NotPresent) => parse_dev_mode(None),
        Err(env::VarError::NotUnicode(_)) => {
            Err(ProverError::InvalidDevMode("non-Unicode value".to_owned()))
        }
        Ok(value) => parse_dev_mode(Some(&value)),
    }
}

fn parse_dev_mode(value: Option<&str>) -> Result<DevModeStatus, ProverError> {
    match value {
        None => Ok(DevModeStatus::Disabled),
        Some(value) => match value.trim().to_ascii_lowercase().as_str() {
            "" | "0" | "false" | "off" => Ok(DevModeStatus::Disabled),
            "1" | "true" | "on" => Ok(DevModeStatus::Enabled),
            _ => Err(ProverError::InvalidDevMode(value.to_owned())),
        },
    }
}

/// Fails if dev mode is enabled — evidence must be real proofs.
///
/// # Errors
/// - [`ProverError::InvalidDevMode`] for unsupported `RISC0_DEV_MODE` values.
/// - [`ProverError::DevModeEnabled`] when dev mode is active.
pub fn ensure_real_proving_mode() -> Result<(), ProverError> {
    match dev_mode_status()? {
        DevModeStatus::Disabled => Ok(()),
        DevModeStatus::Enabled => Err(ProverError::DevModeEnabled),
    }
}

/// Generates a real (succinct) threshold proof.
///
/// # Errors
/// Any [`ProverError`] variant, including witness rejection, dev-mode being
/// enabled, proving/verification failures, or journal mismatch.
pub fn prove(witness: &ThresholdWitness) -> Result<QuorumProof, ProverError> {
    ensure_real_proving_mode()?;
    let expected_journal = evaluate(witness)?;

    let env = ExecutorEnv::builder()
        .write(witness)
        .map_err(|error| ProverError::ExecutorInput(error.to_string()))?
        .build()
        .map_err(|error| ProverError::ExecutorInput(error.to_string()))?;
    let prove_info = default_prover()
        .prove_with_opts(env, THRESHOLD_ELF, &ProverOpts::succinct())
        .map_err(|error| ProverError::Proving(error.to_string()))?;
    prove_info
        .receipt
        .verify(THRESHOLD_ID)
        .map_err(|error| ProverError::ReceiptVerification(error.to_string()))?;

    let journal = prove_info
        .receipt
        .journal
        .decode::<ThresholdJournal>()
        .map_err(|error| ProverError::JournalDecode(error.to_string()))?;
    if journal != expected_journal {
        return Err(ProverError::JournalMismatch);
    }

    let receipt = bincode::serialize(&prove_info.receipt)
        .map_err(|error| ProverError::ReceiptEncode(error.to_string()))?;
    Ok(QuorumProof { journal, receipt })
}

/// Verifies a stored proof and returns its journal.
///
/// # Errors
/// Any [`ProverError`] variant: receipt decode/verify failure or journal mismatch.
pub fn verify_receipt(proof: &QuorumProof) -> Result<ThresholdJournal, ProverError> {
    let receipt = decode_receipt(&proof.receipt)?;
    receipt
        .verify(THRESHOLD_ID)
        .map_err(|error| ProverError::ReceiptVerification(error.to_string()))?;
    let journal = receipt
        .journal
        .decode::<ThresholdJournal>()
        .map_err(|error| ProverError::JournalDecode(error.to_string()))?;
    if journal != proof.journal {
        return Err(ProverError::JournalMismatch);
    }
    Ok(journal)
}

/// Decodes a bincode-serialized receipt.
///
/// # Errors
/// [`ProverError::ReceiptDecode`] if the bytes are not a valid receipt.
pub fn decode_receipt(bytes: &[u8]) -> Result<Receipt, ProverError> {
    bincode::deserialize(bytes).map_err(|error| ProverError::ReceiptDecode(error.to_string()))
}

/// The pinned image ID of the threshold guest.
#[must_use]
pub const fn threshold_image_id() -> [u32; 8] {
    THRESHOLD_ID
}

mod receipt_base64 {
    use super::*;

    pub fn serialize<S>(value: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&BASE64.encode(value))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        BASE64.decode(value).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quorum_circuit::{ActionData, MemberApprovalWitness};
    use quorum_core::merkle::MemberTree;
    use quorum_core::nullifier::member_commitment;
    use risc0_zkvm::default_executor;

    fn two_of_three_witness() -> ThresholdWitness {
        let secrets = [[1; 32], [2; 32], [3; 32]];
        let commitments: Vec<[u8; 32]> = secrets.iter().map(member_commitment).collect();
        let tree = MemberTree::new(&commitments);
        let approval_for = |secret: [u8; 32]| {
            let p = tree
                .proof_for(&member_commitment(&secret))
                .expect("member proof");
            MemberApprovalWitness {
                member_secret: secret,
                viewing_public_key: [0_u8; quorum_core::VIEWING_PUBLIC_KEY_LEN],
                account_identifier: 0,
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
                amount: 500,
                tier_id: 1,
                tier_max_amount: 1_000,
            },
            proposal_id: 7,
            constitution_version: 1,
        }
    }

    #[test]
    fn guest_executes_same_statement_as_host() {
        let witness = two_of_three_witness();
        let expected = evaluate(&witness).unwrap();
        let env = ExecutorEnv::builder()
            .write(&witness)
            .unwrap()
            .build()
            .unwrap();
        let session = default_executor().execute(env, THRESHOLD_ELF).unwrap();
        let actual = session.journal.decode::<ThresholdJournal>().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn guest_rejects_under_threshold() {
        let mut witness = two_of_three_witness();
        witness.required_threshold = 3;
        let env = ExecutorEnv::builder()
            .write(&witness)
            .unwrap()
            .build()
            .unwrap();
        assert!(default_executor().execute(env, THRESHOLD_ELF).is_err());
    }

    #[test]
    fn proof_json_uses_base64_receipt() {
        let proof = QuorumProof {
            journal: evaluate(&two_of_three_witness()).unwrap(),
            receipt: vec![0, 1, 2, 253, 254, 255],
        };
        let json = serde_json::to_string(&proof).unwrap();
        assert!(json.contains("\"receipt\":\"AAEC/f7/\""));
        assert_eq!(serde_json::from_str::<QuorumProof>(&json).unwrap(), proof);
    }

    #[test]
    fn dev_mode_parser_is_strict() {
        assert_eq!(parse_dev_mode(None).unwrap(), DevModeStatus::Disabled);
        assert_eq!(parse_dev_mode(Some("0")).unwrap(), DevModeStatus::Disabled);
        assert_eq!(
            parse_dev_mode(Some("true")).unwrap(),
            DevModeStatus::Enabled
        );
        assert!(parse_dev_mode(Some("sometimes")).is_err());
    }

    #[test]
    fn pinned_image_id_matches_compiled_guest() {
        // Guards against drift between the image verified by the on-chain gate
        // and the compiled guest used by the prover. If they ever diverge,
        // every on-chain receipt is
        // rejected. Refresh the pin with scripts/update-image-id.sh.
        assert_eq!(
            threshold_image_id(),
            quorum_image_id::THRESHOLD_IMAGE_ID,
            "quorum-image-id pin drifted from the compiled guest — run scripts/update-image-id.sh"
        );
    }

    #[test]
    #[ignore = "generates a real succinct proof and requires RISC0_DEV_MODE=0"]
    fn real_succinct_proof_round_trip() {
        assert_eq!(env::var("RISC0_DEV_MODE").unwrap(), "0");
        let witness = two_of_three_witness();
        let proof = prove(&witness).unwrap();
        let journal = verify_receipt(&proof).unwrap();
        assert_eq!(journal, evaluate(&witness).unwrap());
        assert!(proof.receipt.len() > 1_000);
    }
}
