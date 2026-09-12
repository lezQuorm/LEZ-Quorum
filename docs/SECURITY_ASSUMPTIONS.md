# Security Assumptions

Quorum is experimental and unaudited. The testnet result demonstrates
interoperability, not production security. Do not use this release to custody
assets of material value.

## Cryptographic Assumptions

The design assumes:

- SHA-256 provides collision resistance, preimage resistance, and adequate
  domain separation for commitments, Merkle nodes, and nullifiers;
- Risc0 3.0.5's RISC-V and recursion STARKs are sound and zero knowledge as
  used by succinct receipts;
- Risc0 receipt serialization, image-ID binding, and recursive assumptions are
  implemented correctly; and
- the LEZ v0.2.2 privacy circuit correctly proves private account ownership and
  valid account transitions.

Quorum uses `ProverOpts::succinct()`, not the Groth16 wrapper. There is no
Quorum-specific setup ceremony and the current receipt path does not rely on
Risc0's Groth16 trusted setup. A future switch to Groth16 receipts would add
that ceremony to the trust model and require a documented upgrade.

## Software And Supply Chain

Security depends on the exact source, build inputs, and program artifacts:

- host Rust `1.91.0`, Risc0 `3.0.5`, and guest Rust `1.97.0` are pinned in CI;
- LEZ integration is pinned to commit
  `d6e4ae694e7419f5906b340c232704466a1917b7` (`v0.2.2`);
- the gate embeds one threshold guest image ID at compile time;
- clients must verify the deployed gate program and local ELF hashes before
  submitting; and
- Basecamp inputs are locked in `apps/basecamp-quorum/flake.lock`.

The dependency graph includes upstream RustSec advisories for versions pinned
by Logos/Risc0 compatibility. These cannot be treated as resolved until
compatible upstream releases are adopted and retested. Run `cargo audit` and
review every advisory before any deployment.

## Protocol Invariants

The security argument relies on all of these checks remaining present:

1. member commitments bind complete LEZ private account identities;
2. each Merkle path resolves to the live constitution root;
3. nullifiers bind member secret, proposal ID, and constitution version;
4. receipt image ID and exact journal are verified by the gate assumption;
5. credential commitments match authorized private accounts in the outer LEZ
   proof;
6. action, root, version, proposal ID, tier cap, and threshold match live state;
7. duplicate nullifiers are rejected both within and across receipts;
8. the vault and recipient equal the deterministic and approved accounts; and
9. execution changes proposal status so the action cannot execute twice.

Host preflight checks improve error reporting but are not a security boundary.
The guest and LEZ sequencer must enforce every consensus-relevant invariant.

## Key Management

Member secrets must be independently generated with a cryptographically secure
random source and held only by the intended members. A stolen member bundle is
equivalent to a stolen approval credential. Quorum has no revocation service;
recovery requires an approved member rotation while the remaining threshold is
still available.

The CLI's restrictive file mode and Git ignores reduce accidental exposure but
do not encrypt secrets at rest. Operators are responsible for device security,
backups, distribution, deletion, and any hardware-wallet integration.

## Operational Assumptions

- Operators verify the target network ID, RPC endpoint, program ID, account
  IDs, transaction preview, and transaction hash before confirming a write.
- Enough honest members review the public action rather than approving blindly.
- At least the threshold number of credentials remain available.
- The sequencer provides the finality and availability expected by LEZ.
- RPC responses are checked against transaction hashes and bytes; a malicious
  endpoint can still delay, censor, or withhold responses.
- Development receipts never reach a network whose verifier accepts them.

The CLI enforces a real-proof policy for `--target testnet`, but a modified
client is outside that policy boundary. Network verifier configuration remains
authoritative.

## Upgrade Assumptions

Changing the threshold guest changes its image ID. Changing the gate changes
the deployed program ID or artifact hash. Such changes require a new reviewed
deployment, updated evidence, and regeneration of client constants. Existing
constitutions are not automatically migrated.

Constitution versioning handles member and threshold changes within the same
gate protocol. It is not a general software upgrade mechanism.

## Audit Status

Coverage includes unit, integration, CLI, gate, proof, standalone sequencer,
and testnet lifecycle evidence. It does not replace:

- an independent circuit and protocol audit;
- an LEZ/SPEL integration audit;
- a reproducible-build and dependency review;
- adversarial network testing; or
- production key-management review.

See [Known Limitations](KNOWN_LIMITATIONS.md) and
[Deployment](DEPLOYMENT.md) before reproducing the testnet flow.
