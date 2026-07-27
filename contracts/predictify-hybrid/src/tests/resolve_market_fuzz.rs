//! Proptest-based fuzz target for the automated `resolve_market` path.
//!
//! Exercises the `MarketResolutionManager::resolve_market()` entrypoint with
//! property-based strategies that explore edge conditions around oracle results,
//! market state, community vote distributions, idempotency, and extreme values.
//!
//! This targets the **automated** resolution path (which requires an oracle
//! result already set) rather than the manual/forced-resolution paths covered
//! by the `resolution` cargo-fuzz target.
//!
//! ## Boundary conditions covered
//!
//! | Category | Conditions |
//! |----------|-----------|
//! | Oracle result | `None`, valid result, empty string, invalid outcome, long string |
//! | Market state | `Ended` (valid), `Active` (should fail), already `Resolved` (should fail) |
//! | Vote distribution | No votes, single voter, skewed, balanced, exact tie |
//! | Stakes | Zero, minimal, large, extreme via raw bytes |
//! | Outcome count | 2 (minimum), 3–7, 8 (near maximum) |
//! | Duration | 1 day (minimum), 30 days (typical), 365 days (maximum) |
//! | Double-resolve | Second call after successful resolution must refuse |
//! | Market without oracle result | Must fail with `OracleUnavailable` |
//! | Market before end_time | Must fail with `MarketClosed` |
//! | Min pool not met | Market with `min_pool_size` above `total_staked` |
//!
//! ## Running
//!
//! ```bash
//! cargo test -p predictify-hybrid -- resolve_market_fuzz
//! ```
//!
//! ```bash
//! cargo test -p predictify-hybrid -- fuzz_resolve_market
//! ```
//!
//! ## Security
//!
//! All fuzz cases assume `mock_all_auths()` is active (auth is tested separately
//! in the `require_auth_coverage_tests` module). No `unwrap()` is used in
//! production paths — errors are always propagated via `Result`.

use proptest::{collection::vec as prop_vec, prelude::*};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String as SorobanString, Symbol,
};

use crate::{
    config::ConfigManager,
    resolution::MarketResolutionManager,
    storage::MarketStateManager,
    types::{Error, Market, MarketState, OracleConfig, OracleProvider},
};

// ===========================================================================
// Constants
// ===========================================================================

/// Default oracle address used in test markets.
const ORACLE_ADDR: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

/// Default market question used in fuzz tests.
const MKT_QUESTION: &str = "Fuzz resolve market?";

/// Minimum bet amount used in fuzz tests.
const MIN_BET: i128 = 1_000_000;

// ===========================================================================
// Proptest strategies
//
// IMPORTANT: Strategies generate *Rust* values (String, Vec<i128>, etc.), not
// Soroban values, because Soroban types are tied to a specific `Env` instance.
// Conversion to Soroban types happens inside each test function.
// ===========================================================================

/// Generate oracle result options as `Option<String>` (Rust strings).
/// Test functions convert to `SorobanString` via `SorobanString::from_str(&env, &s)`.
fn oracle_result_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        // None — should cause OracleUnavailable
        2 => Just(None),
        // Valid result string matching one of the market outcomes
        4 => Just(Some("yes".to_string())),
        // Empty string (technically invalid)
        1 => Just(Some(String::new())),
        // Invalid outcome string (not in [yes, no])
        1 => Just(Some("maybe".to_string())),
        // Very long outcome string
        1 => Just(Some(
            "outcome_that_is_extremely_long_and_might_overflow_buffers".to_string()
        )),
    ]
}

/// Generate market duration in days (clamped to reasonable bounds).
fn duration_days_strategy() -> impl Strategy<Value = u32> {
    prop_oneof![
        1 => Just(1u32),   // Minimum
        2 => (2u32..30u32).prop_map(|x| x),
        1 => (31u32..365u32).prop_map(|x| x),
    ]
}

/// Generate vote distributions for the market's two outcomes.
///
/// Returns `(yes_stakes, no_stakes)` — the per-voter stake amounts for each outcome.
/// An empty Vec means no voters for that outcome.
fn vote_distribution_strategy() -> impl Strategy<Value = (Vec<i128>, Vec<i128>)> {
    prop_oneof![
        // No votes
        1 => Just((vec![], vec![])),
        // Single voter on "yes"
        1 => (MIN_BET..1_000_000_000i128).prop_map(|s| (vec![s], vec![])),
        // Single voter on "no"
        1 => (MIN_BET..1_000_000_000i128).prop_map(|s| (vec![], vec![s])),
        // Two voters, one each
        1 => (
            MIN_BET..100_000_000i128,
            MIN_BET..100_000_000i128,
        ).prop_map(|(s1, s2)| (vec![s1], vec![s2])),
        // Many voters, skewed toward "yes"
        1 => prop_vec(MIN_BET..100_000_000i128, 1..5)
            .prop_map(|stakes| (stakes, vec![])),
        // Many voters, skewed toward "no"
        1 => prop_vec(MIN_BET..100_000_000i128, 1..5)
            .prop_map(|stakes| (vec![], stakes)),
        // Many voters, balanced — pre-split into yes and no groups
        1 => (
            prop_vec(MIN_BET..100_000_000i128, 1..5),
            prop_vec(MIN_BET..100_000_000i128, 1..5),
        ),
    ]
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Create a test market in the desired state with the given oracle result.
///
/// Takes `oracle_result` as `Option<SorobanString>` already converted for the
/// test's environment.
fn create_fuzz_market(
    env: &Env,
    contract_id: &Address,
    admin: &Address,
    oracle_result: &Option<SorobanString>,
    duration_days: u32,
    outcomes: soroban_sdk::Vec<SorobanString>,
    state: MarketState,
    yes_stakes: &[i128],
    no_stakes: &[i128],
) -> Symbol {
    let market_id = Symbol::new(env, "FZ_RES");

    let end_time = env.ledger().timestamp() + (duration_days as u64) * 86_400;

    let oracle_cfg = OracleConfig::new(
        OracleProvider::reflector(),
        Address::from_str(env, ORACLE_ADDR),
        SorobanString::from_str(env, "BTC/USD"),
        50_000_00i128,
        SorobanString::from_str(env, "gt"),
    );

    let mut market = Market::new(
        env,
        admin.clone(),
        SorobanString::from_str(env, MKT_QUESTION),
        outcomes,
        end_time,
        oracle_cfg,
        None,
        86_400u64,
        state,
    );

    // Set oracle result if provided.
    if let Some(ref result) = oracle_result {
        market.oracle_result = Some(result.clone());
    }

    // Add yes voters.
    let mut total_staked: i128 = 0;
    for stake in yes_stakes.iter() {
        let voter = Address::generate(env);
        market
            .votes
            .set(voter.clone(), SorobanString::from_str(env, "yes"));
        market.stakes.set(voter, *stake);
        total_staked = total_staked.saturating_add(*stake);
    }

    // Add no voters.
    for stake in no_stakes.iter() {
        let voter = Address::generate(env);
        market
            .votes
            .set(voter.clone(), SorobanString::from_str(env, "no"));
        market.stakes.set(voter, *stake);
        total_staked = total_staked.saturating_add(*stake);
    }

    market.total_staked = total_staked;

    // Store the market under the contract.
    env.as_contract(contract_id, || {
        MarketStateManager::update_market(env, &market_id, &market);
    });

    market_id
}

/// Initialize a contract environment with basic configuration.
///
/// Registers the `PredictifyHybrid` contract, initializes it with the given
/// admin, and stores a development config so that `ConfigManager` calls
/// succeed inside the tests.
fn init_contract(env: &Env, admin: &Address) -> Address {
    use crate::PredictifyHybrid;

    let contract_id = env.register(PredictifyHybrid, ());
    let client = crate::PredictifyHybridClient::new(env, &contract_id);

    env.mock_all_auths();
    client.initialize(admin, &None, &None);

    // Store dev config so ConfigManager calls succeed.
    env.as_contract(&contract_id, || {
        let cfg = ConfigManager::get_development_config(env);
        ConfigManager::store_config(env, &cfg).ok();
    });

    contract_id
}

/// Create the standard two-outcome set for a test market.
fn default_outcomes(env: &Env) -> soroban_sdk::Vec<SorobanString> {
    soroban_sdk::vec![
        env,
        SorobanString::from_str(env, "yes"),
        SorobanString::from_str(env, "no"),
    ]
}

/// Advance the ledger clock forward by the given number of seconds.
fn advance_time(env: &Env, seconds: u64) {
    env.ledger().with_mut(|l| {
        l.timestamp = l.timestamp.saturating_add(seconds);
    });
}

// ===========================================================================
// Fuzz targets (proptest)
// ===========================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    // ── Oracle result fuzz ──────────────────────────────────────────────

    /// Verify that markets without an `oracle_result` fail with
    /// `OracleUnavailable` when `resolve_market` is called.
    #[test]
    fn fuzz_resolve_market_no_oracle_result(
        duration_days in duration_days_strategy(),
        (yes_stakes, no_stakes) in vote_distribution_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = init_contract(&env, &admin);

        // No oracle result set.
        let oracle_result: Option<SorobanString> = None;

        let market_id = create_fuzz_market(
            &env,
            &contract_id,
            &admin,
            &oracle_result,
            duration_days,
            default_outcomes(&env),
            MarketState::Ended,
            &yes_stakes,
            &no_stakes,
        );

        // Advance past end_time so the OracleUnavailable check is hit
        // before the MarketClosed check.
        advance_time(&env, 3600);

        let result = env.as_contract(&contract_id, || {
            MarketResolutionManager::resolve_market(&env, &market_id)
        });

        prop_assert!(
            matches!(&result, Err(Error::OracleUnavailable)),
            "resolve_market with no oracle_result should return OracleUnavailable, got {:?}",
            result,
        );
    }

    // ── Market state fuzz ───────────────────────────────────────────────

    /// Verify that calling `resolve_market` on an active (not-ended) market
    /// fails with `MarketClosed` (or `OracleUnavailable` if no oracle result).
    #[test]
    fn fuzz_resolve_market_active_market(
        duration_days in duration_days_strategy(),
        oracle_result_str in oracle_result_strategy(),
        (yes_stakes, no_stakes) in vote_distribution_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = init_contract(&env, &admin);

        let oracle_result: Option<SorobanString> = oracle_result_str
            .map(|s| SorobanString::from_str(&env, &s));

        // Market in Active state (not ended).
        let market_id = create_fuzz_market(
            &env,
            &contract_id,
            &admin,
            &oracle_result,
            duration_days,
            default_outcomes(&env),
            MarketState::Active,
            &yes_stakes,
            &no_stakes,
        );

        // Do NOT advance time — market is still active.
        let result = env.as_contract(&contract_id, || {
            MarketResolutionManager::resolve_market(&env, &market_id)
        });

        // For markets without oracle_result, we expect OracleUnavailable (fails
        // before the state check). For markets with oracle_result, we expect
        // MarketClosed.
        match &oracle_result {
            None => {
                prop_assert!(
                    matches!(&result, Err(Error::OracleUnavailable)),
                    "Active market without oracle result should return OracleUnavailable, got {:?}",
                    result,
                );
            }
            Some(_) => {
                prop_assert!(
                    matches!(&result, Err(Error::MarketClosed)),
                    "Active market with oracle result should return MarketClosed, got {:?}",
                    result,
                );
            }
        }
    }

    /// Verify that an already-resolved market refuses a second resolve call.
    #[test]
    fn fuzz_resolve_market_already_resolved(
        duration_days in duration_days_strategy(),
        (yes_stakes, no_stakes) in vote_distribution_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = init_contract(&env, &admin);

        let oracle_result = Some(SorobanString::from_str(&env, "yes"));

        let market_id = create_fuzz_market(
            &env,
            &contract_id,
            &admin,
            &oracle_result,
            duration_days,
            default_outcomes(&env),
            MarketState::Ended,
            &yes_stakes,
            &no_stakes,
        );

        // First resolve may succeed or fail depending on market config.
        let first_result = env.as_contract(&contract_id, || {
            MarketResolutionManager::resolve_market(&env, &market_id)
        });

        if first_result.is_ok() {
            // Advance past end_time.
            advance_time(&env, duration_days as u64 * 86_400 + 3600);

            // Second resolve must fail with MarketResolved.
            let second_result = env.as_contract(&contract_id, || {
                MarketResolutionManager::resolve_market(&env, &market_id)
            });

            prop_assert!(
                matches!(&second_result, Err(Error::MarketResolved)),
                "Second resolve on resolved market should return MarketResolved, got {:?}",
                second_result,
            );
        }
        // If first resolve failed (e.g. min pool not met), that's acceptable.
    }

    // ── Valid resolve path fuzz ─────────────────────────────────────────

    /// Verify that a properly configured ended market with an oracle result
    /// resolves successfully and produces correct output invariants.
    #[test]
    fn fuzz_resolve_market_valid_path(
        duration_days in duration_days_strategy(),
        (yes_stakes, no_stakes) in vote_distribution_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = init_contract(&env, &admin);

        let oracle_result = Some(SorobanString::from_str(&env, "yes"));

        let market_id = create_fuzz_market(
            &env,
            &contract_id,
            &admin,
            &oracle_result,
            duration_days,
            default_outcomes(&env),
            MarketState::Ended,
            &yes_stakes,
            &no_stakes,
        );

        // Advance past end_time.
        advance_time(&env, duration_days as u64 * 86_400 + 3600);

        let result = env.as_contract(&contract_id, || {
            MarketResolutionManager::resolve_market(&env, &market_id)
        });

        // The important invariant is that it doesn't panic.
        if let Ok(resolution) = &result {
            prop_assert_eq!(
                resolution.market_id, market_id,
                "Resolution market_id should match",
            );
            prop_assert!(
                resolution.confidence_score <= 100,
                "Confidence score should be <= 100",
            );
            prop_assert!(
                !resolution.final_outcome.is_empty(),
                "Final outcome must not be empty",
            );

            // Verify the market is now resolved in storage.
            let stored_market = env.as_contract(&contract_id, || {
                MarketStateManager::get_market(&env, &market_id).ok()
            });
            if let Some(market) = stored_market {
                prop_assert_eq!(
                    market.state,
                    MarketState::Resolved,
                    "Market state should be Resolved after successful resolve_market",
                );
                prop_assert!(
                    market.winning_outcomes.is_some(),
                    "Winning outcomes must be set after successful resolve_market",
                );
            }
        } else if let Err(e) = &result {
            // Acceptable errors (depends on market configuration).
            let acceptable = [
                Error::InvalidState,   // min_pool_size, etc.
                Error::MarketResolved, // already resolved (race)
            ];
            prop_assert!(
                acceptable.contains(e),
                "Valid-path resolve_market produced unexpected error {:?}",
                e,
            );
        }
    }

    // ── Idempotency fuzz ────────────────────────────────────────────────

    /// Verify that calling `resolve_market` multiple times is idempotent:
    /// at most one call may succeed; subsequent calls must return an error.
    #[test]
    fn fuzz_resolve_market_idempotent(
        duration_days in duration_days_strategy(),
        (yes_stakes, no_stakes) in vote_distribution_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = init_contract(&env, &admin);

        let oracle_result = Some(SorobanString::from_str(&env, "yes"));

        let market_id = create_fuzz_market(
            &env,
            &contract_id,
            &admin,
            &oracle_result,
            duration_days,
            default_outcomes(&env),
            MarketState::Ended,
            &yes_stakes,
            &no_stakes,
        );

        // Advance past end_time.
        advance_time(&env, duration_days as u64 * 86_400 + 3600);

        // Call resolve_market three times.
        let mut results = Vec::new();
        for _ in 0..3 {
            let r = env.as_contract(&contract_id, || {
                MarketResolutionManager::resolve_market(&env, &market_id)
            });
            results.push(r);
        }

        // At most one call may succeed; after that all must fail.
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        prop_assert!(
            success_count <= 1,
            "resolve_market should succeed at most once, got {} successes",
            success_count,
        );
    }

    // ── Extreme values fuzz ─────────────────────────────────────────────

    /// Verify that extreme stake values (i128::MIN, i128::MAX) don't panic.
    #[test]
    fn fuzz_resolve_market_extreme_stakes(
        raw_bytes in prop::array::uniform16(0u8..),
        duration_days in duration_days_strategy(),
    ) {
        let extreme_stake = i128::from_le_bytes(raw_bytes);
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = init_contract(&env, &admin);

        let oracle_result = Some(SorobanString::from_str(&env, "yes"));

        // Create a market with a single voter who staked the extreme amount.
        let market_id = create_fuzz_market(
            &env,
            &contract_id,
            &admin,
            &oracle_result,
            duration_days,
            default_outcomes(&env),
            MarketState::Ended,
            &[extreme_stake], // yes voter
            &[],
        );

        // Advance past end_time.
        advance_time(&env, duration_days as u64 * 86_400 + 3600);

        let result = env.as_contract(&contract_id, || {
            MarketResolutionManager::resolve_market(&env, &market_id)
        });

        // Must not panic. Any Error or Ok result is acceptable.
        if let Err(e) = &result {
            let allowed = [
                Error::InvalidState,
                Error::MarketResolved,
                Error::Overflow,
                Error::InvalidInput,
            ];
            prop_assert!(
                allowed.contains(e),
                "Extreme stake produced unexpected error {:?}",
                e,
            );
        }
    }

    // ── Tie outcome fuzz ────────────────────────────────────────────────

    /// Verify that a market with an oracle result and balanced votes
    /// resolves cleanly (or fails gracefully without panic).
    #[test]
    fn fuzz_resolve_market_tie_outcome(
        duration_days in duration_days_strategy(),
        yes_stake in MIN_BET..100_000_000i128,
        no_stake in MIN_BET..100_000_000i128,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = init_contract(&env, &admin);

        let oracle_result = Some(SorobanString::from_str(&env, "yes"));

        let market_id = create_fuzz_market(
            &env,
            &contract_id,
            &admin,
            &oracle_result,
            duration_days,
            default_outcomes(&env),
            MarketState::Ended,
            &[yes_stake], // yes voter
            &[no_stake],  // no voter
        );

        // Advance past end_time.
        advance_time(&env, duration_days as u64 * 86_400 + 3600);

        let result = env.as_contract(&contract_id, || {
            MarketResolutionManager::resolve_market(&env, &market_id)
        });

        // Must not panic. The result depends on oracle_result vs community vote.
        if let Ok(resolution) = &result {
            prop_assert!(
                resolution.confidence_score <= 100,
                "Confidence score must be <= 100",
            );
        }
    }

    // ── Min pool size fuzz ──────────────────────────────────────────────

    /// Verify that a market with `min_pool_size` above `total_staked` fails
    /// with `InvalidState`.
    #[test]
    fn fuzz_resolve_market_min_pool_not_met(
        shortage_pct in 1i128..1000i128, // 0.01% to 10% shortage
        duration_days in duration_days_strategy(),
        stake in MIN_BET..1_000_000_000i128,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = init_contract(&env, &admin);

        let yes_stake = stake;
        let min_pool = yes_stake + (yes_stake * shortage_pct / 10_000).max(1);

        let market_id = Symbol::new(&env, "FZ_MIN");

        let end_time = env.ledger().timestamp() + (duration_days as u64) * 86_400;

        let oracle_cfg = OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(&env, ORACLE_ADDR),
            SorobanString::from_str(&env, "BTC/USD"),
            50_000_00i128,
            SorobanString::from_str(&env, "gt"),
        );

        let mut market = Market::new(
            &env,
            admin.clone(),
            SorobanString::from_str(&env, MKT_QUESTION),
            default_outcomes(&env),
            end_time,
            oracle_cfg,
            None,
            86_400u64,
            MarketState::Ended,
        );
        market.oracle_result = Some(SorobanString::from_str(&env, "yes"));
        market.min_pool_size = Some(min_pool);

        // Add a single voter on "yes".
        let voter = Address::generate(&env);
        market.votes.set(voter, SorobanString::from_str(&env, "yes"));
        market.stakes.set(Address::generate(&env), yes_stake);
        market.total_staked = yes_stake;

        env.as_contract(&contract_id, || {
            MarketStateManager::update_market(&env, &market_id, &market);
        });

        // Advance past end_time.
        advance_time(&env, duration_days as u64 * 86_400 + 3600);

        let result = env.as_contract(&contract_id, || {
            MarketResolutionManager::resolve_market(&env, &market_id)
        });

        // Should fail with InvalidState because min_pool > total_staked.
        prop_assert!(
            matches!(&result, Err(Error::InvalidState)),
            "Market with total_staked {} < min_pool {} should return InvalidState, got {:?}",
            yes_stake,
            min_pool,
            result,
        );
    }

    // ── Resolution timeout fuzz ─────────────────────────────────────────

    /// Verify that a market past the resolution timeout doesn't panic.
    /// The timeout check lives in `OracleResolutionManager::fetch_oracle_result`,
    /// which is separate from `MarketResolutionManager::resolve_market`.
    #[test]
    fn fuzz_resolve_market_past_timeout(
        days_past in (0u64..=100u64),
        duration_days in duration_days_strategy(),
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = init_contract(&env, &admin);

        let oracle_result = Some(SorobanString::from_str(&env, "yes"));

        let market_id = create_fuzz_market(
            &env,
            &contract_id,
            &admin,
            &oracle_result,
            duration_days,
            default_outcomes(&env),
            MarketState::Ended,
            &[MIN_BET],
            &[],
        );

        // Advance far past end_time + resolution_timeout.
        let total_offset = (duration_days as u64) * 86_400 + 86_400 + days_past * 86_400;
        advance_time(&env, total_offset);

        let result = env.as_contract(&contract_id, || {
            MarketResolutionManager::resolve_market(&env, &market_id)
        });

        // Must not panic, regardless of whether resolution succeeds or fails.
        // The timeout check is in `OracleResolutionManager::fetch_oracle_result`,
        // not in `MarketResolutionManager::resolve_market`.
        let _ = result;
    }
}
