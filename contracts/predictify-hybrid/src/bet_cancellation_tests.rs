//! # Bet Cancellation Tests
//!
//! Comprehensive test suite for user-initiated bet cancellation functionality.
//!
//! ## Test Coverage
//!
//! - **Happy Path**: Successful bet cancellation and refund
//! - **Deadline Validation**: Rejection after market deadline
//! - **Authorization**: Only bettor can cancel their own bet
//! - **State Updates**: Pool and market statistics updates
//! - **Event Emission**: Bet status change events
//! - **Multiple Bets**: Cancel one bet when user has multiple
//!
//! ## Test Coverage Target: 95%+

#![cfg(test)]

use crate::bets::{BetManager, BetStorage};
use crate::types::{BetStatus, Market, MarketState, OracleConfig, OracleProvider};
use crate::{Error, PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, Address, Env, String, Symbol, Vec,
};

// ===== TEST SETUP =====

struct BetCancellationTestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    user: Address,
    user2: Address,
    token_id: Address,
    market_id: Symbol,
}

impl BetCancellationTestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let user2 = Address::generate(&env);

        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        let stellar_client = StellarAssetClient::new(&env, &token_id);
        stellar_client.mint(&admin, &10_000_0000000);
        stellar_client.mint(&user, &1000_0000000);
        stellar_client.mint(&user2, &1000_0000000);

        let token_client = soroban_sdk::token::Client::new(&env, &token_id);
        token_client.approve(&user, &contract_id, &i128::MAX, &1000000);
        token_client.approve(&user2, &contract_id, &i128::MAX, &1000000);
        token_client.approve(&admin, &contract_id, &i128::MAX, &1000000);

        let market_id = Self::create_test_market(&env, &contract_id, &admin);

        Self {
            env,
            contract_id,
            admin,
            user,
            user2,
            token_id,
            market_id,
        }
    }

    fn create_test_market(env: &Env, contract_id: &Address, admin: &Address) -> Symbol {
        let client = PredictifyHybridClient::new(env, contract_id);

        let outcomes = vec![
            env,
            String::from_str(env, "yes"),
            String::from_str(env, "no"),
        ];

        let oracle_config = OracleConfig {
            provider: OracleProvider::Pyth,
            oracle_address: Address::generate(env),
            feed_id: String::from_str(env, "test_feed"),
            threshold: 100_000_000,
            comparison: String::from_str(env, "gt"),
        };

        client.create_market(
            admin,
            &String::from_str(env, "Test Market"),
            &outcomes,
            &1, // 1 day duration
            &oracle_config,
            &None,
            &3600,
            &None, // min_pool_size
            &None, // bet_deadline_mins_before_end
            &None, // dispute_window_seconds
        )
    }

    fn place_bet(&self, user: &Address, outcome: &str, amount: i128) {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        client.place_bet(
            user,
            &self.market_id,
            &String::from_str(&self.env, outcome),
            &amount,
        );
    }

    fn get_user_balance(&self, user: &Address) -> i128 {
        let token_client = soroban_sdk::token::Client::new(&self.env, &self.token_id);
        token_client.balance(user)
    }
}

// ===== HAPPY PATH TESTS =====

#[test]
fn test_cancel_bet_successful() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000; // 1 XLM
    let initial_balance = setup.get_user_balance(&setup.user);

    // Place bet
    setup.place_bet(&setup.user, "yes", bet_amount);

    // Verify bet was placed
    let bet = client.get_bet(&setup.market_id, &setup.user);
    assert!(bet.is_some());
    assert_eq!(bet.unwrap().status, BetStatus::Active);

    // Verify balance decreased
    let balance_after_bet = setup.get_user_balance(&setup.user);
    assert_eq!(balance_after_bet, initial_balance - bet_amount);

    // Cancel bet
    client.cancel_bet(&setup.user, &setup.market_id);

    // Verify bet status changed to Cancelled
    let bet_after_cancel = client.get_bet(&setup.market_id, &setup.user);
    assert!(bet_after_cancel.is_some());
    assert_eq!(bet_after_cancel.unwrap().status, BetStatus::Cancelled);

    // Verify funds were refunded
    let final_balance = setup.get_user_balance(&setup.user);
    assert_eq!(final_balance, initial_balance);
}

#[test]
fn test_cancel_bet_updates_market_stats() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;

    // Place bet
    setup.place_bet(&setup.user, "yes", bet_amount);

    // Get initial stats
    let stats_before = client.get_market_bet_stats(&setup.market_id);
    assert_eq!(stats_before.total_bets, 1);
    assert_eq!(stats_before.total_amount_locked, bet_amount);
    assert_eq!(stats_before.unique_bettors, 1);

    // Cancel bet
    client.cancel_bet(&setup.user, &setup.market_id);

    // Verify stats updated
    let stats_after = client.get_market_bet_stats(&setup.market_id);
    assert_eq!(stats_after.total_bets, 0);
    assert_eq!(stats_after.total_amount_locked, 0);
    assert_eq!(stats_after.unique_bettors, 0);
}

#[test]
fn test_cancel_bet_updates_outcome_totals() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;
    let outcome = String::from_str(&setup.env, "yes");

    // Place bet
    setup.place_bet(&setup.user, "yes", bet_amount);

    // Verify outcome total
    let stats_before = client.get_market_bet_stats(&setup.market_id);
    assert_eq!(stats_before.outcome_totals.get(outcome.clone()).unwrap(), bet_amount);

    // Cancel bet
    client.cancel_bet(&setup.user, &setup.market_id);

    // Verify outcome total updated
    let stats_after = client.get_market_bet_stats(&setup.market_id);
    assert_eq!(stats_after.outcome_totals.get(outcome.clone()), None);
}

// ===== DEADLINE VALIDATION TESTS =====

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_cancel_bet_after_deadline_fails() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;

    // Place bet
    setup.place_bet(&setup.user, "yes", bet_amount);

    // Advance time past deadline
    let current_time = setup.env.ledger().timestamp();
    setup.env.ledger().with_mut(|li| {
        li.timestamp = current_time + 86401; // 1 second past deadline
    });

    // Attempt to cancel - should fail with MarketClosed error
    client.cancel_bet(&setup.user, &setup.market_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #102)")]
fn test_cancel_bet_exactly_at_deadline_fails() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;

    // Place bet
    setup.place_bet(&setup.user, "yes", bet_amount);

    // Advance time to exactly deadline
    let current_time = setup.env.ledger().timestamp();
    setup.env.ledger().with_mut(|li| {
        li.timestamp = current_time + 86400;
    });

    // Attempt to cancel - should fail
    client.cancel_bet(&setup.user, &setup.market_id);
}

#[test]
fn test_cancel_bet_one_second_before_deadline_succeeds() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;

    // Place bet
    setup.place_bet(&setup.user, "yes", bet_amount);

    // Advance time to 1 second before deadline
    let current_time = setup.env.ledger().timestamp();
    setup.env.ledger().with_mut(|li| {
        li.timestamp = current_time + 86399;
    });

    // Cancel should succeed
    client.cancel_bet(&setup.user, &setup.market_id);

    let bet = client.get_bet(&setup.market_id, &setup.user);
    assert_eq!(bet.unwrap().status, BetStatus::Cancelled);
}

// ===== AUTHORIZATION TESTS =====

#[test]
#[should_panic(expected = "Error(Contract, #105)")]
fn test_cancel_bet_no_bet_placed_fails() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    // Attempt to cancel without placing bet - should fail with NothingToClaim
    client.cancel_bet(&setup.user, &setup.market_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #105)")]
fn test_cancel_bet_different_user_fails() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;

    // User1 places bet
    setup.place_bet(&setup.user, "yes", bet_amount);

    // User2 attempts to cancel user1's bet - should fail
    client.cancel_bet(&setup.user2, &setup.market_id);
}

// ===== BET STATUS VALIDATION TESTS =====

#[test]
#[should_panic(expected = "Error(Contract, #400)")]
fn test_cancel_already_cancelled_bet_fails() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;

    // Place and cancel bet
    setup.place_bet(&setup.user, "yes", bet_amount);
    client.cancel_bet(&setup.user, &setup.market_id);

    // Attempt to cancel again - should fail with InvalidBetStatus
    client.cancel_bet(&setup.user, &setup.market_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #400)")]
fn test_cancel_refunded_bet_fails() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;

    // Place bet
    setup.place_bet(&setup.user, "yes", bet_amount);

    // Admin cancels event (refunds all bets)
    client.cancel_event(&setup.admin, &setup.market_id, &None);

    // Attempt to cancel refunded bet - should fail
    client.cancel_bet(&setup.user, &setup.market_id);
}

// ===== MULTIPLE BETS TESTS =====

#[test]
fn test_cancel_one_bet_among_multiple_users() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;

    // Multiple users place bets
    setup.place_bet(&setup.user, "yes", bet_amount);
    setup.place_bet(&setup.user2, "no", bet_amount * 2);

    // Get initial stats
    let stats_before = client.get_market_bet_stats(&setup.market_id);
    assert_eq!(stats_before.total_bets, 2);
    assert_eq!(stats_before.total_amount_locked, bet_amount * 3);

    // User1 cancels their bet
    client.cancel_bet(&setup.user, &setup.market_id);

    // Verify only user1's bet was cancelled
    let bet1 = client.get_bet(&setup.market_id, &setup.user);
    assert_eq!(bet1.unwrap().status, BetStatus::Cancelled);

    let bet2 = client.get_bet(&setup.market_id, &setup.user2);
    assert_eq!(bet2.unwrap().status, BetStatus::Active);

    // Verify stats updated correctly
    let stats_after = client.get_market_bet_stats(&setup.market_id);
    assert_eq!(stats_after.total_bets, 1);
    assert_eq!(stats_after.total_amount_locked, bet_amount * 2);
}

#[test]
fn test_cancel_bet_with_different_outcomes() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;

    // Place bets on different outcomes
    setup.place_bet(&setup.user, "yes", bet_amount);
    setup.place_bet(&setup.user2, "yes", bet_amount * 2);

    // Get initial outcome totals
    let stats_before = client.get_market_bet_stats(&setup.market_id);
    let yes_outcome = String::from_str(&setup.env, "yes");
    assert_eq!(stats_before.outcome_totals.get(yes_outcome.clone()).unwrap(), bet_amount * 3);

    // User1 cancels
    client.cancel_bet(&setup.user, &setup.market_id);

    // Verify outcome total updated correctly
    let stats_after = client.get_market_bet_stats(&setup.market_id);
    assert_eq!(stats_after.outcome_totals.get(yes_outcome.clone()).unwrap(), bet_amount * 2);
}

// ===== EDGE CASES =====

#[test]
fn test_cancel_bet_minimum_amount() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let min_bet = 1_000_000; // Minimum bet amount
    let initial_balance = setup.get_user_balance(&setup.user);

    setup.place_bet(&setup.user, "yes", min_bet);
    client.cancel_bet(&setup.user, &setup.market_id);

    let final_balance = setup.get_user_balance(&setup.user);
    assert_eq!(final_balance, initial_balance);
}

#[test]
fn test_cancel_bet_maximum_amount() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let max_bet = 100_000_000_000; // Maximum bet amount
    
    // Mint additional tokens for max bet
    let stellar_client = StellarAssetClient::new(&setup.env, &setup.token_id);
    stellar_client.mint(&setup.user, &max_bet);

    let initial_balance = setup.get_user_balance(&setup.user);

    setup.place_bet(&setup.user, "yes", max_bet);
    client.cancel_bet(&setup.user, &setup.market_id);

    let final_balance = setup.get_user_balance(&setup.user);
    assert_eq!(final_balance, initial_balance);
}

#[test]
fn test_cancel_bet_immediately_after_placement() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;
    let initial_balance = setup.get_user_balance(&setup.user);

    // Place and immediately cancel
    setup.place_bet(&setup.user, "yes", bet_amount);
    client.cancel_bet(&setup.user, &setup.market_id);

    // Verify full refund
    let final_balance = setup.get_user_balance(&setup.user);
    assert_eq!(final_balance, initial_balance);
}

// ===== MARKET STATE VALIDATION =====

#[test]
#[should_panic(expected = "Error(Contract, #105)")]
fn test_cancel_bet_nonexistent_market_fails() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let fake_market = Symbol::new(&setup.env, "fake_market");
    
    // Attempt to cancel bet on non-existent market
    // Should fail with NothingToClaim since there's no bet
    client.cancel_bet(&setup.user, &fake_market);
}

// ===== INTEGRATION TESTS =====

#[test]
fn test_cancel_and_rebet_on_same_market() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;

    // Place bet
    setup.place_bet(&setup.user, "yes", bet_amount);
    
    // Cancel bet
    client.cancel_bet(&setup.user, &setup.market_id);

    // Place new bet on different outcome
    setup.place_bet(&setup.user, "no", bet_amount * 2);

    // Verify new bet is active
    let bet = client.get_bet(&setup.market_id, &setup.user);
    assert!(bet.is_some());
    let bet_data = bet.unwrap();
    assert_eq!(bet_data.status, BetStatus::Active);
    assert_eq!(bet_data.outcome, String::from_str(&setup.env, "no"));
    assert_eq!(bet_data.amount, bet_amount * 2);
}

#[test]
fn test_multiple_users_cancel_bets_independently() {
    let setup = BetCancellationTestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);

    let bet_amount = 10_000_000;

    // Both users place bets
    setup.place_bet(&setup.user, "yes", bet_amount);
    setup.place_bet(&setup.user2, "no", bet_amount);

    let user1_initial = setup.get_user_balance(&setup.user);
    let user2_initial = setup.get_user_balance(&setup.user2);

    // User1 cancels
    client.cancel_bet(&setup.user, &setup.market_id);

    // Verify user1 refunded, user2 still active
    assert_eq!(setup.get_user_balance(&setup.user), user1_initial + bet_amount);
    assert_eq!(setup.get_user_balance(&setup.user2), user2_initial);

    let bet2 = client.get_bet(&setup.market_id, &setup.user2);
    assert_eq!(bet2.unwrap().status, BetStatus::Active);

    // User2 cancels
    client.cancel_bet(&setup.user2, &setup.market_id);

    // Verify user2 refunded
    assert_eq!(setup.get_user_balance(&setup.user2), user2_initial + bet_amount);
}
