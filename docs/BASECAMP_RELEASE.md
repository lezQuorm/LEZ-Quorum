# Basecamp Release 0.1.1

This is the release manifest for the documentation-complete build. The source
files are staged under `target/release-assets/`, which is intentionally
Git-ignored. The published release is
<https://github.com/lezQuorm/LEZ-Quorum/releases/tag/v0.1.1>.

## Build Inputs

| Input | Value |
|---|---|
| Module version | `0.1.1` |
| Platform | `x86_64-linux` |
| `logos-module-builder` | `ed0cde56e2eae66030886abf7efff09eaadbc805` |
| `nix-bundle-lgx` | `6671436d7505e20cc568599c993b8c514bf50f94` |
| Manifest | `apps/basecamp-quorum/metadata.json` |

## Staged Assets

| Asset | SHA-256 |
|---|---|
| `lez-quorum-basecamp-0.1.1-linux-amd64-dev.lgx` | `595dd8ada37080ac25be9c386465fd69cf4a5cd201b814bcbfe3685aeec1a610` |
| `lez-quorum-basecamp-0.1.1-linux-amd64-portable.lgx` | `89d2496cafb1904a44827d4a598384846d409d5876adb4a11ac6510169444c40` |
| `quorum-0.1.1-linux-amd64` | `7763840fbe853189e00bf3c71ef05333da0a9964688f4255fb99f7d9228254e4` |

The development bundle contains variant `linux-amd64-dev`. The portable bundle
contains `linux-amd64` and carries its non-Qt runtime libraries. Both archives
use LGX manifest version `0.3.0`, name `quorum_ui`, entry point
`quorum_ui_plugin.so`, and view `qml/QuorumView.qml`.

The module launches the Quorum CLI as a process. A user must also install or
build the matching `quorum` 0.1.1 binary and select it in the module. The LGX
does not embed the Rust CLI.

## Rebuild

```bash
cargo build --release -p quorum-cli
mkdir -p target/release-assets
cd apps/basecamp-quorum
nix --extra-experimental-features 'nix-command flakes' build .#generate
nix --extra-experimental-features 'nix-command flakes' build .#lib
DEV_LGX_OUTPUT=$(nix --extra-experimental-features 'nix-command flakes' \
  build .#lgx --no-link --print-out-paths)
PORTABLE_LGX_OUTPUT=$(nix --extra-experimental-features 'nix-command flakes' \
  build .#lgx-portable --no-link --print-out-paths)
install -m 0644 "$DEV_LGX_OUTPUT/logos-quorum_ui-module.lgx" \
  ../../target/release-assets/lez-quorum-basecamp-0.1.1-linux-amd64-dev.lgx
install -m 0644 "$PORTABLE_LGX_OUTPUT/logos-quorum_ui-module.lgx" \
  ../../target/release-assets/lez-quorum-basecamp-0.1.1-linux-amd64-portable.lgx
install -m 0755 ../../target/release/quorum \
  ../../target/release-assets/quorum-0.1.1-linux-amd64
```

Verify the staged outputs against the documented release checksums:

```bash
cd ../../target/release-assets
sha256sum -c SHA256SUMS
```

Also verify both archive manifests before upload:

```bash
tar -xOf lez-quorum-basecamp-0.1.1-linux-amd64-dev.lgx manifest.json
tar -xOf lez-quorum-basecamp-0.1.1-linux-amd64-portable.lgx manifest.json
```

## Local Validation

On 2026-09-10 all four locked flake outputs (`generate`, `lib`, `lgx`, and
`lgx-portable`) built successfully. Both LGX manifests reported version
`0.1.1`, manifest format `0.3.0`, the expected QML view, and the expected
native entry point.

The complete official standalone Basecamp host closure was then built and run
with `QT_QPA_PLATFORM=offscreen`. It loaded the built-in capability module and
started a persistent child process with these resolved arguments:

```text
ui-host --name quorum_ui --path <plugin-dir>/quorum_ui_plugin.so
```

The bounded 60-second smoke test ended only when its timeout stopped the live
host. There were no module-loader, QML, or unresolved-library errors. This
validates the local build and load path. The published assets were also
downloaded into a clean temporary directory on 2026-09-14; all checksums and
portable manifest fields matched this document, and the downloaded CLI reported
`quorum 0.1.1`.

## Release Notes

Quorum 0.1.1 provides a native and portable Basecamp QML module for the private
M-of-N LEZ treasury. It adds complete circuit, privacy, security, limitation,
integration, error, benchmark, deployment, and demonstration documentation.
The module supports local and LEZ testnet workflows, forces real proofs for UI
operations, previews public transactions before confirmation, reports proof
phases, and exposes transaction reconciliation.

This release is experimental and unaudited. Do not use it to custody assets of
material value.

## Publication

The `v0.1.1` release is published at
<https://github.com/lezQuorm/LEZ-Quorum/releases/tag/v0.1.1> with:

- both `.lgx` files;
- `SHA256SUMS`; and
- `quorum-0.1.1-linux-amd64`.

Then download the public assets into a clean directory, rerun
`sha256sum -c SHA256SUMS`, load the portable LGX in Basecamp, point it at the
matching CLI, and update the submission with direct release links.
