# Technical Reference

## Compute Units

LEZ v0.2.2 reports Risc0 `user_cycles`. Measurements use LEZ commit
`d6e4ae694e7419f5906b340c232704466a1917b7`, one warmup, and three runs.

| Operation | Guest execution | User cycles |
|---|---|---:|
| Deploy gate | No guest execution | N/A |
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

Reproduce the measurements:

```bash
RISC0_DEV_MODE=1 cargo run --release -p quorum-composer \
  --example compute_units
```

Development mode supplies the receipt assumption used by the approval fixture.
It does not change the guest instruction trace or `user_cycles`. Client proof
time is separate from program compute units.

## Proof Statement

Each approval proves:

1. control of an enrolled private credential;
2. membership under the current member root;
3. binding to the proposal, action, and constitution version; and
4. a unique proposal-scoped nullifier.

The gate verifies the receipt, rejects duplicate nullifiers, and records the
approval. Execution checks the threshold and calls the LEZ token program for a
transfer.

## Transaction Safety

Transactions are saved before submission. A confirmation is accepted only when
the sequencer returns the same hash and transaction bytes. Missing confirmed
transactions become `Orphaned` and block dependent actions.

Gate errors `4001` through `4017` are defined in
`crates/quorum-gate-core/src/lib.rs` and exported in
`programs/quorum-gate/idl/quorum_gate.idl.json`.

## CI

[Run 31466516663](https://github.com/lezQuorm/LEZ-Quorum/actions/runs/31466516663)
passed:

- formatting, strict Clippy, and workspace tests;
- the standalone LEZ v0.2.2 sequencer lifecycle; and
- a real 2-of-3 threshold proof with `RISC0_DEV_MODE=0`.

The real threshold proof took `675.542 s`, produced a `224,866` byte receipt,
and passed host verification. The sequencer lifecycle ended with
`RESULT=PASS`.

Run the same checks locally:

```bash
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
LEZ_REPO=../../logos-execution-zone-v022 ./scripts/sequencer-e2e.sh
```

## Security

Quorum is unaudited and should not custody assets of material value. The pinned
upstream dependency set includes open RustSec advisories in Logos networking
and Risc0 tooling; compatible upstream releases are required to remove them.
