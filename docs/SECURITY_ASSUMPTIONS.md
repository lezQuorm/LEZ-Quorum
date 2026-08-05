# Security Assumptions

Quorum is a research implementation and has not been independently audited. A
deployment must review these assumptions against the exact pinned LEZ, SPEL,
NSSA, and Risc0 revisions.

## Cryptography and composition

| Assumption | Consequence if false |
|---|---|
| SHA-256 is collision and preimage resistant | Credential commitments, Merkle roots, nullifiers, and PDA derivation may be forgeable |
| Risc0 receipts and assumptions are sound | An attacker may claim approvals or program execution without valid witnesses |
| The pinned threshold image ID is correct | The gate may verify a different statement |
| LEZ private account derivation and privacy circuit are correct | Credential control or transaction privacy may fail |
| Randomness and protected storage remain secure | Member and signer credentials may be stolen |

`RISC0_DEV_MODE=1` is only for tests. The Basecamp backend forces dev mode off,
and the real-proof path rejects development mode.

## State and account assumptions

- Constitution and proposal accounts are owned by the deployed gate program.
- Every proposal remains bound to its multisig ID and creation-time version.
- Tier policy in the constitution is authoritative.
- The treasury vault is the PDA derived from the multisig ID and is initialized
  under the intended token program.
- Credential account identities supplied to the privacy circuit are current,
  and wallets construct valid membership proofs for credential updates.
- Runtime serialization matches the pinned dependencies and committed IDL.

The gate checks proposal, policy, recipient, vault, nullifier, and credential
bindings. The composer verifies the threshold artifact before proving the gate
and refuses proposal or credential substitution.

## Operational assumptions

- Fewer than the configured threshold of member secrets are compromised.
- Replacement credentials are distributed privately and old credentials are
  retired after rotation.
- Backups, recovery, and key deletion follow an operator-approved policy.
- Proving and wallet hosts do not leak witnesses.
- Confirmation timeouts are reconciled by transaction hash before retrying.
- Network timing and metadata exposure are acceptable for the deployment's
  threat model.

## Remaining assurance work

The local proof and composition boundary is tested, including missing,
wrong-image, malformed, and substituted inputs. A standalone sequencer
lifecycle covers deployment, treasury initialization and funding, private
approval, execution, confirmation, and final state reads. Native and portable
Basecamp packages also build from the pinned Nix closure. Public testnet
behavior, existing-account wallet scans, failure retries, production state
reconciliation, and an independent circuit and program audit remain required
before production use.

Quorum does not hide proposal contents, approval count, policy changes, or
rotation timing. It cannot prevent approval withholding, loss of credentials,
or collusion at or above the configured threshold.
