# Quorum

Quorum is a private threshold treasury for the Logos Execution Zone (LEZ). It
lets an M-of-N group authorize transfers, membership rotations, and threshold
changes without publishing a member list or attributing approvals to individual
members.

Quorum is currently a research prototype. The local state machine, Risc0
threshold circuit, proof generation, CLI workflow, and SPEL gate compile and
are covered by tests. Live LEZ transaction composition, shielded-account
credential binding, Basecamp packaging, and testnet validation remain in
progress. The code has not been independently audited.

## Product capabilities

| Capability | Current implementation |
|---|---|
| Private membership | The constitution stores one Merkle root over member commitments |
| Anonymous approvals | Proof journals expose proposal-bound nullifiers, not member secrets or leaf positions |
| Threshold policies | A default threshold and per-tier transfer thresholds and caps |
| Membership rotation | A threshold-approved root change retires the previous set atomically |
| Replay protection | Nullifiers bind an approval to a proposal and constitution version |
| Treasury execution | The SPEL gate validates the vault PDA and emits a token transfer chained call |
| Local operations | CLI support for create, propose, approve, execute, rotate, and key activation |

## How it works

1. Operators create a constitution with a threshold, member count, Merkle root,
   and optional spending tiers.
2. A proposal is bound to its multisig account and the current constitution
   version.
3. Members prove Merkle membership and derive distinct proposal-bound
   nullifiers inside the Risc0 guest.
4. The gate validates the proof journal, proposal ID, multisig ownership,
   constitution version, threshold, tier policy, and transfer recipient.
5. Execution either emits a treasury transfer or atomically updates governance
   state.

The on-chain proof path uses Risc0 receipt composition. The LEZ transaction
builder that attaches the threshold receipt as an assumption to the SPEL
execution is not implemented in this repository yet. See
[Known Limitations](docs/KNOWN_LIMITATIONS.md) for the exact boundary.

## Quick start

Requirements: Rust 1.91 or newer and the Risc0 toolchain used by the workspace.

~~~bash
cargo build -p quorum-cli
RISC0_DEV_MODE=1 ./scripts/demo.sh
~~~

The demo creates a 2-of-3 treasury, proves an aggregated transfer approval,
executes it locally, rotates the private member set, verifies an old key is
rejected, activates the replacement bundle, and uses the new keys.

A minimal CLI flow is:

~~~bash
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
~~~

Dev mode produces fast test receipts and must never be used for production
claims. Real proving rejects dev mode:

~~~bash
RISC0_DEV_MODE=0 \
  cargo run -p quorum-prover --example prove_threshold --release
~~~

## Repository map

| Path | Purpose |
|---|---|
| crates/quorum-core | Domain types, Merkle membership, nullifiers, and policy validation |
| crates/quorum-circuit | Pure threshold statement evaluated by the Risc0 guest |
| crates/quorum-prover | Host-side proving and receipt verification |
| crates/quorum-gate-core | Constitution, proposal, claim, and execution rules shared with SPEL |
| crates/quorum-sdk | Client-side member, proposal, proof, and local state APIs |
| crates/quorum-cli | Offline-first operator workflow and private key files |
| crates/lez-compat | LEZ account commitment compatibility model; not yet wired into membership proofs |
| guests/quorum-threshold | Embedded Risc0 threshold guest |
| programs/quorum-gate | SPEL program and generated IDL |
| apps/basecamp-quorum | Basecamp QML interaction prototype |
| docs | Architecture, privacy, security, deployment, and performance notes |

## Development

~~~bash
cargo fmt --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace
~~~

The local CLI writes quorum.json, member-N.json, rotation.json, and claim
artifacts with mode 0600. These files contain operational state or secrets and
must not be committed.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Circuit design](docs/CIRCUIT_DESIGN.md)
- [Privacy model](docs/PRIVACY_MODEL.md)
- [Security assumptions](docs/SECURITY_ASSUMPTIONS.md)
- [Known limitations](docs/KNOWN_LIMITATIONS.md)
- [Deployment and integration](docs/DEPLOYMENT.md)
- [Error codes](docs/ERROR_CODES.md)
- [Benchmarks](docs/BENCHMARKS.md)
- [Architecture decisions](docs/adr/README.md)

## License

Licensed under either Apache-2.0 or MIT, at your option.
