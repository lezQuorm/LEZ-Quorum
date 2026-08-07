# Architecture

LEZ-Quorum is an M-of-N authorization layer for an LEZ treasury. Members stay
private; proposals, approval nullifiers, and execution results are public.

## Lifecycle

```text
+---------+     +-----------+     +---------------------+
| Create  | --> | Propose   | --> | Collect approvals   |
| M of N  |     | one action|     | distinct members    |
+---------+     +-----------+     +----------+----------+
                                             |
                                      approvals >= M?
                                             |
                                             v
                                  +----------+----------+
                                  | Execute             |
                                  | apply action once   |
                                  +----------+----------+
                                             |
                                             v
                                  +----------+----------+
                                  | Executed            |
                                  | proposal is closed  |
                                  +---------------------+
```

An approval does not execute a proposal. Execute succeeds only after the
proposal has collected its required number of distinct approval nullifiers.

## Proof Flow

```text
[Member wallet]
  private credential + Merkle path
              |
              v
[RISC Zero threshold guest]
  proves membership in the committed member root
  emits a proposal-bound nullifier
              |
              | threshold receipt + public journal
              v
[Quorum SPEL gate]
  binds the receipt to the constitution and proposal
  updates proposal and credential state
              |
              | gate receipt
              v
[LEZ privacy circuit]
  proves the private account state transition
              |
              | PrivacyPreservingTransaction
              v
[LEZ sequencer]
  confirms state updates and chained calls
```

`quorum-composer` verifies and connects the three proof layers. The threshold
receipt is an assumption of the gate proof; the gate receipt is an assumption
of the LEZ privacy proof.

## Execute Flow

```text
Execute(proposal)
       |
       +-- proposal exists? ---------------- no --> reject
       |
       +-- status is Active? --------------- no --> reject
       |
       +-- distinct approvals >= threshold? no --> keep Active
       |
       +-- Transfer --------> vault -> approved recipient
       |
       +-- RotateMembers ---> replace member root and member count
       |
       +-- ChangeThreshold -> replace M
       |
       +-- mark proposal Executed
```

A transfer emits a chained call from the program-derived vault to the approved
recipient. Rotation and threshold changes update the constitution. Execution is
one-shot.

## State

```text
Constitution
  +-- multisig ID
  +-- version
  +-- threshold (M)
  +-- member count (N)
  +-- member root --------> commits to the private member set
  +-- spending tiers
  +-- proposal counter
            |
            +---- Proposal
                    +-- action
                    +-- constitution version
                    +-- required threshold
                    +-- approval nullifiers
                    +-- Active | Executed | Cancelled
```

The constitution stores the member root, never the member list. Rotation
increments its version, making old credentials and proposals stale.

The treasury address is derived from the multisig account:

```text
vault_seed = SHA256("quorum/vault/v1" || multisig_account_id)
```

## Components

| Component | Responsibility |
|---|---|
| `quorum-core` | Member commitments, proposals, nullifiers, and policy |
| `lez-compat` | LEZ v0.2.2 account derivation and compatibility rules |
| `quorum-circuit` | Threshold statement shared by host and guest |
| `quorum-prover` | Threshold receipt generation and verification |
| `quorum-gate-core` | Gate state and deterministic validation |
| `quorum-gate` | SPEL program and generated IDL |
| `quorum-composer` | Threshold, gate, and LEZ proof composition |
| `quorum-sdk` / `quorum-cli` | Local workflow and state management |
| `basecamp-quorum` | QML interface with an isolated CLI process |

## Privacy Boundary

| Private | Public |
|---|---|
| Member secrets and account IDs | Multisig and proposal IDs |
| Member list and Merkle paths | Member root, count, threshold, and tiers |
| Approval ownership | Approval count and nullifiers |
| Private credential state | Proposal action, status, and transfer result |

The threshold journal contains the member root, proposal binding, action,
approval count, nullifiers, and credential commitments. It contains no secret
keys, private account IDs, member list, or Merkle paths.

## Security Boundary

Security depends on SHA-256, RISC Zero receipt soundness, pinned image IDs, LEZ
account derivation, the LEZ privacy circuit, and secure credential storage.
`RISC0_DEV_MODE=1` creates test receipts only. The project is unaudited and is
not production-ready.
