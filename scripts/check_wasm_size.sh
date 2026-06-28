#!/bin/bash
set -e

# Default budget: 96 KiB = 96 * 1024 = 98304 bytes
BUDGET=${WASM_SIZE_BUDGET:-98304}

echo "Building contract in release mode..."
# Build the contract. We use stellar contract build as per CI.
# We assume it builds the release version or we use cargo.
# To be sure it's release, we can use cargo build.
cargo build --release --target wasm32-unknown-unknown

# Find the wasm file. 
# It should be in target/wasm32-unknown-unknown/release/*.wasm
WASM_FILE=$(find target/wasm32-unknown-unknown/release -name "*.wasm" | head -n 1)

if [ -z "$WASM_FILE" ]; then
  echo "Error: WASM file not found in target/wasm32-unknown-unknown/release"
  exit 1
fi

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
  exit 1
fi

echo "WASM size is within budget."
