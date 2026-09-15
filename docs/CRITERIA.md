# LP-0002 Success-Criteria Evidence

This matrix maps the [LP-0002 criteria](https://github.com/logos-co/lambda-prize/blob/master/prizes/LP-0002.md)
to source, tests, CI, release artifacts, and testnet evidence. Checked rows have
the stated supporting evidence. The sections below describe the final video
coverage and the metrics available from the pinned LEZ implementation.

## Functionality

| Status | Criterion | Evidence |
|---|---|---|
| [x] | Shielded member approval without public identity | Threshold witness and credential binding in `quorum-circuit` and `quorum-gate` |
| [x] | Verify M approvals without recording member identities | Constitution root plus proposal nullifier set; `check_claim` and `threshold_met` |
| [x] | Prevent double approval | Proposal/version-scoped nullifier plus circuit and gate duplicate checks |
| [x] | Execution unlinkable to an individual shielded account | Scoped credential commitments and outer private LEZ transaction; see `PRIVACY_MODEL.md` for metadata limits |
| [x] | Client-side proof generation | `quorum-prover`, CLI approve commands, real-proof CI job |
| [x] | Threshold-gated LEZ testnet action | 2-of-3 session initialized at 9021, approved at 9100 and 9176, and executed at 9544; [verified transactions](DEMO_TESTNET_EVIDENCE.md) |
| [x] | Reproducible testnet instance and evidence | Exact commits, program/artifact hashes, commands, accounts, transactions, and final state |
| [x] | Full documentation and clean public repository | README and focused documents listed below; format, Clippy, tests, and secret ignores |

## Usability

| Status | Criterion | Evidence |
|---|---|---|
| [x] | Module/SDK | `quorum-sdk`, `quorum-composer`, `quorum-cli`, and integration guide |
| [x] | Basecamp GUI, local build, downloadable assets, loadable in Basecamp | Native/portable LGX and matching CLI are downloadable in [v0.1.1](https://github.com/lezQuorm/LEZ-Quorum/releases/tag/v0.1.1); checksums and host-load verification are described in [Basecamp Release](BASECAMP_RELEASE.md) |
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
| [x] | CU cost for each on-chain operation | Reproducible guest `user_cycles` table, including chained calls and governance, with exact Risc0 measurement semantics in [Benchmarks](BENCHMARKS.md) |

Gas/fee measurements are unavailable from the pinned LEZ RPC. The
[source-backed explanation](BENCHMARKS.md#gas-and-fee-boundary) distinguishes
measured guest execution from cryptographic verification gas and token fees.
No numeric verifier gas cost is claimed.

## Supportability

| Status | Criterion | Evidence |
|---|---|---|
| [x] | Deployed and tested on LEZ testnet | Gate deployment at block 4024 and completed demo lifecycle in [Demo Testnet Evidence](DEMO_TESTNET_EVIDENCE.md) |
| [x] | Standalone sequencer E2E in CI | `scripts/sequencer-e2e.sh` and the `sequencer-e2e` CI job |
| [x] | Green default-branch CI | [Run 34942595687](https://github.com/lezQuorm/LEZ-Quorum/actions/runs/34942595687) passed for published commit `456ece6524ca1b651ff9fd3ed31f416b7038df61` |
| [x] | README with deployment, addresses, CLI, and Basecamp use | README plus `INTEGRATION.md` and `DEPLOYMENT.md` |
| [x] | Real local-sequencer demo script | Script defaults to `RISC0_DEV_MODE=0`, checks the pinned LEZ commit and exact PASS markers; the local 0.1.1 real run ended with both PASS markers after 5620 seconds |
| Supplied; coverage below | Narrated E2E video and terminal proof output | Final [Basecamp](https://www.youtube.com/watch?v=m65kwds8LOc) and [CLI verification](https://www.youtube.com/watch?v=zBWPmJSlVj8) recordings, plus real-proof CI logs and verified completion evidence |

## Required Write-Up

| Requirement | Document |
|---|---|
| Cryptographic and threshold proof approach | [Circuit Design](CIRCUIT_DESIGN.md) |
| Nullifier and anti-replay design | [Circuit Design](CIRCUIT_DESIGN.md), [Privacy Model](PRIVACY_MODEL.md) |
| Trusted setup | [Security Assumptions](SECURITY_ASSUMPTIONS.md) |
| LEZ nonce and `program_owner` compatibility | [Integration](INTEGRATION.md#lez-account-compatibility) |
| Security assumptions | [Security Assumptions](SECURITY_ASSUMPTIONS.md) |
| Known limitations | [Known Limitations](KNOWN_LIMITATIONS.md) |
| Integration instructions | [Integration](INTEGRATION.md) |
| Proof generation time and CU cost | [Benchmarks](BENCHMARKS.md) |

## Final Demonstration Coverage

The author selected both published videos as final and confirmed that their
audio is clear. The Basecamp recording shows setup, treasury submissions, and
the start of the first real approval proof. The CLI recording shows a testnet
status check of the same completed session: ten confirmed transactions, two
approvals, Executed, vault 500, and recipient 250.

The videos are complemented by [transaction and receipt verification](DEMO_TESTNET_EVIDENCE.md)
and the [real-proof CI job](https://github.com/lezQuorm/LEZ-Quorum/actions/runs/34942595687/job/104295017075).
The [official video criterion](https://github.com/logos-co/lambda-prize/blob/master/prizes/LP-0002.md#supportability)
includes terminal proof generation. The supplied
CLI recording checks existing state; it does not record new proof generation
or transaction submission.

## Submission References

| Artifact | Reference |
|---|---|
| Verified public code/evidence commit | [456ece6524ca1b651ff9fd3ed31f416b7038df61](https://github.com/lezQuorm/LEZ-Quorum/commit/456ece6524ca1b651ff9fd3ed31f416b7038df61) |
| Release and source revision | [v0.1.1](https://github.com/lezQuorm/LEZ-Quorum/releases/tag/v0.1.1), source `09e3bed69b229ecbec30e8bc24fe87ba3dac46e2` |
| Package hashes and installation | [Basecamp Release](BASECAMP_RELEASE.md) |
| Testnet program and reproduction | [Deployment](DEPLOYMENT.md) |
| Current demo transactions and videos | [Demo Testnet Evidence](DEMO_TESTNET_EVIDENCE.md) |
| Default-branch CI | [Run 34942595687](https://github.com/lezQuorm/LEZ-Quorum/actions/runs/34942595687) |

The release source and later verified documentation commit are listed
separately so reviewers can reproduce the published packages and identify the
revision checked by CI.
