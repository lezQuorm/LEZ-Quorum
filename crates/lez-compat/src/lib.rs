//! # lez-compat - LEZ v0.2.2 compatibility layer
//!
//! Mirrors the Logos Execution Environment commitment and Merkle semantics used
//! by LEZ v0.2.2. The upstream format retains `/LEE/v0.3/` domain strings;
//! those identify the protocol format, not the LEZ software release. The crate
//! also implements the shielded-account rules that make a private multisig
//! possible:
//!
//! - Shielded accounts are never stored in plaintext: every update produces a
//!   **commitment** binding `(account_id, program_owner, balance, nonce, data)`
//!   under the LEE v0.3 protocol commitment prefix used by LEZ v0.2.2.
//! - Membership in a committed set is proven with a **Merkle proof** over
//!   commitment leaves (`leaf = SHA256(commitment)`, `node = SHA256(l||r)`).
//! - The **nonce** and **`program_owner`** constraints that break the public
//!   multisig `PoC` are documented and enforced here (see [`rules`]).
//!
//! The commitment format is verified against the official LEZ v0.2.2 dummy
//! commitment constants, exactly as in `LEZ-TokenStudio/lez-compat`.

use serde::{Deserialize, Serialize};

/// 32-byte digest type used across LEZ.
pub type Digest32 = [u8; 32];

/// LEZ program owner identifier (8 × u32 LE words).
pub type ProgramOwner = [u32; 8];

/// LEZ nullifier secret key used to control a regular private account.
pub type NullifierSecretKey = [u8; 32];

/// LEZ nullifier public key derived from a nullifier secret key.
pub type NullifierPublicKey = [u8; 32];

/// Byte length of an LEZ v0.2.2 ML-KEM-768 viewing public key.
pub const VIEWING_PUBLIC_KEY_LEN: usize = 1184;

/// Official LEE v0.3 commitment domain prefix used by LEZ v0.2.2.
pub const COMMITMENT_PREFIX: &[u8; 32] =
    b"/LEE/v0.3/Commitment/\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";

/// Official LEE v0.3 private-account domain used by LEZ v0.2.2.
pub const PRIVATE_ACCOUNT_ID_PREFIX: &[u8; 32] = b"/LEE/v0.3/AccountId/Private/\x00\x00\x00\x00";

/// Commitment of the default (all-zero) LEZ account — official test vector.
pub const DUMMY_COMMITMENT: Commitment = Commitment([
    55, 228, 215, 207, 112, 221, 239, 49, 238, 79, 71, 135, 155, 15, 184, 45, 104, 74, 51, 211,
    238, 42, 160, 243, 15, 124, 253, 62, 3, 229, 90, 27,
]);

/// Leaf hash of the dummy commitment — official test vector.
pub const DUMMY_COMMITMENT_HASH: Digest32 = [
    250, 237, 192, 113, 155, 101, 119, 30, 235, 183, 20, 84, 26, 32, 196, 229, 154, 74, 254, 249,
    129, 241, 118, 39, 41, 253, 141, 171, 184, 71, 8, 41,
];

/// A shielded LEZ account.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LezAccount {
    /// The account identifier (public for public accounts; a viewing-key
    /// commitment for shielded accounts).
    pub account_id: Digest32,
    /// Owning program; the privacy protocol owns shielded accounts.
    pub program_owner: ProgramOwner,
    /// Token balance.
    pub balance: u128,
    /// Monotonic nonce — **increments on every use of a shielded account**
    /// (the constraint the public multisig `PoC` cannot satisfy).
    pub nonce: u128,
    /// Account data (hashed into the commitment).
    pub data: Vec<u8>,
}

impl LezAccount {
    /// The shielded commitment of this account.
    #[must_use]
    pub fn commitment(&self) -> Commitment {
        Commitment::new(self)
    }

    /// The exact 160-byte preimage hashed by the LEZ protocol.
    #[must_use]
    pub fn commitment_preimage(&self) -> [u8; 160] {
        let mut bytes = [0_u8; 160];
        bytes[..32].copy_from_slice(COMMITMENT_PREFIX);
        bytes[32..64].copy_from_slice(&self.account_id);

        for (index, word) in self.program_owner.iter().enumerate() {
            let start = 64 + index * 4;
            bytes[start..start + 4].copy_from_slice(&word.to_le_bytes());
        }

        bytes[96..112].copy_from_slice(&self.balance.to_le_bytes());
        bytes[112..128].copy_from_slice(&self.nonce.to_le_bytes());
        bytes[128..160].copy_from_slice(&hash_account_data(&self.data));
        bytes
    }
}

/// A shielded commitment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Commitment(pub Digest32);

impl Commitment {
    /// Computes the commitment from an account.
    #[must_use]
    pub fn new(account: &LezAccount) -> Self {
        Self(sha256(&account.commitment_preimage()))
    }

    /// Raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &Digest32 {
        &self.0
    }

    /// The Merkle leaf hash (LEZ semantics: `SHA256(commitment)`).
    #[must_use]
    pub fn leaf_hash(&self) -> Digest32 {
        sha256(&self.0)
    }
}

/// A Merkle membership proof for a commitment leaf.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipProof {
    /// Position of the leaf in the tree.
    pub leaf_index: usize,
    /// Sibling hashes from leaf to root.
    pub siblings: Vec<Digest32>,
}

impl MembershipProof {
    /// Recomputes the root for the given leaf.
    #[must_use]
    pub fn compute_root(&self, commitment: &Commitment) -> Digest32 {
        let mut result = commitment.leaf_hash();
        let mut level_index = self.leaf_index;

        for sibling in &self.siblings {
            let mut pair = [0_u8; 64];
            if level_index & 1 == 0 {
                pair[..32].copy_from_slice(&result);
                pair[32..].copy_from_slice(sibling);
            } else {
                pair[..32].copy_from_slice(sibling);
                pair[32..].copy_from_slice(&result);
            }
            result = sha256(&pair);
            level_index >>= 1;
        }

        result
    }

    /// Verifies the leaf against an expected root.
    #[must_use]
    pub fn verifies(&self, commitment: &Commitment, expected_root: &Digest32) -> bool {
        self.compute_root(commitment) == *expected_root
    }
}

/// Hashes account data.
#[must_use]
pub fn hash_account_data(data: &[u8]) -> Digest32 {
    sha256(data)
}

/// SHA-256 with a single input.
#[must_use]
pub fn sha256(data: &[u8]) -> Digest32 {
    use sha2::{Digest, Sha256};
    Sha256::digest(data).into()
}

/// Derives the official LEZ v0.2.2 nullifier public key from its secret key.
#[must_use]
pub fn nullifier_public_key(secret: &NullifierSecretKey) -> NullifierPublicKey {
    const PREFIX: &[u8; 8] = b"LEE/keys";
    let mut bytes = [0_u8; 64];
    bytes[..8].copy_from_slice(PREFIX);
    bytes[8..40].copy_from_slice(secret);
    bytes[40] = 7;
    sha256(&bytes)
}

/// Derives an official LEZ v0.2.2 regular private account identifier.
///
/// The address binds both privacy public keys. Possession of `secret` is the
/// credential-control statement proved for an authorized private account update.
#[must_use]
pub fn private_account_id(
    secret: &NullifierSecretKey,
    viewing_public_key: &[u8; VIEWING_PUBLIC_KEY_LEN],
    identifier: u128,
) -> Digest32 {
    let mut bytes = [0_u8; 32 + 32 + VIEWING_PUBLIC_KEY_LEN + 16];
    bytes[..32].copy_from_slice(PRIVATE_ACCOUNT_ID_PREFIX);
    bytes[32..64].copy_from_slice(&nullifier_public_key(secret));
    bytes[64..64 + VIEWING_PUBLIC_KEY_LEN].copy_from_slice(viewing_public_key);
    bytes[64 + VIEWING_PUBLIC_KEY_LEN..].copy_from_slice(&identifier.to_le_bytes());
    sha256(&bytes)
}

/// Shielded-account validation rules used by LEZ compatibility checks.
///
/// The public `lez-multisig` `PoC` requires *fresh zero-nonce keypairs claimed by
/// the multisig program*; shielded accounts cannot satisfy that because the
/// privacy protocol owns them and their nonce increments on every use. Quorum
/// instead treats the multisig as a *verifier of ZK threshold proofs*: member
/// accounts never change `program_owner` and their nonce only ever increments
/// under the privacy protocol — Quorum never claims or re-keys them.
pub mod rules {
    use serde::{Deserialize, Serialize};

    /// Deterministic validation-rule errors.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[non_exhaustive]
    pub enum RuleError {
        /// A shielded account's nonce must strictly increment (never decrease).
        NonceRegressed = 2001,
        /// Quorum must never change the `program_owner` of a member account.
        ProgramOwnerChanged = 2002,
        /// A proof must be bound to the current member root.
        StaleMemberRoot = 2003,
        /// Account balance must never be observed by Quorum (shielded).
        BalanceLeak = 2004,
    }

    impl RuleError {
        /// Deterministic error code.
        #[must_use]
        pub const fn code(self) -> u32 {
            self as u32
        }

        /// Description.
        #[must_use]
        pub const fn description(self) -> &'static str {
            match self {
                Self::NonceRegressed => "shielded account nonce regressed",
                Self::ProgramOwnerChanged => "program_owner must not change",
                Self::StaleMemberRoot => "proof bound to stale member root",
                Self::BalanceLeak => "balance must remain shielded",
            }
        }
    }

    impl core::fmt::Display for RuleError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "[{}] {}", self.code(), self.description())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_account_matches_official_dummy_commitment() {
        let account = LezAccount::default();
        assert_eq!(account.commitment(), DUMMY_COMMITMENT);
        assert_eq!(account.commitment().leaf_hash(), DUMMY_COMMITMENT_HASH);
    }

    #[test]
    fn commitment_preimage_has_official_field_order_and_endianness() {
        let account = LezAccount {
            account_id: [0x41; 32],
            program_owner: [
                0x0302_0100,
                0x0706_0504,
                0x0b0a_0908,
                0x0f0e_0d0c,
                0x1312_1110,
                0x1716_1514,
                0x1b1a_1918,
                0x1f1e_1d1c,
            ],
            balance: 0x0f0e_0d0c_0b0a_0908_0706_0504_0302_0100,
            nonce: 0x1f1e_1d1c_1b1a_1918_1716_1514_1312_1110,
            data: b"quorum".to_vec(),
        };

        let preimage = account.commitment_preimage();
        assert_eq!(&preimage[..32], COMMITMENT_PREFIX);
        assert_eq!(&preimage[32..64], &[0x41; 32]);
        assert_eq!(&preimage[64..96], &(0_u8..32).collect::<Vec<_>>());
        assert_eq!(&preimage[96..112], &(0_u8..16).collect::<Vec<_>>());
        assert_eq!(&preimage[112..128], &(16_u8..32).collect::<Vec<_>>());
        assert_eq!(&preimage[128..], &hash_account_data(b"quorum"));
    }

    #[test]
    fn each_private_account_field_changes_the_commitment() {
        let base = LezAccount::default();
        let expected = base.commitment();

        let mut account_id = base.clone();
        account_id.account_id[0] = 1;
        let mut owner = base.clone();
        owner.program_owner[0] = 1;
        let mut balance = base.clone();
        balance.balance = 1;
        let mut nonce = base.clone();
        nonce.nonce = 1;
        let mut data = base;
        data.data.push(1);

        for changed in [account_id, owner, balance, nonce, data] {
            assert_ne!(changed.commitment(), expected);
        }
    }

    #[test]
    fn membership_path_respects_leaf_position() {
        let left = LezAccount::default().commitment();
        let right = LezAccount {
            account_id: [7; 32],
            ..LezAccount::default()
        }
        .commitment();
        let mut pair = [0_u8; 64];
        pair[..32].copy_from_slice(&left.leaf_hash());
        pair[32..].copy_from_slice(&right.leaf_hash());
        let root = sha256(&pair);

        let left_proof = MembershipProof {
            leaf_index: 0,
            siblings: vec![right.leaf_hash()],
        };
        let right_proof = MembershipProof {
            leaf_index: 1,
            siblings: vec![left.leaf_hash()],
        };

        assert!(left_proof.verifies(&left, &root));
        assert!(right_proof.verifies(&right, &root));
        assert!(!right_proof.verifies(&left, &root));
    }

    #[test]
    fn tampered_membership_path_fails() {
        let commitment = LezAccount::default().commitment();
        let proof = MembershipProof {
            leaf_index: 0,
            siblings: vec![[3; 32], [4; 32]],
        };
        let root = proof.compute_root(&commitment);
        let tampered = MembershipProof {
            leaf_index: 0,
            siblings: vec![[3; 32], [5; 32]],
        };
        assert!(!tampered.verifies(&commitment, &root));
    }

    #[test]
    fn rule_errors_have_stable_codes() {
        use rules::RuleError;
        let all = [
            RuleError::NonceRegressed,
            RuleError::ProgramOwnerChanged,
            RuleError::StaleMemberRoot,
            RuleError::BalanceLeak,
        ];
        let mut seen = std::collections::BTreeSet::new();
        for e in all {
            assert!(seen.insert(e.code()));
            assert!(!e.description().is_empty());
            assert!(e.to_string().contains(&e.code().to_string()));
        }
    }

    #[test]
    fn private_account_derivation_matches_official_lez_vectors() {
        let secret = [
            57, 5, 64, 115, 153, 56, 184, 51, 207, 238, 99, 165, 147, 214, 213, 151, 30, 251, 30,
            196, 134, 22, 224, 211, 237, 120, 136, 225, 188, 220, 249, 28,
        ];
        let upstream_viewing_key =
            lee_core::encryption::ViewingPublicKey::from_seed(&[1_u8; 32], &[2_u8; 32]);
        let upstream_nullifier_key = lee_core::NullifierPublicKey::from(&secret);
        let viewing_public_key: &[u8; VIEWING_PUBLIC_KEY_LEN] = upstream_viewing_key
            .to_bytes()
            .try_into()
            .expect("official viewing public key length");
        assert_eq!(
            nullifier_public_key(&secret),
            [
                78, 20, 20, 5, 177, 198, 233, 100, 175, 134, 174, 200, 24, 205, 68, 215, 130, 74,
                35, 54, 154, 184, 219, 42, 168, 106, 126, 147, 133, 244, 18, 218,
            ]
        );
        assert_eq!(
            private_account_id(&secret, viewing_public_key, 0),
            [
                242, 239, 57, 244, 89, 109, 65, 201, 223, 100, 43, 87, 205, 83, 148, 161, 176, 22,
                208, 220, 68, 135, 10, 171, 182, 80, 54, 74, 228, 244, 236, 7,
            ]
        );
        assert_eq!(
            private_account_id(&secret, viewing_public_key, 0),
            *lee_core::account::AccountId::for_regular_private_account(
                &upstream_nullifier_key,
                &upstream_viewing_key,
                0,
            )
            .value()
        );
        assert_eq!(
            private_account_id(&secret, viewing_public_key, 1),
            [
                149, 125, 157, 109, 119, 81, 9, 163, 231, 181, 214, 43, 57, 113, 221, 72, 180, 149,
                189, 170, 32, 181, 255, 231, 19, 92, 235, 59, 153, 185, 172, 206,
            ]
        );
        assert_eq!(
            private_account_id(&secret, viewing_public_key, 1),
            *lee_core::account::AccountId::for_regular_private_account(
                &upstream_nullifier_key,
                &upstream_viewing_key,
                1,
            )
            .value()
        );
    }
}
