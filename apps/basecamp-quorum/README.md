# Quorum Multisig — Basecamp Module

A `ui_qml` module exposing the Quorum treasury workflow inside Logos Basecamp:

- **Create** a private M-of-N multisig (commits only a member-set Merkle root);
- **Propose** a treasury transfer (tiered: threshold + amount cap enforced by
  the constitution);
- **Approve** as a shielded member (client-side Risc0 proof; only a nullifier
  is ever recorded on-chain);
- **Execute** once the threshold is reached;
- **Rotate** members privately (Idea 02 differentiator — the only on-chain
  artifact is a new commitment root).

## Architecture

Follows the universal Logos `ui_qml` pattern:

- `metadata.json` declares the module and its `lez_core` dependency;
- `src/qml/QuorumView.qml` is the Basecamp view with tabs for each workflow
  step;
- the module backend launches the audited `quorum` CLI via `QProcess` argument
  lists (never a shell), forces `RISC0_DEV_MODE=0` for real proofs, and
  streams output back to the result pane.

Member secrets live in permission-restricted files (`member-<i>.json`, mode
0600) managed by the Rust CLI; they are never returned as QML values.

## Build

Build the Rust CLI first:

```bash
cargo build --release -p quorum-cli
```

Then build the module with the official Logos module builder:

```bash
cd apps/basecamp-quorum
nix build .#lgx
```

Install the resulting `.lgx` with `lgpm`. In the module header, set the
Quorum binary to the absolute path of `target/release/quorum` and select
**Use binary**.

For a non-Nix developer build, set `LOGOS_MODULE_BUILDER_ROOT` and run CMake
in the usual out-of-tree build directory. Qt 6 and the module builder's
generated SDK are required.

## Runtime Notes

Proof generation takes minutes on commodity hardware with
`RISC0_DEV_MODE=0`. The backend runs proofs asynchronously and keeps the
Basecamp UI responsive. Closing Basecamp or selecting **Cancel** terminates
the child process, so an interrupted proof must be restarted.

See `scripts/demo.sh` for the equivalent CLI flow and `docs/PRIVACY_MODEL.md`
for the privacy guarantees.
