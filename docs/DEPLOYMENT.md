# Deployment

## Public Testnet

| Field | Value |
|---|---|
| LEZ | `v0.2.2` at `d6e4ae694e7419f5906b340c232704466a1917b7` |
| RPC | `https://testnet.lez.logos.co` |
| Explorer | `https://explorer.testnet.lez.logos.co` |
| Network ID | `0101010101010101010101010101010101010101010101010101010101010101` |
| Gate program | `f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec` |
| Deployment | `4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43` at block `693` |

The sequencer's direct transaction index no longer returns the old deployment.
Quorum verifies the exact transaction from retained block `693` and matches its
bytecode before accepting it.

The explorer uses a separate index. At final verification the live sequencer
was at block `1225`, while the explorer stopped at `1158`. It displays the
deployment and first approval, but reports `Transaction not found` for the
second approval at `1163` and execution at `1165`. This is explorer lag; live
account state and RPC reconciliation confirm the completed lifecycle.

## Public Accounts

| Account | ID |
|---|---|
| Multisig | `Public/9VLN1hz8CbrvuDSdWMW35n2mndFuXVCFAmjfgcT62fkE` |
| Token definition | `Public/7fyy8znDTgBWMyCE3CMfv1SKmLkWWm2awC2YqgsRP9r4` |
| Token supply | `Public/8tHUL4qmSS2w1KrsdxTutAgJppqHN9GiY7qYefWmkHA4` |
| Vault | `Public/ArYBK93Gi4cvvdxh21EFxLkr9ifFDf7ambnE1yvGC6B6` |
| Recipient | `Public/HUpTpH9aMY4xDmiLXnL2CZY77uQjBrXUkmqAF7fM2PnT` |
| Proposal | `Public/ETK3s3vHXHwcYANDGGfxkawiXRMXP53d4s2XSqAaFTAR` |

## Confirmed Transactions

| Operation | Transaction | Block |
|---|---|---:|
| Deploy | `4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43` | 693 |
| Initialize | `8d3fd2ad6c4b0fe93017c0a38705c426e027eb5aa0d3a0292ad51c8dcf18a98a` | 937 |
| Create token | `61486adfb9b2d084f55ccd999bb456b27684517bc7bd0bf9b0c1d0744261d369` | 938 |
| Initialize recipient | `423a5f8ae85515f9e950132424a3dd49d313eb23af2663abd1159a87ab70cb09` | 939 |
| Initialize vault | `2a84da42c25e657ab1371eaabfc6adf2e2e4fb6c9a336af39a3a615dc4a86237` | 940 |
| Fund vault | `f18fcf63ce9bb44ffa068e8b6e10e12c8fc52da7bf670f2818aae440c8774fc8` | 941 |
| Propose | `0dedb21fadbed1aef61eb8690c67eab8ce55faab63faab06932a6b80ee72be5e` | 942 |
| Private approval 1 | `9f55acb4e072827ef789c9dda7050b27a3fb4ca221eca6c1cf6c8c2662c0fcb3` | 1078 |
| Private approval 2 | `ff12eef7169bea94d6e7ea5e141a852f1fe37f45f7ffc4526591a0ea29640d5d` | 1163 |
| Execute | `630bcc1d896497ec2868c665119389960cb974d0657610f3b0111d8d50e16b29` | 1165 |

Final state verified on 2026-08-07:

```text
approvals=2
required_approvals=2
vault_balance=500
recipient_balance=250
proposal_status=Executed
RESULT=PASS
```

## Build And Verify

Use Rust 1.91 or newer and Risc0 3.0.5:

```bash
cargo build --release -p quorum-cli
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
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

Real approvals can take more than an hour each on a 15 GiB machine. Never
publish `.quorum-testnet/`, member material, claims, passwords, or recovery
phrases.

After any timeout, run `network --target testnet reconcile`. Resubmit only an
unchanged journaled payload that reconciliation reports absent.

## Local Rehearsal

```bash
cd ../../logos-execution-zone-v022
RISC0_DEV_MODE=1 just run-sequencer-standalone

# In LEZ-Quorum from a second terminal
RISC0_DEV_MODE=1 cargo run -p quorum-composer --features network \
  --example local_lez_e2e -- http://127.0.0.1:3040 91
```

Development receipts are for local rehearsal only.
