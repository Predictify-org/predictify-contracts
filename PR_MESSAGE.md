# PR: feat(contract): harden oracle timeout and dispute timing (#396)

## Summary

Fixes #396 — `resolution_timeout` could deadlock markets or overlap dispute rules ambiguously.

Three bugs fixed:

### Bug 1 — Disputed markets cancelled by oracle timeout (`resolution.rs`)
`fetch_oracle_result` was cancelling markets via timeout even when they were in `Disputed` state. A disputed market has its own resolution path and must never be force-cancelled.

**Fix:** Added an early guard — if `market.state == Disputed`, return `Error::AlreadyDisputed` immediately.

### Bug 2 — Oracle resolution not blocked during active dispute (`resolution.rs`)
Nothing prevented `fetch_oracle_result` from running on a `Disputed` market, bypassing the dispute process entirely.

**Fix:** Same guard as above — oracle resolution is blocked while a dispute is active.

### Bug 3 — No cross-field validation (`validation.rs`)
`resolution_timeout` could be set shorter than `dispute_window_seconds`. If `resolution_timeout < dispute_window_seconds`, the oracle timeout fires before the dispute window closes — deadlocking the market.

**Fix:** Added cross-field check in `MarketValidator::validate_market_creation`:
```rust
let default_dispute_window = crate::config::DISPUTE_EXTENSION_HOURS as u64 * 3600;
if *resolution_timeout < default_dispute_window {
    result.add_error();
}
```

---

## Files Changed

| File | Change |
|------|--------|
| `src/resolution.rs` | Guard disputed state before timeout check; return `ResolutionTimeoutReached` instead of `InvalidState` |
| `src/validation.rs` | Cross-field check: `resolution_timeout >= dispute_window_seconds` |
| `src/oracle_fallback_timeout_tests.rs` | Replaced 27 placeholder tests with 6 real regression tests |
| `src/lib.rs` | Enable `oracles`, `resolution`, `validation` under `#[cfg(any(test, feature = "testutils"))]`; enable test module |

---

## Invariants Enforced

1. A market in `Disputed` state is **never** cancelled by the resolution timeout.
2. Oracle resolution is blocked while a dispute is active.
3. `resolution_timeout >= dispute_window_seconds` enforced at market creation.
4. A non-disputed market past its timeout is cancelled and returns `ResolutionTimeoutReached`.

## Test Coverage

- `test_disputed_market_not_cancelled_by_timeout`
- `test_oracle_resolution_blocked_during_active_dispute`
- `test_resolution_timeout_shorter_than_dispute_window_is_invalid`
- `test_resolution_timeout_equal_to_dispute_window_is_valid`
- `test_non_disputed_market_cancelled_after_timeout`
- `test_non_disputed_market_within_timeout_not_cancelled`

## Security Notes

**Threat model (before fix):**
- Disputed market cancelled by timeout → users lose funds, dispute bypassed
- Oracle resolution on disputed market → dispute process bypassed
- `resolution_timeout < dispute_window` → market deadlocked, unresolvable

**Non-goals:** Does not change dispute extension logic, multi-dispute handling, or dispute resolution flow.

---

## How to Test

```bash
cargo test -p predictify-hybrid oracle_fallback_timeout_tests --test-threads=1
```

Expected: All 6 tests pass.

---

## Commit

Branch: `feature/oracle-timeout-dispute`
Commit: `2259284`
