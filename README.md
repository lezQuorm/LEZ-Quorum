# Quorum

[![CI](https://github.com/lezQuorm/LEZ-Quorum/actions/workflows/ci.yml/badge.svg)](https://github.com/lezQuorm/LEZ-Quorum/actions/workflows/ci.yml)

Quorum is a private M-of-N treasury for Logos Execution Zone (LEZ) v0.2.2.
Members authorize transfers, member rotation, and threshold changes without
publishing the member list or linking a public approval to a named member.

The deployed design combines two proofs:

1. A Quorum Risc0 receipt proves that one or more distinct private credentials
   belong to the committed member set and emits proposal-scoped nullifiers.
2. The outer LEZ privacy proof authorizes the private credential accounts used
   by that receipt and carries the gate call to the sequencer.

The SPEL gate binds both layers, records distinct nullifiers, and executes the
approved action when the proposal threshold is met.

## Demos

- [Basecamp demo](https://www.youtube.com/watch?v=m65kwds8LOc): setup, treasury
  transactions, and the start of real approval proof generation.
- [CLI verification demo](https://www.youtube.com/watch?v=zBWPmJSlVj8): checking
  the completed session's confirmed transactions, approvals, and final balances
  with `quorum network --target testnet status`.

Both approval confirmations and execution occurred after the Basecamp recording
ended. The CLI walkthrough checks the completed state, and the
[verified demo testnet evidence](docs/DEMO_TESTNET_EVIDENCE.md) documents the
transaction flow for the same session.

## Testnet Result

| Field | Value |
|---|---|
| RPC | `https://testnet.lez.logos.co` |
| LEZ | `v0.2.2` at `d6e4ae694e7419f5906b340c232704466a1917b7` |
| Gate program | `f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec` |
| Deployment | [`4635b013...c24b43`](https://explorer.testnet.lez.logos.co/transaction/4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43) |
| Completed lifecycle | Approvals `2/2`, vault `500`, recipient `250`, proposal `Executed` |

[Demo testnet evidence](docs/DEMO_TESTNET_EVIDENCE.md) records the accounts,
transaction hashes, blocks, and final state for the recorded session.
[Deployment](docs/DEPLOYMENT.md) provides artifact verification, reproduction
instructions, and a separate historical lifecycle record.

## What Is Private

| Private witness or local state | Public protocol data |
|---|---|
| Member nullifier secret keys | Member Merkle root and count |
| Member private account IDs | Threshold and spending policies |
| Viewing keys and account identifiers | Proposal action and status |
| Member list and Merkle paths | Approval count and nullifiers |
| Which named person approved | Constitution and proposal versions |

The design does not hide that a proposal exists, what it does, how many
approvals it has, or when approvals arrive. See the full
[privacy model](docs/PRIVACY_MODEL.md).

## Prerequisites

- Rust `1.91.0` with `rustfmt` and `clippy`
- Risc0 `cargo-risczero 3.0.5` and guest Rust `1.97.0`
- Nix with flakes enabled for Basecamp packaging
- A checkout of LEZ v0.2.2 for the standalone sequencer test

The exact Rust and Risc0 versions are also pinned in CI.

## Build And Verify

```bash
cargo build --release -p quorum-cli
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
```

`RISC0_DEV_MODE=1` creates development receipts that are suitable only for
tests and rehearsal. Omit the variable or set it to `0` for real succinct
proofs. The network CLI rejects development mode when targeting testnet.

Generate and verify a real 2-of-3 receipt:

```bash
RISC0_DEV_MODE=0 cargo run --release -p quorum-prover --example prove_threshold
```

## Local CLI Demo

The fast state-machine demonstration exercises transfer, member rotation, an
old-member rejection, and activation of the new member bundle:

```bash
RISC0_DEV_MODE=1 ./scripts/demo.sh
```

To exercise the actual SPEL program against a developer-owned LEZ sequencer,
first check out the pinned LEZ release:

```bash
git clone --branch v0.2.2 --depth 1 \
  https://github.com/logos-blockchain/logos-execution-zone.git \
  ../logos-execution-zone-v022
RISC0_DEV_MODE=1 LEZ_REPO=../logos-execution-zone-v022 \
  ./scripts/sequencer-e2e.sh
```

Remove `RISC0_DEV_MODE=1` for the real-proof path. A successful run ends with:

```text
vault_balance=500
recipient_balance=250
proposal_status=Executed
RESULT=PASS
```

## Network CLI

Every write is prepared and hashed before submission. Run a command once to
inspect the transaction, then repeat it with `--confirm-public-write` to submit
the exact prepared bytes. The standard flow is:

```text
health -> deployment -> prepare -> initialize -> create-token
       -> initialize-recipient -> initialize-vault -> fund -> propose
       -> approve-threshold -> approve-threshold -> execute -> reconcile
```

For a fresh session against a running local sequencer, start at the repository
root and deploy the gate before initializing:

```bash
Q="$(realpath target/release/quorum)"
QUORUM_SESSION="$(mktemp -d "${TMPDIR:-/tmp}/quorum-network.XXXXXX")"
cd "$QUORUM_SESSION" || exit 1
"$Q" network --target local health
"$Q" network --target local prepare
"$Q" network --target local deploy
"$Q" network --target local deploy --confirm-public-write
"$Q" network --target local initialize
"$Q" network --target local initialize --confirm-public-write
"$Q" network --target local status
```

Use `--target testnet` only with real proofs and after reviewing the exact
network, program, accounts, and hashes in [Deployment](docs/DEPLOYMENT.md).
Private material is written under `.quorum-network-local/` or `.quorum-testnet/` and
must not be committed or shared. The complete command guide is in
[Integration](docs/INTEGRATION.md).

## Basecamp Module

The QML module invokes the same release CLI and supports `Local` and
`LEZ Testnet` targets. Download the Linux amd64 LGX packages and matching CLI from the
[Quorum v0.1.1 release](https://github.com/lezQuorm/LEZ-Quorum/releases/tag/v0.1.1).
Follow [Download And Load](docs/BASECAMP_RELEASE.md#download-and-load) to verify
the checksums, import the portable LGX, and select the CLI in Runtime settings.

For source builds and the complete GUI lifecycle, use the
[Basecamp Guide](docs/BASECAMP_GUIDE.md). The LGX contains the QML module; the
CLI is installed separately. Real approvals require the Risc0 prover runtime.

## Workspace

| Path | Responsibility |
|---|---|
| `crates/quorum-core` | Pure policy model, commitments, nullifiers, and Merkle tree |
| `crates/lez-compat` | LEZ v0.2.2 account encodings and derivations |
| `crates/quorum-circuit` | Threshold statement and public journal |
| `crates/quorum-prover` | Real or development receipt generation |
| `crates/quorum-gate-core` | Runtime state, claim checks, and action application |
| `programs/quorum-gate` | SPEL guest, generated methods, and IDL |
| `crates/quorum-composer` | LEZ proof composition, RPC, and transactions |
| `crates/quorum-sdk` | Client state and high-level API |
| `crates/quorum-cli` | Local and network workflows with transaction journal |
| `apps/basecamp-quorum` | Basecamp QML frontend and native backend |

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Circuit design](docs/CIRCUIT_DESIGN.md)
- [Privacy model](docs/PRIVACY_MODEL.md)
- [Security assumptions](docs/SECURITY_ASSUMPTIONS.md)
- [Known limitations](docs/KNOWN_LIMITATIONS.md)
- [Integration guide](docs/INTEGRATION.md)
- [Running Quorum in Basecamp](docs/BASECAMP_GUIDE.md)
- [Basecamp release manifest](docs/BASECAMP_RELEASE.md)
- [Deployment evidence](docs/DEPLOYMENT.md)
- [Demo testnet evidence](docs/DEMO_TESTNET_EVIDENCE.md)
- [Benchmarks](docs/BENCHMARKS.md)
- [Error codes](docs/ERROR_CODES.md)
- [Technical reference](docs/REFERENCE.md)
- [Success-criteria evidence](docs/CRITERIA.md)
- [Architecture decisions](docs/adr/README.md)

Quorum is experimental, has not received an independent security audit, and
must not custody assets of material value. Review
[Security Assumptions](docs/SECURITY_ASSUMPTIONS.md) before deployment.

## License

MIT OR Apache-2.0.
