# Known Limitations

Quorum is an experimental implementation, not a production-ready treasury.

## Deployment and lifecycle evidence

The gate and token lifecycle have been exercised against a persistent local LEZ
v0.2.0 standalone sequencer in development-proof mode. The run deploys the
program, initializes and funds the treasury PDA, records a private threshold
approval, executes a transfer, and verifies final public state through RPC. No
public LEZ testnet deployment or public-network cost record exists yet.

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
headlessly without a load error. Installation, visual interaction, and capture
inside the released Basecamp desktop still require a GUI session; live wallet
and composer operations require a supported Basecamp/LEZ wallet binding.

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

Real Risc0 proving is CPU intensive and takes minutes per aggregated private
approval on the recorded machine. A full real-mode lifecycle attempt reached
approximately 8.6 GiB resident memory in the final privacy-wrapper prover and
was killed when the 15 GiB workstation and its 4 GiB swap were exhausted by
the concurrent desktop session. Operators must provide sufficient free memory
or swap before running the nested real-proof path. Local sequencer confirmations
have been exercised; public-network compute accounting, latency, and fees remain
network-specific and have not been measured.

## Compatibility

LEZ v0.2.0, SPEL commit `0cb7e098`, and Risc0 3.0.5 are pinned. Upgrade work
must regenerate both circuit image IDs and IDL, then rerun real proofs,
composition tests, and network lifecycle tests.
