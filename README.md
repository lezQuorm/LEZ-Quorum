# Quorum

Quorum is a private threshold treasury for the Logos Execution Zone (LEZ). An
M-of-N group can authorize transfers, membership rotations, and threshold
changes without publishing its member list or attributing approvals to named
members.

Quorum is an experimental implementation, not a production treasury. The
credential-aware Risc0 circuit, SPEL gate, recursive private transaction
composer, local operator workflow, generated IDL, and Basecamp module source
are implemented and tested locally. The gate has not been deployed to an LEZ
testnet, the Basecamp package has not been built in this environment, and the
code has not received an independent audit.

## What Quorum provides

| Capability | Implementation |
|---|---|
| Private membership | A constitution stores one Merkle root over LEZ private-account credentials |
| Anonymous approvals | Proof artifacts use proposal-scoped nullifiers and credential bindings, not member secrets, account IDs, or Merkle paths |
| Credential control | The threshold proof binds enrollment to an LEZ private account; the outer LEZ privacy proof proves control of that same account |
| Threshold policy | Default and per-tier thresholds with transfer amount caps |
| Private transaction composition | A verified threshold receipt is attached to the gate execution and recursively wrapped by the LEZ privacy circuit |
| Treasury execution | The gate validates the vault PDA and approved recipient before emitting a token transfer chained call |
| Membership rotation | A threshold-approved root change retires the previous member set atomically |
| Client contract | Complete IDL, generated-client validation, and instruction codec round-trip tests |
| Operator surfaces | Offline CLI plus a native Basecamp QProcess module source |

## Protocol flow

1. Operators create a constitution with a threshold, member count, credential
   Merkle root, and optional spending tiers.
2. A proposal is bound to its multisig account and constitution version.
3. Members prove membership of LEZ private-account credentials and derive
   distinct proposal-bound nullifiers in the Risc0 guest.
4. The composer verifies the receipt locally, binds its journal to an approve
   instruction, and attaches the receipt to the gate proof as an assumption.
5. The LEZ privacy circuit verifies the gate proof and the private account
   identities, then emits encrypted credential post-states and public gate
   state updates.
6. Once enough nullifiers have accumulated, the gate applies governance state
   or executes the treasury transfer.

Credential account IDs remain private transaction inputs. Proposal content,
policy, approval count, Quorum nullifiers, and governance changes are public by
design.

## Quick start

Requirements: Rust 1.91 or newer and the Risc0 toolchain used by the workspace.

```bash
cargo build -p quorum-cli
RISC0_DEV_MODE=1 ./scripts/demo.sh
```

The demo creates a 2-of-3 treasury, proves an aggregated transfer approval,
executes it in the local mirror, rotates the member set, rejects an old key,
activates the replacement bundle, and uses the replacement keys.

```bash
quorum create \
  --threshold 2 \
  --members 3 \
  --tiers '[{"id":1,"threshold":2,"max_amount":1000}]'

quorum propose \
  --action transfer \
  --recipient <64-hex-account-id> \
  --amount 500 \
  --tier 1

quorum approve-all --proposal 0 --members 0,1
quorum execute --proposal 0
```

Dev mode creates mock receipts and must not be used for deployment evidence.
Generate and verify a real succinct threshold receipt with:

```bash
RISC0_DEV_MODE=0 \
  cargo run -p quorum-prover --example prove_threshold --release
```

## Repository map

| Path | Purpose |
|---|---|
| `crates/quorum-core` | Merkle membership, credential commitments, nullifiers, and policy validation |
| `crates/quorum-circuit` | Pure credential-aware threshold statement |
| `crates/quorum-prover` | Host proving, artifact encoding, and pinned-image verification |
| `crates/quorum-gate-core` | Constitution, proposal, claim, credential, and execution rules |
| `crates/quorum-composer` | LEZ private transaction composition, RPC submission, confirmation, and public state reads |
| `crates/quorum-sdk` | Member, proposal, proof, rotation, and local state APIs |
| `crates/quorum-cli` | Offline operator workflow and protected key files |
| `crates/lez-compat` | LEZ v0.3 private account ID and account commitment compatibility |
| `guests/quorum-threshold` | Embedded Risc0 threshold guest |
| `programs/quorum-gate` | SPEL program, complete IDL, and IDL generator |
| `apps/basecamp-quorum` | Basecamp QML view, native backend, and module packaging |

## Development

```bash
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-features
```

Regenerate the gate IDL after changing an instruction, account, or custom type:

```bash
cargo run -p quorum-gate-methods --example generate_idl
```

The CLI writes `quorum.json`, `member-N.json`, `rotation.json`, and proof
artifacts with mode 0600. These files contain operational state or secrets and
must not be committed.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Circuit design](docs/CIRCUIT_DESIGN.md)
- [Privacy model](docs/PRIVACY_MODEL.md)
- [Security assumptions](docs/SECURITY_ASSUMPTIONS.md)
- [Known limitations](docs/KNOWN_LIMITATIONS.md)
- [Deployment and integration](docs/DEPLOYMENT.md)
- [Benchmarks](docs/BENCHMARKS.md)
- [Error codes](docs/ERROR_CODES.md)

## License

Licensed under either Apache-2.0 or MIT, at your option.
