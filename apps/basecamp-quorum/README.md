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

Requirements are Rust, the Risc0 workspace toolchain, and Nix with flakes
enabled. The committed `flake.lock` pins the Logos builder and LGX bundler.

Build the Quorum CLI first:

```bash
cargo build --release -p quorum-cli
```

Build named native and portable package links:

```bash
cd apps/basecamp-quorum
nix build .#generate .#lib
nix build .#lgx --out-link result-lgx
nix build .#lgx-portable --out-link result-lgx-portable
```

The installable archives are:

```text
result-lgx/logos-quorum_ui-module.lgx
result-lgx-portable/logos-quorum_ui-module.lgx
```

The native archive targets a Nix-based Basecamp installation. The portable
archive contains the non-Qt shared libraries needed by the module and expects
Basecamp to provide Qt 6 and the Logos QML modules. Import the appropriate LGX
through the Basecamp module manager, select the absolute path to
`target/release/quorum`, and choose a protected working directory.

## Verified build

The pinned build has been completed with Nix 2.35.1, CMake, Ninja, Qt 6.9.2,
Qt QML, and Qt Remote Objects. Both archives contain `QuorumView.qml`,
`quorum_ui_plugin.so`, `quorum_ui_replica_factory.so`, and their manifests.
The native plugin closure resolves all linked libraries, and Qt `qmlformat`
parses the packaged view successfully.
The module-builder standalone preview also starts headlessly with the app,
capability host, and Quorum `ui-host` processes running without a plugin or QML
load error. Interactive execution requires a desktop Basecamp host.

## Runtime boundary

The controls drive the protected offline CLI workflow. Live private credential
updates additionally require a supported LEZ wallet to scan encrypted state
and provide current membership proofs. Connecting that wallet/composer flow to
Basecamp remains integration work; the standalone LEZ lifecycle is available
through the `local_lez_e2e` example documented in the repository README.
