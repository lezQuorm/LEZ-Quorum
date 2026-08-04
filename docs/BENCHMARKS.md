# Benchmarks

Measurements were recorded locally on 2026-08-04 with Linux, 16 CPU cores,
Risc0 3.0.5, succinct proving, and RISC0_DEV_MODE=0.

## Aggregated 2-of-3 threshold proof

| Metric | Recorded value |
|---|---|
| Proof generation time | 417.435 seconds |
| Serialized receipt size | 224,346 bytes |
| Approvals in the proof | 2 |
| Public nullifiers | 2 |
| Threshold image ID | [3200284588, 1852504360, 2332593133, 3866069938, 4186485082, 2581798040, 3100454683, 3649897487] |

Regenerate the proof locally:

~~~bash
RISC0_DEV_MODE=0 \
  cargo run -p quorum-prover --example prove_threshold --release
~~~

These numbers describe proof generation only. They are not LEZ transaction
costs. The project has no live deployment yet, so sequencer execution time,
verification cost, compute usage, and transaction fees have not been measured.

Proof time is dominated by SHA-256 Merkle path evaluation and Risc0 proving.
Results will vary with hardware, Risc0 version, proof options, member count, and
approval count.
