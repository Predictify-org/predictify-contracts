#!/usb/env bash
# Build the predictify-analytics contract WASM and enforce the size budget.
#
# The deployable analytics contract must fit within the Soroban 128 KiB contract
# limit. This script fails if the produced WASM exceeds 131,072 bytes (128 KiB).
#
set -euo pipefail

MAX_SIZE_BYTES=131072
CRATE_DIR="contracts/predictify-analytics"
WASM_NAME="predictify_analytics.wasm"

ROOT_DIR="$(cd "$(dirname "${"}