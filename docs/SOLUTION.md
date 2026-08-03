# Solution: LP-0002 — Quorum, a Private M-of-N Multisig for LEZ

**Submitted by:** FidelCoder

## Summary

Quorum is a privacy-first treasury primitive for the Logos Execution Zone (LEZ):
M-of-N shielded members approve proposals, **nobody learns who voted or who is
in the set**, and — the differentiator — membership **evolves privately**
(rotation) under **tiered spending** policies.

A Risc0 guest proves that *M distinct* approvals came from members committed in
the current shielded member-set Merkle root, that the proposal is valid for the
requested spending tier (threshold + amount cap), and that every approval is
fresh (nullifier never used before). The on-chain SPEL program verifies one
aggregated receipt and records only a **nullifier set** — never member
identities.

## Repository

- **Repo:** <https://github.com/FidelCoder/LEZ-Quorum>
- **License:** MIT OR Apache-2.0

## The problem

Teams change. Someone leaves, someone joins, the rule changes from 2-of-3 to
3-of-5. Today that means:

1. Creating a brand-new multisig and moving all the funds; and
2. On public chains, every membership change publishes your org chart — who's
   in, who's out, when.

A treasury that can't evolve its membership privately is a governance
straitjacket. Teams keep stale signer sets long past safety (a departed
employee still holds a key) because rotation is too costly and too public.

The public [`lez-multisig`](https://github.com/jimmy-claw/lez-multisig) PoC
additionally requires members to be **fresh zero-nonce keypairs claimed by the
program** — a constraint shielded (private) LEZ accounts cannot satisfy,
because they are owned by the privacy protocol and increment nonce on every
use. Quorum is built for shielded accounts from the ground up.

## Approach

Quorum implements the LP-0002 core (shielded commitments, threshold proofs,
nullifiers) plus a **constitution**:

- **Member rotation** — add or remove a member through the same private M-of-N
  approval flow. The old set is revoked, the new set is committed; the only
  on-chain artifact is a new commitment root. An observer cannot even tell
  whether the set changed, let alone who's in it (*plausible continuity*).
- **Threshold change** — move from 2-of-3 to 3-of-5 with a
  threshold-approved proposal.
- **Tiered spending** — separate shielded policies per spending category
  (e.g. ops = 2-of-3, strategic = 3-of-5, emergency = 2-of-5 + timelock).
  Categories are commitments, never plaintext.

**Privacy model:**

| Leak surface (public PoC) | Quorum |
|---|---|
| Member list on-chain | Only a **Merkle root** over member commitments |
| Every vote attributed | Only a **nullifier set** + one aggregated ZK threshold proof |
| Rotation publishes the org chart | New root, nothing else — *plausible continuity* |
| One fixed rule | **Constitution**: tiers, caps, threshold changes gated by the same private flow |

**Proof flow (client-side):** each approving member runs the Risc0 guest with
their member secret, the current member root, and their Merkle path. The proof
is an aggregated threshold proof covering M distinct approvals. The on-chain
SPEL verifier checks the receipt against the pinned image ID and records the
nullifiers; a duplicate nullifier is rejected deterministically (error 1005).

## Success Criteria Checklist

See [`criteria-checklist.md`](../criteria-checklist.md) for the full 1:1 map to
`logos-lambda-prize/prizes/LP-0002.md`.

- [x] Any M-of-N member holding a shielded LEZ account can submit an approval
  without revealing identity to on-chain observers or other members (ZK
  threshold proof; member root commitment, never plaintext).
- [x] On-chain verifier confirms a threshold of M approvals without recording
  which members approved (nullifier set only).
- [x] Double-vote prevention via nullifiers (`nullifier = H(member_secret,
  proposal_id, version)`; program rejects duplicates, error 1005).
- [x] Completed execution unlinkable to any individual member's shielded
  account (single aggregated proof).
- [x] Proof generation runs client-side on a standard laptop
  (see [`docs/BENCHMARKS.md`](BENCHMARKS.md)).
- [x] Reference integration: threshold-gated treasury transfer + rotation via
  the CLI/SDK, verified end-to-end locally
  ([`scripts/demo.sh`](../scripts/demo.sh)); on-chain testnet deploy pending a
  funded wallet (`docs/KNOWN_LIMITATIONS.md` #2).
- [x] Reproducible end-to-end flow with evidence
  ([`scripts/regenerate-evidence.sh`](../scripts/regenerate-evidence.sh)).
- [x] Full documentation + clean public repository (CI green, clippy clean
  under `-D warnings`).
- [x] Module/SDK for Logos modules: `quorum-sdk` (Rust).
- [x] SPEL IDL for the LEZ program: `programs/quorum-gate/idl/quorum_gate.idl.json`.
- [x] Deterministic, documented error codes for all invalid-proof and
  double-vote scenarios ([`docs/ERROR_CODES.md`](ERROR_CODES.md)).
- [x] Partial approvals (< M) preserved and resumable across client restarts
  (on-chain nullifier set is the source of truth).

## FURPS Self-Assessment

### Functionality

Quorum covers multisig creation, private approval proving, threshold
enforcement on-chain, shielded member rotation, tiered spending policies, and
restart-safe partial approvals.

### Usability

The CLI (`quorum create / propose / approve / execute / reject / info /
new-root`), the Rust SDK, the demo script, and task-specific documentation
cover the treasury operator, member, and developer flows.

### Reliability

Deterministic error codes (1001–1013), atomic rotation (old root retired →
removed key provably dead via nullifier), restart-safe state (on-chain
nullifier set as source of truth), and strict CI (fmt + clippy `-D warnings` +
tests) protect every path.

### Performance

Real proof sizes and timings are reported in
[`docs/BENCHMARKS.md`](BENCHMARKS.md). The current LEZ testnet RPC does not
expose per-transaction CU/gas-used metadata (same limitation the LP-0005
solution reported); Risc0 cycles are documented separately.

### Supportability

The workspace separates core domain (`quorum-core`), LEZ compatibility
(`lez-compat`), circuit (`quorum-circuit`), proving (`quorum-prover`), gate
logic (`quorum-gate-core`), SPEL program (`programs/quorum-gate`), SDK
(`quorum-sdk`), and CLI (`quorum-cli`). Local strict CI and reproducible
evidence are part of the repository.

## Validation

- Strict Clippy (`-D warnings`) and workspace tests pass (54 tests, dev-mode
  proofs for speed).
- Real proofs generated with `RISC0_DEV_MODE=0` (2-of-3, receipt verified,
  receipt size ~219 KiB).
- End-to-end demo verified: create → propose → approve ×2 → execute →
  rotate member → constitution v2 → execute.
- No mock proofs or placeholder artifacts in the **real-proof** evidence path
  (the fast local demo uses dev-mode proofs by design and says so).

| Measurement | Result |
| --- | ---: |
| Real 2-of-3 threshold proof (single approval) | ~368 s |
| Receipt size (bincode) | 224,346 bytes |
| Pinned image ID | `[2504793846, 1302641585, 509407582, 452779787, 1019694882, 662766674, 1532127949, 2008668271]` |

## Privacy And Security

**Hidden:** member identities and membership set (only the Merkle root is
public); who approved which proposal (nullifier set only); individual approval
proofs (aggregated into one receipt); rotation history (new root, nothing
else).

**Public:** member-set Merkle root; nullifier set; proposal actions and amounts
(execution transparency); constitution version and tier policies (committed).

Quorum has not received an independent security audit.

## Reproduction

```bash
git clone https://github.com/FidelCoder/LEZ-Quorum.git
cd LEZ-Quorum
cargo build -p quorum-cli
# fast local demo (dev-mode proofs):
RISC0_DEV_MODE=1 ./scripts/demo.sh
# real proof (takes minutes):
RISC0_DEV_MODE=0 cargo run -p quorum-prover --example prove_threshold --release
# full strict validation:
cargo fmt --check && RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets -- -D warnings && RISC0_DEV_MODE=1 cargo test --workspace
```

## Impact

Quorum makes private, evolvable multisig governance practical: teams can
rotate members and change thresholds without leaking their org chart, while a
constitution keeps spending rules explicit, tiered, and enforceable on-chain.

## Terms & Conditions

By submitting this solution, I confirm that I have read and agree to the
[Lambda Prize Terms and Conditions](https://github.com/logos-co/lambda-prize/blob/master/TERMS.md).
