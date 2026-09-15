# Demo testnet evidence: complete treasury transaction flow

The LEZ-Quorum Basecamp demo session completed a **2-of-3** treasury transfer on LEZ testnet: **750 QUORUM-DEMO units funded, two private approvals confirmed, 250 units transferred, 500 units left in the vault, and proposal 0 marked Executed**.

Verification on **15 September 2026** found **no transaction hash, confirmation block, session transaction-byte, account, or action mismatches** in the supplied records. All **10 unique transactions** were present, including the pre-existing gate deployment and the nine transactions for this treasury session.

[Watch the Basecamp demo](https://www.youtube.com/watch?v=m65kwds8LOc).

The 13-minute-57-second Basecamp video shows setup, treasury transactions, and the start of the first real approval proof. Both approval confirmations and execution happened **after recording ended**. This document supplies the verified completion evidence for the same session; it does not claim those later confirmations appear in the Basecamp video.

[Watch the companion CLI verification demo](https://www.youtube.com/watch?v=zBWPmJSlVj8). This walkthrough uses `quorum network --target testnet status` to check the completed session's confirmed transactions, approvals, and final balances. It is a status check of existing transactions, with no new proof generation or transaction submission.

These are the final demonstration recordings selected by the author, who confirmed that the audio is clear in both. The later CLI status output reports block **9784**, the same ten confirmed transaction hashes and inclusion blocks, and the same final balances. This later status check supplements the transaction and receipt verification recorded below; it does not change the original verification timestamp.

## Session and verification context

| Field | Value |
|---|---|
| Target | LEZ testnet |
| Sequencer RPC | `https://testnet.lez.logos.co` |
| Explorer | [LEZ testnet explorer](https://explorer.testnet.lez.logos.co) |
| Network/channel ID | `0101010101010101010101010101010101010101010101010101010101010101` |
| Basecamp session used for treasury transactions | `lez-quorum-testnet-20260914-230942769` |
| Configuration | 2 approvals required from 3 member credentials |
| Constitution version | 1 |
| Proposal | ID 0; proposal counter 1 |
| Spending tier | ID 1; threshold 2; maximum transfer 750 units |
| Token | QUORUM-DEMO; initial total supply 1,000 units |
| Funding / proposed transfer | 750 / 250 units |
| User's final status snapshot | Block 9563; journal consistent |
| Fresh account verification | 2026-09-15 at approximately 06:33:44 UTC; node height 9641 |
| Source revision inspected | `34c280eef551619b7cf63da6ff154f40a4eab1a5` |
| LEZ decoding and verification library revision | v0.2.2, `d6e4ae694e7419f5906b340c232704466a1917b7` |

The supplied transcript and local `records.md` were byte-identical at verification. Their SHA-256 was `7c74de1a10bba34b9a3586e29e5d0249c7e6304b6888db8c989f0fedd276ca58`.

The fresh account queries were separate current-state reads. Block 9641 is the node height observed during those reads; it is not a claim that an atomic historical snapshot was requested.

## Public program and account identifiers

| Program/artifact | Verified value |
|---|---|
| Quorum gate program ID | `f84e14137c10cd3c7261f98d675ae7fcbe6cf8f8448ecd2f82dd8b7234ce98ec` |
| Gate ELF SHA-256 | `72351623f9a703c40736ab5645b047d39b3c5b688f2c2c47302cf62d1762fd3b` |
| Built-in token program ID | `ccc4713e2b5ecdff37b0c67c295369effc04b7e8994eb11c3f410bb226b82e9b` |
| LEZ privacy circuit ID used to verify both receipts | `383e884f67e016e9e046294a6f8ed2dab5b516bbcf452c18f32145f2d400206f` |

The deployed gate bytecode exactly matches the repository's [gate artifact](../programs/quorum-gate/artifacts/quorum_gate.bin). Recomputing its program ID produced the gate ID above.

| Account role | Public account ID |
|---|---|
| `multisig` | `Public/CcdYiXAinekHswyzEhDHFik9gdkbx5vHLotxi9FRFMXN` |
| `definition` | `Public/5q9AbWTPZKzQ7mraoP4vaKi3T1foPX3iyr2zKMLpd1NF` |
| `supply` | `Public/7VNa1wcYTo9mMBhdbaBAQ49QETx3PWKEmTCLxzkwc9YV` |
| `recipient` | `Public/GeCrcrw84xomDq9vRev1vrYz5ntW4fz6FH8gtWYvHV7d` |
| `vault` | `Public/3GkxyKKGWF7FDAsYBxAUf1qiC21iV7zyhN4U132S62vc` |
| `proposal` | `Public/2ZKzpqzKamMKgAsXipkXtEigg3mg1yjNWhZLBeJaiqw7` |

The multisig account stores the constitution. The proposal account stores the action and approval nullifiers. The definition account identifies the token; supply, vault, and recipient hold that token.

## Confirmed transactions

Every transaction below passed hash recomputation, recorded-block inclusion, and comparison with the saved transaction bytes in the session shown in the Basecamp video. At verification, the sequencer reported `bedrock_status=Finalized` for all ten inclusion blocks.

Times below are **block timestamps in UTC**, not proof-generation duration. Nairobi time is UTC+03:00.

| Local transaction label | Transaction hash / explorer | Inclusion block | Block timestamp |
|---|---|---:|---|
| `deploy` | [4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43](https://explorer.testnet.lez.logos.co/transaction/4635b013b5d3c1b2b4f3d50af938808be839727a90bd293de2ba799b83c24b43) | [4024](https://explorer.testnet.lez.logos.co/block/4024) | 2026-09-11 08:44:08.922 UTC |
| `initialize` | [0dbdb99d31da0572cb6a50c5e27b21ddf2520c7935c092d3834bcf55d45e00ff](https://explorer.testnet.lez.logos.co/transaction/0dbdb99d31da0572cb6a50c5e27b21ddf2520c7935c092d3834bcf55d45e00ff) | [9021](https://explorer.testnet.lez.logos.co/block/9021) | 2026-09-14 20:11:42.585 UTC |
| `create-token` | [7e7b29fc9cdbaaf71c5b1e091ead6336c4435c07b053a4c55cbeef9e6b830372](https://explorer.testnet.lez.logos.co/transaction/7e7b29fc9cdbaaf71c5b1e091ead6336c4435c07b053a4c55cbeef9e6b830372) | [9022](https://explorer.testnet.lez.logos.co/block/9022) | 2026-09-14 20:12:42.766 UTC |
| `initialize-recipient` | [b464bbe3e86c2c2419e3aaee713e8d5d3ab9264ee76ab1372d15d441ab1719d5](https://explorer.testnet.lez.logos.co/transaction/b464bbe3e86c2c2419e3aaee713e8d5d3ab9264ee76ab1372d15d441ab1719d5) | [9023](https://explorer.testnet.lez.logos.co/block/9023) | 2026-09-14 20:13:42.936 UTC |
| `initialize-vault` | [d811b1f21ffc03741030c411059494d2bf652cf816d28b8fd6d5f46b57a3f604](https://explorer.testnet.lez.logos.co/transaction/d811b1f21ffc03741030c411059494d2bf652cf816d28b8fd6d5f46b57a3f604) | [9025](https://explorer.testnet.lez.logos.co/block/9025) | 2026-09-14 20:15:43.230 UTC |
| `fund` | [ef1196eaad0cfd242dbadd0ea5d2eb87b9956ff563a1f610a639dc7764bc5fd3](https://explorer.testnet.lez.logos.co/transaction/ef1196eaad0cfd242dbadd0ea5d2eb87b9956ff563a1f610a639dc7764bc5fd3) | [9026](https://explorer.testnet.lez.logos.co/block/9026) | 2026-09-14 20:16:43.483 UTC |
| `propose` | [11878ff3b4df25c073f5542e3d9e9bb3cf7fa13d07f4311deca4a833b2f37073](https://explorer.testnet.lez.logos.co/transaction/11878ff3b4df25c073f5542e3d9e9bb3cf7fa13d07f4311deca4a833b2f37073) | [9027](https://explorer.testnet.lez.logos.co/block/9027) | 2026-09-14 20:17:43.657 UTC |
| `approve-0-0` | [ac9c69ce7bf7b6bbd03e411b179021ab398398832303dd83115c4b30a9577d9a](https://explorer.testnet.lez.logos.co/transaction/ac9c69ce7bf7b6bbd03e411b179021ab398398832303dd83115c4b30a9577d9a) | [9100](https://explorer.testnet.lez.logos.co/block/9100) | 2026-09-14 21:30:53.006 UTC |
| `approve-0-1` | [288b903f22e99ea943c54965a3351c445449c0d29d86c80de738e53d599aba3b](https://explorer.testnet.lez.logos.co/transaction/288b903f22e99ea943c54965a3351c445449c0d29d86c80de738e53d599aba3b) | [9176](https://explorer.testnet.lez.logos.co/block/9176) | 2026-09-14 22:47:02.793 UTC |
| `execute` | [4547d1674ca1c16d3c31780a718dbea8844ecbc3dce10c8d2d7aeeb12fb643fe](https://explorer.testnet.lez.logos.co/transaction/4547d1674ca1c16d3c31780a718dbea8844ecbc3dce10c8d2d7aeeb12fb643fe) | [9544](https://explorer.testnet.lez.logos.co/block/9544) | 2026-09-15 04:55:49.617 UTC |

The deployment at block 4024 occurred on 11 September. The demo verifies and reuses that deployment; it does not deploy a fresh gate during recording. Initialization through proposal were confirmed on 14 September while recording. In Nairobi time, approval 1 confirmed on **15 September at 00:30:53**, approval 2 at **01:47:02**, and execution at **07:55:49**.

## Transaction contents and state continuity

| Transaction | Decoded contents and cross-check |
|---|---|
| `deploy` | ProgramDeployment transaction containing the exact gate artifact; its computed program ID matches the multisig and proposal owners. |
| `initialize` | Gate `Initialize` instruction for the listed multisig: threshold 2, member count 3, and spending tier 1 with threshold 2 and cap 750. Its member commitment root matches both approvals and the live constitution. |
| `create-token` | Built-in token `NewFungibleDefinition`: QUORUM-DEMO, supply 1,000, using the listed definition and supply accounts. |
| `initialize-recipient` | Token `InitializeAccount` using that definition and the listed recipient. |
| `initialize-vault` | Gate `InitializeVault` using the same multisig, definition, and vault. |
| `fund` | Token `Transfer` of 750 units from the listed supply account to the listed vault. |
| `propose` | Gate `Propose` using that multisig and proposal account: transfer 250 to the listed recipient, tier 1, cap 750. |
| `approve-0-0` | PrivacyPreserving transaction whose public post-state contains the same constitution and proposal 0, still Active, with one approval nullifier. |
| `approve-0-1` | PrivacyPreserving transaction preserving the first nullifier and adding exactly one distinct second nullifier. The action, recipient, amount, threshold, and constitution remain unchanged; proposal remains Active pending execution. |
| `execute` | Gate `Execute { proposal_id: 0 }` using exactly the listed multisig, proposal, vault, and recipient. Live state confirms Executed and the expected token balances. |

Both approval transactions contain one private action. Each final privacy receipt is a **Succinct** Risc0 receipt and passed cryptographic verification against the pinned LEZ privacy circuit, with development mode unset and fake receipts explicitly rejected.

For that verification, the expected public pre-state was reconstructed from the recorded approval transition: the constitution is unchanged, and the proposal has one fewer nullifier before each approval. The receipt was verified against the resulting public/private circuit output. The second approval's pre-state matches the first approval's post-state.

| Approval | Proposal nullifier added | Verified final privacy receipt size |
|---|---|---:|
| First | `5078f86530a4871302f29bfc87368062bebd218a15f90f590d59dbf14e72b246` | 230,547 bytes |
| Second | `e78c1ea4fa30562fd6d3f1cd971fcdc9097aca8edc20f80593e60ce468d98d66` | 230,803 bytes |

These are public proposal nullifiers, not member secrets. The suffixes in `approve-0-0` and `approve-0-1` are local member-slot labels. Verification establishes distinct credential approvals; it does not identify the members or establish that separate people operated them.

## Matching the Basecamp video

The following timestamps are inspected frames where the relevant output is visible, rather than exact button-click times. The visible hashes and blocks match the transaction table above.

| Video reference | Visible evidence |
|---|---|
| [04:00](https://www.youtube.com/watch?v=m65kwds8LOc&t=240s) | Threshold 2, members 3; deployment hash `4635b013…`, block 4024, and the matching gate program ID. |
| [06:15](https://www.youtube.com/watch?v=m65kwds8LOc&t=375s) | Initialization hash `0dbdb99d…`, confirmed in block 9021. |
| [07:15](https://www.youtube.com/watch?v=m65kwds8LOc&t=435s) | Token creation hash `7e7b29fc…`, confirmed in block 9022. |
| [09:00](https://www.youtube.com/watch?v=m65kwds8LOc&t=540s) | Copied initialization, token, and recipient records; recipient hash `b464bbe3…`, confirmed in block 9023. |
| [10:12](https://www.youtube.com/watch?v=m65kwds8LOc&t=612s) | Vault initialization hash `d811b1f2…`, confirmed in block 9025. |
| [11:30](https://www.youtube.com/watch?v=m65kwds8LOc&t=690s) | Funding hash `ef1196ea…`, confirmed in block 9026; proposal submission is next. |
| [12:15](https://www.youtube.com/watch?v=m65kwds8LOc&t=735s) | Proposal hash `11878ff3…`, confirmed in block 9027; proposal ID 0 selected. |
| [13:55](https://www.youtube.com/watch?v=m65kwds8LOc&t=835s) | “Real proof active”; stage 1 of 3; proving approval for member 0. Proof generation is still running at the end. |

All ten network transactions match the saved public transaction journal of session `lez-quorum-testnet-20260914-230942769`, which is visible during the treasury and approval steps. This ties the later confirmations to the session used in the Basecamp recording.

## Clarifications and discrepancies found

- **No recorded transaction hash or block mismatch was found.** Repeated label/hash pairs are repeated output for the same transaction, not additional transactions.
- **The vault preview was not a failed transaction.** The `submission=blocked` entry and later confirmation contain the same hash, `d811b1f2…`; that transaction subsequently confirmed in block 9025.
- **The Basecamp video does not contain the final confirmations.** Both approvals and execution are later completion evidence. Their network block timestamps establish this distinction; the companion CLI demo checks the completed state afterward.
- **The Basecamp video displays an incorrect `0 / 0 confirmed approvals` before the first proof.** The label reads numeric fields from the latest activity output, and missing fields become zero. The proposal-submission output has no approval counts. The recorded initialization and live constitution require 2 approvals; receipt-verified proposal states progress from 1/2 to 2/2. This is a display issue, not a zero-threshold onchain configuration. See [the approval display implementation](../apps/basecamp-quorum/src/qml/QuorumView.qml).
- **An earlier session directory is visible during deployment verification at 04:00.** The treasury transaction sequence subsequently uses the session named above. The shared deployment is reusable, and the later session journal matches all ten network transactions.
- **The [earlier deployment snapshot](https://github.com/lezQuorm/LEZ-Quorum/blob/34c280eef551619b7cf63da6ff154f40a4eab1a5/docs/DEPLOYMENT.md) describes a different historical run.** Its treasury accounts and transaction hashes are not this demo's records. It lists the shared deployment hash at block 693, whereas this verification and the Basecamp recording place it at block 4024. The current [deployment guide](DEPLOYMENT.md) links this demo as the current evidence and retains a reference to that historical snapshot.
- **2/2 approvals is a satisfied threshold in a 2-of-3 configuration.** It does not mean the treasury has only two members.

## Final state and token accounting

The user's status check at block 9563 reported a healthy RPC, a consistent journal, ten confirmed transactions, and proposal 0 Executed. Fresh account reads during this verification independently confirmed the same governance and balances.

| Final field | Verified value |
|---|---|
| Constitution status / version | Initialized / 1 |
| Members / required approvals | 3 / 2 |
| Proposal counter / ID | 1 / 0 |
| Distinct confirmed proposal approvals | 2 |
| Proposal status | Executed |
| Supply account token balance | 250 |
| Vault token balance | 500 |
| Recipient token balance | 250 |
| Total token supply | 1,000 |

```text
Initial supply:          1,000
Funding sent to vault:     750
Supply remaining:          250

Vault:       750 - 250 =    500
Recipient:     0 + 250 =    250

250 supply + 500 vault + 250 recipient = 1,000 token units
```

All three holding accounts reference the same token definition. These values are token units, not fees or a monetary valuation.

The execution output's `RESULT=PASS` is consistent with these independent checks. The [CLI final-state verifier](../crates/quorum-cli/src/network_workflow.rs) prints that result only after checking Executed, the expected token definition, and both expected balances.

## Verification method and scope

1. Compared the supplied transcript with the local record, checked every repeated transaction hash, and deduplicated the ten final journal entries.
2. Queried `getTransaction` for each hash and `getBlock` for each recorded inclusion block using the public testnet RPC.
3. Decoded the returned transactions and blocks with the pinned LEZ types, recomputed every transaction hash and block hash, and confirmed exact transaction-byte equality and a single occurrence in its recorded block.
4. Ran LEZ stateless transaction checks, including applicable signature checks. Compared every returned transaction with the public transaction journal from the Basecamp video session.
5. Decoded public instructions and private approval post-states; checked shared account IDs, program IDs, token identity, amounts, proposal ID, constitution version, thresholds, member root, and nullifier continuity.
6. Verified both final real privacy receipts against the pinned circuit and their reconstructed expected outputs. No new proofs were generated and no transactions were submitted.
7. Queried network identity, health, built-in program IDs, and the six live accounts; verified final token accounting and matched visible transaction output against frames from the published Basecamp video.

`Finalized` above is the status supplied by the sequencer for those blocks. This review checked their contents and hashes; it did not independently replay Bedrock consensus or the full LEZ chain. The evidence establishes this completed testnet treasury flow, rather than a general security audit or coverage of every governance feature.
