#!/usr/bin/env bash
# Regenerates ALL Quorum evidence after a testnet reset or code change:
#  1. unit tests + clippy (local CI)
#  2. a real 2-of-3 threshold proof (RISC0_DEV_MODE=0) with timings
#  3. the end-to-end demo flow
# Re-run and re-pin the hashes in docs/evidence/ after any change.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== 1. local CI (fmt + clippy + tests, dev mode) =="
cargo fmt --check
RISC0_DEV_MODE=1 cargo clippy --workspace --all-targets -- -D warnings
RISC0_DEV_MODE=1 cargo test --workspace

echo "== 2. real proof (RISC0_DEV_MODE=0) =="
RISC0_DEV_MODE=0 cargo run -p quorum-prover --example prove_threshold --release

echo "== 3. end-to-end demo =="
RISC0_DEV_MODE=1 ./scripts/demo.sh

echo
echo "✅ evidence regenerated — re-pin hashes in docs/evidence/LIVE_TESTNET.md"
