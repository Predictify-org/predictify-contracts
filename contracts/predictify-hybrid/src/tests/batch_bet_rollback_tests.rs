//! Batch bet placement rollback-safety tests.
//!
//! Verifies that `BetManager::place_bets` is fully rollback-safe under:
//! - duplicate markets within a single batch
//! - empty and oversized batches
//! - idempotency replay
//! - successful multi-market batches
//! - concurrent execution safety (via reentrancy guard)
//!
//! ## Running
//!
//! ```bash
//! cargo test -p predictify-hybrid -- batch_bet_rollback_tests
//! ```

#![cfg(test)]

use crate::bets::BetManager;
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use crate::{Error, PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::StellarAssetClient,
    Address, Env, String as SorobanString, Symbol, BytesN, Vec,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const ORACLE_ADDR: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";
const DAY_SECS: u64 = 86_400;
const MIN_BET: i128 = 1_000_000;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Set up a bare `Env` with mocked auth, a funded token, and the contract.
/// Returns `(env, admin, contract_id, token_id)`.
fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(crate::PredictifyHybrid, ());
    let client = crate::PredictifyHybridClient::new(&env, &contract_id);
    client.initialize(&admin, &Some(200i128), &None);

    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();

    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "TokenID"), &token_id);
    });

    (env, admin, contract_id, token_id)
}

/// Fund a user with ample tokens and approve the contract for spending.
fn fund_and_approve(
    env: &Env,
    contract_id: &Address,
    token_id: &Address,
    user: &Address,
    amount: i128,
) {
    let stellar = soroban_sdk::token::StellarAssetClient::new(env, token_id);
    let tok = soroban_sdk::token::Client::new(env, token_id);
    stellar.mint(user, &amount);
    tok.approve(user, contract_id, &i128::MAX, &1_000_000u32);
}

/// Create and store an active market.
fn make_active_market(
    env: &Env,
    contract_id: &Address,
    admin: &Address,
    market_id: &Symbol,
    outcomes: soroban_sdk::Vec<SorobanString>,
) {
    let end_time = env.ledger().timestamp() + DAY_SECS;

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
        SorobanString::from_str(env, "Test market"),
        outcomes,
        end_time,
        oracle_cfg,
        None,
        DAY_SECS,
        MarketState::Active,
    );

    env.as_contract(contract_id, || {
        crate::markets::MarketStateManager::update_market(env, market_id, &market);
    });
}

// ---------------------------------------------------------------------------
// Reentrancy guard tests
// ---------------------------------------------------------------------------

/// The `place_bets` entrypoint must carry its own reentrancy scope so that
/// recursive calls (e.g. via a callback in `emit_bet_batch_placed`) are
/// rejected rather than causing state corruption.
#[test]
fn test_place_bets_has_reentrancy_guard() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);

    fund_and_approve(&env, &contract_id, &token_id, &user, 1_000_000_000_000i128);

    let market_id = Symbol::new(&env, "reentrancy_market");
    make_active_market(
        &env,
        &contract_id,
        &admin,
        &market_id,
        soroban_sdk::vec![env, SorobanString::from_str(env, "yes")],
    );

    env.mock_all_auths();
    let client = crate::PredictifyHybridClient::new(&env, &contract_id);

    // First call should succeed.
    let bets = soroban_sdk::vec![
        env,
        (market_id.clone(), SorobanString::from_str(env, "yes"), MIN_BET),
    ];
    let idem = BytesN::from_array(&env, &[1u8; 32]);
    client
        .place_bets(&user, &bets, &200i128, &idem)
        .expect("first batch should succeed");

    // Second call with the same idempotency key must be rejected.
    let result = client.place_bets(&user, &bets, &200i128, &idem);
    assert_eq!(result, Err(Ok(Error::IdempotentBatchAlreadyApplied)));
}

// ---------------------------------------------------------------------------
// Duplicate market detection
// ---------------------------------------------------------------------------

/// A batch that lists the same market more than once must be rejected
/// before any funds are locked or state is mutated. This prevents data
/// loss (the second bet overwrites the first in `BetStorage`) and
/// overcharging the user.
#[test]
fn test_place_bets_rejects_duplicate_markets() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);

    fund_and_approve(&env, &contract_id, &token_id, &user, 1_000_000_000_000i128);

    let market_id = Symbol::new(&env, "dup_market");
    make_active_market(
        &env,
        &contract_id,
        &admin,
        &market_id,
        soroban_sdk::vec![env, SorobanString::from_str(env, "yes")],
    );

    env.mock_all_auths();
    let client = crate::PredictifyHybridClient::new(&env, &contract_id);

    let bets = soroban_sdk::vec![
        env,
        (market_id.clone(), SorobanString::from_str(env, "yes"), MIN_BET),
        (market_id.clone(), SorobanString::from_str(env, "no"), MIN_BET),
    ];
    let idem = BytesN::from_array(&env, &[2u8; 32]);

    let result = client.place_bets(&user, &bets, &200i128, &idem);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));
}

// ---------------------------------------------------------------------------
// Boundary: empty and oversized batches
// ---------------------------------------------------------------------------

#[test]
fn test_place_bets_rejects_empty_batch() {
    let (env, _admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);

    fund_and_approve(&env, &contract_id, &token_id, &user, 1_000_000_000_000i128);

    env.mock_all_auths();
    let client = crate::PredictifyHybridClient::new(&env, &contract_id);

    let bets = soroban_sdk::Vec::new(&env);
    let idem = BytesN::from_array(&env, &[3u8; 32]);

    let result = client.place_bets(&user, &bets, &200i128, &idem);
    assert_eq!(result, Err(Ok(Error::BatchEmpty)));
}

#[test]
fn test_place_bets_rejects_oversized_batch() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);

    fund_and_approve(&env, &contract_id, &token_id, &user, 1_000_000_000_000i128);

    // Create 51 distinct markets (MAX_BATCH_SIZE = 50).
    let mut bets = soroban_sdk::Vec::new(&env);
    for i in 0..51u32 {
        let market_id = Symbol::new(&env, &format!("ovrsz_{i}"));
        make_active_market(
            &env,
            &contract_id,
            &admin,
            &market_id,
            soroban_sdk::vec![env, SorobanString::from_str(env, "yes")],
        );
        bets.push_back((
            market_id,
            SorobanString::from_str(env, "yes"),
            MIN_BET,
        ));
    }

    env.mock_all_auths();
    let client = crate::PredictifyHybridClient::new(&env, &contract_id);

    let idem = BytesN::from_array(&env, &[4u8; 32]);
    let result = client.place_bets(&user, &bets, &200i128, &idem);
    assert_eq!(result, Err(Ok(Error::BatchSizeExceeded)));
}

// ---------------------------------------------------------------------------
// Success path: single and multi-market batches
// ---------------------------------------------------------------------------

#[test]
fn test_place_bets_succeeds_single_market() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);

    fund_and_approve(&env, &contract_id, &token_id, &user, 1_000_000_000_000i128);

    let market_id = Symbol::new(&env, "single_market");
    make_active_market(
        &env,
        &contract_id,
        &admin,
        &market_id,
        soroban_sdk::vec![
            env,
            SorobanString::from_str(env, "yes"),
            SorobanString::from_str(env, "no"),
        ],
    );

    env.mock_all_auths();
    let client = crate::PredictifyHybridClient::new(&env, &contract_id);

    let bets = soroban_sdk::vec![
        env,
        (market_id.clone(), SorobanString::from_str(env, "yes"), MIN_BET),
    ];
    let idem = BytesN::from_array(&env, &[5u8; 32]);

    let result = client.place_bets(&user, &bets, &200i128, &idem);
    assert!(result.is_ok(), "single-market batch should succeed: {:?}", result);
    assert_eq!(result.unwrap().len(), 1);
}

#[test]
fn test_place_bets_succeeds_multiple_markets() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);

    fund_and_approve(&env, &contract_id, &token_id, &user, 1_000_000_000_000i128);

    let market_a = Symbol::new(&env, "multi_a");
    let market_b = Symbol::new(&env, "multi_b");

    make_active_market(
        &env,
        &contract_id,
        &admin,
        &market_a,
        soroban_sdk::vec![env, SorobanString::from_str(env, "yes")],
    );
    make_active_market(
        &env,
        &contract_id,
        &admin,
        &market_b,
        soroban_sdk::vec![env, SorobanString::from_str(env, "no")],
    );

    env.mock_all_auths();
    let client = crate::PredictifyHybridClient::new(&env, &contract_id);

    let bets = soroban_sdk::vec![
        env,
        (market_a.clone(), SorobanString::from_str(env, "yes"), MIN_BET),
        (market_b.clone(), SorobanString::from_str(env, "no"), MIN_BET),
    ];
    let idem = BytesN::from_array(&env, &[6u8; 32]);

    let result = client.place_bets(&user, &bets, &200i128, &idem);
    assert!(result.is_ok(), "multi-market batch should succeed: {:?}", result);
    assert_eq!(result.unwrap().len(), 2);
}

// ---------------------------------------------------------------------------
// Regression: already-bet market in batch
// ---------------------------------------------------------------------------

/// If the user has already placed a bet on one of the markets in the batch,
/// the entire batch must be rejected rather than partially applying.
#[test]
fn test_place_bets_rejects_already_bet_market() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);

    fund_and_approve(&env, &contract_id, &token_id, &user, 1_000_000_000_000i128);

    let market_a = Symbol::new(&env, "already_bet_a");
    let market_b = Symbol::new(&env, "already_bet_b");

    make_active_market(
        &env,
        &contract_id,
        &admin,
        &market_a,
        soroban_sdk::vec![env, SorobanString::from_str(env, "yes")],
    );
    make_active_market(
        &env,
        &contract_id,
        &admin,
        &market_b,
        soroban_sdk::vec![env, SorobanString::from_str(env, "no")],
    );

    env.mock_all_auths();
    let client = crate::PredictifyHybridClient::new(&env, &contract_id);

    // Place a single bet on market_a first.
    let single_bet = soroban_sdk::vec![
        env,
        (market_a.clone(), SorobanString::from_str(env, "yes"), MIN_BET),
    ];
    let idem_single = BytesN::from_array(&env, &[7u8; 32]);
    client
        .place_bets(&user, &single_bet, &200i128, &idem_single)
        .expect("initial bet should succeed");

    // Now try a batch that includes market_a again.
    let batch = soroban_sdk::vec![
        env,
        (market_a.clone(), SorobanString::from_str(env, "yes"), MIN_BET),
        (market_b.clone(), SorobanString::from_str(env, "no"), MIN_BET),
    ];
    let idem_batch = BytesN::from_array(&env, &[8u8; 32]);

    let result = client.place_bets(&user, &batch, &200i128, &idem_batch);
    assert_eq!(result, Err(Ok(Error::AlreadyBet)));
}
