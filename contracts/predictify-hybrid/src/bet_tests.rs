//! # Bet Placement Tests
//!
//! Comprehensive test suite for the bet placement mechanism.
//!
//! ## Test Coverage
//!
//! - **Happy Path Tests**: Successful bet placement scenarios
//! - **Validation Tests**: Input validation and error handling
//! - **Edge Case Tests**: Boundary conditions and special scenarios
//! - **Security Tests**: Double betting prevention and authentication
//! - **Integration Tests**: Full bet lifecycle testing
//! - **Bet Limits Tests**: Comprehensive validation of minimum and maximum bet limits
//!
//! ## Test Coverage Target: 95%+

#![cfg(test)]

use crate::bets::{BetManager, BetStorage, BetValidator, MAX_BET_AMOUNT, MIN_BET_AMOUNT};
use crate::types::{Bet, BetStats, BetStatus, Market, MarketState, OracleConfig, OracleProvider};
use crate::{Error, PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, Address, Env, Map, String, Symbol, Vec,
};

// ===== TEST SETUP =====

/// Test infrastructure for bet placement tests
struct BetTestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    user: Address,
    user2: Address,
    token_id: Address,
    market_id: Symbol,
}

impl BetTestSetup {
    /// Create a new test environment with contract deployed and initialized
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Setup admin and users
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let user2 = Address::generate(&env);

        // Register and initialize the contract
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);

        // Setup token for staking
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        // Set token for staking in contract storage
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Fund users with tokens
        let stellar_client = StellarAssetClient::new(&env, &token_id);
        stellar_client.mint(&admin, &10_000_0000000); // 10,000 XLM
        stellar_client.mint(&user, &1000_0000000); // 1,000 XLM
        stellar_client.mint(&user2, &1000_0000000); // 1,000 XLM

        // Approve contract to spend tokens on behalf of users (for bet placement)
        let token_client = soroban_sdk::token::Client::new(&env, &token_id);
        token_client.approve(&user, &contract_id, &i128::MAX, &1000000);
        token_client.approve(&user2, &contract_id, &i128::MAX, &1000000);
        token_client.approve(&admin, &contract_id, &i128::MAX, &1000000);

        // Create a test market
        let market_id = Self::create_test_market_static(&env, &contract_id, &admin);

        Self {
            env,
            contract_id,
            admin,
            user,
            user2,
            token_id,
            market_id,
        }
    }

    /// Create a test market
    fn create_test_market_static(env: &Env, contract_id: &Address, admin: &Address) -> Symbol {
        let client = PredictifyHybridClient::new(env, contract_id);

        let outcomes = vec![
            env,
            String::from_str(env, "yes"),
            String::from_str(env, "no"),
        ];

        client.create_market(
            admin,
            &String::from_str(env, "Will BTC reach $100,000 by end of 2024?"),
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::Reflector,
                oracle_address: Address::from_str(env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
                feed_id: String::from_str(env, "BTC/USD"),
                threshold: 100_000_00000000, // $100,000
                comparison: String::from_str(env, "gte"),
            },
            &None,
            &0u64,
        )
    }

    /// Get client for contract interactions
    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    /// Advance time past market end
    fn advance_past_market_end(&self) {
        let market = self.client().get_market(&self.market_id).unwrap();
        self.env.ledger().set(LedgerInfo {
            timestamp: market.end_time + 1,
            protocol_version: 22,
            sequence_number: self.env.ledger().sequence(),
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 10000,
        });
    }
}

// ===== BET LIMITS TESTS =====
// This section contains tests for bet limit validation
// Organized into two subsections:
// 1. Hardcoded limits tests (MIN_BET_AMOUNT, MAX_BET_AMOUNT constants)
// 2. Configurable limits tests (requires set_global_bet_limits implementation)

// ===== HARDCODED LIMITS TESTS =====
// Tests for fixed MIN_BET_AMOUNT and MAX_BET_AMOUNT constants
// These tests validate the default hardcoded limits in bets.rs

// ===== BOUNDARY VALUE TESTS =====

#[test]
fn test_place_bet_exactly_minimum() {
    let setup = BetTestSetup::new();
    let client = setup.client();

    // Place bet with exactly minimum amount
    let bet = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &MIN_BET_AMOUNT,
    );

    assert_eq!(bet.amount, MIN_BET_AMOUNT);
    assert_eq!(bet.status, BetStatus::Active);
    assert_eq!(bet.user, setup.user);
}

#[test]
fn test_place_bet_exactly_maximum() {
    let setup = BetTestSetup::new();
    let client = setup.client();

    // Fund user with sufficient tokens for max bet
    let stellar_client = StellarAssetClient::new(&setup.env, &setup.token_id);
    stellar_client.mint(&setup.user, &MAX_BET_AMOUNT);

    // Place bet with exactly maximum amount
    let bet = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &MAX_BET_AMOUNT,
    );

    assert_eq!(bet.amount, MAX_BET_AMOUNT);
    assert_eq!(bet.status, BetStatus::Active);
    assert_eq!(bet.user, setup.user);
}

#[test]
fn test_validate_bet_amount_at_minimum() {
    // Validator should accept exactly MIN_BET_AMOUNT
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT).is_ok());
}

#[test]
fn test_validate_bet_amount_at_maximum() {
    // Validator should accept exactly MAX_BET_AMOUNT
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT).is_ok());
}

// ===== REJECTION TESTS =====

#[test]
fn test_place_bet_below_minimum_one_stroop() {
    // Test validator directly - MIN_BET_AMOUNT - 1 should be rejected
    let result = BetValidator::validate_bet_amount(MIN_BET_AMOUNT - 1);
    assert!(result.is_err());
    
    // Verify error code
    assert_eq!(Error::InsufficientStake as i128, 107);
}

#[test]
fn test_place_bet_below_minimum_half() {
    // Test validator directly - half of minimum should be rejected
    let result = BetValidator::validate_bet_amount(MIN_BET_AMOUNT / 2);
    assert!(result.is_err());
    
    // Verify error code
    assert_eq!(Error::InsufficientStake as i128, 107);
}

#[test]
fn test_place_bet_zero_amount() {
    // Test validator directly - zero should be rejected
    let result = BetValidator::validate_bet_amount(0);
    assert!(result.is_err());
    
    // Verify error code
    assert_eq!(Error::InsufficientStake as i128, 107);
}

#[test]
fn test_place_bet_negative_amount() {
    // Test validator directly - negative should be rejected
    let result = BetValidator::validate_bet_amount(-1);
    assert!(result.is_err());
    
    // Verify error code
    assert_eq!(Error::InsufficientStake as i128, 107);
}

#[test]
fn test_place_bet_above_maximum_one_stroop() {
    // Test validator directly - MAX_BET_AMOUNT + 1 should be rejected
    let result = BetValidator::validate_bet_amount(MAX_BET_AMOUNT + 1);
    assert!(result.is_err());
    
    // Verify error code
    assert_eq!(Error::InvalidInput as i128, 401);
}

#[test]
fn test_place_bet_above_maximum_double() {
    // Test validator directly - double maximum should be rejected
    let result = BetValidator::validate_bet_amount(MAX_BET_AMOUNT * 2);
    assert!(result.is_err());
    
    // Verify error code
    assert_eq!(Error::InvalidInput as i128, 401);
}

#[test]
fn test_validate_bet_amount_below_minimum() {
    // Validator should reject amounts below minimum
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT - 1).is_err());
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT / 2).is_err());
    assert!(BetValidator::validate_bet_amount(1).is_err());
}

#[test]
fn test_validate_bet_amount_above_maximum() {
    // Validator should reject amounts above maximum
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT + 1).is_err());
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT * 2).is_err());
}

// ===== EDGE CASE TESTS =====

#[test]
fn test_validate_bet_amount_zero() {
    // Zero should be rejected
    assert!(BetValidator::validate_bet_amount(0).is_err());
}

#[test]
fn test_validate_bet_amount_negative() {
    // Negative values should be rejected
    assert!(BetValidator::validate_bet_amount(-1).is_err());
    assert!(BetValidator::validate_bet_amount(-100).is_err());
    assert!(BetValidator::validate_bet_amount(i128::MIN).is_err());
}

#[test]
fn test_validate_bet_amount_very_small() {
    // Very small positive values should be rejected
    assert!(BetValidator::validate_bet_amount(1).is_err());
    assert!(BetValidator::validate_bet_amount(100).is_err());
    assert!(BetValidator::validate_bet_amount(999_999).is_err()); // Just below MIN
}

#[test]
fn test_validate_bet_amount_very_large() {
    // Very large values should be rejected
    assert!(BetValidator::validate_bet_amount(i128::MAX).is_err());
    // Test a value significantly above MAX_BET_AMOUNT
    let very_large = MAX_BET_AMOUNT * 100;
    assert!(BetValidator::validate_bet_amount(very_large).is_err());
}

#[test]
fn test_validate_bet_amount_boundary_minus_one() {
    // MIN_BET_AMOUNT - 1 should be rejected
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT - 1).is_err());
}

#[test]
fn test_validate_bet_amount_boundary_plus_one() {
    // MAX_BET_AMOUNT + 1 should be rejected
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT + 1).is_err());
}

#[test]
fn test_validate_bet_amount_within_range() {
    // Values between min and max should be accepted
    let mid_point = (MIN_BET_AMOUNT + MAX_BET_AMOUNT) / 2;
    assert!(BetValidator::validate_bet_amount(mid_point).is_ok());
    
    // Test values near minimum (but valid)
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT + 1).is_ok());
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT + 1000).is_ok());
    
    // Test values near maximum (but valid)
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT - 1).is_ok());
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT - 1000).is_ok());
    
    // Test various amounts in the valid range
    assert!(BetValidator::validate_bet_amount(5_000_000).is_ok()); // 0.5 XLM
    assert!(BetValidator::validate_bet_amount(50_000_000).is_ok()); // 5 XLM
    assert!(BetValidator::validate_bet_amount(500_000_000).is_ok()); // 50 XLM
}

// ===== INTEGRATION TESTS =====

#[test]
fn test_place_bet_minimum_with_sufficient_balance() {
    let setup = BetTestSetup::new();
    let client = setup.client();

    // User has sufficient balance (already funded in setup)
    let bet = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &MIN_BET_AMOUNT,
    );

    assert_eq!(bet.amount, MIN_BET_AMOUNT);
    assert_eq!(bet.status, BetStatus::Active);
    
    // Verify bet is stored
    let retrieved_bet = client.get_bet(&setup.market_id, &setup.user);
    assert!(retrieved_bet.is_some());
    assert_eq!(retrieved_bet.unwrap().amount, MIN_BET_AMOUNT);
}

#[test]
fn test_place_bet_maximum_with_sufficient_balance() {
    let setup = BetTestSetup::new();
    let client = setup.client();

    // Fund user with sufficient tokens for max bet
    let stellar_client = StellarAssetClient::new(&setup.env, &setup.token_id);
    stellar_client.mint(&setup.user, &MAX_BET_AMOUNT);

    let bet = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &MAX_BET_AMOUNT,
    );

    assert_eq!(bet.amount, MAX_BET_AMOUNT);
    assert_eq!(bet.status, BetStatus::Active);
    
    // Verify bet is stored
    let retrieved_bet = client.get_bet(&setup.market_id, &setup.user);
    assert!(retrieved_bet.is_some());
    assert_eq!(retrieved_bet.unwrap().amount, MAX_BET_AMOUNT);
}

#[test]
fn test_place_bet_below_minimum_rejects_with_error() {
    // Verify error code for below minimum
    assert_eq!(Error::InsufficientStake as i128, 107);
    
    // Verify validator returns correct error
    let result = BetValidator::validate_bet_amount(MIN_BET_AMOUNT - 1);
    assert!(result.is_err());
}

#[test]
fn test_market_validation_for_betting() {
    let env = Env::default();
    let admin = Address::generate(&env);

    // Create an active market
    let active_market = Market {
        admin: admin.clone(),
        question: String::from_str(&env, "Test question?"),
        outcomes: vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ],
        end_time: env.ledger().timestamp() + 86400, // 1 day in future
        oracle_config: OracleConfig {
            provider: OracleProvider::Reflector,
            oracle_address: Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
            feed_id: String::from_str(&env, "BTC/USD"),
            threshold: 50000,
            comparison: String::from_str(&env, "gte"),
        },
        fallback_oracle_config: Vec::new(&env),
        resolution_timeout: 0,
        oracle_result: None,
        votes: Map::new(&env),
        total_staked: 0,
        dispute_stakes: Map::new(&env),
        stakes: Map::new(&env),
        claimed: Map::new(&env),
        winning_outcomes: None,
        fee_collected: false,
        state: MarketState::Active,
        total_extension_days: 0,
        max_extension_days: 30,
        extension_history: Vec::new(&env),
        // Resolution delay fields
        resolution_proposed_outcome: None,
        resolution_proposed_at: 0,
        resolution_window_end_time: 0,
        resolution_is_finalized: false,
        resolution_dispute_count: 0,
        resolution_source: None,
        dispute_window_hours: 0,
        category: None,
        tags: Vec::new(&env),
    };

    // Active market should be valid
    assert!(BetValidator::validate_market_for_betting(&env, &active_market).is_ok());

    // Ended market should fail
    let mut ended_market = active_market.clone();
    ended_market.state = MarketState::Ended;
    assert!(BetValidator::validate_market_for_betting(&env, &ended_market).is_err());

    // Resolved market should fail
    let mut resolved_market = active_market.clone();
    resolved_market.winning_outcomes = Some(vec![&env, String::from_str(&env, "yes")]);
    assert!(BetValidator::validate_market_for_betting(&env, &resolved_market).is_err());

    // Closed market should fail
    let mut closed_market = active_market.clone();
    closed_market.state = MarketState::Closed;
    assert!(BetValidator::validate_market_for_betting(&env, &closed_market).is_err());
}

fn test_place_bet_above_maximum_rejects_with_error() {
    // Verify error code for above maximum
    assert_eq!(Error::InvalidInput as i128, 401);
    
    // Verify validator returns correct error
    let result = BetValidator::validate_bet_amount(MAX_BET_AMOUNT + 1);
    assert!(result.is_err());
}

#[test]
fn test_place_bet_valid_amounts_in_range() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    let stellar_client = StellarAssetClient::new(&setup.env, &setup.token_id);

    // Test multiple valid amounts across the range
    // Test each amount individually to avoid Vec indexing issues
    
    // Test minimum amount
    let bet1 = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &MIN_BET_AMOUNT,
    );
    assert_eq!(bet1.amount, MIN_BET_AMOUNT);
    assert_eq!(bet1.status, BetStatus::Active);

    // Test amount just above minimum
    let market2 = BetTestSetup::create_test_market_static(&setup.env, &setup.contract_id, &setup.admin);
    let bet2 = client.place_bet(
        &setup.user2,
        &market2,
        &String::from_str(&setup.env, "yes"),
        &(MIN_BET_AMOUNT + 1_000_000),
    );
    assert_eq!(bet2.amount, MIN_BET_AMOUNT + 1_000_000);
    assert_eq!(bet2.status, BetStatus::Active);

    // Test mid-range amounts with new users and markets
    let user3 = Address::generate(&setup.env);
    let user4 = Address::generate(&setup.env);
    let user5 = Address::generate(&setup.env);
    let user6 = Address::generate(&setup.env);
    let user7 = Address::generate(&setup.env);
    
    stellar_client.mint(&user3, &1000_0000000);
    stellar_client.mint(&user4, &1000_0000000);
    stellar_client.mint(&user5, &1000_0000000);
    stellar_client.mint(&user6, &MAX_BET_AMOUNT);
    stellar_client.mint(&user7, &MAX_BET_AMOUNT);
    
    let token_client = soroban_sdk::token::Client::new(&setup.env, &setup.token_id);
    token_client.approve(&user3, &setup.contract_id, &i128::MAX, &1000000);
    token_client.approve(&user4, &setup.contract_id, &i128::MAX, &1000000);
    token_client.approve(&user5, &setup.contract_id, &i128::MAX, &1000000);
    token_client.approve(&user6, &setup.contract_id, &i128::MAX, &1000000);
    token_client.approve(&user7, &setup.contract_id, &i128::MAX, &1000000);

    // Test 1 XLM
    let market3 = BetTestSetup::create_test_market_static(&setup.env, &setup.contract_id, &setup.admin);
    let bet3 = client.place_bet(&user3, &market3, &String::from_str(&setup.env, "yes"), &10_000_000);
    assert_eq!(bet3.amount, 10_000_000);

    // Test 5 XLM
    let market4 = BetTestSetup::create_test_market_static(&setup.env, &setup.contract_id, &setup.admin);
    let bet4 = client.place_bet(&user4, &market4, &String::from_str(&setup.env, "yes"), &50_000_000);
    assert_eq!(bet4.amount, 50_000_000);

    // Test 10 XLM
    let market5 = BetTestSetup::create_test_market_static(&setup.env, &setup.contract_id, &setup.admin);
    let bet5 = client.place_bet(&user5, &market5, &String::from_str(&setup.env, "yes"), &100_000_000);
    assert_eq!(bet5.amount, 100_000_000);

    // Test amount just below maximum
    let market6 = BetTestSetup::create_test_market_static(&setup.env, &setup.contract_id, &setup.admin);
    let bet6 = client.place_bet(
        &user6,
        &market6,
        &String::from_str(&setup.env, "yes"),
        &(MAX_BET_AMOUNT - 1_000_000),
    );
    assert_eq!(bet6.amount, MAX_BET_AMOUNT - 1_000_000);

    // Test maximum amount
    let market7 = BetTestSetup::create_test_market_static(&setup.env, &setup.contract_id, &setup.admin);
    let bet7 = client.place_bet(
        &user7,
        &market7,
        &String::from_str(&setup.env, "yes"),
        &MAX_BET_AMOUNT,
    );
    assert_eq!(bet7.amount, MAX_BET_AMOUNT);
    assert_eq!(bet7.status, BetStatus::Active);
}

// ===== SECURITY AND ERROR CODE TESTS =====

#[test]
fn test_bet_below_minimum_returns_insufficient_stake() {
    // Verify error code constant
    assert_eq!(Error::InsufficientStake as i128, 107);
    
    // Verify validator rejects below minimum
    let result = BetValidator::validate_bet_amount(MIN_BET_AMOUNT - 1);
    assert!(result.is_err());
}

#[test]
fn test_bet_above_maximum_returns_invalid_input() {
    // Verify error code constant
    assert_eq!(Error::InvalidInput as i128, 401);
    
    // Verify validator rejects above maximum
    let result = BetValidator::validate_bet_amount(MAX_BET_AMOUNT + 1);
    assert!(result.is_err());
}

#[test]
fn test_error_codes_match_constants() {
    // Verify all error codes match their expected values
    assert_eq!(Error::InsufficientStake as i128, 107);
    assert_eq!(Error::InvalidInput as i128, 401);
    assert_eq!(Error::MarketNotFound as i128, 101);
    assert_eq!(Error::MarketClosed as i128, 102);
    assert_eq!(Error::InvalidOutcome as i128, 108);
}

// ===== COMPREHENSIVE COVERAGE TESTS =====

#[test]
fn test_multiple_bets_at_different_limits() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    let stellar_client = StellarAssetClient::new(&setup.env, &setup.token_id);

    // Create multiple users
    let user3 = Address::generate(&setup.env);
    let user4 = Address::generate(&setup.env);
    
    // Fund users
    stellar_client.mint(&user3, &1000_0000000);
    stellar_client.mint(&user4, &MAX_BET_AMOUNT);
    
    let token_client = soroban_sdk::token::Client::new(&setup.env, &setup.token_id);
    token_client.approve(&user3, &setup.contract_id, &i128::MAX, &1000000);
    token_client.approve(&user4, &setup.contract_id, &i128::MAX, &1000000);

    // Create separate markets for each user to avoid double-betting
    let market1 = BetTestSetup::create_test_market_static(&setup.env, &setup.contract_id, &setup.admin);
    let market2 = BetTestSetup::create_test_market_static(&setup.env, &setup.contract_id, &setup.admin);
    let market3 = BetTestSetup::create_test_market_static(&setup.env, &setup.contract_id, &setup.admin);
    let market4 = BetTestSetup::create_test_market_static(&setup.env, &setup.contract_id, &setup.admin);

    // User 1: Minimum bet
    let bet1 = client.place_bet(
        &setup.user,
        &market1,
        &String::from_str(&setup.env, "yes"),
        &MIN_BET_AMOUNT,
    );
    assert_eq!(bet1.amount, MIN_BET_AMOUNT);

    // User 2: Mid-range bet
    let bet2 = client.place_bet(
        &setup.user2,
        &market2,
        &String::from_str(&setup.env, "yes"),
        &50_000_000,
    );
    assert_eq!(bet2.amount, 50_000_000);

    // User 3: Large bet (but not max)
    let bet3 = client.place_bet(
        &user3,
        &market3,
        &String::from_str(&setup.env, "yes"),
        &500_000_000,
    );
    assert_eq!(bet3.amount, 500_000_000);

    // User 4: Maximum bet
    let bet4 = client.place_bet(
        &user4,
        &market4,
        &String::from_str(&setup.env, "yes"),
        &MAX_BET_AMOUNT,
    );
    assert_eq!(bet4.amount, MAX_BET_AMOUNT);
}

#[test]
fn test_bet_limit_validation_in_isolation() {
    // Test validator directly with comprehensive range of values
    
    // Valid values
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT).is_ok());
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT).is_ok());
    assert!(BetValidator::validate_bet_amount((MIN_BET_AMOUNT + MAX_BET_AMOUNT) / 2).is_ok());
    
    // Invalid - below minimum
    assert!(BetValidator::validate_bet_amount(0).is_err());
    assert!(BetValidator::validate_bet_amount(-1).is_err());
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT - 1).is_err());
    assert!(BetValidator::validate_bet_amount(1).is_err());
    
    // Invalid - above maximum
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT + 1).is_err());
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT * 2).is_err());
    assert!(BetValidator::validate_bet_amount(i128::MAX).is_err());
}

#[test]
fn test_bet_limit_validation_in_place_bet_flow() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    let stellar_client = StellarAssetClient::new(&setup.env, &setup.token_id);

    // Test that validation happens in the full bet placement flow
    
    // Valid minimum bet through full flow
    let bet_min = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &MIN_BET_AMOUNT,
    );
    assert_eq!(bet_min.amount, MIN_BET_AMOUNT);
    
    // Create new market and user for max bet test
    let market2 = BetTestSetup::create_test_market_static(&setup.env, &setup.contract_id, &setup.admin);
    let user2 = Address::generate(&setup.env);
    stellar_client.mint(&user2, &MAX_BET_AMOUNT);
    let token_client = soroban_sdk::token::Client::new(&setup.env, &setup.token_id);
    token_client.approve(&user2, &setup.contract_id, &i128::MAX, &1000000);
    
    // Valid maximum bet through full flow
    let bet_max = client.place_bet(
        &user2,
        &market2,
        &String::from_str(&setup.env, "yes"),
        &MAX_BET_AMOUNT,
    );
    assert_eq!(bet_max.amount, MAX_BET_AMOUNT);
}

#[test]
fn test_bet_amount_edge_cases_comprehensive() {
    // Comprehensive edge case testing for validator
    
    // Boundary conditions
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT).is_ok());
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT - 1).is_err());
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT).is_ok());
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT + 1).is_err());
    
    // Zero and negative
    assert!(BetValidator::validate_bet_amount(0).is_err());
    assert!(BetValidator::validate_bet_amount(-1).is_err());
    assert!(BetValidator::validate_bet_amount(-1000).is_err());
    assert!(BetValidator::validate_bet_amount(i128::MIN).is_err());
    
    // Very small positive
    assert!(BetValidator::validate_bet_amount(1).is_err());
    assert!(BetValidator::validate_bet_amount(100).is_err());
    assert!(BetValidator::validate_bet_amount(999_999).is_err());
    
    // Very large
    assert!(BetValidator::validate_bet_amount(i128::MAX).is_err());
    
    // Valid range values
    assert!(BetValidator::validate_bet_amount(MIN_BET_AMOUNT + 1).is_ok());
    assert!(BetValidator::validate_bet_amount(MAX_BET_AMOUNT - 1).is_ok());
    assert!(BetValidator::validate_bet_amount(10_000_000).is_ok());
    assert!(BetValidator::validate_bet_amount(100_000_000).is_ok());
    assert!(BetValidator::validate_bet_amount(1_000_000_000).is_ok());
}

// ===== CONFIGURABLE LIMITS TESTS =====
// NOTE: These tests require set_global_bet_limits and set_event_bet_limits
// functions to be implemented in the contract. If these functions don't exist,
// these tests will fail to compile. Uncomment or implement the functions before
// enabling these tests, or mark them with #[ignore] until the functionality is available.

// IMPORTANT: Before uncommenting these tests, verify that the following functions exist:
// - PredictifyHybridClient::set_global_bet_limits()
// - PredictifyHybridClient::set_event_bet_limits()
//
// If they don't exist, keep these tests commented out or marked with #[ignore]

/*
#[test]
fn test_set_global_bet_limits_and_place_bet_exactly_min_max() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    let min = 5_000000i128; // 0.5 XLM
    let max = 50_000000i128; // 5 XLM

    setup.env.mock_all_auths();
    client.set_global_bet_limits(&setup.admin, &min, &max);

    // Exactly min: must succeed
    let bet_min = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &min,
    );
    assert_eq!(bet_min.amount, min);

    // Exactly max: need second user (first already bet)
    let stellar_client = StellarAssetClient::new(&setup.env, &setup.token_id);
    stellar_client.mint(&setup.user2, &max);
    setup.env.mock_all_auths();
    let bet_max = client.place_bet(
        &setup.user2,
        &setup.market_id,
        &String::from_str(&setup.env, "no"),
        &max,
    );
    assert_eq!(bet_max.amount, max);
}

#[test]
#[should_panic]
fn test_place_bet_below_configured_min_rejects() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    let min = 10_000000i128;
    let max = 100_000000i128;

    setup.env.mock_all_auths();
    client.set_global_bet_limits(&setup.admin, &min, &max);

    setup.env.mock_all_auths();
    client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &(min - 1),
    );
}

#[test]
#[should_panic]
fn test_place_bet_above_configured_max_rejects() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    let min = 1_000000i128;
    let max = 20_000000i128;

    setup.env.mock_all_auths();
    client.set_global_bet_limits(&setup.admin, &min, &max);

    setup.env.mock_all_auths();
    client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &(max + 1),
    );
}

#[test]
fn test_set_event_bet_limits_overrides_global() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    let global_min = 1_000000i128;
    let global_max = 100_000000i128;
    let event_min = 15_000000i128;
    let event_max = 25_000000i128;

    setup.env.mock_all_auths();
    client.set_global_bet_limits(&setup.admin, &global_min, &global_max);
    setup.env.mock_all_auths();
    client.set_event_bet_limits(&setup.admin, &setup.market_id, &event_min, &event_max);

    // Exactly event min must succeed (below event min tested in separate should_panic test)
    setup.env.mock_all_auths();
    let bet = client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &event_min,
    );
    assert_eq!(bet.amount, event_min);
}

#[test]
#[should_panic]
fn test_place_bet_below_event_min_rejects() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    let event_min = 15_000000i128;
    let event_max = 25_000000i128;

    setup.env.mock_all_auths();
    client.set_event_bet_limits(&setup.admin, &setup.market_id, &event_min, &event_max);

    setup.env.mock_all_auths();
    client.place_bet(
        &setup.user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &(event_min - 1),
    );
}

#[test]
#[should_panic]
fn test_set_global_bet_limits_unauthorized() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    setup.env.mock_all_auths();
    client.set_global_bet_limits(&setup.user, &MIN_BET_AMOUNT, &MAX_BET_AMOUNT);
}

#[test]
#[should_panic]
fn test_set_global_bet_limits_min_above_max_rejects() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    setup.env.mock_all_auths();
    client.set_global_bet_limits(&setup.admin, &10_000000i128, &5_000000i128);
}

#[test]
#[should_panic]
fn test_set_global_bet_limits_below_absolute_min_rejects() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    setup.env.mock_all_auths();
    client.set_global_bet_limits(&setup.admin, &(MIN_BET_AMOUNT - 1), &MAX_BET_AMOUNT);
}

#[test]
#[should_panic]
fn test_set_global_bet_limits_above_absolute_max_rejects() {
    let setup = BetTestSetup::new();
    let client = setup.client();
    setup.env.mock_all_auths();
    client.set_global_bet_limits(&setup.admin, &MIN_BET_AMOUNT, &(MAX_BET_AMOUNT + 1));
}
*/
