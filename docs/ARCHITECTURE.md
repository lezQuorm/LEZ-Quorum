# Architecture

Quorum adds private M-of-N authorization to an LEZ treasury.

## Flow

![LEZ-Quorum system flow](assets/architecture-flow.svg)

1. A member proves credential control and membership under the constitution
   root.
2. `quorum-composer` verifies the threshold receipt, runs the gate, and wraps
   the result in an LEZ private transaction.
3. The sequencer verifies the proof and records a proposal-scoped nullifier.
4. Once M distinct approvals exist, execution applies the action and closes the
   proposal.

## State

| Constitution | Proposal |
|---|---|
| Multisig ID and version | Action and constitution version |
| Threshold and member count | Required approvals |
| Member root and spending tiers | Approval nullifiers |
| Proposal counter | Active or Executed status |

The constitution stores a member root, not a member list. Rotation changes the
root and version, invalidating old credentials and open proposals.

The treasury vault is derived from the multisig account:

```text
vault_seed = SHA256("quorum/vault/v1" || multisig_account_id)
```

## Privacy

| Private | Public |
|---|---|
| Member secrets and account IDs | Multisig and proposal IDs |
| Member list and Merkle paths | Member root, count, threshold, and tiers |
| Approval ownership | Approval count and nullifiers |
| Credential state | Proposal action, status, and result |

The receipt journal contains the proposal binding, action, approval count,
nullifiers, member root, and credential commitments. It does not contain secret
keys, account IDs, the member list, or Merkle paths.

## Components

| Component | Responsibility |
|---|---|
| `quorum-core` | Member commitments, proposals, nullifiers, and policy |
| `lez-compat` | LEZ account derivation and compatibility |
| `quorum-circuit` | Threshold statement |
| `quorum-prover` | Threshold proving and verification |
| `quorum-gate-core` | Gate state and validation |
| `quorum-gate` | SPEL program and IDL |
| `quorum-composer` | Proof composition and LEZ transactions |
| `quorum-sdk` / `quorum-cli` | Client workflow and transaction journal |
| `basecamp-quorum` | Basecamp interface |

## Security

Security depends on SHA-256, Risc0 receipt verification, pinned guest image IDs,
LEZ account derivation, the LEZ privacy circuit, and protected credential
storage. `RISC0_DEV_MODE=1` produces test receipts only.
