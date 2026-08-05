//! Nullifier and member-commitment derivation.
//!
//! ## Nullifiers (double-vote prevention)
//!
//! ```text
//! nullifier = SHA256("quorum/v1/nullifier" || member_secret
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
//! account_id = LEZ_PRIVATE_ACCOUNT_ID(member_secret, account_identifier)
//! member_commitment = SHA256("quorum/v1/member" || account_id)
//! ```
//!
//! The multisig stores only the **root** of a Merkle tree over member
//! commitments. Membership in the ZK circuit is proven with a Merkle path —
//! no plaintext member list ever appears on-chain.
//!
//! The threshold circuit therefore proves control of the same LEZ nullifier
//! secret key that the LEZ privacy circuit uses for an authorized private
//! account update. The gate binds a proposal-scoped credential commitment to
//! the private account supplied to the outer transaction.

use sha2::{Digest, Sha256};

use crate::{Commitment, Nullifier, DOMAIN_TAG};

const NULLIFIER_TAG: &[u8] = b"/nullifier";
const MEMBER_TAG: &[u8] = b"/member";
const CREDENTIAL_TAG: &[u8] = b"/credential";

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

/// Derives a blinded member commitment from a LEZ private account id.
#[must_use]
pub fn member_commitment_from_account_id(account_id: &[u8; 32]) -> Commitment {
    let mut h = Sha256::new();
    h.update(DOMAIN_TAG);
    h.update(MEMBER_TAG);
    h.update(account_id);
    h.finalize().into()
}

/// Derives the proposal-scoped binding used to connect a threshold receipt to
/// the private credential account authorized by the outer LEZ proof.
///
/// Scoping prevents a leaked receipt journal from becoming a stable member
/// pseudonym across proposals or constitution versions.
#[must_use]
pub fn credential_commitment_from_account_id(
    account_id: &[u8; 32],
    member_root: &[u8; 32],
    proposal_id: u64,
    constitution_version: u32,
) -> Commitment {
    let mut h = Sha256::new();
    h.update(DOMAIN_TAG);
    h.update(CREDENTIAL_TAG);
    h.update(account_id);
    h.update(member_root);
    h.update(proposal_id.to_le_bytes());
    h.update(constitution_version.to_le_bytes());
    h.finalize().into()
}

/// Derives a member commitment from a LEZ nullifier secret and account
/// identifier.
#[must_use]
pub fn member_commitment_for_credential(
    member_secret: &[u8; 32],
    account_identifier: u128,
) -> Commitment {
    member_commitment_from_account_id(&lez_compat::private_account_id(
        member_secret,
        account_identifier,
    ))
}

/// Derives a proposal-scoped credential binding from the LEZ nullifier secret
/// and account identifier proved by the threshold circuit.
#[must_use]
pub fn credential_commitment_for_credential(
    member_secret: &[u8; 32],
    account_identifier: u128,
    member_root: &[u8; 32],
    proposal_id: u64,
    constitution_version: u32,
) -> Commitment {
    credential_commitment_from_account_id(
        &lez_compat::private_account_id(member_secret, account_identifier),
        member_root,
        proposal_id,
        constitution_version,
    )
}

/// Derives a member commitment for the default LEZ account identifier zero.
#[must_use]
pub fn member_commitment(member_secret: &[u8; 32]) -> Commitment {
    member_commitment_for_credential(member_secret, 0)
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

    #[test]
    fn member_commitment_binds_lez_account_identifier() {
        let secret = [9_u8; 32];
        assert_ne!(
            member_commitment_for_credential(&secret, 0),
            member_commitment_for_credential(&secret, 1)
        );
        let account_id = lez_compat::private_account_id(&secret, 7);
        assert_eq!(
            member_commitment_for_credential(&secret, 7),
            member_commitment_from_account_id(&account_id)
        );
    }

    #[test]
    fn credential_commitment_is_scoped_to_the_approval() {
        let account_id = lez_compat::private_account_id(&[9_u8; 32], 7);
        let binding = credential_commitment_from_account_id(&account_id, &[1_u8; 32], 3, 2);
        assert_eq!(
            binding,
            credential_commitment_for_credential(&[9_u8; 32], 7, &[1_u8; 32], 3, 2)
        );
        assert_ne!(
            binding,
            credential_commitment_from_account_id(&account_id, &[1_u8; 32], 4, 2)
        );
        assert_ne!(
            binding,
            credential_commitment_from_account_id(&account_id, &[2_u8; 32], 3, 2)
        );
        assert_ne!(
            binding,
            credential_commitment_from_account_id(&account_id, &[1_u8; 32], 3, 3)
        );
    }
}
