#![cfg(test)]

//! Focused property tests for betting-state invariants.
//!
//! These tests exercise the public betting entrypoints over small generated
//! action sequences and assert that market-level aggregate stats stay
//! consistent with the bet records visible through query APIs.
//!
//! Invariants covered:
//! - `total_bets` equals the number of active bets returned by `get_bet`
//! - `unique_bettors` equals the number of active bettors
//! - `total_amount_locked` equals the sum of active bet amounts
//! - each `outcome_totals[outcome]` equals the sum of active bet amounts for
//!   that outcome
//! - after cancellation, a user's bet remains queryable but is no longer active
//!   and no longer contributes to aggregate stats

use crate::bets::MIN_BET_AMOUNT;
use crate::types::{BetStatus, OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use proptest::prelude::*;
use soroban_sdk::{
    testutils::Address as _,
    token::StellarAssetClient,
    vec, Address, Env, Map, String, Symbol,
};

#[derive(Clone, Debug)]
enum Action {
    Place { user_ix: usize, outcome_ix: usize, units: u32 },
    Cancel { user_ix: usize },
}

struct BettingPropSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    token_id: Address,
    market_id: Symbol,
    users: std::vec::Vec<Address>,
}

impl BettingPropSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None, &None);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin);
        let token_id = token_contract.address();

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        let stellar = StellarAssetClient::new(&env, &token_id);
        stellar.mint(&admin, &10_000_0000000);

        let users: std::vec::Vec<Address> = (0..4).map(|_| Address::generate(&env)).collect();
        let token_client = soroban_sdk::token::Client::new(&env, &token_id);
        for user in &users {
            stellar.mint(user, &1_000_0000000);
            token_client.approve(user, &contract_id, &i128::MAX, &1_000_000);
        }
        token_client.approve(&admin, &contract_id, &i128::MAX, &1_000_000);

        let outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];

        let market_id = client.create_market(
            &admin,
            &String::from_str(&env, "Will betting invariants hold?"),
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::reflector(),
                oracle_address: Address::from_str(
                    &env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                feed_id: String::from_str(&env, "BTC/USD"),
                threshold: 100_000_00000000,
                comparison: String::from_str(&env, "gt"),
            },
            &None,
            &86400u64,
            &None,
            &None,
            &None,
        );

        Self {
            env,
            contract_id,
            admin,
            token_id,
            market_id,
            users,
        }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    fn outcome(&self, ix: usize) -> String {
        match ix % 2 {
            0 => String::from_str(&self.env, "yes"),
            _ => String::from_str(&self.env, "no"),
        }
    }

    fn amount_from_units(units: u32) -> i128 {
        MIN_BET_AMOUNT * i128::from(units.max(1))
    }

    fn assert_invariants(&self) {
        let client = self.client();
        let stats = client.get_market_bet_stats(&self.market_id);

        let mut expected_total_bets = 0u32;
        let mut expected_total_amount = 0i128;
        let mut expected_outcomes = Map::new(&self.env);

        for user in &self.users {
            let bet = client.get_bet(&self.market_id, user);
            if let Some(bet) = bet {
                if bet.status == BetStatus::Active {
                    expected_total_bets += 1;
                    expected_total_amount += bet.amount;
                    let prior = expected_outcomes.get(bet.outcome.clone()).unwrap_or(0);
                    expected_outcomes.set(bet.outcome.clone(), prior + bet.amount);
                }
            }
        }

        assert_eq!(stats.total_bets, expected_total_bets, "total_bets invariant violated");
        assert_eq!(
            stats.unique_bettors, expected_total_bets,
            "unique_bettors should equal active bettor count in single-bet-per-user model"
        );
        assert_eq!(
            stats.total_amount_locked, expected_total_amount,
            "total_amount_locked invariant violated"
        );

        for outcome in [String::from_str(&self.env, "yes"), String::from_str(&self.env, "no")] {
            assert_eq!(
                stats.outcome_totals.get(outcome.clone()).unwrap_or(0),
                expected_outcomes.get(outcome).unwrap_or(0),
                "outcome_totals invariant violated"
            );
        }
    }
}

fn action_strategy() -> impl Strategy<Value = std::vec::Vec<Action>> {
    prop::collection::vec(
        prop_oneof![
            (0usize..4, 0usize..2, 1u32..=25).prop_map(|(user_ix, outcome_ix, units)| {
                Action::Place { user_ix, outcome_ix, units }
            }),
            (0usize..4).prop_map(|user_ix| Action::Cancel { user_ix }),
        ],
        1..=24,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn betting_stats_match_active_bets(actions in action_strategy()) {
        let setup = BettingPropSetup::new();
        let client = setup.client();

        for action in actions {
            match action {
                Action::Place { user_ix, outcome_ix, units } => {
                    let user = setup.users[user_ix].clone();
                    let _ = client.try_place_bet(
                        &user,
                        &setup.market_id,
                        &setup.outcome(outcome_ix),
                        &BettingPropSetup::amount_from_units(units),
                        &250,
                    );
                }
                Action::Cancel { user_ix } => {
                    let user = setup.users[user_ix].clone();
                    let _ = client.try_cancel_bet(&user, &setup.market_id);
                }
            }

            setup.assert_invariants();
        }
    }
}

#[test]
fn betting_invariant_cancellation_removes_only_cancelled_bet_from_totals() {
    let setup = BettingPropSetup::new();
    let client = setup.client();

    let user_a = setup.users[0].clone();
    let user_b = setup.users[1].clone();
    let yes = String::from_str(&setup.env, "yes");
    let no = String::from_str(&setup.env, "no");

    client.place_bet(&user_a, &setup.market_id, &yes, &(MIN_BET_AMOUNT * 2), &250);
    client.place_bet(&user_b, &setup.market_id, &no, &(MIN_BET_AMOUNT * 3), &250);

    client.cancel_bet(&user_a, &setup.market_id);

    let bet_a = client.get_bet(&setup.market_id, &user_a).expect("bet should remain queryable");
    let bet_b = client.get_bet(&setup.market_id, &user_b).expect("other active bet should exist");
    let stats = client.get_market_bet_stats(&setup.market_id);

    assert_eq!(bet_a.status, BetStatus::Cancelled);
    assert_eq!(bet_b.status, BetStatus::Active);
    assert_eq!(stats.total_bets, 1);
    assert_eq!(stats.unique_bettors, 1);
    assert_eq!(stats.total_amount_locked, MIN_BET_AMOUNT * 3);
    assert_eq!(stats.outcome_totals.get(yes), None);
    assert_eq!(stats.outcome_totals.get(no).unwrap_or(0), MIN_BET_AMOUNT * 3);
}
