# Reference

## Verification

| Check | Result |
|---|---|
| Quorum workspace | 87 passed, 0 failed, 2 ignored real-proof tests |
| SPEL workspace | 262 passed |
| SPEL fixtures | 40 passed |
| Local LEZ lifecycle, development receipts | Passed |
| Local LEZ lifecycle, real nested proofs | Passed |
| Public gate deployment | Confirmed in block 693 |
| Public wallet funding | Confirmed in blocks 690 and 691 |
| Native and portable Basecamp LGX builds | Passed from the pinned Nix lock |

The real v0.2.2 lifecycle used seed `121`, Risc0 `3.0.5`, and this threshold
image ID:

```text
[1186714911, 372965427, 361634562, 623475285,
 4245419629, 3728370648, 573247614, 3919023327]
```

It completed in about 2 hours 19 minutes and ended with vault balance `500`,
recipient balance `250`, and proposal status `Executed`.

| Operation | Transaction |
|---|---|
| Gate deployment | `4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43` |
| Constitution initialization | `87ffd230370d142225b419dc5ca497966c6b660c7fe7a679844a8490e9a280ae` |
| Token initialization | `a23a3c551ffe60af4636f83d1504265f58428f82659d8bad7902e6ed9a22ba00` |
| Recipient initialization | `7380e0539f49b9fa29603639402627a1e258bc71f75df5227758e10ee7d8cf9e` |
| Vault initialization | `0fe085dbfb804b98e4a45e98fa3ceeaa1ec0e910f1c60acf8e03074cbce1a88c` |
| Vault funding | `15c45608c144c7f1087206375c665eb2b6d6617e6f7cec6acc35818e3eb52ba1` |
| Proposal | `51adcd059dbcbe76e98326bf573c4b6b8fd2c19d88a72f337c2933a4d969f086` |
| Private approval | `179d3764ea60a2bff7edd4a470beb642f27ee0e88b3dac4ad7509579eff9032e` |
| Execution | `4c9b55ec930b819b09775482ba47d5b708d909a3b37f40519f36378af46b18d4` |

## Error contract

| Range | Owner | Source |
|---|---|---|
| `1001-1013` | Domain policy | `quorum-core/src/error.rs` |
| `2001-2005` | LEZ account compatibility | `lez-compat/src/lib.rs` and gate mapping |
| `3001-3008` | Threshold circuit | `quorum-circuit/src/lib.rs` |
| `4001-4017` | SPEL gate | `quorum-gate-core/src/lib.rs` and `quorum_gate.idl.json` |

Gate errors cover malformed constitutions, missing tiers, duplicate
nullifiers, inactive proposals, journal or threshold mismatch, invalid
rotations, stale state, tier-cap inflation, invalid vaults or recipients,
proposal binding, and credential mismatch.

## Limits

| Area | Limit |
|---|---|
| Members or approvals per proof | 10 |
| Spending tiers | 8 |
| Proof mode | Real proving is CPU and memory intensive |
| Wallet | Existing private accounts require encrypted-output scanning |
| Basecamp | Live private state requires a supported wallet binding |
| Public evidence | Gate deployment and wallet funding only; no public treasury lifecycle |
| Security | No independent audit or comprehensive fuzzing |
| Privacy | Proposal contents, approval count, rotations, timing, and network metadata remain public |

The CLI protects member and rotation files with mode `0600`. The Basecamp
backend uses a mode `0700` working directory, invokes no shell, rejects unknown
commands, and forces real proving.

## Compatibility

The project pins LEZ v0.2.2 commit
`d6e4ae694e7419f5906b340c232704466a1917b7`, SPEL compatibility commit
`1fef85203c3130676a49aaed1b4387d16be9fe94`, and Risc0 3.0.5.

The SPEL pin is temporary until upstream supports LEZ v0.2.2. Any dependency,
circuit, or instruction change requires regenerated image IDs and IDL, followed
by the full proof-composition and sequencer test suite.

