# Technical Reference

This page is a compact index of compatibility constants and persisted formats.
Design rationale lives in the focused documents linked at the end.

## Pinned Versions

| Dependency | Version or commit |
|---|---|
| Host Rust | `1.91.0` |
| Risc0 crate and CLI | `3.0.5` |
| Risc0 guest Rust | `1.97.0` |
| LEZ | `v0.2.2`, `d6e4ae694e7419f5906b340c232704466a1917b7` |
| Basecamp builder | Locked by `apps/basecamp-quorum/flake.lock` |

## Limits

| Constant | Value |
|---|---:|
| Maximum members / approvals per receipt | 10 |
| Maximum spending tiers | 8 |
| Viewing public key length | 1184 bytes |
| Constitution state schema | 1 |
| Proposal state schema | 1 |

Every constitution requires `1 <= threshold <= member_count <= 10`. Tier IDs
must be unique and each tier threshold must be within the member count.

## Cryptographic Domains

| Purpose | Prefix or formula |
|---|---|
| Quorum base | `quorum/v1` |
| Approval nullifier | `SHA256("quorum/v1/nullifier" || secret || proposal_le64 || version_le32)` |
| Member commitment | `SHA256("quorum/v1/member" || private_account_id)` |
| Credential binding | `SHA256("quorum/v1/credential" || account_id || root || proposal_le64 || version_le32)` |
| Vault seed | `SHA256("quorum/vault/v1" || multisig_account_id)` |
| LEZ commitment | `/LEE/v0.3/Commitment/` padded to 32 bytes |
| LEZ private account ID | `/LEE/v0.3/AccountId/Private/` padded to 32 bytes |

The LEE v0.3 strings identify the upstream protocol serialization used by LEZ
v0.2.2; they are not a mismatch in the application version.

## Merkle Rules

```text
commitments are lexicographically sorted
leaf = SHA256(member_commitment)
node = SHA256(left || right)
an unpaired final node is duplicated
```

These rules are implemented in `crates/quorum-core/src/merkle.rs` and tested
against membership, non-membership, odd-level, and rotation cases.

## Actions

| Action | Public fields | Execution |
|---|---|---|
| Transfer | recipient, amount, tier ID, state-derived tier cap | Chained call from vault to LEZ token program |
| Rotate members | new root and member count | Replace root/count and increment version |
| Change threshold | new threshold | Replace threshold and increment version |

The canonical deployed action type is `quorum_circuit::ActionData`, re-exported
by `quorum-gate-core`.

## Program Interface

| Instruction | Accounts and purpose |
|---|---|
| `initialize` | Claims the multisig state with root, threshold, count, and tiers |
| `initialize_vault` | Creates the deterministic token holding through a chained call |
| `propose` | Claims proposal state and binds an action to current version |
| `approve` | Verifies receipt assumption and private credential bindings |
| `execute` | Applies an approved action once and marks proposal executed |

The generated machine-readable interface is
`programs/quorum-gate/idl/quorum_gate.idl.json`.

## Local Files

The standalone state CLI writes `quorum.json`, `member-<index>.json`, optional
`rotation.json`, and `claims/` in its current working directory. The network
workflow writes:

| Target | Directory |
|---|---|
| Local sequencer | `.quorum-network-local/` |
| Public testnet | `.quorum-testnet/` |

Network state includes public configuration, protected secret material, and a
transaction journal. Records transition through `Prepared`, `Submitted`,
`Unknown`, `Confirmed`, or `Orphaned`. Secret-bearing paths are Git-ignored but
not encrypted.

## Artifacts

| Artifact | SHA-256 |
|---|---|
| `guests/quorum-threshold/artifacts/threshold.bin` | `7533ba0608cf00b1eb8b8b57d259d3594ff1d886acc33e9696ae57726ee951df` |
| `programs/quorum-gate/artifacts/quorum_gate.bin` | `72351623f9a703c40736ab5645b047d39b3c5b688f2c2c47302cf62d1762fd3b` |

The deployed testnet program ID is recorded in [Deployment](DEPLOYMENT.md).
Regenerate method metadata only with the pinned Risc0 toolchain, then update
the corresponding image-ID crates and deployment evidence together.

## Further Reading

- [Architecture](ARCHITECTURE.md)
- [Circuit design](CIRCUIT_DESIGN.md)
- [Privacy model](PRIVACY_MODEL.md)
- [Security assumptions](SECURITY_ASSUMPTIONS.md)
- [Error codes](ERROR_CODES.md)
- [Benchmarks](BENCHMARKS.md)
- [Integration](INTEGRATION.md)
- [Known limitations](KNOWN_LIMITATIONS.md)
