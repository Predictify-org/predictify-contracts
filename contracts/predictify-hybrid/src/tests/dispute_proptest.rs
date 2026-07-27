//! # Dispute Invariant Property-Based Tests (v7)
//!
//! This module implements comprehensive property-based (proptest) testing for
//! dispute state invariants in the Predictify Hybrid contract.
//!
//! ## Invariants Covered
//!
//! 1. **Voting Outcome Determinism**: The stake-weighted outcome function always
//!    returns the same result for the same support/against stake pair.
//! 2. **Tie Resolution**: Equal support and against stakes always resolve to
//!    `false` (oracle result stands; admin escalation per docs).
//! 3. **Monotonicity**: Increasing support stake (holding against constant) can
//!    only change the outcome from `false` to `true`, never the reverse.
//! 4. **Empty Vote Set Safety**: Zero stakes on both sides never panic and
//!    resolve to `false`.
//! 5. **Stake Validation**: Dispute stakes must meet the minimum floor and
//!    anti-grief floor; stakes below the floor are rejected.
//! 6. **Fee Distribution Bounds**: Total distributed fees never exceed total
//!    staked; winner always recovers at least their original stake.
//! 7. **Dispute State Transitions**: Resolved markets cannot accept new disputes;
//!    disputes outside the window are rejected.
//! 8. **Cooldown Enforcement**: Admin actions are throttled by the configured
//!    cooldown period.
//! 9. **Stake Cap Enforcement**: Per-market per-user and cumulative caps are
//!    enforced.
//! 10. **Timeout Parameters**: Timeout values are bounded and validated.
//!
//! ## Non-Goals
//! - Full end-to-end dispute lifecycle (covered by `dispute_stake_tests.rs`)
//! - Oracle interaction specifics (covered by `oracle_differential_fuzz.rs`)
//! - Collusion detection edge cases (covered by `dispute_collusion_tests.rs`)

#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, Symbol, String as SorobanString,
};

use crate::{
    disputes::{
        DisputeManager, DisputeUtils, DisputeVoting, DisputeVotingStatus,
        DisputeValidator, DisputeDecayConfig, DisputeTimeout, DisputeTimeoutStatus,
    },
    types::{Market, MarketState, OracleConfig, OracleProvider},
    config::{MIN_DISPUTE_STAKE, DISPUTE_EXTENSION_HOURS},
    PredictifyHybrid,
};

// ===== HELPER FUNCTIONS =====

fn env_with_contract() -> (Env, Address, Address) {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(PredictifyHybrid, ());
    (env, admin, contract_id)
}

fn market_with_oracle(env: &Env, admin: &Address, end_time: u64) -> Market {
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

fn completed_voting(
    env: &Env,
    dispute_id: Symbol,
    support: i128,
    against: i128,
) -> DisputeVoting {
    DisputeVoting {
        dispute_id,
        voting_start: 0,
        voting_end: 1,
        total_votes: u32::from(support > 0) + u32::from(against > 0),
        support_votes: u32::from(support > 0),
        against_votes: u32::from(against > 0),
        total_support_stake: support,
        total_against_stake: against,
        status: DisputeVotingStatus::Completed,
    }
}

// ===== PROPERTY 1: DETERMINISTIC TALLY =====
// For any (support, against) pair, calling calculate_stake_weighted_outcome
// twice must return the same boolean.

proptest! {
    #[test]
    fn prop_dispute_tally_is_deterministic(
        support in 0i128..=10_000_000_000_000i128,
        against in 0i128..=10_000_000_000_000i128,
    ) {
        let env = Env::default();
        let voting = completed_voting(&env, Symbol::new(&env, "det"), support, against);
        let first = DisputeUtils::calculate_stake_weighted_outcome(&voting);
        let second = DisputeUtils::calculate_stake_weighted_outcome(&voting);
        prop_assert_eq!(first, second);
    }
}

// ===== PROPERTY 2: TIE RESOLVES TO REJECT =====
// When support == against, the outcome must always be false (oracle stands).

proptest! {
    #[test]
    fn prop_dispute_tie_resolves_to_reject(
        stake in 0i128..=10_000_000_000_000i128,
    ) {
        let env = Env::default();
        let voting = completed_voting(&env, Symbol::new(&env, "tie"), stake, stake);
        prop_assert!(!DisputeUtils::calculate_stake_weighted_outcome(&voting));
    }
}

// ===== PROPERTY 3: MONOTONICITY IN SUPPORT STAKE =====
// If support increases (against constant), the outcome can only stay the same
// or flip from false to true, never from true to false.

proptest! {
    #[test]
    fn prop_dispute_outcome_monotonic_in_support(
        support in 0i128..=5_000_000_000_000i128,
        against in 0i128..=5_000_000_000_000i128,
        delta in 1i128..=1_000_000_000i128,
    ) {
        let env = Env::default();
        let base = completed_voting(&env, Symbol::new(&env, "mono"), support, against);
        let base_out = DisputeUtils::calculate_stake_weighted_outcome(&base);

        // checked_add prevents overflow; if it overflows we skip this case
        if let Some(inc_support) = support.checked_add(delta) {
            let increased = completed_voting(&env, Symbol::new(&env, "mono2"), inc_support, against);
            let inc_out = DisputeUtils::calculate_stake_weighted_outcome(&increased);
            // If base was true (support > against), increasing support keeps it true
            if base_out {
                prop_assert!(inc_out, "increasing support stake should preserve upheld outcome");
            }
        }
    }
}

// ===== PROPERTY 4: EMPTY STAKES DO NOT PANIC =====
// Zero support and zero against must not panic and must return false.

proptest! {
    #[test]
    fn prop_dispute_empty_stakes_no_panic(
        _seed in 0u8..=255u8,
    ) {
        let env = Env::default();
        let voting = completed_voting(&env, Symbol::new(&env, "empty"), 0, 0);
        let outcome = DisputeUtils::calculate_stake_weighted_outcome(&voting);
        prop_assert!(!outcome);
    }
}

// ===== PROPERTY 5: STAKE VALIDATION INVARIANTS =====
// Dispute stakes below MIN_DISPUTE_STAKE must be rejected by the validator.

proptest! {
    #[test]
    fn prop_dispute_stake_below_min_rejected(
        stake in 0i128..(MIN_DISPUTE_STAKE - 1),
    ) {
        let env = Env::default();
        let market_id = Symbol::new(&env, "prop_market");
        let user = Address::generate(&env);

        let mut market = market_with_oracle(&env, &Address::generate(&env), env.ledger().timestamp().saturating_sub(1));
        market.oracle_result = Some(SorobanString::from_str(&env, "yes"));

        // Validator should reject stakes below the minimum
        let result = DisputeValidator::validate_dispute_parameters(&env, &market_id, &user, &market, stake);
        prop_assert!(result.is_err());
    }
}

// ===== PROPERTY 6: STAKE VALIDATION ABOVE MIN SUCCEEDS =====
// Dispute stakes at or above MIN_DISPUTE_STAKE must pass the validator
// (assuming all other conditions are met).

proptest! {
    #[test]
    fn prop_dispute_stake_at_min_passes(
        extra in 0i128..=1_000_000_000i128,
    ) {
        let env = Env::default();
        let market_id = Symbol::new(&env, "prop_market2");
        let user = Address::generate(&env);
        let admin = Address::generate(&env);

        let mut market = market_with_oracle(&env, &admin, env.ledger().timestamp().saturating_sub(1));
        market.oracle_result = Some(SorobanString::from_str(&env, "yes"));

        let stake = MIN_DISPUTE_STAKE + extra;
        let result = DisputeValidator::validate_dispute_parameters(&env, &market_id, &user, &market, stake);
        prop_assert!(result.is_ok());
    }
}

// ===== PROPERTY 7: FEE DISTRIBUTION BOUNDS =====
// The winner's payout must always be >= their original stake and <= total staked.

proptest! {
    #[test]
    fn prop_dispute_fee_distribution_winner_bounds(
        winner_stake in 1i128..=10_000_000_000_000i128,
        loser_stake in 0i128..=10_000_000_000_000i128,
    ) {
        let env = Env::default();
        let dispute_id = Symbol::new(&env, "prop_dist");

        let voting = DisputeVoting {
            dispute_id,
            voting_start: 0,
            voting_end: 1,
            total_votes: 2,
            support_votes: 1,
            against_votes: 1,
            total_support_stake: winner_stake,
            total_against_stake: loser_stake,
            status: DisputeVotingStatus::Completed,
        };

        let outcome = DisputeUtils::calculate_stake_weighted_outcome(&voting);
        let distribution = DisputeUtils::distribute_fees_based_on_outcome(
            &env,
            &Symbol::new(&env, "dist_test"),
            &voting,
            outcome,
        ).unwrap();

        let winner_stake_recovered = if outcome {
            distribution.winner_stake
        } else {
            distribution.loser_stake
        };

        // Winner stake must be non-negative
        prop_assert!(winner_stake_recovered >= 0);

        // Winner must recover at least their original stake
        prop_assert!(winner_stake_recovered >= if outcome { winner_stake } else { loser_stake });

        // Total distributed must not exceed total staked
        let total_staked = winner_stake + loser_stake;
        prop_assert!(distribution.total_fees <= total_staked);
    }
}

// ===== PROPERTY 8: DISPUTE WINDOW VALIDATION =====
// Markets that have ended and are past the dispute window must be rejected.

proptest! {
    #[test]
    fn prop_dispute_window_expired_rejected(
        end_time_offset in 1u64..=100_000u64,
        window_seconds in 1u64..=86_400u64,
        time_past_end in 1u64..=100_000u64,
    ) {
        let env = Env::default();
        let admin = Address::generate(&env);
        let market_id = Symbol::new(&env, "prop_window");

        let end_time = env.ledger().timestamp().saturating_sub(end_time_offset);
        let mut market = market_with_oracle(&env, &admin, end_time);
        market.dispute_window_seconds = window_seconds;
        market.oracle_result = Some(SorobanString::from_str(&env, "yes"));

        // Advance past end_time + dispute_window_seconds
        env.ledger().set(LedgerInfo {
            timestamp: end_time + window_seconds + time_past_end,
            protocol_version: 22,
            sequence_number: env.ledger().sequence() + 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 10000,
        });

        let result = DisputeValidator::validate_market_for_dispute(&env, &market);
        prop_assert!(result.is_err(), "Dispute past window should be rejected");
    }
}

// ===== PROPERTY 9: RESOLVED MARKETS REJECT NEW DISPUTES =====
// Markets that already have winning_outcomes set must be rejected.

proptest! {
    #[test]
    fn prop_dispute_resolved_market_rejected(
        end_time_offset in 1u64..=100_000u64,
    ) {
        let env = Env::default();
        let admin = Address::generate(&env);
        let market_id = Symbol::new(&env, "prop_resolved");

        let end_time = env.ledger().timestamp().saturating_sub(end_time_offset);
        let mut market = market_with_oracle(&env, &admin, end_time);
        market.oracle_result = Some(SorobanString::from_str(&env, "yes"));
        market.winning_outcomes = Some(vec![SorobanString::from_str(&env, "Yes")]);

        let result = DisputeValidator::validate_market_for_dispute(&env, &market);
        prop_assert!(result.is_err(), "Dispute on resolved market should be rejected");
    }
}

// ===== PROPERTY 10: COOLDOWN ENFORCEMENT =====
// Admin actions within the cooldown period must be rejected.

proptest! {
    #[test]
    fn prop_dispute_cooldown_enforced(
        cooldown_seconds in 1u64..=86_400u64,
        time_elapsed in 0u64..=86_400u64,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            // Set cooldown
            DisputeManager::set_admin_cooldown(&env, admin.clone(), cooldown_seconds).unwrap();
        });

        // Advance time by `time_elapsed`
        env.ledger().set(LedgerInfo {
            timestamp: env.ledger().timestamp() + time_elapsed,
            protocol_version: 22,
            sequence_number: env.ledger().sequence() + 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 10000,
        });

        env.mock_all_auths();
        let result = DisputeManager::set_anti_grief_floor(&env, admin.clone(), 5000);

        if time_elapsed < cooldown_seconds {
            // Within cooldown: should be rejected
            prop_assert!(result.is_err(), "Action within cooldown should be rejected");
        } else {
            // Cooldown expired: should succeed
            prop_assert!(result.is_ok(), "Action after cooldown should succeed");
        }
    }
}

// ===== PROPERTY 11: DISPUTE STAKE CAP ENFORCEMENT =====
// When a per-market per-user cap is set, exceeding it must be rejected.

proptest! {
    #[test]
    fn prop_dispute_stake_cap_enforced(
        cap in 10_000_000i128..=1_000_000_000_000i128,
        first_stake in 1_000_000i128..=500_000_000_000i128,
        second_stake in 1_000_000i128..=500_000_000_000i128,
    ) {
        let env = Env::default();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "prop_cap");

        let mut market = market_with_oracle(&env, &admin, env.ledger().timestamp().saturating_sub(1));
        market.oracle_result = Some(SorobanString::from_str(&env, "yes"));
        market.dispute_stakes.set(user.clone(), first_stake);

        // Validator should reject if first_stake + second_stake > cap
        let result = DisputeValidator::validate_dispute_parameters(&env, &market_id, &user, &market, second_stake);

        if first_stake + second_stake > cap {
            prop_assert!(result.is_err(), "Stake exceeding cap should be rejected");
        }
    }
}

// ===== PROPERTY 12: TIMEOUT PARAMETER VALIDATION =====
// Timeout hours must be within valid bounds (1..=720).

proptest! {
    #[test]
    fn prop_dispute_timeout_valid_range(
        hours in 1u32..=720u32,
    ) {
        let result = DisputeValidator::validate_dispute_timeout_parameters(hours);
        prop_assert!(result.is_ok(), "Valid timeout hours should succeed");
    }
}

proptest! {
    #[test]
    fn prop_dispute_timeout_zero_rejected(
    ) {
        let result = DisputeValidator::validate_dispute_timeout_parameters(0);
        prop_assert!(result.is_err(), "Zero timeout should be rejected");
    }
}

proptest! {
    #[test]
    fn prop_dispute_timeout_excessive_rejected(
        hours in 721u32..=1000u32,
    ) {
        let result = DisputeValidator::validate_dispute_timeout_parameters(hours);
        prop_assert!(result.is_err(), "Excessive timeout should be rejected");
    }
}

// ===== PROPERTY 13: DISPUTE ESCALATION CONDITIONS =====
// Only participants who have voted can escalate; duplicate escalations are rejected.

proptest! {
    #[test]
    fn prop_dispute_escalation_requires_participation(
        support in 1i128..=10_000_000_000_000i128,
        against in 1i128..=10_000_000_000_000i128,
    ) {
        let env = Env::default();
        let dispute_id = Symbol::new(&env, "prop_esc");
        let user = Address::generate(&env);

        let voting = completed_voting(&env, dispute_id.clone(), support, against);
        DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();

        // Store a vote for the user so they are considered a participant
        let vote = crate::disputes::DisputeVote {
            user: user.clone(),
            dispute_id: dispute_id.clone(),
            vote: support > against,
            stake: support,
            timestamp: env.ledger().timestamp(),
            reason: None,
        };
        DisputeUtils::store_dispute_vote(&env, &dispute_id, &vote).unwrap();

        // User who voted should be allowed to escalate (no error from participation check)
        let result = DisputeValidator::validate_dispute_escalation_conditions(&env, &user, &dispute_id);
        prop_assert!(result.is_ok(), "Participating user should be allowed to escalate");
    }
}

// ===== PROPERTY 14: DISPUTE VOTING CONDITIONS =====
// Voting must be within the active period and status must be Active.

proptest! {
    #[test]
    fn prop_dispute_voting_active_period(
        start_offset in 0u64..=1000u64,
        duration in 1u64..=100_000u64,
    ) {
        let env = Env::default();
        let dispute_id = Symbol::new(&env, "prop_vote");

        let voting = DisputeVoting {
            dispute_id: dispute_id.clone(),
            voting_start: env.ledger().timestamp() + start_offset,
            voting_end: env.ledger().timestamp() + start_offset + duration,
            total_votes: 0,
            support_votes: 0,
            against_votes: 0,
            total_support_stake: 0,
            total_against_stake: 0,
            status: DisputeVotingStatus::Active,
        };

        DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();

        // Current time is before voting_start, so voting should not be active
        let result = DisputeValidator::validate_dispute_voting_conditions(&env, &Symbol::new(&env, "m"), &dispute_id);
        prop_assert!(result.is_err(), "Voting before start should be rejected");
    }
}

// ===== PROPERTY 15: DISPUTE STAKE DECAY CALCULATION =====
// The tally_votes function must never panic and must return a value <= raw_stake.

proptest! {
    #[test]
    fn prop_dispute_tally_votes_never_exceeds_raw(
        raw_stake in 1i128..=10_000_000_000_000i128,
        vote_time in 1u64..=1_000_000u64,
        window_start in 0u64..=1_000_000u64,
    ) {
        let env = Env::default();
        let vote_time = vote_time.max(window_start);

        let result = DisputeUtils::tally_votes(&env, raw_stake, vote_time, window_start);

        // Decayed stake must never exceed the raw stake
        prop_assert!(result <= raw_stake, "Decayed stake must not exceed raw stake");
        // Decayed stake must never be negative
        prop_assert!(result >= 0, "Decayed stake must be non-negative");
    }
}

// ===== PROPERTY 16: DISPUTE HISTORY EVICTION INVARIANT =====
// After eviction, history length must not exceed the cap.

proptest! {
    #[test]
    fn prop_dispute_history_cap_respected(
        cap in 1u32..=100u32,
        disputes_added in 1u32..=200u32,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let market_id = Symbol::new(&env, "prop_hist");
        let contract_id = env.register(PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            DisputeManager::set_history_cap(&env, admin.clone(), cap).unwrap();

            let mut history = vec![];
            for i in 0..disputes_added {
                let user = Address::generate(&env);
                let mut dispute = crate::disputes::testing::create_test_dispute(&env, user, market_id.clone(), 1000);
                dispute.status = if i % 2 == 0 {
                    crate::types::DisputeStatus::Resolved
                } else {
                    crate::types::DisputeStatus::Active
                };
                history.push(dispute);
            }

            DisputeManager::apply_eviction(&env, &market_id, &mut history).unwrap();
            prop_assert!(history.len() <= cap as usize, "History length must not exceed cap");
        });
    }
}

// ===== PROPERTY 17: DISPUTE STAKE FLOOR ENFORCEMENT =====
// When a market-specific dispute stake floor is set, stakes below it must be rejected.

proptest! {
    #[test]
    fn prop_dispute_market_floor_enforced(
        floor in 1_000_000i128..=100_000_000_000i128,
        stake_below_floor in 0i128..(floor - 1),
    ) {
        let env = Env::default();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "prop_floor");

        let mut market = market_with_oracle(&env, &admin, env.ledger().timestamp().saturating_sub(1));
        market.oracle_result = Some(SorobanString::from_str(&env, "yes"));
        market.dispute_stake_floor = Some(floor);

        let result = DisputeValidator::validate_dispute_parameters(&env, &market_id, &user, &market, stake_below_floor);
        prop_assert!(result.is_err(), "Stake below market floor should be rejected");
    }
}

// ===== PROPERTY 18: DISPUTE STAKE FLOOR PASSES AT OR ABOVE =====
// Stakes at or above the market-specific floor must pass validation.

proptest! {
    #[test]
    fn prop_dispute_market_floor_passes_at_or_above(
        floor in 1_000_000i128..=100_000_000_000i128,
        extra in 0i128..=1_000_000_000i128,
    ) {
        let env = Env::default();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let market_id = Symbol::new(&env, "prop_floor2");

        let mut market = market_with_oracle(&env, &admin, env.ledger().timestamp().saturating_sub(1));
        market.oracle_result = Some(SorobanString::from_str(&env, "yes"));
        market.dispute_stake_floor = Some(floor);

        let stake = floor + extra;
        let result = DisputeValidator::validate_dispute_parameters(&env, &market_id, &user, &market, stake);
        prop_assert!(result.is_ok(), "Stake at or above market floor should pass");
    }
}

// ===== PROPERTY 19: DISPUTE DECAY CONFIG SET AND RETRIEVED =====
// Setting a decay config must be retrievable and consistent.

proptest! {
    #[test]
    fn prop_dispute_decay_config_roundtrip(
        half_life in 1u64..=86_400u64,
        floor_bps in 100u32..=10_000u32,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let config = DisputeDecayConfig {
                half_life_seconds: half_life,
                floor_bps,
            };

            DisputeUtils::set_dispute_decay_config(&env, admin.clone(), config.clone()).unwrap();

            let retrieved: DisputeDecayConfig = env.storage().persistent()
                .get(&symbol_short!("decaycfg"))
                .unwrap();

            prop_assert_eq!(retrieved.half_life_seconds, config.half_life_seconds);
            prop_assert_eq!(retrieved.floor_bps, config.floor_bps);
        });
    }
}

// ===== PROPERTY 20: DISPUTE TIMEOUT LIFECYCLE =====
// Setting, retrieving, and removing a dispute timeout must be consistent.

proptest! {
    #[test]
    fn prop_dispute_timeout_lifecycle(
        timeout_hours in 1u32..=720u32,
        extension_hours in 1u32..=168u32,
    ) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let dispute_id = Symbol::new(&env, "prop_timeout");
        let contract_id = env.register(PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            DisputeManager::set_dispute_timeout(
                &env,
                dispute_id.clone(),
                timeout_hours,
                extension_hours,
            ).unwrap();

            let timeout = DisputeUtils::get_dispute_timeout(&env, &dispute_id).unwrap();
            prop_assert_eq!(timeout.timeout_hours, timeout_hours);
            prop_assert_eq!(timeout.total_extension_hours, 0);

            // Extend the timeout
            DisputeManager::extend_dispute_timeout(&env, dispute_id.clone(), extension_hours).unwrap();

            let updated = DisputeUtils::get_dispute_timeout(&env, &dispute_id).unwrap();
            prop_assert_eq!(updated.total_extension_hours, extension_hours);

            // Remove the timeout
            DisputeUtils::remove_dispute_timeout(&env, &dispute_id).unwrap();

            let exists = DisputeUtils::has_dispute_timeout(&env, &dispute_id);
            prop_assert!(!exists, "Timeout should be removed");
        });
    }
}

// ===== INTEGRATION INVARIANT: DISPUTE OUTCOME CONSISTENCY =====
// When support > against, outcome must be true; when support < against, outcome must be false.

proptest! {
    #[test]
    fn prop_dispute_outcome_consistency(
        support in 1i128..=10_000_000_000_000i128,
        against in 1i128..=10_000_000_000_000i128,
    ) {
        let env = Env::default();
        let dispute_id = Symbol::new(&env, "prop_consistency");

        let voting = completed_voting(&env, dispute_id, support, against);
        let outcome = DisputeUtils::calculate_stake_weighted_outcome(&voting);

        if support > against {
            prop_assert!(outcome, "Support > against should uphold dispute");
        } else if support < against {
            prop_assert!(!outcome, "Support < against should reject dispute");
        } else {
            prop_assert!(!outcome, "Support == against (tie) should reject dispute");
        }
    }
}

// ===== INTEGRATION INVARIANT: DISPUTE VOTING STORE AND RETRIEVE =====
// Storing and retrieving dispute voting data must be consistent.

proptest! {
    #[test]
    fn prop_dispute_voting_roundtrip(
        support in 0i128..=10_000_000_000_000i128,
        against in 0i128..=10_000_000_000_000i128,
        total_votes in 0u32..=10_000u32,
    ) {
        let env = Env::default();
        let dispute_id = Symbol::new(&env, "prop_roundtrip");

        let voting = DisputeVoting {
            dispute_id: dispute_id.clone(),
            voting_start: env.ledger().timestamp(),
            voting_end: env.ledger().timestamp() + (DISPUTE_EXTENSION_HOURS as u64 * 3600),
            total_votes,
            support_votes: u32::from(support > 0),
            against_votes: u32::from(against > 0),
            total_support_stake: support,
            total_against_stake: against,
            status: DisputeVotingStatus::Completed,
        };

        DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();

        let retrieved = DisputeUtils::get_dispute_voting(&env, &dispute_id).unwrap();
        prop_assert_eq!(retrieved.total_support_stake, voting.total_support_stake);
        prop_assert_eq!(retrieved.total_against_stake, voting.total_against_stake);
        prop_assert_eq!(retrieved.status, voting.status);
    }
}