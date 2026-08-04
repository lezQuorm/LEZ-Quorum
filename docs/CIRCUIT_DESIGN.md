# Circuit Design

## Statement

The quorum-threshold Risc0 guest proves that:

- every supplied approval knows a secret whose commitment belongs to the
  specified member root;
- all approval nullifiers are correctly derived and distinct;
- the number of approvals meets the required threshold;
- the proof is bound to one proposal ID and constitution version; and
- the action passes the circuit-level transfer, rotation, or threshold check.

## Private witness

~~~rust
ThresholdWitness {
    member_root,
    required_threshold,
    approvals: Vec<MemberApprovalWitness>,
    action,
    proposal_id,
    constitution_version,
}
~~~

Each MemberApprovalWitness contains a private member secret, leaf index, and
Merkle siblings.

## Evaluation

1. Require a nonzero threshold and at most ten approvals.
2. Derive each Quorum member commitment with SHA-256 domain separation.
3. Recompute each SHA-256 Merkle path to the supplied member root.
4. Derive a proposal- and version-bound nullifier for each secret.
5. Reject duplicate nullifiers.
6. Require the approval count to meet the threshold.
7. Check that a transfer is below its supplied cap, a rotation changes the
   root, or a threshold change is nonzero.

The gate separately re-derives authoritative tier policy and validates all
state and account bindings. Circuit checks do not replace gate checks.

## Public journal

~~~rust
ThresholdJournal {
    member_root,
    proposal_id,
    constitution_version,
    required_threshold,
    approval_count,
    nullifiers,
    action,
}
~~~

The journal contains no member secrets, commitment list, leaf indices, or
Merkle paths.

## Proof modes

Individual mode proves one member at a time and lets the gate accumulate
nullifiers. Aggregated mode proves several distinct approvals in one receipt.
Both use the same guest and image ID.

## Hash domains

Quorum membership and nullifiers use the domains defined in
crates/quorum-core/src/nullifier.rs. The Merkle tree hashes commitment leaves
and internal node pairs with SHA-256. This is separate from the LEZ account
commitment format modeled by lez-compat.

## Receipt verification

quorum-prover verifies generated receipts against the pinned image ID on the
host. The SPEL guest calls env::verify for nested verification. A LEZ
transaction composer must add the threshold receipt as an executor assumption;
that integration is still pending.
