# Error Codes

Quorum uses deterministic numeric errors for domain, compatibility, circuit, and
gate validation.

## Domain errors

| Code | Variant | Meaning |
|---|---|---|
| 1001 | InvalidConstitution | Constitution invariants failed |
| 1002 | ThresholdOutOfRange | Threshold is outside 1..=member_count |
| 1003 | TierNotFound | Spending tier does not exist |
| 1004 | AmountExceedsTierCap | Transfer exceeds the tier cap |
| 1005 | DuplicateNullifier | Approval was already recorded |
| 1006 | InvalidMemberRoot | Member root does not match |
| 1007 | ProposalNotFound | Proposal does not exist |
| 1008 | ProposalNotActive | Proposal is no longer active |
| 1009 | ThresholdNotMet | Approval count is below the threshold |
| 1010 | UnknownProposalKind | Proposal action is unsupported |
| 1011 | RotationWouldBreakThreshold | New member count is below the threshold |
| 1012 | RotationNoop | Rotation keeps the same member root |
| 1013 | DuplicateTierId | Tier IDs must be unique |

## LEZ compatibility errors

| Code | Variant | Meaning |
|---|---|---|
| 2001 | NonceRegressed | Shielded account nonce regressed |
| 2002 | ProgramOwnerChanged | Shielded account owner changed |
| 2003 | StaleMemberRoot | Proof uses an inactive member root |
| 2004 | BalanceLeak | A shielded balance became public |

Account state encoding and decoding failures in the SPEL guest use code 2005.

## Circuit errors

| Code | Variant | Meaning |
|---|---|---|
| 3001 | ZeroThreshold | Required threshold is zero |
| 3002 | TooManyApprovals | Approval count exceeds the circuit limit |
| 3003 | ThresholdNotMet | Witness has too few approvals |
| 3004 | DuplicateNullifier | The same member appears more than once |
| 3005 | InvalidMembership | Merkle path does not reach the member root |
| 3006 | AmountExceedsCap | Transfer exceeds the witness policy cap |
| 3007 | NoopRotation | Rotation keeps the current root |
| 3008 | InvalidThresholdChange | New threshold is zero |

## Gate errors

| Code | Variant | Meaning |
|---|---|---|
| 4001 | InvalidConstitution | Constitution is malformed |
| 4002 | TierNotFound | Spending tier does not exist |
| 4003 | DuplicateNullifier | Nullifier was already accepted |
| 4004 | ProposalNotActive | Proposal is inactive or below threshold |
| 4005 | JournalMismatch | Proof journal does not bind to the proposal |
| 4006 | ThresholdMismatch | Journal threshold or count is invalid |
| 4007 | NoopRotation | Rotation keeps the current root |
| 4008 | RotationWouldBreakThreshold | Rotation violates threshold policy |
| 4009 | InvalidThresholdChange | New threshold is out of range |
| 4010 | StaleConstitution | Journal uses an older constitution |
| 4011 | TierCapMismatch | Journal cap differs from constitution policy |
| 4012 | InvalidVault | Runtime vault is not the treasury PDA |
| 4013 | ProposalBindingMismatch | Proposal belongs to another multisig |
| 4014 | StaleProposal | Proposal was created under an older constitution |
| 4015 | InvalidRecipient | Runtime recipient differs from the approved action |
| 4016 | ProposalIdMismatch | Instruction ID differs from proposal state |

Threshold journal serialization failures in the SPEL guest use code 1011.
