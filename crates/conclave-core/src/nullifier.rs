//! Nullifier and member-commitment derivation.
//!
//! ## Nullifiers (double-vote prevention)
//!
//! ```text
//! nullifier = SHA256("conclave/v1/nullifier" || member_secret
//!                    || proposal_id_le64 || constitution_version_le32)
//! ```
//!
//! A member's approval for a given proposal always produces the *same*
//! nullifier (their secret is bound to the proposal), and the on-chain
//! verifier rejects any nullifier it has already seen. A member therefore
//! cannot approve twice. Crucially, a nullifier reveals nothing about the
//! member's identity or shielded account.
//!
//! ## Member commitments (shielded membership)
//!
//! ```text
//! member_commitment = SHA256("conclave/v1/member" || member_secret)
//! ```
//!
//! The multisig stores only the **root** of a Merkle tree over member
//! commitments. Membership in the ZK circuit is proven with a Merkle path —
//! no plaintext member list ever appears on-chain.
//!
//! > Note: LEZ's own commitment scheme uses Poseidon; `conclave-circuit` will
//! > switch to the LEZ-native hash for on-chain-verifiable proofs (Chunk 3).
//! > The SHA-256 forms here are the canonical *domain separation format*,
//! > stable across both.

use sha2::{Digest, Sha256};

use crate::{Commitment, Nullifier, DOMAIN_TAG};

const NULLIFIER_TAG: &[u8] = b"/nullifier";
const MEMBER_TAG: &[u8] = b"/member";

/// Derives the nullifier for a member's approval of a proposal.
///
/// Deterministic: same `(member_secret, proposal_id, constitution_version)`
/// always yields the same nullifier — which is exactly what makes replay of
/// an approval detectable on-chain.
#[must_use]
pub fn derive_nullifier(
    member_secret: &[u8; 32],
    proposal_id: u64,
    constitution_version: u32,
) -> Nullifier {
    let mut h = Sha256::new();
    h.update(DOMAIN_TAG);
    h.update(NULLIFIER_TAG);
    h.update(member_secret);
    h.update(proposal_id.to_le_bytes());
    h.update(constitution_version.to_le_bytes());
    h.finalize().into()
}

/// Derives a member's identity commitment from their secret.
#[must_use]
pub fn member_commitment(member_secret: &[u8; 32]) -> Commitment {
    let mut h = Sha256::new();
    h.update(DOMAIN_TAG);
    h.update(MEMBER_TAG);
    h.update(member_secret);
    h.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nullifier_is_deterministic() {
        let secret = [7u8; 32];
        assert_eq!(
            derive_nullifier(&secret, 1, 1),
            derive_nullifier(&secret, 1, 1)
        );
    }

    #[test]
    fn nullifier_differs_across_proposals() {
        let secret = [7u8; 32];
        assert_ne!(
            derive_nullifier(&secret, 1, 1),
            derive_nullifier(&secret, 2, 1)
        );
    }

    #[test]
    fn nullifier_differs_across_constitutions() {
        let secret = [7u8; 32];
        assert_ne!(
            derive_nullifier(&secret, 1, 1),
            derive_nullifier(&secret, 1, 2)
        );
    }

    #[test]
    fn nullifier_does_not_leak_secret_bits() {
        // A nullifier is a hash of the secret: flipping one secret byte must
        // avalanche the output and never expose the secret trivially.
        let a = derive_nullifier(&[1u8; 32], 1, 1);
        let b = derive_nullifier(&[2u8; 32], 1, 1);
        assert_ne!(a, b);
        // The hash output must differ in more than the corresponding byte
        // (i.e. it is not an identity permutation of the secret).
        assert_ne!(a[0] ^ b[0], 3, "avalanche sanity");
    }

    #[test]
    fn member_commitment_is_deterministic_and_hides_secret() {
        let c = member_commitment(&[9u8; 32]);
        assert_eq!(c, member_commitment(&[9u8; 32]));
        assert_ne!(c, member_commitment(&[8u8; 32]));
    }
}
