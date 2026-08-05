# Architecture Decisions

## ADR-0001: Commit the member set as one Merkle root

**Status:** Accepted.

The constitution stores a root over member commitments rather than a plaintext
member list. Each approval proves a private Merkle path. This hides set
membership but does not hide the public root, member count, version, or the
fact that a rotation occurred.

## ADR-0002: Use proposal- and version-bound nullifiers

**Status:** Accepted.

Each approval derives a nullifier from the member secret, proposal ID, and
constitution version. Duplicate nullifiers are rejected. Changing either the
proposal or constitution changes the nullifier and prevents replay across
governance epochs.

## ADR-0003: Support individual and aggregated proof modes

**Status:** Accepted.

The same circuit supports one approval per receipt and multiple distinct
approvals in one receipt. Aggregated mode reduces the number of proof artifacts;
individual mode allows approvals to accumulate over time. Both paths update the
same proposal nullifier set.

## ADR-0004: Bind proposals to account and constitution version

**Status:** Accepted.

A proposal records its owning multisig ID and the constitution version at
creation. Approval and execution reject a proposal from another multisig or an
older constitution. Rotation changes the root and version atomically, making
prior proposals stale.

## ADR-0005: Verify threshold receipts through Risc0 composition

**Status:** Implemented and locally verified.

The SPEL guest verifies the pinned threshold image and journal with env::verify.
Nested verification requires the host transaction executor to attach the
threshold receipt as an assumption. `quorum-composer` performs this step and
then attaches the gate receipt to the LEZ privacy circuit. Standalone sequencer
and testnet evidence remain pending.

## ADR-0006: Isolate LEZ account compatibility

**Status:** Implemented and locally verified.

`lez-compat` models LEZ v0.3 private account IDs, commitments, nonce
progression, owner stability, and Merkle proofs. Member commitments now bind an
LEZ regular private account ID, and the outer privacy proof establishes control
of that same credential.
