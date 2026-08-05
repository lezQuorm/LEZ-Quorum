# Deployment and Integration

This runbook covers offline checks, the verified local LEZ lifecycle, Basecamp
packaging, and the separate operator requirements for public testnet deployment.

## Local verification

```bash
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo check --workspace --all-targets --all-features
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-features
RISC0_DEV_MODE=1 ./scripts/demo.sh
```

Generate a real threshold receipt:

```bash
RISC0_DEV_MODE=0 \
  cargo run -p quorum-prover --example prove_threshold --release
```

Regenerate and validate the gate IDL:

```bash
cargo run -p quorum-gate-methods --example generate_idl
RISC0_DEV_MODE=1 cargo test -p quorum-gate-methods
```

## Local standalone lifecycle

Use the sibling LEZ v0.2.0 checkout. For a fast demonstration, start the
sequencer in development-proof mode in terminal A:

```bash
cd ../logos-execution-zone
RISC0_DEV_MODE=1 just run-sequencer standalone
```

Run the complete lifecycle in terminal B:

```bash
RISC0_DEV_MODE=1 cargo run -p quorum-composer --features network \
  --example local_lez_e2e -- http://127.0.0.1:3040 91
```

The numeric seed selects deterministic public test accounts and private member
credentials. Use a fresh value from 1 through 250 for every run against the
same persistent chain. The example deploys the gate; initializes the
constitution, token definition, recipient, and treasury PDA; funds the vault;
proposes a transfer; submits a recursively composed private 2-of-3 approval;
executes; and re-reads all final state.

For real Risc0 proofs, omit the development variable in both terminals:

```bash
env -u RISC0_DEV_MODE just run-sequencer standalone
env -u RISC0_DEV_MODE cargo run -p quorum-composer --features network \
  --example local_lez_e2e -- http://127.0.0.1:3040 101
```

A successful run ends with `RESULT=PASS`. Development mode is appropriate for
a responsive video but must be disclosed on screen; real mode is the intended
cryptographic evidence path and requires substantial free memory for the
nested threshold, gate, and privacy proofs.

## Basecamp package verification

```bash
cd apps/basecamp-quorum
nix build .#generate .#lib
nix build .#lgx --out-link result-lgx
nix build .#lgx-portable --out-link result-lgx-portable
```

The verified archives contain the module manifest, `QuorumView.qml`,
`quorum_ui_plugin.so`, and `quorum_ui_replica_factory.so`. The native closure
resolves Qt 6.9.2 Core, Network, QML, and Remote Objects. The portable package
bundles its non-Qt external libraries. Visual execution still requires a
running Basecamp host because `Logos.Controls`, `Logos.Theme`, and the module
manager are host-provided.

## Video demo sequence

1. Show the sequencer starting on `127.0.0.1:3040`.
2. Run `local_lez_e2e` with a fresh seed and keep the full terminal visible.
3. Point out the printed gate, multisig, vault, and recipient IDs.
4. Let the transaction hashes show deployment, initialization, funding,
   proposal, private approval, and execution.
5. Finish on the four final assertions: vault 500, recipient 250, executed
   status, and `RESULT=PASS`.
6. For a UI segment, import the appropriate LGX into Basecamp, select the
   absolute `target/release/quorum` path, choose a protected empty working
   directory, then demonstrate create, propose, approve, execute, and state.

## Composer API

`quorum-composer` accepts a verified `QuorumProof` and wallet-prepared
`PrivateApprovalRequest`. The request contains the deployed gate program,
current multisig/proposal states, private credential identities, public account
IDs and nonces, and any public signer keys.

The composer verifies the proof artifact and credential binding, proves the
gate with the threshold receipt assumption, proves the outer LEZ privacy
circuit, and returns a `PrivacyPreservingTransaction`. With the `network`
feature, `NetworkClient` submits once and confirms by hash:

```bash
cargo check -p quorum-composer --features network
```

On confirmation, re-read the public multisig and proposal accounts. Reconcile
private credential changes through the wallet's encrypted-output scan and new
commitments. If confirmation times out, query the existing hash before
rebuilding or resubmitting a transaction.

## Program deployment

A public testnet deployment requires these operator-controlled steps:

1. Start a supported LEZ v0.2 sequencer or select a compatible testnet RPC.
2. Use a funded deployment wallet to deploy `quorum-gate` and record its
   program ID and dependency revisions.
3. Initialize the constitution account and create a proposal account through
   the deployed program.
4. Submit `InitializeVault` with the constitution signer, then fund the derived
   treasury token holding.
5. Enroll private credential commitments and ensure the wallet can construct
   private account init/update identities.
6. Compose and submit propose, approve, execute, rotate, and threshold-change
   transactions.
7. Re-read public state and scan private outputs after every confirmation.

The treasury vault seed is:

```text
SHA256("quorum/vault/v1" || multisig_account_id)
```

Execution validates that PDA and the approved recipient before emitting the
token transfer chained call.

## Rotation operations

The offline workflow creates a protected replacement bundle and prints its
public root:

```bash
NEW_ROOT="$(quorum new-root --members 3)"
quorum propose \
  --action rotate \
  --new-member-root "$NEW_ROOT" \
  --new-member-count 3
```

Approve and execute under the old constitution. Activate the replacement
bundle only after confirmed chain state contains the new root:

```bash
quorum activate-rotation
```

The bundle contains credentials and must be distributed through an approved
secure channel.

## Required network evidence

Before calling a deployment complete, capture:

- deployed program ID and exact dependency revisions;
- multisig, proposal, vault, and recipient account IDs;
- transaction hashes for every lifecycle operation;
- missing/wrong receipt, recipient, vault, stale credential, and duplicate
  approval failures on the runtime;
- old-member rejection and replacement-member approval after rotation;
- timeout/retry reconciliation behavior; and
- confirmation latency, compute use, and fees where exposed.
