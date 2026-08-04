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

## Chunk 0 — Foundation ✅
**Goal:** Stand up the repo, plan, and criteria map; clone reference material.
- [x] Clone `jimmy-claw/lez-multisig` (public PoC) into `references/` — study its architecture (Squads-style PDAs, ChainedCall, fresh-keypair constraint).
- [x] Scaffold `Quorum/` workspace (crates/, programs/, docs/, scripts/, examples/, .github/).
- [x] Write `PLAN.md` (this file) + `criteria-checklist.md` mapped to every LP-0002 criterion.
- [x] `quorum-core` crate: domain model (Constitution, tiers, Proposal, nullifiers, deterministic error codes) — compiles.
- [x] LICENSE (MIT OR Apache-2.0), .gitignore, README, initial git commit.
- [x] Confirm current LEZ testnet version + commitment format (v0.3) before writing `lez-compat` — `/LEE/v0.3/Commitment/` everywhere in `logos-execution-zone`; testnet RPC reachable.

## Chunk 1 — LEZ v0.3 compatibility layer ✅
**Goal:** `crates/lez-compat` mirrors the CURRENT testnet commitment + Merkle semantics, plus the shielded-account nonce/`program_owner` rules.
- [x] Study `logos-execution-zone` (local) for the live commitment format, account model, and validation rules (nonce, program_owner, balance preservation).
- [x] Port/adapt the `lez-compat` approach from LEZ-TokenStudio to v0.3 (`/LEE/v0.3/Commitment/`).
- [x] Implement: private-account commitment binding, Merkle membership check, nonce handling for shielded accounts (they increment nonce on every use — the constraint the public PoC can't satisfy).
- **DoD:** unit tests pass; `cargo test` in workspace green.

## Chunk 2 — Quorum core: privacy model + state machine ✅
**Goal:** Complete the domain model: shielded member-set commitment (evolving root), proposal/approval types, nullifier design, Constitution (tiers + rotation), restart-safe approval state.
- [x] Member set as Merkle **root** over member commitments — never plaintext (`merkle.rs` `MemberTree`).
- [x] Rotation = new root; revocation atomic with new-root commitment (old key provably dead via version-bound nullifier).
- [x] Tiered spending: per-category threshold + amount cap, category labels committed.
- [x] Restart-safe partial approvals: on-chain nullifier set is the source of truth; client re-reads state on restart.
- [x] Deterministic error-code contract (established in chunk 0) fully implemented (1001–1013).
- **DoD:** unit tests for all state transitions; invariants (rotation breaks nothing, double-vote rejected).

## Chunk 3 — ZK threshold circuit (Risc0 guest) ✅
**Goal:** `crates/quorum-circuit` — ONE aggregated recursive proof proving "M distinct valid approvals from committed member root, tier threshold + cap satisfied."
- [x] Risc0 guest: verify M membership paths + M fresh nullifiers + threshold/cap enforcement in a single proof.
- [x] `quorum-image-id` constants (real image ID pinned from `RISC0_DEV_MODE=0` run); recursion so the on-chain verifier is tiny.
- **DoD:** real proofs with `RISC0_DEV_MODE=0` (verified ~368 s, 224 KB receipt); benchmarks in `docs/BENCHMARKS.md`.

## Chunk 4 — LEZ verifier program (SPEL) ✅
**Goal:** `programs/quorum-gate` — on-chain verifier that gates execution of a threshold-gated action.
- [x] SPEL program + IDL (`programs/quorum-gate/idl/quorum_gate.idl.json`), privacy-preserving verification path (proof verified in a private tx).
- [x] `quorum-gate-core`: initialize/propose/approve/execute/reject logic, nullifier bookkeeping, deterministic errors — 5/5 tests.
- [x] Marker-PDA evidence design documented (ADR-0004/0005); re-derivation runs post-testnet-deploy.
- [x] Deterministic error codes for invalid-proof / double-vote / stale-key.
- **DoD:** SPEL guest compiles; IDL generated and JSON-valid.

## Chunk 5 — SDK + CLI ✅
**Goal:** `quorum-sdk` + `quorum-cli`: proof generation, proposal submission, approve (with ZK proof), rotate members, tiered spend.
- [x] CLI: `quorum create / propose / approve / approve-all / execute / reject / info / new-root`.
- [x] SDK: `Multisig`, `MemberSet`, `approve` (per-member) + `approve_many`
  (aggregated single-proof mode) with client-side proofs, claims written as JSON artifacts.
- **DoD:** full flow reproducible from CLI (verified: create → propose → approve×2 → execute → rotate → execute).

## Chunk 6 — Reference integration + testnet evidence 🟡
**Goal:** 2-of-3 treasury transfer + rotation demo live on testnet, evidence regenerable.
- [x] Demo: create → propose transfer → 2 approvals → execute; then rotate a member → old key rejected, new set approves (`scripts/demo.sh` verified).
- [x] `scripts/regenerate-evidence.sh` — replays everything and re-pins hashes.
- [ ] Evidence doc with program ID, deployment tx, per-step txs, final state — **needs a funded testnet wallet** (`docs/KNOWN_LIMITATIONS.md` #2).
- **DoD:** every tx re-fetchable on the explorer; evidence survives a testnet reset.

## Chunk 7 — Basecamp GUI + deliverables ✅
**Goal:** Basecamp app (QML) + module/SDK packaging + IDL deliverable per Usability criteria.
- [x] QML views: create, propose, approve/execute, rotate, state — `apps/basecamp-quorum` (+ metadata.json, README with build/load instructions).
- [x] SDK packaging (`quorum-sdk`), IDL deliverable (`quorum_gate.idl.json`).
- **DoD:** loadable in Logos app (Basecamp) with local build instructions.

## Chunk 8 — Docs + evidence package ✅
**Goal:** Full write-up + the package that wins reviews.
- [x] `docs/`: ARCHITECTURE, CIRCUIT_DESIGN, PRIVACY_MODEL, ERROR_CODES, BENCHMARKS, SOLUTION, SECURITY_ASSUMPTIONS, KNOWN_LIMITATIONS (+ ADR-0001..0006).
- [x] ADRs for key decisions; `BUGS_FILED.md`; crates.io publish = one operator command (`cargo publish --workspace`).
- **DoD:** reviewer can clone → read → verify in one sitting.

## Chunk 9 — CI + submission 🟡
**Goal:** Hosted CI green, demo video, solution PR.
- [x] `.github/workflows/ci.yml`: fmt + clippy (`-D warnings`) + tests + real-proof job (RISC0_DEV_MODE=0).
- [x] Fill `solutions/LP-0002.md` template; **PR opened**:
  `logos-co/lambda-prize#120` — `Solution: LP-0002 — LEZ-Quorum: Private M-of-N
  Multisig with Shielded Rotation and Tiered Thresholds`.
- [ ] Fix GitHub billing lock so hosted jobs run (operator-side; same lock hit in LP-0005).
- [ ] Narrated video showing proof generation (`RISC0_DEV_MODE=0` in terminal).
- **DoD:** all criteria boxes checked; PR opened with complete evidence.

## Post-chunk hardening (landed after Chunk 8)

- [x] **Aggregated proof mode shipped in the CLI/SDK** (`quorum approve-all
  --members 0,1` / `Multisig::approve_many`): M approvals in ONE receipt —
  B3 no longer documented-but-unwired.
- [x] **Tier cap is constitution-authoritative**: `Multisig::propose` forces
  the tier cap from on-chain state; the gate re-checks it (`GateError::TierCapMismatch`, 4011).
- [x] **On-chain Transfer executed via ChainedCall**: the SPEL guest emits a
  token-program transfer from the treasury vault PDA (seed:
  `quorum/vault/v1`), validates the vault PDA (4012), and passes the amount
  through a serde mirror of `token_core::Instruction::Transfer`.
- [x] **Image ID re-pinned** to the current build (`[114484643, ...]`);
  `scripts/update-image-id.sh` now actually writes the pin (compute_image_id,
  no proving).
- [x] **CLI integration tests** (`crates/quorum-cli/tests/cli_flow.rs`) +
  `apps/basecamp-quorum/module.json` (clears submission-validator warnings).
- [x] `docs/DEPLOYMENT.md` + `docs/evidence/` added; doc drift fixed.

---

## Reference material
- `references/lez-multisig/` — public PoC (Squads-style, fresh-keypair constraint, ChainedCall).
- `../LEZ-TokenStudio/` — your LP-0005 repo: proven structure + ZK patterns to reuse.
- `../logos-lambda-prize/prizes/LP-0002.md` — the spec.
- `../logos-execution-zone/` — LEZ source (commitment format, validation rules).
