#!/usr/bin/env bash
# Quorum end-to-end demo: 2-of-3 multisig approves a treasury transfer,
# then rotates a member (Idea 02 differentiator) and executes.
#
# Uses RISC0_DEV_MODE=1 (fast mock proofs) for a quick local demo.
# For REAL proofs, run with RISC0_DEV_MODE=0 (each proof takes minutes).
set -euo pipefail
cd "$(dirname "$0")/.."

export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"

DEMO_DIR="${DEMO_DIR:-/tmp/quorum-demo}"
rm -rf "$DEMO_DIR"
mkdir -p "$DEMO_DIR"
cd "$DEMO_DIR"

echo "== building CLI =="
cargo build -q -p quorum-cli --manifest-path "$OLDPWD/Cargo.toml" 2>/dev/null || true
Q="$OLDPWD/target/debug/quorum"

RECIPIENT="0909090909090909090909090909090909090909090909090909090909090909"

echo "== 1. create 2-of-3 multisig =="
"$Q" create --threshold 2 --members 3 --tiers '[{"id":1,"threshold":2,"max_amount":1000}]'

echo "== 2. propose treasury transfer (tier 1, 500) =="
"$Q" propose --action transfer --recipient "$RECIPIENT" --amount 500 --tier 1

echo "== 3. member 0 approves (client-side proof) =="
"$Q" approve --member 0 --proposal 0

echo "== 4. member 1 approves -> threshold reached =="
"$Q" approve --member 1 --proposal 0

echo "== 5. execute =="
"$Q" execute --proposal 0

echo "== 6. rotate member 2 -> newcomer (Idea 02) =="
NEW_ROOT="$("$Q" new-root --members 3)"
echo "   new member root: $NEW_ROOT"
"$Q" propose --action rotate --new-member-root "$NEW_ROOT" --new-member-count 3
"$Q" approve --member 0 --proposal 1
"$Q" approve --member 2 --proposal 1
"$Q" execute --proposal 1

echo "== 7. final state =="
"$Q" info

echo
echo "demo artifacts: $(ls claims/ | tr '\n' ' ')"
echo "NOTE: RISC0_DEV_MODE=$RISC0_DEV_MODE — set 0 for real proofs."
