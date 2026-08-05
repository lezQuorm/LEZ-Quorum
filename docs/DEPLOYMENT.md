# Deployment and Integration

This runbook separates locally verified implementation from operations that
require a running LEZ sequencer, wallet credentials, and funded accounts.

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

The remaining network procedure is:

1. Start a supported LEZ v0.2 sequencer or select a compatible testnet RPC.
2. Use a funded deployment wallet to deploy `quorum-gate` and record its
   program ID and dependency revisions.
3. Initialize the constitution account and create a proposal account through
   the deployed program.
4. Derive, create, and fund the treasury token holding.
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
