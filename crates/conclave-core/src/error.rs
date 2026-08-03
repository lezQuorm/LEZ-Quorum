//! Deterministic error contract for Conclave.
//!
//! LP-0002 requires *"deterministic, documented error codes for all
//! invalid-proof and double-vote scenarios"*. These codes are the contract
//! shared by the ZK verifier, the LEZ program, the SDK, and the CLI.

use core::fmt;

/// Errors produced by Conclave core validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ConclaveError {
    /// Constitution violates invariants (threshold range, empty tiers, ...).
    InvalidConstitution = 1001,
    /// Threshold outside `1..=MAX_MEMBERS` or `threshold > member_count`.
    ThresholdOutOfRange = 1002,
    /// A proposal referenced a spending tier that does not exist.
    TierNotFound = 1003,
    /// Transfer amount exceeds the tier's configured cap.
    AmountExceedsTierCap = 1004,
    /// The same nullifier was submitted twice (double-vote).
    DuplicateNullifier = 1005,
    /// The presented member root is not the current constitution's root.
    InvalidMemberRoot = 1006,
    /// No proposal exists for the given id.
    ProposalNotFound = 1007,
    /// Proposal is not in `Active` state.
    ProposalNotActive = 1008,
    /// Fewer distinct approvals than the constitution requires.
    ThresholdNotMet = 1009,
    /// Unknown or unsupported proposal kind.
    UnknownProposalKind = 1010,
    /// A rotation would leave fewer members than the threshold (M > N').
    RotationWouldBreakThreshold = 1011,
    /// The rotating-out member set does not intersect the current set.
    RotationNoop = 1012,
    /// A tier id was reused; tier ids must be unique.
    DuplicateTierId = 1013,
}

/// Convenience alias.
pub type Result<T> = core::result::Result<T, ConclaveError>;

impl ConclaveError {
    /// The deterministic on-chain/off-chain error code.
    #[must_use]
    pub const fn code(self) -> u32 {
        self as u32
    }

    /// Human-readable description (used by CLI and error tables).
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::InvalidConstitution => "constitution violates invariants",
            Self::ThresholdOutOfRange => "threshold out of range",
            Self::TierNotFound => "spending tier not found",
            Self::AmountExceedsTierCap => "amount exceeds tier cap",
            Self::DuplicateNullifier => "duplicate nullifier (double-vote)",
            Self::InvalidMemberRoot => "member root mismatch",
            Self::ProposalNotFound => "proposal not found",
            Self::ProposalNotActive => "proposal not active",
            Self::ThresholdNotMet => "threshold not met",
            Self::UnknownProposalKind => "unknown proposal kind",
            Self::RotationWouldBreakThreshold => "rotation would break threshold",
            Self::RotationNoop => "rotation does not change membership",
            Self::DuplicateTierId => "duplicate tier id",
        }
    }
}

impl fmt::Display for ConclaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code(), self.description())
    }
}

impl std::error::Error for ConclaveError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable_and_unique() {
        let mut seen = std::collections::BTreeSet::new();
        let all = [
            ConclaveError::InvalidConstitution,
            ConclaveError::ThresholdOutOfRange,
            ConclaveError::TierNotFound,
            ConclaveError::AmountExceedsTierCap,
            ConclaveError::DuplicateNullifier,
            ConclaveError::InvalidMemberRoot,
            ConclaveError::ProposalNotFound,
            ConclaveError::ProposalNotActive,
            ConclaveError::ThresholdNotMet,
            ConclaveError::UnknownProposalKind,
            ConclaveError::RotationWouldBreakThreshold,
            ConclaveError::RotationNoop,
            ConclaveError::DuplicateTierId,
        ];
        for e in all {
            assert!(seen.insert(e.code()), "duplicate code {}", e.code());
            assert!(!e.description().is_empty());
        }
    }

    #[test]
    fn display_includes_code() {
        let s = ConclaveError::DuplicateNullifier.to_string();
        assert!(s.contains("1005"), "got: {s}");
        assert!(s.contains("double-vote"));
    }
}
