# Known Limitations

Quorum is an experimental implementation, not a production-ready treasury.

## Deployment and lifecycle evidence

The gate and token lifecycle have been exercised against the official local LEZ
v0.2.2 standalone sequencer in both development and real-proof modes. The run
deploys the program, initializes and funds the treasury PDA, records a private
threshold approval, executes a transfer, and verifies final state through RPC.
The gate is deployed on the public testnet, and a public wallet account has
been initialized and funded through the Piñata claim. A complete public Quorum
treasury lifecycle and public-network cost record do not yet exist. The
sequencer RPC confirms the deployment in block 693, while the public explorer
indexer did not yet return that block or transaction during verification.

## Wallet integration

The composer accepts current public and private account witnesses prepared by
a wallet. Constructing those witnesses for an existing private credential
requires the wallet's encrypted-state scan and commitment membership proof.
The offline CLI is still a local state mirror and does not fetch or reconcile
live chain state.

## Basecamp package

The Basecamp native and portable LGX packages build successfully with the
pinned Nix, CMake, Ninja, Qt 6.9.2, Qt QML, and Qt Remote Objects closure. The
plugins resolve their native dependencies and the QML parses with the pinned
Qt tooling. The module-builder standalone host starts the Quorum UI plugin
headlessly without a load error. Interactive execution inside the released
Basecamp desktop requires a GUI session; live wallet and composer operations
require a supported Basecamp/LEZ wallet binding.

## Key operations

The CLI protects local member secrets and rotation bundles with mode 0600, and
the Basecamp backend uses a mode-0700 working directory. Secure distribution,
backup, recovery, hardware-backed storage, and reliable deletion of retired
credentials remain operator responsibilities.

## Security assurance

No independent audit has been completed. Fuzzing of state decoding, journal
validation, and malformed instruction inputs is not yet comprehensive. Network
metadata resistance, denial of service, compromised proving hosts, and
operational governance need deployment-specific review.

## Performance

Real Risc0 proving is CPU intensive. The complete nested threshold, gate, and
privacy lifecycle passed on a 15 GiB workstation after swap was expanded to
21 GiB and competing desktop workloads were reduced. Operators must provide
similar memory headroom. Local sequencer confirmations have been exercised;
public-network compute accounting, latency, and fees remain network-specific
and have not been measured.

## Compatibility

LEZ v0.2.2 commit `d6e4ae694e7419f5906b340c232704466a1917b7`, the v0.2.2
SPEL compatibility commit `1fef85203c3130676a49aaed1b4387d16be9fe94`, and Risc0
3.0.5 are pinned. Upgrade work must regenerate both circuit image IDs and IDL,
then rerun real proofs, composition tests, and network lifecycle tests.
