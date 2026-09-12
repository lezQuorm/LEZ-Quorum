# Benchmarks

Quorum reports proof generation separately from LEZ guest compute. These are
different costs: proof time is client wall-clock work; `user_cycles` measures
program execution inside LEZ and is not a token fee or wall-clock latency.

## Client Proof

A real 2-of-3 threshold proof using `RISC0_DEV_MODE=0` was recorded in CI:

| Metric | Result |
|---|---:|
| Receipt kind | Risc0 succinct |
| Wall time | `675.542 s` |
| Encoded receipt | `224,866 bytes` |
| Host verification | Passed |

An independent local rerun on 2026-09-10, after the 0.1.1 documentation and
version update, completed in `1491.200 s`, produced the same `224,866` byte
receipt, and printed `verify ok: yes`. The difference from CI illustrates why
proof time is reported with its environment rather than as a fixed protocol
constant.

Reproduce it with the pinned toolchain:

```bash
RISC0_DEV_MODE=0 cargo run --release -p quorum-prover --example prove_threshold
```

Proof time varies materially with CPU, memory, accelerator support, cache
state, and Risc0 release. The measurement demonstrates laptop/CI-class client
execution; it is not a service-level guarantee. Development mode is useful for
tests but must not be used to publish a proof-time result.

## Full Real-Proof Lifecycle

The standalone LEZ v0.2.2 sequencer script was also run end to end locally on
2026-09-10 with `RISC0_DEV_MODE=0`:

| Stage | Wall time |
|---|---:|
| Aggregated 2-of-3 threshold proof | `1001.001 s` |
| Gate and outer LEZ privacy proof composition | `4488.275 s` |
| Complete script, including builds, writes, confirmations, and final checks | `5620 s` |

The composed approval and execution both confirmed. The script asserted vault
`500`, recipient `250`, proposal `Executed`, `RESULT=PASS`, and
`SEQUENCER_E2E=PASS`. The composition timing starts after the threshold receipt
and ends before network submission; it therefore measures the gate/privacy
proof work and transaction construction, not confirmation latency.

Reproduce that exact path with no development-mode override:

```bash
LEZ_REPO=../logos-execution-zone-v022 ./scripts/sequencer-e2e.sh
```

## LEZ Compute Units

Measurements below use LEZ v0.2.2 commit
`d6e4ae694e7419f5906b340c232704466a1917b7`, one warmup, and three measured
runs. Values are Risc0 executor `user_cycles` from local guest execution.

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

Reproduce the table:

```bash
RISC0_DEV_MODE=1 cargo run --release -p quorum-composer --example compute_units
```

Development receipts supply the approval assumption for this executor fixture.
They do not change the guest instruction trace counted as `user_cycles`. This
method therefore measures deterministic guest compute, while the real-proof
command above measures cryptographic proof generation.

## Interpretation

- The largest measured program operation is transfer execution because it runs
  the gate and a chained token transfer.
- Approval cost is per network approval transaction. An aggregated receipt can
  carry several nullifiers but has a different client proving workload.
- LEZ's transaction budget and fee mapping may change during testnet. Confirm
  current limits before deployment.
- Testnet explorer records transaction inclusion but does not expose this local
  benchmark methodology; the source example is the reproducible evidence.

The CI evidence, local real-proof lifecycle, and testnet transactions are linked from
[Deployment](DEPLOYMENT.md).
