# Running Quorum In Basecamp

A step-by-step guide to building and running the Quorum module in Logos
Basecamp. It covers the fastest local path (no sequencer) and the full LEZ
testnet path.

Quorum's Basecamp module is a QML frontend: it launches the `quorum` CLI as a
child process and parses its output. The `.lgx` package does **not** embed the
CLI. Install the matching release CLI or build it locally, then select it in
the module.

> Quorum is experimental and unaudited. Do not use it to custody assets of
> material value.

## Install The Published Release

Download the portable LGX, matching CLI, and checksum file from the
[Quorum v0.1.1 release](https://github.com/lezQuorm/LEZ-Quorum/releases/tag/v0.1.1).
Follow [Download And Load](BASECAMP_RELEASE.md#download-and-load) to verify the
files, load the LGX in Basecamp, and select the CLI. The native LGX is intended
for hosts with the matching Nix runtime; prefer the portable LGX for a desktop
installation.

A release installation skips the compilation steps below. Real approvals still
require the Risc0 prover runtime and sufficient memory. Continue at
[Point The Module At The CLI](#3-point-the-module-at-the-cli) after installation.

## Build Requirements

| Requirement | Version / Notes |
|---|---|
| Rust | `1.91.0` (toolchain pinned in the workspace) |
| Risc0 | `cargo-risczero 3.0.5`, guest Rust `1.97.0` |
| Nix | flakes enabled (builds and runs the module) |
| LEZ checkout | `v0.2.2` at `d6e4ae694e7419f5906b340c232704466a1917b7` — only for the standalone-sequencer E2E script, not for the module |

Install the pinned Risc0 toolchain with `rzup`. The module sets
`RISC0_DEV_MODE=0` for its child commands. Approval commands generate real
proofs; status queries and public setup/execution commands do not generate a
new private approval proof.

## 1. Build The CLI

From the repository root:

```bash
cargo build --release -p quorum-cli
realpath target/release/quorum   # note this absolute path for later
```

This produces `target/release/quorum` (CLI version `0.1.1`, matching the
module's `metadata.json`).

## 2. Build And Run The Module

```bash
cd apps/basecamp-quorum

# Build all locked flake outputs
nix --extra-experimental-features 'nix-command flakes' build .#generate
nix --extra-experimental-features 'nix-command flakes' build .#lib
nix --extra-experimental-features 'nix-command flakes' build .#lgx
nix --extra-experimental-features 'nix-command flakes' build .#lgx-portable

# Run the development module in the standalone host
nix --extra-experimental-features 'nix-command flakes' run .
```

`nix run .` launches the official standalone Basecamp host with the Quorum
view loaded. To load the module inside an existing Logos Basecamp app, use the
packaged `.lgx` (prefer `.#lgx-portable`, which carries its non-Qt runtime
libraries). Packaging, checksums, and the release layout are documented in
[BASECAMP_RELEASE.md](BASECAMP_RELEASE.md).

## 3. Point The Module At The CLI

The module first uses the CLI path saved in settings
(`~/.config/Logos/Quorum.conf`). If none is saved, it searches for `quorum` on
`PATH`. You can set an absolute path in the UI:

1. Stay on the **Local** tab.
2. In **Runtime**, set **CLI binary** to the absolute path of the downloaded
   `quorum-0.1.1-linux-amd64` or locally built `target/release/quorum`.
3. Set **Private state directory** to a new, absolute, empty directory.
4. Click **Apply**. The module persists both values.

The module always sets `RISC0_DEV_MODE=0` for its child commands, so approvals
generate **real** proofs and each one can take several minutes. Proof
operations have no timeout; other operations have a 30-minute limit and can be
cancelled from the toolbar.

## 4. First Run: Local Mode

Local mode exercises the full create → propose → approve → execute lifecycle
against the CLI's filesystem state machine. It needs no sequencer and writes
no testnet transactions. Approvals still generate real threshold proofs and can
take several minutes.

1. **Create** — choose **Threshold** and **Members** (default 2-of-3), then
   click **Create multisig**. This writes the constitution and one private
   member file per member into the state directory.
2. **Propose** — enter a 64-character hex **Recipient**, an **Amount**
   (default 500), and a **Policy tier** (default 1), then click
   **Propose transfer**.
3. **Approve** — set **Member index** and **Proposal ID**, then click
   **Approve privately**. This runs a real threshold proof. Watch its progress
   in the **Activity** pane. Repeat for a second distinct member index to
   reach the 2-of-3 threshold.
4. **Execute** — from the same tab, click **Execute**.
5. **State** — click **Refresh state** and confirm the balances, approval
   count, and `Executed` proposal status.

Governance actions live on the **Rotate** tab: **Generate root** creates a new
member root, **Propose rotation** opens it, and **Activate replacement keys**
switches the local key material after the rotation is approved and executed.

The Activity pane on the right shows live output, the elapsed timer, a
**Explorer** link for confirmed transactions, and a **Copy** button. Errors
are surfaced in red with a plain-language hint where the module recognizes a
known case.

## 5. Full Run: LEZ Testnet Mode

Testnet mode drives `quorum network --target testnet` and writes real
transactions. It forces real proofs, defaults to RPC
`https://testnet.lez.logos.co`. Start a fresh private session for a new
independent lifecycle; reuse the existing session when resuming or inspecting
its transactions.

1. Switch the mode selector to **LEZ Testnet**. The module creates a session
   directory named `lez-quorum-testnet-<timestamp>` beside the current working
   directory (falling back to `/tmp`). It is editable; use **New session** to
   generate another.
2. Click **Check RPC** to confirm health, then **Verify deployment** on
   **1 Setup**.
3. Set **Threshold** to 2 and **Members** to 3, then click **Prepare private
   state**. With the standard funding and transfer values this runs
   `prepare --threshold 2 --members 3 --funding 750 --transfer 250` and
   generates an isolated member set for this session.
4. Enable the **Submit next action to LEZ testnet** switch, then click
   **Submit initialization**. The switch resets automatically after every
   submission, so enable it again for each public write.
5. **2 Treasury** walks the lifecycle one step at a time: **Create token →
   Initialize recipient → Initialize vault → Fund vault → Open proposal**.
   Each step is preview-then-submit.
6. **3 Approve** generates one private approval per click
   (`approve-threshold --proposal 0`). Run it twice for a 2-of-3. The counter
   shows `N / M confirmed approvals`; the **Refresh** button re-reads state.
7. **4 Execute** becomes available once the threshold is met. Use **Refresh**,
   then **Submit execution**.
8. **5 State** shows the live block, accounts, balances, approvals, and
   transaction journal.

If a write is interrupted or times out, use **Reconcile** on **4 Execute**
(optionally with a transaction label). Only resubmit with **Resubmit exact**
after reconciliation reports the transaction is not found, and only with the
submission switch enabled.

## Completed Session And Videos

The [Basecamp demo](https://www.youtube.com/watch?v=m65kwds8LOc) covers setup,
treasury writes, and the start of approval proving. The
[CLI verification demo](https://www.youtube.com/watch?v=zBWPmJSlVj8) checks the
completed session. Full transaction hashes, blocks, and final balances are in
[Demo Testnet Evidence](DEMO_TESTNET_EVIDENCE.md).

After execution, use **State** or the CLI's `network --target testnet status`
from that session directory to inspect the result. Keep the same session;
there is no need to prepare another one for a status check.

## Troubleshooting

| Symptom | Cause / Fix |
|---|---|
| "Quorum binary is not executable" | Point **CLI binary** at the absolute `target/release/quorum` path and click **Apply**. Build the CLI first. |
| "This session is already prepared" | The state directory already has a session. Click **New session**, then **Prepare private state**. |
| "Initialize must be confirmed first" | Initialization was only previewed. Enable **Submit next action to LEZ testnet** and submit it. |
| Module shows "Connecting" | The view module has not reported ready yet, or no `quorum` binary is resolvable. Set the binary path. |
| Approval appears to hang | Expected: real proofs take minutes. Watch the proof phase and elapsed time, or **Cancel**. |
| Money/state looks stale | Click **Refresh state** (or **Refresh** on Approve/Execute) to re-read on-chain state. |

## Secret Hygiene

The state directory holds private member material. Treat it as a secret:

- never commit or share the session directory, `member-*.json`, or
  `.quorum-testnet/`;
- do not screen-share raw witness data, passwords, or recovery phrases; and
- use a throwaway directory for experiments.

See [Error Codes](ERROR_CODES.md) for deterministic failures and [Known
Limitations](KNOWN_LIMITATIONS.md) before building a user-facing module.
