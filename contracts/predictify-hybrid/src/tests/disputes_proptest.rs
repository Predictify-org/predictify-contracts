//! Property tests for dispute voting state invariants.
//!
//! These tests operate on the same Soroban storage helpers used by dispute
//! entrypoints. Generated vote sequences are checked after every transition so
//! proptest can shrink a failure to the smallest state-changing sequence.

use alloc::vec::Vec as StdVec;

use proptest::prelude::*;
use soroban_sdk::{symbol_short, testutils::Address as _, Address, Env, Symbol};

use crate::{
    disputes::{DisputeDecayConfig, DisputeUtils, DisputeVote, DisputeVoting, DisputeVotingStatus},
    Error, PredictifyHybrid,
};

const MAX_GENERATED_STAKE: i128 = 1_000_000_000_000;

/// Converts a typed contract error into a shrink-friendly proptest failure.
fn contract_result<T>(result: Result<T, Error>) -> Result<T, TestCaseError> {
    result.map_err(|error| TestCaseError::fail(alloc::format!("contract error: {error:?}")))
}

/// Runs a property inside registered contract storage.
fn with_contract<T>(test: impl FnOnce(&Env) -> T) -> T {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    env.as_contract(&contract_id, || test(&env))
}

/// Builds an active voting record with deterministic timestamps.
fn active_voting(env: &Env, dispute_id: &Symbol) -> DisputeVoting {
    DisputeVoting {
        dispute_id: dispute_id.clone(),
        voting_start: 1_000,
        voting_end: 100_000,
        total_votes: 0,
        support_votes: 0,
        against_votes: 0,
        total_support_stake: 0,
        total_against_stake: 0,
        status: DisputeVotingStatus::Active,
    }
}

/// Creates a generated vote for the supplied side and stake.
fn vote(
    env: &Env,
    dispute_id: &Symbol,
    supports: bool,
    stake: i128,
    timestamp: u64,
) -> DisputeVote {
    DisputeVote {
        user: Address::generate(env),
        dispute_id: dispute_id.clone(),
        vote: supports,
        stake,
        timestamp,
        reason: None,
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 128,
        max_shrink_iters: 2_048,
        ..ProptestConfig::default()
    })]

    /// Every successful vote preserves count and stake conservation.
    #[test]
    fn generated_vote_sequences_preserve_tally_invariants(
        votes in prop::collection::vec(
            (any::<bool>(), 0i128..=MAX_GENERATED_STAKE),
            0..64,
        ),
    ) {
        with_contract(|env| {
            let dispute_id = Symbol::new(env, "inv_seq");
            let initial = active_voting(env, &dispute_id);
            contract_result(DisputeUtils::store_dispute_voting(env, &dispute_id, &initial))?;

            let mut expected_support_votes = 0u32;
            let mut expected_against_votes = 0u32;
            let mut expected_support_stake = 0i128;
            let mut expected_against_stake = 0i128;

            for (index, (supports, stake)) in votes.iter().enumerate() {
                let generated_vote = vote(
                    env,
                    &dispute_id,
                    *supports,
                    *stake,
                    1_000 + index as u64,
                );
                contract_result(DisputeUtils::add_vote_to_dispute(env, &dispute_id, generated_vote))?;

                if *supports {
                    expected_support_votes += 1;
                    expected_support_stake += *stake;
                } else {
                    expected_against_votes += 1;
                    expected_against_stake += *stake;
                }

                let stored = contract_result(DisputeUtils::get_dispute_voting(env, &dispute_id))?;
                prop_assert_eq!(
                    stored.total_votes,
                    stored.support_votes + stored.against_votes,
                );
                prop_assert_eq!(stored.support_votes, expected_support_votes);
                prop_assert_eq!(stored.against_votes, expected_against_votes);
                prop_assert_eq!(stored.total_support_stake, expected_support_stake);
                prop_assert_eq!(stored.total_against_stake, expected_against_stake);
                prop_assert_eq!(stored.dispute_id, dispute_id.clone());
                prop_assert!(matches!(stored.status, DisputeVotingStatus::Active));
            }

            Ok::<(), TestCaseError>(())
        })?;
    }

    /// Decay never creates stake and cannot increase as a vote gets later.
    #[test]
    fn decayed_stake_is_bounded_and_monotonic(
        raw_stake in 0i128..=i128::MAX,
        half_life in 1u64..=604_800,
        floor_bps in 0u32..=20_000,
        first_elapsed in 0u64..=10_000_000,
        extra_elapsed in 0u64..=10_000_000,
    ) {
        with_contract(|env| {
            env.storage().persistent().set(
                &symbol_short!("decaycfg"),
                &DisputeDecayConfig {
                    half_life_seconds: half_life,
                    floor_bps,
                },
            );

            let later_elapsed = first_elapsed.saturating_add(extra_elapsed);
            let first = DisputeUtils::tally_votes(env, raw_stake, first_elapsed, 0);
            let later = DisputeUtils::tally_votes(env, raw_stake, later_elapsed, 0);

            prop_assert!(first >= 0);
            prop_assert!(first <= raw_stake);
            prop_assert!(later >= 0);
            prop_assert!(later <= first);
        });
    }

    /// The outcome is support only for a strict support-stake majority.
    #[test]
    fn outcome_matches_strict_stake_majority(
        support in 0i128..=i128::MAX,
        against in 0i128..=i128::MAX,
    ) {
        let env = Env::default();
        let dispute_id = Symbol::new(&env, "inv_out");
        let mut voting = active_voting(&env, &dispute_id);
        voting.total_support_stake = support;
        voting.total_against_stake = against;

        prop_assert_eq!(
            DisputeUtils::calculate_stake_weighted_outcome(&voting),
            support > against,
        );
    }

    /// Overflow errors are atomic: rejected votes leave stored state unchanged.
    #[test]
    fn overflow_rejection_preserves_stored_state(
        supports in any::<bool>(),
        counter_overflow in any::<bool>(),
    ) {
        with_contract(|env| {
            let dispute_id = Symbol::new(env, "inv_ovf");
            let mut initial = active_voting(env, &dispute_id);
            if counter_overflow {
                initial.total_votes = u32::MAX;
            } else if supports {
                initial.total_support_stake = i128::MAX;
            } else {
                initial.total_against_stake = i128::MAX;
            }
            contract_result(DisputeUtils::store_dispute_voting(env, &dispute_id, &initial))?;

            let result = DisputeUtils::add_vote_to_dispute(
                env,
                &dispute_id,
                vote(env, &dispute_id, supports, 1, 1_000),
            );
            prop_assert_eq!(result, Err(Error::Overflow));

            let stored = contract_result(DisputeUtils::get_dispute_voting(env, &dispute_id))?;
            prop_assert_eq!(stored.total_votes, initial.total_votes);
            prop_assert_eq!(stored.support_votes, initial.support_votes);
            prop_assert_eq!(stored.against_votes, initial.against_votes);
            prop_assert_eq!(stored.total_support_stake, initial.total_support_stake);
            prop_assert_eq!(stored.total_against_stake, initial.total_against_stake);

            Ok::<(), TestCaseError>(())
        })?;
    }
}
