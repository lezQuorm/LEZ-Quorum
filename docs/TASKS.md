# Testnet Readiness

Updated 2026-08-07. Public writes remain disabled.

## Status

| Phase | State |
|---|---|
| Read-only testnet preflight | Complete |
| Gate redeployment | Prepared; owner approval required |
| Network composer and CLI | Complete |
| Basecamp testnet mode | Complete |
| Local rehearsal | Development-proof flow passed |
| Public treasury lifecycle | Not started |
| Demo recording | Pending public lifecycle |

## Completed

- [x] Pin LEZ `v0.2.2` at `d6e4ae694e7419f5906b340c232704466a1917b7`.
- [x] Reproduce gate program ID
  `f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec`.
- [x] Reproduce deployment hash
  `4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43`.
- [x] Extract typed builders for deployment, constitution, token, recipient,
  vault, funding, proposal, approval, execution, and state reads.
- [x] Add typed confirmation hashes, blocks, and decoded account state.
- [x] Add `network --target local|testnet` CLI commands.
- [x] Isolate state in `.quorum-network-local/` and `.quorum-testnet/`.
- [x] Protect directories with mode `0700` and private files with `0600`.
- [x] Keep account and member secrets separate from the public journal.
- [x] Journal the exact transaction before submission.
- [x] Query timed-out transactions before optional exact resubmission.
- [x] Require `--confirm-public-write` for submission.
- [x] Reject development proofs for the public testnet target.
- [x] Validate network ID, version, program, vault, constitution, proposal,
  recipient, token definition, and balances.
- [x] Add mock RPC coverage for confirmation, rejection, and timeout.
- [x] Add transaction serialization, mismatch, permission, overwrite, and
  exact-payload recovery tests.
- [x] Add Basecamp `Local | LEZ Testnet` modes, live status, approval progress,
  execution guard, single-use write approval, and reconciliation controls.
- [x] Run two distinct private approvals against local LEZ `v0.2.2`.
- [x] Confirm local final state: vault `500`, recipient `250`, proposal
  `Executed`, `RESULT=PASS`.
- [x] Pass the full Rust workspace test suite and strict Clippy checks.
- [x] Save sanitized preflight evidence under ignored
  `target/testnet-evidence/`.
- [x] Re-run public preflight after RPC recovery at block `845`.
- [x] Confirm the earlier deployment transaction is absent after testnet reset.
- [x] Prepare protected 2-of-3 testnet state with `0700` directories and `0600`
  files.
- [x] Journal the exact gate deployment locally without submitting it.
- [x] Build generated sources, module library, native LGX, and portable LGX in
  a clean Nix container.
- [x] Parse the final QML with Qt 6.9.2 tooling.

## Current Gate

`https://testnet.lez.logos.co` is healthy. At the 2026-08-07 preflight:

- network ID: `0101010101010101010101010101010101010101010101010101010101010101`
- latest block: `845`
- built-in programs: `5`
- published test account balance: `150`
- prior gate transaction: not found

The deployment hash remains
`4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43`.
Submission is blocked until the owner approves the first public write.

## Remaining Automated Work

- [ ] Rehearse Basecamp restart and reconciliation from each transaction stage.
- [ ] Run the final release flow with real proofs if a fresh proof artifact is
  required for judging.
- [ ] Execute the public lifecycle after owner approval.
- [ ] Add confirmed public IDs, hashes, blocks, and balances to
  `DEPLOYMENT.md`.

## Public Lifecycle

Run only after read-only preflight passes and the owner approves public writes.

1. [x] Prepare a fresh 2-of-3 testnet state directory.
2. [ ] Submit and confirm the prepared gate deployment.
3. [ ] Initialize the constitution and demo token.
4. [ ] Initialize recipient and program-derived vault accounts.
5. [ ] Fund the vault with `750` tokens.
6. [ ] Propose a tier-1 transfer of `250` tokens.
7. [ ] Generate and confirm member 0's real private approval.
8. [ ] Re-read proposal state before generating member 1's proof.
9. [ ] Generate and confirm member 1's real private approval.
10. [ ] Confirm live proposal state is `Active` with `2/2` approvals.
11. [ ] Execute once.
12. [ ] Confirm vault `500`, recipient `250`, and `Executed`.

For every write, save the exact payload and hash before submission, then save
its confirmation block. On timeout, reconcile by hash before any resubmission.

## Owner Actions

- [ ] Approve the first public testnet write after preflight passes.
- [ ] Approve publication of new account and transaction identifiers.
- [ ] Record the demo after the public final state is confirmed.
- [ ] Review the recording for secrets before publishing it.

Wallet recovery phrases, passwords, member secrets, Merkle paths, and private
account material must never enter logs, screenshots, QML, source control, or
the recording.

## Stop Conditions

Stop without resubmitting if the network ID, version, bytecode, image ID,
account, nonce, proposal, recipient, vault, tier, proof, or balance differs
from the prepared state. Also stop if a transaction has unknown status or any
private material appears in output.
