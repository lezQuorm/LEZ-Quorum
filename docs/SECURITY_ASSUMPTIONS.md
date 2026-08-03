# Quorum — Security Assumptions

This document states the explicit trust and security assumptions of Quorum.
Reviewers should verify these hold for their deployment.

## Trust model

| Assumption | Detail |
|---|---|
| **Member secrets stay secret** | Each member's `member_secret` (committed via `H(secret)`) is held client-side. A leaked secret lets an attacker produce approvals for that member only. |
| **Threshold of honest members** | Security of the multisig requires at least `threshold` members to be honest. M-of-N with M≤N: compromise of ≥M members breaks the gate by definition (same as any multisig). |
| **Commitment root is authentic** | The member-set Merkle root is committed by the constitution at creation and after each rotation, and verified by the circuit. If an attacker could change the root without a threshold-approved proposal, they could add themselves. The SPEL program only mutates the root through `RotateMembers` proposals. |
| **Risc0 is sound** | Security rests on Risc0's STARK→SNARK recursion (`RISC0_DEV_MODE=0` only). Dev-mode receipts are explicitly rejected in production paths. |
| **Image ID is pinned** | On-chain verification checks the receipt against the pinned image ID (`quorum-image-id`). Anyone who could change the pinned ID could change the circuit semantics. |
| **LEZ privacy protocol is sound** | Shielded accounts, commitments, and nullifiers follow the LEZ privacy protocol (`/LEE/v0.3/Commitment/`). Quorum inherits LEZ's guarantees. |
| **Host is trusted** | Proof generation runs client-side; a compromised host can refuse to prove but cannot forge a proof without the member secret. |

## What is NOT protected

- **Proposal contents** (amounts, recipients, actions) are public by design
  after execution — this is execution transparency, not identity leakage.
- **Timing / metadata correlation** — the nullifier set and proposal IDs are
  public; sophisticated network-level correlation is out of scope (standard
  for privacy protocols without a mixing layer).
- **Denial of service** — a member can simply not approve. This is a feature
  (a veto), not a vulnerability.
- **Social engineering of keyholders** — no scheme can prevent a member from
  approving under duress; the tiered constitution reduces blast radius.

## Design mitigations

1. **Rotation is atomic** (ADR-0004): a removed key's nullifiers are
   version-bound, so old-set approvals fail at the exact block the new root
   lands. No stale-key window.
2. **Double-vote prevention** (ADR-0002): `H(member_secret || proposal_id ||
   version)` — replaying a claim on-chain is impossible (nullifier already
   recorded, error 1005).
3. **Tiered caps** (B2): each spending tier has a max amount; a tier-threshold
   proof is rejected if the proposed amount exceeds the tier cap.
4. **Deterministic errors** (1001–1013): no panic paths, no error messages
   that could leak private inputs.
5. **Regenerable evidence**: `scripts/regenerate-evidence.sh` replays every
   proof and demo step, so evidence survives testnet resets.

## Audit status

Quorum has **not** received an independent security audit. The circuit is
deliberately small (SHA-256 Merkle-path evaluation + nullifier derivation) to
minimize the audit surface.
