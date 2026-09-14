# LP-0002 Success-Criteria Evidence

This matrix mirrors the official LP-0002 criteria. A checked item has source,
test, CI, or testnet evidence in the repository. Unchecked items require an
external publication step and must be completed before the final submission.

## Functionality

| Status | Criterion | Evidence |
|---|---|---|
| [x] | Shielded member approval without public identity | Threshold witness and credential binding in `quorum-circuit` and `quorum-gate` |
| [x] | Verify M approvals without recording member identities | Constitution root plus proposal nullifier set; `check_claim` and `threshold_met` |
| [x] | Prevent double approval | Proposal/version-scoped nullifier plus circuit and gate duplicate checks |
| [x] | Execution unlinkable to an individual shielded account | Scoped credential commitments and outer private LEZ transaction; see `PRIVACY_MODEL.md` for metadata limits |
| [x] | Client-side proof generation | `quorum-prover`, CLI approve commands, real-proof CI job |
| [x] | Threshold-gated LEZ testnet action | 2-of-3 transfer at blocks 2359-2547 in `DEPLOYMENT.md` |
| [x] | Reproducible testnet instance and evidence | Exact commits, program/artifact hashes, commands, accounts, transactions, and final state |
| [x] | Full documentation and clean public repository | README and focused documents listed below; format, Clippy, tests, and secret ignores |

## Usability

| Status | Criterion | Evidence |
|---|---|---|
| [x] | Module/SDK | `quorum-sdk`, `quorum-composer`, `quorum-cli`, and integration guide |
| [ ] | Basecamp GUI, local build, downloadable assets, loadable in Basecamp | UI, locked native/portable builds, checksums, and an official-host module-load smoke test pass locally; the assets still need a public release and clean-download test |
| [x] | SPEL IDL | `programs/quorum-gate/idl/quorum_gate.idl.json` plus consistency test |

## Reliability

| Status | Criterion | Evidence |
|---|---|---|
| [x] | Clear proof-generation failures | Typed `ProverError`, SDK/composer context, CLI nonzero exits, Basecamp activity/error views |
| [x] | Partial approvals persist across restart | Nullifiers are proposal state; transaction journal and resume tests cover interruption |
| [x] | Deterministic invalid-proof and double-vote errors | Circuit 3001-3008 and gate 4001-4017 in `ERROR_CODES.md` and IDL |

## Performance

| Status | Criterion | Evidence |
|---|---|---|
| [x] | CU cost for each on-chain operation | Reproducible `user_cycles` table in `BENCHMARKS.md`, including transfer and governance paths; the gas/fee boundary is stated explicitly there |

## Supportability

| Status | Criterion | Evidence |
|---|---|---|
| [x] | Deployed and tested on LEZ testnet | Gate deployment and full lifecycle transaction links in `DEPLOYMENT.md` |
| [x] | Standalone sequencer E2E in CI | `scripts/sequencer-e2e.sh` and the `sequencer-e2e` CI job |
| [x] | Green default-branch CI | Linked run in `DEPLOYMENT.md` includes checks, sequencer E2E, and real proof |
| [x] | README with deployment, addresses, CLI, and Basecamp use | README plus `INTEGRATION.md` and `DEPLOYMENT.md` |
| [x] | Real local-sequencer demo script | Script defaults to `RISC0_DEV_MODE=0`, checks the pinned LEZ commit and exact PASS markers; the local 0.1.1 real run ended with both PASS markers after 5620 seconds |
| [ ] | Narrated E2E video showing terminal proof generation and `RISC0_DEV_MODE=0` | Record using the local recording checklist, verify audio, publish, and insert the final URL |

## Required Write-Up

| Requirement | Document |
|---|---|
| Cryptographic and threshold proof approach | `CIRCUIT_DESIGN.md` |
| Nullifier and anti-replay design | `CIRCUIT_DESIGN.md`, `PRIVACY_MODEL.md` |
| Trusted setup | `SECURITY_ASSUMPTIONS.md` |
| LEZ nonce and `program_owner` compatibility | `INTEGRATION.md` |
| Security assumptions | `SECURITY_ASSUMPTIONS.md` |
| Known limitations | `KNOWN_LIMITATIONS.md` |
| Integration instructions | `INTEGRATION.md` |
| Proof generation time and CU cost | `BENCHMARKS.md` |

## Final Publication Gate

Before opening the final solution PR:

1. build and smoke-test both Basecamp packages from the final commit;
2. attach native and portable `.lgx` files plus `SHA256SUMS` to a public release;
3. record one narrated demo that includes the Basecamp flow and terminal CLI
   transactions, clearly showing `proof_mode=real` or `RISC0_DEV_MODE=0`;
4. listen to the published recording on ordinary speakers and headphones;
5. replace all release, commit, CI, and video placeholders in the solution;
6. run the complete verification commands from `DEPLOYMENT.md`; and
7. confirm the solution matrix has no unchecked boxes.

Do not represent a development receipt, old video, local-only package, or
unpublished asset as satisfying the corresponding criterion.
