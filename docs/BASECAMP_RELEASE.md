# Basecamp Release 0.1.1

This manifest identifies the published Linux amd64 module packages and matching
CLI. Local build outputs belong under the Git-ignored `target/release-assets/`.
The published release is
<https://github.com/lezQuorm/LEZ-Quorum/releases/tag/v0.1.1>.

## Build Inputs

| Input | Value |
|---|---|
| Module version | `0.1.1` |
| Release tag / source | `v0.1.1` / [`09e3bed69b229ecbec30e8bc24fe87ba3dac46e2`](https://github.com/lezQuorm/LEZ-Quorum/commit/09e3bed69b229ecbec30e8bc24fe87ba3dac46e2) |
| Platform | `x86_64-linux` |
| `logos-module-builder` | `ed0cde56e2eae66030886abf7efff09eaadbc805` |
| `nix-bundle-lgx` | `6671436d7505e20cc568599c993b8c514bf50f94` |
| Manifest | `apps/basecamp-quorum/metadata.json` |

## Published Assets

| Asset | SHA-256 |
|---|---|
| [Native LGX](https://github.com/lezQuorm/LEZ-Quorum/releases/download/v0.1.1/lez-quorum-basecamp-0.1.1-linux-amd64-dev.lgx) | `595dd8ada37080ac25be9c386465fd69cf4a5cd201b814bcbfe3685aeec1a610` |
| [Portable LGX](https://github.com/lezQuorm/LEZ-Quorum/releases/download/v0.1.1/lez-quorum-basecamp-0.1.1-linux-amd64-portable.lgx) | `89d2496cafb1904a44827d4a598384846d409d5876adb4a11ac6510169444c40` |
| [Quorum CLI](https://github.com/lezQuorm/LEZ-Quorum/releases/download/v0.1.1/quorum-0.1.1-linux-amd64) | `7763840fbe853189e00bf3c71ef05333da0a9964688f4255fb99f7d9228254e4` |

The development bundle contains variant `linux-amd64-dev`. The portable bundle
contains `linux-amd64` and carries its non-Qt runtime libraries. Both archives
use LGX manifest version `0.3.0`, name `quorum_ui`, entry point
`quorum_ui_plugin.so`, and view `qml/QuorumView.qml`.

The module launches the Quorum CLI as a process. A user must also install or
build the matching `quorum` 0.1.1 binary and select it in the module. The LGX
does not embed the Rust CLI.

## Download And Load

On Linux amd64, download the release assets and the published
[SHA256SUMS](https://github.com/lezQuorm/LEZ-Quorum/releases/download/v0.1.1/SHA256SUMS):

```bash
mkdir -p "$HOME/Downloads/quorum-0.1.1"
cd "$HOME/Downloads/quorum-0.1.1" || exit 1
QUORUM_RELEASE_URL="https://github.com/lezQuorm/LEZ-Quorum/releases/download/v0.1.1"
for QUORUM_ASSET in \
  lez-quorum-basecamp-0.1.1-linux-amd64-dev.lgx \
  lez-quorum-basecamp-0.1.1-linux-amd64-portable.lgx \
  quorum-0.1.1-linux-amd64 SHA256SUMS; do
  curl --fail --location "$QUORUM_RELEASE_URL/$QUORUM_ASSET" \
    --output "$QUORUM_ASSET" || exit 1
done
sha256sum -c SHA256SUMS || exit 1
chmod u+x quorum-0.1.1-linux-amd64
./quorum-0.1.1-linux-amd64 --version
realpath quorum-0.1.1-linux-amd64
```

All three checksum entries must report `OK`; the CLI must report `quorum 0.1.1`.
In Basecamp's package import/file chooser, select
`lez-quorum-basecamp-0.1.1-linux-amd64-portable.lgx`. Open **Quorum Multisig**, set
**Runtime → CLI binary** to the absolute CLI path printed above, select a private
state directory, and click **Apply**. Follow [Basecamp Guide](BASECAMP_GUIDE.md)
for the Local or LEZ Testnet lifecycle. Real approvals require the Risc0 prover
runtime; a version check or module load does not run a proof.

## Rebuild

Use a separate checkout of the release tag to reproduce the package inputs:

```bash
git clone --branch v0.1.1 --depth 1 https://github.com/lezQuorm/LEZ-Quorum.git LEZ-Quorum-v0.1.1
cd LEZ-Quorum-v0.1.1
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
curl --fail --location \
  https://github.com/lezQuorm/LEZ-Quorum/releases/download/v0.1.1/SHA256SUMS \
  --output SHA256SUMS
sha256sum -c SHA256SUMS
```

Also inspect both archive manifests:

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

On **2026-09-15**, both LGX packages, the CLI, and `SHA256SUMS` were downloaded
again from the public release into a new directory. All three hashes matched
and the CLI again reported `quorum 0.1.1`. Each archive's variant was extracted
and loaded separately in the pinned official standalone Basecamp host, with
fresh XDG configuration/data/cache directories and offscreen Qt rendering.

For both variants, process inspection confirmed that `ui-host --name quorum_ui`
mapped the **downloaded** `quorum_ui_plugin.so`, and the parent host mapped the
downloaded replica factory. Each 20-second observation completed without loader,
QML, or unresolved-library errors; the check then stopped its own host process.
This verifies loading public package contents with the supported host on the
existing Linux/Nix runtime. It does not claim a fresh operating-system install,
a Basecamp store-import UI test, or another on-chain lifecycle.

## Release Notes

Quorum 0.1.1 provides a native and portable Basecamp QML module for the private
M-of-N LEZ treasury. It adds complete circuit, privacy, security, limitation,
integration, error, benchmark, deployment, and demonstration documentation.
The module supports local and LEZ testnet workflows, forces real proofs for UI
operations, previews public transactions before confirmation, reports proof
phases, and exposes transaction reconciliation.

This release is experimental and unaudited. Do not use it to custody assets of
material value.

## Related Evidence

The [deployment guide](DEPLOYMENT.md) identifies the later public commit and
passing CI run. The [demo testnet evidence](DEMO_TESTNET_EVIDENCE.md) links both
final videos and the completed transfer. The release tag above remains the
source reference for these published package bytes.
