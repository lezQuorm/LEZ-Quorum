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

## Accounts

| Account | ID |
|---|---|
| Multisig | `Public/Dhmdgo7ggkAyGgR6nnWJGKsVt4KyTSTeJVNJWP2LXwbK` |
| Token definition | `Public/4zDx88PpQodhCCozBHzjqDWmZETUw41GXTtkTPtyWRxM` |
| Token supply | `Public/BXyX7e1abdBYR51dyqQa9gCn24RfuyayG4oHf2wkmK8e` |
| Recipient | `Public/3KxvbW6wV6ffDUNJu2dQAU3cJ5YZtonhh7Aoh17zqHH3` |
| Vault | `Public/6idoC223kScRDbCtP64AMYBXh89RtttdpBxuW7AjkbNX` |
| Proposal | `Public/942Td58tiJAseykKsXGLe2HgueWBn84z7D89R1gYBEi2` |

## Transactions

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

| Final state | Value |
|---|---:|
| Approvals | `2 / 2` |
| Vault balance | `500` |
| Recipient balance | `250` |
| Proposal status | `Executed` |

## Build

```bash
cargo build --release -p quorum-cli
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
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

## Local Sequencer

```bash
LEZ_REPO=../../logos-execution-zone-v022 ./scripts/sequencer-e2e.sh
```

For a faster development run:

```bash
RISC0_DEV_MODE=1 LEZ_REPO=../../logos-execution-zone-v022 \
  ./scripts/sequencer-e2e.sh
```
