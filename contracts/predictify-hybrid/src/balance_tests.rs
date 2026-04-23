#![cfg(test)]

use crate::errors::Error;
use crate::types::ReflectorAsset;
use soroban_sdk::{
    testutils::{Address as _, Events},
    token::{Client as TokenClient, StellarAssetClient},
    Address, Env, Symbol,
};

/// Simplified test setup for balance tests to avoid dependency on broken test.rs
pub struct BalanceTestSetup {
    pub env: Env,
    pub contract_id: Address,
    pub token_id: Address,
    pub admin: Address,
    pub user: Address,
}

impl BalanceTestSetup {
    pub fn setup() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Setup token
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        // Setup admin and user
        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        // Register and initialize contract
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let client = crate::PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);

        // Set token for the contract (simulate what PredictifyTest::setup does)
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Fund user
        let stellar_client = StellarAssetClient::new(&env, &token_id);
        stellar_client.mint(&user, &1000_0000000); // 1000 XLM

        Self {
            env,
            contract_id,
            token_id,
            admin,
            user,
        }
    }
}

#[test]
fn test_deposit_and_withdrawal_flow() {
    let setup = BalanceTestSetup::setup();
    let env = &setup.env;
    let user = &setup.user;
    let contract_address = &setup.contract_id;

    let token_client = TokenClient::new(env, &setup.token_id);
    assert_eq!(token_client.balance(user), 1000_0000000);
    assert_eq!(token_client.balance(contract_address), 0);

    let deposit_amount = 500_0000000;
    let client = crate::PredictifyHybridClient::new(env, contract_address);
    
    env.mock_all_auths();
    let balance = client.deposit(user, &ReflectorAsset::Stellar, &deposit_amount);

    assert_eq!(balance.amount, deposit_amount);
    let stored_balance = client.get_balance(user, &ReflectorAsset::Stellar);
    assert_eq!(stored_balance.amount, deposit_amount);
    assert_eq!(token_client.balance(user), 500_0000000);
    assert_eq!(token_client.balance(contract_address), 500_0000000);

    let withdraw_amount = 200_0000000;
    let balance_after_withdraw = client.withdraw(user, &ReflectorAsset::Stellar, &withdraw_amount);

    assert_eq!(balance_after_withdraw.amount, 300_0000000);
    let stored_balance_2 = client.get_balance(user, &ReflectorAsset::Stellar);
    assert_eq!(stored_balance_2.amount, 300_0000000);
    assert_eq!(token_client.balance(user), 700_0000000);
    assert_eq!(token_client.balance(contract_address), 300_0000000);
}

#[test]
fn test_insufficient_balance_withdrawal() {
    let setup = BalanceTestSetup::setup();
    let env = &setup.env;
    let user = &setup.user;
    let client = crate::PredictifyHybridClient::new(env, &setup.contract_id);
    env.mock_all_auths();

    client.deposit(user, &ReflectorAsset::Stellar, &100_0000000);

    let result = client.try_withdraw(user, &ReflectorAsset::Stellar, &150_0000000);
    assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
}

#[test]
fn test_invalid_deposit_amount() {
    let setup = BalanceTestSetup::setup();
    let env = &setup.env;
    let user = &setup.user;
    let client = crate::PredictifyHybridClient::new(env, &setup.contract_id);
    env.mock_all_auths();

    let result = client.try_deposit(user, &ReflectorAsset::Stellar, &0);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));

    let result_neg = client.try_deposit(user, &ReflectorAsset::Stellar, &-100);
    assert_eq!(result_neg, Err(Ok(Error::InvalidInput)));
}

#[test]
fn test_invalid_withdraw_amount() {
    let setup = BalanceTestSetup::setup();
    let env = &setup.env;
    let user = &setup.user;
    let client = crate::PredictifyHybridClient::new(env, &setup.contract_id);
    env.mock_all_auths();
    client.deposit(user, &ReflectorAsset::Stellar, &1000);

    let result = client.try_withdraw(user, &ReflectorAsset::Stellar, &0);
    assert_eq!(result, Err(Ok(Error::InvalidInput)));

    let result_neg = client.try_withdraw(user, &ReflectorAsset::Stellar, &-100);
    assert_eq!(result_neg, Err(Ok(Error::InvalidInput)));
}

#[test]
fn test_repeated_partial_withdrawals_until_zero() {
    let setup = BalanceTestSetup::setup();
    let env = &setup.env;
    let user = &setup.user;
    let client = crate::PredictifyHybridClient::new(env, &setup.contract_id);
    env.mock_all_auths();

    client.deposit(user, &ReflectorAsset::Stellar, &1000_0000000);

    client.withdraw(user, &ReflectorAsset::Stellar, &300_0000000);
    client.withdraw(user, &ReflectorAsset::Stellar, &300_0000000);
    client.withdraw(user, &ReflectorAsset::Stellar, &300_0000000);

    let b1 = client.get_balance(user, &ReflectorAsset::Stellar);
    assert_eq!(b1.amount, 100_0000000);

    client.withdraw(user, &ReflectorAsset::Stellar, &100_0000000);
    let b2 = client.get_balance(user, &ReflectorAsset::Stellar);
    assert_eq!(b2.amount, 0);

    let result = client.try_withdraw(user, &ReflectorAsset::Stellar, &1);
    assert_eq!(result, Err(Ok(Error::InsufficientBalance)));
}

#[test]
fn test_withdrawal_exact_balance() {
    let setup = BalanceTestSetup::setup();
    let env = &setup.env;
    let user = &setup.user;
    let client = crate::PredictifyHybridClient::new(env, &setup.contract_id);
    env.mock_all_auths();

    client.deposit(user, &ReflectorAsset::Stellar, &500);
    client.withdraw(user, &ReflectorAsset::Stellar, &500);
    
    let b = client.get_balance(user, &ReflectorAsset::Stellar);
    assert_eq!(b.amount, 0);
}

#[test]
fn test_large_deposit_amount() {
    let test = PredictifyTest::setup();
    let env = &test.env;
    let user = &test.user;
    let contract_address = &test.contract_id;
    let client = crate::PredictifyHybridClient::new(env, contract_address);

    env.mock_all_auths();

    // Try to deposit more than i128::MAX / 2
    let large_amount = (i128::MAX / 2) + 1;
    let result = client.try_deposit(user, &ReflectorAsset::Stellar, &large_amount);
    
    // This should fail due to my new check
    assert_eq!(result, Err(Ok(Error::InvalidInput)));
}

#[test]
fn test_deposit_and_withdraw_full_balance() {
    let test = PredictifyTest::setup();
    let env = &test.env;
    let user = &test.user;
    let contract_address = &test.contract_id;
    let client = crate::PredictifyHybridClient::new(env, contract_address);

    env.mock_all_auths();

    let amount = 1000_0000000;
    client.deposit(user, &ReflectorAsset::Stellar, &amount);
    
    let b1 = client.get_balance(user, &ReflectorAsset::Stellar);
    assert_eq!(b1.amount, amount);

    client.withdraw(user, &ReflectorAsset::Stellar, &amount);
    let b2 = client.get_balance(user, &ReflectorAsset::Stellar);
    assert_eq!(b2.amount, 0);
}

