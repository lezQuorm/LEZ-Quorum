# Tasks

Updated 2026-08-09.

## Complete

- [x] Pin LEZ `v0.2.2` and reproduce the gate program and deployment IDs.
- [x] Implement the circuit, gate, composer, SDK, guarded CLI, and Basecamp LGX.
- [x] Protect private state and journal exact transactions before submission.
- [x] Pass local development-proof and real-proof 2-of-3 lifecycles.
- [x] Build the native and portable LGX packages with Nix and Qt 6.9.2.
- [x] Verify the retained deployment transaction at testnet block `693`.
- [x] Complete the public lifecycle with two real private approvals.
- [x] Confirm execution at block `1165`.
- [x] Confirm vault `500`, recipient `250`, proposal `Executed`, and
  `RESULT=PASS`.
- [x] Publish public IDs, transaction hashes, blocks, and balances in
  `DEPLOYMENT.md`.

## Demo Delivery

- [x] Record the demo using the public testnet state.
- [x] Review the recording for exposed secrets before publishing it.
- [x] Add the final video URL to PR #120.

All engineering, testnet-readiness, and demo-delivery work is complete. An
independent security audit is scheduled before production use.

Never show `.quorum-testnet/`, recovery phrases, passwords, member secrets,
Merkle paths, claims, or private account material.
