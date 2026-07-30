#![cfg(test)]

//! Structured lifecycle event tests for the tokens contract.
//!
//! `soroban_sdk::testutils::Events::all()` returns the emitted events in
//! their raw XDR form in this SDK version, so these tests assert on event
//! *counts* per lifecycle transition rather than decoding payloads — the
//! payload shape itself is covered by the `#[contracttype]` definitions in
//! `src/events.rs`.

use soroban_sdk::{
    testutils::{Address as _, Events as _},
    Address, BytesN, Env,
};
use tokens::{AccountLimits, AccountStateKind, TokensContract, TokensContractClient};

fn limits(bets: u32, positions: u32, subscriptions: u32) -> AccountLimits {
    AccountLimits {
        bets,
        positions,
        subscriptions,
    }
}

fn item_id(env: &Env, byte: u8) -> BytesN<32> {
    BytesN::from_array(env, &[byte; 32])
}

fn event_count(env: &Env) -> usize {
    env.events().all().events().len()
}

#[test]
fn initialize_emits_exactly_one_event() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TokensContract, ());
    let client = TokensContractClient::new(&env, &contract_id);

    assert_eq!(event_count(&env), 0);
    client.initialize(&admin, &limits(2, 2, 2));
    assert_eq!(event_count(&env), 1);
}

#[test]
fn set_account_limits_emits_exactly_one_event() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TokensContract, ());
    let client = TokensContractClient::new(&env, &contract_id);
    client.initialize(&admin, &limits(2, 2, 2));

    client.set_account_limits(&admin, &limits(5, 5, 5));
    assert_eq!(event_count(&env), 1, "last invocation must publish exactly one event");
}

#[test]
fn track_account_item_emits_exactly_one_event() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TokensContract, ());
    let client = TokensContractClient::new(&env, &contract_id);
    client.initialize(&admin, &limits(2, 2, 2));
    let account = Address::generate(&env);

    client.track_account_item(&account, &AccountStateKind::Bet, &item_id(&env, 1));
    assert_eq!(event_count(&env), 1, "last invocation must publish exactly one event");
}

#[test]
fn untrack_account_item_emits_exactly_one_event() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TokensContract, ());
    let client = TokensContractClient::new(&env, &contract_id);
    client.initialize(&admin, &limits(2, 2, 2));
    let account = Address::generate(&env);
    let id = item_id(&env, 1);
    client.track_account_item(&account, &AccountStateKind::Position, &id);

    client.untrack_account_item(&account, &AccountStateKind::Position, &id);
    assert_eq!(event_count(&env), 1, "last invocation must publish exactly one event");
}

#[test]
fn full_lifecycle_emits_one_event_per_transition() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TokensContract, ());
    let client = TokensContractClient::new(&env, &contract_id);
    let account = Address::generate(&env);
    let id = item_id(&env, 7);

    client.initialize(&admin, &limits(2, 2, 2));
    assert_eq!(event_count(&env), 1);

    client.set_account_limits(&admin, &limits(3, 3, 3));
    assert_eq!(event_count(&env), 1);

    client.track_account_item(&account, &AccountStateKind::Subscription, &id);
    assert_eq!(event_count(&env), 1);

    client.untrack_account_item(&account, &AccountStateKind::Subscription, &id);
    assert_eq!(event_count(&env), 1);
}
