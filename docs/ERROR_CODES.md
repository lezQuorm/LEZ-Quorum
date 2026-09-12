# Error Codes

Quorum uses stable numeric errors for checks that run in the threshold circuit
or deployed gate. Host SDK, composer, RPC, and filesystem errors are typed and
human-readable but do not have consensus error numbers.

## Threshold Circuit

These codes are defined by `CircuitError` in
`crates/quorum-circuit/src/lib.rs`.

| Code | Name | Meaning |
|---:|---|---|
| 3001 | `ZeroThreshold` | The receipt requested a zero threshold |
| 3002 | `TooManyApprovals` | More than ten approvals were supplied |
| 3003 | `ThresholdNotMet` | The receipt has fewer approvals than requested |
| 3004 | `DuplicateNullifier` | The same member appears twice in one witness |
| 3005 | `InvalidMembership` | A Merkle path does not reach the member root |
| 3006 | `AmountExceedsCap` | A transfer exceeds its supplied policy cap |
| 3007 | `NoopRotation` | A rotation keeps the same member root |
| 3008 | `InvalidThresholdChange` | A threshold change sets zero |

The gate repeats all checks that depend on live constitution or proposal state.
For example, it re-derives the tier cap from constitution state rather than
trusting the circuit witness.

## SPEL Gate

These codes are defined by `GateError` in
`crates/quorum-gate-core/src/lib.rs` and exported in
`programs/quorum-gate/idl/quorum_gate.idl.json`.

| Code | Name | Meaning |
|---:|---|---|
| 4001 | `InvalidConstitution` | Constitution fields or proposal counter are invalid |
| 4002 | `TierNotFound` | The transfer references an unknown tier |
| 4003 | `DuplicateNullifier` | This proposal already contains the nullifier |
| 4004 | `ProposalNotActive` | Proposal is executed, or execution threshold is unmet |
| 4005 | `JournalMismatch` | Journal root, proposal ID, or action differs from state |
| 4006 | `ThresholdMismatch` | Receipt threshold/count/vector lengths are inconsistent |
| 4007 | `NoopRotation` | The new member root equals the current root |
| 4008 | `RotationWouldBreakThreshold` | New member count is below current threshold |
| 4009 | `InvalidThresholdChange` | New threshold is outside `1..=member_count` |
| 4010 | `StaleConstitution` | Receipt uses another constitution version |
| 4011 | `TierCapMismatch` | Receipt action cap differs from constitution policy |
| 4012 | `InvalidVault` | Supplied vault is not the multisig's gate PDA |
| 4013 | `ProposalBindingMismatch` | Proposal belongs to another multisig |
| 4014 | `StaleProposal` | Proposal predates current constitution version |
| 4015 | `InvalidRecipient` | Runtime recipient differs from approved action |
| 4016 | `ProposalIdMismatch` | Instruction ID differs from proposal account state |
| 4017 | `CredentialMismatch` | Outer private credential accounts do not match receipt |

Receipt-assumption failure is raised by Risc0's `env::verify` rather than
mapped to a Quorum 4000-series error. Serialization and SPEL framework failures
likewise retain their framework error codes.

## Client Failures

The prover distinguishes invalid witnesses, invalid `RISC0_DEV_MODE`, executor
input failures, proof failures, receipt verification failures, decoding
failures, and host/guest journal mismatch. The SDK adds proposal lookup,
threshold, member-set, and proposal-binding errors. The composer distinguishes
credential mismatch, malformed account lists, chained-call limits, privacy
proof failures, and network confirmation errors.

The network CLI prints these errors with context and exits nonzero. It also
persists transaction status:

| Status | Meaning | Operator response |
|---|---|---|
| `Prepared` | Exact bytes and hash saved, not confirmed | Review, then confirm or reconcile |
| `Submitted` | Submission attempted, final result unknown | Reconcile before any retry |
| `Unknown` | Confirmation timed out or RPC state is inconclusive | Reconcile before any retry |
| `Confirmed` | Sequencer returned the exact hash and bytes | Continue |
| `Orphaned` | Previously expected transaction is absent | Inspect state; dependent actions stop |

An unconfirmed transaction is never silently rebuilt and retried. Use
`quorum network --target <target> reconcile`; exact resubmission requires both
`--resubmit-unconfirmed` and `--confirm-public-write`.

## Updating The Table

After changing gate errors, regenerate and check the IDL:

```bash
cargo run -p quorum-gate-methods --example generate_idl
git diff --exit-code -- programs/quorum-gate/idl/quorum_gate.idl.json
```

The test suite asserts stable representative codes and IDL/source consistency.
