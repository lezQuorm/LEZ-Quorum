# Architecture Decision Records

These records capture the security-relevant choices in the current LEZ v0.2.2
implementation. They describe deployed behavior, not future aspirations.

## ADR-001: Commit The Member Set As A Merkle Root

**Status:** Accepted

The constitution stores a canonically derived SHA-256 Merkle root and member
count rather than public member account IDs. A member supplies the private path
inside the threshold witness.

This keeps the list out of public state and permits compact membership proofs.
It still reveals member count and root changes, and it requires clients to
distribute and preserve correct membership paths.

## ADR-002: Use Proposal- And Version-Scoped Nullifiers

**Status:** Accepted

An approval nullifier hashes the member secret with proposal ID and
constitution version. The gate publishes and deduplicates nullifiers.

This makes a replay for the same proposal deterministic while avoiding a
stable identifier across proposals. Version binding also retires approvals
after member rotation or threshold change. Approval counts and timing remain
public.

## ADR-003: Compose A Threshold Receipt Into The LEZ Privacy Proof

**Status:** Accepted

The threshold guest proves membership and nullifier derivation. The receipt is
an assumption of the outer LEZ execution proof, where the gate verifies the
pinned image and journal. Proposal-scoped credential commitments bind the
receipt identities to private credential accounts authorized by LEZ.

A standalone membership receipt would not prove a valid LEZ account
transition; using only the outer LEZ proof would not implement the Quorum
member-set statement. Composition enforces both. It increases proving time and
integration complexity.

## ADR-004: Accumulate Per-Member Approvals In Proposal State

**Status:** Accepted

The network workflow uses one-member receipts and appends their nullifiers to
the proposal. Execution checks the accumulated count against the threshold.
The circuit and SDK retain an aggregated-receipt path for callers who can
collect several witnesses safely.

Persistent public nullifiers make partial progress restart-safe and let
members submit independently. They reveal approval count and arrival timing.

## ADR-005: Keep Actions And Policy Public

**Status:** Accepted

Proposal action, destination, amount, threshold, tiers, and outcome are public.
The gate compares the receipt action with state and re-derives tier policy from
the constitution.

This limits the privacy claim to membership and approval ownership and makes
execution auditable. Hiding proposal content is explicitly outside LP-0002.

## ADR-006: Use A Program-Derived Treasury Vault

**Status:** Accepted

Each multisig has a deterministic vault derived from the gate program and
multisig account ID. Execution validates that vault and the approved recipient
before emitting a chained token call.

This avoids giving a human signer unilateral control of treasury funds and
binds transfer authority to gate execution. Token-program serialization is a
version-sensitive compatibility boundary and is covered by tests.

## ADR-007: Preview, Persist, Then Submit Exact Bytes

**Status:** Accepted

The network CLI writes the complete transaction and hash to its journal before
submission. Public writes require a second invocation with
`--confirm-public-write`. Confirmation is accepted only for the same hash and
bytes; ambiguous results require reconciliation before exact resubmission.

This prevents accidental writes and unsafe transaction regeneration after a
timeout. It adds an intentional two-step operator workflow.

## ADR-008: Use The Official Builder's `metadata.json`

**Status:** Accepted

The Basecamp flake passes `metadata.json` to the pinned
`logos-module-builder.lib.mkLogosQmlModule`. That builder documents this file as
the single module configuration source and generates native and portable LGX
outputs.

A duplicate `module.json` would create two sources of truth and is not added.
If Basecamp adopts another manifest contract, migrate the locked builder and
manifest together.

## ADR-009: Pin Compatibility-Critical Versions

**Status:** Accepted

CI pins Rust, Risc0, the Risc0 guest toolchain, and LEZ source. Program and
threshold ELF hashes are published with deployment evidence.

Pinning supports reproducibility but also retains upstream security advisories
until a compatible upgrade is reviewed. Upgrades require regenerated image
IDs, artifacts, test evidence, and deployment records.
