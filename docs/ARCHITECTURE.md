# Conclave — Architecture

*Status: draft (Chunk 0). Expanded as each chunk lands.*

## Components

```
                        ┌──────────────────────────────┐
                        │         conclave-cli         │  [Chunk 5]
                        │  create / propose / approve  │
                        │  rotate / execute / info     │
                        └──────────────┬───────────────┘
                                       │
                        ┌──────────────▼───────────────┐
                        │         conclave-sdk         │  [Chunk 5]
                        │ proof-gen (client-side),     │
                        │ state reads, resumable votes │
                        └──────────────┬───────────────┘
                                       │
        ┌──────────────────────────────▼──────────────────────────────┐
        │                 conclave-circuit (Risc0 guest)              │  [Chunk 3]
        │  ONE aggregated proof: M distinct nullifiers, valid Merkle  │
        │  paths in member_root, tier threshold + cap satisfied       │
        └──────────────────────────────┬──────────────────────────────┘
                                       │ receipt (SUCCINCT — recursion)
        ┌──────────────────────────────▼──────────────────────────────┐
        │               conclave-gate (SPEL program, LEZ)             │  [Chunk 4]
        │  verifies receipt on-chain in a privacy-preserving tx,      │
        │  updates nullifier set, gates the action (Transfer /        │
        │  RotateMembers / ChangeThreshold)                           │
        └──────────────────────────────┬──────────────────────────────┘
                                       │
                       ┌───────────────▼───────────────┐
                       │   LEZ runtime (token, clock)  │
                       └───────────────────────────────┘
```

Supporting crates: `conclave-core` (domain model, this repo), `lez-compat`
(LEZ v0.3 commitment/Merkle semantics), `conclave-image-id` (verifier
constants), `conclave-verifier` (off-chain receipt verification).

## Data flow (2-of-3 treasury transfer)

1. **Create** — deploy `conclave-gate`; initialize a Constitution: `threshold=2,
   member_count=3, member_root=<Merkle root over member commitments>`, tiers.
2. **Propose** — member builds a `Transfer { recipient, amount, tier_id }`
   proposal (public action — per spec, only identity/vote are private).
3. **Approve** — each member runs the circuit **client-side**, producing a
   nullifier + a share of the aggregated threshold proof.
4. **Verify & gate** — the receipt is submitted in a **privacy-preserving
   transaction**; `conclave-gate` verifies it against the pinned circuit ID and
   the current `member_root`, appends nullifiers, and on reaching the tier
   threshold emits the gated action (token transfer / rotation / threshold change).
5. **Rotate** (Idea 02 differentiator) — a `RotateMembers` proposal swaps
   `member_root` for a new one in the same verified transition. Old members'
   nullifiers/keys become unprovable; a marker-PDA re-derived under the **old**
   threshold lands on an **unclaimed** address — on-chain proof the old set is dead.

## Restart-safe approvals

The on-chain nullifier set is the source of truth. A member who approves and
crashes re-reads program state and resumes; partial approvals (< M) are never
lost because they are never stored only client-side.

## Error contract

`ConclaveError` (crates/conclave-core/src/error.rs) defines deterministic codes
(`1001`–`1013`) shared by circuit, program, SDK, and CLI — satisfying the LP-0002
reliability criterion.
