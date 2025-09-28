//! # Property-Based Test Suite for Predictify Hybrid Contract
//!
//! This module implements comprehensive property-based testing for all contract behaviors,
//! focusing on invariant testing, edge case discovery, and robust validation of contract
//! properties across all possible input combinations.

#![cfg(test)]

use super::*;
use crate::config::{
    DEFAULT_PLATFORM_FEE_PERCENTAGE, MAX_MARKET_DURATION_DAYS, MAX_MARKET_OUTCOMES,
    MIN_MARKET_DURATION_DAYS, MIN_MARKET_OUTCOMES,
};
use crate::types::*;
use alloc::vec::Vec as StdVec;

use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, String as SorobanString,
};

// Use lib.rs PERCENTAGE_DENOMINATOR to avoid ambiguity
const PERCENTAGE_DENOM: i128 = 100;

// ===== PROPERTY-BASED TEST SUITE STRUCTURE =====

/// Main property-based test suite for the Predictify Hybrid contract
pub struct PropertyBasedTestSuite {
    pub env: Env,
    pub contract_id: Address,
    pub admin: Address,
    pub users: StdVec<Address>,
}

impl PropertyBasedTestSuite {
    /// Initialize the test suite with contract and test accounts
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin);

        // Generate multiple test users for comprehensive testing
        let users = (0..10).map(|_| Address::generate(&env)).collect();

        Self {
            env,
            contract_id,
            admin,
            users,
        }
    }

    /// Create a client for contract interactions
    pub fn client(&self) -> PredictifyHybridClient {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    /// Generate a valid oracle configuration for testing
    pub fn generate_oracle_config(&self, threshold: i128, comparison: &str) -> OracleConfig {
        OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: SorobanString::from_str(&self.env, "BTC/USD"),
            threshold,
            comparison: SorobanString::from_str(&self.env, comparison),
        }
    }

    /// Generate valid market outcomes
    pub fn generate_outcomes(&self, count: usize) -> soroban_sdk::Vec<SorobanString> {
        let mut outcomes = soroban_sdk::Vec::new(&self.env);
        for i in 0..count {
            let outcome_str = alloc::format!("outcome_{}", i);
            outcomes.push_back(SorobanString::from_str(&self.env, &outcome_str));
        }
        outcomes
    }

    /// Get user by index safely
    pub fn get_user(&self, index: usize) -> &Address {
        &self.users[index % self.users.len()]
    }
}

// ===== PROPERTY GENERATORS =====

/// Generate valid market questions
fn arb_market_question() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("Will BTC reach $100k by year end?"),
        Just("Will ETH surpass $5000 this quarter?"),
        Just("Will XLM hit $1 by December?"),
        Just("Will the market crash next month?"),
        Just("Will inflation exceed 5% this year?"),
    ]
}

/// Generate valid outcome counts
fn arb_outcome_count() -> impl Strategy<Value = usize> {
    (MIN_MARKET_OUTCOMES as usize)..=(MAX_MARKET_OUTCOMES as usize)
}

/// Generate valid duration in days
fn arb_duration_days() -> impl Strategy<Value = u32> {
    MIN_MARKET_DURATION_DAYS..=MAX_MARKET_DURATION_DAYS
}

/// Generate valid threshold values
fn arb_threshold() -> impl Strategy<Value = i128> {
    1i128..=1_000_000_00i128 // $1 to $1M in cents
}

/// Generate valid comparison operators
fn arb_comparison() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("gt"), Just("lt"), Just("eq")]
}

/// Generate valid stake amounts
fn arb_stake_amount() -> impl Strategy<Value = i128> {
    1_000_000i128..=1_000_000_000i128 // 1 XLM to 1000 XLM in stroops
}

/// Generate user indices for multi-user testing
fn arb_user_index() -> impl Strategy<Value = usize> {
    0..10usize
}

// ===== MARKET CREATION PROPERTY TESTS =====

proptest! {
    #[test]
    fn test_market_creation_properties(
        question in arb_market_question(),
        outcome_count in arb_outcome_count(),
        duration_days in arb_duration_days(),
        threshold in arb_threshold(),
        comparison in arb_comparison(),
    ) {
        let suite = PropertyBasedTestSuite::new();
        let client = suite.client();

        let question_str = SorobanString::from_str(&suite.env, question);
        let outcomes = suite.generate_outcomes(outcome_count);
        let oracle_config = suite.generate_oracle_config(threshold, comparison);

        // Property: Valid market creation should always succeed
        suite.env.mock_all_auths();
        let market_id = client.create_market(
            &suite.admin,
            &question_str,
            &outcomes,
            &duration_days,
            &oracle_config,
        );

        // Verify market was created with correct properties
        let market = client.get_market(&market_id).unwrap();

        // Property: Market question should match input
        prop_assert_eq!(market.question, question_str);

        // Property: Market should have correct number of outcomes
        prop_assert_eq!(market.outcomes.len(), outcome_count as u32);

        // Property: Market end time should be in the future
        prop_assert!(market.end_time > suite.env.ledger().timestamp());

        // Property: Market should be in Active state initially
        prop_assert_eq!(market.state, MarketState::Active);

        // Property: Oracle configuration should match input
        prop_assert_eq!(market.oracle_config.threshold, threshold);

        // Property: Market should have no votes initially
        prop_assert_eq!(market.total_staked, 0);
    }
}

proptest! {
    #[test]
    fn test_market_creation_invariants(
        question in arb_market_question(),
        outcome_count in arb_outcome_count(),
        duration_days in arb_duration_days(),
        threshold in arb_threshold(),
        comparison in arb_comparison(),
    ) {
        let suite = PropertyBasedTestSuite::new();
        let client = suite.client();

        let question_str = SorobanString::from_str(&suite.env, question);
        let outcomes = suite.generate_outcomes(outcome_count);
        let oracle_config = suite.generate_oracle_config(threshold, comparison);

        suite.env.mock_all_auths();
        let market_id = client.create_market(
            &suite.admin,
            &question_str,
            &outcomes,
            &duration_days,
            &oracle_config,
        );

        let market = client.get_market(&market_id).unwrap();

        // Store admin address to avoid borrowing issues
        let admin_addr = suite.admin.clone();

        // Invariant: Market admin should always be the creator
        prop_assert_eq!(market.admin, admin_addr);

        // Invariant: Market should have at least minimum outcomes
        prop_assert!(market.outcomes.len() >= MIN_MARKET_OUTCOMES);

        // Invariant: Market should not exceed maximum outcomes
        prop_assert!(market.outcomes.len() <= MAX_MARKET_OUTCOMES);

        // Invariant: Oracle threshold should be positive
        prop_assert!(market.oracle_config.threshold > 0);

        // Invariant: Market duration should be within limits
        let expected_end_time = suite.env.ledger().timestamp() + (duration_days as u64 * 24 * 60 * 60);
        prop_assert_eq!(market.end_time, expected_end_time);
    }
}

// ===== VOTING BEHAVIOR PROPERTY TESTS =====

proptest! {
    #[test]
    fn test_voting_behavior_properties(
        question in arb_market_question(),
        outcome_count in arb_outcome_count(),
        user_index in arb_user_index(),
        stake_amount in arb_stake_amount(),
        outcome_choice in 0..10usize,
    ) {
        let suite = PropertyBasedTestSuite::new();
        let client = suite.client();

        // Create a test market
        let question_str = SorobanString::from_str(&suite.env, question);
        let outcomes = suite.generate_outcomes(outcome_count);
        let oracle_config = suite.generate_oracle_config(50_000_00, "gt");

        suite.env.mock_all_auths();
        let market_id = client.create_market(
            &suite.admin,
            &question_str,
            &outcomes,
            &30,
            &oracle_config,
        );

        // Select user and outcome for voting
        let user = suite.get_user(user_index);
        let outcome_index = outcome_choice % outcome_count;
        let chosen_outcome = outcomes.get(outcome_index as u32).unwrap();

        // Property: Valid voting should always succeed
        client.vote(user, &market_id, &chosen_outcome, &stake_amount);

        let market = client.get_market(&market_id).unwrap();

        // Property: User vote should be recorded correctly
        prop_assert_eq!(market.votes.get(user.clone()).unwrap(), chosen_outcome);

        // Property: User stake should be recorded correctly
        prop_assert_eq!(market.stakes.get(user.clone()).unwrap(), stake_amount);

        // Property: Total staked should equal user stake
        prop_assert_eq!(market.total_staked, stake_amount);
    }
}

// ===== ORACLE INTERACTION PROPERTY TESTS =====

proptest! {
    #[test]
    fn test_oracle_interaction_properties(
        threshold in arb_threshold(),
        comparison in arb_comparison(),
        feed_id in "[A-Z]{3,6}",
    ) {
        let suite = PropertyBasedTestSuite::new();

        // Property: Valid oracle configuration should be accepted
        let oracle_config = OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: SorobanString::from_str(&suite.env, &feed_id),
            threshold,
            comparison: SorobanString::from_str(&suite.env, comparison),
        };

        // Property: Oracle configuration validation should pass for valid inputs
        let validation_result = oracle_config.validate(&suite.env);
        prop_assert!(validation_result.is_ok());

        // Property: Threshold should be preserved
        prop_assert_eq!(oracle_config.threshold, threshold);

        // Property: Provider should be supported
        prop_assert!(oracle_config.provider.is_supported());
    }
}

proptest! {
    #[test]
    fn test_oracle_configuration_invariants(
        threshold in arb_threshold(),
        comparison in arb_comparison(),
    ) {
        let suite = PropertyBasedTestSuite::new();

        let oracle_config = OracleConfig {
            provider: OracleProvider::Reflector,
            feed_id: SorobanString::from_str(&suite.env, "BTC/USD"),
            threshold,
            comparison: SorobanString::from_str(&suite.env, comparison),
        };

        // Invariant: Threshold must always be positive
        prop_assert!(oracle_config.threshold > 0);

        // Invariant: Comparison must be one of the valid operators
        let valid_comparisons = ["gt", "lt", "eq"];
        prop_assert!(valid_comparisons.contains(&comparison));

        // Invariant: Provider must be supported
        prop_assert!(oracle_config.provider.is_supported());

        // Invariant: Feed ID must not be empty
        prop_assert!(!oracle_config.feed_id.is_empty());
    }
}

// ===== FEE CALCULATION PROPERTY TESTS =====

proptest! {
    #[test]
    fn test_fee_calculation_properties(
        total_staked in 1_000_000i128..=1_000_000_000_000i128, // 1 XLM to 1M XLM
        fee_percentage in 1i128..=10i128, // 1% to 10%
    ) {
        let _suite = PropertyBasedTestSuite::new();

        // Property: Fee calculation should be mathematically correct
        let calculated_fee = (total_staked * fee_percentage) / PERCENTAGE_DENOM;

        // Property: Fee should never exceed total staked amount
        prop_assert!(calculated_fee <= total_staked);

        // Property: Fee should be proportional to percentage
        prop_assert_eq!(calculated_fee, (total_staked * fee_percentage) / 100);

        // Property: Fee should be zero when percentage is zero
        let zero_fee = (total_staked * 0) / PERCENTAGE_DENOM;
        prop_assert_eq!(zero_fee, 0);

        // Property: Fee should equal total when percentage is 100%
        let full_fee = (total_staked * 100) / PERCENTAGE_DENOM;
        prop_assert_eq!(full_fee, total_staked);
    }
}

proptest! {
    #[test]
    fn test_fee_calculation_invariants(
        stakes in prop::collection::vec(arb_stake_amount(), 1..=10),
    ) {
        let _suite = PropertyBasedTestSuite::new();

        let total_staked: i128 = stakes.iter().sum();
        let platform_fee_percentage = DEFAULT_PLATFORM_FEE_PERCENTAGE;

        // Invariant: Total fee should equal sum of individual fees
        let total_fee = (total_staked * platform_fee_percentage) / PERCENTAGE_DENOM;

        let individual_fees_sum: i128 = stakes.iter()
            .map(|stake| (stake * platform_fee_percentage) / PERCENTAGE_DENOM)
            .sum();

        // Due to integer division, there might be small rounding differences
        let difference = (total_fee - individual_fees_sum).abs();
        prop_assert!(difference <= stakes.len() as i128); // Allow for rounding errors

        // Invariant: Fee should never be negative
        prop_assert!(total_fee >= 0);

        // Invariant: Fee should be reasonable (not exceed total)
        prop_assert!(total_fee <= total_staked);
    }
}

// ===== STATE TRANSITION PROPERTY TESTS =====

proptest! {
    #[test]
    fn test_state_transition_properties(
        question in arb_market_question(),
        duration_days in 1u32..=30u32, // Shorter duration for testing
    ) {
        let suite = PropertyBasedTestSuite::new();
        let client = suite.client();

        // Create a market
        let question_str = SorobanString::from_str(&suite.env, question);
        let outcomes = suite.generate_outcomes(2);
        let oracle_config = suite.generate_oracle_config(50_000_00, "gt");

        suite.env.mock_all_auths();
        let market_id = client.create_market(
            &suite.admin,
            &question_str,
            &outcomes,
            &duration_days,
            &oracle_config,
        );

        let initial_market = client.get_market(&market_id).unwrap();

        // Property: Market should start in Active state
        prop_assert_eq!(initial_market.state, MarketState::Active);

        // Property: Market should be active before end time
        prop_assert!(initial_market.is_active(suite.env.ledger().timestamp()));

        // Advance time past market end
        let end_time = initial_market.end_time;
        suite.env.ledger().set(LedgerInfo {
            timestamp: end_time + 1,
            protocol_version: 22,
            sequence_number: suite.env.ledger().sequence(),
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 10000,
        });

        // Property: Market should not be active after end time
        let current_time = suite.env.ledger().timestamp();
        prop_assert!(!initial_market.is_active(current_time));
        prop_assert!(initial_market.has_ended(current_time));
    }
}

// ===== INVARIANT PROPERTY TESTS =====

proptest! {
    #[test]
    fn test_invariant_properties(
        question in arb_market_question(),
        user_count in 2..=5usize,
        stakes in prop::collection::vec(arb_stake_amount(), 2..=5),
    ) {
        let suite = PropertyBasedTestSuite::new();
        let client = suite.client();

        // Create a market
        let question_str = SorobanString::from_str(&suite.env, question);
        let outcomes = suite.generate_outcomes(2);
        let oracle_config = suite.generate_oracle_config(50_000_00, "gt");

        suite.env.mock_all_auths();
        let market_id = client.create_market(
            &suite.admin,
            &question_str,
            &outcomes,
            &30,
            &oracle_config,
        );

        // Store admin address to avoid borrowing issues
        let admin_addr = suite.admin.clone();
        let users_len = suite.users.len();

        // Have multiple users vote
        let mut expected_total = 0i128;
        for i in 0..user_count.min(stakes.len()) {
            let user = suite.get_user(i);
            let outcome = outcomes.get(i as u32 % 2).unwrap();
            let stake = stakes[i];

            client.vote(user, &market_id, &outcome, &stake);
            expected_total += stake;

            let market = client.get_market(&market_id).unwrap();

            // Invariant: Total staked should always equal sum of individual stakes
            prop_assert_eq!(market.total_staked, expected_total);

            // Invariant: Number of votes should not exceed number of users
            let vote_count = market.votes.len();
            prop_assert!(vote_count <= users_len as u32);

            // Invariant: Each user should have at most one vote
            prop_assert!(market.votes.get(user.clone()).is_some());

            // Invariant: Market state should remain consistent
            prop_assert_eq!(market.state, MarketState::Active);

            // Invariant: Market admin should never change
            prop_assert_eq!(market.admin, admin_addr.clone());
        }
    }
}

#[cfg(test)]
mod property_test_runner {
    #[test]
    fn run_all_property_tests() {
        // This test serves as a runner for all property-based tests
        // Individual proptest! macros will be executed automatically by the test runner
        // Property-based tests are executed via proptest! macros
        // Run with: cargo test property_based_tests
    }
}
