# WASM Size Measurement for Issue #1460

## Measurement Instructions

To measure the WASM size delta from sharding the market ID registry:

### Before Measurement (Baseline)

First, checkout the commit before this PR and build:

```bash
git checkout main  # or the parent commit
cd contracts/predictify-hybrid
cargo build --release --target wasm32-unknown-unknown --no-default-features
ls -lh target/wasm32-unknown-unknown/release/predictify_hybrid.wasm
# Record the size
```

### After Measurement (With Changes)

Then checkout this branch and build:

```bash
git checkout fix/market-id-registry
cd contracts/predictify-hybrid
cargo build --release --target wasm32-unknown-unknown --no-default-features
ls -lh target/wasm32-unknown-unknown/release/predictify_hybrid.wasm
# Record the size
```

### Expected Impact

The changes in this PR **remove** the `alloc::string::ToString` import from WASM builds:

- **Removed**: `#[cfg(not(target_family = "wasm"))] use alloc::string::ToString;`
- **Replaced with**: XDR byte inspection that doesn't require string allocation

This should result in a **size reduction** since:
1. We removed string conversion dependency from WASM
2. XDR byte inspection is more lightweight than string operations
3. No additional heavy dependencies were added

### Acceptance Criteria

- ✅ The slim WASM build must be ≤ 131,072 bytes (128 KB)
- ✅ Size should be equal or smaller than baseline (no regression)
- Expected: slight reduction due to removed `ToString` trait

### Size Delta Format for PR

```
WASM Size Impact:
- Before: XXX,XXX bytes
- After:  XXX,XXX bytes
- Delta:  -XXX bytes (X.X% reduction)
- Status: ✅ Under 131,072 byte limit
```

## Verification Commands

```bash
# Build slim WASM (no default features)
cd contracts/predictify-hybrid
cargo build --release --target wasm32-unknown-unknown --no-default-features

# Check size
ls -lh target/wasm32-unknown-unknown/release/*.wasm

# Verify it's under the limit
SIZE=$(stat -f%z target/wasm32-unknown-unknown/release/predictify_hybrid.wasm 2>/dev/null || stat -c%s target/wasm32-unknown-unknown/release/predictify_hybrid.wasm)
if [ $SIZE -le 131072 ]; then
  echo "✅ Size check passed: $SIZE bytes"
else
  echo "❌ Size check failed: $SIZE bytes (exceeds 131,072 limit)"
fi
```

## Notes

- The size measurement should be done with `--no-default-features` to build the slim variant
- If building with `--target wasm32v1-none` (Soroban target), adjust commands accordingly
- Record both the absolute size and percentage change for the PR description
