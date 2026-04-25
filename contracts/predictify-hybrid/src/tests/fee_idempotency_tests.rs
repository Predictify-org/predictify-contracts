#![cfg(test)]

//! Fee idempotency tests for `collect_fees`.
//!
//! Invariants proven:
//! - A resolved market with sufficient stake can have fees collected exactly once.
//! - A second call on the same market is a no-op (`Ok(0)`), not an error.
//! - The `fee_collected` flag is set to `true` after the first successful collection.
//! - Markets that are unresolved or below threshold are correctly rejected.
//!
//! Non-goals:
//! - Token transfer correctness (covered by integration tests).
//! - Fee amount calculation accuracy (covered by fees.rs unit tests).

use crate::fees::{FeeManager, FeeUtils, FeeValidator};
use crate::markets::MarketStateManager;
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String, Symbol};

const SUFFICIENT_STAKE: i128 = 100_000_000; // 10 XLM

fn make_env() -> Env {
    Env::default()
}

fn register_contract(env: &Env) -> Address {
    env.register(crate::PredictifyHybrid, ())
}

fn make_resolved_market(env: &Env) -> Market {
    let mut m = Market::new(
        env,
        Address::generate(env),
        String::from_str(env, "Will BTC exceed $100k?"),
        vec![
            env,
            String::from_str(env, "yes"),
            String::from_str(env, "no"),
        ],
        env.ledger().timestamp() + 86_400,
        OracleConfig::new(
            OracleProvider::pyth(),
            Address::from_str(
                env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            String::from_str(env, "BTC/USD"),
            2_500_000,
            String::from_str(env, "gt"),
        ),
        None,
        86_400,
        MarketState::Active,
    );
    let mut outcomes = soroban_sdk::Vec::new(env);
    outcomes.push_back(String::from_str(env, "yes"));
    m.winning_outcomes = Some(outcomes);
    m.total_staked = SUFFICIENT_STAKE;
    m.fee_collected = false;
    m
}

fn set_admin(env: &Env, admin: &Address) {
    env.storage()
        .persistent()
        .set(&Symbol::new(env, "Admin"), admin);
}

// ── tests ────────────────────────────────────────────────────────────────────

/// First call on a resolved, sufficiently-staked market succeeds and returns
/// a positive fee amount.
#[test]
fn test_first_collection_succeeds() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);
        let market_id = Symbol::new(&env, "mkt1");
        MarketStateManager::update_market(&env, &market_id, &make_resolved_market(&env));

        let result = FeeManager::collect_fees(&env, admin.clone(), market_id.clone());
        assert!(result.is_ok(), "first collect_fees should succeed");
        let stored = MarketStateManager::get_market(&env, &market_id).unwrap();
        assert!(stored.fee_collected, "fee_collected must be true after collection");
    });
}

/// Second call on the same market returns `Ok(0)` — no error, no double charge.
#[test]
fn test_second_collection_is_noop() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);
        let market_id = Symbol::new(&env, "mkt2");
        MarketStateManager::update_market(&env, &market_id, &make_resolved_market(&env));

        FeeManager::collect_fees(&env, admin.clone(), market_id.clone()).unwrap();

        let retry = FeeManager::collect_fees(&env, admin.clone(), market_id.clone());
        assert!(retry.is_ok(), "retry must not error: {:?}", retry);
        assert_eq!(retry.unwrap(), 0, "retry must return 0 (no double charge)");
    });
}

/// Multiple retries all return `Ok(0)` and never alter stored market state.
#[test]
fn test_repeated_retries_are_stable() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);
        let market_id = Symbol::new(&env, "mkt3");
        MarketStateManager::update_market(&env, &market_id, &make_resolved_market(&env));

        FeeManager::collect_fees(&env, admin.clone(), market_id.clone()).unwrap();

        for _ in 0..5 {
            let r = FeeManager::collect_fees(&env, admin.clone(), market_id.clone());
            assert_eq!(r.unwrap(), 0, "each retry must return 0");
        }

        let stored = MarketStateManager::get_market(&env, &market_id).unwrap();
        assert!(stored.fee_collected, "flag must remain true after retries");
    });
}

/// An unresolved market is rejected — idempotency guard must not bypass this.
#[test]
fn test_unresolved_market_rejected() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);
        let market_id = Symbol::new(&env, "mkt4");
        let mut market = make_resolved_market(&env);
        market.winning_outcomes = None;
        MarketStateManager::update_market(&env, &market_id, &market);

        let result = FeeManager::collect_fees(&env, admin.clone(), market_id.clone());
        assert!(result.is_err(), "unresolved market must be rejected");
    });
}

/// A market below the stake threshold is rejected.
#[test]
fn test_below_threshold_rejected() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);
        let market_id = Symbol::new(&env, "mkt5");
        let mut market = make_resolved_market(&env);
        market.total_staked = 0;
        MarketStateManager::update_market(&env, &market_id, &market);

        let result = FeeManager::collect_fees(&env, admin.clone(), market_id.clone());
        assert!(result.is_err(), "below-threshold market must be rejected");
    });
}

/// `FeeValidator::validate_market_for_fee_collection` returns
/// `Error::FeeAlreadyCollected` (not `InvalidFeeConfig`) when fees are done.
#[test]
fn test_validator_returns_correct_error_for_already_collected() {
    let env = make_env();
    let mut market = make_resolved_market(&env);
    market.fee_collected = true;

    let err = FeeValidator::validate_market_for_fee_collection(&market).unwrap_err();
    assert_eq!(
        err,
        crate::err::Error::FeeAlreadyCollected,
        "validator must return FeeAlreadyCollected, got {:?}",
        err
    );
}

/// `FeeUtils::can_collect_fees` returns false when fee_collected is true.
#[test]
fn test_can_collect_fees_false_after_collection() {
    let env = make_env();
    let mut market = make_resolved_market(&env);

    assert!(FeeUtils::can_collect_fees(&market), "should be collectable before");
    market.fee_collected = true;
    assert!(!FeeUtils::can_collect_fees(&market), "should not be collectable after");
}
