# Quorum — Privacy Model

## What is hidden

| Surface | Leak in the public PoC | Leak in Quorum |
|---|---|---|
| Member set | Full member list on-chain | A **Merkle root** over member commitments only |
| Votes | Each approval attributed to a member | A **nullifier set** — reveals nothing about identity |
| Membership changes | Org chart published | A new root; observers can't even tell the set changed |
| Approval timing | Linked to a member's tx | Submitted in **privacy-preserving transactions** (sender hidden by LEZ) |
| Member secrets | — | Never leave the member's device; only commitments/nullifiers ever appear |

## How it's achieved

- **Shielded membership.** Each member commits to an identity secret:
  `member_commitment = H("conclave/v1/member" ‖ secret)`. The multisig stores only
  `member_root = MerkleRoot(member_commitments)`. Membership is proven in-ZK with a
  Merkle path (`quorum-circuit`), so no plaintext member list exists anywhere on-chain.

- **Anonymous approval.** A member's approval is a Risc0 proof that their commitment
  is in the root and that their **nullifier** is correctly derived:
  `nullifier = H("conclave/v1/nullifier" ‖ secret ‖ proposal_id ‖ version)`. The
  nullifier is public (it must be, for double-vote prevention) but is a one-way
  hash of the secret — unlinkable to the member.

- **On-chain sees only "threshold reached".** The gate aggregates nullifiers and
  exposes `approvals = n/ threshold`. It never records *who* approved. Per LP-0002,
  the proposed *action* is public by design — only identity and vote are private.

- **Evolving anonymity (rotation).** Rotation replaces `member_root`. Because a
  removed member's commitment is no longer in the tree, their key is provably dead
  (no valid Merkle path), and no on-chain artifact ties the change to a person.

## What is public (by design)

- Constitution: threshold, member count, tier limits, `member_root`.
- Proposal: id, action, nullifier set, status.
- Proof: journal (member_root, proposal_id, version, threshold, nullifiers, action).

## Unlinkability guarantees

1. A nullifier is derived from the secret + proposal id: the same member produces
   a **different** nullifier per proposal (tested in `quorum-circuit`).
2. Approvals travel in private LEZ transactions: the submitter's account is hidden
   by the protocol.
3. Membership proofs reveal only "a path exists to `member_root`" — not which leaf.

## Model summary

```
member secret ──► member commitment ──► member_root (public, shielded set)
      │                                              │
      └──► nullifier (public, identity-free) ──► gate aggregates → threshold
```
