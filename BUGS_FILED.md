# BUGS_FILED — Upstream findings

Issues discovered in reference material / dependencies while building Quorum.
"Filed" means reported to the upstream repo where a URL is given; otherwise it
is a documented local finding pending report.

## 1. `jimmy-claw/lez-multisig` — fresh zero-nonce keypair constraint

The public LP-0002 PoC requires members to be **fresh zero-nonce keypairs
claimed by the program**. Shielded LEZ accounts are owned by the privacy
protocol and increment nonce on every use, so they cannot satisfy this
constraint. This is the core design gap Quorum solves (ADR-0006).

- **Status:** documented in README + ADR-0006. Reported in the LP-0002 PR
  discussion context.

## 2. LEZ testnet RPC — no per-transaction CU / gas-used field

The current LEZ testnet RPC does not expose per-transaction CU or gas-used
metadata. This blocks criterion P1 (CU cost per operation) at the network
layer, not at the application layer. Same limitation reported by the LP-0005
solution.

- **Status:** documented in `docs/BENCHMARKS.md` + `docs/KNOWN_LIMITATIONS.md`.
  Risc0 cycles reported instead.

## 3. SPEL `generate_idl!` macro — writes to stdout, not a file

`spel_framework::generate_idl!` prints the IDL JSON to stdout; it does not
write the file itself (verified against `balance-gate` in LEZ-TokenStudio,
whose `idl/*.idl.json` was captured manually).

- **Status:** local finding. The Quorum repo captures the output into
  `programs/quorum-gate/idl/quorum_gate.idl.json` (validated JSON).

## 4. GitHub account billing lock blocks hosted CI

Same as LP-0005: hosted default-branch jobs are blocked before execution by
the account-level billing state. Strict CI (fmt + clippy `-D warnings` +
tests) passes locally and is wired in `.github/workflows/ci.yml`.

- **Status:** operator-side unlock needed; see `docs/KNOWN_LIMITATIONS.md` #7.

## 5. `spel-framework` instruction mapping — every field must be a handler param

The `#[lez_program]` macro requires every instruction field to appear as a
handler parameter (missing params fail at compile time with confusing
macro-generated errors). Documented here so future SPEL authors add all fields
to handlers.

- **Status:** resolved locally; no upstream change needed.
