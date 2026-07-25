## Summary

Add `Recovery::dry_run` — a read-only entrypoint that returns the recovery plan without executing any side effects. Useful for ops verification before executing a live recovery.

Closes #708

## Changes

### `contracts/predictify-hybrid/src/recovery.rs`

- **New type**: `DryRunResult` (contracttype) with fields:
  - `integrity_ok: bool` — whether the validator passes
  - `can_recover: bool` — whether reconstruction is possible
  - `issues_detected: Vec<String>` — detected problems (e.g. `negative_total_staked`, `total_staked_mismatch`, `stake_overflow`, `too_few_outcomes`, `zero_end_time`)
  - `planned_actions: Vec<String>` — what `recover_market_state` would do (e.g. `no_action_needed`, `reconstruct_totals`, `skip_cannot_reconstruct_closed_or_cancelled`)
  - `state_description: String` — human-readable market state

- **New function**: `RecoveryManager::recovery_dry_run(env, market_id) -> Result<DryRunResult, Error>`
  - Mirrors `recover_market_state` logic but is **strictly read-only**
  - No storage writes, no event emissions, **no admin authentication required**
  - Uses `checked_add` with explicit overflow detection for stake summation
  - Correctly identifies Closed/Cancelled markets as non-recoverable

### `contracts/predictify-hybrid/src/lib.rs`

- **New entrypoint**: `PredictifyHybrid::recovery_dry_run(env, market_id) -> DryRunResult`
  - Thin wrapper that delegates to `RecoveryManager` and panics on error (e.g. market not found)

### Tests (8 new tests in `recovery.rs` test module)

| Test | What it covers |
|------|---------------|
| `test_recovery_dry_run_market_not_found` | Error on non-existent market |
| `test_recovery_dry_run_valid_market` | `integrity_ok = true`, `no_action_needed` |
| `test_recovery_dry_run_closed_market` | Closed market → `can_recover = false` |
| `test_recovery_dry_run_cancelled_market` | Cancelled market → `can_recover = false` |
| `test_recovery_dry_run_total_staked_mismatch` | Stakes sum ≠ total_staked detection |
| `test_recovery_dry_run_stakes_map_sums_correctly` | Consistent stakes → `integrity_ok` |
| `test_recovery_dry_run_no_auth_required` | Works without authentication |
| `test_dry_run_result_struct_fields` | Struct construction and field access |
