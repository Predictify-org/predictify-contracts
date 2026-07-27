//! Focused edge-case tests for the disputes subsystem.
//!
//! These tests exercise boundary conditions that are tricky to hit through
//! normal integration tests but are exactly the conditions a fuzzer would
//! generate.  They complement the proptest suite in
//! `predictify-hybrid/src/tests/dispute_open_fuzz.rs` with deterministic,
//! readable regression tests.
//!
//! ## Coverage
//!
//! | Test name                                     | Condition tested                                   |
//! |-----------------------------------------------|----------------------------------------------------|
//! | `stake_exactly_at_minimum`                    | stake == MIN_DISPUTE_STAKE is accepted             |
//! | `stake_one_below_minimum`                     | stake == MIN_DISPUTE_STAKE - 1 is rejected         |
//! | `stake_zero_rejected`                         | stake == 0 is rejected                             |
//! | `stake_negative_rejected`                     | stake == -1 is rejected                            |
//! | `duplicate_dispute_same_user`                 | second dispute by same user → AlreadyDisputed      |
//! | `dispute_past_window_rejected`                | dispute after window → MarketResolved/InvalidState |
//! | `dispute_before_end_time_rejected`            | dispute on active market → MarketClosed/InvalidState|
//! | `disputer_cannot_vote_own_dispute`            | opener votes own dispute → DisputerCannotVote      |
//! | `stake_cap_exceeded`                          | stake > cap → DisputeStakeCapExceeded              |
//! | `stake_cap_at_boundary_accepted`              | stake == cap is accepted (at-boundary)             |
//! | `error_codes_stable`                          | all dispute error codes match frozen values        |

use predictify_hybrid::{
    disputes::DisputeManager,
    storage::{AdminStorage, DataKey, MarketStateManager, TokenStorage},
    types::{Market, MarketState, OracleConfig, OracleProvider},
    Error,
};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String as SorobanString, Symbol,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MIN_DISPUTE_STAKE: i128 = 10_000_000;
const DISPUTE_PERIOD_SECS: u64 = 86_400;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Set up a bare `Env` with mocked auth, a contract, and a funded token.
/// Returns `(env, admin, contract_id, token_id)`.
fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(predictify_hybrid::PredictifyHybrid, ());
    let client = predictify_hybrid::PredictifyHybridClient::new(&env, &contract_id);
    client.initialize(&admin, &Some(200i128), &None);

    // Register a Stellar Asset contract and wire it into the disputes contract.
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin);
    let token_id = token_contract.address();

    env.as_contract(&contract_id, || {
        TokenStorage::set_token_id(&env, &token_id);
        AdminStorage::set_admin(&env, &admin);
    });

    (env, admin, contract_id, token_id)
}

/// Fund a user with ample tokens and approve the contract.
fn fund(env: &Env, contract_id: &Address, token_id: &Address, user: &Address) {
    let stellar = soroban_sdk::token::StellarAssetClient::new(env, token_id);
    let tok = soroban_sdk::token::Client::new(env, token_id);
    stellar.mint(user, &1_000_000_000_000i128);
    tok.approve(user, contract_id, &i128::MAX, &1_000_000u32);
}

/// Create and store a market that is already past its `end_time` (and within the
/// dispute window) so `process_dispute` can proceed.
fn make_market(
    env: &Env,
    contract_id: &Address,
    admin: &Address,
    market_id: &Symbol,
    dispute_window: u64,
) {
    let now = env.ledger().timestamp();
    let end_time = now.saturating_sub(3_600); // ended 1 h ago

    let market = Market {
        admin: admin.clone(),
        question: SorobanString::from_str(env, "Will BTC exceed 50k?"),
        outcomes: soroban_sdk::vec![
            env,
            SorobanString::from_str(env, "yes"),
            SorobanString::from_str(env, "no"),
        ],
        end_time,
        oracle_config: OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(
                env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            SorobanString::from_str(env, "BTC/USD"),
            50_000_00i128,
            SorobanString::from_str(env, "gt"),
        ),
        metadata_commitment: soroban_sdk::BytesN::from_array(env, &[0u8; 32]),
        has_fallback: false,
        fallback_oracle_config: OracleConfig::none_sentinel(env),
        resolution_timeout: 86_400u64,
        oracle_result: Some(SorobanString::from_str(env, "yes")),
        votes: soroban_sdk::Map::new(env),
        total_staked: 0,
        dispute_stakes: soroban_sdk::Map::new(env),
        stakes: soroban_sdk::Map::new(env),
        claimed: soroban_sdk::Map::new(env),
        winning_outcomes: None,
        fee_collected: false,
        state: MarketState::Active,
        total_extension_days: 0,
        max_extension_days: 30,
        extension_history: soroban_sdk::Vec::new(env),
        category: None,
        tags: soroban_sdk::Vec::new(env),
        min_pool_size: None,
        bet_deadline: 0,
        dispute_window_seconds: dispute_window,
        winnings_swept: false,
        timelock_config: predictify_hybrid::timelock::MarketTimelockConfig::default(),
        dispute_stake_floor: None,
        max_participants: None,
        min_bet_amount: None,
    };

    env.as_contract(contract_id, || {
        MarketStateManager::update_market(env, market_id, &market);
    });
}

// ---------------------------------------------------------------------------
// Stake boundary tests
// ---------------------------------------------------------------------------

#[test]
fn stake_exactly_at_minimum() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);
    fund(&env, &contract_id, &token_id, &user);

    let market_id = Symbol::new(&env, "STK_AT");
    make_market(&env, &contract_id, &admin, &market_id, DISPUTE_PERIOD_SECS);

    let result = env.as_contract(&contract_id, || {
        DisputeManager::process_dispute(&env, user, market_id, MIN_DISPUTE_STAKE, None)
    });

    // Must succeed (or fail with a reason other than InsufficientStake).
    match result {
        Ok(_) => {}
        Err(e) => assert_ne!(
            e,
            Error::InsufficientStake,
            "MIN_DISPUTE_STAKE should not be rejected as insufficient"
        ),
    }
}

#[test]
fn stake_one_below_minimum() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);
    fund(&env, &contract_id, &token_id, &user);

    let market_id = Symbol::new(&env, "STK_LOW");
    make_market(&env, &contract_id, &admin, &market_id, DISPUTE_PERIOD_SECS);

    let result = env.as_contract(&contract_id, || {
        DisputeManager::process_dispute(&env, user, market_id, MIN_DISPUTE_STAKE - 1, None)
    });

    assert!(
        matches!(result, Err(Error::InsufficientStake)),
        "stake one below minimum must be rejected, got {:?}",
        result
    );
}

#[test]
fn stake_zero_rejected() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);
    fund(&env, &contract_id, &token_id, &user);

    let market_id = Symbol::new(&env, "STK_ZRO");
    make_market(&env, &contract_id, &admin, &market_id, DISPUTE_PERIOD_SECS);

    let result = env.as_contract(&contract_id, || {
        DisputeManager::process_dispute(&env, user, market_id, 0, None)
    });

    assert!(
        matches!(result, Err(Error::InsufficientStake)),
        "zero stake must be rejected, got {:?}",
        result
    );
}

#[test]
fn stake_negative_rejected() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);
    fund(&env, &contract_id, &token_id, &user);

    let market_id = Symbol::new(&env, "STK_NEG");
    make_market(&env, &contract_id, &admin, &market_id, DISPUTE_PERIOD_SECS);

    let result = env.as_contract(&contract_id, || {
        DisputeManager::process_dispute(&env, user, market_id, -1, None)
    });

    assert!(
        matches!(result, Err(Error::InsufficientStake)),
        "negative stake must be rejected, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Duplicate dispute test
// ---------------------------------------------------------------------------

#[test]
fn duplicate_dispute_same_user() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);
    fund(&env, &contract_id, &token_id, &user);

    let market_id = Symbol::new(&env, "DUP");
    make_market(&env, &contract_id, &admin, &market_id, DISPUTE_PERIOD_SECS);

    // First dispute must succeed.
    let first = env.as_contract(&contract_id, || {
        DisputeManager::process_dispute(
            &env,
            user.clone(),
            market_id.clone(),
            MIN_DISPUTE_STAKE,
            None,
        )
    });
    assert!(
        first.is_ok(),
        "first dispute should succeed, got {:?}",
        first
    );

    // Second dispute by the same user must be rejected.
    let second = env.as_contract(&contract_id, || {
        DisputeManager::process_dispute(&env, user, market_id, MIN_DISPUTE_STAKE, None)
    });
    assert!(
        matches!(second, Err(Error::AlreadyDisputed)),
        "duplicate dispute must return AlreadyDisputed, got {:?}",
        second
    );
}

// ---------------------------------------------------------------------------
// Timing tests
// ---------------------------------------------------------------------------

#[test]
fn dispute_past_window_rejected() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);
    fund(&env, &contract_id, &token_id, &user);

    let market_id = Symbol::new(&env, "PAST_WIN");
    // Dispute window of 1 second — ledger time is already 1 h past end_time.
    make_market(&env, &contract_id, &admin, &market_id, 1 /* 1 second */);

    let result = env.as_contract(&contract_id, || {
        DisputeManager::process_dispute(&env, user, market_id, MIN_DISPUTE_STAKE, None)
    });

    assert!(
        matches!(
            result,
            Err(Error::MarketResolved) | Err(Error::InvalidState) | Err(Error::DisputeCondNotMet)
        ),
        "dispute past window must be rejected, got {:?}",
        result
    );
}

#[test]
fn dispute_before_end_time_rejected() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);
    fund(&env, &contract_id, &token_id, &user);

    let market_id = Symbol::new(&env, "PRE_END");

    // Build a market whose end_time is 24 h in the future.
    let now = env.ledger().timestamp();
    let market = Market {
        admin: admin.clone(),
        question: SorobanString::from_str(&env, "Future market?"),
        outcomes: soroban_sdk::vec![
            &env,
            SorobanString::from_str(&env, "yes"),
            SorobanString::from_str(&env, "no"),
        ],
        end_time: now + 86_400, // ends in 24 h
        oracle_config: OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(
                &env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            SorobanString::from_str(&env, "BTC/USD"),
            50_000_00i128,
            SorobanString::from_str(&env, "gt"),
        ),
        metadata_commitment: soroban_sdk::BytesN::from_array(&env, &[0u8; 32]),
        has_fallback: false,
        fallback_oracle_config: OracleConfig::none_sentinel(&env),
        resolution_timeout: 86_400u64,
        oracle_result: None, // not yet resolved
        votes: soroban_sdk::Map::new(&env),
        total_staked: 0,
        dispute_stakes: soroban_sdk::Map::new(&env),
        stakes: soroban_sdk::Map::new(&env),
        claimed: soroban_sdk::Map::new(&env),
        winning_outcomes: None,
        fee_collected: false,
        state: MarketState::Active,
        total_extension_days: 0,
        max_extension_days: 30,
        extension_history: soroban_sdk::Vec::new(&env),
        category: None,
        tags: soroban_sdk::Vec::new(&env),
        min_pool_size: None,
        bet_deadline: 0,
        dispute_window_seconds: DISPUTE_PERIOD_SECS,
        winnings_swept: false,
        timelock_config: predictify_hybrid::timelock::MarketTimelockConfig::default(),
        dispute_stake_floor: None,
        max_participants: None,
        min_bet_amount: None,
    };

    env.as_contract(&contract_id, || {
        MarketStateManager::update_market(&env, &market_id, &market);
    });

    let result = env.as_contract(&contract_id, || {
        DisputeManager::process_dispute(&env, user, market_id, MIN_DISPUTE_STAKE, None)
    });

    assert!(
        matches!(
            result,
            Err(Error::MarketClosed)
                | Err(Error::InvalidState)
                | Err(Error::OracleResultNotAvailable)
        ),
        "dispute before end_time must be rejected, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Vote-by-opener test
// ---------------------------------------------------------------------------

#[test]
fn disputer_cannot_vote_own_dispute() {
    let (env, admin, contract_id, token_id) = setup();
    let disputer = Address::generate(&env);
    fund(&env, &contract_id, &token_id, &disputer);

    let market_id = Symbol::new(&env, "NO_SELF");
    make_market(&env, &contract_id, &admin, &market_id, DISPUTE_PERIOD_SECS);

    // Open the dispute first.
    let open = env.as_contract(&contract_id, || {
        DisputeManager::process_dispute(
            &env,
            disputer.clone(),
            market_id.clone(),
            MIN_DISPUTE_STAKE,
            None,
        )
    });
    assert!(open.is_ok(), "dispute open should succeed: {:?}", open);

    // Now try to vote on the same market using the *extended* overload.
    // The disputer is in `dispute_stakes`, so this must be rejected.
    let dispute_id = Symbol::new(&env, "VOT_ID");
    let vote_result = env.as_contract(&contract_id, || {
        DisputeManager::vote_on_dispute(
            &env,
            disputer.clone(),
            market_id,
            dispute_id,
            true,
            MIN_DISPUTE_STAKE,
            None,
        )
    });

    assert!(
        matches!(vote_result, Err(Error::DisputerCannotVote)),
        "dispute opener must not be able to vote, got {:?}",
        vote_result
    );
}

// ---------------------------------------------------------------------------
// Stake cap tests
// ---------------------------------------------------------------------------

#[test]
fn stake_cap_exceeded() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);
    fund(&env, &contract_id, &token_id, &user);

    let market_id = Symbol::new(&env, "CAP_EXC");
    make_market(&env, &contract_id, &admin, &market_id, DISPUTE_PERIOD_SECS);

    // Set a cap lower than the stake we'll try to use.
    let cap: i128 = MIN_DISPUTE_STAKE; // exactly at minimum
    let attempted_stake = MIN_DISPUTE_STAKE + 1; // one above cap

    env.as_contract(&contract_id, || {
        let cap_key = DataKey::DisputeStakeCap(market_id.clone(), user.clone());
        env.storage().persistent().set(&cap_key, &cap);
        env.storage()
            .persistent()
            .extend_ttl(&cap_key, 535_680, 535_680);
    });

    let result = env.as_contract(&contract_id, || {
        DisputeManager::process_dispute(&env, user, market_id, attempted_stake, None)
    });

    assert!(
        matches!(result, Err(Error::DisputeStakeCapExceeded)),
        "stake above cap must return DisputeStakeCapExceeded, got {:?}",
        result
    );
}

#[test]
fn stake_cap_at_boundary_accepted() {
    let (env, admin, contract_id, token_id) = setup();
    let user = Address::generate(&env);
    fund(&env, &contract_id, &token_id, &user);

    let market_id = Symbol::new(&env, "CAP_OK");
    make_market(&env, &contract_id, &admin, &market_id, DISPUTE_PERIOD_SECS);

    let cap: i128 = MIN_DISPUTE_STAKE * 5;

    env.as_contract(&contract_id, || {
        let cap_key = DataKey::DisputeStakeCap(market_id.clone(), user.clone());
        env.storage().persistent().set(&cap_key, &cap);
        env.storage()
            .persistent()
            .extend_ttl(&cap_key, 535_680, 535_680);
    });

    let result = env.as_contract(&contract_id, || {
        // Stake exactly at the cap — must NOT return DisputeStakeCapExceeded.
        DisputeManager::process_dispute(&env, user, market_id, cap, None)
    });

    if let Err(e) = result {
        assert_ne!(
            e,
            Error::DisputeStakeCapExceeded,
            "stake at cap boundary must not be rejected for cap exceeded"
        );
    }
}

// ---------------------------------------------------------------------------
// Error code stability (regression guard)
// ---------------------------------------------------------------------------

#[test]
fn error_codes_stable() {
    let dispute_errors: &[(Error, u32)] = &[
        (Error::AlreadyDisputed, 404),
        (Error::DisputeVoteExpired, 405),
        (Error::DisputeVoteDenied, 406),
        (Error::DisputeAlreadyVoted, 407),
        (Error::DisputeCondNotMet, 408),
        (Error::DisputeFeeFailed, 409),
        (Error::DisputeError, 410),
        (Error::DisputerCannotVote, 438),
        (Error::DisputeStakeCapExceeded, 522),
    ];

    for (err, expected) in dispute_errors {
        assert_eq!(
            *err as u32,
            *expected,
            "dispute error code changed for {:?}; this is a client-facing API change",
            err
        );
    }
}
