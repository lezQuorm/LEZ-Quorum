#!/usr/bin/env bash
# Refreshes crates/quorum-image-id/src/lib.rs with the compiled guest's image ID.
# Run after ANY change to guests/quorum-threshold/guest/.
#
# The ID is computed from the guest ELF directly (risc0_zkvm::compute_image_id),
# so this is fast and does NOT require RISC0_DEV_MODE=0 or a minutes-long proof.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== computing guest image id (no proving required) =="
OUTPUT="$(cargo run -q -p quorum-prover --example print_image_id 2>&1)"
ARRAY="$(printf '%s\n' "$OUTPUT" | sed -n 's/^image_id rust: \[\(.*\)\]$/\1/p' | head -1)"
if [[ -z "$ARRAY" ]]; then
  echo "could not extract image id from print_image_id output:" >&2
  printf '%s\n' "$OUTPUT" >&2
  exit 1
fi
echo "new image id: [$ARRAY]"

TARGET="crates/quorum-image-id/src/lib.rs"
python3 - "$TARGET" "$ARRAY" <<'PY'
import re, sys

path, array = sys.argv[1], sys.argv[2]
# Format each number with underscore thousands separators (clippy: unreadable_literal).
formatted = ", ".join(f"{int(n):,}".replace(",", "_") for n in array.split(", "))
src = open(path, encoding="utf-8").read()
new = re.sub(
    r"(pub const THRESHOLD_IMAGE_ID: \[u32; 8\] = \[)[^]]*(\])",
    rf"\1\n    {formatted},\n\2",
    src,
    count=1,
)
assert new != src, f"THRESHOLD_IMAGE_ID const not found in {path}"
open(path, "w", encoding="utf-8").write(new)
print(f"updated {path}")
PY

echo "== done — verify with: cargo test -p quorum-image-id =="
