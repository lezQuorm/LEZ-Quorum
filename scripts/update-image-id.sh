#!/usr/bin/env bash
# Refreshes crates/quorum-image-id/src/lib.rs with the compiled guest's image ID.
# Run after ANY change to guests/quorum-threshold/guest/.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "Building guest and printing image id…"
RISC0_DEV_MODE=1 cargo run -q -p quorum-prover --example prove_threshold 2>/dev/null \
  | grep '^image_id rust:' | head -1 || {
    echo "could not extract image id (is RISC0_DEV_MODE unset ok? run the example manually)" >&2
    exit 1
  }
