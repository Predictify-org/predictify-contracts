/// Overflow-safe arithmetic tests for score and probability calculations.
///
/// These tests verify that all arithmetic entry points:
/// 1. Produce correct results for normal inputs (success path)
/// 2. Return appropriate errors / safe fallbacks for boundary inputs
/// 3. Never panic or silently wrap for adversarial / maximal inputs
/// 4. Remain idempotent (pure functions with no side effects)
///
/// Mapping to acceptance criteria:
/// - Deterministic for valid, invalid, duplicate, and boundary inputs → every
///   test table is exhaustive on the relevant boundary.
/// - Retries / concurrent execution cannot produce unsafe results → all
///   functions tested here are pure (no mutable state); calling them twice
///   with the same inputs always yields the same result.
/// - Relevant errors make failures diagnosable → tests assert on the exact
///   `Error::Overflow` code rather than just `is_err()`.
#[cfg(test)]
mod arithmetic_utils_tests {
    use crate::err::Error;
    use crate::utils::ArithmeticUtils;

    // ── checked_add ──────────────────────────────────────────────────────────

    #[test]
    fn test_checked_add_normal() {
        assert_eq!(ArithmeticUtils::checked_add(0, 0), Ok(0));
        assert_eq!(ArithmeticUtils::checked_add(100, 200), Ok(300));
        assert_eq!(ArithmeticUtils::checked_add(i128::MAX - 1, 1), Ok(i128::MAX));
    }

    #[test]
    fn test_checked_add_overflow_returns_overflow_error() {
        let result = ArithmeticUtils::checked_add(i128::MAX, 1);
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn test_checked_add_negative_lhs_returns_overflow_error() {
        // Contract invariant: negative financial values are invalid
        let result = ArithmeticUtils::checked_add(-1, 100);
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn test_checked_add_negative_rhs_returns_overflow_error() {
        let result = ArithmeticUtils::checked_add(100, -1);
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn test_checked_add_both_negative_returns_overflow_error() {
        let result = ArithmeticUtils::checked_add(-1, -1);
        assert_eq!(result, Err(Error::Overflow));
    }

    // ── checked_mul ──────────────────────────────────────────────────────────

    #[test]
    fn test_checked_mul_normal() {
        assert_eq!(ArithmeticUtils::checked_mul(0, 100), Ok(0));
        assert_eq!(ArithmeticUtils::checked_mul(100, 0), Ok(0));
        assert_eq!(ArithmeticUtils::checked_mul(50, 2), Ok(100));
        assert_eq!(ArithmeticUtils::checked_mul(i128::MAX / 2, 2), Ok(i128::MAX / 2 * 2));
    }

    #[test]
    fn test_checked_mul_overflow_returns_overflow_error() {
        let result = ArithmeticUtils::checked_mul(i128::MAX, 2);
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn test_checked_mul_large_stake_times_100_overflow() {
        // Represents: outcome_amount (near i128::MAX) * 100 — the exact pattern
        // in calculate_implied_probability before the fix.
        let huge_amount = i128::MAX / 50; // * 100 would overflow
        let result = ArithmeticUtils::checked_mul(huge_amount, 100);
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn test_checked_mul_negative_operand_returns_overflow_error() {
        assert_eq!(ArithmeticUtils::checked_mul(-1, 100), Err(Error::Overflow));
        assert_eq!(ArithmeticUtils::checked_mul(100, -1), Err(Error::Overflow));
    }

    // ── checked_mul_div ──────────────────────────────────────────────────────

    #[test]
    fn test_checked_mul_div_normal() {
        // 500_000 * 100 / 1_000_000 = 50 (implied probability: 50%)
        assert_eq!(ArithmeticUtils::checked_mul_div(500_000, 100, 1_000_000), Ok(50));
        // 0 * 100 / anything = 0
        assert_eq!(ArithmeticUtils::checked_mul_div(0, 100, 1_000_000), Ok(0));
        // anything * 0 / anything = 0
        assert_eq!(ArithmeticUtils::checked_mul_div(500_000, 0, 1_000_000), Ok(0));
    }

    #[test]
    fn test_checked_mul_div_boundary_max_value() {
        // Largest safe input: value * numerator exactly fits in i128
        let value = i128::MAX / 100;
        let result = ArithmeticUtils::checked_mul_div(value, 100, i128::MAX);
        assert!(result.is_ok());
    }

    #[test]
    fn test_checked_mul_div_overflow_in_product() {
        // outcome_amount = i128::MAX / 50 → * 100 overflows
        let outcome_amount = i128::MAX / 50;
        let result = ArithmeticUtils::checked_mul_div(outcome_amount, 100, 1_000_000);
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn test_checked_mul_div_zero_denominator_returns_overflow_error() {
        let result = ArithmeticUtils::checked_mul_div(500_000, 100, 0);
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn test_checked_mul_div_negative_inputs_return_overflow_error() {
        assert_eq!(ArithmeticUtils::checked_mul_div(-1, 100, 1_000_000), Err(Error::Overflow));
        assert_eq!(ArithmeticUtils::checked_mul_div(500_000, -1, 1_000_000), Err(Error::Overflow));
        assert_eq!(ArithmeticUtils::checked_mul_div(500_000, 100, -1), Err(Error::Overflow));
    }

    // ── checked_accumulate ───────────────────────────────────────────────────

    #[test]
    fn test_checked_accumulate_normal() {
        let mut total = 0i128;
        for stake in [100, 200, 300] {
            total = ArithmeticUtils::checked_accumulate(total, stake).unwrap();
        }
        assert_eq!(total, 600);
    }

    #[test]
    fn test_checked_accumulate_overflow_returns_overflow_error() {
        let result = ArithmeticUtils::checked_accumulate(i128::MAX, 1);
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn test_checked_accumulate_zero_item() {
        let total = ArithmeticUtils::checked_accumulate(500, 0).unwrap();
        assert_eq!(total, 500);
    }

    #[test]
    fn test_checked_accumulate_negative_item_returns_overflow_error() {
        // Negative stakes are invalid; accumulate must reject them
        let result = ArithmeticUtils::checked_accumulate(100, -1);
        assert_eq!(result, Err(Error::Overflow));
    }

    // ── Idempotency (replay safety) ──────────────────────────────────────────

    #[test]
    fn test_pure_functions_are_idempotent() {
        // Calling the same function twice with the same inputs must return the
        // same result.  This guarantees retry/replay safety.
        let a = ArithmeticUtils::checked_mul_div(50_000, 100, 100_000);
        let b = ArithmeticUtils::checked_mul_div(50_000, 100, 100_000);
        assert_eq!(a, b);

        let c = ArithmeticUtils::checked_add(i128::MAX, 1);
        let d = ArithmeticUtils::checked_add(i128::MAX, 1);
        assert_eq!(c, d); // Both Err(Overflow)
    }
}

// ── NumericUtils fixed functions ─────────────────────────────────────────────

#[cfg(test)]
mod numeric_utils_overflow_tests {
    use crate::utils::NumericUtils;

    #[test]
    fn test_calculate_percentage_normal() {
        // 20 * 500 / 1000 = 10
        assert_eq!(NumericUtils::calculate_percentage(&20, &500, &1000), 10);
        // 0 * anything = 0
        assert_eq!(NumericUtils::calculate_percentage(&0, &500, &1000), 0);
    }

    #[test]
    fn test_calculate_percentage_zero_denominator_returns_zero() {
        // Previously panicked with divide-by-zero; now returns 0 safely
        assert_eq!(NumericUtils::calculate_percentage(&10, &500, &0), 0);
    }

    #[test]
    fn test_calculate_percentage_saturates_instead_of_wrapping() {
        // i128::MAX * i128::MAX would wrap; saturating_mul caps at i128::MAX
        let result = NumericUtils::calculate_percentage(&i128::MAX, &i128::MAX, &1);
        // Result should be i128::MAX (saturated), not a wrapped value
        assert_eq!(result, i128::MAX);
    }

    #[test]
    fn test_weighted_average_normal() {
        use soroban_sdk::Env;
        let env = Env::default();
        // (10*1 + 20*2) / (1+2) = 50/3 = 16
        let vals = soroban_sdk::vec![&env, 10i128, 20i128];
        let weights = soroban_sdk::vec![&env, 1i128, 2i128];
        assert_eq!(NumericUtils::weighted_average(&vals, &weights), 16);
    }

    #[test]
    fn test_weighted_average_zero_weight_returns_zero() {
        use soroban_sdk::Env;
        let env = Env::default();
        let vals = soroban_sdk::vec![&env, 10i128, 20i128];
        let weights = soroban_sdk::vec![&env, 0i128, 0i128];
        assert_eq!(NumericUtils::weighted_average(&vals, &weights), 0);
    }

    #[test]
    fn test_weighted_average_large_values_saturate_not_wrap() {
        use soroban_sdk::Env;
        let env = Env::default();
        // value * weight would overflow; saturating_mul prevents wrap
        let vals = soroban_sdk::vec![&env, i128::MAX];
        let weights = soroban_sdk::vec![&env, i128::MAX];
        // Should not panic; result is saturated / clamped
        let result = NumericUtils::weighted_average(&vals, &weights);
        // weighted_sum = i128::MAX (saturated), total_weight = i128::MAX
        // weighted_sum / total_weight = 1
        assert_eq!(result, 1);
    }

    #[test]
    fn test_simple_interest_normal() {
        // 1000 * 5 * 2 / 100 = 100
        assert_eq!(NumericUtils::simple_interest(&1000, &5, &2), 100);
    }

    #[test]
    fn test_simple_interest_large_values_saturate_not_wrap() {
        // Previously: i128::MAX * large * large would panic in release with
        // overflow-checks=true; now uses saturating_mul
        let result = NumericUtils::simple_interest(&i128::MAX, &2, &2);
        // saturating: i128::MAX * 2 = i128::MAX, then * 2 = i128::MAX
        // final / 100 = some large value but no panic
        // Just assert it doesn't panic and result is >= 0
        assert!(result >= 0);
    }
}

// ── markets::MarketUtils::calculate_payout ───────────────────────────────────

#[cfg(test)]
mod market_utils_payout_tests {
    use crate::err::Error;
    use crate::markets::MarketUtils;

    #[test]
    fn test_calculate_payout_normal() {
        // user_stake=1000, winning_total=5000, total_pool=10000, fee=2%
        // user_share = 1000 * 98 / 100 = 980
        // payout = 980 * 10000 / 5000 = 1960
        let payout = MarketUtils::calculate_payout(1000, 5000, 10000, 2).unwrap();
        assert_eq!(payout, 1960);
    }

    #[test]
    fn test_calculate_payout_zero_stake() {
        // user with no stake gets 0 payout
        let payout = MarketUtils::calculate_payout(0, 5000, 10000, 2).unwrap();
        assert_eq!(payout, 0);
    }

    #[test]
    fn test_calculate_payout_zero_winning_total_returns_nothing_to_claim() {
        let err = MarketUtils::calculate_payout(1000, 0, 10000, 2).unwrap_err();
        assert_eq!(err, Error::NothingToClaim);
    }

    #[test]
    fn test_calculate_payout_overflow_on_stake_times_fee_complement() {
        // user_stake near i128::MAX; * (100-2) overflows
        let huge_stake = i128::MAX / 50; // * 98 would overflow
        let result = MarketUtils::calculate_payout(huge_stake, 1_000_000, 1_000_000, 2);
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn test_calculate_payout_overflow_on_user_share_times_pool() {
        // After fee deduction, user_share * total_pool still overflows
        let large_share = i128::MAX / 2 + 1;
        let result = MarketUtils::calculate_payout(large_share, large_share, i128::MAX, 0);
        // fee_complement check: 100 - 0 = 100 (ok)
        // user_share = large_share * 100 / 100 = large_share
        // user_share * total_pool = large_share * i128::MAX → overflow
        assert_eq!(result, Err(Error::Overflow));
    }

    #[test]
    fn test_calculate_payout_proportional() {
        // Two users: user1 staked 200, user2 staked 300, total winning = 500
        // total_pool = 1000, fee = 0%
        // user1 payout = 200 * 100/100 * 1000 / 500 = 400
        // user2 payout = 300 * 100/100 * 1000 / 500 = 600
        assert_eq!(MarketUtils::calculate_payout(200, 500, 1000, 0).unwrap(), 400);
        assert_eq!(MarketUtils::calculate_payout(300, 500, 1000, 0).unwrap(), 600);
    }

    #[test]
    fn test_calculate_payout_idempotent() {
        // Same inputs always produce the same output (replay safety)
        let a = MarketUtils::calculate_payout(1000, 5000, 10000, 2);
        let b = MarketUtils::calculate_payout(1000, 5000, 10000, 2);
        assert_eq!(a, b);
    }
}

// ── bets::BetAnalytics ───────────────────────────────────────────────────────

#[cfg(test)]
mod bet_analytics_overflow_tests {
    use crate::bets::BetAnalytics;
    use soroban_sdk::testutils::{Ledger, LedgerInfo};
    use soroban_sdk::{Env, String, Symbol};

    fn test_env() -> Env {
        let env = Env::default();
        env.ledger().set(LedgerInfo {
            timestamp: 1_000_000,
            protocol_version: 25,
            sequence_number: 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 6312000,
        });
        env
    }

    #[test]
    fn test_implied_probability_zero_for_empty_market() {
        let env = test_env();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        let market_id = Symbol::new(&env, "TEST");
        let outcome = String::from_str(&env, "yes");
        // No bets placed: storage access must be wrapped in env.as_contract()
        let prob = env.as_contract(&contract_id, || {
            BetAnalytics::calculate_implied_probability(&env, &market_id, &outcome)
        });
        assert_eq!(prob, 0);
    }

    #[test]
    fn test_payout_multiplier_zero_for_empty_market() {
        let env = test_env();
        let contract_id = env.register_contract(None, crate::PredictifyHybrid);
        let market_id = Symbol::new(&env, "TEST");
        let outcome = String::from_str(&env, "yes");
        let mult = env.as_contract(&contract_id, || {
            BetAnalytics::calculate_payout_multiplier(&env, &market_id, &outcome)
        });
        assert_eq!(mult, 0);
    }
}
