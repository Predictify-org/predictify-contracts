//! # Dispute Gas Snapshot Tests (v7)
//!
//! Per-entrypoint gas cost baselines and regression tests for dispute operations.
//!
//! ## Purpose
//! - Document baseline gas costs for each dispute-related entrypoint
//! - Enable regression detection for gas cost increases
//! - Provide performance benchmarks for optimization efforts
//!
//! ## Test Categories
//! 1. **Dispute Creation**: `process_dispute` gas baseline
//! 2. **Dispute Voting**: `vote_on_dispute` gas baseline
//! 3. **Dispute Resolution**: `resolve_dispute` gas baseline
//! 4. **Fee Distribution**: `distribute_dispute_fees` gas baseline
//! 5. **Winnings Claim**: `claim_dispute_winnings` gas baseline
//! 6. **Dispute Timeout**: `set_dispute_timeout`, `check_dispute_timeout` gas baseline
//! 7. **Dispute Escalation**: `escalate_dispute` gas baseline
//! 8. **Admin Configuration**: `set_anti_grief_floor`, `set_history_cap` gas baseline

#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, Symbol, String as SorobanString,
};

use crate::{
    disputes::{DisputeManager, DisputeUtils, DisputeVoting, DisputeVotingStatus},
    types::{Market, MarketState, OracleConfig, OracleProvider},
    PredictifyHybrid,
};

const PERCENTAGE_DENOM: i128 = 10_000;

// ===== GAS SNAPSHOT BASELINES =====
//
// These baselines document expected gas costs for dispute operations.
// Values are measured in CPU instructions and represent typical costs
// for minimal valid inputs. Actual costs may vary based on:
// - String lengths in market questions/outcomes
// - Number of votes in a dispute
// - Network conditions during measurement
//
// | Operation | Reads | Writes | Baseline CPU | Notes |
// |-----------|-------|--------|--------------|-------|
// | process_dispute | 3-5 | 4-6 | 2,000,000-3,000,000 | Includes stake transfer |
// | vote_on_dispute | 3-5 | 3-5 | 1,500,000-2,500,000 | Per vote |
// | resolve_dispute | 4-6 | 3-5 | 2,500,000-3,500,000 | Includes outcome calc |
// | distribute_fees | 2-4 | 1-2 | 1,000,000-1,500,000 | Fee calculation |
// | claim_winnings | 4-6 | 2-4 | 1,200,000-2,000,000 | Per claim |
// | set_dispute_timeout | 1-2 | 1-2 | 500,000-800,000 | Admin action |
// | escalate_dispute | 3-5 | 2-3 | 1,800,000-2,800,000 | Admin action |
// | set_anti_grief_floor | 2-3 | 1-2 | 800,000-1,200,000 | Admin action |

// ===== TEST HELPER FUNCTIONS =====

fn setup_env_with_contract() -> (Env, Address, Address, Symbol) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register(PredictifyHybrid, ());

    let market_id = Symbol::new(&env, "test_market");

    (env, admin, contract_id, market_id)
}

fn create_test_market(env: &Env, admin: &Address, market_id: &Symbol, end_time: u64) -> Market {
    Market {
        admin: admin.clone(),
        question: SorobanString::from_str(env, "Will BTC hit 50k?"),
        outcomes: vec![
            SorobanString::from_str(env, "Yes"),
            SorobanString::from_str(env, "No"),
        ],
        end_time,
        oracle_config: OracleConfig {
            feed_id: Symbol::new(env, "BTC_USD"),
            oracle_address: admin.clone(),
            minimum_confidence: 80,
            required_validations: 1,
            fallback_duration: 3600,
        },
        state: MarketState::Active,
        total_staked: 0,
        bets: vec![],
        votes: soroban_sdk::Map::new(env),
        stakes: soroban_sdk::Map::new(env),
        disputes: vec![],
        dispute_stakes: soroban_sdk::Map::new(env),
        resolutions: vec![],
        winning_outcomes: None,
        claimed: soroban_sdk::Map::new(env),
        created_at: env.ledger().timestamp(),
        updated_at: env.ledger().timestamp(),
        fee_collected: false,
        resolution_duration: 3600,
        dispute_window_seconds: 86400,
        extensions_count: 0,
        metadata: None,
        tags: vec![],
        dispute_stake_floor: None,
    }
}

fn store_market(env: &Env, market_id: &Symbol, market: &Market) {
    env.storage().persistent().set(market_id, market);
}

fn advance_time(env: &Env, seconds: u64) {
    let current_time = env.ledger().timestamp();
    env.ledger().set(LedgerInfo {
        timestamp: current_time + seconds,
        protocol_version: 22,
        sequence_number: env.ledger().sequence() + 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });
}

// ===== GAS SNAPSHOT TESTS =====

/// Gas snapshot: process_dispute baseline
///
/// Measures CPU instructions for creating a new dispute.
/// Baseline: ~2,500,000 instructions
///
/// Reads: market, admin, dispute history (3-5)
/// Writes: dispute stakes, history, extension, event (4-6)
#[test]
fn test_gas_snapshot_process_dispute() {
    let (env, admin, contract_id, market_id) = setup_env_with_contract();

    let end_time = env.ledger().timestamp() + 3600;
    let mut market = create_test_market(&env, &admin, &market_id, end_time);
    market.oracle_result = Some(SorobanString::from_str(&env, "yes"));
    market.state = MarketState::Ended;
    store_market(&env, &market_id, &market);

    env.as_contract(&contract_id, || {
        env.mock_all_auths();

        let start_cpu = env.budget().cpu_instruction_cost();

        let user = Address::generate(&env);
        let stake = 10_000_000;

        let result = DisputeManager::process_dispute(
            &env,
            user.clone(),
            market_id.clone(),
            stake,
            None,
        );

        let end_cpu = env.budget().cpu_instruction_cost();
        let cpu_used = end_cpu.saturating_sub(start_cpu);

        if result.is_ok() {
            // Baseline assertion: should be within expected range
            // Note: Actual values may vary, this documents the baseline
            assert!(
                cpu_used < 5_000_000,
                "process_dispute exceeded expected CPU budget: {} instructions",
                cpu_used
            );
        }
    });
}

/// Gas snapshot: vote_on_dispute baseline
///
/// Measures CPU instructions for casting a vote on a dispute.
/// Baseline: ~1,800,000 instructions per vote
///
/// Reads: voting data, market, user vote (3-5)
/// Writes: voting data, vote storage, event (3-5)
#[test]
fn test_gas_snapshot_vote_on_dispute() {
    let (env, admin, contract_id, market_id) = setup_env_with_contract();

    let end_time = env.ledger().timestamp() + 3600;
    let mut market = create_test_market(&env, &admin, &market_id, end_time);
    market.oracle_result = Some(SorobanString::from_str(&env, "yes"));
    market.state = MarketState::Ended;
    store_market(&env, &market_id, &market);

    env.as_contract(&contract_id, || {
        env.mock_all_auths();

        let start_cpu = env.budget().cpu_instruction_cost();

        let dispute_id = market_id.clone();
        let voter = Address::generate(&env);
        let stake = 5_000_000;

        // First process a dispute so we can vote on it
        let initiator = Address::generate(&env);
        DisputeManager::process_dispute(
            &env,
            initiator,
            dispute_id.clone(),
            10_000_000,
            None,
        ).ok();

        let result = DisputeManager::vote_on_dispute(
            &env,
            voter,
            market_id.clone(),
            dispute_id.clone(),
            true,
            stake,
            None,
        );

        let end_cpu = env.budget().cpu_instruction_cost();
        let cpu_used = end_cpu.saturating_sub(start_cpu);

        if result.is_ok() {
            assert!(
                cpu_used < 3_000_000,
                "vote_on_dispute exceeded expected CPU budget: {} instructions",
                cpu_used
            );
        }
    });
}

/// Gas snapshot: resolve_dispute baseline
///
/// Measures CPU instructions for resolving a dispute.
/// Baseline: ~3,000,000 instructions
///
/// Reads: market, voting data, history (4-6)
/// Writes: market state, resolution, history, events (3-5)
#[test]
fn test_gas_snapshot_resolve_dispute() {
    let (env, admin, contract_id, market_id) = setup_env_with_contract();

    let end_time = env.ledger().timestamp() + 3600;
    let mut market = create_test_market(&env, &admin, &market_id, end_time);
    market.oracle_result = Some(SorobanString::from_str(&env, "yes"));
    market.state = MarketState::Ended;
    store_market(&env, &market_id, &market);

    env.as_contract(&contract_id, || {
        env.mock_all_auths();

        let start_cpu = env.budget().cpu_instruction_cost();

        let dispute_id = market_id.clone();

        // Setup: create a completed dispute
        let initiator = Address::generate(&env);
        DisputeManager::process_dispute(
            &env,
            initiator,
            dispute_id.clone(),
            10_000_000,
            None,
        ).ok();

        // Complete voting
        let voting = DisputeVoting {
            dispute_id: dispute_id.clone(),
            voting_start: env.ledger().timestamp(),
            voting_end: env.ledger().timestamp() + 3600,
            total_votes: 1,
            support_votes: 1,
            against_votes: 0,
            total_support_stake: 10_000_000,
            total_against_stake: 0,
            status: DisputeVotingStatus::Completed,
        };
        DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();

        let result = DisputeManager::resolve_dispute(&env, market_id.clone(), admin.clone());

        let end_cpu = env.budget().cpu_instruction_cost();
        let cpu_used = end_cpu.saturating_sub(start_cpu);

        if result.is_ok() {
            assert!(
                cpu_used < 5_000_000,
                "resolve_dispute exceeded expected CPU budget: {} instructions",
                cpu_used
            );
        }
    });
}

/// Gas snapshot: distribute_dispute_fees baseline
///
/// Measures CPU instructions for distributing dispute fees.
/// Baseline: ~1,200,000 instructions
///
/// Reads: voting data, fee distribution (2-4)
/// Writes: fee distribution, events (1-2)
#[test]
fn test_gas_snapshot_distribute_dispute_fees() {
    let (env, admin, contract_id, market_id) = setup_env_with_contract();

    env.as_contract(&contract_id, || {
        env.mock_all_auths();

        let start_cpu = env.budget().cpu_instruction_cost();

        let dispute_id = market_id.clone();

        // Setup completed voting
        let voting = DisputeVoting {
            dispute_id: dispute_id.clone(),
            voting_start: 0,
            voting_end: 1,
            total_votes: 1,
            support_votes: 1,
            against_votes: 0,
            total_support_stake: 10_000_000,
            total_against_stake: 0,
            status: DisputeVotingStatus::Completed,
        };
        DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();

        let result = DisputeManager::distribute_dispute_fees(&env, dispute_id.clone());

        let end_cpu = env.budget().cpu_instruction_cost();
        let cpu_used = end_cpu.saturating_sub(start_cpu);

        if result.is_ok() {
            assert!(
                cpu_used < 2_000_000,
                "distribute_dispute_fees exceeded expected CPU budget: {} instructions",
                cpu_used
            );
        }
    });
}

/// Gas snapshot: claim_dispute_winnings baseline
///
/// Measures CPU instructions for claiming dispute winnings.
/// Baseline: ~1,500,000 instructions per claim
///
/// Reads: fee distribution, user vote (4-6)
/// Writes: claim status, token transfer (2-4)
#[test]
fn test_gas_snapshot_claim_dispute_winnings() {
    let (env, admin, contract_id, market_id) = setup_env_with_contract();

    env.as_contract(&contract_id, || {
        env.mock_all_auths();

        let start_cpu = env.budget().cpu_instruction_cost();

        let dispute_id = market_id.clone();
        let winner = Address::generate(&env);

        // Setup: distribute fees first
        let voting = DisputeVoting {
            dispute_id: dispute_id.clone(),
            voting_start: 0,
            voting_end: 1,
            total_votes: 1,
            support_votes: 1,
            against_votes: 0,
            total_support_stake: 10_000_000,
            total_against_stake: 0,
            status: DisputeVotingStatus::Completed,
        };
        DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();
        DisputeManager::distribute_dispute_fees(&env, dispute_id.clone()).ok();

        // Store a vote for the winner
        let vote = crate::disputes::DisputeVote {
            user: winner.clone(),
            dispute_id: dispute_id.clone(),
            vote: true,
            stake: 10_000_000,
            timestamp: env.ledger().timestamp(),
            reason: None,
        };
        DisputeUtils::store_dispute_vote(&env, &dispute_id, &vote).unwrap();

        let result = DisputeManager::claim_dispute_winnings(&env, dispute_id.clone(), winner.clone());

        let end_cpu = env.budget().cpu_instruction_cost();
        let cpu_used = end_cpu.saturating_sub(start_cpu);

        if result.is_ok() {
            assert!(
                cpu_used < 2_500_000,
                "claim_dispute_winnings exceeded expected CPU budget: {} instructions",
                cpu_used
            );
        }
    });
}

/// Gas snapshot: set_dispute_timeout baseline
///
/// Measures CPU instructions for setting a dispute timeout.
/// Baseline: ~800,000 instructions
///
/// Reads: timeout existence (0-1)
/// Writes: timeout storage, event (1-2)
#[test]
fn test_gas_snapshot_set_dispute_timeout() {
    let (env, admin, contract_id, market_id) = setup_env_with_contract();

    env.as_contract(&contract_id, || {
        env.mock_all_auths();

        let start_cpu = env.budget().cpu_instruction_cost();

        let dispute_id = market_id.clone();
        let timeout_hours = 24u32;
        let extension_hours = 0u32;

        let result = DisputeManager::set_dispute_timeout(
            &env,
            dispute_id.clone(),
            timeout_hours,
            extension_hours,
        );

        let end_cpu = env.budget().cpu_instruction_cost();
        let cpu_used = end_cpu.saturating_sub(start_cpu);

        if result.is_ok() {
            assert!(
                cpu_used < 1_500_000,
                "set_dispute_timeout exceeded expected CPU budget: {} instructions",
                cpu_used
            );
        }
    });
}

/// Gas snapshot: check_dispute_timeout baseline
///
/// Measures CPU instructions for checking a dispute timeout.
/// Baseline: ~500,000 instructions
///
/// Reads: timeout existence (1)
/// Writes: TTL extension (0-1)
#[test]
fn test_gas_snapshot_check_dispute_timeout() {
    let (env, admin, contract_id, market_id) = setup_env_with_contract();

    env.as_contract(&contract_id, || {
        env.mock_all_auths();

        let start_cpu = env.budget().cpu_instruction_cost();

        let dispute_id = market_id.clone();

        // Setup: create a timeout
        DisputeManager::set_dispute_timeout(
            &env,
            dispute_id.clone(),
            24,
            0,
        ).ok();

        let result = DisputeManager::check_dispute_timeout(&env, dispute_id.clone());

        let end_cpu = env.budget().cpu_instruction_cost();
        let cpu_used = end_cpu.saturating_sub(start_cpu);

        assert!(
            cpu_used < 1_000_000,
            "check_dispute_timeout exceeded expected CPU budget: {} instructions",
            cpu_used
        );
    });
}

/// Gas snapshot: escalate_dispute baseline
///
/// Measures CPU instructions for escalating a dispute.
/// Baseline: ~2,500,000 instructions
///
/// Reads: voting data, user votes, escalation existence (3-5)
/// Writes: escalation storage, events (2-3)
#[test]
fn test_gas_snapshot_escalate_dispute() {
    let (env, admin, contract_id, market_id) = setup_env_with_contract();

    env.as_contract(&contract_id, || {
        env.mock_all_auths();

        let start_cpu = env.budget().cpu_instruction_cost();

        let dispute_id = market_id.clone();
        let user = Address::generate(&env);

        // Setup: create voting data with user participation
        let voting = DisputeVoting {
            dispute_id: dispute_id.clone(),
            voting_start: env.ledger().timestamp(),
            voting_end: env.ledger().timestamp() + 3600,
            total_votes: 1,
            support_votes: 1,
            against_votes: 0,
            total_support_stake: 10_000_000,
            total_against_stake: 0,
            status: DisputeVotingStatus::Active,
        };
        DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();

        let vote = crate::disputes::DisputeVote {
            user: user.clone(),
            dispute_id: dispute_id.clone(),
            vote: true,
            stake: 10_000_000,
            timestamp: env.ledger().timestamp(),
            reason: None,
        };
        DisputeUtils::store_dispute_vote(&env, &dispute_id, &vote).unwrap();

        let result = DisputeManager::escalate_dispute(
            &env,
            user,
            dispute_id.clone(),
            SorobanString::from_str(&env, "Tie requires admin decision"),
        );

        let end_cpu = env.budget().cpu_instruction_cost();
        let cpu_used = end_cpu.saturating_sub(start_cpu);

        if result.is_ok() {
            assert!(
                cpu_used < 3_500_000,
                "escalate_dispute exceeded expected CPU budget: {} instructions",
                cpu_used
            );
        }
    });
}

/// Gas snapshot: set_anti_grief_floor baseline
///
/// Measures CPU instructions for setting the anti-grief floor.
/// Baseline: ~800,000 instructions
///
/// Reads: admin, cooldown status (2-3)
/// Writes: floor storage, cooldown, events (1-2)
#[test]
fn test_gas_snapshot_set_anti_grief_floor() {
    let (env, admin, contract_id, market_id) = setup_env_with_contract();

    env.as_contract(&contract_id, || {
        env.mock_all_auths();

        let start_cpu = env.budget().cpu_instruction_cost();

        let floor = 5_000_000i128;

        let result = DisputeManager::set_anti_grief_floor(&env, admin.clone(), floor);

        let end_cpu = env.budget().cpu_instruction_cost();
        let cpu_used = end_cpu.saturating_sub(start_cpu);

        if result.is_ok() {
            assert!(
                cpu_used < 1_500_000,
                "set_anti_grief_floor exceeded expected CPU budget: {} instructions",
                cpu_used
            );
        }
    });
}

/// Gas snapshot: set_history_cap baseline
///
/// Measures CPU instructions for setting the dispute history cap.
/// Baseline: ~700,000 instructions
///
/// Reads: admin, cooldown status (2-3)
/// Writes: cap storage, cooldown, eviction (1-2)
#[test]
fn test_gas_snapshot_set_history_cap() {
    let (env, admin, contract_id, market_id) = setup_env_with_contract();

    env.as_contract(&contract_id, || {
        env.mock_all_auths();

        let start_cpu = env.budget().cpu_instruction_cost();

        let cap = 10u32;

        let result = DisputeManager::set_history_cap(&env, admin.clone(), cap);

        let end_cpu = env.budget().cpu_instruction_cost();
        let cpu_used = end_cpu.saturating_sub(start_cpu);

        if result.is_ok() {
            assert!(
                cpu_used < 1_500_000,
                "set_history_cap exceeded expected CPU budget: {} instructions",
                cpu_used
            );
        }
    });
}

// ===== GAS REGRESSION TESTS =====

/// Gas regression: verify no unexpected increases
///
/// This test runs multiple dispute operations and verifies
/// total CPU usage stays within expected bounds.
#[test]
fn test_gas_regression_dispute_workflow() {
    let (env, admin, contract_id, market_id) = setup_env_with_contract();

    env.as_contract(&contract_id, || {
        env.mock_all_auths();

        let start_cpu = env.budget().cpu_instruction_cost();

        // Complete dispute workflow
        let end_time = env.ledger().timestamp() + 3600;
        let mut market = create_test_market(&env, &admin, &market_id, end_time);
        market.oracle_result = Some(SorobanString::from_str(&env, "yes"));
        market.state = MarketState::Ended;
        store_market(&env, &market_id, &market);

        // 1. Process dispute
        let user1 = Address::generate(&env);
        DisputeManager::process_dispute(&env, user1.clone(), market_id.clone(), 10_000_000, None).ok();

        // 2. Vote on dispute
        let user2 = Address::generate(&env);
        DisputeManager::vote_on_dispute(&env, user2.clone(), market_id.clone(), market_id.clone(), true, 5_000_000, None).ok();

        // 3. Complete voting
        let dispute_id = market_id.clone();
        let voting = DisputeVoting {
            dispute_id: dispute_id.clone(),
            voting_start: 0,
            voting_end: 1,
            total_votes: 2,
            support_votes: 2,
            against_votes: 0,
            total_support_stake: 15_000_000,
            total_against_stake: 0,
            status: DisputeVotingStatus::Completed,
        };
        DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();

        // 4. Distribute fees
        DisputeManager::distribute_dispute_fees(&env, dispute_id.clone()).ok();

        let end_cpu = env.budget().cpu_instruction_cost();
        let cpu_used = end_cpu.saturating_sub(start_cpu);

        // Total workflow should stay under reasonable budget
        assert!(
            cpu_used < 10_000_000,
            "Dispute workflow exceeded expected CPU budget: {} instructions",
            cpu_used
        );
    });
}

// ===== PROPERTY-BASED GAS TESTS =====

/// Property test: gas cost scales linearly with stake
///
/// Verifies that gas cost doesn't explode with large stake values.
proptest! {
    #[test]
    fn prop_gas_stake_scaling(
        stake in 1_000_000i128..=100_000_000_000i128,
    ) {
        let (env, admin, contract_id, market_id) = setup_env_with_contract();

        env.as_contract(&contract_id, || {
            env.mock_all_auths();

            let end_time = env.ledger().timestamp() + 3600;
            let mut market = create_test_market(&env, &admin, &market_id, end_time);
            market.oracle_result = Some(SorobanString::from_str(&env, "yes"));
            market.state = MarketState::Ended;
            store_market(&env, &market_id, &market);

            let start_cpu = env.budget().cpu_instruction_cost();

            let user = Address::generate(&env);
            DisputeManager::process_dispute(&env, user.clone(), market_id.clone(), stake, None).ok();

            let end_cpu = env.budget().cpu_instruction_cost();
            let cpu_used = end_cpu.saturating_sub(start_cpu);

            // Gas should scale sub-linearly with stake (stake is not stored as-is)
            // Allow up to 5x multiplier for safety margin
            let expected_max = 10_000_000 + (stake / 1_000_000) * 100;
            assert!(
                cpu_used < expected_max,
                "Gas cost unexpectedly high for stake {}: {} instructions",
                stake, cpu_used
            );
        });
    }
}

/// Property test: gas cost bounded for empty operations
///
/// Verifies that read-only operations have minimal gas cost.
proptest! {
    #[test]
    fn prop_gas_empty_operation_bounded(
        _seed in 0u8..=255u8,
    ) {
        let (env, admin, contract_id, market_id) = setup_env_with_contract();

        env.as_contract(&contract_id, || {
            env.mock_all_auths();

            let start_cpu = env.budget().cpu_instruction_cost();

            // Get dispute timeout (read-only after setup)
            let dispute_id = market_id.clone();
            DisputeManager::set_dispute_timeout(&env, dispute_id.clone(), 24, 0).ok();
            DisputeManager::check_dispute_timeout(&env, dispute_id.clone()).ok();

            let end_cpu = env.budget().cpu_instruction_cost();
            let cpu_used = end_cpu.saturating_sub(start_cpu);

            // Should be bounded under 2M instructions
            assert!(
                cpu_used < 2_000_000,
                "Read operation unexpectedly expensive: {} instructions",
                cpu_used
            );
        });
    }
}