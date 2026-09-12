# Privacy Model

Quorum hides membership and approval ownership. It does not attempt to hide
governance activity, policy, treasury movements, or network metadata.

## Protected Information

Assuming private material is generated and stored securely, SHA-256 is not
broken, and the Risc0 and LEZ proofs are sound, an observer does not learn:

- the member nullifier secret keys;
- member viewing public keys or account identifiers;
- the private LEZ credential account IDs;
- the complete member list or any Merkle authentication path; or
- the real-world identity associated with an approval nullifier.

Only a root commitment to the member set is stored in the constitution.

## Public Information

The following is intentionally public:

- the multisig, proposal, vault, token, and recipient public account IDs;
- member count, threshold, spending tiers, member root, and version;
- proposal ID, action details, amount, recipient, and status;
- approval count and every proposal-scoped nullifier;
- transaction timing, ordering, size, sender/network metadata visible to LEZ;
- deployments, program image IDs, and resulting treasury balances.

Consequently, Quorum is not a shielded governance system. It provides private
authorization for public governance and public treasury execution.

## Linkability

An approval nullifier includes both proposal ID and constitution version. The
same member therefore emits the same nullifier when replaying an approval for
that proposal, but a different value for another proposal or version. This
supports public duplicate detection without creating a stable cross-proposal
member identifier.

Credential commitments are also scoped to member root, proposal, and version.
They bind the threshold receipt to the outer private LEZ accounts without
publishing those account IDs as stable identifiers.

Network-layer observations can still correlate activity. A member who submits
from a recognizable host, at a recognizable time, or with a unique interaction
pattern may be identified outside the cryptographic protocol. Quorum includes
no relay, anonymity network, batching service, or cover traffic.

## Membership Privacy

The public root is a deterministic commitment to the canonically sorted set.
It is computationally hiding for high-entropy private account IDs, but it does
not protect a weak or already-known credential set from offline enumeration.
Member count is public and bounded at ten.

Rotation publishes a new root, new member count, and incremented version.
Observers learn when governance changes and may infer information from count or
timing changes, even though removed and added identities remain hidden.

## Approval Privacy

The threshold receipt proves that the nullifier corresponds to an enrolled
credential. The gate sees an identity-free nullifier and a scoped credential
commitment, while LEZ verifies the corresponding private account inside the
outer proof. Neither public output names the member.

The public transaction sequence reveals how many approval transactions were
submitted. The standard network flow uses one proof per member, so timing may
provide more correlation than an aggregated proof. The SDK can aggregate
multiple members, but that requires collecting their witnesses on one client.

## Secret Handling

The CLI writes member bundles with restrictive file permissions and ignores
`.quorum-network-local/` and `.quorum-testnet/` in Git. These are operational controls,
not cryptographic protection at rest. The files are not a hardware wallet or a
general encrypted keystore; host compromise exposes credentials.

Never publish member JSON, recovery material, viewing keys, passwords, claims,
or a prepared transaction directory containing secrets. Use isolated demo
credentials for recordings and public testnet demonstrations.

## Non-Goals

Quorum does not provide:

- hidden proposal actions, values, destinations, policies, or balances;
- hidden approval count or transaction timing;
- anonymous networking or resistance to endpoint traffic analysis;
- coercion resistance, deniability, or protection after member secret loss;
- secrecy against a client that aggregates several members' witnesses; or
- post-quantum proof security. The credential ID includes an ML-KEM viewing
  key, but Quorum's hashes and proof system have their own assumptions.

See [Circuit Design](CIRCUIT_DESIGN.md),
[Security Assumptions](SECURITY_ASSUMPTIONS.md), and
[Known Limitations](KNOWN_LIMITATIONS.md).
