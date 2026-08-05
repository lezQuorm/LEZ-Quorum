# Architecture

Quorum separates private credential proving, program policy enforcement, and
network submission. The local SDK and SPEL program share deterministic gate
logic, while the composer is responsible for the two levels of Risc0 receipt
composition required by LEZ.

```text
LEZ nullifier secret + Merkle path
                 |
                 v
        quorum-threshold guest
                 |
        receipt + public journal
                 |
                 v  receipt assumption
          quorum-gate SPEL guest
                 |
        unconditional gate receipt
                 |
                 v  receipt assumption
          LEZ privacy circuit
                 |
       private LEZ transaction
                 |
                 v
        sequencer RPC (optional)
```

## Client and proof layers

`quorum-sdk` creates member sets, builds credential-aware Merkle witnesses,
invokes the prover, and mirrors state for offline workflows. `quorum-cli`
persists that mirror, protected member files, receipt artifacts, and rotation
bundles.

`quorum-circuit` evaluates the threshold statement as pure Rust.
`quorum-threshold` commits the journal in a Risc0 guest. `quorum-prover`
creates succinct receipts and verifies the pinned image ID before returning an
artifact.

Each approval proves:

- knowledge of an LEZ nullifier secret key;
- derivation of the corresponding regular private account ID;
- membership of the credential commitment under the active member root;
- a distinct proposal- and constitution-bound nullifier; and
- satisfaction of the requested threshold and circuit-level action rules.

## Gate layer

`quorum-gate-core` owns serializable state and deterministic validation rules.
`quorum-gate` exposes initialize, propose, approve, and execute instructions.

Approve takes the multisig, proposal, and a variable-length list of authorized
private credential accounts. The gate checks their proposal-scoped account-ID
commitments against the threshold journal, verifies the threshold receipt
assumption, updates proposal nullifiers, and returns every account in LEZ's
required positional order.

The gate rejects cross-multisig proposals, stale versions, mismatched proposal
IDs, inflated tier caps, duplicate nullifiers, credential substitution,
invalid vaults, and recipient substitution.

## Composer and network layer

`quorum-composer` performs the receipt composition boundary:

1. verify and decode the threshold artifact against the pinned image;
2. verify proposal and credential-account bindings;
3. prove the gate with the threshold receipt as an assumption;
4. pass the gate receipt to the LEZ privacy circuit;
5. prove private account authorization and post-state encryption; and
6. build a signed `PrivacyPreservingTransaction`.

Its optional `network` feature uses the pinned LEZ v0.2 sequencer RPC to submit
once, confirm by transaction hash, and read public account state. Private
credential reconciliation is a wallet scan of encrypted outputs and new
commitments; private account IDs are not public state lookup keys.

## State and execution

The constitution stores the multisig ID, version, proposal counter, threshold,
member count, credential root, and spending tiers. A proposal stores its
multisig ID, version, action, required threshold, accepted nullifiers, and
status.

Rotation atomically replaces the root and member count and increments the
version, making old proposals and credentials stale. Transfer execution
derives the treasury vault PDA from the multisig ID, validates the approved
recipient, and emits a token-program chained call authorized by the vault PDA
seed.

## Verification boundary

Local tests execute the compiled threshold guest, gate guest, and LEZ privacy
circuit together, including missing and malformed assumptions. A real
non-development threshold receipt is also generated and host-verified. A
standalone sequencer lifecycle and live testnet deployment are not yet recorded.
