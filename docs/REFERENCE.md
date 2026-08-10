# Technical Reference

## Compute Units

LEZ v0.2.2 measures program work as Risc0 `user_cycles`. Quorum uses the same
`default_executor().execute(...).cycles()` metric as the official LEZ
`tools/cycle_bench` utility.

Measurements use LEZ commit
`d6e4ae694e7419f5906b340c232704466a1917b7`, one warmup, and three measured
executions. Every measured guest used one segment.

| Lifecycle operation | Guest execution | User cycles |
|---|---|---:|
| Deploy gate | Bytecode validation; no guest execution | N/A |
| Initialize constitution | Quorum gate | 75,173 |
| Create token | LEZ token | 97,284 |
| Initialize recipient | LEZ token | 104,491 |
| Initialize vault | Gate 217,755 + token 104,491 | 322,246 |
| Fund vault | LEZ token transfer | 128,649 |
| Propose transfer | Quorum gate | 154,336 |
| Approve one member | Quorum gate | 300,267 |
| Execute transfer | Gate 371,170 + token 128,649 | 499,819 |
| Propose member rotation | Quorum gate | 150,486 |
| Execute member rotation | Quorum gate | 307,506 |
| Propose threshold change | Quorum gate | 132,404 |
| Execute threshold change | Quorum gate | 292,096 |

Reproduce:

```bash
RISC0_DEV_MODE=1 cargo run --release -p quorum-composer \
  --example compute_units
```

Development mode is used only to create the receipt assumption needed by the
approval fixture. It does not change the guest instruction trace or
`user_cycles`. The public RPC returns transaction bytes and block IDs, but no
gas or CU field. Local wall-clock time is therefore not presented as testnet
CU.

A private approval also generates a threshold receipt and an outer LEZ privacy
receipt on the client. Those proof times are separate from the gate's program
CU and depend on hardware.

## Proof Model

The constitution stores a Merkle root, member count, threshold, spending tiers,
version, and proposal counter. It does not store the member list.

Each approval proves:

1. control of an enrolled private-account credential;
2. membership under the active constitution root;
3. binding to the proposal, action, and constitution version; and
4. a distinct proposal-scoped nullifier.

The gate verifies the threshold receipt assumption, rejects duplicate
nullifiers, and updates only public proposal state. Transfer execution chains
into the LEZ token program using the treasury vault PDA.

Private material includes nullifier secrets, viewing secrets, credential IDs,
member paths, and private post-state. Public state includes policy, proposal
content, approval count, nullifiers, balances, and execution status.

## Error Surface

Gate errors `4001` through `4017` are defined in
`crates/quorum-gate-core/src/lib.rs` and exported through
`programs/quorum-gate/idl/quorum_gate.idl.json`. They cover invalid policies,
stale proposals, threshold mismatch, duplicate votes, credential mismatch,
invalid vaults, and action-binding failures.

Network transactions are journaled before submission. A saved confirmation is
accepted only when the current chain returns the same hash and identical
transaction bytes. A vanished confirmation becomes `Orphaned` and blocks all
dependent actions.

## Validation

Fast local checks:

```bash
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
RISC0_DEV_MODE=1 LEZ_REPO=../../logos-execution-zone-v022 \
  ./scripts/sequencer-e2e.sh
```

Real-proof standalone lifecycle:

```bash
LEZ_REPO=../../logos-execution-zone-v022 ./scripts/sequencer-e2e.sh
```

The runner verifies the exact LEZ commit, refuses an occupied RPC endpoint,
owns and cleans up the sequencer process, and requires `RESULT=PASS`.

Measured on 2026-08-10 with four CPU threads and 15 GiB RAM:

| Measurement | Time |
|---|---:|
| Threshold receipt | 1,431.910 s |
| Complete private approval | 5,617.298 s |
| Full standalone lifecycle | 7,429 s |

The complete approval measurement includes threshold, gate, and LEZ privacy
proofs. These are local wall-clock timings, not compute units or testnet
confirmation times.

## Security

Quorum is experimental. It has no trusted setup beyond Risc0's assumptions and
has not received an independent security audit. Do not use it to custody assets
of material value before an audit and operational key-management review.

A RustSec scan on 2026-08-10 reports four advisories in pinned upstream
dependencies: two in `hickory-proto` through Logos networking, one in
`tracing-subscriber` through Logos proof dependencies, and one in `rsa`
through Risc0's `rzup` build tooling. It also reports an `lru` unsoundness
warning through Logos networking. Resolving them requires compatible upstream
LEZ, Logos blockchain, and Risc0 releases.
