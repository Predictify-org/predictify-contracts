#![cfg(test)]

extern crate std;

use markets::{ContractError, MarketsContract, MarketsContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn deploy(env: &Env) -> MarketsContractClient<'_> {
    let contract_id = env.register(MarketsContract, ());
    MarketsContractClient::new(env, &contract_id)
}

#[test]
fn defaults_to_not_paused() {
    let env = Env::default();
    let client = deploy(&env);

    assert!(!client.is_paused());
}

#[test]
fn admin_can_pause_and_unpause() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    assert!(!client.is_paused());

    client.pause_markets(&admin);
    assert!(client.is_paused());

    client.unpause_markets(&admin);
    assert!(!client.is_paused());
}

#[test]
fn non_admin_cannot_pause() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);
    let stranger = Address::generate(&env);

    client.initialize(&admin);

    assert_eq!(
        client.try_pause_markets(&stranger),
        Err(Ok(ContractError::Unauthorized))
    );
}

#[test]
fn pause_before_initialize_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);

    assert_eq!(
        client.try_pause_markets(&admin),
        Err(Ok(ContractError::InvalidState))
    );
}

#[test]
fn cannot_initialize_twice() {
    let env = Env::default();
    env.mock_all_auths();
    let client = deploy(&env);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    assert_eq!(
        client.try_initialize(&admin),
        Err(Ok(ContractError::InvalidState))
    );
}
