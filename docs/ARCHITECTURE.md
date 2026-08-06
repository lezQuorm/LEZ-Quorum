# Architecture

Quorum separates threshold authorization, gate execution, and LEZ transaction
privacy. The threshold proof establishes M-of-N approval. The SPEL gate applies
treasury policy. The LEZ privacy circuit authorizes and updates the private
credential accounts.

## Components

| Component | Role |
|---|---|
| `quorum-core` | Member commitments, Merkle proofs, nullifiers, proposals, and policy |
| `lez-compat` | LEZ v0.2.2 private account IDs, commitments, and account rules |
| `quorum-circuit` | Pure threshold statement shared by host and guest |
| `quorum-prover` | Receipt generation, encoding, and image verification |
| `quorum-gate-core` | Serializable gate state and deterministic validation |
| `quorum-gate` | SPEL program and generated IDL |
| `quorum-composer` | Threshold, gate, and LEZ privacy receipt composition |
| `quorum-sdk` / `quorum-cli` | Local member, proposal, rotation, and proof workflows |
| `basecamp-quorum` | QML interface and process-isolated CLI backend |

```text
private credential + Merkle path
              |
              v
      threshold Risc0 guest
              |
       receipt + journal
              |
              v
          SPEL gate
              |
          gate receipt
              |
              v
      LEZ privacy circuit
              |
              v
   privacy-preserving transaction
```

## State

The constitution stores the multisig ID, version, proposal counter, threshold,
member count, member root, and spending tiers. It never stores the member list.

A proposal stores its constitution version, action, required threshold,
accepted nullifiers, and status. Rotation replaces the member root and count,
then increments the constitution version. Old proposals and credentials become
stale.

The treasury is a program-derived token account:

```text
vault_seed = SHA256("quorum/vault/v1" || multisig_account_id)
```

Execution verifies the vault and approved recipient before emitting the token
transfer chained call.

## Threshold proof

Each approval contains these private inputs:

- LEZ nullifier secret key;
- ML-KEM viewing public key;
- private account identifier;
- Merkle leaf position and siblings.

The guest derives the LEZ private account ID, verifies its member commitment,
and derives a proposal-bound nullifier:

```text
member = SHA256("quorum/v1/member" || private_account_id)
nullifier = SHA256("quorum/v1/nullifier" || secret || proposal_id || version)
```

The public journal contains the member root, proposal ID, constitution version,
required threshold, approval count, nullifiers, proposal-scoped credential
commitments, and action. It contains no secret keys, account IDs, member list,
or Merkle paths.

The circuit accepts at most ten approvals. Individual and aggregated approvals
use the same guest and image ID.

## Receipt composition

`quorum-composer` performs one ordered operation:

1. Verify the threshold receipt against the pinned image.
2. Check the proposal and credential bindings.
3. Prove the gate with the threshold receipt as an assumption.
4. Prove the LEZ privacy circuit with the gate receipt as an assumption.
5. Build the signed `PrivacyPreservingTransaction`.

The network client submits once, confirms by transaction hash, and reads public
state. Private state reconciliation remains a wallet operation over encrypted
outputs and commitment proofs.

## Privacy boundary

| Private | Public |
|---|---|
| Member secrets and private account IDs | Multisig and proposal IDs |
| Member list and Merkle paths | Threshold, member count, root, and tiers |
| Approval attribution | Proposal action, approval count, and nullifiers |
| Private credential post-state | Rotations, threshold changes, and transfer result |

Nullifiers and credential bindings change with the proposal or constitution
version. Proposal contents, approval progress, transaction timing, and network
metadata remain observable.

## Security boundary

The design depends on SHA-256, Risc0 receipt soundness, the pinned image IDs,
LEZ private-account derivation, the LEZ privacy circuit, and secure credential
storage. `RISC0_DEV_MODE=1` creates non-cryptographic test receipts. Quorum has
not received an independent security audit and is not production-ready.
