# Known Limitations

This release is a prize implementation and testnet prototype, not a
production treasury.

## Security And Privacy

- The project has not received an independent security audit.
- Local member files are permission-restricted but not encrypted or protected
  by hardware-backed keys.
- Threshold and gate proofs are computationally expensive and depend on the
  pinned Risc0 and LEZ stacks.
- Proposals, actions, thresholds, member count, roots, approval counts,
  nullifiers, timing, and execution results are public.
- The project provides no relay, cover traffic, or network anonymity.
- A compromised member credential remains usable until a successful rotation;
  there is no unilateral revocation path.
- Losing enough member credentials to fall below threshold can permanently
  prevent execution or governance recovery.

## Protocol Scope

- At most ten members and eight spending tiers are supported.
- The deployed proposal lifecycle has only `Active` and `Executed`; there is no
  cancellation, rejection, expiry, or garbage collection.
- Multiple proposals are supported by the state model and local CLI, but the
  packaged network demonstration is intentionally centered on proposal `0`.
- Transfer, member rotation, and threshold change are the only action types.
- Policies cap each transfer; there is no time-window allowance, rate limit,
  daily aggregate cap, or destination allowlist.
- Member count, threshold, and policies are public and can leak organizational
  structure.
- Version changes make older open proposals stale. They are not migrated.

## Credential Integration

- The network demonstration generates isolated private credential accounts via
  `PrivateAuthorizedInit`; it does not provide full discovery and import of an
  arbitrary existing Basecamp wallet account.
- The composer contains LEZ commitment scanning primitives, but the current UI
  does not expose a complete existing-wallet onboarding workflow.
- Viewing keys are deterministically derived for the generated demo bundle.
  Integration with an external wallet's key lifecycle requires additional
  review and UX work.
- The standard network path submits one member per approval transaction. SDK
  aggregation is available, but centralizing several witnesses on one client
  changes the operational trust model.

## Operations

- The Basecamp module launches `target/release/quorum` as a child process. The
  repository CLI must exist at the expected relative path.
- Basecamp packages are locally buildable, but release assets must be uploaded
  separately to a GitHub release; the current repository release has no
  downloadable package attached.
- The public testnet deployment is fixed to LEZ v0.2.2 and may stop working if
  the network resets, upgrades, or removes historical state.
- Proof generation is CPU- and memory-intensive. The recorded real 2-of-3
  threshold proof took about 11 minutes on a hosted CI runner; a complete
  real-proof sequencer lifecycle can take substantially longer.
- Transaction reconciliation detects unknown or mismatched submissions but
  cannot force a sequencer to include a transaction.
- There is no hosted coordinator, notification service, multisig indexer, or
  browser wallet flow.

## Compatibility

- Host tooling is pinned to Rust 1.91.0, Risc0 3.0.5, and LEZ v0.2.2. Other
  versions are unsupported until tested.
- The Basecamp module follows the pinned `logos-module-builder` contract where
  `metadata.json` is the manifest. Validators that require `module.json` are
  checking a different or older convention.
- The repository mirrors selected LEZ account and token serialization shapes
  to remain standalone. Compatibility tests reduce, but do not eliminate, the
  risk of upstream format drift.
- The dependency tree currently contains unresolved upstream RustSec
  advisories in pinned Logos networking and Risc0 tooling.

## Evidence Boundaries

- Testnet hashes prove that the listed public accounts changed as recorded;
  they do not by themselves prove member anonymity or source reproducibility.
- Development-mode test receipts do not provide cryptographic security.
- Compute-unit measurements are local guest `user_cycles`, not a testnet fee,
  gas price, or end-to-end latency guarantee.
- The older repository video predates the final implementation and was rejected
  for audio and coverage. It must not be used as the final submission demo.

The remaining external release tasks are tracked in
[Success-Criteria Evidence](CRITERIA.md). Technical integration instructions
are in [Integration](INTEGRATION.md).
