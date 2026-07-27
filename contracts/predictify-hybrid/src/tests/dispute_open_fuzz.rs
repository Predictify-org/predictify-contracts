//! Proptest-based fuzz target for `dispute_open` boundary cases.
//!
//! Exercises the `process_dispute` entrypoint with property-based strategies
//! that explore edge conditions around stake amounts, market timing,
//! anti-grief floors, duplicate disputes, and stake caps.
//!
//! ## Boundary conditions covered
//!
//! | Category | Conditions |
//! |----------|-----------|
//! | Stake | `MIN_DISPUTE_STAKE` (10_000_000), just below, just above, i128::MAX, zero, negative |
//! | Market timing | Before `end_time`, after `end_time`, after dispute window closes |
//! | Anti-grief | Stake below floor, at floor, above floor |
//! | Duplicates | Same user disputing twice |
//! | Stake caps | Per-market per-user cap exceeded |
//! | Reason | `None`, empty `Some`, long string |
//!
//! ## Running
//!
//! ```bash
//! cargo test -p predictify-hybrid -- dispute_open_fuzz
//! ```
//!
//! ## Security
//!
//! All fuzz cases assume `mock_all_auths()` is active (auth is tested separately
//! in the `require_auth_coverage_tests` module). No `unwrap()` is used in
//! production paths — errors are always propagated via `Result`.

use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String as SorobanString, Symbol,
};

use crate::{
    disputes::DisputeManager,
    storage::{BalanceStorage, DataKey, MarketStateManager},
    types::{Error, Market, MarketState, OracleConfig, OracleProvider},
    ReflectorAsset,
};

// ===========================================================================
// proptest strategies
// ===========================================================================

/// Generate stake values covering all boundary cases.
fn stake_strategy() -> impl Strategy<Value = i128> {
    prop_oneof![
        // Boundary: zero / negative
        1 => (i128::MIN..=0i128).prop_map(|x| x),
        // Below MIN_DISPUTE_STAKE (10_000_000)
        2 => (1i128..9_999_999i128).prop_map(|x| x),
        // Exactly MIN_DISPUTE_STAKE
        1 => Just(10_000_000i128),
        // Just above MIN_DISPUTE_STAKE
        2 => (10_000_001i128..100_000_000i128).prop_map(|x| x),
        // Large values (up to i128::MAX, clamped for fuzz speed)
        2 => (100_000_001i128..1_000_000_000_000i128).prop_map(|x| x),
    ]
}

/// Generate reason options covering all boundary cases.
fn reason_strategy() -> impl Strategy<Value = Option<SorobanString>> {
    prop_oneof![
        2 => Just(None),
        1 => Just(Some(SorobanString::from_str(&Env::default(), ""))),
        2 => Just(Some(SorobanString::from_str(&Env::default(), "Fuzz dispute"))),
        1 => Just(Some(SorobanString::from_str(
            &Env::default(),
            "Very long dispute reason that might overflow or cause issues with storage encoding",
        ))),
    ]
}

/// Generate the end-time offset from the current ledger time.
fn end_time_offset_strategy() -> impl Strategy<Value = u64> {
    prop_oneof![
        // Market not yet ended (current < end_time)
        1 => (1u64..3599u64).prop_map(|x| x),
        // Market recently ended
        3 => (3601u64..100_000u64).prop_map(|x| x),
        // Market ended long ago (past dispute window)
        1 => (200_000u64..1_000_000_000u64).prop_map(|x| x),
    ]
}

// ===========================================================================
// Helpers
// ===========================================================================

fn create_disputable_market(env: &Env, admin: &Address, market_id: &Symbol, time_since_end: u64) {
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
        SorobanString::from_str(env, "Fuzz market?"),
        soroban_sdk::vec![
            env,
            SorobanString::from_str(env, "yes"),
            SorobanString::from_str(env, "no")
        ],
        env.ledger().timestamp() + 3600,
        oracle_config,
        None,
        86_400u64,
        MarketState::Active,
    );

    // Advance past end_time and set oracle result.
    env.ledger().with_mut(|l| l.timestamp += time_since_end);

    let mut ended_market = market;
    ended_market.oracle_result = Some(SorobanString::from_str(env, "yes"));
    MarketStateManager::update_market(env, market_id, &ended_market);
}

fn fund_user(env: &Env, user: &Address) {
    let _ = BalanceStorage::add_balance(env, user, &ReflectorAsset::Stellar, 1_000_000_000_000i128);
}

// ===========================================================================
// Fuzz targets (proptest)
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// Verify that stake values below `MIN_DISPUTE_STAKE` are always rejected.
    #[test]
    fn fuzz_dispute_open_stake_below_minimum(
        sub_min_stake in 1i128..9_999_999i128,
        reason in reason_strategy(),
        time_offset in end_time_offset_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "STK_MIN");

        create_disputable_market(&env, &admin, &market_id, time_offset);
        fund_user(&env, &user);

        let result = DisputeManager::process_dispute(
            &env, user, market_id, sub_min_stake, reason,
        );

        prop_assert!(
            matches!(result, Err(Error::InsufficientStake)),
            "Stake {} below MIN_DISPUTE_STAKE should be rejected, got {:?}",
            sub_min_stake,
            result,
        );
    }

    /// Verify that stake values at / above `MIN_DISPUTE_STAKE` pass the
    /// minimum-stake check (may still fail other validations).
    #[test]
    fn fuzz_dispute_open_stake_at_or_above_minimum(
        valid_stake in 10_000_000i128..1_000_000_000i128,
        reason in reason_strategy(),
        time_offset in (3601u64..100_000u64),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "STK_OK");

        create_disputable_market(&env, &admin, &market_id, time_offset);
        fund_user(&env, &user);

        let result = DisputeManager::process_dispute(
            &env, user.clone(), market_id.clone(), valid_stake, reason,
        );

        // Must NOT fail with InsufficientStake for a stake >= MIN_DISPUTE_STAKE.
        if let Err(e) = &result {
            prop_assert_ne!(
                *e,
                Error::InsufficientStake,
                "Stake {} >= MIN_DISPUTE_STAKE should not get InsufficientStake, got {:?}",
                valid_stake,
                result,
            );
        }

        // ----- Duplicate dispute test -----
        if result.is_ok() {
            let dup_result = DisputeManager::process_dispute(
                &env, user, market_id, valid_stake, None,
            );
            prop_assert!(
                matches!(dup_result, Err(Error::AlreadyDisputed)),
                "Expected AlreadyDisputed for duplicate, got {:?}",
                dup_result,
            );
        }
    }

    /// Verify that disputing before the market has ended is rejected.
    #[test]
    fn fuzz_dispute_open_before_end_time(
        stake in 10_000_000i128..100_000_000i128,
        before_end_offset in (1u64..3599u64),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "PRE_END");

        let oracle_config = OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(
                &env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            SorobanString::from_str(&env, "BTC/USD"),
            50_000_00i128,
            SorobanString::from_str(&env, "gt"),
        );

        // Create market with end_time far in the future.
        let _market = Market::new(
            &env,
            admin,
            SorobanString::from_str(&env, "Fuzz pre-end?"),
            soroban_sdk::vec![&env, SorobanString::from_str(&env, "yes"), SorobanString::from_str(&env, "no")],
            env.ledger().timestamp() + 100_000,
            oracle_config,
            None,
            86_400u64,
            MarketState::Active,
        );

        // Advance only a little so we're still before end_time.
        env.ledger().with_mut(|l| l.timestamp += before_end_offset);

        let result = DisputeManager::process_dispute(
            &env, user, market_id, stake, None,
        );

        prop_assert!(
            matches!(result, Err(Error::MarketClosed) | Err(Error::InvalidState)),
            "Expected MarketClosed or InvalidState for dispute before end_time, got {:?}",
            result,
        );
    }

    /// Verify that disputing after the dispute window has closed is rejected.
    #[test]
    fn fuzz_dispute_open_after_window_closes(
        stake in 10_000_000i128..100_000_000i128,
        late_offset in (200_000u64..5_000_000u64),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "POST_WIN");

        let oracle_config = OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(
                &env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            SorobanString::from_str(&env, "BTC/USD"),
            50_000_00i128,
            SorobanString::from_str(&env, "gt"),
        );

        let market = Market::new(
            &env,
            admin,
            SorobanString::from_str(&env, "Fuzz window?"),
            soroban_sdk::vec![&env, SorobanString::from_str(&env, "yes"), SorobanString::from_str(&env, "no")],
            env.ledger().timestamp() + 3600,
            oracle_config,
            None,
            86_400u64,
            MarketState::Active,
        );

        // Advance far past end_time + dispute_window_seconds (86400).
        env.ledger().with_mut(|l| l.timestamp += 3600 + late_offset);

        let mut ended = market;
        ended.oracle_result = Some(SorobanString::from_str(&env, "yes"));
        MarketStateManager::update_market(&env, &market_id, &ended);

        let result = DisputeManager::process_dispute(
            &env, user, market_id, stake, None,
        );

        prop_assert!(
            matches!(result, Err(Error::MarketResolved) | Err(Error::InvalidState)),
            "Expected MarketResolved or InvalidState for dispute after window, got {:?}",
            result,
        );
    }

    /// Verify that per-market per-user dispute stake caps are enforced.
    #[test]
    fn fuzz_dispute_open_stake_cap_enforced(
        cap in 1_000_000i128..20_000_000i128,
        dispute_stake in 10_000_000i128..100_000_000i128,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "CAP");

        create_disputable_market(&env, &admin, &market_id, 7200);
        fund_user(&env, &user);

        // Set a per-market per-user cap.
        let cap_key = DataKey::DisputeStakeCap(market_id.clone(), user.clone());
        env.storage().persistent().set(&cap_key, &cap);
        env.storage().persistent().extend_ttl(&cap_key, 535680, 535680);

        let result = DisputeManager::process_dispute(
            &env, user, market_id, dispute_stake, None,
        );

        if dispute_stake > cap {
            prop_assert!(
                matches!(result, Err(Error::DisputeStakeCapExceeded)),
                "Expected DisputeStakeCapExceeded for stake {} > cap {}, got {:?}",
                dispute_stake,
                cap,
                result,
            );
        } else if let Err(e) = &result {
            prop_assert_ne!(
                *e,
                Error::DisputeStakeCapExceeded,
                "Should not get cap exceeded for stake {} <= cap {}",
                dispute_stake,
                cap,
            );
        }
    }

    /// Verify that extreme stake values (including i128::MIN, i128::MAX)
    /// don't cause panics.
    #[test]
    fn fuzz_dispute_open_extreme_stakes(
        raw_bytes in prop::array::uniform16(0u8..),
    ) {
        let stake = i128::from_le_bytes(raw_bytes);
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "EXTREME");

        create_disputable_market(&env, &admin, &market_id, 7200);
        fund_user(&env, &user);

        let result = DisputeManager::process_dispute(
            &env, user, market_id, stake, None,
        );

        // Must not panic. Any Error result is acceptable.
        if let Err(e) = &result {
            let allowed = [
                Error::InsufficientStake,
                Error::InvalidState,
                Error::AlreadyDisputed,
                Error::DisputeStakeCapExceeded,
                Error::InsufficientBalance,
                Error::MarketResolved,
                Error::OracleUnavailable,
                Error::RateLimitExceeded,
            ];
            prop_assert!(
                allowed.contains(e),
                "Extreme stake {} produced unexpected error {:?}",
                stake,
                e,
            );
        }
    }

    /// Verify various reason strings don't cause panics.
    #[test]
    fn fuzz_dispute_open_reason_variants(
        stake in 10_000_000i128..100_000_000i128,
        reason_len in (0usize..256usize),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "REASON");

        create_disputable_market(&env, &admin, &market_id, 7200);
        fund_user(&env, &user);

        let base = "A".repeat(reason_len);
        let reason = Some(SorobanString::from_str(&env, &base[..reason_len.min(256)]));

        let result = DisputeManager::process_dispute(
            &env, user, market_id, stake, reason,
        );

        // Must not panic; any result is fine.
        let _ = result;
    }
}
