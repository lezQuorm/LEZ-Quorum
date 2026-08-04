# Quorum Basecamp Prototype

This directory contains a QML prototype for operating Quorum from Logos
Basecamp. It covers the intended create, propose, approve, execute, rotate,
replacement-key activation, and state views.

## Current contents

- metadata.json: Basecamp module metadata.
- module.json: compatibility metadata for module tooling.
- src/qml/QuorumView.qml: the interaction prototype.

## Integration status

This is not currently an installable Basecamp module. The directory does not
contain the native process backend, CMake project, Nix flake, generated SDK
bindings, or LGX packaging needed to run it in Basecamp.

The QML expects a backend object capable of executing the Quorum CLI without a
shell, reporting asynchronous output, exposing busy state, and cancelling a
running proof. That backend still needs to be implemented against the current
Basecamp module SDK.

## Backend requirements

A production module should:

1. Launch a configured quorum binary with an argument list through QProcess.
2. Set RISC0_DEV_MODE=0 for any production approval.
3. Use a dedicated mode-0700 working directory.
4. Keep member and rotation files at mode 0600.
5. Stream stdout and stderr without exposing secret file contents.
6. Support cancellation, timeout, and process cleanup.
7. Validate the binary path and surface structured failures.
8. Integrate the LEZ transaction composer rather than treating local CLI state
   as on-chain state.

Until that work and package verification are complete, use
../../scripts/demo.sh for the supported local workflow.
