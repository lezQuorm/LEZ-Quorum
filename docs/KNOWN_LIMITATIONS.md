# Known Limitations

Quorum is an experimental implementation, not a production-ready treasury.

## Deployment and lifecycle evidence

The gate has not been deployed to a standalone sequencer or public LEZ testnet.
There is no funded vault, deployed program ID, account sequence, transaction
hash set, or network cost record in this repository. The optional RPC client
compiles and implements submit-once, hash confirmation, and public state reads,
but it has not been exercised against a running sequencer here.

## Wallet integration

The composer accepts current public and private account witnesses prepared by
a wallet. Constructing those witnesses for an existing private credential
requires the wallet's encrypted-state scan and commitment membership proof.
The offline CLI is still a local state mirror and does not fetch or reconcile
live chain state.

## Basecamp package

The Basecamp directory contains the QML view, Qt Remote Objects contract,
native `QProcess` backend, CMake project, and Nix flake. This environment lacks
Nix, CMake, and Qt development packages, so no LGX artifact has been built,
installed, or tested. The UI currently drives the offline CLI; live wallet and
composer operations need a supported Basecamp/LEZ wallet binding.

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

The credential-aware aggregated proof takes about ten minutes on the recorded
machine. The outer LEZ privacy proof, sequencer verification cost, transaction
compute use, confirmation latency, and fees have not been measured in real mode
against a network.

## Compatibility

LEZ v0.2.0, SPEL commit `0cb7e098`, and Risc0 3.0.5 are pinned. Upgrade work
must regenerate both circuit image IDs and IDL, then rerun real proofs,
composition tests, and network lifecycle tests.
