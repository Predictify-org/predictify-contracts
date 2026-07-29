#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, BytesN, Env, IntoVal,
};
use tokens::{
    AccountLimits, AccountStateKind, AccountUsage, TokenLimitError, TokensContract,
    TokensContractClient, MAX_CONFIGURABLE_ACCOUNT_LIMIT,
};

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

fn indexed_item_id(env: &Env, index: u32) -> BytesN<32> {
    let mut bytes = [0u8; 32];
    bytes[..4].copy_from_slice(&index.to_be_bytes());
    BytesN::from_array(env, &bytes)
}

fn setup<'a>(env: &'a Env, account_limits: &AccountLimits) -> (TokensContractClient<'a>, Address) {
    env.mock_all_auths();
    let admin = Address::generate(env);
    let contract_id = env.register(TokensContract, ());
    let client = TokensContractClient::new(env, &contract_id);
    client.initialize(&admin, account_limits);
    (client, admin)
}

#[test]
fn initialize_requires_admin_authentication() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(TokensContract, ());
    let client = TokensContractClient::new(&env, &contract_id);

    assert!(client.try_initialize(&admin, &limits(1, 1, 1)).is_err());
}

#[test]
fn initialize_records_admin_and_limits_and_cannot_repeat() {
    let env = Env::default();
    let configured_limits = limits(3, 4, 5);
    let (client, admin) = setup(&env, &configured_limits);

    assert_eq!(client.get_admin(), admin);
    assert_eq!(client.get_account_limits(), configured_limits);
    assert_eq!(
        client.try_initialize(&admin, &configured_limits),
        Err(Ok(TokenLimitError::AlreadyInitialized))
    );
}

#[test]
fn initialize_rejects_limits_above_hard_maximum() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(TokensContract, ());
    let client = TokensContractClient::new(&env, &contract_id);

    let invalid_limits = [
        limits(MAX_CONFIGURABLE_ACCOUNT_LIMIT + 1, 1, 1),
        limits(1, MAX_CONFIGURABLE_ACCOUNT_LIMIT + 1, 1),
        limits(1, 1, MAX_CONFIGURABLE_ACCOUNT_LIMIT + 1),
    ];

    for invalid in invalid_limits {
        assert_eq!(
            client.try_initialize(&admin, &invalid),
            Err(Ok(TokenLimitError::InvalidLimit))
        );
    }
}

#[test]
fn only_admin_can_update_limits() {
    let env = Env::default();
    let (client, admin) = setup(&env, &limits(1, 1, 1));
    let non_admin = Address::generate(&env);

    assert_eq!(
        client.try_set_account_limits(&non_admin, &limits(2, 2, 2)),
        Err(Ok(TokenLimitError::Unauthorized))
    );

    client.set_account_limits(&admin, &limits(2, 3, 4));
    assert_eq!(client.get_account_limits(), limits(2, 3, 4));

    assert_eq!(
        client.try_set_account_limits(&admin, &limits(2, 3, MAX_CONFIGURABLE_ACCOUNT_LIMIT + 1)),
        Err(Ok(TokenLimitError::InvalidLimit))
    );
    assert_eq!(client.get_account_limits(), limits(2, 3, 4));
}

#[test]
fn uninitialized_views_and_mutations_return_stable_defaults_or_errors() {
    let env = Env::default();
    env.mock_all_auths();
    let account = Address::generate(&env);
    let contract_id = env.register(TokensContract, ());
    let client = TokensContractClient::new(&env, &contract_id);
    let id = item_id(&env, 18);

    assert_eq!(
        client.try_get_admin(),
        Err(Ok(TokenLimitError::NotInitialized))
    );
    assert_eq!(
        client.try_get_account_limits(),
        Err(Ok(TokenLimitError::NotInitialized))
    );
    assert_eq!(
        client.try_get_remaining_capacity(&account),
        Err(Ok(TokenLimitError::NotInitialized))
    );
    assert_eq!(client.get_account_usage(&account), AccountUsage::default());
    assert!(!client.is_account_item_tracked(&account, &AccountStateKind::Bet, &id));
    assert_eq!(
        client.try_track_account_item(&account, &AccountStateKind::Bet, &id),
        Err(Ok(TokenLimitError::NotInitialized))
    );
    assert_eq!(
        client.try_untrack_account_item(&account, &AccountStateKind::Bet, &id),
        Err(Ok(TokenLimitError::NotInitialized))
    );
}

#[test]
fn every_state_category_is_capped_without_partial_mutation() {
    let env = Env::default();
    let (client, _) = setup(&env, &limits(1, 1, 1));
    let account = Address::generate(&env);

    let cases = [
        (
            AccountStateKind::Bet,
            item_id(&env, 1),
            item_id(&env, 2),
            TokenLimitError::BetLimitExceeded,
        ),
        (
            AccountStateKind::Position,
            item_id(&env, 3),
            item_id(&env, 4),
            TokenLimitError::PositionLimitExceeded,
        ),
        (
            AccountStateKind::Subscription,
            item_id(&env, 5),
            item_id(&env, 6),
            TokenLimitError::SubscriptionLimitExceeded,
        ),
    ];

    for (kind, accepted_id, rejected_id, expected_error) in cases {
        client.track_account_item(&account, &kind, &accepted_id);

        assert_eq!(
            client.try_track_account_item(&account, &kind, &rejected_id),
            Err(Ok(expected_error))
        );
        assert!(client.is_account_item_tracked(&account, &kind, &accepted_id));
        assert!(!client.is_account_item_tracked(&account, &kind, &rejected_id));
    }

    assert_eq!(
        client.get_account_usage(&account),
        AccountUsage {
            bets: 1,
            positions: 1,
            subscriptions: 1,
        }
    );
}

#[test]
fn duplicate_item_does_not_consume_capacity() {
    let env = Env::default();
    let (client, _) = setup(&env, &limits(2, 0, 0));
    let account = Address::generate(&env);
    let id = item_id(&env, 7);

    client.track_account_item(&account, &AccountStateKind::Bet, &id);

    assert_eq!(
        client.try_track_account_item(&account, &AccountStateKind::Bet, &id),
        Err(Ok(TokenLimitError::ItemAlreadyTracked))
    );
    assert_eq!(client.get_account_usage(&account).bets, 1);
}

#[test]
fn usage_and_capacity_are_isolated_per_account() {
    let env = Env::default();
    let (client, _) = setup(&env, &limits(1, 1, 1));
    let first = Address::generate(&env);
    let second = Address::generate(&env);
    let shared_id = item_id(&env, 8);

    client.track_account_item(&first, &AccountStateKind::Bet, &shared_id);
    client.track_account_item(&second, &AccountStateKind::Bet, &shared_id);

    assert_eq!(client.get_account_usage(&first).bets, 1);
    assert_eq!(client.get_account_usage(&second).bets, 1);
    assert_eq!(client.get_remaining_capacity(&first).bets, 0);
    assert_eq!(client.get_remaining_capacity(&second).bets, 0);
}

#[test]
fn untracking_exact_item_releases_capacity() {
    let env = Env::default();
    let (client, _) = setup(&env, &limits(1, 0, 0));
    let account = Address::generate(&env);
    let first_id = item_id(&env, 9);
    let second_id = item_id(&env, 10);

    client.track_account_item(&account, &AccountStateKind::Bet, &first_id);
    client.untrack_account_item(&account, &AccountStateKind::Bet, &first_id);

    assert_eq!(client.get_account_usage(&account), AccountUsage::default());
    assert_eq!(client.get_remaining_capacity(&account).bets, 1);
    assert!(!client.is_account_item_tracked(&account, &AccountStateKind::Bet, &first_id));

    client.track_account_item(&account, &AccountStateKind::Bet, &second_id);
    assert!(client.is_account_item_tracked(&account, &AccountStateKind::Bet, &second_id));
}

#[test]
fn untracking_one_category_preserves_other_usage() {
    let env = Env::default();
    let (client, _) = setup(&env, &limits(1, 1, 0));
    let account = Address::generate(&env);
    let bet_id = item_id(&env, 19);
    let position_id = item_id(&env, 20);

    client.track_account_item(&account, &AccountStateKind::Bet, &bet_id);
    client.track_account_item(&account, &AccountStateKind::Position, &position_id);
    client.untrack_account_item(&account, &AccountStateKind::Bet, &bet_id);

    assert_eq!(
        client.get_account_usage(&account),
        AccountUsage {
            bets: 0,
            positions: 1,
            subscriptions: 0,
        }
    );
    assert!(client.is_account_item_tracked(&account, &AccountStateKind::Position, &position_id));
}

#[test]
fn fabricated_untrack_is_rejected_without_changing_usage() {
    let env = Env::default();
    let (client, _) = setup(&env, &limits(2, 0, 0));
    let account = Address::generate(&env);
    let stored_id = item_id(&env, 11);
    let fabricated_id = item_id(&env, 12);

    client.track_account_item(&account, &AccountStateKind::Bet, &stored_id);

    assert_eq!(
        client.try_untrack_account_item(&account, &AccountStateKind::Bet, &fabricated_id),
        Err(Ok(TokenLimitError::ItemNotFound))
    );
    assert_eq!(client.get_account_usage(&account).bets, 1);
    assert!(client.is_account_item_tracked(&account, &AccountStateKind::Bet, &stored_id));
}

#[test]
fn zero_limit_disables_new_items_for_that_category() {
    let env = Env::default();
    let (client, _) = setup(&env, &limits(0, 1, 1));
    let account = Address::generate(&env);
    let id = item_id(&env, 13);

    assert_eq!(
        client.try_track_account_item(&account, &AccountStateKind::Bet, &id),
        Err(Ok(TokenLimitError::BetLimitExceeded))
    );
    assert_eq!(client.get_account_usage(&account), AccountUsage::default());
}

#[test]
fn hard_maximum_accepts_exact_boundary_and_rejects_next_item() {
    let env = Env::default();
    let (client, _) = setup(&env, &limits(MAX_CONFIGURABLE_ACCOUNT_LIMIT, 0, 0));
    let account = Address::generate(&env);

    for index in 0..MAX_CONFIGURABLE_ACCOUNT_LIMIT {
        client.track_account_item(
            &account,
            &AccountStateKind::Bet,
            &indexed_item_id(&env, index),
        );
    }

    assert_eq!(
        client.get_account_usage(&account).bets,
        MAX_CONFIGURABLE_ACCOUNT_LIMIT
    );
    assert_eq!(
        client.try_track_account_item(
            &account,
            &AccountStateKind::Bet,
            &indexed_item_id(&env, MAX_CONFIGURABLE_ACCOUNT_LIMIT),
        ),
        Err(Ok(TokenLimitError::BetLimitExceeded))
    );
}

#[test]
fn lowering_limit_below_usage_retains_state_and_blocks_growth() {
    let env = Env::default();
    let (client, admin) = setup(&env, &limits(2, 0, 0));
    let account = Address::generate(&env);
    let first_id = item_id(&env, 14);
    let second_id = item_id(&env, 15);
    let third_id = item_id(&env, 16);

    client.track_account_item(&account, &AccountStateKind::Bet, &first_id);
    client.track_account_item(&account, &AccountStateKind::Bet, &second_id);
    client.set_account_limits(&admin, &limits(1, 0, 0));

    assert_eq!(client.get_account_usage(&account).bets, 2);
    assert_eq!(client.get_remaining_capacity(&account).bets, 0);
    assert_eq!(
        client.try_track_account_item(&account, &AccountStateKind::Bet, &third_id),
        Err(Ok(TokenLimitError::BetLimitExceeded))
    );
}

#[test]
fn tracking_captures_the_account_as_required_signer() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let account = Address::generate(&env);
    let contract_id = env.register(TokensContract, ());
    let client = TokensContractClient::new(&env, &contract_id);

    env.mock_auths(&[MockAuth {
        address: &admin,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "initialize",
            args: (&admin, limits(1, 1, 1)).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.initialize(&admin, &limits(1, 1, 1));

    env.mock_auths(&[MockAuth {
        address: &account,
        invoke: &MockAuthInvoke {
            contract: &contract_id,
            fn_name: "track_account_item",
            args: (&account, AccountStateKind::Bet, item_id(&env, 17)).into_val(&env),
            sub_invokes: &[],
        },
    }]);
    client.track_account_item(&account, &AccountStateKind::Bet, &item_id(&env, 17));

    let auths = env.auths();
    assert_eq!(auths.len(), 1);
    assert_eq!(auths[0].0, account);
}
