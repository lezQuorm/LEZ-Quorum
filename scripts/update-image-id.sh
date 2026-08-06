#!/usr/bin/env bash
# Refreshes the pinned threshold guest image ID from the compiled ELF.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== computing threshold image id =="
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
# Rust formatting for the pinned u32 array.
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
cargo fmt -q -p quorum-image-id

echo "== verify with: cargo test -p quorum-image-id =="
