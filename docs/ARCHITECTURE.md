# Architecture

Quorum adds private member authorization to a public LEZ treasury. The public
gate owns governance state; members use private LEZ credential accounts to
submit proposal-scoped proofs of membership.

## End-To-End Flow

![LEZ-Quorum system flow](assets/architecture-flow.svg)

1. The client derives each member's regular private LEZ account ID from its
   nullifier secret key, viewing public key, and account identifier.
2. It commits those account IDs into a canonically sorted Merkle tree. Only the
   root, member count, threshold, and spending policies enter the constitution.
3. A public proposal stores an action and the current constitution version.
4. A member proves credential derivation, Merkle membership, proposal binding,
   action policy, and a proposal-scoped nullifier in the threshold guest.
5. The composer supplies that receipt as an assumption to the outer LEZ
   privacy proof and supplies the same private credential account as an
   authorized input.
6. The gate verifies the pinned threshold image and journal, binds the journal's
   credential commitment to the outer private input, and appends new
   nullifiers to proposal state.
7. Execution requires enough accumulated nullifiers. Transfers are emitted as
   a chained call from the program-derived vault to the LEZ token program;
   governance actions mutate the constitution.

The network workflow normally submits one private member proof per approval.
The circuit and SDK also support aggregating multiple distinct approvals in a
single receipt.

## Proof Composition

```text
private member material
        |
        v
quorum-threshold guest -- succinct Risc0 receipt
        |  journal + receipt assumption
        v
LEZ privacy transaction -- proves authorized private credential input
        |
        v
quorum-gate SPEL guest -- env::verify(pinned threshold image, journal)
        |
        v
proposal nullifiers / constitution update / token chained call
```

Receipt bytes are not serialized into gate instruction data. The outer
executor carries the receipt as a Risc0 assumption, and the gate calls
`env::verify` with the compile-time `THRESHOLD_IMAGE_ID` and exact journal.
Risc0 proof composition resolves that assumption in the outer proof.

## On-Chain State

| Constitution state | Proposal state |
|---|---|
| Multisig account ID | Multisig account ID |
| Version | Constitution version at creation |
| Threshold and member count | Proposal ID and required threshold |
| Member Merkle root | Public action |
| Spending tiers | Approval nullifiers |
| Monotonic proposal counter | `Active` or `Executed` status |

The vault account is deterministic:

```text
vault_seed = SHA256("quorum/vault/v1" || multisig_account_id)
vault_id   = LEZ_PUBLIC_PDA(quorum_gate_program_id, vault_seed)
```

The gate validates this address before emitting any token transfer. A caller
cannot substitute another sender or recipient for the action that was proved.

## Versioning And Governance

Member rotation replaces the root and member count, then increments the
constitution version. A threshold change also increments the version. Open
proposals and receipts bound to an earlier version are consequently stale and
rejected. Rotation cannot retain the same root or reduce the member count below
the active threshold; threshold changes must remain within `1..=member_count`.

The deployed proposal lifecycle has two states: `Active` and `Executed`. It
does not currently provide cancellation or expiry.

## Component Boundaries

| Component | Responsibility |
|---|---|
| `quorum-core` | Pure client-side policy helpers and cryptographic derivations |
| `lez-compat` | Byte-compatible LEZ v0.2.2 account and Merkle formats |
| `quorum-circuit` | Threshold witness validation and journal construction |
| `quorum-prover` | Receipt creation and host-side verification |
| `quorum-gate-core` | Canonical deployed gate state and deterministic validation |
| `quorum-gate` | SPEL instruction handler and Risc0 assumption verification |
| `quorum-composer` | Private LEZ transaction composition and RPC client |
| `quorum-sdk` | Member material, proposal workflow, persistence API |
| `quorum-cli` | Operator workflow and crash-safe transaction journal |
| `basecamp-quorum` | QML process frontend over the CLI |

`quorum-gate-core`, not the earlier `quorum-core` convenience proposal model,
is authoritative for the deployed state layout and lifecycle.

## Trust Boundary

The protocol depends on SHA-256, correct Risc0 receipt verification, the pinned
threshold and gate image IDs, LEZ's outer privacy proof and account transition
rules, correct transaction composition, and secret credential storage. It does
not rely on a private sequencer or on obscuring public state. See
[Security Assumptions](SECURITY_ASSUMPTIONS.md) and
[Circuit Design](CIRCUIT_DESIGN.md).
