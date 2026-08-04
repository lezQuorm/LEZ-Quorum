# Evidence

On-chain evidence lives here, pinned **after** a testnet/standalone-sequencer
deployment by the operator (needs a funded LEZ wallet — see
`docs/KNOWN_LIMITATIONS.md`). Everything is regenerable with
`scripts/regenerate-evidence.sh`.

| File | Contents | Status |
|---|---|---|
| `LIVE_TESTNET.md` | Program ID, deployment/init/claim tx hashes, resulting accounts, marker-PDA re-derivation | ⬜ post-deploy |
| `LEZ_TESTNET_COSTS.md` | CU / transaction-cost measurements per operation (when the RPC exposes them) | ⬜ post-deploy |
| `BASECAMP_EVIDENCE.md` | Basecamp `.lgx` asset hash, screenshots, install log | ⬜ post-build |

Current locally-verified evidence (no testnet required):

- Real 2-of-3 threshold proof: `RISC0_DEV_MODE=0`, ~446 s, 224,346-byte receipt,
  pinned image ID `[114484643, 2738439775, 93721807, 2809967440, 468656058,
  4246638024, 2892828720, 3001232771]` (regenerate with the
  `prove_threshold` example; refresh the pin with `scripts/update-image-id.sh`).
- End-to-end demo (dev-mode proofs): `scripts/demo.sh` — create → propose →
  aggregated approve-all → execute → rotate → old-key-dead.
- CLI integration tests: `crates/quorum-cli/tests/cli_flow.rs`.
