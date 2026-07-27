#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Env, String};

#[test]
fn test_place_and_get_bet() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, BettingContract);
    let client = BettingContractClient::new(&env, &contract_id);

    let bettor = Address::generate(&env);
    let market_id = String::from_str(&env, "market_1");
    let outcome = String::from_str(&env, "yes");
    let amount = 1000;

    // Place bet
    client.place_bet(&bettor, &market_id, &amount, &outcome);

    // Get bet
    let bet_opt = client.get_bet(&bettor, &market_id);
    assert!(bet_opt.is_some());
    let bet = bet_opt.unwrap();
    assert_eq!(bet.amount, 1000);
    assert_eq!(bet.outcome, outcome);
}
