# Quorum — Circuit Design

## Statement

One Risc0 guest (`quorum-threshold`, pinned image ID in `quorum-image-id`) proves:

> "Each of the supplied approvals is by a **distinct** member of the committed set
> `member_root` (a valid SHA-256 Merkle path exists for their commitment), their
> **nullifier** is correctly derived for `proposal_id` under constitution version
> `V`, the approval count meets the required threshold, and the action respects
> its policy (transfer under tier cap / non-noop rotation / valid threshold change)."

## Witness (private inputs)

```rust
ThresholdWitness {
    member_root,          // public binding
    required_threshold,   // public binding
    approvals: Vec<MemberApprovalWitness>,  // member_secret is PRIVATE
    action,               // public binding
    proposal_id,          // public binding
    constitution_version, // public binding
}
```

## Evaluation (`quorum-circuit::evaluate`)

1. `required_threshold ≥ 1`, `approvals.len() ≤ MAX_APPROVALS` (10).
2. For each approval: `commitment = member_commitment(secret)`; verify the Merkle
   path against `member_root` (LEZ hashing: `leaf = H(commitment)`,
   `node = H(left‖right)`).
3. `nullifier = derive_nullifier(secret, proposal_id, version)`; all nullifiers
   must be pairwise distinct (double-vote prevention).
4. `approvals.len() ≥ required_threshold`.
5. Action policy: `Transfer.amount ≤ tier_max_amount`; `RotateMembers.new_root ≠
   member_root`; `ChangeThreshold.new_threshold ≥ 1`.

## Journal (public outputs)

```rust
ThresholdJournal {
    member_root, proposal_id, constitution_version,
    required_threshold, approval_count,
    nullifiers,               // on-chain double-vote prevention
    action,                   // what the gate executes
}
```

The journal contains **no secrets** (tested: `journal_does_not_expose_secrets`).

## Usage modes

- **Per-member approvals:** each member proves one approval
  (`required_threshold = 1`); the gate aggregates nullifiers and enforces the
  threshold (`quorum approve --member i --proposal p`).
- **Aggregated single proof (differentiator B3, shipped):** all M members'
  approvals are proven in **one** proof (`required_threshold = M`) for a
  single-tx path (`quorum approve-all --proposal p --members 0,1` / SDK
  `Multisig::approve_many`).

## Hashing

All hashing matches LEZ semantics (`lez-compat`): commitments use the
`/LEE/v0.3/Commitment/` prefix; Merkle leaves are `SHA256(commitment)`.

## Errors

`CircuitError` codes `3001`–`3008` (see `ERROR_CODES.md`). A failing guest aborts
proof generation — the prover never produces a receipt for an invalid witness.

## Proving stack

Risc0 **3.0.5**, succinct proofs via `default_prover().prove_with_opts(..., &ProverOpts::succinct())`,
strict `RISC0_DEV_MODE=0` enforcement in the prover (`ProverError::DevModeEnabled`).
