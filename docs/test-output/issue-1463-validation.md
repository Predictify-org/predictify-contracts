# Issue #1463 validation

Closes #1463.

Base revision: `78ce97e`. The source tree has 205 ignore attributes at this
revision (the previous testing guide reported 203). This change restores all
26 ignored bets tests, including five fee-slippage tests, and all five balance
tests. The new source budget is 174.

## Restored tests

| Test | Status |
| --- | --- |
| `bets::test_fee_slippage_guard_accepts_equal_fee` | Restored |
| `bets::test_fee_slippage_guard_accepts_higher_fee` | Restored |
| `bets::test_fee_slippage_guard_rejects_lower_fee` | Restored |
| `bets::test_fee_slippage_guard_fallback_storage` | Restored |
| `bets::test_fee_slippage_guard_default_fallback` | Restored |
| `bets::test_get_effective_bet_limits_returns_defaults_when_empty` | Restored |
| `bets::test_get_effective_bet_limits_prefers_per_event_over_global` | Restored |
| `bets::test_get_effective_bet_limits_falls_back_to_global` | Restored |
| `bets::test_set_event_bet_limits_persists_and_retrieves` | Restored |
| `bets::test_set_market_max_bet_cap_and_get` | Restored |
| `bets::test_remove_market_max_bet_cap` | Restored |
| `bets::test_bet_analytics_implied_probability_empty_market` | Restored |
| `bets::test_bet_analytics_implied_probability_with_data` | Restored |
| `bets::test_bet_analytics_payout_multiplier_empty_outcome` | Restored |
| `bets::test_bet_analytics_payout_multiplier_with_data` | Restored |
| `bets::test_bet_analytics_get_market_summary_delegates` | Restored |
| `bets::test_bet_storage_store_and_get_roundtrip` | Restored |
| `bets::test_bet_storage_remove_bet` | Restored |
| `bets::test_bet_storage_get_all_bets_for_market_empty` | Restored |
| `bets::test_bet_storage_get_all_bets_for_market_multiple` | Restored |
| `bets::test_validate_bet_amount_against_limits_uses_per_event` | Restored |
| `bets::test_validate_bet_amount_against_limits_enforces_market_cap` | Restored |
| `bets::test_atomic_bet_stats_are_prepared_without_writing_storage` | Restored |
| `bets::test_atomic_bet_stats_overflow_is_rejected_before_commit` | Restored |
| `bets::test_atomic_user_stake_is_prepared_without_writing_storage` | Restored |
| `bets::test_atomic_user_stake_overflow_is_rejected_before_commit` | Restored |
| `balances::test_deposit_credits_balance_after_transfer` | Restored |
| `balances::test_withdraw_exact_balance_reaches_zero` | Restored |
| `balances::test_withdraw_allocates_remainder_and_conserves_balances` | Restored |
| `balances::test_withdraw_over_balance_returns_typed_error_without_mutation` | Restored |
| `balances::test_withdraw_transfer_failure_does_not_leave_phantom_debit` | Restored |

## Production defects

All 31 bets/balances tests pass after fixture corrections; those suites required
no production behavior changes. Enabling integration execution exposed the
following small production defects, fixed in separate commits:

| Entrypoint/helper | Defect and correction | Regression test |
| --- | --- | --- |
| `set_treasury` | Entrypoint and queue manager both authenticated the same frame; delegate authentication to the queue manager, keeping the stored-admin check. | `auth_snapshot::snapshot_set_treasury_requires_admin_auth`; added missing-auth and non-admin rejection cases |
| `set_history_cap` | Wrapper repeated the manager's authentication; manager retains auth and primary-admin validation. | `auth_snapshot_disputes::snapshot_set_history_cap_requires_admin_auth` and existing rejection tests |
| `set_anti_grief_floor` | Wrapper repeated the manager's authentication; manager retains auth and primary-admin validation. | `auth_snapshot_disputes::snapshot_set_anti_grief_floor_requires_admin_auth` and existing rejection tests |
| `resolve_dispute` | Wrapper repeated the manager's authentication; manager retains auth and primary-admin validation. | `auth_snapshot_disputes::snapshot_resolve_dispute_requires_admin_auth` and existing rejection tests |
| `check_admin_cooldown` | Authenticated manager methods authenticated again in this helper; remove the repeated auth while retaining admin validation and cooldown logic. All callers authenticate before calling it. | The three dispute regressions above |

[Baseline failure output](issue-1463-baseline-auth.txt) comes from an isolated
`git archive` of `78ce97e`, executed with the original production sources and the
original integration regressions. It records `Error(Auth, ExistingValue)` for
three entrypoints and an auth-induced `Abort` for dispute resolution. All four
regressions pass on the final branch. The existing regressions were previously
outside the CI command; they are now enforced by the integration step.

## Integration repairs

- `err_stability.rs`: repair invalid crate-level documentation syntax.
- `conservation.rs`: correct an invalid oracle address, use stakes above the
  absolute minimum, and reconcile the current automatic payout and fee-vault
  accounting. It checks actual token withdrawals, a one-unit rounding remainder,
  total conservation, and replay of both distribution and fee collection.
- `event_replay_nonce.rs`: account for legacy plus market-scoped event emissions;
  the nonce must increase once per logical emission, independently per topic.
- `stateful.rs`: use supported question/outcome lengths and valid stake ranges.
  Require the first vote to succeed, then assert the exact duplicate-vote error,
  so the idempotency property cannot pass on an invalid initial fixture. Retain
  minimized regression seeds.

No active integration file was disabled. The two existing `.disabled` fuzz files
remain outside scope.

## Coverage

Command on each revision: `cargo llvm-cov -p predictify-hybrid --lib --json`.
The baseline ran on the isolated original revision; the after run used the final
production and harness sources. [Machine-readable bets coverage](issue-1463-bets-coverage.json)
contains the extracted LLVM file summaries.

| `src/bets.rs` metric | Before (`78ce97e`) | After |
| --- | ---: | ---: |
| Lines | 291 / 1,238 (23.51%) | 895 / 1,346 (66.49%) |
| Functions | 29 / 106 (27.36%) | 98 / 132 (74.24%) |
| Regions | 437 / 2,293 (19.06%) | 1,608 / 2,475 (64.97%) |

Line coverage increases by 42.99 percentage points. These are default LLVM file
summaries, including unit-test code and the added closures; they are not a
production-only percentage. Branch coverage was not enabled. The before library
run passed 697 tests with 203 ignored; the after run passed 730 with 172 ignored.

## Validation

| Check | Result |
| --- | --- |
| `cargo test -p predictify-hybrid --lib bets::tests` | 41 passed, none ignored |
| `cargo test -p predictify-hybrid --lib balances` | 5 passed, none ignored |
| `cargo test -p predictify-hybrid --lib` | 730 passed, 172 ignored |
| `cargo test -p predictify-hybrid --tests` | Library plus all 11 integration targets; 119 integration tests, none ignored |
| `cargo test -p hello-world -p oracles` | 3 passed across stable-crate targets |
| `cargo check -p predictify-hybrid --no-default-features` | Passed |
| Ratchet at baseline | 174 / 174, exit 0 |
| Temporary extra source `#[ignore]` | 175 / 174, exit 1; probe removed |
| Harness storage/client visibility and market/funding | Both new tests passed in the library run |

## Timing

Measured on Windows with Rust 1.97.1, SDK 25.3.1, warm Cargo cache, default
property-test case counts, and no concurrent compiler work during the integration
step:

| Step | Local wall-clock seconds |
| --- | ---: |
| Existing library command (warm rerun) | 2.60 |
| Added `cargo test -p predictify-hybrid --tests` step | 39.92 |
| Added ignored-test ratchet | 0.28 |
| Total added steps | 40.20 |

The measured added workflow workload is about 40 seconds with warm artifacts.
`--tests` includes a library rerun (~1.5 seconds of test execution). Its Cargo
preparation was 0.91 seconds in this measurement; property tests dominate runtime.
The earlier library timing overlapped a Cargo lock and was discarded; the table
uses a separate uncontended warm rerun.

No hosted GitHub Actions run was triggered from this local branch, so this is a
local measurement of the added CI commands, not a measured Ubuntu runner delta.
Cold compilation and runner load can change the number. The existing
`Swatinem/rust-cache@v2` remains shared, and the new step appends its actual
elapsed time to `$GITHUB_STEP_SUMMARY` on every successful CI execution.
