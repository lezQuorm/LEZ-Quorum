# Quorum Basecamp Module

This module exposes the Quorum CLI through a process-isolated QML interface.

| File | Role |
|---|---|
| `metadata.json` | Basecamp module manifest |
| `src/quorum_ui.rep` | Qt Remote Objects contract |
| `src/quorum_ui_backend.cpp` | Validated `QProcess` backend |
| `src/qml/QuorumView.qml` | Treasury interface |

The backend accepts only known commands, invokes no shell, forces real proving,
uses a mode `0700` working directory, rejects concurrent operations, supports
cancellation, and enforces a 30-minute timeout.

## Build

Build the CLI and both LGX variants:

```bash
cargo build --release -p quorum-cli
cd apps/basecamp-quorum
nix build .#generate .#lib
nix build .#lgx --out-link result-lgx
nix build .#lgx-portable --out-link result-lgx-portable
```

Outputs:

```text
result-lgx/logos-quorum_ui-module.lgx
result-lgx-portable/logos-quorum_ui-module.lgx
```

The native package targets the Nix Basecamp runtime. The portable package
bundles non-Qt libraries and uses Qt and Logos QML modules from Basecamp.

## Runtime

Import the LGX, select the absolute `target/release/quorum` path, and use a
protected working directory. The CLI stores member and rotation secrets with
mode `0600`.

The interface currently drives the local CLI state. Live private-account state
requires a wallet capable of scanning encrypted LEZ outputs and constructing
current membership proofs.
