#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEZ_REPO="${LEZ_REPO:-$PROJECT_ROOT/../../logos-execution-zone-v022}"
EXPECTED_LEZ_COMMIT="d6e4ae694e7419f5906b340c232704466a1917b7"
RPC_URL="${QUORUM_RPC_URL:-http://127.0.0.1:3040}"
IDENTITY_SEED="${QUORUM_IDENTITY_SEED:-91}"
PROOF_MODE="${RISC0_DEV_MODE:-0}"
LOG_DIR="${QUORUM_E2E_LOG_DIR:-$PROJECT_ROOT/target/e2e-logs}"
SEQUENCER_LOG="$LOG_DIR/sequencer.log"
LIFECYCLE_LOG="$LOG_DIR/lifecycle.log"
LEZ_TARGET_DIR="${LEZ_TARGET_DIR:-}"
SEQUENCER_PID=""
SEQUENCER_HOME="${QUORUM_SEQUENCER_HOME:-}"
OWNS_SEQUENCER_HOME=false
RUN_STARTED_SECONDS=$SECONDS

fail() {
    echo "ERROR: $*" >&2
    if [[ -f "$SEQUENCER_LOG" ]]; then
        echo "== sequencer log tail ==" >&2
        tail -n 80 "$SEQUENCER_LOG" >&2
    fi
    exit 1
}

cleanup() {
    if [[ -n "$SEQUENCER_PID" ]] && kill -0 "$SEQUENCER_PID" 2>/dev/null; then
        kill -- "-$SEQUENCER_PID" 2>/dev/null || kill "$SEQUENCER_PID" 2>/dev/null || true
        wait "$SEQUENCER_PID" 2>/dev/null || true
    fi
    if [[ "$OWNS_SEQUENCER_HOME" == true && -n "$SEQUENCER_HOME" ]]; then
        rm -rf -- "$SEQUENCER_HOME"
    fi
}
trap cleanup EXIT INT TERM

requested_lez_repo="$LEZ_REPO"
if ! LEZ_REPO="$(cd "$requested_lez_repo" 2>/dev/null && pwd)"; then
    fail "LEZ_REPO is not a directory: $requested_lez_repo"
fi
if [[ -z "$LEZ_TARGET_DIR" ]]; then
    LEZ_TARGET_DIR="$LEZ_REPO/target"
elif [[ "$LEZ_TARGET_DIR" != /* ]]; then
    LEZ_TARGET_DIR="$PROJECT_ROOT/$LEZ_TARGET_DIR"
fi


[[ -f "$PROJECT_ROOT/Cargo.toml" ]] || fail "invalid Quorum project root: $PROJECT_ROOT"
[[ -f "$LEZ_REPO/Cargo.toml" ]] || fail "LEZ_REPO is not a LEZ checkout: $LEZ_REPO"
[[ -f "$LEZ_REPO/lez/sequencer/service/configs/debug/sequencer_config.json" ]] \
    || fail "LEZ standalone sequencer config is missing"
command -v cargo >/dev/null || fail "cargo is required"
command -v git >/dev/null || fail "git is required"
command -v setsid >/dev/null || fail "setsid is required"
command -v mktemp >/dev/null || fail "mktemp is required"

actual_lez_commit="$(git -C "$LEZ_REPO" rev-parse HEAD 2>/dev/null)" \
    || fail "LEZ_REPO is not a Git checkout"
[[ "$actual_lez_commit" == "$EXPECTED_LEZ_COMMIT" ]] \
    || fail "LEZ_REPO must be pinned to $EXPECTED_LEZ_COMMIT; found $actual_lez_commit"
[[ "$PROOF_MODE" == 0 || "$PROOF_MODE" == 1 ]] \
    || fail "RISC0_DEV_MODE must be 0 or 1"

if [[ -z "$SEQUENCER_HOME" ]]; then
    SEQUENCER_HOME="$(mktemp -d "${TMPDIR:-/tmp}/lez-quorum-sequencer.XXXXXX")"
    OWNS_SEQUENCER_HOME=true
elif [[ ! -d "$SEQUENCER_HOME" ]]; then
    fail "QUORUM_SEQUENCER_HOME must name an existing empty directory"
elif [[ -n "$(find "$SEQUENCER_HOME" -mindepth 1 -print -quit)" ]]; then
    fail "QUORUM_SEQUENCER_HOME must be empty"
fi

mkdir -p "$LOG_DIR"
: > "$SEQUENCER_LOG"
: > "$LIFECYCLE_LOG"

echo "== build Quorum network lifecycle =="
RISC0_DEV_MODE="$PROOF_MODE" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
    cargo build --release --manifest-path "$PROJECT_ROOT/Cargo.toml" -p quorum-cli
RISC0_DEV_MODE="$PROOF_MODE" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
    cargo build --release --manifest-path "$PROJECT_ROOT/Cargo.toml" \
    -p quorum-composer --features network --example local_lez_e2e
QUORUM_BIN="$PROJECT_ROOT/target/release/quorum"
E2E_BIN="$PROJECT_ROOT/target/release/examples/local_lez_e2e"
[[ -x "$QUORUM_BIN" ]] || fail "Quorum CLI was not built"
[[ -x "$E2E_BIN" ]] || fail "local_lez_e2e was not built"
if RISC0_DEV_MODE="$PROOF_MODE" "$QUORUM_BIN" network \
    --target local --rpc "$RPC_URL" health >/dev/null 2>&1; then
    fail "RPC is already serving at $RPC_URL; stop it so this test controls the sequencer"
fi

echo "== build LEZ v0.2.2 standalone sequencer =="
(
    cd "$LEZ_REPO/lez/sequencer/service"
    RISC0_DEV_MODE="$PROOF_MODE" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" \
        CARGO_TARGET_DIR="$LEZ_TARGET_DIR" \
        cargo build --features standalone --release -p sequencer_service
)
SEQUENCER_BIN="$LEZ_TARGET_DIR/release/sequencer_service"
[[ -x "$SEQUENCER_BIN" ]] || fail "standalone sequencer was not built"

echo "== start LEZ v0.2.2 standalone sequencer =="
(
    cd "$LEZ_REPO/lez/sequencer/service"
    exec setsid env \
        RISC0_DEV_MODE="$PROOF_MODE" \
        RUST_LOG="${RUST_LOG:-info}" \
        "$SEQUENCER_BIN" configs/debug/sequencer_config.json \
        --listen-address 127.0.0.1 \
        --home "$SEQUENCER_HOME"
) >"$SEQUENCER_LOG" 2>&1 &
SEQUENCER_PID=$!

echo "== wait for sequencer health =="
healthy=false
for _ in $(seq 1 180); do
    if ! kill -0 "$SEQUENCER_PID" 2>/dev/null; then
        fail "standalone sequencer exited before becoming healthy"
    fi
    if RISC0_DEV_MODE="$PROOF_MODE" "$QUORUM_BIN" network \
        --target local --rpc "$RPC_URL" health >/dev/null 2>&1; then
        healthy=true
        break
    fi
    sleep 2
done
[[ "$healthy" == true ]] || fail "standalone sequencer did not become healthy within 6 minutes"

echo "== run full Quorum lifecycle =="
set +e
if [[ "$PROOF_MODE" == 1 ]]; then
    expected_proof_mode="development"
    default_timeout="30m"
else
    expected_proof_mode="real"
    default_timeout="4h"
fi
RISC0_DEV_MODE="$PROOF_MODE" timeout "${QUORUM_E2E_TIMEOUT:-$default_timeout}" \
    "$E2E_BIN" "$RPC_URL" "$IDENTITY_SEED" 2>&1 | tee "$LIFECYCLE_LOG"
e2e_status=${PIPESTATUS[0]}
set -e
[[ "$e2e_status" -eq 0 ]] || fail "Quorum lifecycle exited with status $e2e_status"

grep -Fxq "proof_mode=$expected_proof_mode" "$LIFECYCLE_LOG" \
    || fail "lifecycle did not report $expected_proof_mode proof mode"
grep -Fxq 'vault_balance=500' "$LIFECYCLE_LOG" || fail "vault balance assertion missing"
grep -Fxq 'recipient_balance=250' "$LIFECYCLE_LOG" || fail "recipient balance assertion missing"
grep -Fxq 'proposal_status=Executed' "$LIFECYCLE_LOG" || fail "execution assertion missing"
grep -Fxq 'RESULT=PASS' "$LIFECYCLE_LOG" || fail "lifecycle PASS marker missing"

echo "SEQUENCER_E2E=PASS"
echo "logs=$LOG_DIR"
echo "elapsed_seconds=$((SECONDS - RUN_STARTED_SECONDS))"
