//! Property test for the Governance contract's core tally invariant.
//!
//! For an arbitrary sequence of distinct voters casting weighted votes on an
//! active proposal, the invariant under test is:
//!
//! 1. `votes_for` / `votes_against` always equal the sum of weights cast for
//!    that choice, regardless of the order votes are cast in.
//! 2. `execute_proposal` finalizes to `Executed` iff `votes_for >
//!    votes_against`, else `Rejected` — and never anything else.
//! 3. Once finalized, the proposal is terminal: neither further votes nor a
//!    second `execute_proposal` call can change its status or tallies.

#![cfg(test)]

use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

use governance::{
    GovernanceContract, GovernanceContractClient, GovernanceError, ProposalStatus, VoteChoice,
};

const VOTING_PERIOD: u64 = 3_600;

fn deploy(env: &Env) -> (GovernanceContractClient<'_>, Address) {
    env.mock_all_auths();
    let contract_id = env.register(GovernanceContract, ());
    let client = GovernanceContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin, &VOTING_PERIOD);
    (client, admin)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Core invariant: tallies always equal the sum of cast weights, and
    /// finalization is a deterministic, terminal function of those tallies —
    /// for any valid sequence of distinct-voter, weighted votes.
    #[test]
    fn tally_and_finalization_invariant(
        votes in prop::collection::vec((any::<bool>(), 1u64..1_000), 0..8),
    ) {
        let env = Env::default();
        let (client, admin) = deploy(&env);

        let id = client.create_proposal(&admin, &String::from_str(&env, "prop"));

        let mut expected_for: u64 = 0;
        let mut expected_against: u64 = 0;

        for (is_for, weight) in &votes {
            let voter = Address::generate(&env);
            let choice = if *is_for { VoteChoice::For } else { VoteChoice::Against };
            client.cast_vote(&voter, &id, &choice, weight);

            if *is_for {
                expected_for += weight;
            } else {
                expected_against += weight;
            }

            let proposal = client.get_proposal(&id);
            prop_assert_eq!(proposal.votes_for, expected_for);
            prop_assert_eq!(proposal.votes_against, expected_against);
            prop_assert_eq!(proposal.status, ProposalStatus::Active);
        }

        // A second cast from the SAME voter must be rejected and must never
        // move the tally, no matter how many other voters preceded it.
        if let Some((is_for, weight)) = votes.first() {
            let repeat_voter = Address::generate(&env);
            let choice = if *is_for { VoteChoice::For } else { VoteChoice::Against };
            client.cast_vote(&repeat_voter, &id, &choice, weight);
            if *is_for {
                expected_for += weight;
            } else {
                expected_against += weight;
            }

            let result = client.try_cast_vote(&repeat_voter, &id, &choice, weight);
            prop_assert!(matches!(result, Err(Ok(GovernanceError::AlreadyVoted))));

            let proposal = client.get_proposal(&id);
            prop_assert_eq!(proposal.votes_for, expected_for);
            prop_assert_eq!(proposal.votes_against, expected_against);
        }

        // Close the voting window and finalize.
        env.ledger().with_mut(|l| l.timestamp += VOTING_PERIOD + 1);
        let final_status = client.execute_proposal(&admin, &id);

        let expected_status = if expected_for > expected_against {
            ProposalStatus::Executed
        } else {
            ProposalStatus::Rejected
        };
        prop_assert_eq!(final_status, expected_status);

        let proposal = client.get_proposal(&id);
        prop_assert_eq!(proposal.status, expected_status);
        prop_assert_eq!(proposal.votes_for, expected_for);
        prop_assert_eq!(proposal.votes_against, expected_against);

        // Terminal: neither a repeat finalize nor a new vote can change it.
        let refinalize = client.try_execute_proposal(&admin, &id);
        prop_assert!(matches!(refinalize, Err(Ok(GovernanceError::InvalidStateTransition))));

        let late_voter = Address::generate(&env);
        let late_vote = client.try_cast_vote(&late_voter, &id, &VoteChoice::For, &1);
        prop_assert!(matches!(late_vote, Err(Ok(GovernanceError::InvalidStateTransition))));

        let proposal = client.get_proposal(&id);
        prop_assert_eq!(proposal.status, expected_status);
        prop_assert_eq!(proposal.votes_for, expected_for);
        prop_assert_eq!(proposal.votes_against, expected_against);
    }
}
