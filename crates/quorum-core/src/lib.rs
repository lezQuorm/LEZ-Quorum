//! # Quorum core
//!
//! Domain types and validation rules for a private threshold treasury on the
//! Logos Execution Zone.
//!
//! Quorum commits the member set as one Merkle root, represents approvals with
//! proposal-bound nullifiers, and supports tiered transfers, member rotation,
//! and threshold changes. Member secrets and Merkle paths remain private;
//! policies, proposal actions, nullifiers, roots, versions, and rotations are
//! public by design.
//!
//! This crate is network-independent. Live shielded-account credential binding
//! and LEZ transaction composition are integration responsibilities documented
//! at the workspace level.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod constitution;
pub mod error;
pub mod merkle;
pub mod nullifier;
pub mod proposal;

pub use constitution::{Constitution, SpendingTier, MAX_MEMBERS, MAX_TIERS};
pub use error::{QuorumError, Result};
pub use lez_compat::VIEWING_PUBLIC_KEY_LEN;
pub use nullifier::{derive_nullifier, member_commitment};
pub use proposal::{Proposal, ProposalKind, ProposalStatus};

/// A LEZ account identifier (public or shielded commitment).
pub type AccountId = [u8; 32];

/// A 32-byte commitment (SHA-256 based; circuit replaces with Poseidon if LEZ requires).
pub type Commitment = [u8; 32];

/// A 32-byte nullifier preventing double-voting.
pub type Nullifier = [u8; 32];

/// A LEZ program identifier.
pub type ProgramId = [u8; 32];

/// Canonical domain-separation tag for all Quorum hashes.
pub const DOMAIN_TAG: &[u8] = b"quorum/v1";
