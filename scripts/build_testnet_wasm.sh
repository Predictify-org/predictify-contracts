#!/usr/bin/env bash
# Build the slim predictify-hybrid wasm that fits Stellar's 128 KiB contract
# size limit: core entry points only (--no-default-features) + spec doc strip.
# Output: target/wasm32v1-none/release/predictify_hybrid.slim.wasm
set -euo pipefail
cd "$(dirname "$0")/.."
stellar contract build --package predictify-hybrid --no-default-features
W=target/wasm32v1-none/release
python3 scripts/strip_wasm_spec.py "$W/predictify_hybrid.wasm" "$W/predictify_hybrid.slim.wasm"
SIZE=$(wc -c < "$W/predictify_hybrid.slim.wasm")
echo "slim wasm: $SIZE bytes (limit 131072)"
[ "$SIZE" -le 131072 ] || { echo "too large for on-chain deploy"; exit 1; }
