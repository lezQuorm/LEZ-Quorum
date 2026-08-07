# Deployment

## Target

| Field | Value |
|---|---|
| LEZ | `v0.2.2` at `d6e4ae694e7419f5906b340c232704466a1917b7` |
| RPC | `https://testnet.lez.logos.co` |
| Explorer | `https://explorer.testnet.lez.logos.co` |
| Gate program | `f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec` |
| Deployment transaction | `4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43` |
| Current block | `845` when checked on 2026-08-07 |

The gate was recorded at block `693` before a testnet reset. Its transaction is
no longer present. The exact deployment is prepared locally, but no new public
transaction has been submitted.

## Build

Use Rust 1.91 or newer and Risc0 3.0.5:

```bash
cargo build --release -p quorum-cli
```

The operator binary is `target/release/quorum`. Testnet commands reject
`RISC0_DEV_MODE=1`.

## Preflight

Run these read-only commands from the repository root:

```bash
env -u RISC0_DEV_MODE target/release/quorum network --target testnet health
env -u RISC0_DEV_MODE target/release/quorum network --target testnet deployment
```

Health currently succeeds. Deployment currently reports the recorded
transaction absent, so the guarded `deploy` step is required.

## Prepare

```bash
env -u RISC0_DEV_MODE target/release/quorum network --target testnet prepare \
  --threshold 2 --members 3 --funding 750 --transfer 250
env -u RISC0_DEV_MODE target/release/quorum network --target testnet status
```

This creates `.quorum-testnet/`. Secret files use mode `0600`; directories use
mode `0700`. Back up that directory privately before writing to the network.

## Public Lifecycle

Each write is a two-step operation. The first command creates and journals the
exact transaction without sending it:

```bash
env -u RISC0_DEV_MODE target/release/quorum network --target testnet initialize
```

Review the printed hash, then repeat the same command with the approval flag:

```bash
env -u RISC0_DEV_MODE target/release/quorum network --target testnet initialize \
  --confirm-public-write
```

Use the same prepare, review, and submit pattern in this order:

```text
deploy                         only if the pinned deployment is absent
initialize
create-token
initialize-recipient
initialize-vault
fund
propose
approve --member 0 --proposal 0
status
approve --member 1 --proposal 0
status
execute --proposal 0
status
```

The approval steps generate real private proofs and can take hours on a
memory-constrained machine. Do not expose `.quorum-testnet/`, member material,
claims, passwords, or recovery phrases.

Expected final state:

```text
vault_balance=500
recipient_balance=250
proposal_status=Executed
RESULT=PASS
```

## Recovery

After a timeout, query the saved hash before doing anything else:

```bash
env -u RISC0_DEV_MODE target/release/quorum network --target testnet reconcile
```

Resubmit only when reconciliation reports the transaction absent. Name the
journal entry and require a fresh approval:

```bash
env -u RISC0_DEV_MODE target/release/quorum network --target testnet reconcile \
  --label initialize --resubmit-unconfirmed --confirm-public-write
```

This sends the exact saved payload. Never rebuild an unknown transaction.

## Local Rehearsal

Start an LEZ v0.2.2 sequencer:

```bash
cd ../logos-execution-zone-v022
RISC0_DEV_MODE=1 just run-sequencer-standalone
```

From this repository, in a second terminal:

```bash
RISC0_DEV_MODE=1 cargo run -p quorum-composer --features network \
  --example local_lez_e2e -- http://127.0.0.1:3040 91
```

The same lifecycle also passed through `quorum network --target local` with two
separate approvals. Development receipts are for rehearsal only.

## Verification

```bash
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
```

Build the Basecamp packages after installing Nix:

```bash
cd apps/basecamp-quorum
nix build .#generate .#lib .#lgx .#lgx-portable
```
