# Quorum

Quorum is a private M-of-N treasury for Logos Execution Zone (LEZ). Members
approve transfers, rotations, and threshold changes without publishing the
member list or linking approvals to named members.

The project targets LEZ v0.2.2. It is experimental and has not received an
independent security audit.

## Public testnet

| Field | Value |
|---|---|
| RPC | `https://testnet.lez.logos.co` |
| LEZ | `v0.2.2` at `d6e4ae694e7419f5906b340c232704466a1917b7` |
| Gate program | `f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec` |
| Deployment transaction | `4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43` |
| Deployment status | Prepared locally; the prior testnet record is no longer present |
| RPC status | Healthy at block `845` on 2026-08-07 |

The testnet has reset since the earlier block `693` deployment. The full
treasury lifecycle has passed locally but has not been broadcast to public
testnet. See [Deployment](docs/DEPLOYMENT.md) for current verification and
operator commands.

## Protocol

1. A constitution commits the member set as one Merkle root.
2. A proposal binds its action to the constitution version.
3. Members prove credential control and Merkle membership in Risc0.
4. Proposal-bound nullifiers prevent duplicate approvals.
5. The SPEL gate enforces threshold, tier, vault, recipient, and state rules.
6. The LEZ privacy circuit authorizes private credential updates.

Private inputs include member secrets, account IDs, and Merkle paths. Public
state includes policy, proposal content, approval count, nullifiers, and
governance changes.

## Quick start

Requirements: Rust 1.91 or newer and the Risc0 3.0.5 toolchain.

```bash
cargo build -p quorum-cli
RISC0_DEV_MODE=1 ./scripts/demo.sh
```

The local flow creates a 2-of-3 treasury, executes a transfer, rotates the
member set, rejects a retired credential, and activates the replacement set.
Development receipts are not cryptographic evidence.

## LEZ lifecycle

Start an LEZ v0.2.2 standalone sequencer:

```bash
cd ../logos-execution-zone-v022
RISC0_DEV_MODE=1 just run-sequencer-standalone
```

Run Quorum from a second terminal:

```bash
RISC0_DEV_MODE=1 cargo run -p quorum-composer --features network \
  --example local_lez_e2e -- http://127.0.0.1:3040 91
```

Success ends with vault balance `500`, recipient balance `250`, proposal status
`Executed`, and `RESULT=PASS`. Unset `RISC0_DEV_MODE` for real proofs.

The guarded operator workflow uses isolated state and two member approvals:

```bash
quorum network --target local prepare
quorum network --target local status
quorum network --target testnet health
```

Every network write is prepared and journaled first. Submission requires a
second invocation with `--confirm-public-write`.

## Basecamp

```bash
cargo build --release -p quorum-cli
cd apps/basecamp-quorum
nix build .#generate .#lib .#lgx .#lgx-portable
```

The module provides `Local` and `LEZ Testnet` modes. Testnet submission requires
a single-use confirmation in the interface. The build produces native and
portable LGX packages.

## Workspace

| Path | Purpose |
|---|---|
| `crates/quorum-core` | Treasury domain rules |
| `crates/lez-compat` | LEZ v0.2.2 account compatibility |
| `crates/quorum-circuit` | Threshold statement |
| `crates/quorum-prover` | Receipt generation and verification |
| `crates/quorum-gate-core` | Gate state and policy |
| `programs/quorum-gate` | SPEL guest and IDL |
| `crates/quorum-composer` | Private LEZ transaction composition |
| `crates/quorum-sdk` | Client API |
| `crates/quorum-cli` | Local and sequencer operator CLI |
| `apps/basecamp-quorum` | Basecamp module |

## Development

```bash
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
```

Generated interfaces:

```bash
./scripts/update-image-id.sh
cargo run -p quorum-gate-methods --example generate_idl
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Deployment](docs/DEPLOYMENT.md)
- [Testnet readiness tasks](docs/TASKS.md)

## License

MIT or Apache-2.0.
