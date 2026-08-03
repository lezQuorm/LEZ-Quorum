# LEZ-Quorum

**Private M-of-N multisig for the Logos Execution Zone (LEZ) — λPrize LP-0002.**

Quorum is a privacy-first treasury primitive: M-of-N shielded members approve
proposals, **nobody learns who voted or who is in the set**, and membership can
**evolve privately** (rotation) with **tiered spending** policies.

> Status: **All chunks complete** (see [`PLAN.md`](PLAN.md) and
> [`criteria-checklist.md`](criteria-checklist.md)). Chunks 0–8 ✅; Chunk 9
> (testnet deployment, video, solution PR) needs operator-side wallet/video.

## Why not the existing PoC?

The public [`lez-multisig`](https://github.com/jimmy-claw/lez-multisig) PoC
requires members to be **fresh zero-nonce keypairs claimed by the program** —
a constraint shielded (private) LEZ accounts cannot satisfy, because they are
owned by the privacy protocol and increment nonce on every use. Quorum is
built for shielded accounts from the ground up.

## Privacy model

| Leak surface (public PoC) | Quorum |
|---|---|
| Member list on-chain | Only a **Merkle root** over member commitments |
| Every vote attributed | Only a **nullifier set** + one aggregated ZK threshold proof |
| Rotation publishes the org chart | New root, nothing else — *plausible continuity* |
| One fixed rule | **Constitution**: tiers, caps, and threshold changes gated by the same private flow |

## Repository layout

```
LEZ-Quorum/
├── crates/
│   ├── quorum-core/      — domain model: Constitution, tiers, Proposal, nullifiers, error contract, Merkle member tree
│   ├── lez-compat/       — LEZ v0.3 commitment/Merkle compatibility + shielded-account rules
│   ├── quorum-circuit/   — Risc0 threshold proof (pure logic)
│   ├── quorum-prover/    — host prover (Risc0), example: real 2-of-3 proof
│   ├── quorum-image-id/  — pinned circuit image ID
│   ├── quorum-gate-core/ — on-chain gate logic (initialize/propose/approve/execute/reject)
│   ├── quorum-sdk/       — Rust SDK for Logos modules
│   └── quorum-cli/       — CLI: create / propose / approve / execute / reject / info / new-root
├── guests/quorum-threshold/ — Risc0 guest
├── programs/quorum-gate/    — SPEL verifier program + IDL (idl/quorum_gate.idl.json)
├── apps/basecamp-quorum/    — Logos Basecamp GUI module
├── docs/                    — architecture, circuit design, privacy model, benchmarks, ADRs, write-up
├── scripts/                 — demo + evidence regeneration
└── .github/workflows/       — CI (fmt, clippy -D warnings, tests, real-proof job)
```

## Quick start

```bash
cargo build -p quorum-cli
RISC0_DEV_MODE=1 ./scripts/demo.sh      # fast local demo (dev-mode proofs)
RISC0_DEV_MODE=0 cargo run -p quorum-prover --example prove_threshold --release   # real proof (~6 min)
```

## Build & test

```bash
cargo fmt --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace
```

## Build & test

```bash
cargo test --workspace      # unit tests (RISC0_DEV_MODE unset — core is pure Rust)
cargo clippy --workspace    # lints
```

## License

Licensed under either of Apache-2.0 or MIT, at your option.

## Documents

- [`PLAN.md`](PLAN.md) — chunked execution plan (LP-0002 win conditions)
- [`criteria-checklist.md`](criteria-checklist.md) — every LP-0002 criterion mapped
- [`docs/SOLUTION.md`](docs/SOLUTION.md) — the LP-0002 submission write-up
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — component overview
- [`docs/CIRCUIT_DESIGN.md`](docs/CIRCUIT_DESIGN.md) — threshold proof scheme
- [`docs/PRIVACY_MODEL.md`](docs/PRIVACY_MODEL.md) — nullifier + leak-surface analysis
- [`docs/ERROR_CODES.md`](docs/ERROR_CODES.md) — deterministic error contract 1001–1013
- [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) — real proof timings/sizes
- [`docs/SECURITY_ASSUMPTIONS.md`](docs/SECURITY_ASSUMPTIONS.md) — trust model
- [`docs/KNOWN_LIMITATIONS.md`](docs/KNOWN_LIMITATIONS.md) — honest disclosure
- [`docs/adr/`](docs/adr/) — architecture decision records (ADR-0001..0006)
- [`BUGS_FILED.md`](BUGS_FILED.md) — upstream findings
- [`references/lez-multisig/`](../references/lez-multisig/) — the public PoC we replace
