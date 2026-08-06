# Deployment and Integration

This runbook covers offline checks, the verified LEZ v0.2.2 lifecycle,
public-testnet deployment, wallet funding, and Basecamp packaging.

## Local verification

```bash
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo check --workspace --all-targets --all-features
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
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

Create the sibling LEZ v0.2.2 checkout once:

```bash
git clone --branch v0.2.2 --depth 1 \
  https://github.com/logos-blockchain/logos-execution-zone.git \
  ../logos-execution-zone-v022
```

For a fast demonstration, start the sequencer in development-proof mode in
terminal A:

```bash
cd ../logos-execution-zone-v022
RISC0_DEV_MODE=1 just run-sequencer-standalone
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
env -u RISC0_DEV_MODE just run-sequencer-standalone
env -u RISC0_DEV_MODE cargo run -p quorum-composer --features network \
  --example local_lez_e2e -- http://127.0.0.1:3040 101
```

A successful run ends with `RESULT=PASS`. Development mode is appropriate for
a responsive video but must be disclosed on screen; real mode is the intended
cryptographic evidence path and requires substantial free memory for the
nested threshold, gate, and privacy proofs. Both modes are exercised end to
end. Recorded transaction hashes are maintained in
[the verification evidence](evidence/README.md). The current real v0.2.2 run
used seed `121` and completed in approximately 2 hours 19 minutes.

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
6. Run the read-only public `getTransaction` and `getAccountBalance` requests
   below to show block 693 and balance 150. Do not unlock the wallet or show its
   recovery phrase, password, configuration, or private keys on camera.
7. For a UI segment, import the appropriate LGX into Basecamp, select the
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

## Public testnet deployment

The gate was deployed to `https://testnet.lez.logos.co` on 2026-08-06:

```text
LEZ tag:       v0.2.2
LEZ commit:    d6e4ae694e7419f5906b340c232704466a1917b7
Program ID:    [320098040, 1020072060, 2381930866, 4243020391,
                4177030334, 802000452, 1921768834, 3969437236]
Transaction:   4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43
Block:         693
```

Deploy or re-deploy the deterministic gate bytecode with:

```bash
env -u RISC0_DEV_MODE cargo run -p quorum-composer --features network \
  --example deploy_gate -- https://testnet.lez.logos.co
```

LEZ v0.2.2 `ProgramDeployment` transactions contain bytecode without a signer
or fee payer, so this step does not require a funded deployment authority. The
public testnet reset during verification and its large deployment responses may
be truncated by the HTTP gateway. Recheck the current block before presenting
the hash as live state.

```bash
curl -sS https://testnet.lez.logos.co \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"getTransaction","params":["4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43"]}' \
  | jq '.result[1]'
```

The expected block is `693`. A `null` result means the ephemeral testnet state
has reset and the deterministic gate must be deployed again.

The public explorer is `https://explorer.testnet.lez.logos.co`. Its transaction
and block pages had not indexed this deployment during verification even though
the sequencer returned it, so use the RPC response as the canonical live check
until the explorer catches up.

A public wallet account was created, initialized, funded with the official
Piñata proof-of-work claim, and read back on 2026-08-06:

```text
Account:       Public/81yCTY7Sk9h1yjzj5Du4urxxAF5ysLnmnBvtDYaEsUxh
Init tx:       dc995ae3311064981468036810c24f5a315d26cd4718f4cd49e8ff8cc812aae2
Init block:    690
Piñata tx:     f276765e4e74f5b0d85901172a1af97c8f2d751962b95db3a3cf7028732e5c41
Piñata block:  691
Balance read:  150
```

A complete funded public Quorum lifecycle still requires these operator steps:

1. Initialize the constitution, treasury, token, and recipient accounts.
2. Compose and submit propose, private approve, execute, rotate, and
   threshold-change transactions.
3. Re-read public state and scan private outputs after every confirmation.

Run the official wallet from the pinned v0.2.2 checkout. It defaults to the
public testnet endpoint:

```bash
cd ../logos-execution-zone-v022
export LEE_WALLET_HOME_DIR=/home/core/.local/share/lez-wallet-testnet
cargo run --release -p wallet -- check-health
cargo run --release -p wallet -- account new public --label deployer
cargo run --release -p wallet -- auth-transfer init \
  --account-id Public/YOUR_ACCOUNT_ID
cargo run --release -p wallet -- pinata claim \
  --to Public/YOUR_ACCOUNT_ID
cargo run --release -p wallet -- account get \
  --account-id Public/YOUR_ACCOUNT_ID
```

The first command prompts for a wallet password and prints the only recovery
phrase. Record it offline and never include it in logs, screenshots, issues, or
commits.

The public balance can be checked without unlocking the wallet. The sequencer
RPC accepts the bare account ID, without the `Public/` display prefix:

```bash
curl -sS https://testnet.lez.logos.co \
  -H 'content-type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"getAccountBalance","params":["81yCTY7Sk9h1yjzj5Du4urxxAF5ysLnmnBvtDYaEsUxh"]}' \
  | jq '.result'
```

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

## Required lifecycle evidence

Before calling the full public lifecycle complete, capture:

- deployed program ID and exact dependency revisions;
- multisig, proposal, vault, and recipient account IDs;
- transaction hashes for every lifecycle operation;
- missing/wrong receipt, recipient, vault, stale credential, and duplicate
  approval failures on the runtime;
- old-member rejection and replacement-member approval after rotation;
- timeout/retry reconciliation behavior; and
- confirmation latency, compute use, and fees where exposed.
