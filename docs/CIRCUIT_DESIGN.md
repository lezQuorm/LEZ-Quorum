# Circuit Design

## Statement

The `quorum-threshold` guest proves that every approval controls a credential
committed under the active member root, that all proposal-bound nullifiers are
distinct, that the approval count meets the required threshold, and that the
public action satisfies the circuit-level policy check.

## Private witness

```rust
ThresholdWitness {
    member_root,
    required_threshold,
    approvals: Vec<MemberApprovalWitness> {
        member_secret,       // LEZ nullifier secret key
        viewing_public_key,  // LEZ ML-KEM viewing public key
        account_identifier,  // LEZ regular-private-account identifier
        leaf_index,
        siblings,
    },
    action,
    proposal_id,
    constitution_version,
}
```

For each approval the guest follows the pinned LEZ v0.2.2 derivation:

1. derive the nullifier public key from the secret;
2. derive the regular private account ID from the nullifier public key, ML-KEM
   viewing public key, and identifier;
3. validate the viewing key encoding and bind the complete identity;
4. blind the account ID into the Quorum member commitment;
5. verify its SHA-256 Merkle path;
6. derive a proposal- and version-bound Quorum nullifier; and
7. reject duplicate credentials or nullifiers.

The gate independently re-derives tier policy and validates all account and
state bindings. Circuit checks do not replace gate checks.

## Public journal

```rust
ThresholdJournal {
    member_root,
    proposal_id,
    constitution_version,
    required_threshold,
    approval_count,
    nullifiers,
    credential_commitments,
    action,
}
```

`credential_commitments` bind the derived private account IDs to the member
root, proposal ID, and constitution version. This prevents a leaked journal
from exposing a stable cross-proposal credential pseudonym. The journal
contains no secret keys, private account IDs, member list, leaf indices, or
Merkle paths.

## Receipt composition

The host verifies the threshold receipt against the pinned image ID. The gate
then calls `env::verify` for the same image and journal, which succeeds only
when the composer supplies that receipt as an assumption. The resulting gate
receipt becomes an assumption of the LEZ privacy circuit.

Inside the privacy circuit, each private credential identity derives the same
account ID from its nullifier secret. This connects the threshold statement to
live LEZ account control without publishing the account ID in the final
transaction message.

Individual and aggregated approvals use the same guest and image ID. The
current hard limit is ten approvals per receipt.
