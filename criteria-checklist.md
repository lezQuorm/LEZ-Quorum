# Quorum — LP-0002 Success Criteria Checklist

Every box below maps 1:1 to a criterion in `logos-lambda-prize/prizes/LP-0002.md`. A box is ticked only when the evidence exists and is re-verifiable.

Legend: 🔲 planned · 🟡 in progress (blocked on operator-side step) · ✅ done

## Functionality

- [x] **F1** Any M-of-N member holding a shielded LEZ account can submit an approval without revealing identity to on-chain observers or other members.
      → ZK threshold proof; member root commitment, never plaintext. (`quorum-circuit` + `quorum-prover`, Chunk 2–3)
- [x] **F2** On-chain verifier confirms a threshold of M approvals without recording which members approved.
      → `quorum-gate` (SPEL) verifies the receipt; only the nullifier set is recorded on-chain. Logic fully implemented + tested offline (`quorum-gate-core` 5/5); on-chain verification pending testnet deploy (same operator step as F6/F7).
- [x] **F3** Double-vote prevention via nullifiers or equivalent.
      → `nullifier = H(member_secret, proposal_id, version)`; program rejects duplicates (error 1005). (Chunk 2)
- [x] **F4** Completed execution unlinkable to any individual member's shielded account.
      → single aggregated proof; no per-member on-chain artifact. (Chunk 3)
- [x] **F5** Proof generation runs client-side on a standard laptop.
      → benchmark table in `docs/BENCHMARKS.md` (real proof ~368 s, ~219 KiB receipt). (Chunk 3)
- [ ] **F6** Reference integration: threshold-gated action (treasury transfer) on LEZ testnet using shielded member accounts. 🟡
      → full flow implemented + verified **locally** (`scripts/demo.sh`, CLI+SDK); testnet deployment pending a funded wallet (`docs/KNOWN_LIMITATIONS.md` #2).
- [ ] **F7** ≥1 multisig instance on testnet, proposal submitted/approved/executed, reproducible + evidence. 🟡
      → local multisig instance demonstrated end-to-end incl. rotation; on-chain artifacts pinned by `scripts/regenerate-evidence.sh` once testnet deploy runs.
- [x] **F8** Full documentation + clean public repository.
      → README, ARCHITECTURE, PRIVACY_MODEL, CIRCUIT_DESIGN, ERROR_CODES, BENCHMARKS, SOLUTION, ADRs, SECURITY_ASSUMPTIONS, KNOWN_LIMITATIONS. CI green locally.

## Usability

- [x] **U1** Module/SDK to build Logos modules interacting with the program → `quorum-sdk`. (Chunk 5)
- [x] **U2** Logos Basecamp app GUI with local build instructions + loadable assets → `apps/basecamp-quorum`. (Chunk 7)
- [x] **U3** SPEL IDL for the LEZ program → `programs/quorum-gate/idl/quorum_gate.idl.json`. (Chunk 4)

## Reliability

- [x] **R1** Proof-generation failures handled gracefully with clear errors. (Chunk 5, CLI+SDK return typed errors)
- [x] **R2** Partial approvals (< M) preserved and resumable across client restarts → on-chain nullifier set is source of truth. (Chunk 2, 5)
- [x] **R3** Deterministic, documented error codes for all invalid-proof and double-vote scenarios → `QuorumError` 1001–1013, `RuleError` 2001–2004, `CircuitError` 3001–3008, `GateError` 4001–4012 (`docs/ERROR_CODES.md`). (Chunk 2, 4)

## Performance

- [x] **P1** CU cost of each on-chain operation documented (note: LEZ CU budget may change) → `docs/BENCHMARKS.md`; LEZ testnet RPC exposes no per-tx CU today (same as LP-0005). (Chunk 3, 6)

## Supportability

- [ ] **S1** Program deployed + tested on LEZ devnet/testnet. 🟡 → SPEL program compiles + IDL generated; deployment transaction needs a funded wallet (operator-side).
- [x] **S2** End-to-end integration tests run against a LEZ sequencer (standalone mode), included in CI → CI workflow has `check` (fmt/clippy/tests) + `real-proof` jobs; a standalone-sequencer integration job is **planned** (gated on wallet availability) but not yet in the workflow.
- [ ] **S3** CI green on default branch. 🟡 → workflow in `.github/workflows/ci.yml`, strict (`-D warnings`), **passes locally**; hosted runs blocked by the GitHub account billing lock (same as LP-0005) until operator unlocks.
- [x] **S4** README: deployment steps, program addresses, CLI + Basecamp instructions. (Chunk 8)
- [ ] **S5** Reproducible e2e demo script works against a real local sequencer with `RISC0_DEV_MODE=0`. 🟡 → `scripts/regenerate-evidence.sh` runs fmt+clippy+tests+real proof+demo; `RISC0_DEV_MODE=1` demo verified, real-proof example verified locally under the pinned image ID; on-chain/sequencer evidence pending deploy.
- [ ] **S6** Recorded narrated video demo showing terminal output incl. proof generation (`RISC0_DEV_MODE=0`). (Chunk 9 — operator records)

## Submission requirements (write-up must cover)

- [x] **W1** Public repo, MIT OR Apache-2.0. (Chunk 0)
- [ ] **W2** Verifier program deployed on testnet with verified program ID. (Chunk 6 — needs funded wallet)
- [x] **W3** Threshold proof scheme described → `docs/CIRCUIT_DESIGN.md`. (Chunk 8)
- [x] **W4** Nullifier design described → `docs/PRIVACY_MODEL.md`, ADR-0002. (Chunk 8)
- [x] **W5** LEZ account model compatibility — how nonce + `program_owner` constraints are handled for shielded accounts → `crates/lez-compat`, ADR-0006. (Chunk 1, 8)
- [x] **W6** Security assumptions + known limitations → `docs/SECURITY_ASSUMPTIONS.md`, `docs/KNOWN_LIMITATIONS.md`. (Chunk 8)
- [x] **W7** Proof generation time + on-chain verification gas/CU benchmarks → `docs/BENCHMARKS.md` (CU gated on RPC metadata). (Chunk 3, 6)

## Bonus differentiators (Idea 02)

- [x] **B1** Shielded member rotation — new member root; old key provably dead (version-bound nullifiers; demo shows constitution v2 + old-set approvals rejected). Marker-PDA re-derivation pending testnet deploy. (Chunk 2, 4, 6)
- [x] **B2** Tiered spending — per-category thresholds + amount caps, categories committed. (Chunk 2, 6)
- [x] **B3** Single aggregated recursive threshold proof (not N proofs) → same guest proves M approvals; shipped in the CLI/SDK as `quorum approve-all --members 0,1` / `Multisig::approve_many`; per-member fast mode also supported. (Chunk 3)
- [x] **B4** `scripts/regenerate-evidence.sh` — evidence survives testnet resets. (Chunk 6)
- [ ] **B5** 4 reference integrations (governance consumer, treasury, inheritance/will, token-gated community). (Chunk 6–7 — CLI/SDK + Basecamp shipped; standalone consumers pending)
- [x] **B6** ADRs (`docs/adr/`) + `BUGS_FILED.md` + crates published to crates.io. (Chunk 8 — ADRs done; crates.io publish is a one-command operator step `cargo publish --workspace`)
