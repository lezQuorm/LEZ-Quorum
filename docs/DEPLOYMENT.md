# Deployment

## Public Testnet

| Field | Value |
|---|---|
| LEZ | `v0.2.2` at `d6e4ae694e7419f5906b340c232704466a1917b7` |
| RPC | `https://testnet.lez.logos.co` |
| Explorer | `https://explorer.testnet.lez.logos.co` |
| Network ID | `0101010101010101010101010101010101010101010101010101010101010101` |
| Gate program | `f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec` |
| Deployment | [`4635b013...c24b43`](https://explorer.testnet.lez.logos.co/transaction/4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43) at [block `693`](https://explorer.testnet.lez.logos.co/block/693) |

The sequencer's direct transaction index no longer returns the old deployment.
Quorum reads retained block `693` and matches the deployed bytecode exactly.

## Verified Lifecycle

The lifecycle completed on 2026-08-10 with `RISC0_DEV_MODE` unset. Each saved
transaction was reconciled against the live sequencer's exact transaction bytes.
Every linked transaction and block page was then checked on the official
explorer after its index reached block `2547`.

| Account | ID |
|---|---|
| Multisig | `Public/Dhmdgo7ggkAyGgR6nnWJGKsVt4KyTSTeJVNJWP2LXwbK` |
| Token definition | `Public/4zDx88PpQodhCCozBHzjqDWmZETUw41GXTtkTPtyWRxM` |
| Token supply | `Public/BXyX7e1abdBYR51dyqQa9gCn24RfuyayG4oHf2wkmK8e` |
| Recipient | `Public/3KxvbW6wV6ffDUNJu2dQAU3cJ5YZtonhh7Aoh17zqHH3` |
| Vault | `Public/6idoC223kScRDbCtP64AMYBXh89RtttdpBxuW7AjkbNX` |
| Proposal | `Public/942Td58tiJAseykKsXGLe2HgueWBn84z7D89R1gYBEi2` |

| Operation | Transaction | Block |
|---|---|---:|
| Deploy gate | [`4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43`](https://explorer.testnet.lez.logos.co/transaction/4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43) | [`693`](https://explorer.testnet.lez.logos.co/block/693) |
| Initialize constitution | [`81f9bd58913d5cb160d221810362fcb220dd5672c8ac51b0f71fc96ffaeeb73f`](https://explorer.testnet.lez.logos.co/transaction/81f9bd58913d5cb160d221810362fcb220dd5672c8ac51b0f71fc96ffaeeb73f) | [`2359`](https://explorer.testnet.lez.logos.co/block/2359) |
| Create token | [`2b6b7487aabe20f078991904243771f6cd19410e4a83ae0600a99504944f0962`](https://explorer.testnet.lez.logos.co/transaction/2b6b7487aabe20f078991904243771f6cd19410e4a83ae0600a99504944f0962) | [`2361`](https://explorer.testnet.lez.logos.co/block/2361) |
| Initialize recipient | [`26822f43243542613417307ccdfb5630c3e4923be4cd1fb995ecd6e2cfca6d51`](https://explorer.testnet.lez.logos.co/transaction/26822f43243542613417307ccdfb5630c3e4923be4cd1fb995ecd6e2cfca6d51) | [`2363`](https://explorer.testnet.lez.logos.co/block/2363) |
| Initialize vault | [`cc542de2db8394b6adb500de74d53963610872836daa6df58cd181a4f6f019a3`](https://explorer.testnet.lez.logos.co/transaction/cc542de2db8394b6adb500de74d53963610872836daa6df58cd181a4f6f019a3) | [`2364`](https://explorer.testnet.lez.logos.co/block/2364) |
| Fund vault | [`e39c83c3f5ccb66eda322881bddb4a0c308c02f234131bde9fc9b089c73f22ee`](https://explorer.testnet.lez.logos.co/transaction/e39c83c3f5ccb66eda322881bddb4a0c308c02f234131bde9fc9b089c73f22ee) | [`2365`](https://explorer.testnet.lez.logos.co/block/2365) |
| Propose transfer | [`1aabb2115e8d673412a10f4db476cc59aa6d3fe8b1017481035d6734a066dbd8`](https://explorer.testnet.lez.logos.co/transaction/1aabb2115e8d673412a10f4db476cc59aa6d3fe8b1017481035d6734a066dbd8) | [`2366`](https://explorer.testnet.lez.logos.co/block/2366) |
| Private approval 1 | [`203e7d9ddcf16f9206f0cabdede6255af1d06749aa7f8f02ce0c1896a37c8fd5`](https://explorer.testnet.lez.logos.co/transaction/203e7d9ddcf16f9206f0cabdede6255af1d06749aa7f8f02ce0c1896a37c8fd5) | [`2456`](https://explorer.testnet.lez.logos.co/block/2456) |
| Private approval 2 | [`40c9ce28b6bea114d84a7fd4642f564a80c8df12f1cd733d073138359c00e548`](https://explorer.testnet.lez.logos.co/transaction/40c9ce28b6bea114d84a7fd4642f564a80c8df12f1cd733d073138359c00e548) | [`2545`](https://explorer.testnet.lez.logos.co/block/2545) |
| Execute transfer | [`21dcc74c864b0678e2b4259c56908352c6917d2359204e3215247bc177b6c7e9`](https://explorer.testnet.lez.logos.co/transaction/21dcc74c864b0678e2b4259c56908352c6917d2359204e3215247bc177b6c7e9) | [`2547`](https://explorer.testnet.lez.logos.co/block/2547) |

The two real approval previews completed in `1:26:58` and `1:27:30`.

| Final state | Value |
|---|---:|
| Approvals | `2 / 2` |
| Vault balance | `500` |
| Recipient balance | `250` |
| Proposal status | `Executed` |

## Evidence Rules

Publish a lifecycle only when all of these checks pass:

1. Every command reports `transaction_status=confirmed` and a block.
2. `getTransaction` returns the exact transaction after the lifecycle ends.
3. The explorer page loads for each submitted hash.
4. Public reads show two approvals, vault `500`, recipient `250`, and proposal
   `Executed`.
5. The same checks still pass immediately before submission.

If any hash disappears, discard the session and create a fresh one. Never
describe explorer failure as indexing lag without independent current evidence.

## Build And Verify

Use Rust 1.91 or newer and Risc0 3.0.5:

```bash
cargo build --release -p quorum-cli
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
RISC0_DEV_MODE=1 cargo run --release -p quorum-composer \
  --example compute_units
```

Read the public state without exposing local secrets:

```bash
env -u RISC0_DEV_MODE target/release/quorum network --target testnet health
env -u RISC0_DEV_MODE target/release/quorum network --target testnet deployment
env -u RISC0_DEV_MODE target/release/quorum network --target testnet status
env -u RISC0_DEV_MODE target/release/quorum network --target testnet reconcile
```

## Guarded Writes

`prepare` creates protected state in `.quorum-testnet/`. Each write is first
journaled without submission. Review its hash, then repeat the same command
with `--confirm-public-write`.

```text
prepare -> initialize -> create-token -> initialize-recipient
        -> initialize-vault -> fund -> propose
        -> approve member 0 -> approve member 1 -> execute
```

Real proof duration depends on CPU and memory. Never publish `.quorum-testnet/`,
member material, claims, passwords, or recovery phrases.

After a timeout, run `network --target testnet reconcile`. A previously
confirmed hash that is missing becomes `Orphaned` and cannot be resubmitted.
Start a fresh session. Only an unchanged transaction that was never confirmed
is eligible for exact resubmission.

## Local Rehearsal

```bash
# Real proofs
LEZ_REPO=../../logos-execution-zone-v022 ./scripts/sequencer-e2e.sh

# Fast rehearsal only
RISC0_DEV_MODE=1 LEZ_REPO=../../logos-execution-zone-v022 \
  ./scripts/sequencer-e2e.sh
```
