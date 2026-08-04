# Quorum — Deployment & On-Chain Submission

Everything here is **operator-executed** and currently pending a funded LEZ
testnet wallet (see `docs/KNOWN_LIMITATIONS.md` #2). The offline half — proof
generation, proposal state, claim artifacts — is fully implemented, tested, and
verified locally.

## 1. Build

```bash
cargo build --release -p quorum-cli
cargo build --release -p quorum-gate-methods   # SPEL program (guest)
```

## 2. Deploy the gate

Deploy the compiled `quorum_gate` guest as an LEZ program on devnet/testnet and
record the resulting **program ID**. The gate's image ID must match the
receipts' pinned ID (`quorum_image_id::THRESHOLD_IMAGE_ID`, refreshed by
`scripts/update-image-id.sh`).

## 3. Initialize a multisig

```bash
quorum create --threshold 2 --members 3 --tiers '[{"id":1,"threshold":2,"max_amount":1000}]'
```

Submit the on-chain `Initialize` instruction with
`(threshold, member_count, member_root, tiers)` from the resulting
`quorum.json`. The program writes a `ConstitutionState` v1 into the multisig
account (claimed by the program).

## 4. Create & fund the treasury vault

The treasury vault is a **program-derived account** of the gate:

```
seed     = SHA256("quorum/vault/v1" || multisig_account_id)
vault_id = AccountId::for_public_pda(gate_program_id, seed)
```

1. Create a token **holding** account at `vault_id` (token program
   `InitializeAccount`, definition = the LEZ token the treasury uses).
2. Fund it (mint or deposit) with the treasury balance.

The gate never moves balances itself — on `Execute` it authorizes a
`ChainedCall` into the vault's token program (`program_owner` of the holding).

## 5. Propose & approve

```bash
quorum propose --action transfer --recipient <hex> --amount 500 --tier 1
# per-member (M correlated claims) OR aggregated single-proof (B3):
quorum approve --member 0 --proposal 0
quorum approve-all --proposal 0 --members 0,1
```

Each approval writes a claim to `claims/` (`claim-<proposal>-<member>.json` or
`claim-<proposal>-aggregated.json`). Submit each claim on-chain via the
`Approve` instruction; the guest verifies the receipt against the pinned image
ID, runs `check_claim` (stale constitution 4010 / journal mismatch 4005 /
tier-cap mismatch 4011 / duplicate nullifier 4003), and appends the nullifiers.

## 6. Execute

```bash
quorum execute --proposal 0
```

Submit the `Execute` instruction with the accounts:

| Account | Note |
|---|---|
| `multisig` | program-owned constitution (owner = gate) |
| `proposal` | program-owned proposal state (owner = gate) |
| `vault` | treasury holding PDA — required for `Transfer` actions |
| `recipient` | target account of the transfer |

- **Transfer:** the gate validates `vault` is the treasury PDA (error 4012),
  marks the proposal executed, and emits a `ChainedCall` to the vault's token
  program transferring `amount` (serde-mirrored
  `token_core::Instruction::Transfer`) with the vault PDA seed authorized.
- **Rotate / threshold change:** `vault` and `recipient` are ignored (pass any
  account, e.g. the multisig itself); no call is emitted. The SPEL macro
  requires the full account list on every `Execute`, so these two must always
  be supplied even for governance actions.

## 7. Governance

Rotation and threshold changes go through the same propose → approve →
execute flow. After a rotation the old member set's keys are provably dead
(version-bound nullifiers + new root) — demonstrated by `scripts/demo.sh` and
`crates/quorum-cli/tests/cli_flow.rs`.

## 8. Evidence

After any testnet deployment, re-pin the hashes and artifacts in
`docs/evidence/` (see its README) via `scripts/regenerate-evidence.sh`.
