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
cargo build -q -p quorum-cli --manifest-path "$OLDPWD/Cargo.toml"
Q="$OLDPWD/target/debug/quorum"

RECIPIENT="0909090909090909090909090909090909090909090909090909090909090909"

echo "== 1. create 2-of-3 multisig =="
"$Q" create --threshold 2 --members 3 --tiers '[{"id":1,"threshold":2,"max_amount":1000}]'

echo "== 2. propose treasury transfer (tier 1, 500) =="
"$Q" propose --action transfer --recipient "$RECIPIENT" --amount 500 --tier 1

echo "== 3. members 0 and 1 approve in ONE aggregated proof (B3 mode) =="
"$Q" approve-all --proposal 0 --members 0,1

echo "== 4. execute (threshold reached) =="
"$Q" execute --proposal 0

echo "== 5. rotate the member set (Idea 02: new root, nothing else) =="
NEW_ROOT="$("$Q" new-root --members 3)"
echo "   new member root: $NEW_ROOT"
"$Q" propose --action rotate --new-member-root "$NEW_ROOT" --new-member-count 3
"$Q" approve --member 0 --proposal 1
"$Q" approve --member 2 --proposal 1
"$Q" execute --proposal 1

echo "== 6. final state =="
"$Q" info

echo "== 7. a rotated-out member's key is provably dead =="
"$Q" propose --action transfer --recipient "$RECIPIENT" --amount 100 --tier 1
if APPROVE_OUT="$("$Q" approve --member 1 --proposal 2 2>&1)"; then
  echo "ERROR: a rotated-out member could still approve — security property broken!"
  exit 1
fi
echo "PASS: member 1 (old set) rejected after rotation"
echo "   $APPROVE_OUT"

echo
echo "demo artifacts: $(ls claims/ | tr '\n' ' ')"
echo "NOTE: RISC0_DEV_MODE=$RISC0_DEV_MODE — set 0 for real proofs."
# (A full member-set rotation is shown here; the single-member-swap case with a
#  removed key provably dead is covered by the rotation_flow_updates_constitution
#  SDK test. Distributing new member secret files after a rotation is an
#  operator step — see docs/DEPLOYMENT.md.)
