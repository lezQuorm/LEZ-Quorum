# Quorum — Deterministic Error Codes

Every layer exposes stable, documented codes (LP-0002 reliability criterion).

## quorum-core (`ConclaveError` → renamed `QuorumError`)

| Code | Variant | Meaning |
|---|---|---|
| 1001 | `InvalidConstitution` | constitution violates invariants |
| 1002 | `ThresholdOutOfRange` | threshold outside `1..=member_count` |
| 1003 | `TierNotFound` | spending tier not found |
| 1004 | `AmountExceedsTierCap` | amount exceeds tier cap |
| 1005 | `DuplicateNullifier` | double-vote |
| 1006 | `InvalidMemberRoot` | member root mismatch |
| 1007 | `ProposalNotFound` | proposal not found |
| 1008 | `ProposalNotActive` | proposal not active |
| 1009 | `ThresholdNotMet` | approval count below threshold |
| 1010 | `UnknownProposalKind` | unsupported proposal kind |
| 1011 | `RotationWouldBreakThreshold` | rotation would leave M > N |
| 1012 | `RotationNoop` | rotation does not change the set |
| 1013 | `DuplicateTierId` | tier ids must be unique |

## lez-compat (`RuleError`)

| Code | Variant | Meaning |
|---|---|---|
| 2001 | `NonceRegressed` | shielded nonce regressed |
| 2002 | `ProgramOwnerChanged` | program_owner must not change |
| 2003 | `StaleMemberRoot` | proof bound to a stale root |
| 2004 | `BalanceLeak` | balance must remain shielded |

## quorum-circuit (`CircuitError`)

| Code | Variant | Meaning |
|---|---|---|
| 3001 | `ZeroThreshold` | required threshold is zero |
| 3002 | `TooManyApprovals` | more than `MAX_APPROVALS` |
| 3003 | `ThresholdNotMet` | approvals below threshold |
| 3004 | `DuplicateNullifier` | same member approved twice |
| 3005 | `InvalidMembership` | commitment not in member root |
| 3006 | `AmountExceedsCap` | transfer over tier cap |
| 3007 | `NoopRotation` | rotation to same root |
| 3008 | `InvalidThresholdChange` | threshold change to 0 |

## quorum-gate-core (`GateError`)

| Code | Variant | Meaning |
|---|---|---|
| 4001 | `InvalidConstitution` | constitution malformed |
| 4002 | `TierNotFound` | unknown tier |
| 4003 | `DuplicateNullifier` | double-vote on-chain |
| 4004 | `ProposalNotActive` | proposal not active / below threshold |
| 4005 | `JournalMismatch` | journal ≠ proposal binding |
| 4006 | `ThresholdMismatch` | degenerate proof |
| 4007 | `NoopRotation` | rotation to same root |
| 4008 | `RotationWouldBreakThreshold` | rotation breaks threshold |
| 4009 | `InvalidThresholdChange` | threshold out of range |
| 4010 | `StaleConstitution` | proof bound to an older constitution |
| 4011 | `TierCapMismatch` | journal tier cap ≠ constitution tier cap |
| 4012 | `InvalidVault` | vault account is not the treasury PDA |

## On-chain program (`fail` codes)

- `1011` journal encoding failure
- `2005` account state encode/decode failure
- gate errors surface as their `GateError` codes
