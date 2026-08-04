# Privacy Model

Quorum aims to hide membership identities and approval attribution while
keeping treasury policy and execution auditable. This document describes the
current prototype, including what it does not yet prove.

## Hidden by the threshold proof

- Member secrets.
- Merkle leaf positions and authentication paths.
- The plaintext member commitment list.
- Which committed member produced each nullifier.

The constitution stores only a Merkle root over Quorum member commitments. A
proof shows that distinct secrets belong to that root and derives one public
nullifier per approval.

## Public by design

- Multisig account ID.
- Constitution version, threshold, member count, member root, and spending
  tiers.
- Proposal ID, action, recipient, amount, tier, and status.
- Required threshold, approval count, and accepted nullifiers.
- Rotation and threshold-change actions.

A rotation is observable: the action, member root, member count, and
constitution version change publicly. Observers cannot derive the member
identities or map old members to new members from those values alone, but the
implementation does not claim that a rotation is undetectable.

## Nullifier behavior

A nullifier is derived from the member secret, proposal ID, and constitution
version. The same member produces a different value across proposals or
constitution versions. Reusing the same approval for one proposal produces the
same nullifier and is rejected.

Nullifiers prevent duplicate approval; they do not hide proposal timing or the
number of approvals submitted.

## Transaction privacy

Approval attribution also depends on the privacy properties of the LEZ
transaction carrying the claim. The repository does not yet include the live
transaction composer or testnet evidence, so sender unlinkability has not been
demonstrated end to end.

## Credential boundary

The current membership secret is a Quorum-specific random value. It is not yet
bound to control of a live shielded LEZ account. lez-compat provides the
account commitment model needed for that work, but it is currently isolated
from the threshold circuit.

## Summary

~~~text
private member secret
        |
        +--> member commitment --> Merkle root (public)
        |
        +--> proposal-bound nullifier (public)
                                      |
                                      +--> threshold state (public)
~~~

The privacy claim relies on SHA-256 preimage resistance, Risc0 proof soundness,
correct transaction composition, secure member-secret handling, and the
privacy guarantees of the eventual LEZ submission path.
