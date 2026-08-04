# Deployment and Integration

This is an integration runbook, not a turnkey production deployment guide.
Quorum does not yet include the transaction composer or network client required
to submit threshold receipts to LEZ.

## Local verification

~~~bash
cargo build --release -p quorum-cli
cargo build --release -p quorum-gate-methods

cargo fmt --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace
RISC0_DEV_MODE=1 ./scripts/demo.sh
~~~

For a real local threshold receipt:

~~~bash
RISC0_DEV_MODE=0 \
  cargo run -p quorum-prover --example prove_threshold --release
~~~

## Required transaction composer

Before deployment, implement a client that performs all of the following:

1. Reads the multisig and proposal accounts from LEZ.
2. Builds a threshold witness from the active constitution.
3. Produces and verifies the threshold receipt locally.
4. Converts the proof journal to the approve instruction type.
5. Adds the threshold receipt as an assumption to the outer SPEL execution.
6. Builds, signs, submits, and confirms the LEZ transaction.
7. Re-reads proposal state and checks the accepted nullifiers.

The serialized receipt currently present in CLI proof artifacts is not consumed
by env::verify inside the gate guest. Nested receipt verification succeeds only
when the outer executor receives the receipt as an assumption.

## Program deployment

After the composer exists:

1. Build the quorum-gate method and record its image ID.
2. Deploy the method using the supported LEZ program deployment flow.
3. Initialize a multisig constitution with a nonzero account ID, threshold,
   member count, member root, and validated tiers.
4. Create the treasury token holding at the gate-derived vault PDA.
5. Fund the vault with a test-only amount.
6. Submit propose, approve, and execute transactions through the composer.

The vault seed is:

~~~text
SHA256("quorum/vault/v1" || multisig_account_id)
~~~

Transfer execution validates both that PDA and the approved recipient before
emitting the token chained call.

## Rotation operations

The local CLI creates a private replacement bundle and prints its public root:

~~~bash
NEW_ROOT="$(quorum new-root --members 3)"
quorum propose \
  --action rotate \
  --new-member-root "$NEW_ROOT" \
  --new-member-count 3
~~~

Approve and execute the rotation under the old constitution. Only after the
new root is active should operators run:

~~~bash
quorum activate-rotation
~~~

Activation re-derives the bundle commitments and root, checks the root and
member count against the active constitution, then installs the replacement
local key files. The bundle contains secrets and must be distributed and
stored through an operator-approved secure channel.

## Minimum integration tests

A deployment is not complete until automated tests cover:

- missing, wrong-image, malformed, and tampered receipt assumptions;
- cross-multisig proposal substitution;
- stale proposal and stale constitution claims;
- duplicate nullifiers;
- recipient and vault substitution;
- transfer above the tier cap;
- old-member rejection after rotation;
- replacement-member approval after rotation; and
- interrupted transaction and retry behavior.

Record the deployed program ID, dependency revisions, account IDs, transaction
hashes, and observed costs under docs/evidence.
