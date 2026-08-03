# Quorum — LP-0002 Execution Plan

**Prize:** LP-0002 — Private M-of-N Multisig ($1,200, Large)
**Idea:** Idea 02 — private multisig with shielded member rotation + tiered thresholds
**Repo (proposed):** `FidelCoder/Quorum`
**Race:** 3 open submissions (#92 Tranquil-Flow, #97 jeefxM, #115 duongja) — all flawed. First-complete wins.

## Win conditions (from LP-0005 lessons)
1. Fresh evidence on the **current** testnet (v0.3 commitment format, `/LEE/v0.3/Commitment/`).
2. **Hosted CI green** on default branch — fix the GitHub billing lock.
3. On-chain ZK verification in a **privacy-preserving transaction** as the evidence centerpiece + marker-PDA proof of the enforced threshold.
4. Per-criterion compliance map, ADRs, `BUGS_FILED.md`, crates on crates.io, narrated video with `RISC0_DEV_MODE=0` shown.
5. `scripts/regenerate-evidence.sh` — evidence survives testnet resets (what killed #92).

## Execution model
Chunks are executed **in order**. Each chunk has a Goal, Tasks, and Definition of Done (DoD). A chunk is complete only when its DoD passes (compile / test / evidence). One submission per week, max 3 per prize — submit only when every box is ticked.

---

## Chunk 0 — Foundation ✅ (in progress)
**Goal:** Stand up the repo, plan, and criteria map; clone reference material.
- [x] Clone `jimmy-claw/lez-multisig` (public PoC) into `references/` — study its architecture (Squads-style PDAs, ChainedCall, fresh-keypair constraint).
- [x] Scaffold `Quorum/` workspace (crates/, programs/, docs/, scripts/, examples/, .github/).
- [x] Write `PLAN.md` (this file) + `criteria-checklist.md` mapped to every LP-0002 criterion.
- [x] `quorum-core` crate: domain model (Constitution, tiers, Proposal, nullifiers, deterministic error codes) — compiles.
- [x] LICENSE (MIT OR Apache-2.0), .gitignore, README, initial git commit.
- [ ] Confirm current LEZ testnet version + commitment format (v0.3) before writing `lez-compat`.

## Chunk 1 — LEZ v0.3 compatibility layer
**Goal:** `crates/lez-compat` mirrors the CURRENT testnet commitment + Merkle semantics, plus the shielded-account nonce/`program_owner` rules.
- Study `logos-execution-zone` (local) for the live commitment format, account model, and validation rules (nonce, program_owner, balance preservation).
- Port/adapt the `lez-compat` approach from LEZ-TokenStudio to v0.3 (`/LEE/v0.3/Commitment/`).
- Implement: private-account commitment binding, Merkle membership check, nonce handling for shielded accounts (they increment nonce on every use — the constraint the public PoC can't satisfy).
- **DoD:** unit tests pass; `cargo test` in workspace green.

## Chunk 2 — Quorum core: privacy model + state machine
**Goal:** Complete the domain model: shielded member-set commitment (evolving root), proposal/approval types, nullifier design, Constitution (tiers + rotation), restart-safe approval state.
- Member set as Merkle **root** over member commitments — never plaintext.
- Rotation = new root; revocation atomic with new-root commitment (old key provably dead via nullifier).
- Tiered spending: per-category threshold + amount cap, category labels committed.
- Restart-safe partial approvals: on-chain nullifier set is the source of truth; client re-reads state on restart.
- Deterministic error-code contract (established in chunk 0) fully implemented.
- **DoD:** unit tests for all state transitions; invariants (rotation breaks nothing, double-vote rejected).

## Chunk 3 — ZK threshold circuit (Risc0 guest)
**Goal:** `crates/quorum-circuit` — ONE aggregated recursive proof proving "M distinct valid approvals from committed member root, tier threshold + cap satisfied."
- Risc0 guest: verify M membership paths + M fresh nullifiers + threshold/cap enforcement in a single proof.
- `quorum-image-id` constants; recursion so the on-chain verifier is tiny.
- **DoD:** real proofs with `RISC0_DEV_MODE=0`; cycle/timing benchmarks documented.

## Chunk 4 — LEZ verifier program (SPEL)
**Goal:** `programs/quorum-gate` — on-chain verifier that gates execution of a threshold-gated action.
- SPEL program + IDL (`quorum_gate.idl.json`), privacy-preserving verification path (proof verified in a private tx).
- Marker-PDA evidence: marker derived from verifier ImageID + enforced threshold; re-derive under old threshold post-rotation → unclaimed address.
- Deterministic error codes for invalid-proof / double-vote / stale-key.
- **DoD:** integration test vs standalone sequencer passes.

## Chunk 5 — SDK + CLI
**Goal:** `quorum-sdk` + `quorum-cli`: proof generation, proposal submission, approve (with ZK proof), rotate members, tiered spend.
- CLI: `quorum create / propose / approve / execute / rotate / info`.
- **DoD:** full flow reproducible from CLI on a local sequencer.

## Chunk 6 — Reference integration + testnet evidence
**Goal:** 2-of-3 treasury transfer + rotation demo live on testnet, evidence regenerable.
- Demo: create → propose transfer → 2 approvals → execute; then rotate a member → old key rejected, new set approves.
- `scripts/regenerate-evidence.sh` — replays everything and re-pins tx hashes.
- Evidence doc with program ID, deployment tx, per-step txs, final state.
- **DoD:** every tx re-fetchable on the explorer; evidence survives a testnet reset.

## Chunk 7 — Basecamp GUI + deliverables
**Goal:** Basecamp app (QML) + module/SDK packaging + IDL deliverable per Usability criteria.
- QML views: create/join multisig, propose, approve, rotate, tier config.
- **DoD:** loadable in Logos app (Basecamp) with local build instructions.

## Chunk 8 — Docs + evidence package
**Goal:** Full write-up + the package that wins reviews.
- `docs/`: ARCHITECTURE, CIRCUIT_DESIGN, PRIVACY_MODEL, ERROR_CODES, BENCHMARKS, LEZ_ACCOUNT_COMPATIBILITY (nonce + program_owner), SECURITY_ASSUMPTIONS, KNOWN_LIMITATIONS.
- ADRs for key decisions; `BUGS_FILED.md`; crates published to crates.io.
- **DoD:** reviewer can clone → read → verify in one sitting.

## Chunk 9 — CI + submission
**Goal:** Hosted CI green, demo video, solution PR.
- `.github/workflows/ci.yml`: fmt + clippy + tests + integration tests vs standalone sequencer.
- Fix GitHub billing lock so hosted jobs run.
- Narrated video showing proof generation (`RISC0_DEV_MODE=0` in terminal).
- Fill `solutions/LP-0002.md` template; open PR `Solution: LP-0002 — Quorum...`.
- **DoD:** all criteria boxes checked; PR opened with complete evidence.

---

## Reference material
- `references/lez-multisig/` — public PoC (Squads-style, fresh-keypair constraint, ChainedCall).
- `../LEZ-TokenStudio/` — your LP-0005 repo: proven structure + ZK patterns to reuse.
- `../logos-lambda-prize/prizes/LP-0002.md` — the spec.
- `../logos-execution-zone/` — LEZ source (commitment format, validation rules).
