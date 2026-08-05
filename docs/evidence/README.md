# Evidence

This directory summarizes reproducible evidence and keeps local, public-testnet,
and Basecamp-host claims separate.

| Evidence | Status |
|---|---|
| Offline CLI lifecycle | Verified by `scripts/demo.sh` and workspace tests |
| Real threshold receipt | Generated and host-verified against the pinned image |
| Local LEZ v0.2.0 lifecycle | Verified in development-proof mode through sequencer RPC by `local_lez_e2e` |
| Native and portable Basecamp LGX builds | Verified from the committed Nix lock |
| Public LEZ testnet deployment | Pending a compatible public RPC and funded authority |
| Released Basecamp visual installation and capture | Requires a desktop Basecamp session |

The local sequencer lifecycle deploys gate program ID
`[4111006493, 1080021697, 3288851758, 2774569147, 3884519978, 2340268407,
3146869960, 2689516161]`, initializes and funds its treasury holding, records a
private 2-of-3 approval, executes a 250-unit transfer, and verifies these final
values:

```text
vault_balance=500
recipient_balance=250
proposal_status=Executed
RESULT=PASS
```

The latest repeat run used identity seed 151 and recorded these local
development-proof transaction hashes:

| Operation | Transaction hash |
|---|---|
| Initialize constitution | `deb5ab9d04819ad5b42673fc1842feb272ab4c9d2a572d4c639ca1b817d24cb0` |
| Create token | `4540456dc7c7a32bcdbb9e587471bfcf60380dc1f7f4da9a1f87e113077c04a2` |
| Initialize recipient | `4454c8e1b20bb9595ce37c94ce7981006c39c12c317065f06f0a9b9d5b44793c` |
| Initialize vault | `e6835933734506fd264040771fed23bf679a4ccd1424a8ebfa33f3f749c132e8` |
| Fund vault | `c04eea71b5ee5ad05863f6fbeb543182d8e4ed2a135799e068c0b0ed6591c7bd` |
| Propose | `c46122a33c845cfd74332db22cedccab11b6462b73fed6eba23d62e5d1f36a22` |
| Private approval | `49734beeb4ad22ec4cf5229873418489e59f9280749e551660bc0c43ba93768f` |
| Execute | `8321df55946e507964e59e0de366cd0a94779fd4b49ffc904d34637bc9dd29b3` |

The Basecamp build produces native `linux-amd64-dev` and portable
`linux-amd64` LGX variants. Both contain the QML view, backend plugin, replica
factory, metadata, and manifest. The native Nix closure resolves Qt 6.9.2 and
Qt Remote Objects without missing libraries; the view parses with the pinned Qt
tooling.

The module-builder standalone preview started headlessly with the application,
capability host, and Quorum `ui-host` processes alive and no plugin or QML load
error. The official Basecamp 0.2.1 Linux AppImage also started successfully
offscreen and loaded Logos Core, the capability module, package manager, and
downloader. Importing the LGX and recording visual interaction still require a
desktop run.

These results prove a local standalone deployment, not a public testnet
deployment. Public evidence must record its RPC/network identity, program and
account IDs, transaction hashes, costs, and final state separately.
