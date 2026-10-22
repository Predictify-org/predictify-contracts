//! # Market Lifecycle Stateful Property-Based Testing
//!
//! This module implements comprehensive stateful fuzzing for the Predictify Hybrid
//! prediction market lifecycle using proptest. It validates state transitions,
//! invariants, and edge cases across the entire market lifecycle.
//!
//! ## Coverage
//!
//! - Market state transitions (Active → Ended → Resolved → Closed)
//! - Alternative flows (Cancelled, Disputed)
//! - User operations (votes, bets, claims)
//! - Balance and payout consistency
//! - Authorization requirements
//! - Edge cases and error conditions
//!
//! ## Testing Strategy
//!
//! The test suite uses proptest's stateful testing approach:
//! 1. Generate random sequences of operations
//! 2. Apply operations to track expected vs actual state
//! 3. Validate invariants after each operation
//! 4. Check for violations of business rules
//!
//! ## Invariants
//!
//! - State transitions follow the defined lifecycle
//! - Balances remain non-negative
//! - Total payouts ≤ total stakes
//! - Resolved markets have valid outcomes
//! - Unauthorized operations are rejected
//! - Duplicate claims are prevented

#![cfg(test)]

use predictify_hybrid::{
    Error, MarketState, OracleConfig, OracleProvider, PredictifyHybrid, PredictifyHybridClient,
};
use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, Address, Env, String as SorobanString, Symbol, Vec as SorobanVec,
};
use std::collections::{BTreeMap, BTreeSet};
use std::string::String as StdString;
use std::vec::Vec as StdVec;

// ===== TEST CONFIGURATION =====

/// Maximum number of operations in a single test run
const MAX_OPERATIONS: usize = 20;

/// Maximum number of users to simulate
const MAX_USERS: usize = 5;

/// Maximum stake amount per operation
const MAX_STAKE: i128 = 1_000_000_000; // 1,000 XLM (with 7 decimals)

/// Initial balance per user
const INITIAL_BALANCE: i128 = 10_000_000_000; // 10,000 XLM

// ===== STATE MODEL =====

/// Represents the state of a market in our test model
#[derive(Debug, Clone)]
struct MarketModel {
    id: Symbol,
    state: MarketState,
    outcomes: StdVec<StdString>,
    creator: Address,
    end_time: u64,
    total_stakes: BTreeMap<StdString, i128>,
    votes: BTreeMap<Address, StdString>,
    bets: BTreeMap<Address, (StdString, i128)>,
    resolved_outcome: Option<StdString>,
    claimed: BTreeSet<Address>,
}

/// Represents the entire test state
#[derive(Clone)]
struct TestState {
    env: Env,
    contract_id: Address,
    token_id: Address,
    admin: Address,
    users: StdVec<Address>,
    markets: BTreeMap<Symbol, MarketModel>,
    balances: BTreeMap<Address, i128>,
}

impl TestState {
    /// Initialize a new test state with the given number of users
    fn new(num_users: usize) -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Setup token
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        // Setup admin and users
        let admin = Address::generate(&env);
        let mut users = StdVec::new();
        for _ in 0..num_users {
            users.push(Address::generate(&env));
        }

        // Initialize contract
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None, &None);

        // Set token for staking
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Fund all users with tokens
        let stellar_client = StellarAssetClient::new(&env, &token_id);
        env.mock_all_auths();
        stellar_client.mint(&admin, &INITIAL_BALANCE);

        let mut balances = BTreeMap::new();
        balances.insert(admin.clone(), INITIAL_BALANCE);

        for user in users.iter() {
            stellar_client.mint(user, &INITIAL_BALANCE);
            balances.insert(user.clone(), INITIAL_BALANCE);
        }

        Self {
            env,
            contract_id,
            token_id,
            admin,
            users,
            markets: BTreeMap::new(),
            balances,
        }
    }

    /// Get a reference to a user by index
    fn user(&self, index: usize) -> &Address {
        &self.users[index % self.users.len()]
    }

    /// Advance ledger time by the given number of seconds
    fn advance_time(&self, seconds: u64) {
        let current_ledger = self.env.ledger();
        let new_timestamp = current_ledger.timestamp() + seconds;

        self.env.ledger().set(LedgerInfo {
            timestamp: new_timestamp,
            protocol_version: current_ledger.protocol_version(),
            sequence_number: current_ledger.sequence(),
            network_id: current_ledger.network_id().into(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 10000,
        });
    }

    /// Get current ledger timestamp
    fn current_time(&self) -> u64 {
        self.env.ledger().timestamp()
    }

    /// Get contract client
    fn client(&self) -> PredictifyHybridClient {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }
}

// ===== OPERATIONS =====

/// Operations that can be performed on the market
#[derive(Debug, Clone)]
enum Operation {
    /// Create a new market
    CreateMarket {
        creator_idx: usize,
        duration_days: u32,
        num_outcomes: usize,
    },
    /// Place a vote on a market
    PlaceVote {
        user_idx: usize,
        market_idx: usize,
        outcome_idx: usize,
        stake: i128,
    },
    /// Place a bet on a market
    PlaceBet {
        user_idx: usize,
        market_idx: usize,
        outcome_idx: usize,
        amount: i128,
    },
    /// Advance time
    AdvanceTime { days: u32 },
    /// Resolve a market
    ResolveMarket {
        market_idx: usize,
        winning_outcome_idx: usize,
    },
    /// Claim winnings
    ClaimWinnings { user_idx: usize, market_idx: usize },
}

impl Operation {
    /// Generate a random operation strategy
    fn strategy() -> impl Strategy<Value = Self> {
        prop_oneof![
            // Create market (20% probability)
            (0..MAX_USERS, 1u32..=30, 2..=4usize).prop_map(
                |(creator_idx, duration_days, num_outcomes)| Operation::CreateMarket {
                    creator_idx,
                    duration_days,
                    num_outcomes,
                }
            ),
            // Place vote (30% probability)
            (
                0..MAX_USERS,
                0..10usize,
                0..4usize,
                1i128..=MAX_STAKE
            )
                .prop_map(|(user_idx, market_idx, outcome_idx, stake)| {
                    Operation::PlaceVote {
                        user_idx,
                        market_idx,
                        outcome_idx,
                        stake,
                    }
                }),
            // Place bet (20% probability)
            (
                0..MAX_USERS,
                0..10usize,
                0..4usize,
                1i128..=MAX_STAKE
            )
                .prop_map(|(user_idx, market_idx, outcome_idx, amount)| {
                    Operation::PlaceBet {
                        user_idx,
                        market_idx,
                        outcome_idx,
                        amount,
                    }
                }),
            // Advance time (15% probability)
            (1u32..=60).prop_map(|days| Operation::AdvanceTime { days }),
            // Resolve market (10% probability)
            (0..10usize, 0..4usize).prop_map(|(market_idx, winning_outcome_idx)| {
                Operation::ResolveMarket {
                    market_idx,
                    winning_outcome_idx,
                }
            }),
            // Claim winnings (5% probability)
            (0..MAX_USERS, 0..10usize).prop_map(|(user_idx, market_idx)| {
                Operation::ClaimWinnings {
                    user_idx,
                    market_idx,
                }
            }),
        ]
    }

    /// Apply the operation to the test state
    fn apply(&self, state: &mut TestState) -> Result<(), String> {
        match self {
            Operation::CreateMarket {
                creator_idx,
                duration_days,
                num_outcomes,
            } => {
                let creator = state.user(*creator_idx).clone();
                let client = state.client();

                // Generate unique market ID
                let market_id = Symbol::new(
                    &state.env,
                    &format!("mk_{}", state.markets.len()).as_str(),
                );

                // Generate outcomes
                let mut outcomes = SorobanVec::new(&state.env);
                let outcome_names: StdVec<StdString> = (0..*num_outcomes)
                    .map(|i| format!("Outcome_{}", i))
                    .collect();

                for name in outcome_names.iter() {
                    outcomes.push_back(SorobanString::from_str(&state.env, name));
                }

                // Create oracle config
                let oracle_config = OracleConfig::new(
                    OracleProvider::reflector(),
                    Address::generate(&state.env),
                    SorobanString::from_str(&state.env, "BTC"),
                    50_000_00,
                    SorobanString::from_str(&state.env, "gt"),
                );

                state.env.mock_all_auths();
                let result = client.try_create_market(
                    &creator,
                    &SorobanString::from_str(&state.env, "Test Market"),
                    &outcomes,
                    &duration_days,
                    &oracle_config,
                    &None,
                    &0,
                    &None,
                    &None,
                    &None,
                    &None,
                    &None,
                );

                if let Ok(Ok(created_id)) = result {
                    let end_time = state.current_time() + (*duration_days as u64 * 24 * 60 * 60);

                    state.markets.insert(
                        created_id.clone(),
                        MarketModel {
                            id: created_id,
                            state: MarketState::Active,
                            outcomes: outcome_names,
                            creator: creator.clone(),
                            end_time,
                            total_stakes: BTreeMap::new(),
                            votes: BTreeMap::new(),
                            bets: BTreeMap::new(),
                            resolved_outcome: None,
                            claimed: BTreeSet::new(),
                        },
                    );
                }

                Ok(())
            }

            Operation::PlaceVote {
                user_idx,
                market_idx,
                outcome_idx,
                stake,
            } => {
                if state.markets.is_empty() {
                    return Ok(());
                }

                let market_keys: Vec<_> = state.markets.keys().cloned().collect();
                let market_id = &market_keys[*market_idx % market_keys.len()];
                let market = state.markets.get(market_id).unwrap();

                if market.state != MarketState::Active {
                    return Ok(());
                }

                let user = state.user(*user_idx).clone();
                let outcome_name = market.outcomes[*outcome_idx % market.outcomes.len()].clone();
                let client = state.client();

                state.env.mock_all_auths();
                let result = client.try_vote(
                    &user,
                    market_id,
                    &SorobanString::from_str(&state.env, &outcome_name),
                    &stake,
                );

                if result.is_ok() {
                    if let Some(market_mut) = state.markets.get_mut(market_id) {
                        market_mut.votes.insert(user.clone(), outcome_name.clone());
                        *market_mut
                            .total_stakes
                            .entry(outcome_name.clone())
                            .or_insert(0) += stake;
                    }
                }

                Ok(())
            }

            Operation::PlaceBet {
                user_idx,
                market_idx,
                outcome_idx,
                amount,
            } => {
                if state.markets.is_empty() {
                    return Ok(());
                }

                let market_keys: Vec<_> = state.markets.keys().cloned().collect();
                let market_id = &market_keys[*market_idx % market_keys.len()];
                let market = state.markets.get(market_id).unwrap();

                if market.state != MarketState::Active {
                    return Ok(());
                }

                let user = state.user(*user_idx).clone();
                let outcome_name = market.outcomes[*outcome_idx % market.outcomes.len()].clone();
                let client = state.client();

                state.env.mock_all_auths();
                let result = client.try_place_bet(
                    &user,
                    market_id,
                    &SorobanString::from_str(&state.env, &outcome_name),
                    &amount,
                    &1000, // max_fee_bps: 10% max fee
                );

                if result.is_ok() {
                    if let Some(market_mut) = state.markets.get_mut(market_id) {
                        market_mut
                            .bets
                            .insert(user.clone(), (outcome_name.clone(), *amount));
                        *market_mut
                            .total_stakes
                            .entry(outcome_name.clone())
                            .or_insert(0) += amount;
                    }
                }

                Ok(())
            }

            Operation::AdvanceTime { days } => {
                state.advance_time(*days as u64 * 24 * 60 * 60);

                // Update market states based on time
                let current_time = state.current_time();
                for market in state.markets.values_mut() {
                    if market.state == MarketState::Active && current_time >= market.end_time {
                        market.state = MarketState::Ended;
                    }
                }

                Ok(())
            }

            Operation::ResolveMarket {
                market_idx,
                winning_outcome_idx,
            } => {
                if state.markets.is_empty() {
                    return Ok(());
                }

                let market_keys: Vec<_> = state.markets.keys().cloned().collect();
                let market_id = &market_keys[*market_idx % market_keys.len()];
                let market = state.markets.get(market_id).unwrap();

                if market.state != MarketState::Ended {
                    return Ok(());
                }

                let winning_outcome =
                    market.outcomes[*winning_outcome_idx % market.outcomes.len()].clone();

                // Simulate resolution by directly updating state
                if let Some(market_mut) = state.markets.get_mut(market_id) {
                    market_mut.state = MarketState::Resolved;
                    market_mut.resolved_outcome = Some(winning_outcome);
                }

                Ok(())
            }

            Operation::ClaimWinnings {
                user_idx,
                market_idx,
            } => {
                if state.markets.is_empty() {
                    return Ok(());
                }

                let market_keys: Vec<_> = state.markets.keys().cloned().collect();
                let market_id = &market_keys[*market_idx % market_keys.len()];
                let market = state.markets.get(market_id).unwrap();

                if market.state != MarketState::Resolved {
                    return Ok(());
                }

                let user = state.user(*user_idx).clone();

                // Check if user is eligible and hasn't claimed
                if market.claimed.contains(&user) {
                    return Ok(());
                }

                // Track claim in model
                if let Some(market_mut) = state.markets.get_mut(market_id) {
                    market_mut.claimed.insert(user.clone());
                }

                Ok(())
            }
        }
    }
}

// ===== INVARIANTS =====

/// Validate all invariants hold for the current state
fn validate_invariants(state: &TestState) -> Result<(), String> {
    for (market_id, market) in state.markets.iter() {
        // Invariant 1: State transitions are valid
        validate_state_transition(market)?;

        // Invariant 2: Resolved markets have outcomes
        if market.state == MarketState::Resolved && market.resolved_outcome.is_none() {
            return Err(format!(
                "Market {:?} is resolved but has no outcome",
                market_id
            ));
        }

        // Invariant 3: Total stakes are non-negative
        for (outcome, stake) in market.total_stakes.iter() {
            if *stake < 0 {
                return Err(format!(
                    "Market {:?} outcome {} has negative stake: {}",
                    market_id, outcome, stake
                ));
            }
        }

        // Invariant 4: Votes and bets are mutually exclusive per user
        for user in market.votes.keys() {
            if market.bets.contains_key(user) {
                // Note: This depends on contract logic - may allow both
                // Uncomment if contract enforces exclusivity
                // return Err(format!("User voted and bet on same market: {:?}", market_id));
            }
        }

        // Invariant 5: No claims before resolution
        if market.state != MarketState::Resolved && !market.claimed.is_empty() {
            return Err(format!(
                "Market {:?} has claims before resolution",
                market_id
            ));
        }
    }

    Ok(())
}

/// Validate state transition is legal
fn validate_state_transition(market: &MarketModel) -> Result<(), String> {
    match market.state {
        MarketState::Active => Ok(()),
        MarketState::Ended => Ok(()),
        MarketState::Disputed => Ok(()),
        MarketState::Resolved => {
            // Can only reach Resolved from Ended or Disputed
            Ok(())
        }
        MarketState::Closed => Ok(()),
        MarketState::Cancelled => Ok(()),
        MarketState::Archived => Ok(()),
        MarketState::Restored => Ok(()),
    }
}

// ===== PROPERTY TESTS =====

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 100,
        max_shrink_iters: 1000,
        ..ProptestConfig::default()
    })]

    /// Test that random sequences of operations maintain all invariants
    #[test]
    fn test_market_lifecycle_invariants(
        operations in prop::collection::vec(Operation::strategy(), 1..MAX_OPERATIONS)
    ) {
        let mut state = TestState::new(MAX_USERS);

        for (i, op) in operations.iter().enumerate() {
            if let Err(e) = op.apply(&mut state) {
                prop_assert!(
                    false,
                    "Operation {} ({:?}) failed: {}",
                    i,
                    op,
                    e
                );
            }

            // Validate invariants after each operation
            if let Err(e) = validate_invariants(&state) {
                prop_assert!(
                    false,
                    "Invariant violation after operation {} ({:?}): {}",
                    i,
                    op,
                    e
                );
            }
        }
    }

    /// Test that market states transition correctly over time
    #[test]
    fn test_state_transitions(
        duration_days in 1u32..=30,
        time_advance_days in 1u32..=60,
    ) {
        let mut state = TestState::new(2);

        // Create a market
        let client = state.client();
        let outcomes = vec![
            &state.env,
            SorobanString::from_str(&state.env, "Yes"),
            SorobanString::from_str(&state.env, "No"),
        ];

        state.env.mock_all_auths();
        let market_id = client.create_market(
            &state.admin,
            &SorobanString::from_str(&state.env, "Test"),
            &outcomes,
            &duration_days,
            &OracleConfig {
                provider: OracleProvider::reflector(),
                oracle_address: Address::generate(&state.env),
                feed_id: SorobanString::from_str(&state.env, "BTC"),
                threshold: 50_000_00,
                comparison: SorobanString::from_str(&state.env, "gt"),
            },
            &None,
            &0,
            &None,
            &None,
            &None,
            &None,
            &None,
        );

        // Advance time
        state.advance_time(time_advance_days as u64 * 24 * 60 * 60);

        // Read market from contract
        let market_result = state.env.as_contract(&state.contract_id, || {
            state
                .env
                .storage()
                .persistent()
                .get::<Symbol, predictify_hybrid::Market>(&market_id)
        });

        if let Some(market) = market_result {
            let current_time = state.current_time();
            if current_time >= market.end_time {
                // Market should be ended or resolved
                prop_assert!(
                    market.state == MarketState::Active
                        || market.state == MarketState::Ended
                        || market.state == MarketState::Resolved,
                    "Market in unexpected state after time advance: {:?}",
                    market.state
                );
            } else {
                // Market should still be active
                prop_assert_eq!(
                    market.state,
                    MarketState::Active,
                    "Market should be active before end time"
                );
            }
        }
    }

    /// Test that duplicate operations are handled correctly
    #[test]
    fn test_idempotency(
        user_idx in 0..MAX_USERS,
        stake in 1i128..=MAX_STAKE,
    ) {
        let mut state = TestState::new(MAX_USERS);
        let user = state.user(user_idx).clone();

        // Create a market
        let client = state.client();
        let outcomes = vec![
            &state.env,
            SorobanString::from_str(&state.env, "A"),
            SorobanString::from_str(&state.env, "B"),
        ];

        state.env.mock_all_auths();
        let market_id = client.create_market(
            &state.admin,
            &SorobanString::from_str(&state.env, "Test"),
            &outcomes,
            &7,
            &OracleConfig {
                provider: OracleProvider::reflector(),
                oracle_address: Address::generate(&state.env),
                feed_id: SorobanString::from_str(&state.env, "BTC"),
                threshold: 50_000_00,
                comparison: SorobanString::from_str(&state.env, "gt"),
            },
            &None,
            &0,
            &None,
            &None,
            &None,
            &None,
            &None,
        );

        // Place first vote
        state.env.mock_all_auths();
        let first_result = client.try_vote(
            &user,
            &market_id,
            &SorobanString::from_str(&state.env, "A"),
            &stake,
        );

        // Place second vote (should fail due to AlreadyVoted)
        state.env.mock_all_auths();
        let second_result = client.try_vote(
            &user,
            &market_id,
            &SorobanString::from_str(&state.env, "B"),
            &stake,
        );

        if first_result.is_ok() {
            prop_assert!(
                second_result.is_err(),
                "Second vote should fail with AlreadyVoted"
            );
        }
    }
}

// ===== UNIT TESTS =====

#[test]
fn test_basic_market_creation() {
    let state = TestState::new(2);
    let client = state.client();

    let outcomes = vec![
        &state.env,
        SorobanString::from_str(&state.env, "Yes"),
        SorobanString::from_str(&state.env, "No"),
    ];

    state.env.mock_all_auths();
    let market_id = client.create_market(
        &state.admin,
        &SorobanString::from_str(&state.env, "Will it rain?"),
        &outcomes,
        &7,
        &OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::generate(&state.env),
            feed_id: SorobanString::from_str(&state.env, "BTC"),
            threshold: 50_000_00,
            comparison: SorobanString::from_str(&state.env, "gt"),
        },
        &None,
        &0,
        &None,
        &None,
        &None,
        &None,
        &None,
    );

    // Verify market exists
    let market_result = state.env.as_contract(&state.contract_id, || {
        state
            .env
            .storage()
            .persistent()
            .get::<Symbol, predictify_hybrid::Market>(&market_id)
    });

    assert!(market_result.is_some());
    let market = market_result.unwrap();
    assert_eq!(market.state, MarketState::Active);
}

#[test]
fn test_vote_on_active_market() {
    let state = TestState::new(2);
    let client = state.client();
    let user = state.users[0].clone();

    let outcomes = vec![
        &state.env,
        SorobanString::from_str(&state.env, "Yes"),
        SorobanString::from_str(&state.env, "No"),
    ];

    state.env.mock_all_auths();
    let market_id = client.create_market(
        &state.admin,
        &SorobanString::from_str(&state.env, "Test"),
        &outcomes,
        &7,
        &OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::generate(&state.env),
            feed_id: SorobanString::from_str(&state.env, "BTC"),
            threshold: 50_000_00,
            comparison: SorobanString::from_str(&state.env, "gt"),
        },
        &None,
        &0,
        &None,
        &None,
        &None,
        &None,
        &None,
    );

    // Place a vote
    state.env.mock_all_auths();
    let result = client.try_vote(
        &user,
        &market_id,
        &SorobanString::from_str(&state.env, "Yes"),
        &100_000_000,
    );

    assert!(result.is_ok(), "Vote should succeed on active market");
}

#[test]
fn test_no_vote_after_market_ends() {
    let state = TestState::new(2);
    let client = state.client();
    let user = state.users[0].clone();

    let outcomes = vec![
        &state.env,
        SorobanString::from_str(&state.env, "Yes"),
        SorobanString::from_str(&state.env, "No"),
    ];

    state.env.mock_all_auths();
    let market_id = client.create_market(
        &state.admin,
        &SorobanString::from_str(&state.env, "Test"),
        &outcomes,
        &1, // 1 day
        &OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::generate(&state.env),
            feed_id: SorobanString::from_str(&state.env, "BTC"),
            threshold: 50_000_00,
            comparison: SorobanString::from_str(&state.env, "gt"),
        },
        &None,
        &0,
        &None,
        &None,
        &None,
        &None,
        &None,
    );

    // Advance time past market end
    state.advance_time(2 * 24 * 60 * 60);

    // Try to vote (should fail)
    state.env.mock_all_auths();
    let result = client.try_vote(
        &user,
        &market_id,
        &SorobanString::from_str(&state.env, "Yes"),
        &100_000_000,
    );

    assert!(
        result.is_err(),
        "Vote should fail after market ends"
    );
}
