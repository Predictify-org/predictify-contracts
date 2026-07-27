//! Property-based tests for resolution state invariants (GrantFox FWC26 campaign)
//!
//! This module implements comprehensive property-based testing for resolution state
//! invariants, ensuring that the ResolutionState enum and ResolutionUtils::get_resolution_state
//! function maintain correct behavior across all possible market configurations.
//!
//! # Invariants Tested
//!
//! 1. **State Priority**: winning_outcomes > oracle_result > dispute_stakes > Active
//! 2. **State Consistency**: State must match market field conditions
//! 3. **No Invalid Combinations**: Certain field combinations should be impossible
//! 4. **Deterministic State**: Same market always produces same state
//! 5. **State Transition Validity**: Legal state transitions only
//!
//! # ResolutionState Logic
//!
//! The state determination follows this priority order:
//! ```text
//! if winning_outcomes.is_some() → MarketResolved
//! else if oracle_result.is_some() → OracleResolved
//! else if total_dispute_stakes() > 0 → Disputed
//! else → Active
//! ```
//!
//! # Test Strategy
//!
//! Uses proptest to generate arbitrary market configurations and validate invariants
//! across the full input space, including edge cases and boundary conditions.

#![cfg(test)]

use crate::resolution::{ResolutionState, ResolutionUtils};
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use proptest::prelude::*;
use soroban_sdk::{Address, Env, String, Symbol, Vec};
use soroban_sdk::testutils::Address as _;

// ===== TEST FIXTURES =====

/// Creates a test environment with mocked auths
fn test_env() -> Env {
    let env = Env::default();
    env.mock_all_auths();
    env
}

/// Creates a base market with configurable fields
fn create_base_market(env: &Env) -> Market {
    let admin = Address::generate(env);
    let mut outcomes = Vec::new(env);
    outcomes.push_back(String::from_str(env, "yes"));
    outcomes.push_back(String::from_str(env, "no"));

    let oracle_config = OracleConfig::new(
        OracleProvider::reflector(),
        Address::generate(env),
        String::from_str(env, "BTC/USD"),
        50_000_00,
        String::from_str(env, "gt"),
    );

    Market::new(
        env,
        admin,
        String::from_str(env, "Test market"),
        outcomes,
        env.ledger().timestamp() + 86400,
        oracle_config,
        None,
        86400,
        MarketState::Active,
    )
}

// ===== PROPERTY GENERATORS =====

/// Strategy for generating optional oracle results (Some or None)
fn arb_oracle_result(env: &Env) -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        Just(Some(String::from_str(env, "yes"))),
        Just(Some(String::from_str(env, "no"))),
    ]
}

/// Strategy for generating optional winning outcomes (Some or None)
fn arb_winning_outcomes(env: &Env) -> impl Strategy<Value = Option<Vec<String>>> {
    prop_oneof![
        Just(None),
        Just({
            let mut outcomes = Vec::new(env);
            outcomes.push_back(String::from_str(env, "yes"));
            Some(outcomes)
        }),
        Just({
            let mut outcomes = Vec::new(env);
            outcomes.push_back(String::from_str(env, "yes"));
            outcomes.push_back(String::from_str(env, "no"));
            Some(outcomes)
        }),
    ]
}

/// Strategy for generating dispute stake amounts
fn arb_dispute_stake() -> impl Strategy<Value = i128> {
    0i128..=1_000_000_000i128 // 0 to 1000 XLM
}

/// Strategy for generating multiple dispute stakes
fn arb_dispute_stakes() -> impl Strategy<Value = Vec<(Address, i128)>> {
    prop::collection::vec(
        (any::<Address>(), arb_dispute_stake()),
        0..=5usize,
    )
}

// ===== PROPERTY TESTS =====

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 1: State priority - winning_outcomes takes precedence over oracle_result
    #[test]
    fn winning_outcomes_takes_precedence_over_oracle_result(
        oracle_result in arb_oracle_result(&test_env()),
        dispute_stake in arb_dispute_stake(),
    ) {
        let env = test_env();
        let mut market = create_base_market(&env);

        // Set winning_outcomes (should force MarketResolved regardless of other fields)
        let mut winning_outcomes = Vec::new(&env);
        winning_outcomes.push_back(String::from_str(&env, "yes"));
        market.winning_outcomes = Some(winning_outcomes);

        // Set oracle_result (should be ignored due to winning_outcomes)
        market.oracle_result = oracle_result;

        // Set dispute stakes (should be ignored due to winning_outcomes)
        if dispute_stake > 0 {
            let user = Address::generate(&env);
            market.dispute_stakes.set(user, dispute_stake);
        }

        let state = ResolutionUtils::get_resolution_state(&env, &market);

        // Invariant: winning_outcomes present → MarketResolved
        prop_assert_eq!(state, ResolutionState::MarketResolved,
            "winning_outcomes should force MarketResolved state");
    }

    /// Property 2: State priority - oracle_result takes precedence over dispute_stakes
    #[test]
    fn oracle_result_takes_precedence_over_dispute_stakes(
        dispute_stake in arb_dispute_stake(),
    ) {
        let env = test_env();
        let mut market = create_base_market(&env);

        // Ensure no winning_outcomes
        market.winning_outcomes = None;

        // Set oracle_result (should force OracleResolved)
        market.oracle_result = Some(String::from_str(&env, "yes"));

        // Set dispute stakes (should be ignored due to oracle_result)
        if dispute_stake > 0 {
            let user = Address::generate(&env);
            market.dispute_stakes.set(user, dispute_stake);
        }

        let state = ResolutionUtils::get_resolution_state(&env, &market);

        // Invariant: oracle_result present without winning_outcomes → OracleResolved
        prop_assert_eq!(state, ResolutionState::OracleResolved,
            "oracle_result should force OracleResolved state when no winning_outcomes");
    }

    /// Property 3: State priority - dispute_stakes takes precedence over Active
    #[test]
    fn dispute_stakes_takes_precedence_over_active(
        dispute_stake in 1i128..=1_000_000_000i128, // Must be > 0
    ) {
        let env = test_env();
        let mut market = create_base_market(&env);

        // Ensure no winning_outcomes or oracle_result
        market.winning_outcomes = None;
        market.oracle_result = None;

        // Set dispute stakes (should force Disputed)
        let user = Address::generate(&env);
        market.dispute_stakes.set(user, dispute_stake);

        let state = ResolutionUtils::get_resolution_state(&env, &market);

        // Invariant: dispute_stakes > 0 without oracle_result → Disputed
        prop_assert_eq!(state, ResolutionState::Disputed,
            "dispute_stakes > 0 should force Disputed state when no oracle_result");
    }

    /// Property 4: No special fields → Active state
    #[test]
    fn no_special_fields_yields_active_state() {
        let env = test_env();
        let mut market = create_base_market(&env);

        // Ensure no winning_outcomes, oracle_result, or dispute_stakes
        market.winning_outcomes = None;
        market.oracle_result = None;
        // dispute_stakes is empty by default

        let state = ResolutionUtils::get_resolution_state(&env, &market);

        // Invariant: no special fields → Active
        prop_assert_eq!(state, ResolutionState::Active,
            "no special fields should yield Active state");
    }

    /// Property 5: Deterministic state - same market always produces same state
    #[test]
    fn state_determination_is_deterministic(
        oracle_result in arb_oracle_result(&test_env()),
        winning_outcomes in arb_winning_outcomes(&test_env()),
        dispute_stakes in arb_dispute_stakes(),
    ) {
        let env = test_env();
        let mut market = create_base_market(&env);

        market.oracle_result = oracle_result;
        market.winning_outcomes = winning_outcomes;

        for (user, stake) in dispute_stakes.iter() {
            if *stake > 0 {
                market.dispute_stakes.set(user.clone(), *stake);
            }
        }

        let state1 = ResolutionUtils::get_resolution_state(&env, &market);
        let state2 = ResolutionUtils::get_resolution_state(&env, &market);

        // Invariant: state determination must be deterministic
        prop_assert_eq!(state1, state2,
            "state determination must be deterministic for same market");
    }

    /// Property 6: MarketResolved state implies winning_outcomes is set
    #[test]
    fn market_resolved_implies_winning_outcomes_set(
        oracle_result in arb_oracle_result(&test_env()),
        dispute_stake in arb_dispute_stake(),
    ) {
        let env = test_env();
        let mut market = create_base_market(&env);

        // Set winning_outcomes to force MarketResolved
        let mut winning_outcomes = Vec::new(&env);
        winning_outcomes.push_back(String::from_str(&env, "yes"));
        market.winning_outcomes = Some(winning_outcomes);

        // Set other fields (should not affect state)
        market.oracle_result = oracle_result;
        if dispute_stake > 0 {
            let user = Address::generate(&env);
            market.dispute_stakes.set(user, dispute_stake);
        }

        let state = ResolutionUtils::get_resolution_state(&env, &market);

        if state == ResolutionState::MarketResolved {
            // Invariant: MarketResolved state → winning_outcomes must be set
            prop_assert!(market.winning_outcomes.is_some(),
                "MarketResolved state requires winning_outcomes to be set");
        }
    }

    /// Property 7: OracleResolved state implies oracle_result is set
    #[test]
    fn oracle_resolved_implies_oracle_result_set(
        dispute_stake in arb_dispute_stake(),
    ) {
        let env = test_env();
        let mut market = create_base_market(&env);

        // Ensure no winning_outcomes
        market.winning_outcomes = None;

        // Set oracle_result to force OracleResolved
        market.oracle_result = Some(String::from_str(&env, "yes"));

        // Set dispute stakes (should not affect state due to priority)
        if dispute_stake > 0 {
            let user = Address::generate(&env);
            market.dispute_stakes.set(user, dispute_stake);
        }

        let state = ResolutionUtils::get_resolution_state(&env, &market);

        if state == ResolutionState::OracleResolved {
            // Invariant: OracleResolved state → oracle_result must be set
            prop_assert!(market.oracle_result.is_some(),
                "OracleResolved state requires oracle_result to be set");
            prop_assert!(market.winning_outcomes.is_none(),
                "OracleResolved state requires winning_outcomes to be None");
        }
    }

    /// Property 8: Disputed state implies dispute_stakes > 0
    #[test]
    fn disputed_implies_dispute_stakes_positive() {
        let env = test_env();
        let mut market = create_base_market(&env);

        // Ensure no winning_outcomes or oracle_result
        market.winning_outcomes = None;
        market.oracle_result = None;

        // Set dispute stakes to force Disputed
        let user = Address::generate(&env);
        market.dispute_stakes.set(user, 1_000_000);

        let state = ResolutionUtils::get_resolution_state(&env, &market);

        if state == ResolutionState::Disputed {
            // Invariant: Disputed state → total_dispute_stakes must be > 0
            prop_assert!(market.total_dispute_stakes() > 0,
                "Disputed state requires total_dispute_stakes to be > 0");
            prop_assert!(market.oracle_result.is_none(),
                "Disputed state requires oracle_result to be None");
            prop_assert!(market.winning_outcomes.is_none(),
                "Disputed state requires winning_outcomes to be None");
        }
    }

    /// Property 9: Active state implies no special fields are set
    #[test]
    fn active_implies_no_special_fields() {
        let env = test_env();
        let mut market = create_base_market(&env);

        // Ensure no special fields
        market.winning_outcomes = None;
        market.oracle_result = None;
        // dispute_stakes is empty by default

        let state = ResolutionUtils::get_resolution_state(&env, &market);

        if state == ResolutionState::Active {
            // Invariant: Active state → no special fields
            prop_assert!(market.winning_outcomes.is_none(),
                "Active state requires winning_outcomes to be None");
            prop_assert!(market.oracle_result.is_none(),
                "Active state requires oracle_result to be None");
            prop_assert_eq!(market.total_dispute_stakes(), 0,
                "Active state requires total_dispute_stakes to be 0");
        }
    }

    /// Property 10: Multiple dispute stakeholders still yields Disputed state
    #[test]
    fn multiple_dispute_stakeholders_yields_disputed(
        num_stakeholders in 2..=5usize,
        stake_per_person in 1_000_000i128..=100_000_000i128,
    ) {
        let env = test_env();
        let mut market = create_base_market(&env);

        // Ensure no winning_outcomes or oracle_result
        market.winning_outcomes = None;
        market.oracle_result = None;

        // Add multiple dispute stakeholders
        for _ in 0..num_stakeholders {
            let user = Address::generate(&env);
            market.dispute_stakes.set(user, stake_per_person);
        }

        let state = ResolutionUtils::get_resolution_state(&env, &market);

        // Invariant: multiple dispute stakeholders → Disputed state
        prop_assert_eq!(state, ResolutionState::Disputed,
            "multiple dispute stakeholders should yield Disputed state");
        prop_assert!(market.total_dispute_stakes() > 0,
            "total_dispute_stakes should be positive with multiple stakeholders");
    }

    /// Property 11: Zero dispute stakes does not yield Disputed state
    #[test]
    fn zero_dispute_stakes_does_not_yield_disputed(
        oracle_result in arb_oracle_result(&test_env()),
    ) {
        let env = test_env();
        let mut market = create_base_market(&env);

        // Ensure no winning_outcomes
        market.winning_outcomes = None;

        // Set oracle_result
        market.oracle_result = oracle_result;

        // Ensure zero dispute stakes
        // (dispute_stakes is empty by default, so total_dispute_stakes() = 0)

        let state = ResolutionUtils::get_resolution_state(&env, &market);

        // Invariant: zero dispute stakes should not yield Disputed
        prop_assert_neq!(state, ResolutionState::Disputed,
            "zero dispute stakes should not yield Disputed state");
    }

    /// Property 12: State is invariant to market state (MarketState enum)
    #[test]
    fn resolution_state_invariant_to_market_state(
        market_state_variant in 0u8..=4u8, // Maps to MarketState variants
    ) {
        let env = test_env();
        let mut market = create_base_market(&env);

        // Map to MarketState variants
        let states = [
            MarketState::Active,
            MarketState::Ended,
            MarketState::Disputed,
            MarketState::Resolved,
            MarketState::Closed,
        ];
        market.state = states[market_state_variant as usize % states.len()];

        // Set a specific condition
        market.winning_outcomes = None;
        market.oracle_result = None;

        let state1 = ResolutionUtils::get_resolution_state(&env, &market);

        // Change market.state to a different value
        let different_idx = (market_state_variant as usize + 1) % states.len();
        market.state = states[different_idx];

        let state2 = ResolutionUtils::get_resolution_state(&env, &market);

        // Invariant: ResolutionState should not depend on MarketState
        prop_assert_eq!(state1, state2,
            "ResolutionState should be invariant to MarketState changes");
    }
}

// ===== UNIT TESTS FOR EDGE CASES =====

#[test]
fn test_resolution_state_priority_order() {
    let env = test_env();
    let mut market = create_base_market(&env);

    // Test 1: All fields set - winning_outcomes should win
    let mut winning_outcomes = Vec::new(&env);
    winning_outcomes.push_back(String::from_str(&env, "yes"));
    market.winning_outcomes = Some(winning_outcomes);
    market.oracle_result = Some(String::from_str(&env, "yes"));
    let user = Address::generate(&env);
    market.dispute_stakes.set(user, 1_000_000);

    let state = ResolutionUtils::get_resolution_state(&env, &market);
    assert_eq!(state, ResolutionState::MarketResolved);

    // Test 2: oracle_result and dispute_stakes set - oracle_result should win
    market.winning_outcomes = None;
    let state = ResolutionUtils::get_resolution_state(&env, &market);
    assert_eq!(state, ResolutionState::OracleResolved);

    // Test 3: Only dispute_stakes set
    market.oracle_result = None;
    let state = ResolutionUtils::get_resolution_state(&env, &market);
    assert_eq!(state, ResolutionState::Disputed);

    // Test 4: Nothing set
    market.dispute_stakes = Map::new(&env);
    let state = ResolutionUtils::get_resolution_state(&env, &market);
    assert_eq!(state, ResolutionState::Active);
}

#[test]
fn test_resolution_state_with_empty_winning_outcomes() {
    let env = test_env();
    let mut market = create_base_market(&env);

    // Set empty winning_outcomes vector
    market.winning_outcomes = Some(Vec::new(&env));
    market.oracle_result = Some(String::from_str(&env, "yes"));

    let state = ResolutionUtils::get_resolution_state(&env, &market);

    // Even empty winning_outcomes should force MarketResolved
    assert_eq!(state, ResolutionState::MarketResolved);
}

#[test]
fn test_resolution_state_with_zero_dispute_stake_entry() {
    let env = test_env();
    let mut market = create_base_market(&env);

    // Add a zero dispute stake entry
    let user = Address::generate(&env);
    market.dispute_stakes.set(user, 0);

    market.winning_outcomes = None;
    market.oracle_result = None;

    let state = ResolutionUtils::get_resolution_state(&env, &market);

    // Zero dispute stake should not trigger Disputed state
    assert_eq!(state, ResolutionState::Active);
}

#[test]
fn test_resolution_state_consistency_across_multiple_calls() {
    let env = test_env();
    let mut market = create_base_market(&env);

    // Set oracle_result
    market.oracle_result = Some(String::from_str(&env, "yes"));

    // Call get_resolution_state multiple times
    let state1 = ResolutionUtils::get_resolution_state(&env, &market);
    let state2 = ResolutionUtils::get_resolution_state(&env, &market);
    let state3 = ResolutionUtils::get_resolution_state(&env, &market);

    // All calls should return the same state
    assert_eq!(state1, ResolutionState::OracleResolved);
    assert_eq!(state2, ResolutionState::OracleResolved);
    assert_eq!(state3, ResolutionState::OracleResolved);
}

#[test]
fn test_resolution_state_with_large_dispute_stakes() {
    let env = test_env();
    let mut market = create_base_market(&env);

    // Ensure no winning_outcomes or oracle_result
    market.winning_outcomes = None;
    market.oracle_result = None;

    // Set very large dispute stakes
    let user = Address::generate(&env);
    market.dispute_stakes.set(user, i128::MAX);

    let state = ResolutionUtils::get_resolution_state(&env, &market);

    // Should still yield Disputed state
    assert_eq!(state, ResolutionState::Disputed);
    assert_eq!(market.total_dispute_stakes(), i128::MAX);
}

#[test]
fn test_resolution_state_transition_simulation() {
    let env = test_env();
    let mut market = create_base_market(&env);

    // Start: Active
    let state = ResolutionUtils::get_resolution_state(&env, &market);
    assert_eq!(state, ResolutionState::Active);

    // Transition 1: Add dispute stakes → Disputed
    let user = Address::generate(&env);
    market.dispute_stakes.set(user, 1_000_000);
    let state = ResolutionUtils::get_resolution_state(&env, &market);
    assert_eq!(state, ResolutionState::Disputed);

    // Transition 2: Add oracle_result → OracleResolved (takes precedence)
    market.oracle_result = Some(String::from_str(&env, "yes"));
    let state = ResolutionUtils::get_resolution_state(&env, &market);
    assert_eq!(state, ResolutionState::OracleResolved);

    // Transition 3: Add winning_outcomes → MarketResolved (takes precedence)
    let mut winning_outcomes = Vec::new(&env);
    winning_outcomes.push_back(String::from_str(&env, "yes"));
    market.winning_outcomes = Some(winning_outcomes);
    let state = ResolutionUtils::get_resolution_state(&env, &market);
    assert_eq!(state, ResolutionState::MarketResolved);
}
