#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MAX_SIZE_BYTES="${WASM_SIZE_BUDGET:-131072}"
WASM_FILE="$ROOT_DIR/target/wasm32v1-none/release/predictify_analytics.wasm"

cd "$ROOT_DIR"
rustup target add wasm32v1-none
cargo build --release --target wasm32v1-none -p predictify-analytics

if [[ ! -f "$WASM_FILE" ]]; then
    echo "Error: analytics WASM was not produced: $WASM_FILE" >&2
    exit 1
fi

SIZE_BYTES="$(wc -c < "$WASM_FILE")"
echo "Analytics WASM: $SIZE_BYTES bytes (limit: $MAX_SIZE_BYTES)"

if (( SIZE_BYTES > MAX_SIZE_BYTES )); then
    echo "Error: analytics WASM exceeds the configured size budget." >&2
    exit 1
fi
