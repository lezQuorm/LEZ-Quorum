# Quorum

Quorum is a privacy-preserving threshold treasury for the Logos Execution Zone
(LEZ). An M-of-N group can authorize transfers, membership rotations, and
threshold changes without publishing its member list or attributing approvals
to individual members.

The protocol combines a credential-aware Risc0 threshold circuit, a SPEL gate,
recursive LEZ private transaction composition, an operator CLI, and a Basecamp
module. It targets LEZ v0.2.2 and remains experimental software that has not
received an independent security audit.

## Capabilities

| Capability | Implementation |
|---|---|
| Private membership | The constitution stores a Merkle root over LEZ private-account credentials, not a member list |
| Anonymous approvals | Proofs expose proposal-scoped nullifiers and credential bindings, not secrets, account IDs, or Merkle paths |
| Credential control | The threshold proof binds enrollment to an LEZ private account; the outer privacy proof establishes control of the same account |
| Threshold policy | Default and per-tier thresholds with transfer amount caps |
| Private composition | A verified threshold receipt is attached to the gate execution and recursively wrapped by the LEZ privacy circuit |
| Treasury execution | The gate validates a deterministic treasury PDA and recipient before emitting a token transfer chained call |
| Governance | Threshold-approved membership rotation and threshold changes invalidate stale credentials and proposals |
| Client integration | Complete IDL, generated-client checks, instruction round trips, RPC submission, and confirmation |
| Operator surfaces | Protected offline CLI plus native and portable Basecamp LGX packages |

## Protocol

1. Operators initialize a constitution with a threshold, member count,
   credential root, and optional spending tiers.
2. A proposal is bound to its multisig account and constitution version.
3. Members prove credential membership and derive distinct proposal-bound
   nullifiers in the Risc0 guest.
4. The composer verifies the threshold receipt, binds its journal to an
   approval, and attaches it to the gate proof as an assumption.
5. The LEZ privacy circuit verifies the gate proof and private-account control,
   then emits encrypted credential post-states and public gate-state updates.
6. Once the threshold is met, the gate applies the governance action or
   executes the treasury transfer.

Credential account IDs remain private transaction inputs. Proposal content,
policy, approval count, Quorum nullifiers, and governance changes are public by
design.

## Testnet status

The v0.2.2 gate is deployed on the public LEZ testnet.

| Field | Value |
|---|---|
| RPC | `https://testnet.lez.logos.co` |
| Explorer | `https://explorer.testnet.lez.logos.co` (may lag sequencer RPC) |
| LEZ revision | tag `v0.2.2`, commit `d6e4ae694e7419f5906b340c232704466a1917b7` |
| Gate program ID | `[320098040, 1020072060, 2381930866, 4243020391, 4177030334, 802000452, 1921768834, 3969437236]` |
| Deployment transaction | `4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43` |
| Confirmed block | `693` on 2026-08-06 |

The public network reset once during verification. Testnet state is therefore
ephemeral, and the transaction should be rechecked before a live demonstration.
The deployment transaction contains no signer or fee payer in LEZ v0.2.2; a
funded authority is needed for funded account demonstrations, not program
deployment. The complete treasury lifecycle, including a real nested proof,
has been verified against the official v0.2.2 standalone sequencer. Public
full-lifecycle evidence is tracked separately in
[the deployment runbook](docs/DEPLOYMENT.md).

## Quick start

Requirements: Rust 1.91 or newer and the Risc0 toolchain used by the workspace.

```bash
cargo build -p quorum-cli
RISC0_DEV_MODE=1 ./scripts/demo.sh
```

The offline demo creates a 2-of-3 treasury, approves and executes a transfer,
rotates the member set, proves an old credential is rejected, and activates the
replacement credentials.

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

Development mode creates mock receipts and must be disclosed in demonstrations.
Generate and host-verify a real succinct threshold receipt with:

```bash
RISC0_DEV_MODE=0 \
  cargo run -p quorum-prover --example prove_threshold --release
```

## Complete LEZ lifecycle

Start the official LEZ v0.2.2 standalone sequencer in terminal A:

```bash
cd ../logos-execution-zone-v022
RISC0_DEV_MODE=1 just run-sequencer-standalone
```

Run the 2-of-3 lifecycle in terminal B. Use a fresh seed from 1 through 250
when reusing a persistent chain.

```bash
cd ../LOGOS/LEZ-Quorum
RISC0_DEV_MODE=1 cargo run -p quorum-composer --features network \
  --example local_lez_e2e -- http://127.0.0.1:3040 91
```

The example deploys the gate, initializes and funds the treasury, proposes a
transfer, submits a recursively composed private approval, executes the
transfer, and verifies final state:

```text
vault_balance=500
recipient_balance=250
proposal_status=Executed
RESULT=PASS
```

For cryptographic evidence, omit `RISC0_DEV_MODE` in both terminals. The full
real-proof run is CPU and memory intensive; exact recorded results are kept in
[the verification evidence](docs/evidence/README.md).

Deploy only the gate to the public testnet with:

```bash
env -u RISC0_DEV_MODE cargo run -p quorum-composer --features network \
  --example deploy_gate -- https://testnet.lez.logos.co
```

## Basecamp package

Nix supplies the pinned CMake, Ninja, Qt 6.9.2, Qt Remote Objects, Logos module
builder, and LGX bundler environment:

```bash
cd apps/basecamp-quorum
nix build .#generate .#lib .#lgx .#lgx-portable
```

The native LGX targets the Basecamp Nix runtime. The portable LGX bundles its
non-Qt external libraries and expects Basecamp to provide Qt and Logos QML
modules. See [the module README](apps/basecamp-quorum/README.md) for the package
and runtime boundaries.

## Repository map

| Path | Purpose |
|---|---|
| `crates/quorum-core` | Merkle membership, credential commitments, nullifiers, and policy validation |
| `crates/lez-compat` | LEZ v0.2.2 private-account derivation and commitment compatibility |
| `crates/quorum-circuit` | Pure credential-aware threshold statement |
| `crates/quorum-prover` | Host proving, artifact encoding, and pinned-image verification |
| `crates/quorum-composer` | Private transaction composition, RPC submission, confirmation, and state reads |
| `crates/quorum-image-id` | Pinned threshold image identifier shared by host and gate |
| `crates/quorum-gate-core` | Constitution, proposal, claim, credential, and execution rules |
| `crates/quorum-sdk` | Member, proposal, proof, rotation, and local state APIs |
| `crates/quorum-cli` | Offline operator workflow and protected key files |
| `guests/quorum-threshold` | Embedded Risc0 threshold guest |
| `programs/quorum-gate` | SPEL program, IDL, and client generator |
| `apps/basecamp-quorum` | Basecamp QML view, native backend, and LGX packaging |

## Development

```bash
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo check --workspace --all-targets --all-features
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
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
