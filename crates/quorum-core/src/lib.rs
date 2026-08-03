//! # Quorum core — private M-of-N multisig domain model for LEZ
//!
//! Quorum is a private M-of-N multisig primitive for the Logos Execution
//! Zone (LEZ), submitted for λPrize **LP-0002**.
//!
//! ## Privacy properties
//!
//! Unlike the public `lez-multisig` `PoC` (which requires fresh zero-nonce
//! keypairs claimed by the program — impossible for shielded accounts),
//! Quorum is built around **shielded member accounts**:
//!
//! - **No member list on-chain.** The member set is stored only as a Merkle
//!   **root** over per-member identity commitments. An observer cannot tell
//!   who is in the set, or even *whether* the set changed.
//! - **No votes on-chain.** Approvals are expressed as ZK threshold proofs
//!   (built in `quorum-circuit`); the on-chain verifier learns only that a
//!   threshold of distinct, valid members approved.
//! - **Double-vote prevention.** A member's approval binds to the proposal via
//!   a domain-separated **nullifier**; the program rejects duplicate nullifiers.
//! - **Evolving membership (rotation).** Adding/removing members produces a new
//!   commitment root. Revocation is atomic: the old root is retired in the same
//!   state transition, so a removed member's key is provably dead.
//! - **Tiered spending.** Per-category thresholds and amount caps, with category
//!   labels stored only as commitments.
//!
//! ## Restart-safe state
//!
//! The on-chain nullifier set is the source of truth for partial approvals.
//! A client that crashes after submitting < M approvals re-reads program state
//! on restart and resumes — nothing lives only in client memory.
//!
//! [`lez-multisig`]: https://github.com/jimmy-claw/lez-multisig

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod constitution;
pub mod error;
pub mod nullifier;
pub mod proposal;

pub use constitution::{Constitution, SpendingTier, MAX_MEMBERS, MAX_TIERS};
pub use error::{QuorumError, Result};
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
