# Verification Evidence

This page separates reproducible local results, public-testnet state, and
desktop-host validation. Current protocol results target LEZ v0.2.2 commit
`d6e4ae694e7419f5906b340c232704466a1917b7` and SPEL compatibility commit
`1fef85203c3130676a49aaed1b4387d16be9fe94`.

| Evidence | Status |
|---|---|
| Quorum workspace matrix | 87 passed, 0 failed, 2 intentionally ignored |
| SPEL compatibility workspace | 262 tests passed, including 5 end-to-end tests |
| SPEL fixture suite | 40 tests passed |
| Local development-proof LEZ lifecycle | Passed through the official v0.2.2 standalone sequencer |
| Local real nested-proof LEZ lifecycle | Passed through threshold, gate, privacy wrapper, submission, and final state reads |
| Public LEZ testnet gate deployment | Confirmed by sequencer RPC in block 693 on 2026-08-06 |
| Public wallet initialization and funding | Confirmed in blocks 690 and 691; balance read as 150 |
| Native and portable Basecamp LGX builds | Verified from the committed Nix lock |

## Public testnet

```text
RPC=https://testnet.lez.logos.co
explorer=https://explorer.testnet.lez.logos.co
program_id=[320098040, 1020072060, 2381930866, 4243020391,
            4177030334, 802000452, 1921768834, 3969437236]
program_id_hex=f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec
deployment_tx=4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43
confirmed_block=693
confirmed_date=2026-08-06
```

The sequencer returned the deployment transaction and block again after the
initial confirmation, establishing that the program remained live at the time
of this update. The testnet reset once during earlier verification, so this
state is explicitly ephemeral. The explorer indexer did not yet return block
693 or the deployment transaction, despite the canonical sequencer response.

The official v0.2.2 wallet health check passed. Its public test account was
initialized and funded through the Piñata proof-of-work claim:

```text
account=Public/81yCTY7Sk9h1yjzj5Du4urxxAF5ysLnmnBvtDYaEsUxh
initialize_tx=dc995ae3311064981468036810c24f5a315d26cd4718f4cd49e8ff8cc812aae2
initialize_block=690
pinata_tx=f276765e4e74f5b0d85901172a1af97c8f2d751962b95db3a3cf7028732e5c41
pinata_block=691
balance=150
nonce=1
```

This is evidence for public program deployment and wallet funding. It does not
claim that the full Quorum treasury lifecycle has been broadcast to the public
network.

## Local development-proof lifecycle

The v0.2.2 run used seed `111`:

```text
multisig=Public/5d34481jD1v67ZZkQGEh87BNURvBpGuWHvUHFh1fyiyS
vault=Public/2CiDN7ksVMBjzYVNU5geEwF9r7GaNQM2uhch6BNzrBdo
recipient=Public/6Jvk2EhMBdUsjx6qS4qSox19t4nzqk3gCycG8c6FRsVG
```

| Operation | Transaction hash |
|---|---|
| Deploy gate | `4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43` |
| Initialize constitution | `b826ca87fcfb9f482542cd57a2ec260c8010fac1f86974f2ea9f905152a9f7af` |
| Initialize token | `f9f1d739b60b019cb81cbc8f9db0ff5f5bf56385ca93f1a6688f62802495a906` |
| Initialize recipient | `2a8fd96170af112e4a2a6372f0e5d9c6b2264457da08905054ee2a7c706cd9aa` |
| Initialize vault | `42fce1f458fa645b34c4f209e30aadadc2d6214fa73d003998a11a9a916d034a` |
| Fund vault | `b109242d578757a68bd6f4cb97dd71cfb4d9fc0db574b1761669eb3cd85438ba` |
| Propose | `93749824a0b1464d1b6810a415666621bd6b289c57ba18dd07c4e3af5bbc1152` |
| Private approval | `fe8d64d45f183d50995c2f195e9494283ad4ee4a79e9902187cdea9dba40cdfa` |
| Execute | `9d7491c312a812c2c3e2ceba0c3b1882ab739ae4aaee067c4e1bc2c0ba982bb6` |

Development mode validates the transaction lifecycle but does not produce
cryptographic receipts.

## Local real-proof lifecycle

The v0.2.2 run used seed `121`, `RISC0_DEV_MODE` unset, Risc0 3.0.5, and the
pinned threshold image. It ran for approximately 2 hours 19 minutes from gate
deployment to executed-state confirmation on a 15 GiB workstation with 21 GiB
swap available.

```text
threshold_image_id=[1186714911, 372965427, 361634562, 623475285,
                    4245419629, 3728370648, 573247614, 3919023327]
multisig=Public/HDX6VJqDDfMdbDXXuaTX1FyACwuGVM4rZ1pJxHShidsT
vault=Public/54NEtjP8AGZFrzBY3EHGfozET4sgHm6D5UNJTRVgnGt3
recipient=Public/5SunmkAneaySEBrBhfGCNBvLixzdxBaj7rqXnmF3eZ9g
```

| Operation | Transaction hash |
|---|---|
| Deploy gate | `4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43` |
| Initialize constitution | `87ffd230370d142225b419dc5ca497966c6b660c7fe7a679844a8490e9a280ae` |
| Initialize token | `a23a3c551ffe60af4636f83d1504265f58428f82659d8bad7902e6ed9a22ba00` |
| Initialize recipient | `7380e0539f49b9fa29603639402627a1e258bc71f75df5227758e10ee7d8cf9e` |
| Initialize vault | `0fe085dbfb804b98e4a45e98fa3ceeaa1ec0e910f1c60acf8e03074cbce1a88c` |
| Fund vault | `15c45608c144c7f1087206375c665eb2b6d6617e6f7cec6acc35818e3eb52ba1` |
| Propose | `51adcd059dbcbe76e98326bf573c4b6b8fd2c19d88a72f337c2933a4d969f086` |
| Private approval, block 539 | `179d3764ea60a2bff7edd4a470beb642f27ee0e88b3dac4ad7509579eff9032e` |
| Execute, block 540 | `4c9b55ec930b819b09775482ba47d5b708d909a3b37f40519f36378af46b18d4` |

The run ended with:

```text
vault_balance=500
recipient_balance=250
proposal_status=Executed
RESULT=PASS
```

Every nested proof boundary used real Risc0 receipts. This was not an isolated
threshold benchmark or a development-mode outer wrapper.

## Basecamp

The Basecamp build produces native `linux-amd64-dev` and portable
`linux-amd64` LGX variants. Both contain the QML view, backend plugin, replica
factory, metadata, and manifest. The native closure resolves Qt 6.9.2 and Qt
Remote Objects; the portable archive bundles non-Qt external libraries.

The module-builder standalone preview started headlessly with the application,
capability host, and Quorum `ui-host` processes alive and no plugin or QML load
error. The official Basecamp 0.2.1 AppImage also started offscreen and loaded
the core modules. Interactive validation requires a desktop Basecamp host.
