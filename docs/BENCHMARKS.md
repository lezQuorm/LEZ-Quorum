# Benchmarks

Measurements were recorded locally on 2026-08-05 with Linux, 16 CPU cores,
Risc0 3.0.5, succinct proving, and `RISC0_DEV_MODE=0`.

## Credential-aware aggregated 2-of-3 proof

| Metric | Recorded value |
|---|---|
| Proof generation time | 714.579 seconds (latest hardened image); 499.816 seconds and 635.656 seconds (prior runs) |
| Serialized receipt size | 224,866 bytes |
| Approvals | 2 |
| Public Quorum nullifiers | 2 |
| Credential commitments | 2 |
| Threshold image ID | `[2579077875, 769874733, 529682050, 4062924364, 2705577364, 2680433381, 735259384, 241280473]` |
| Host verification | Passed |

Reproduce the measurement with:

```bash
RISC0_DEV_MODE=0 \
  cargo run -p quorum-prover --example prove_threshold --release
```

The proof artifact from the latest run was then accepted by the compiled gate
and LEZ privacy-wrapper composition test. The outer executions used dev mode,
so they verify the receipt bridge but are not valid performance measurements.
Real outer privacy-proof time, sequencer execution,
verification cost, confirmation latency, compute use, and fees have not been
measured.

Results vary with hardware, dependency versions, member count, Merkle depth,
approval count, and proof options.
