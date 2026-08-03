# Quorum — Benchmarks (LP-0002 performance criterion)

Measured locally (Linux, 16-core), Risc0 3.0.5, succinct proofs,
`RISC0_DEV_MODE=0` (real proving — no mocks).

## Real 2-of-3 threshold proof (single approval, one Merkle path, tier transfer)

| Metric | Value |
|---|---|
| Proof generation time | **~368 s** (6.1 min) |
| Receipt size (bincode) | **224,346 bytes** (~219 KiB) |
| Guest approvals per proof | 1 (per-member mode) |
| Nullifiers committed | 1 |
| Pinned image ID | `[2504793846, 1302641585, 509407582, 452779787, 1019694882, 662766674, 1532127949, 2008668271]` |

Regenerate with:
```bash
RISC0_DEV_MODE=0 cargo run -p quorum-prover --example prove_threshold --release
```

## Notes

- **CU cost per operation**: the current LEZ testnet RPC does not expose a
  per-transaction CU/gas-used field (same limitation the LP-0005 solution
  reported). Proof cycles map directly to LEZ verification cost; the Risc0
  receipt is verified on-chain via `env::verify` against the pinned image ID.
  When the network exposes CU metadata, this table will be extended
  (`docs/evidence/LEZ_TESTNET_COSTS.md`).
- Proof time is dominated by SHA-256 Merkle-path evaluation in the guest;
  the aggregated single-proof mode (M approvals in one proof) amortizes the
  fixed proving overhead across M approvals.
- LEZ's per-transaction compute budget may change during testnet; the design
  keeps the guest small (one path + one hash per approval).
