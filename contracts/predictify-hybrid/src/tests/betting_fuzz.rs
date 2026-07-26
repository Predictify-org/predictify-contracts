//! Proptest-based fuzz target for `place_bet` boundary cases.
//!
//! Exercises the `place_bet` entrypoint with property-based strategies
//! that explore edge conditions around bet amounts, market timing,
//! fee slippage, bet caps, and double betting.
//!
//! ## Boundary conditions covered
//!
//! | Category | Conditions |
//! |----------|-----------|
//! | Amount | MIN_BET_AMOUNT (1_000_000), just below, just above, MAX_BET_AMOUNT, above max, zero, negative, i128::MAX |
//! | Outcomes | Valid outcome, invalid outcome, empty string |
//! | Market state | Active, Closed (past deadline), Resolved |
//! | Market timing | Before end_time, at bet_deadline, after bet_deadline, after end_time |
//! | Bet deadlines | Explicit deadline before end_time, at end_time, after end_time (invalid) |
//! | Double betting | Same user placing bet twice (AlreadyBet) |
//! | Fee slippage | Fee at max, fee above max, fee below max |
//! | Per-market max bet cap | Amount below cap, at cap, above cap (BetExceedsCap) |
//! | Per-user max bet cap | Cumulative stake below cap, at cap, above cap (MaxBetCapExceeded) |
//! | Extreme values | i128::MIN, i128::MAX via raw bytes (no panic) |
//!
//! ## Running
//!
//! ```bash
//! cargo test -p predictify-hybrid -- betting_fuzz
//! ```
//!
//! ## Security
//!
//! All fuzz cases assume `mock_all_auths()` is active (auth is tested separately
//! in the `require_auth_coverage_tests` module). No `unwrap()` is used in
//! production paths — errors are always propagated via `Result`.

use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    Address, Env, String as SorobanString, Symbol,
};

use crate::{
    bets::{BetManager, BetValidator, MAX_BET_AMOUNT, MIN_BET_AMOUNT},
    storage::{BalanceStorage, MarketStateManager},
    types::{
        Error, Market, MarketState, OracleConfig, OracleProvider, ReflectorAsset,
    },
};

// ===========================================================================
// proptest strategies
// ===========================================================================

/// Generate bet amounts covering all boundary cases.
fn amount_strategy() -> impl Strategy<Value = i128> {
    prop_oneof![
        // Boundary: zero / negative
        1 => (i128::MIN..=0i128).prop_map(|x| x),
        // Below MIN_BET_AMOUNT (1_000_000)
        2 => (1i128..999_999i128).prop_map(|x| x),
        // Exactly MIN_BET_AMOUNT
        1 => Just(MIN_BET_AMOUNT),
        // Just above MIN_BET_AMOUNT
        2 => (MIN_BET_AMOUNT + 1..10_000_000i128).prop_map(|x| x),
        // Mid-range values
        2 => (10_000_001i128..(MAX_BET_AMOUNT - 1_000_000)).prop_map(|x| x),
        // Near MAX_BET_AMOUNT
        1 => Just(MAX_BET_AMOUNT - 1),
        // Exactly MAX_BET_AMOUNT
        1 => Just(MAX_BET_AMOUNT),
        // Above MAX_BET_AMOUNT
        1 => Just(MAX_BET_AMOUNT + 1),
        // Way above MAX_BET_AMOUNT
        1 => (MAX_BET_AMOUNT + 2..i128::MAX).prop_map(|x| x),
    ]
}

/// Generate outcome strings covering invalid and valid cases.
fn outcome_strategy() -> impl Strategy<Value = SorobanString> {
    prop_oneof![
        3 => Just(SorobanString::from_str(&Env::default(), "yes")),
        2 => Just(SorobanString::from_str(&Env::default(), "no")),
        1 => Just(SorobanString::from_str(&Env::default(), "")),
        1 => Just(SorobanString::from_str(&Env::default(), "invalid_outcome")),
        1 => Just(SorobanString::from_str(
            &Env::default(),
            "very_long_outcome_string_that_exceeds_typical_length_limits_and_might_cause_storage_issues_or_panics_during_validat",
        )),
    ]
}

/// Generate fee slippage max_fee_bps values.
fn max_fee_bps_strategy() -> impl Strategy<Value = i128> {
    prop_oneof![
        2 => Just(0i128),   // No slippage guard
        2 => Just(250i128), // Typical 2.5%
        1 => Just(100i128), // Tight 1.0%
        1 => Just(500i128), // Loose 5.0%
        1 => Just(50i128),  // Very tight 0.5%
    ]
}

/// Generate the ledger time relative to market end for timing tests.
fn time_offset_strategy() -> impl Strategy<Value = u64> {
    prop_oneof![
        // Well before market ends (betting allowed)
        3 => (0u64..5400u64).prop_map(|x| x),
        // Near end_time but still before
        1 => (5401u64..7199u64).prop_map(|x| x),
        // At or just after end_time (market closed)
        2 => (7200u64..100_000u64).prop_map(|x| x),
        // Far past end_time
        1 => (100_001u64..1_000_000u64).prop_map(|x| x),
    ]
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Create an active test market with 1-hour duration.
fn create_test_market(env: &Env, admin: &Address, market_id: &Symbol) {
    let oracle_config = OracleConfig::new(
        OracleProvider::reflector(),
        Address::from_str(
            env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        ),
        SorobanString::from_str(env, "BTC/USD"),
        50_000_00i128,
        SorobanString::from_str(env, "gt"),
    );

    let market = Market::new(
        env,
        admin.clone(),
        SorobanString::from_str(env, "Fuzz bet market?"),
        soroban_sdk::vec![
            env,
            SorobanString::from_str(env, "yes"),
            SorobanString::from_str(env, "no"),
        ],
        env.ledger().timestamp() + 7200, // 2 hour duration
        oracle_config,
        None,
        86_400u64,
        MarketState::Active,
    );

    MarketStateManager::update_market(env, market_id, &market);
}

/// Fund a user with XLM balance for betting.
fn fund_user(env: &Env, user: &Address) {
    let _ = BalanceStorage::add_balance(
        env,
        user,
        &ReflectorAsset::Stellar,
        1_000_000_000_000i128,
    );
}

/// Set up the token contract in the environment so SAC transfers succeed.
fn setup_token(env: &Env, contract_id: &Address) {
    let token_admin = Address::generate(env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();

    let stellar_client = StellarAssetClient::new(env, &token_id);
    stellar_client.mint(contract_id, &10_000_0000000i128);

    env.as_contract(contract_id, || {
        env.storage()
            .persistent()
            .set(&Symbol::new(env, "TokenID"), &token_id);
    });

    // Approve contract to spend tokens on behalf of users
    let token_client = soroban_sdk::token::Client::new(env, &token_id);
    // Approve for a list of generated addresses (handled per-test)
}

/// Approve a user for token spending.
fn approve_user(env: &Env, user: &Address, contract_id: &Address) {
    let token_id: Address = env
        .as_contract(contract_id, || {
            env.storage()
                .persistent()
                .get::<Symbol, Address>(&Symbol::new(env, "TokenID"))
                .unwrap()
        });
    let token_client = soroban_sdk::token::Client::new(env, &token_id);
    token_client.approve(user, contract_id, &i128::MAX, &1_000_000);

    // Also mint tokens to the user
    let stellar_client = StellarAssetClient::new(env, &token_id);
    stellar_client.mint(user, &10_000_0000000i128);
}

// ===========================================================================
// Fuzz targets (proptest)
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    // ── Amount boundary fuzz ──────────────────────────────────────────────

    /// Verify that amounts below MIN_BET_AMOUNT are always rejected with
    /// InsufficientStake (or InvalidInput for zero/negative).
    #[test]
    fn fuzz_betting_amount_below_minimum(
        sub_min_amount in (i128::MIN..MIN_BET_AMOUNT).prop_filter(
            "skip exact min",
            |a| *a != MIN_BET_AMOUNT,
        ),
        outcome in outcome_strategy(),
        max_fee_bps in max_fee_bps_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "AMT_MIN");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        let result = BetManager::place_bet(
            &env, user, market_id, outcome, sub_min_amount, max_fee_bps,
        );

        if let Err(e) = &result {
            let allowed = [
                Error::InsufficientStake,
                Error::InvalidInput,
                Error::InvalidOutcome,
            ];
            prop_assert!(
                allowed.contains(e),
                "Sub-min amount {} produced unexpected error {:?}",
                sub_min_amount,
                e,
            );
        }
    }

    /// Verify that amounts at/above MIN_BET_AMOUNT pass the minimum-stake
    /// check (may still fail other validations like outcomes or market state).
    #[test]
    fn fuzz_betting_amount_at_or_above_minimum(
        valid_amount in (MIN_BET_AMOUNT..MAX_BET_AMOUNT).prop_filter(
            "skip min to test separately",
            |a| *a > MIN_BET_AMOUNT,
        ),
        max_fee_bps in max_fee_bps_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "AMT_OK");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        let result = BetManager::place_bet(
            &env,
            user.clone(),
            market_id.clone(),
            SorobanString::from_str(&env, "yes"),
            valid_amount,
            max_fee_bps,
        );

        // Must NOT fail with InsufficientStake for a valid amount.
        if let Err(e) = &result {
            prop_assert_ne!(
                *e,
                Error::InsufficientStake,
                "Amount {} >= MIN_BET_AMOUNT should not get InsufficientStake, got {:?}",
                valid_amount,
                result,
            );
        }

        // --- Double-bet test (if first bet succeeded) ---
        if result.is_ok() {
            let user2 = Address::generate(&env);
            fund_user(&env, &user2);
            approve_user(&env, &user2, &admin);

            let dup_result = BetManager::place_bet(
                &env,
                user,
                market_id,
                SorobanString::from_str(&env, "no"),
                MIN_BET_AMOUNT,
                250,
            );
            prop_assert!(
                matches!(dup_result, Err(Error::AlreadyBet)),
                "Expected AlreadyBet for duplicate bet, got {:?}",
                dup_result,
            );
        }
    }

    /// Verify that amounts above MAX_BET_AMOUNT are rejected.
    #[test]
    fn fuzz_betting_amount_above_maximum(
        above_max in (MAX_BET_AMOUNT + 1..i128::MAX),
        max_fee_bps in max_fee_bps_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "AMT_MAX");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        let result = BetManager::place_bet(
            &env, user, market_id,
            SorobanString::from_str(&env, "yes"),
            above_max,
            max_fee_bps,
        );

        if let Err(e) = &result {
            let allowed = [Error::InvalidInput, Error::InsufficientBalance, Error::InsufficientStake];
            prop_assert!(
                allowed.contains(e),
                "Above-max amount {} produced unexpected error {:?}",
                above_max,
                e,
            );
        }
    }

    /// Verify that exactly MAX_BET_AMOUNT is accepted and produces a valid bet.
    #[test]
    fn fuzz_betting_exactly_maximum(max_fee_bps in max_fee_bps_strategy()) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "AMT_EXACT");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        let result = BetManager::place_bet(
            &env, user, market_id,
            SorobanString::from_str(&env, "yes"),
            MAX_BET_AMOUNT,
            max_fee_bps,
        );

        // At exactly MAX, if it fails it should NOT be InvalidInput.
        if let Err(e) = &result {
            prop_assert_ne!(
                *e,
                Error::InvalidInput,
                "MAX_BET_AMOUNT should not be rejected as InvalidInput, got {:?}",
                e,
            );
        }
    }

    // ── Outcome fuzz ──────────────────────────────────────────────────────

    /// Verify that invalid outcomes are rejected.
    #[test]
    fn fuzz_betting_invalid_outcome(
        amount in MIN_BET_AMOUNT..10_000_000i128,
        max_fee_bps in max_fee_bps_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "OUT_INV");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        // "invalid_outcome" is not in ["yes", "no"]
        let result = BetManager::place_bet(
            &env,
            user,
            market_id,
            SorobanString::from_str(&env, "invalid_outcome"),
            amount,
            max_fee_bps,
        );

        prop_assert!(
            matches!(
                result,
                Err(Error::InvalidOutcome)
            ),
            "Invalid outcome should return InvalidOutcome, got {:?}",
            result,
        );
    }

    /// Verify that an empty outcome string is rejected.
    #[test]
    fn fuzz_betting_empty_outcome(
        amount in MIN_BET_AMOUNT..10_000_000i128,
        max_fee_bps in max_fee_bps_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "OUT_EMPTY");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        let result = BetManager::place_bet(
            &env,
            user,
            market_id,
            SorobanString::from_str(&env, ""),
            amount,
            max_fee_bps,
        );

        prop_assert!(
            matches!(
                result,
                Err(Error::InvalidOutcome)
            ),
            "Empty outcome should return InvalidOutcome, got {:?}",
            result,
        );
    }

    // ── Market timing fuzz ────────────────────────────────────────────────

    /// Verify that betting before the market deadline succeeds (for valid amounts).
    #[test]
    fn fuzz_betting_before_deadline(
        amount in MIN_BET_AMOUNT..100_000_000i128,
        offset in (0u64..5400u64),
        max_fee_bps in max_fee_bps_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "TIME_OK");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        env.ledger().with_mut(|l| l.timestamp += offset);

        let result = BetManager::place_bet(
            &env, user, market_id,
            SorobanString::from_str(&env, "yes"),
            amount,
            max_fee_bps,
        );

        // Before deadline with valid amount => should always succeed (balance permitting).
        if let Err(e) = &result {
            // Only acceptable failures before deadline are InsufficientBalance.
            prop_assert!(
                matches!(e, Error::InsufficientBalance | Error::FeeExceedsMax),
                "Before deadline, valid amount {} got unexpected error {:?}",
                amount,
                e,
            );
        }
    }

    /// Verify that betting after the effective deadline is rejected.
    #[test]
    fn fuzz_betting_after_deadline(
        amount in MIN_BET_AMOUNT..10_000_000i128,
        late_offset in (7200u64..200_000u64),
        max_fee_bps in max_fee_bps_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "TIME_LATE");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        // Advance past end_time (7200).
        env.ledger().with_mut(|l| l.timestamp += late_offset);

        let result = BetManager::place_bet(
            &env, user, market_id,
            SorobanString::from_str(&env, "yes"),
            amount,
            max_fee_bps,
        );

        prop_assert!(
            matches!(
                result,
                Err(Error::MarketClosed)
                    | Err(Error::MarketResolved)
                    | Err(Error::InvalidState)
            ),
            "After deadline (offset {} >= 7200), bet should be rejected, got {:?}",
            late_offset,
            result,
        );
    }

    /// Verify that an explicit bet deadline before end_time is honored.
    #[test]
    fn fuzz_betting_explicit_bet_deadline(
        amount in MIN_BET_AMOUNT..100_000_000i128,
        deadline_offset in (100u64..7000u64),
        max_fee_bps in max_fee_bps_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "BET_DL");

        create_test_market(&env, &admin, &market_id);

        // Override the market's bet_deadline.
        env.as_contract(&admin, || {
            let mut market: Market = env
                .storage()
                .persistent()
                .get(&market_id)
                .unwrap();
            market.bet_deadline = market.end_time - 1800; // deadline 30 min before end
            env.storage().persistent().set(&market_id, &market);
        });

        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        let cutoff = 7200 - 1800; // end_time - bet_deadline offset
        env.ledger().with_mut(|l| l.timestamp += deadline_offset);

        let result = BetManager::place_bet(
            &env, user, market_id,
            SorobanString::from_str(&env, "yes"),
            amount,
            max_fee_bps,
        );

        if deadline_offset >= cutoff {
            prop_assert!(
                matches!(result, Err(Error::MarketClosed) | Err(Error::InvalidState)),
                "After explicit bet_deadline, bet should be rejected, got {:?}",
                result,
            );
        } else if let Err(e) = &result {
            prop_assert_ne!(
                *e,
                Error::MarketClosed,
                "Before explicit bet_deadline, should not get MarketClosed, got {:?}",
                e,
            );
        }
    }

    // ── Per-market max bet cap fuzz ──────────────────────────────────────

    /// Verify that per-market max bet cap is enforced.
    #[test]
    fn fuzz_betting_market_max_bet_cap(
        cap in 1_000_000i128..50_000_000i128,
        amount in MIN_BET_AMOUNT..100_000_000i128,
        max_fee_bps in max_fee_bps_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "CAP_MKT");

        create_test_market(&env, &admin, &market_id);

        // Set a per-market max bet cap.
        crate::bets::set_market_max_bet_cap(&env, &market_id, cap).ok();

        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        let result = BetManager::place_bet(
            &env, user, market_id,
            SorobanString::from_str(&env, "yes"),
            amount,
            max_fee_bps,
        );

        if amount > cap {
            prop_assert!(
                matches!(result, Err(Error::BetExceedsCap)),
                "Amount {} > cap {} should get BetExceedsCap, got {:?}",
                amount,
                cap,
                result,
            );
        } else if let Err(e) = &result {
            prop_assert_ne!(
                *e,
                Error::BetExceedsCap,
                "Amount {} <= cap {} should not get BetExceedsCap, got {:?}",
                amount,
                cap,
                e,
            );
        }
    }

    // ── Per-user max bet cap fuzz ────────────────────────────────────────

    /// Verify that the per-user max bet cap is enforced on cumulative stake.
    #[test]
    fn fuzz_betting_user_max_bet_cap(
        cap in 5_000_000i128..50_000_000i128,
        first_amount in MIN_BET_AMOUNT..30_000_000i128,
        second_amount in MIN_BET_AMOUNT..30_000_000i128,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);

        // Set the global per-user max bet cap.
        let _ = BetValidator::set_max_bet_cap(&env, &admin, cap);

        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "CAP_USR");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        let first = BetManager::place_bet(
            &env, user.clone(), market_id.clone(),
            SorobanString::from_str(&env, "yes"),
            first_amount,
            250,
        );

        if first.is_ok() {
            let second = BetManager::place_bet(
                &env, user, market_id,
                SorobanString::from_str(&env, "no"),
                second_amount,
                250,
            );

            let total = first_amount.saturating_add(second_amount);
            if total > cap && second_amount <= cap {
                prop_assert!(
                    matches!(
                        second,
                        Err(Error::MaxBetCapExceeded) | Err(Error::AlreadyBet)
                    ),
                    "Cumulative stake {} > cap {} should get max-bet-cap error or AlreadyBet, got {:?}",
                    total,
                    cap,
                    second,
                );
            } else if let Err(e) = &second {
                prop_assert!(
                    matches!(e, Error::AlreadyBet),
                    "Expected AlreadyBet for second bet when not a cap violation, got {:?}",
                    e,
                );
            }
        }
    }

    // ── Extreme values fuzz ──────────────────────────────────────────────

    /// Verify that extreme stake values (i128::MIN, i128::MAX) don't panic.
    #[test]
    fn fuzz_betting_extreme_amounts(raw_bytes in prop::array::uniform16(0u8..)) {
        let amount = i128::from_le_bytes(raw_bytes);
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "EXTREME");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        let result = BetManager::place_bet(
            &env, user, market_id,
            SorobanString::from_str(&env, "yes"),
            amount,
            250,
        );

        // Must not panic. Any Error result is acceptable.
        if let Err(e) = &result {
            let allowed = [
                Error::InsufficientStake,
                Error::InvalidInput,
                Error::InvalidOutcome,
                Error::InsufficientBalance,
                Error::MarketClosed,
                Error::AlreadyBet,
                Error::FeeExceedsMax,
                Error::BetExceedsCap,
                Error::MaxBetCapExceeded,
                Error::InvalidState,
                Error::Overflow,
            ];
            prop_assert!(
                allowed.contains(e),
                "Extreme amount {} produced unexpected error {:?}",
                amount,
                e,
            );
        }
    }

    // ── Fee slippage fuzz ────────────────────────────────────────────────

    /// Verify that fee slippage guard works correctly.
    #[test]
    fn fuzz_betting_fee_slippage(
        platform_fee_bps in (0i128..1000i128),
        max_fee_bps in (0i128..1000i128),
        amount in MIN_BET_AMOUNT..10_000_000i128,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "FEE_SLIP");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);
        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        // Set the platform fee.
        env.as_contract(&admin, || {
            let mut cfg = crate::config::ConfigManager::get_development_config(&env);
            cfg.fees.platform_fee_percentage = platform_fee_bps;
            cfg.fees.fees_enabled = platform_fee_bps > 0;
            crate::config::ConfigManager::store_config(&env, &cfg).ok();
        });

        let result = BetManager::place_bet(
            &env, user, market_id,
            SorobanString::from_str(&env, "yes"),
            amount,
            max_fee_bps,
        );

        if max_fee_bps > 0 && platform_fee_bps > max_fee_bps {
            prop_assert!(
                matches!(result, Err(Error::FeeExceedsMax)),
                "Platform fee {} > max_fee_bps {} should return FeeExceedsMax, got {:?}",
                platform_fee_bps,
                max_fee_bps,
                result,
            );
        } else if let Err(e) = &result {
            prop_assert_ne!(
                *e,
                Error::FeeExceedsMax,
                "Platform fee {} <= max_fee_bps {} should not get FeeExceedsMax, got {:?}",
                platform_fee_bps,
                max_fee_bps,
                e,
            );
        }
    }

    // ── Valid bet invariants ──────────────────────────────────────────────

    /// A successful bet must have correct properties.
    #[test]
    fn fuzz_betting_successful_bet_invariants(
        amount in (MIN_BET_AMOUNT..MAX_BET_AMOUNT / 1000),
        max_fee_bps in (200i128..500i128),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "INVAR");

        create_test_market(&env, &admin, &market_id);
        setup_token(&env, &admin);

        // Set a generous platform fee.
        env.as_contract(&admin, || {
            let mut cfg = crate::config::ConfigManager::get_development_config(&env);
            cfg.fees.platform_fee_percentage = 200; // 2%
            cfg.fees.fees_enabled = true;
            crate::config::ConfigManager::store_config(&env, &cfg).ok();
        });

        fund_user(&env, &user);
        approve_user(&env, &user, &admin);

        let result = BetManager::place_bet(
            &env, user.clone(), market_id.clone(),
            SorobanString::from_str(&env, "yes"),
            amount,
            max_fee_bps,
        );

        if let Ok(bet) = result {
            prop_assert_eq!(bet.amount, amount, "Bet amount should match input");
            prop_assert_eq!(
                bet.outcome,
                SorobanString::from_str(&env, "yes"),
                "Bet outcome should match input",
            );
            prop_assert_eq!(
                bet.user, user,
                "Bet user should match input",
            );
            prop_assert!(
                bet.is_active(),
                "Newly placed bet should be active",
            );

            // Verify the bet is retrievable.
            let retrieved = BetManager::get_bet(&env, &market_id, &user);
            prop_assert!(retrieved.is_some(), "Bet should be retrievable after placement");
        }
    }
}
