# Known Limitations

Quorum is an experimental implementation, not a production-ready treasury.
The following items define the remaining engineering boundary.

## Live shielded-account authorization

The current threshold witness proves knowledge of a Quorum member secret.
Although lez-compat models LEZ v0.3 account commitments and validation rules,
the circuit does not yet prove control of a live shielded LEZ account or bind a
member secret to that account credential.

Impact: membership privacy and threshold behavior are demonstrated, but the
identity primitive is still Quorum-specific.

Required work: define the credential binding, include it in the witness and
journal, and test it against current LEZ shielded account data.

## Risc0 receipt transaction composition

The SPEL gate uses env::verify, which expects the threshold receipt to be
provided as an assumption to the outer execution. The repository does not yet
contain a LEZ transaction builder that decodes the CLI proof artifact, adds
that assumption, and submits the approve instruction.

Impact: the gate compiles and its validation logic is tested, but the receipt
path is not yet executable end to end on a sequencer.

Required work: implement the composer against the current LEZ executor API and
add tampered-receipt and missing-assumption integration tests.

## Network integration

There is no RPC client, deployed gate program ID, funded treasury vault, or
captured testnet transaction sequence in this repository.

Impact: create, approve, execute, and rotate are local state-machine workflows,
not evidence of a live LEZ deployment.

Required work: add the transaction client, deploy to the current supported
network, and record reproducible account and transaction evidence.

## Basecamp module

apps/basecamp-quorum contains QML views and manifests only. It does not include
the C++/Qt process backend, CMake or Nix packaging, cancellation plumbing, or a
built LGX artifact.

Impact: the interface is a design and integration prototype, not an installable
Basecamp module.

## Key operations

The CLI stores local member secrets and rotation bundles with mode 0600.
Generation and activation are implemented, but secure out-of-band key
distribution, backup, recovery, hardware-backed storage, and deletion of
retired secrets are operator responsibilities.

## Security assurance

The circuit and gate have not received an independent security audit. Network
metadata resistance, denial of service, compromised proving hosts, key
recovery, and operational governance require deployment-specific review.

## Performance

The recorded real aggregated proof takes several minutes on the benchmark
machine. Network verification cost and transaction compute usage have not been
measured because there is no live deployment.

## Protocol compatibility

LEZ, SPEL, NSSA, and Risc0 interfaces are pinned to repository revisions and
crate versions. Compatibility must be revalidated before upgrading any of
those dependencies.
