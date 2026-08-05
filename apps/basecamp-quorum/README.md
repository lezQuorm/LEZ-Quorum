# Quorum Basecamp Module

This directory contains the native Logos Basecamp module for the Quorum
operator workflow.

## Components

- `metadata.json` is the canonical `ui_qml` module manifest.
- `src/quorum_ui.rep` defines the Qt Remote Objects contract.
- `QuorumUiBackend` runs the Quorum binary asynchronously through `QProcess`.
- `src/qml/QuorumView.qml` provides create, propose, approve, execute, rotate,
  replacement activation, and state views.
- `CMakeLists.txt` and `flake.nix` package the module with the current Logos
  module builder.

The backend never invokes a shell. It validates the executable path, accepts
only known Quorum commands, forces `RISC0_DEV_MODE=0`, uses a mode-0700 working
directory, streams stdout and stderr, rejects concurrent operations, supports
cancellation, and terminates operations after 30 minutes. The Rust CLI keeps
member and rotation files at mode 0600.

## Build

Build the Quorum binary first:

```bash
cargo build --release -p quorum-cli
```

Then build the LGX with the official module builder:

```bash
cd apps/basecamp-quorum
nix build .#lgx
```

Install the resulting package with `lgpm`, select the absolute path to
`target/release/quorum`, and choose a protected working directory.

For a non-Nix developer build, set `LOGOS_MODULE_BUILDER_ROOT` and configure an
out-of-tree CMake build. Qt 6 and the builder's generated SDK are required.

## Current boundary

The source follows the current Basecamp backend contract, but this environment
does not contain Nix, CMake, or Qt development packages. No LGX artifact has
been built or installed here.

The controls currently drive the offline CLI workflow. The Rust transaction
composer is implemented in `crates/quorum-composer`, but existing private
credential updates require a supported LEZ wallet to scan encrypted state and
provide membership proofs. Connecting that wallet/composer flow to Basecamp is
still required for live chain operation.
