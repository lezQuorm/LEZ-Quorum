#!/usr/bin/env bash
# Runs the local 2-of-3 treasury lifecycle. Development receipts are the
# default; set RISC0_DEV_MODE=0 for real proofs.
set -euo pipefail
cd "$(dirname "$0")/.."

export RISC0_DEV_MODE="${RISC0_DEV_MODE:-1}"

RUN_DIR="${QUORUM_RUN_DIR:-}"
if [[ -z "$RUN_DIR" ]]; then
    RUN_DIR="$(mktemp -d /tmp/quorum-run.XXXXXX)"
else
    mkdir -p "$RUN_DIR"
fi
cd "$RUN_DIR"

echo "== building CLI =="
cargo build -q -p quorum-cli --manifest-path "$OLDPWD/Cargo.toml"
Q="$OLDPWD/target/debug/quorum"

RECIPIENT="0909090909090909090909090909090909090909090909090909090909090909"

echo "== 1. create 2-of-3 multisig =="
"$Q" create --threshold 2 --members 3 --tiers '[{"id":1,"threshold":2,"max_amount":1000}]'

echo "== 2. propose treasury transfer (tier 1, 500) =="
"$Q" propose --action transfer --recipient "$RECIPIENT" --amount 500 --tier 1

echo "== 3. members 0 and 1 approve in one aggregated proof =="
"$Q" approve-all --proposal 0 --members 0,1

echo "== 4. execute (threshold reached) =="
"$Q" execute --proposal 0

echo "== 5. rotate the private member set =="
NEW_ROOT="$("$Q" new-root --members 3)"
echo "   new member root: $NEW_ROOT"
"$Q" propose --action rotate --new-member-root "$NEW_ROOT" --new-member-count 3
"$Q" approve --member 0 --proposal 1
"$Q" approve --member 2 --proposal 1
"$Q" execute --proposal 1

echo "== 6. final state =="
"$Q" info

echo "== 7. reject a rotated-out member =="
"$Q" propose --action transfer --recipient "$RECIPIENT" --amount 100 --tier 1
if APPROVE_OUT="$("$Q" approve --member 1 --proposal 2 2>&1)"; then
  echo "ERROR: a rotated-out member could still approve"
  exit 1
fi
echo "PASS: member 1 (old set) rejected after rotation"
echo "   $APPROVE_OUT"

echo "== 8. activate and use the replacement member set =="
"$Q" activate-rotation
"$Q" propose --action transfer --recipient "$RECIPIENT" --amount 100 --tier 1
"$Q" approve-all --proposal 3 --members 0,1
"$Q" execute --proposal 3

echo
echo "run directory: $RUN_DIR"
echo "claim artifacts:"
find claims -maxdepth 1 -type f -printf '  %f\n' | sort
echo "RISC0_DEV_MODE=$RISC0_DEV_MODE"
