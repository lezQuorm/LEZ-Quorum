# Deployment

## Versions

| Dependency | Pinned version |
|---|---|
| LEZ | `v0.2.2`, commit `d6e4ae694e7419f5906b340c232704466a1917b7` |
| SPEL compatibility | commit `1fef85203c3130676a49aaed1b4387d16be9fe94` |
| Risc0 | `3.0.5` |
| Rust | `1.91` or newer |

## Public testnet

The Quorum gate is deployed on the public LEZ testnet.

| Field | Value |
|---|---|
| RPC | `https://testnet.lez.logos.co` |
| Explorer | `https://explorer.testnet.lez.logos.co` |
| Program ID | `f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec` |
| Program ID words | `[320098040, 1020072060, 2381930866, 4243020391, 4177030334, 802000452, 1921768834, 3969437236]` |
| Deployment transaction | `4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43` |
| Confirmation | Block `693`, 2026-08-06 |

This confirms program deployment only. The full funded treasury lifecycle has
passed locally with real proofs but has not been broadcast to the public
testnet. Testnet state can reset, and the explorer may lag the sequencer.

Check the deployment directly:

```bash
curl -sS https://testnet.lez.logos.co \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"getTransaction","params":["4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43"]}' \
  | jq '.result[1]'
```

The expected result is `693`. A `null` result means the testnet reset.

## Deploy the gate

Install the Risc0 toolchain and keep an LEZ v0.2.2 checkout beside this
repository:

```bash
git clone --branch v0.2.2 --depth 1 \
  https://github.com/logos-blockchain/logos-execution-zone.git \
  ../logos-execution-zone-v022
```

Build the transaction without submitting it:

```bash
env -u RISC0_DEV_MODE cargo run -p quorum-composer --features network \
  --example deploy_gate -- https://testnet.lez.logos.co --dry-run
```

Submit and wait for confirmation:

```bash
env -u RISC0_DEV_MODE cargo run -p quorum-composer --features network \
  --example deploy_gate -- https://testnet.lez.logos.co
```

LEZ v0.2.2 program deployments contain bytecode without a signer or fee payer.
No funded wallet is required for this transaction.

## Funded test account

The official v0.2.2 wallet initialized and funded this public account through
the Pinata proof-of-work claim:

| Field | Value |
|---|---|
| Account | `Public/81yCTY7Sk9h1yjzj5Du4urxxAF5ysLnmnBvtDYaEsUxh` |
| Initialize transaction | `dc995ae3311064981468036810c24f5a315d26cd4718f4cd49e8ff8cc812aae2`, block `690` |
| Pinata transaction | `f276765e4e74f5b0d85901172a1af97c8f2d751962b95db3a3cf7028732e5c41`, block `691` |
| Last verified balance | `150` |

Read the balance without unlocking the wallet:

```bash
curl -sS https://testnet.lez.logos.co \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"getAccountBalance","params":["81yCTY7Sk9h1yjzj5Du4urxxAF5ysLnmnBvtDYaEsUxh"]}' \
  | jq '.result'
```

Create and fund another test account from the LEZ checkout:

```bash
cd ../logos-execution-zone-v022
export LEE_WALLET_HOME_DIR=/home/core/.local/share/lez-wallet-testnet
cargo run --release -p wallet -- check-health
cargo run --release -p wallet -- account new public --label deployer
cargo run --release -p wallet -- auth-transfer init \
  --account-id Public/YOUR_ACCOUNT_ID
cargo run --release -p wallet -- pinata claim \
  --to Public/YOUR_ACCOUNT_ID
```

The wallet prints its recovery phrase once. Keep it outside logs and source
control.

## Local lifecycle

Start the LEZ sequencer:

```bash
cd ../logos-execution-zone-v022
RISC0_DEV_MODE=1 just run-sequencer-standalone
```

Run Quorum from a second terminal with a fresh seed from 1 through 250:

```bash
RISC0_DEV_MODE=1 cargo run -p quorum-composer --features network \
  --example local_lez_e2e -- http://127.0.0.1:3040 91
```

The example deploys the gate, initializes and funds the treasury, proves a
private 2-of-3 approval, executes the transfer, and reads final state. Success
ends with:

```text
vault_balance=500
recipient_balance=250
proposal_status=Executed
RESULT=PASS
```

For real proofs, unset development mode in both terminals:

```bash
env -u RISC0_DEV_MODE just run-sequencer-standalone
env -u RISC0_DEV_MODE cargo run -p quorum-composer --features network \
  --example local_lez_e2e -- http://127.0.0.1:3040 101
```

The verified real lifecycle took about 2 hours 19 minutes on a 15 GiB machine
with 21 GiB swap.

## Validation

```bash
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
```

After a circuit or gate interface change:

```bash
./scripts/update-image-id.sh
cargo run -p quorum-gate-methods --example generate_idl
cargo test -p quorum-gate-methods
```

## Basecamp package

```bash
cargo build --release -p quorum-cli
cd apps/basecamp-quorum
nix build .#generate .#lib
nix build .#lgx --out-link result-lgx
nix build .#lgx-portable --out-link result-lgx-portable
```

The LGX archives are written under `result-lgx` and `result-lgx-portable`.
