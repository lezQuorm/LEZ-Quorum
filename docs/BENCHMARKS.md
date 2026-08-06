# Benchmarks

The isolated measurement below was recorded locally on 2026-08-05 with Linux,
16 CPU cores, Risc0 3.0.5, succinct proving, and `RISC0_DEV_MODE=0`. It predates
the current LEZ v0.2.2 compatibility image and is retained only as historical
performance context, not as current proof evidence.

## Historical credential-aware aggregated 2-of-3 proof

| Metric | Recorded value |
|---|---|
| Proof generation time | 714.579 seconds (latest hardened image); 499.816 seconds and 635.656 seconds (prior runs) |
| Serialized receipt size | 224,866 bytes |
| Approvals | 2 |
| Public Quorum nullifiers | 2 |
| Credential commitments | 2 |
| Historical threshold image ID | `[2579077875, 769874733, 529682050, 4062924364, 2705577364, 2680433381, 735259384, 241280473]` |
| Host verification | Passed |

Reproduce the measurement with:

```bash
RISC0_DEV_MODE=0 \
  cargo run -p quorum-prover --example prove_threshold --release
```

The artifact was accepted by its matching compiled gate and privacy-wrapper
composition test. Those outer executions used development mode and are not
valid performance measurements for the current release.

## Current v0.2.2 evidence

The complete current lifecycle uses threshold image ID
`[1186714911, 372965427, 361634562, 623475285, 4245419629, 3728370648,
573247614, 3919023327]` and gate program ID `[320098040, 1020072060,
2381930866, 4243020391, 4177030334, 802000452, 1921768834, 3969437236]`.
It proves the threshold receipt, gate execution, and LEZ privacy wrapper before
sequencer submission. The recorded seed-121 lifecycle completed in
approximately 2 hours 19 minutes from deployment through executed-state
confirmation. Exact transaction and state evidence is recorded in
[the verification log](evidence/README.md); individual proof stages were not
instrumented as reproducible benchmarks.

Public-network verification cost, confirmation latency, compute use, and fees
have not been measured.

Results vary with hardware, dependency versions, member count, Merkle depth,
approval count, and proof options.
