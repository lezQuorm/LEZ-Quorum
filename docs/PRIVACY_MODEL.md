# Privacy Model

Quorum hides member identities and approval attribution while keeping treasury
policy and execution auditable. This document describes the implemented
protocol; it does not claim network-level anonymity or a completed funded
public treasury lifecycle.

## Credential statement

A member secret is an LEZ nullifier secret key. Enrollment commits to the
regular private account ID derived from that key and a `u128` account
identifier. An approval proves both:

1. that the credential commitment belongs to the constitution's member root;
2. that the private LEZ transaction controls the matching account identity.

The threshold circuit journals only a proposal-scoped commitment to the
account ID. The binding also includes the member root and constitution version,
so a leaked proof artifact does not provide a stable cross-proposal pseudonym.
The gate sees the account ID inside its private execution, checks the binding,
and the outer LEZ privacy circuit proves authorization and emits an encrypted
post-state. The final transaction does not list credential account IDs among
its public accounts.

## Hidden by the proof path

- LEZ nullifier secret keys.
- Private credential account IDs.
- Merkle leaf positions and authentication paths.
- The enrolled credential commitment list.
- Which enrolled member produced a Quorum nullifier.
- Private credential post-state contents.

## Public by design

- Multisig and proposal account IDs.
- Constitution version, threshold, member count, member root, and tiers.
- Proposal ID, action, recipient, amount, tier, and status.
- Required threshold, approval count, and accepted Quorum nullifiers.
- New encrypted-state commitments and LEZ nullifiers required by the privacy
  protocol.
- Rotation and threshold-change actions.

A rotation is observable. It reveals the new root and count but not the member
identities or a mapping from old credentials to replacement credentials.

## Linkability

Quorum nullifiers bind the member secret to a proposal ID and constitution
version, so they change across proposals and rotations. Reusing one credential
for the same proposal produces the same value and is rejected.

Credential bindings in proof artifacts are scoped to the member root, proposal
ID, and constitution version. They therefore change with a new proposal or
rotation even when the same LEZ credential remains enrolled.

Approval timing, proof count, proposal content, and threshold progress remain
observable. Network observers may also correlate submission timing, endpoints,
and traffic. The repository has no testnet evidence establishing sender
unlinkability at the transport layer.

## Trust boundary

The privacy claim depends on SHA-256 preimage resistance, Risc0 proof
soundness, the pinned LEZ account derivation, correct receipt composition,
secure credential handling, and the privacy circuit in the pinned LEZ v0.2.2
dependency. A compromised proving host can disclose witnesses before proof
generation even if the resulting transaction is private.
