#![cfg(test)]

use crate::{PredictifyHybrid, PredictifyHybridClient};
use crate::types::{OracleConfig, OracleProvider};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, Address, Env, String, Symbol,
};

// Test setup with flexible token configuration
struct CustomTokenTestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    token_id: Address,
    token_admin: Address,
    market_id: Symbol,
}

impl CustomTokenTestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Setup admin
        let admin = Address::generate(&env);

        // Register contract
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);

        // Setup custom token
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        // Configure contract to use this token
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Create a test market
        let outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];

        let oracle_address = Address::generate(&env);
        let market_id = client.create_market(
            &admin,
            &String::from_str(&env, "Will it rain?"),
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::Reflector,
                oracle_address: oracle_address.clone(),
                feed_id: String::from_str(&env, "RAIN"),
                threshold: 1,
                comparison: String::from_str(&env, "gt"),
            },
            &None,       // fallback_oracle_config
            &3600,       // resolution_timeout
            &None,       // min_pool_size
            &None,       // bet_deadline_mins_before_end
            &None,       // dispute_window_seconds
        );

        Self {
            env,
            contract_id,
            admin,
            token_id,
            token_admin,
            market_id,
        }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    fn token_admin_client(&self) -> StellarAssetClient<'_> {
        StellarAssetClient::new(&self.env, &self.token_id)
    }

    fn token_client(&self) -> soroban_sdk::token::Client<'_> {
        soroban_sdk::token::Client::new(&self.env, &self.token_id)
    }
}

#[test]
fn test_bet_placement_with_custom_token() {
    let setup = CustomTokenTestSetup::new();
    let client = setup.client();
    let token_admin_client = setup.token_admin_client();
    let token_client = setup.token_client();
    
    let user = Address::generate(&setup.env);
    let bet_amount = 10_000_000; // 1 token

    // Mint tokens to user
    token_admin_client.mint(&user, &100_000_000); // 10 tokens

    // Place bet
    client.place_bet(
        &user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &bet_amount,
    );

    // Verify balance decreased
    assert_eq!(token_client.balance(&user), 90_000_000);

    // Verify contract balance increased
    assert_eq!(token_client.balance(&setup.contract_id), bet_amount);
}

#[test]
fn test_insufficient_balance() {
    let setup = CustomTokenTestSetup::new();
    let client = setup.client();
    let token_admin_client = setup.token_admin_client();
    
    let user = Address::generate(&setup.env);
    let bet_amount = 10_000_000;

    // Mint LESS tokens than bet amount
    token_admin_client.mint(&user, &5_000_000); // 0.5 tokens

    // Attempt to place bet using try_place_bet to avoid panics/segfaults
    let result = client.try_place_bet(
        &user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &bet_amount,
    );
    
    // Should return an error (likely HostError due to transfer failure)
    assert!(result.is_err());
}

#[test]
fn test_payout_distribution_flow() {
    let setup = CustomTokenTestSetup::new();
    let client = setup.client();
    let token_admin_client = setup.token_admin_client();
    let token_client = setup.token_client();
    
    let user_winner = Address::generate(&setup.env);
    let user_loser = Address::generate(&setup.env);
    let bet_amount = 10_000_000;

    // Mint tokens
    token_admin_client.mint(&user_winner, &100_000_000);
    token_admin_client.mint(&user_loser, &100_000_000);

    // Place bets
    client.place_bet(
        &user_winner,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &bet_amount,
    );

    client.place_bet(
        &user_loser,
        &setup.market_id,
        &String::from_str(&setup.env, "no"),
        &bet_amount,
    );

    // Advance time to end market but NOT past dispute window
    let market = client.get_market(&setup.market_id).unwrap();
    setup.env.ledger().set(LedgerInfo {
        timestamp: market.end_time + 1,
        protocol_version: 22,
        sequence_number: setup.env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });

    // Resolve market manually (Admin wins "yes")
    client.resolve_market_manual(
        &setup.admin,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
    );

    // Verify market is resolved but no payout distributed yet (due to dispute window)
    let market_after = client.get_market(&setup.market_id).unwrap();
    assert!(market_after.winning_outcomes.is_some());
    assert_eq!(token_client.balance(&user_winner), 90_000_000); // Only initial balance minus bet

    // Advance time past dispute window (24h default)
    setup.env.ledger().set(LedgerInfo {
        timestamp: market.end_time + 86400 + 1,
        protocol_version: 22,
        sequence_number: setup.env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });

    // Distribute payouts
    let total_distributed = client.distribute_payouts(&setup.market_id);
    assert!(total_distributed > 0);

    // Verify winner received payout (Original stake + winnings - fees)
    // Winner staked 10M, Loser staked 10M. Total pool 20M.
    // Fee 2% = 400k. Payout pool = 19.6M.
    // Winner share = 100%. Payout = 19.6M.
    // Final balance = 90M + 19.6M = 109.6M
    let winner_balance = token_client.balance(&user_winner);
    assert_eq!(winner_balance, 109_600_000);

    // Loser gets nothing
    assert_eq!(token_client.balance(&user_loser), 90_000_000);
}

#[test]
fn test_switch_token_support() {
    // This test verifies that we can switch the token used by the contract
    // by updating the TokenID storage key.
    
    let setup = CustomTokenTestSetup::new();
    let token1_client = setup.token_client();
    let client = setup.client();
    
    // 1. Verify betting with Token 1
    let user1 = Address::generate(&setup.env);
    setup.token_admin_client().mint(&user1, &10_000_000);
    
    client.place_bet(
        &user1,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &10_000_000,
    );
    assert_eq!(token1_client.balance(&user1), 0);
    
    // 2. Create and switch to Token 2
    let token2_admin = Address::generate(&setup.env);
    let token2_contract = setup.env.register_stellar_asset_contract_v2(token2_admin.clone());
    let token2_id = token2_contract.address();
    let token2_admin_client = StellarAssetClient::new(&setup.env, &token2_id);
    let token2_client = soroban_sdk::token::Client::new(&setup.env, &token2_id);

    // Update contract storage to use Token 2
    setup.env.as_contract(&setup.contract_id, || {
        setup.env.storage()
            .persistent()
            .set(&Symbol::new(&setup.env, "TokenID"), &token2_id);
    });

    // 3. Verify betting with Token 2
    let user2 = Address::generate(&setup.env);
    token2_admin_client.mint(&user2, &20_000_000);
    
    // Bet on existing one is fine.
    client.place_bet(
        &user2,
        &setup.market_id,
        &String::from_str(&setup.env, "no"),
        &20_000_000,
    );
    
    // Verify balances for Token 2
    assert_eq!(token2_client.balance(&user2), 0);
    assert_eq!(token2_client.balance(&setup.contract_id), 20_000_000);
    
    // Verify Token 1 balances are unchanged
    assert_eq!(token1_client.balance(&setup.contract_id), 10_000_000);
}

#[test]
fn test_cancel_refund_custom_token() {
    let setup = CustomTokenTestSetup::new();
    let client = setup.client();
    let token_admin_client = setup.token_admin_client();
    let token_client = setup.token_client();
    
    let user = Address::generate(&setup.env);
    let bet_amount = 10_000_000;

    // Mint tokens
    token_admin_client.mint(&user, &20_000_000);

    // Place bet
    client.place_bet(
        &user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &bet_amount,
    );

    // Verify balance before cancellation
    assert_eq!(token_client.balance(&user), 10_000_000);
    assert_eq!(token_client.balance(&setup.contract_id), bet_amount);

    // Cancel event
    client.cancel_event(
        &setup.admin,
        &setup.market_id,
        &Some(String::from_str(&setup.env, "Technical issue")),
    );

    // Verify balance after cancellation (should be refunded)
    assert_eq!(token_client.balance(&user), 20_000_000);
    assert_eq!(token_client.balance(&setup.contract_id), 0);
}

#[test]
fn test_fee_collection_custom_token() {
    let setup = CustomTokenTestSetup::new();
    let client = setup.client();
    let token_admin_client = setup.token_admin_client();
    let token_client = setup.token_client();
    
    let user = Address::generate(&setup.env);
    let bet_amount = 100_000_000;

    // Mint tokens
    token_admin_client.mint(&user, &bet_amount);

    // Place bet
    client.place_bet(
        &user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &bet_amount,
    );

    // Advance time to end market
    let market = client.get_market(&setup.market_id).unwrap();
    setup.env.ledger().set(LedgerInfo {
        timestamp: market.end_time + 1,
        protocol_version: 22,
        sequence_number: setup.env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });

    // Resolve market
    client.resolve_market_manual(
        &setup.admin,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
    );

    // Collect fees
    let fee_amount = client.collect_fees(
        &setup.admin,
        &setup.market_id,
    );

    // Verify admin balance has not increased yet (as fees are in vault)
    let admin_balance_before = token_client.balance(&setup.admin);
    assert_eq!(admin_balance_before, 0);

    // Withdraw fees from vault
    let withdrawn_amount = client.withdraw_fees(&setup.admin, &fee_amount);
    assert_eq!(withdrawn_amount, fee_amount);

    // Verify admin balance increased by withdrawn amount
    let admin_balance_after = token_client.balance(&setup.admin);
    assert_eq!(admin_balance_after, fee_amount);
}
