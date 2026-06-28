Closes #633

## Description

Per-market oracle deviation is currently a single bound. This PR tracks a rolling deviation history and rejects quotes that deviate from the rolling median by more than a configurable z-multiple.

## Changes

### `contracts/predictify-hybrid/src/err.rs`
- Added `OracleQuoteOutlier` error variant (code 507) with description and canonical string code

### `contracts/predictify-hybrid/src/types.rs`
- Added `max_deviation_z_multiple: Option<u32>` to `GlobalOracleValidationConfig` — configurable z-multiple threshold in basis points (e.g., 500 = 5%)
- Added `history_size: Option<u32>` to `GlobalOracleValidationConfig` — configurable ring-buffer depth (defaults to 10)
- Same fields added to `EventOracleValidationConfig` for per-event overrides

### `contracts/predictify-hybrid/src/oracles.rs`
- **`OracleDeviationHistory`**: New ring-buffer type storing historical prices per market with FIFO eviction
  - `push()` — insert price; evicts oldest when at capacity
  - `pop_last()` — revert the last push (used when an outlier is rejected)
  - `rolling_median()` — compute median using i128 integer sort (even count → lower middle)
  - `mad()` — Median Absolute Deviation for future use
- **`OracleValidationConfigManager`**:
  - Updated `get_effective_config` to pass through new fields
  - Added `get_or_init_history`, `save_history`, `history_key` for per-market ring buffer storage
  - Modified `validate_oracle_data`: when `max_deviation_z_multiple` is set, computes rolling median from the ring buffer and rejects quotes deviating beyond the threshold with `OracleQuoteOutlier`. Outliers are not persisted in the history. When `max_deviation_z_multiple` is `None`, falls back to legacy single-reference `max_deviation_bps` check.
  - Added `get_deviation_history` and `clear_deviation_history` public helpers
  - Updated `validate_config_values` to validate `max_deviation_z_multiple` (rejects 0 or > 10_000 bps)
  - Updated `set_global_config` and `set_event_config` to pass the new field through validation

### `contracts/predictify-hybrid/src/tests/oracle_rolling_deviation_tests.rs`
Comprehensive test suite (15 tests):
- `OracleDeviationHistory` unit tests: empty, single, odd/even median, FIFO eviction, pop_last, MAD, capacity 0, determinism
- Rolling median integration tests: first price accepted, similar price accepted, outlier rejected, outlier not persisted, multiple stable prices pass, clear history
- Legacy deviation compatibility test (when rolling median disabled)
- Config validation tests (zero/threshold bounds)

## Acceptance Criteria
- [x] Outlier rejection deterministic (same inputs → same median)
- [x] History size from config (defaults to 10)
- [x] No `unwrap()` introduced
- [x] Documented in `oracles.rs`

## Testing

```bash
cargo test -p predictify-hybrid -- oracle_rolling_deviation --nocapture
```

Edge cases covered:
- Empty history (median returns None)
- Single price (always accepted)
- Even/odd entry counts in median calculation
- Ring buffer eviction at capacity
- pop_last on non-empty and empty buffers
- MAD computation with < 2 entries
- Capacity=0 defaults to 1
- Deterministic median from same inputs
- Outlier not persisted in history
- Multiple stable prices accumulate in history
- Legacy deviation still works when rolling median disabled
