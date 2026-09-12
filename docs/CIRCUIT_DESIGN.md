# Circuit Design

This document specifies the statement proved by `quorum-threshold` and how the
SPEL gate consumes it. The implementation is in `crates/quorum-circuit`,
`crates/quorum-prover`, and `programs/quorum-gate`.

## Cryptographic Building Blocks

- SHA-256 for domain-separated member commitments, nullifiers, credential
  commitments, Merkle leaves, and Merkle internal nodes.
- LEZ v0.2.2 private account derivation for the credential identity.
- Risc0 zkVM 3.0.5 for the threshold receipt and recursive proof composition.
- A compile-time threshold guest image ID embedded in the gate guest.

Quorum requests `ProverOpts::succinct()`. In Risc0 3.0 this produces a
succinct STARK receipt. Quorum does not request the Groth16 receipt wrapper, so
the current proof path has no application-specific trusted setup ceremony.

## Domain-Separated Values

All integer fields below use little-endian encoding.

```text
account_id = LEZ_PRIVATE_ACCOUNT_ID(
  nullifier_secret_key,
  viewing_public_key,
  account_identifier
)

member_commitment = SHA256(
  "quorum/v1/member" || account_id
)

approval_nullifier = SHA256(
  "quorum/v1/nullifier" || nullifier_secret_key
  || proposal_id_le64 || constitution_version_le32
)

credential_commitment = SHA256(
  "quorum/v1/credential" || account_id || member_root
  || proposal_id_le64 || constitution_version_le32
)

merkle_leaf = SHA256(member_commitment)
merkle_node = SHA256(left_child || right_child)
```

Member commitments are sorted before tree construction. If a level has an odd
number of nodes, the final node is paired with itself. These rules make the
root deterministic across clients.

## Private Witness

Each `MemberApprovalWitness` contains:

| Field | Meaning |
|---|---|
| `member_secret` | LEZ nullifier secret key |
| `viewing_public_key` | 1184-byte ML-KEM-768 viewing public key |
| `account_identifier` | Identifier used in the private LEZ account ID |
| `leaf_index` | Member position in the canonical tree |
| `siblings` | Merkle authentication path |

The surrounding `ThresholdWitness` also carries the member root, required
threshold, action, proposal ID, and constitution version. Those surrounding
values are public outputs, not hidden claims.

## Public Journal

The receipt commits to:

| Field | Gate use |
|---|---|
| `member_root` | Must equal current constitution root |
| `proposal_id` | Must equal instruction and proposal state |
| `constitution_version` | Rejects old membership and policy |
| `required_threshold` | Validates proof-internal approval count |
| `approval_count` | Must match both output vectors |
| `nullifiers` | Added to proposal after duplicate checks |
| `credential_commitments` | Bound to outer private credential accounts |
| `action` | Must exactly equal the stored proposal action |

The journal never includes a member secret, viewing key, account identifier,
private account ID, or Merkle path.

## Proved Checks

For at most ten approvals, the threshold guest:

1. rejects zero thresholds and insufficient approval counts;
2. derives every credential account ID and member commitment;
3. recomputes every Merkle path against `member_root`;
4. derives proposal- and version-scoped nullifiers;
5. rejects duplicate nullifiers within the receipt;
6. derives proposal-scoped credential commitments;
7. enforces transfer amount at or below the supplied tier cap;
8. rejects a no-op member rotation and a zero threshold change; and
9. commits the exact public journal.

The gate deliberately repeats state-dependent checks that a caller could
otherwise influence. It obtains the transfer cap and proposal threshold from
constitution state, checks the proposal/action/root/version bindings, rejects
nullifiers already recorded in earlier transactions, and validates governance
changes against the live member count.

## Binding To LEZ Private Credentials

A membership receipt alone would only prove knowledge of an enrolled secret.
Quorum additionally requires each receipt credential to appear as an
authorized private input of the outer LEZ transaction:

1. the threshold journal commits to each derived private account ID using the
   proposal-scoped `credential_commitment` formula;
2. the composer supplies those private accounts as credential signers;
3. the gate recomputes commitments from the runtime account IDs and compares
   the two sets; and
4. the outer LEZ privacy circuit proves authorization and its account state
   transition rules, including nonce and owner behavior.

Account ordering is ignored by sorting both commitment lists. Duplicate outer
credential accounts are rejected.

## Threshold Modes

The circuit can prove M distinct approvals at once. The current network
workflow normally proves one member with `required_threshold = 1`; the gate
persists that nullifier and enforces the proposal threshold over the accumulated
set. This makes approvals independently submit-able and restart-safe. The SDK's
`approve_all` path exercises an aggregated M-member receipt.

## Receipt Verification

The prover first evaluates the statement on the host, generates the receipt,
verifies it against `THRESHOLD_IMAGE_ID`, decodes the journal, and requires it
to equal the host result. During transaction proving, the receipt is supplied
as an assumption. The gate runs:

```text
env::verify(THRESHOLD_IMAGE_ID, risc0_serde(journal))
```

Changing either the image ID or one journal byte prevents the assumption from
being satisfied. Development receipts produced with `RISC0_DEV_MODE=1` do not
provide cryptographic security and are rejected by the testnet CLI policy.

## Source Map

| Subject | Source |
|---|---|
| Hash derivations | `crates/quorum-core/src/nullifier.rs` |
| Merkle construction | `crates/quorum-core/src/merkle.rs` |
| Witness and statement | `crates/quorum-circuit/src/lib.rs` |
| Proving policy | `crates/quorum-prover/src/lib.rs` |
| Gate validation | `crates/quorum-gate-core/src/lib.rs` |
| Receipt assumption | `programs/quorum-gate/guest/src/bin/quorum_gate.rs` |

See [Security Assumptions](SECURITY_ASSUMPTIONS.md) for the trust model and
[Error Codes](ERROR_CODES.md) for deterministic rejection cases.
