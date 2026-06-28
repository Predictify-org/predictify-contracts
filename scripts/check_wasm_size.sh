#!/bin/bash
set -e

# Default budget: 512 KiB = 512 * 1024 = 524288 bytes
BUDGET=${WASM_SIZE_BUDGET:-524288}

echo "Building contract in release mode..."
cargo build --release --target wasm32v1-none 2>&1

# Find the wasm file
BASE_WASM_FILE=$(find target/wasm32v1-none/release -name "*.wasm" | head -n 1)

if [ -z "$BASE_WASM_FILE" ]; then
  echo "Error: WASM file not found in target/wasm32v1-none/release"
  exit 1
fi

echo "Base WASM: $BASE_WASM_FILE"

# Optimize with stellar contract optimize if available (optimizes in-place)
if command -v stellar &> /dev/null; then
  echo "Optimizing WASM with stellar contract optimize..."
  stellar contract optimize --wasm "$BASE_WASM_FILE" 2>&1 || echo "Warning: WASM optimization failed, continuing with unoptimized binary"
fi

WASM_FILE="$BASE_WASM_FILE"

# Handle both Linux and macOS stat commands
if [[ "$OSTYPE" == "darwin"* ]]; then
  SIZE=$(stat -f%z "$WASM_FILE")
else
  SIZE=$(stat -c%s "$WASM_FILE")
fi

echo "WASM file: $WASM_FILE"
echo "Size: $SIZE bytes"
echo "Budget: $BUDGET bytes"

if [ "$SIZE" -gt "$BUDGET" ]; then
  echo "Error: WASM size ($SIZE bytes) exceeds budget ($BUDGET bytes)!"
  BASE_SIZE=$(stat -f%z "$BASE_WASM_FILE" 2>/dev/null || stat -c%s "$BASE_WASM_FILE" 2>/dev/null)
  echo "Note: base WASM size was $BASE_SIZE bytes"
  exit 1
fi

echo "WASM size is within budget."
