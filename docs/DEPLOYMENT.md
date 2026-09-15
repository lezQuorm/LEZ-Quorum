# Deployment

## Testnet

| Field | Value |
|---|---|
| LEZ | `v0.2.2` at `d6e4ae694e7419f5906b340c232704466a1917b7` |
| RPC | `https://testnet.lez.logos.co` |
| Explorer | `https://explorer.testnet.lez.logos.co` |
| Network ID | `0101010101010101010101010101010101010101010101010101010101010101` |
| Gate program | `f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec` |
| Gate ELF SHA-256 | `72351623f9a703c40736ab5645b047d39b3c5b688f2c2c47302cf62d1762fd3b` |
| Threshold ELF SHA-256 | `7533ba0608cf00b1eb8b8b57d259d3594ff1d886acc33e9696ae57726ee951df` |

## Verified Demo Lifecycle

The current evidence is the 2-of-3 Basecamp session verified on **15 September
2026**. Its shared gate deployment is confirmed at block **4024**. The nine
session transactions run from initialization at block **9021** to execution
at block **9544**, with two private approvals at blocks **9100** and **9176**.

The final state is approvals **2/2**, proposal **Executed**, vault **500**, and
recipient **250** after funding **750** and transferring **250** token units.
See [Demo Testnet Evidence](DEMO_TESTNET_EVIDENCE.md) for all ten transaction
hashes, account IDs, block links, timestamps, receipt checks, and video mapping.

- [Basecamp demo](https://www.youtube.com/watch?v=m65kwds8LOc): setup, treasury writes, and the start of approval proving.
- [CLI verification demo](https://www.youtube.com/watch?v=zBWPmJSlVj8): checking the completed session's transactions and final state.

## Historical Evidence

An [earlier deployment snapshot](https://github.com/lezQuorm/LEZ-Quorum/blob/34c280eef551619b7cf63da6ff154f40a4eab1a5/docs/DEPLOYMENT.md)
preserves the previous treasury accounts and lifecycle at blocks 2359–2547.
That snapshot listed the shared deployment hash at block 693; the later
verification and current demo place it at block 4024. Use the current demo
evidence for this submission. The historical numbers are retained in that
versioned record and must not be mixed with the demo session.

## Build

```bash
cargo build --release -p quorum-cli
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
```

Verify that the committed program artifacts match the deployment record:

```bash
sha256sum \
  guests/quorum-threshold/artifacts/threshold.bin \
  programs/quorum-gate/artifacts/quorum_gate.bin
```

Expected output:

```text
7533ba0608cf00b1eb8b8b57d259d3594ff1d886acc33e9696ae57726ee951df  guests/quorum-threshold/artifacts/threshold.bin
72351623f9a703c40736ab5645b047d39b3c5b688f2c2c47302cf62d1762fd3b  programs/quorum-gate/artifacts/quorum_gate.bin
```

## Testnet Commands

Read network state:

```bash
env -u RISC0_DEV_MODE target/release/quorum network --target testnet health
env -u RISC0_DEV_MODE target/release/quorum network --target testnet deployment
env -u RISC0_DEV_MODE target/release/quorum network --target testnet status
env -u RISC0_DEV_MODE target/release/quorum network --target testnet reconcile
```

Write operations run in this order:

```text
prepare -> initialize -> create-token -> initialize-recipient
        -> initialize-vault -> fund -> propose
        -> approve member 0 -> approve member 1 -> execute
```

Each write prints its transaction hash before submission. Repeat the command
with `--confirm-public-write` to submit it.

Testnet state is stored in `.quorum-testnet/`. Keep that directory, member
files, claims, passwords, and recovery phrases private.

The deployed gate can be checked without writing:

```bash
env -u RISC0_DEV_MODE target/release/quorum network \
  --target testnet deployment \
  --transaction 4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43
```

This confirms the recorded transaction against the testnet RPC. It does not
rebuild or redeploy the program. A new deployment must publish a new program
ID, artifact hashes, transaction, and lifecycle evidence.

## Local Sequencer

```bash
LEZ_REPO=../logos-execution-zone-v022 ./scripts/sequencer-e2e.sh
```

For a faster development run:

```bash
RISC0_DEV_MODE=1 LEZ_REPO=../../logos-execution-zone-v022 \
  ./scripts/sequencer-e2e.sh
```

The script rejects any LEZ checkout other than commit
`d6e4ae694e7419f5906b340c232704466a1917b7`. With no
`RISC0_DEV_MODE` override it uses real proofs, checks
`proof_mode=real`, and allows four hours. Development mode is the CI-fast path
and reports `proof_mode=development`.

## Reproduce From A Clean Checkout

```bash
git clone --branch v0.2.2 --depth 1 \
  https://github.com/logos-blockchain/logos-execution-zone.git \
  logos-execution-zone-v022
git clone https://github.com/lezQuorm/LEZ-Quorum.git
cd LEZ-Quorum
git checkout --detach 456ece6524ca1b651ff9fd3ed31f416b7038df61

cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
LEZ_REPO=../logos-execution-zone-v022 ./scripts/sequencer-e2e.sh
```

The last command is intentionally the real-proof path. For a short functional
rehearsal, prefix it with `RISC0_DEV_MODE=1`.

## CI Evidence

[CI run 34942595687](https://github.com/lezQuorm/LEZ-Quorum/actions/runs/34942595687)
passed on published commit
[`456ece6524ca1b651ff9fd3ed31f416b7038df61`](https://github.com/lezQuorm/LEZ-Quorum/commit/456ece6524ca1b651ff9fd3ed31f416b7038df61).
Its jobs cover formatting, strict Clippy, workspace tests, the pinned standalone
sequencer lifecycle, and a real 2-of-3 threshold proof. The standalone CI job
uses development receipts for runtime; the separate real-proof job supplies
the cryptographic evidence. These modes are labeled independently so a fast
integration test cannot be mistaken for a production proof.

Local revalidation for 0.1.1 on 2026-09-10 also passed the complete 104-test
development-mode suite and both development- and real-proof standalone
sequencer lifecycles. The real run explicitly reported `proof_mode=real`,
confirmed the private approval and execution, and ended with `RESULT=PASS` and
`SEQUENCER_E2E=PASS` after `5620` seconds. A separate real 2-of-3 threshold
receipt also passed host verification. See [Benchmarks](BENCHMARKS.md) for the
local proof measurements.

See [Benchmarks](BENCHMARKS.md) for proof time and compute units and
[Integration](INTEGRATION.md) for the full two-step network command sequence.
