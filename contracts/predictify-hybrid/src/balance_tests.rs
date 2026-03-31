#![cfg(test)]

use crate::errors::Error;
use crate::test::PredictifyTest;
use crate::types::ReflectorAsset;
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

#[test]
fn test_deposit_and_withdrawal_flow() {
    let test = PredictifyTest::setup();
    let env = &test.env;
    let user = &test.user;
    let contract_address = &test.contract_id;

    // 1. Initial State: User has 1000 XLM (minted in setup), Contract has 0
    let token_client = soroban_sdk::token::Client::new(env, &test.token_test.token_id);
    // Verify initial token balances
    // Note: token_test.token_client is associated with the token contract
    // We need to use the client correctly.
    // In PredictifyTest::setup, we minted 1000_0000000 to user.

    assert_eq!(token_client.balance(user), 1000_0000000);
    assert_eq!(token_client.balance(contract_address), 0);

    // 2. Deposit Funds
    let deposit_amount = 500_0000000; // 500 XLM
    let client = crate::PredictifyHybridClient::new(env, contract_address);

    // We need to mock auth for the user
    env.mock_all_auths();

    let balance = client.deposit(user, &ReflectorAsset::Stellar, &deposit_amount);

    // 3. Verify Deposit
    assert_eq!(balance.amount, deposit_amount);
    assert_eq!(balance.user, *user);

    // Verify stored balance matches
    let stored_balance = client.get_balance(user, &ReflectorAsset::Stellar);
    assert_eq!(stored_balance.amount, deposit_amount);

    // Verify token transfer happened
    assert_eq!(token_client.balance(user), 500_0000000);
    assert_eq!(token_client.balance(contract_address), 500_0000000);

    // 4. Withdraw Funds
    let withdraw_amount = 200_0000000; // 200 XLM
    let balance_after_withdraw = client.withdraw(user, &ReflectorAsset::Stellar, &withdraw_amount);

    // 5. Verify Withdrawal
    assert_eq!(balance_after_withdraw.amount, 300_0000000); // 500 - 200 = 300

    // Verify stored balance updated
    let stored_balance_2 = client.get_balance(user, &ReflectorAsset::Stellar);
    assert_eq!(stored_balance_2.amount, 300_0000000);

    // Verify token transfer happened (Contract -> User)
    assert_eq!(token_client.balance(user), 700_0000000); // 500 + 200 = 700
    assert_eq!(token_client.balance(contract_address), 300_0000000); // 500 - 200 = 300
}

#[test]
fn test_insufficient_balance_withdrawal() {
    let test = PredictifyTest::setup();
    let env = &test.env;
    let user = &test.user;
    let contract_address = &test.contract_id;
    let client = crate::PredictifyHybridClient::new(env, contract_address);

    env.mock_all_auths();

    // Deposit 100
    let deposit_amount = 100_0000000;
    client.deposit(user, &ReflectorAsset::Stellar, &deposit_amount);

    // Try to withdraw 150
    let withdraw_amount = 150_0000000;
    let result = client.try_withdraw(user, &ReflectorAsset::Stellar, &withdraw_amount);

    assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
}

#[test]
fn test_invalid_deposit_amount() {
    let test = PredictifyTest::setup();
    let env = &test.env;
    let user = &test.user;
    let contract_address = &test.contract_id;
    let client = crate::PredictifyHybridClient::new(env, contract_address);

    env.mock_all_auths();

    // Try to deposit 0
    let result = client.try_deposit(user, &ReflectorAsset::Stellar, &0);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));

    // Try to deposit negative
    let result_neg = client.try_deposit(user, &ReflectorAsset::Stellar, &-100);
    assert_eq!(result_neg, Err(Ok(Error::InvalidInput)));
}

#[test]
fn test_invalid_withdraw_amount() {
    let test = PredictifyTest::setup();
    let env = &test.env;
    let user = &test.user;
    let contract_address = &test.contract_id;
    let client = crate::PredictifyHybridClient::new(env, contract_address);

    env.mock_all_auths();

    client.deposit(user, &ReflectorAsset::Stellar, &1000);

    // Try to withdraw 0
    let result = client.try_withdraw(user, &ReflectorAsset::Stellar, &0);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));

    // Try to withdraw negative
    let result_neg = client.try_withdraw(user, &ReflectorAsset::Stellar, &-100);
    assert_eq!(result_neg, Err(Ok(Error::InvalidInput)));
}

#[test]
fn test_multiple_deposits() {
    let test = PredictifyTest::setup();
    let env = &test.env;
    let user = &test.user;
    let contract_address = &test.contract_id;
    let client = crate::PredictifyHybridClient::new(env, contract_address);

    env.mock_all_auths();

    // Deposit 1
    client.deposit(user, &ReflectorAsset::Stellar, &100);
    let b1 = client.get_balance(user, &ReflectorAsset::Stellar);
    assert_eq!(b1.amount, 100);

    // Deposit 2
    client.deposit(user, &ReflectorAsset::Stellar, &200);
    let b2 = client.get_balance(user, &ReflectorAsset::Stellar);
    assert_eq!(b2.amount, 300);
}

#[test]
fn test_deposit_invalid_asset() {
    let test = PredictifyTest::setup();
    let env = &test.env;
    let user = &test.user;
    let contract_address = &test.contract_id;
    let client = crate::PredictifyHybridClient::new(env, contract_address);

    env.mock_all_auths();

    // Try to deposit Bitcoin (not configured/supported yet in balances.rs match statement)
    // The current implementation in balances.rs returns Error::InvalidInput for non-Stellar assets
    // because it can't resolve the token client for them.
    let result = client.try_deposit(user, &ReflectorAsset::BTC, &100);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));
}
