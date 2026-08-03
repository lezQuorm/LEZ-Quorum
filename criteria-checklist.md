# Conclave — LP-0002 Success Criteria Checklist

Every box below maps 1:1 to a criterion in `logos-lambda-prize/prizes/LP-0002.md`. A box is ticked only when the evidence exists and is re-verifiable.

Legend: 🔲 planned · 🟡 in progress · ✅ done

## Functionality

- [ ] **F1** Any M-of-N member holding a shielded LEZ account can submit an approval without revealing identity to on-chain observers or other members.
      → ZK threshold proof; member root commitment, never plaintext. (Chunk 2–3)
- [ ] **F2** On-chain verifier confirms a threshold of M approvals without recording which members approved.
      → `conclave-gate` verifies receipt; only nullifier set on-chain. (Chunk 3–4)
- [ ] **F3** Double-vote prevention via nullifiers or equivalent.
      → `nullifier = H(member_secret, proposal_id, version)`; program rejects duplicates (error 1005). (Chunk 2)
- [ ] **F4** Completed execution unlinkable to any individual member's shielded account.
      → single aggregated proof; no per-member on-chain artifact. (Chunk 3)
- [ ] **F5** Proof generation runs client-side on a standard laptop.
      → benchmark table in `docs/BENCHMARKS.md`. (Chunk 3)
- [ ] **F6** Reference integration: threshold-gated action (treasury transfer) on LEZ testnet using shielded member accounts. (Chunk 6)
- [ ] **F7** ≥1 multisig instance on testnet, proposal submitted/approved/executed, reproducible + evidence. (Chunk 6)
- [ ] **F8** Full documentation + clean public repository. (Chunk 8)

## Usability

- [ ] **U1** Module/SDK to build Logos modules interacting with the program → `conclave-sdk`. (Chunk 5)
- [ ] **U2** Logos Basecamp app GUI with local build instructions + loadable assets. (Chunk 7)
- [ ] **U3** SPEL IDL for the LEZ program → `programs/conclave-gate/idl/conclave_gate.idl.json`. (Chunk 4)

## Reliability

- [ ] **R1** Proof-generation failures handled gracefully with clear errors. (Chunk 5)
- [ ] **R2** Partial approvals (< M) preserved and resumable across client restarts → on-chain nullifier set is source of truth. (Chunk 2, 5)
- [ ] **R3** Deterministic, documented error codes for all invalid-proof and double-vote scenarios → `ConclaveError` contract. (Chunk 2, 4)

## Performance

- [ ] **P1** CU cost of each on-chain operation documented (note: LEZ CU budget may change) → `docs/BENCHMARKS.md` + `docs/evidence/LEZ_TESTNET_COSTS.md`. (Chunk 3, 6)

## Supportability

- [ ] **S1** Program deployed + tested on LEZ devnet/testnet. (Chunk 6)
- [ ] **S2** End-to-end integration tests run against a LEZ sequencer (standalone mode), included in CI. (Chunk 4, 9)
- [ ] **S3** CI green on default branch ⚠️ **requires fixing the GitHub billing lock**. (Chunk 9)
- [ ] **S4** README: deployment steps, program addresses, CLI + Basecamp instructions. (Chunk 8)
- [ ] **S5** Reproducible e2e demo script works against a real local sequencer with `RISC0_DEV_MODE=0`. (Chunk 6)
- [ ] **S6** Recorded narrated video demo showing terminal output incl. proof generation (`RISC0_DEV_MODE=0`). (Chunk 9)

## Submission requirements (write-up must cover)

- [ ] **W1** Public repo, MIT OR Apache-2.0. (Chunk 0)
- [ ] **W2** Verifier program deployed on testnet with verified program ID. (Chunk 6)
- [ ] **W3** Threshold proof scheme described. (Chunk 8)
- [ ] **W4** Nullifier design described. (Chunk 8)
- [ ] **W5** LEZ account model compatibility — how nonce + `program_owner` constraints are handled for shielded accounts. (Chunk 1, 8)
- [ ] **W6** Security assumptions + known limitations. (Chunk 8)
- [ ] **W7** Proof generation time + on-chain verification gas/CU benchmarks. (Chunk 3, 6)

## Bonus differentiators (Idea 02)

- [ ] **B1** Shielded member rotation — new member root; old key provably dead (nullifier + marker-PDA re-derivation under old threshold → unclaimed address). (Chunk 2, 4, 6)
- [ ] **B2** Tiered spending — per-category thresholds + amount caps, categories committed. (Chunk 2, 6)
- [ ] **B3** Single aggregated recursive threshold proof (not N proofs). (Chunk 3)
- [ ] **B4** `scripts/regenerate-evidence.sh` — evidence survives testnet resets. (Chunk 6)
- [ ] **B5** 4 reference integrations (governance consumer, treasury, inheritance/will, token-gated community). (Chunk 6–7)
- [ ] **B6** ADRs + `BUGS_FILED.md` + crates published to crates.io. (Chunk 8)
