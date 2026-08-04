# Security Assumptions

Quorum is a research prototype and has not been independently audited. A
production deployment must validate each assumption below against the exact
LEZ, SPEL, NSSA, and Risc0 versions in use.

## Cryptographic assumptions

| Assumption | Consequence if false |
|---|---|
| SHA-256 is collision and preimage resistant | Member commitments, Merkle roots, nullifiers, and PDA derivation may be forgeable |
| Risc0 receipts are sound | An attacker may claim approvals without valid witnesses |
| The pinned threshold image ID is correct | The gate may verify a different statement |
| Secure randomness and secret storage | Member credentials may be guessed or stolen |

RISC0_DEV_MODE=1 is for local testing only. Real proof generation rejects dev
mode, but operators must also ensure no dev receipt enters a deployment
pipeline.

## State assumptions

- The initialized constitution account ID is authentic and controlled by the
  gate program.
- Proposal accounts are created by the gate and remain bound to the owning
  multisig ID and constitution version.
- Tier policy stored in the constitution is authoritative.
- The treasury vault is the PDA derived from the multisig ID and is initialized
  under the intended token program.
- Runtime account IDs and serialization formats match the pinned LEZ
  dependencies.

The gate validates these bindings where the current API exposes them,
including proposal ID, multisig ownership, stale versions, recipient account,
tier cap, and vault PDA.

## Operational assumptions

- Fewer than the required threshold of member secrets are compromised.
- Replacement keys are distributed privately before operators depend on them.
- Backups, recovery, and retirement of old secrets are handled securely.
- The proving host and transaction composer do not leak witnesses.
- Network-level timing and metadata correlation are acceptable for the
  deployment threat model.

## Pending security boundaries

Two required bindings are not implemented:

1. Quorum member secrets are not yet linked to live shielded LEZ account
   credentials.
2. The threshold receipt is not yet attached as an assumption by a LEZ
   transaction composer.

Local state-machine tests do not close either boundary. Both require integration
tests against the current executor and a live or standalone sequencer before
the system can be evaluated as an on-chain multisig.

## Out of scope

Quorum does not hide proposal contents, approval count, policy changes, or
rotation timing. It does not prevent members from withholding approval,
approving a malicious proposal, losing credentials, or colluding at or above
the configured threshold.
