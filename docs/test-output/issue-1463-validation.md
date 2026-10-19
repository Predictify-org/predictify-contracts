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

Validation in progress. Production changes, if needed, must include baseline
failure evidence and a focused regression test.

## Measurements

Validation in progress. Coverage and timing measurements are recorded after
running the before/after commands, without estimating CI runner timings.
