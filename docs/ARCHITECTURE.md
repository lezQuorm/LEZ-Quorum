# Quorum — Architecture

## Components

```
                        ┌──────────────────────────────┐
                        │         quorum-cli         │  [Chunk 5]
                        │  create / propose / approve  │
                        │  rotate / execute / info     │
                        └──────────────┬───────────────┘
                                       │
                        ┌──────────────▼───────────────┐
                        │         quorum-sdk         │  [Chunk 5]
                        │ proof-gen (client-side),     │
                        │ state reads, resumable votes │
                        └──────────────┬───────────────┘
                                       │
        ┌──────────────────────────────▼──────────────────────────────┐
        │                 quorum-circuit (Risc0 guest)              │  [Chunk 3]
        │  ONE aggregated proof: M distinct nullifiers, valid Merkle  │
        │  paths in member_root, tier threshold + cap satisfied       │
        └──────────────────────────────┬──────────────────────────────┘
                                       │ receipt (SUCCINCT — recursion)
        ┌──────────────────────────────▼──────────────────────────────┐
        │               quorum-gate (SPEL program, LEZ)             │  [Chunk 4]
        │  verifies receipt on-chain in a privacy-preserving tx,      │
        │  updates nullifier set, gates the action (Transfer /        │
        │  RotateMembers / ChangeThreshold)                           │
        └──────────────────────────────┬──────────────────────────────┘
                                       │
                       ┌───────────────▼───────────────┐
                       │   LEZ runtime (token, clock)  │
                       └───────────────────────────────┘
```

Supporting crates: `quorum-core` (domain model, this repo), `lez-compat`
(LEZ v0.3 commitment/Merkle semantics), `quorum-image-id` (verifier
constants), `quorum-prover` (host-side proving + off-chain receipt
verification).

## Data flow (2-of-3 treasury transfer)

1. **Create** — deploy `quorum-gate`; initialize a Constitution: `threshold=2,
   member_count=3, member_root=<Merkle root over member commitments>`, tiers.
2. **Propose** — member builds a `Transfer { recipient, amount, tier_id }`
   proposal (public action — per spec, only identity/vote are private).
3. **Approve** — each member runs the circuit **client-side**, producing a
   nullifier + a share of the aggregated threshold proof.
4. **Verify & gate** — the receipt is submitted in a **privacy-preserving
   transaction**; `quorum-gate` verifies it against the pinned circuit ID and
   the current `member_root`, appends nullifiers, and on reaching the tier
   threshold applies the gated action. A `Transfer` is **executed by chaining
   into the treasury vault's token program** (`ChainedCall`): the gate marks
   the proposal executed and emits the call, the vault holding is authorized
   via its PDA seed, and the token program moves the funds — the gate never
   handles balances itself.
5. **Rotate** (Idea 02 differentiator) — a `RotateMembers` proposal swaps
   `member_root` for a new one in the same verified transition. Old members'
   nullifiers/keys become unprovable; a marker-PDA re-derived under the **old**
   threshold lands on an **unclaimed** address — on-chain proof the old set is dead.

## On-chain transfer execution

A treasury `Transfer` is not executed by the gate directly (it owns no
balances). On `Execute`, the SPEL guest:

1. Derives the treasury vault PDA seed: `SHA256("quorum/vault/v1" || multisig_id)`
   (`quorum_gate_core::vault_pda_seed`) and rejects a caller-supplied vault that
   is not this PDA (`GateError::InvalidVault`, 4012).
2. Serializes `token_core::Instruction::Transfer` via a serde mirror
   (`TokenTransferInstruction`; byte-identical under risc0 serde) with the
   action's `amount`.
3. Emits `ChainedCall { program_id: vault.program_owner, pre_states:
   [vault(authorized), recipient], pda_seeds: [vault_seed] }` — the token
   program moves `amount` from the vault holding to the recipient. Governance
   actions (rotation / threshold change) emit no call.

The vault account itself is created and funded as a deployment step (see
`docs/DEPLOYMENT.md`); the gate only ever *authorizes* its movement.

## Restart-safe approvals

The on-chain nullifier set is the source of truth. A member who approves and
crashes re-reads program state and resumes; partial approvals (< M) are never
lost because they are never stored only client-side.

## Error contract

`QuorumError` (crates/quorum-core/src/error.rs) defines deterministic codes
(`1001`–`1013`) shared by circuit, program, SDK, and CLI — satisfying the LP-0002
reliability criterion.
