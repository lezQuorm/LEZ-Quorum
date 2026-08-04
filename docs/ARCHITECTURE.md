# Architecture

Quorum separates private approval proving from public policy enforcement. The
same pure gate logic is used by the local SDK and the SPEL program, which keeps
state-transition tests independent of network integration.

~~~text
member secrets
     |
     v
quorum-cli / quorum-sdk
     |
     | ThresholdWitness
     v
quorum-threshold guest ---> Risc0 receipt + public journal
                                      |
                                      | transaction assumption (pending builder)
                                      v
                               quorum-gate SPEL
                                      |
                         +------------+-------------+
                         |                          |
                  constitution update       token chained call
~~~

## Components

### Client layer

**quorum-sdk** creates member sets, mirrors constitution and proposal state,
builds Merkle witnesses, invokes the prover, and checks returned journals.
**quorum-cli** persists the local mirror, private member files, proof artifacts,
and replacement rotation bundles.

### Proof layer

**quorum-circuit** evaluates the threshold statement as pure Rust.
**quorum-threshold** commits the resulting journal in a Risc0 guest.
**quorum-prover** creates succinct receipts and verifies the pinned image ID on
the host.

The proof establishes that every supplied approval:

- derives from a secret committed under the active member root;
- has a valid Merkle path;
- produces a distinct nullifier for the proposal and constitution version;
- contributes to the required threshold; and
- approves an action that satisfies its circuit-level policy checks.

### Gate layer

**quorum-gate-core** owns the serializable state and deterministic validation
rules. **quorum-gate** is the SPEL guest that exposes initialize, propose,
approve, and execute instructions.

A constitution is bound to its multisig account ID. Every proposal is bound to
that multisig ID and the exact constitution version at creation. Approval and
execution reject cross-multisig proposals, stale proposals, mismatched
instruction IDs, inflated tier caps, duplicate nullifiers, invalid vaults, and
recipient substitution.

There is no unauthenticated reject instruction. Proposals leave the active
state only through threshold-authorized execution.

### LEZ compatibility layer

**lez-compat** models LEZ v0.3 account commitments, Merkle semantics, nonce
progression, and owner stability. It is tested independently but is not yet
connected to the Quorum membership witness. The current circuit proves
knowledge of a Quorum member secret, not control of a live shielded LEZ
account credential.

## State

The constitution stores:

- owning multisig account ID;
- constitution version and proposal counter;
- default threshold and member count;
- member commitment root; and
- transfer tier thresholds and caps.

A proposal stores:

- owning multisig account ID;
- proposal ID and constitution version;
- action and required threshold;
- accepted nullifiers; and
- active or executed status.

A rotation increments the constitution version and replaces the member root
and count in one validated transition. Proposals created under the prior
version become stale.

## Transfer execution

For transfer proposals, execute validates the runtime recipient against the
approved recipient and derives the treasury vault PDA from the multisig ID.
The gate then emits a token-program chained call with the vault authorized by
its PDA seed. Rotation and threshold changes update constitution state without
emitting a token call.

## Receipt composition boundary

The SPEL guest calls Risc0 env::verify with the pinned threshold image ID and
journal. In nested Risc0 execution, that verifies an assumption supplied to the
outer executor; it does not deserialize the receipt bytes in the instruction.

A production transaction composer must therefore:

1. decode and verify the client threshold receipt;
2. bind its journal to the approve instruction;
3. add the receipt to the SPEL executor assumptions; and
4. submit the resulting LEZ transaction.

That composer and its live sequencer test are pending. Until they exist, local
gate tests demonstrate state rules but do not constitute end-to-end on-chain
receipt verification.
