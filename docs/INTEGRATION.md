# Integration Guide

This guide covers the SDK, CLI, SPEL program, network workflow, and Basecamp
module. All network examples target LEZ v0.2.2.

## Build

Install Rust 1.91.0 and Risc0 3.0.5, then build the client and program:

```bash
cargo build --release -p quorum-cli
cargo build --release -p quorum-gate-methods
```

Run the supported checks before using artifacts:

```bash
cargo fmt --all -- --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets --all-features -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace --all-targets --all-features -- --test-threads=1
```

The release CLI is `target/release/quorum`. Run `quorum <command> --help` for
the complete option set.

## Local State Workflow

The filesystem-backed CLI is the shortest way to integrate the SDK concepts
without a sequencer:

```bash
mkdir /tmp/quorum-example
cd /tmp/quorum-example

/path/to/LEZ-Quorum/target/release/quorum create \
  --threshold 2 --members 3 \
  --tiers '[{"id":1,"threshold":2,"max_amount":1000}]'

/path/to/LEZ-Quorum/target/release/quorum propose \
  --action transfer \
  --recipient 0909090909090909090909090909090909090909090909090909090909090909 \
  --amount 500 --tier 1

RISC0_DEV_MODE=1 /path/to/LEZ-Quorum/target/release/quorum \
  approve-all --proposal 0 --members 0,1
/path/to/LEZ-Quorum/target/release/quorum execute --proposal 0
/path/to/LEZ-Quorum/target/release/quorum info
```

`create` writes `quorum.json` plus one private `member-<index>.json` file per
member. `approve` proves one member; `approve-all` aggregates a comma-separated
set into one receipt. Use `RISC0_DEV_MODE=0` or leave it unset for a real
receipt.

The maintained script also tests rotation and rejects a removed member:

```bash
RISC0_DEV_MODE=1 ./scripts/demo.sh
```

## Standalone LEZ Sequencer

Clone the exact LEZ source used by CI:

```bash
git clone --branch v0.2.2 --depth 1 \
  https://github.com/logos-blockchain/logos-execution-zone.git \
  ../logos-execution-zone-v022
git -C ../logos-execution-zone-v022 rev-parse HEAD
# d6e4ae694e7419f5906b340c232704466a1917b7
```

Run the full gate, token, private approval, and transfer lifecycle:

```bash
RISC0_DEV_MODE=1 LEZ_REPO=../logos-execution-zone-v022 \
  ./scripts/sequencer-e2e.sh
```

The script builds and owns an isolated sequencer, waits for RPC health, runs
the lifecycle, checks exact final values, writes logs under `target/e2e-logs/`,
and shuts down the process. To meet the prize's real-proof reproduction
requirement, use:

```bash
LEZ_REPO=../logos-execution-zone-v022 ./scripts/sequencer-e2e.sh
```

The real path defaults to a four-hour timeout because it recursively proves
the threshold guest, gate, and LEZ privacy transaction.

## Network CLI

Choose `local` for a developer sequencer or `testnet` for the public endpoint:

```bash
Q=target/release/quorum
$Q network --target local health
$Q network --target local deployment
$Q network --target local prepare --threshold 2 --members 3 \
  --funding 750 --transfer 250
```

The state directory is `.quorum-network-local/` for local and
`.quorum-testnet/` for testnet, relative to the current working directory. Use
a new private working directory for every independent lifecycle.

Run writes in order:

```bash
$Q network --target local initialize
$Q network --target local initialize --confirm-public-write
$Q network --target local create-token
$Q network --target local create-token --confirm-public-write
$Q network --target local initialize-recipient
$Q network --target local initialize-recipient --confirm-public-write
$Q network --target local initialize-vault
$Q network --target local initialize-vault --confirm-public-write
$Q network --target local fund
$Q network --target local fund --confirm-public-write
$Q network --target local propose
$Q network --target local propose --confirm-public-write
$Q network --target local approve-threshold
$Q network --target local approve-threshold --confirm-public-write
$Q network --target local approve-threshold
$Q network --target local approve-threshold --confirm-public-write
$Q network --target local execute --proposal 0
$Q network --target local execute --proposal 0 --confirm-public-write
$Q network --target local status
```

The first invocation of each write saves the exact transaction and prints its
hash. The confirming invocation submits those saved bytes. Do not change
configuration between preview and confirmation.

After a timeout or interruption:

```bash
$Q network --target local status
$Q network --target local reconcile
```

Only after a not-found result and manual review may the exact saved bytes be
resubmitted:

```bash
$Q network --target local reconcile \
  --resubmit-unconfirmed --confirm-public-write
```

## Testnet

Use the same commands with `--target testnet`. Testnet mode requires
`RISC0_DEV_MODE=0` or an unset variable. Before any write, compare the health
and deployment output with [Deployment](DEPLOYMENT.md):

```bash
env -u RISC0_DEV_MODE $Q network --target testnet health
env -u RISC0_DEV_MODE $Q network --target testnet deployment
env -u RISC0_DEV_MODE $Q network --target testnet status
```

Do not reuse the published demonstration's member material. `prepare` must
generate a new isolated session. Testnet is public and may reset or upgrade.

## LEZ Account Compatibility

The public multisig pattern claims fresh, zero-nonce member accounts for the
multisig program. That is incompatible with private accounts: the LEZ privacy
protocol remains their `program_owner`, and the account nonce changes when the
private account is used.

Quorum does not claim or re-key member accounts. It treats them as credential
inputs to the outer LEZ privacy transaction:

1. the threshold proof derives membership from the regular private account ID;
2. the gate binds its scoped credential commitment to the supplied account ID;
3. the outer LEZ proof authorizes the private account and applies its normal
   nonce/commitment transition; and
4. Quorum stores only an approval nullifier in public proposal state.

`crates/lez-compat` mirrors the LEZ v0.2.2 account commitment byte layout,
private account derivation, and Merkle conventions. Its `rules` module
documents expected invariants; consensus enforcement is performed by LEZ, not
by that host helper module.

## Rust SDK

The main integration types are:

| Crate | Entry points |
|---|---|
| `quorum-sdk` | `MemberSet`, `Multisig`, `approve`, `approve_all`, persistence |
| `quorum-prover` | witness proving, receipt encoding, image verification |
| `quorum-composer` | `prepare_approval`, private transaction composition, RPC |
| `quorum-gate-core` | instruction/state types and deterministic validation |

Depend on workspace crates by path while developing against this repository.
Callers must preserve member material, construct the canonical member tree,
fetch current constitution/proposal/account state, prove against that exact
state, and submit through the transaction journal or equivalent retry logic.

Do not accept a receipt solely because it decodes. Always verify its image ID,
compare its journal with expected proposal state, and bind the credential
accounts before composition. `quorum-composer::prepare_approval` performs
these checks.

## SPEL IDL

The generated interface is:

```text
programs/quorum-gate/idl/quorum_gate.idl.json
```

It defines `initialize`, `initialize_vault`, `propose`, `approve`, and
`execute`, all state types, and gate errors `4001..4017`. Regenerate and verify
it after a program interface change:

```bash
cargo run -p quorum-gate-methods --example generate_idl
git diff --exit-code -- programs/quorum-gate/idl/quorum_gate.idl.json
```

## Basecamp

For a step-by-step walkthrough of building the CLI, running the module, and
completing both the local and testnet flows, see [Running Quorum in
Basecamp](BASECAMP_GUIDE.md). The build commands are summarized below.

Build all outputs from the locked flake:

```bash
cargo build --release -p quorum-cli
cd apps/basecamp-quorum
nix --extra-experimental-features 'nix-command flakes' build .#generate
nix --extra-experimental-features 'nix-command flakes' build .#lib
nix --extra-experimental-features 'nix-command flakes' build .#lgx
nix --extra-experimental-features 'nix-command flakes' build .#lgx-portable
```

Run the development module with:

```bash
nix --extra-experimental-features 'nix-command flakes' run .
```

The module manifest is `apps/basecamp-quorum/metadata.json`, passed as
`configFile` to `mkLogosQmlModule`. This is the manifest expected by the pinned
official builder. The UI resolves a `quorum` executable or accepts an explicit
binary path, creates a private working directory, and always sets
`RISC0_DEV_MODE=0` for child commands.

In the app, select `Local` or `LEZ Testnet`, configure the release CLI, prepare
a fresh session, then follow Setup, Treasury, Proposals, Approvals, and Activity.
The public-write checkbox is deliberately reset between submissions. Proof
operations expose live phase and elapsed-time output and may be cancelled.

Portable and native `.lgx` results are release artifacts. Build them from the
same commit, calculate SHA-256 checksums, attach both files and the checksum
manifest to a release, then link those downloadable assets from the submission.

## Secret Material

Never commit or share:

- `.quorum-network-local/`, `.quorum-testnet/`, or local demo directories;
- `member-*.json`, credential claims, passwords, or recovery phrases;
- logs containing secret witness data; or
- Basecamp working directories from a real session.

Use [Error Codes](ERROR_CODES.md) for recovery behavior and
[Known Limitations](KNOWN_LIMITATIONS.md) before building a user-facing module.
