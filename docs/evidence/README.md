# Evidence

This directory is reserved for reproducible deployment and performance
artifacts. No live LEZ deployment evidence has been recorded yet.

| Artifact | Required contents | Status |
|---|---|---|
| LIVE_TESTNET.md | Program ID, dependency revisions, account IDs, transaction hashes, and final state | Pending |
| LEZ_TESTNET_COSTS.md | Per-operation compute, latency, receipt size, and transaction cost | Pending |
| BASECAMP_EVIDENCE.md | Built package hash, install log, screenshots, and workflow result | Pending |

Locally reproducible evidence currently consists of:

- the real proving example documented in ../BENCHMARKS.md;
- the dev-mode end-to-end workflow in ../../scripts/demo.sh; and
- workspace and CLI integration tests.

The demo covers creation, an aggregated approval, execution, rotation,
old-member rejection, replacement-key activation, and a transfer approved by
the new members. Local evidence must not be presented as network deployment
evidence.
