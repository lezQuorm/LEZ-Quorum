# LEZ-Quorum

**Private M-of-N multisig for the Logos Execution Zone (LEZ) — λPrize LP-0002.**

Quorum is a privacy-first treasury primitive: M-of-N shielded members approve
proposals, **nobody learns who voted or who is in the set**, and membership can
**evolve privately** (rotation) with **tiered spending** policies.

> Status: **Chunk 0 — Foundation** (see [`PLAN.md`](PLAN.md)). Core domain model
> implemented and tested; circuits, program, SDK, and evidence land in later
> chunks.

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
Quorum/
├── crates/
│   └── quorum-core/      — domain model: Constitution, tiers, Proposal, nullifiers, error contract
│   └── (lez-compat)        — LEZ v0.3 commitment/Merkle compatibility    [Chunk 1]
│   └── (quorum-circuit)  — Risc0 threshold proof                       [Chunk 3]
│   └── (quorum-sdk, -cli, -image-id, -verifier)                        [Chunks 3–5]
├── programs/
│   └── (quorum-gate)     — SPEL verifier program + IDL                 [Chunk 4]
├── docs/                   — architecture, circuit design, privacy model, benchmarks
├── scripts/                — demo + evidence regeneration
├── examples/               — reference integrations
└── .github/workflows/      — CI (fmt, clippy, tests, standalone-sequencer e2e)
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
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — component overview
- [`references/lez-multisig/`](../references/lez-multisig/) — the public PoC we replace
