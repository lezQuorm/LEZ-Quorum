# Quorum

[![CI](https://github.com/lezQuorm/LEZ-Quorum/actions/workflows/ci.yml/badge.svg)](https://github.com/lezQuorm/LEZ-Quorum/actions/workflows/ci.yml)

Quorum is a private M-of-N treasury for Logos Execution Zone (LEZ). Members
approve transfers and governance changes without publishing the member list or
linking an approval to a named member.

## Demo

[![Watch the Quorum testnet demo](docs/assets/quorum-demo-preview.png)](https://cdn.jsdelivr.net/gh/lezQuorm/LEZ-Quorum@6dcfa74d2310152c04485f8fd72728a462c4e832/docs/assets/quorum-demo.mp4)

[Watch the full demo (11:20, MP4)](https://cdn.jsdelivr.net/gh/lezQuorm/LEZ-Quorum@6dcfa74d2310152c04485f8fd72728a462c4e832/docs/assets/quorum-demo.mp4)

## Testnet

| Field | Value |
|---|---|
| RPC | `https://testnet.lez.logos.co` |
| LEZ | `v0.2.2` at `d6e4ae694e7419f5906b340c232704466a1917b7` |
| Gate program | `f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec` |
| Deployment | [`4635b013...c24b43`](https://explorer.testnet.lez.logos.co/transaction/4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43) |
| Final state | Approvals `2/2`, vault `500`, recipient `250`, proposal `Executed` |

[Deployment evidence](docs/DEPLOYMENT.md) lists every account, transaction, and
block in the public lifecycle.

## Protocol

1. The constitution stores a Merkle root over member credentials.
2. A proposal binds an action to the current constitution version.
3. A member proves credential control and Merkle membership in Risc0.
4. A proposal-scoped nullifier prevents duplicate approval.
5. The SPEL gate applies the action after the required approvals.

Member secrets, account IDs, and Merkle paths stay private. Policy, proposals,
approval counts, nullifiers, and execution results are public.

## Build And Test

Requires Rust 1.91 or newer and Risc0 3.0.5.

```bash
cargo build --release -p quorum-cli
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
```

Run a fast local workflow:

```bash
RISC0_DEV_MODE=1 ./scripts/demo.sh
```

Run the complete lifecycle against LEZ v0.2.2:

```bash
LEZ_REPO=../../logos-execution-zone-v022 ./scripts/sequencer-e2e.sh
```

Use development receipts for a faster rehearsal:

```bash
RISC0_DEV_MODE=1 LEZ_REPO=../../logos-execution-zone-v022 \
  ./scripts/sequencer-e2e.sh
```

A successful lifecycle ends with `vault_balance=500`,
`recipient_balance=250`, `proposal_status=Executed`, and `RESULT=PASS`.

## Basecamp

```bash
cargo build --release -p quorum-cli
cd apps/basecamp-quorum
nix build .#generate .#lib .#lgx .#lgx-portable
```

The module supports `Local` and `LEZ Testnet` modes. Testnet actions follow this
order:

1. Check RPC and prepare private state.
2. Verify the gate and initialize the constitution.
3. Create the token, initialize the recipient and vault, fund the vault, and
   open a proposal.
4. Submit private approvals until the threshold is met.
5. Execute the proposal and check the final state.

Each testnet write is previewed before the submission switch is enabled.

## Workspace

| Path | Purpose |
|---|---|
| `crates/quorum-core` | Treasury rules |
| `crates/quorum-circuit` | Threshold statement |
| `crates/quorum-prover` | Receipt generation and verification |
| `crates/quorum-gate-core` | Gate state and validation |
| `programs/quorum-gate` | SPEL program and IDL |
| `crates/quorum-composer` | LEZ transaction composition and RPC |
| `crates/quorum-sdk` | Client API |
| `crates/quorum-cli` | Local and testnet CLI |
| `apps/basecamp-quorum` | Basecamp QML module |

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Deployment](docs/DEPLOYMENT.md)
- [Technical reference](docs/REFERENCE.md)

Quorum is experimental and has not received an independent security audit.

## License

MIT or Apache-2.0.
