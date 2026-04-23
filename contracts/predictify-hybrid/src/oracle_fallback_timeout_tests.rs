#![cfg(test)]

//! Regression tests for oracle timeout vs dispute window interaction (issue #396).
//!
//! # Invariants under test
//!
//! 1. A market in `Disputed` state is **never** cancelled by the resolution timeout.
//! 2. Oracle resolution is blocked while a dispute is active.
//! 3. `resolution_timeout` must be ≥ `dispute_window_seconds`; a shorter value is
//!    rejected at market-creation validation time.
//! 4. A non-disputed market that exceeds its resolution timeout is cancelled and
//!    returns `ResolutionTimeoutReached`.

use crate::config::DISPUTE_EXTENSION_HOURS;
use crate::errors::Error;
use crate::markets::MarketStateManager;
use crate::resolution::OracleResolutionManager;
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use crate::validation::{MarketValidator, OracleConfigValidator};
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::{Address, Env, String, Symbol, Vec};

// ── helpers ──────────────────────────────────────────────────────────────────

fn make_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

fn advance(env: &Env, secs: u64) {
    let ts = env.ledger().timestamp() + secs;
    env.ledger().set(LedgerInfo {
        timestamp: ts,
        protocol_version: 22,
        sequence_number: env.ledger().sequence() + 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 16,
        max_entry_ttl: 6_312_000,
    });
}

fn oracle_cfg(env: &Env) -> OracleConfig {
    OracleConfig::new(
        OracleProvider::reflector(),
        Address::from_str(
            env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        ),
        String::from_str(env, "BTC/USD"),
        50_000_00,
        String::from_str(env, "gt"),
    )
}

fn make_market(env: &Env, end_time: u64, resolution_timeout: u64) -> (Symbol, Market) {
    let admin = Address::generate(env);
    let market_id = Symbol::new(env, "test_mkt");
    let mut outcomes = Vec::new(env);
    outcomes.push_back(String::from_str(env, "yes"));
    outcomes.push_back(String::from_str(env, "no"));

    let market = Market::new(
        env,
        admin,
        String::from_str(env, "Will BTC reach $50k?"),
        outcomes,
        end_time,
        oracle_cfg(env),
        None,
        resolution_timeout,
        MarketState::Active,
    );
    (market_id, market)
}

// ── invariant 1 & 2: disputed market is immune to oracle timeout ──────────────

/// A market that is `Disputed` must not be cancelled by the resolution timeout,
/// and oracle resolution must be blocked while the dispute is active.
#[test]
fn test_disputed_market_not_cancelled_by_timeout() {
    let env = make_env();
    let resolution_timeout: u64 = 7 * 24 * 3600; // 7 days
    let end_time = env.ledger().timestamp() + 3600;
    let (market_id, mut market) = make_market(&env, end_time, resolution_timeout);

    // Transition to Disputed state
    market.state = MarketState::Disputed;
    env.storage().persistent().set(&market_id, &market);

    // Advance well past end_time + resolution_timeout
    advance(&env, end_time + resolution_timeout + 1);

    // fetch_oracle_result must return AlreadyDisputed, not cancel the market
    let result = OracleResolutionManager::fetch_oracle_result(&env, &market_id);
    assert_eq!(result, Err(Error::AlreadyDisputed));

    // Market state must still be Disputed, not Cancelled
    let stored: Market = env.storage().persistent().get(&market_id).unwrap();
    assert_eq!(stored.state, MarketState::Disputed);
}

/// Oracle resolution is blocked the moment a market enters `Disputed` state,
/// even if the resolution timeout has not yet elapsed.
#[test]
fn test_oracle_resolution_blocked_during_active_dispute() {
    let env = make_env();
    let resolution_timeout: u64 = 7 * 24 * 3600;
    let end_time = env.ledger().timestamp() + 3600;
    let (market_id, mut market) = make_market(&env, end_time, resolution_timeout);

    market.state = MarketState::Disputed;
    env.storage().persistent().set(&market_id, &market);

    // Still within resolution_timeout window
    advance(&env, 3700); // just past end_time, well before timeout

    let result = OracleResolutionManager::fetch_oracle_result(&env, &market_id);
    assert_eq!(result, Err(Error::AlreadyDisputed));
}

// ── invariant 3: resolution_timeout must be >= dispute_window_seconds ─────────

/// `resolution_timeout` shorter than the dispute window must be rejected.
/// Default dispute window = DISPUTE_EXTENSION_HOURS * 3600.
#[test]
fn test_resolution_timeout_shorter_than_dispute_window_is_invalid() {
    let dispute_window_secs = DISPUTE_EXTENSION_HOURS as u64 * 3600; // 86 400 s
    let too_short = dispute_window_secs - 1;

    // validate_resolution_timeout alone accepts values ≥ 3600, but the
    // cross-field check in MarketValidator rejects values < dispute_window_secs.
    // We test the standalone validator first (should pass its own range check)…
    assert!(OracleConfigValidator::validate_resolution_timeout(&too_short).is_ok());

    // …then the cross-field check via MarketValidator (should fail).
    let env = make_env();
    let admin = Address::generate(&env);
    let mut outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "yes"));
    outcomes.push_back(String::from_str(&env, "no"));

    let result = MarketValidator::validate_market_creation(
        &env,
        &admin,
        &String::from_str(&env, "Will BTC reach $50k by end of year?"),
        &outcomes,
        &30u32,
        &oracle_cfg(&env),
        false,
        &oracle_cfg(&env),
        &too_short,
    );
    assert!(!result.is_valid, "resolution_timeout < dispute_window must be invalid");
}

/// `resolution_timeout` equal to the dispute window is accepted.
#[test]
fn test_resolution_timeout_equal_to_dispute_window_is_valid() {
    let dispute_window_secs = DISPUTE_EXTENSION_HOURS as u64 * 3600;

    let env = make_env();
    let admin = Address::generate(&env);
    let mut outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "yes"));
    outcomes.push_back(String::from_str(&env, "no"));

    let result = MarketValidator::validate_market_creation(
        &env,
        &admin,
        &String::from_str(&env, "Will BTC reach $50k by end of year?"),
        &outcomes,
        &30u32,
        &oracle_cfg(&env),
        false,
        &oracle_cfg(&env),
        &dispute_window_secs,
    );
    assert!(result.is_valid, "resolution_timeout == dispute_window must be valid");
}

// ── invariant 4: non-disputed market past timeout is cancelled ────────────────

/// A market that is NOT disputed and exceeds its resolution timeout must be
/// cancelled and return `ResolutionTimeoutReached`.
#[test]
fn test_non_disputed_market_cancelled_after_timeout() {
    let env = make_env();
    let resolution_timeout: u64 = DISPUTE_EXTENSION_HOURS as u64 * 3600; // minimum valid
    let end_time = env.ledger().timestamp() + 3600;
    let (market_id, mut market) = make_market(&env, end_time, resolution_timeout);

    market.state = MarketState::Ended;
    env.storage().persistent().set(&market_id, &market);

    // Advance past end_time + resolution_timeout
    advance(&env, end_time + resolution_timeout + 1);

    let result = OracleResolutionManager::fetch_oracle_result(&env, &market_id);
    assert_eq!(result, Err(Error::ResolutionTimeoutReached));

    // Market must now be Cancelled
    let stored: Market = env.storage().persistent().get(&market_id).unwrap();
    assert_eq!(stored.state, MarketState::Cancelled);
}

/// A market that is NOT disputed and is still within its resolution timeout
/// must NOT be cancelled (it should proceed to oracle fetch, which may fail
/// for other reasons, but not due to timeout).
#[test]
fn test_non_disputed_market_within_timeout_not_cancelled() {
    let env = make_env();
    let resolution_timeout: u64 = DISPUTE_EXTENSION_HOURS as u64 * 3600;
    let end_time = env.ledger().timestamp() + 3600;
    let (market_id, mut market) = make_market(&env, end_time, resolution_timeout);

    market.state = MarketState::Ended;
    env.storage().persistent().set(&market_id, &market);

    // Advance just past end_time but well within resolution_timeout
    advance(&env, 3700);

    // Oracle fetch will fail (no real oracle in test), but NOT with ResolutionTimeoutReached
    let result = OracleResolutionManager::fetch_oracle_result(&env, &market_id);
    assert_ne!(result, Err(Error::ResolutionTimeoutReached));

    // Market must NOT have been cancelled
    let stored: Market = env.storage().persistent().get(&market_id).unwrap();
    assert_ne!(stored.state, MarketState::Cancelled);
}
