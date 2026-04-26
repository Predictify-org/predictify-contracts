#![cfg(test)]
extern crate std;

use alloc::vec;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Symbol,
};

use crate::{
    disputes::{
        DisputeManager, DisputeUtils, DisputeFeeDistribution, DisputeVoting,
        DisputeVotingStatus, DisputeVote,
    },
    types::{Error, Market, MarketState, OracleConfig, DisputeStatus},
    voting::VotingUtils,
};

fn setup_env_and_market() -> (Env, Address, Symbol) {
    let env = Env::default();
    let admin = Address::generate(&env);
    let market_id = Symbol::new(&env, "BTC_50K");

    // Initialize market minimally for voting utils
    let market = Market {
        admin: admin.clone(),
        question: String::from_str(&env, "Will BTC hit 50k?"),
        outcomes: vec![
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
        end_time: env.ledger().timestamp() + 3600,
        oracle_config: OracleConfig {
            feed_id: Symbol::new(&env, "BTC_USD"),
            oracle_address: admin.clone(),
            minimum_confidence: 80,
            required_validations: 1,
            fallback_duration: 3600,
        },
        state: MarketState::Active,
        total_staked: 0,
        bets: vec![],
        votes: soroban_sdk::Map::new(&env),
        stakes: soroban_sdk::Map::new(&env),
        disputes: vec![],
        dispute_stakes: soroban_sdk::Map::new(&env),
        resolutions: vec![],
        winning_outcomes: None,
        claimed: soroban_sdk::Map::new(&env),
        created_at: env.ledger().timestamp(),
        updated_at: env.ledger().timestamp(),
        fee_collected: false,
        resolution_duration: 3600,
        dispute_window_seconds: 3600,
        extensions_count: 0,
        metadata: None,
        tags: vec![],
    };

    crate::markets::MarketStateManager::update_market(&env, &market_id, &market);
    (env, admin, market_id)
}

#[test]
fn test_normal_case_valid_dispute_resolution() {
    let (env, admin, market_id) = setup_env_and_market();
    let user1 = Address::generate(&env); // Initiator / Support
    let user2 = Address::generate(&env); // Against

    env.mock_all_auths();

    // Give users balance for mocking token transfer interactions
    crate::storage::BalanceStorage::add_balance(&env, &user1, &crate::types::ReflectorAsset::Stellar, 1000).unwrap();
    crate::storage::BalanceStorage::add_balance(&env, &user2, &crate::types::ReflectorAsset::Stellar, 1000).unwrap();

    let init_stake = 100;
    DisputeManager::process_dispute(&env, user1.clone(), market_id.clone(), String::from_str(&env, "It was Yes!"), init_stake).unwrap();

    let vote_stake = 50;
    DisputeManager::vote_on_dispute(&env, user2.clone(), market_id.clone(), market_id.clone(), false, vote_stake, String::from_str(&env, "It was No!")).unwrap();

    // Distribute fees (outcome=true because 100 (support) > 50 (against))
    let distribution = DisputeManager::distribute_dispute_fees(&env, market_id.clone()).unwrap();
    assert_eq!(distribution.winner_stake, 100);
    assert_eq!(distribution.loser_stake, 50);

    // Initial claim for user1 (winner)
    let payout = DisputeManager::claim_dispute_winnings(&env, market_id.clone(), user1.clone()).unwrap();
    
    // Proportional payout: original_stake + original_stake * loser_pool / winner_pool
    // 100 + (100 * 50) / 100 = 150
    assert_eq!(payout, 150);

    // Assert double claim fails
    let err = DisputeManager::claim_dispute_winnings(&env, market_id.clone(), user1.clone()).unwrap_err();
    assert_eq!(err, Error::AlreadyClaimed);
}

#[test]
fn test_losing_party_extraction_attempt() {
    let (env, admin, market_id) = setup_env_and_market();
    let user1 = Address::generate(&env); // Initiator / Support
    let loser = Address::generate(&env); // Against

    env.mock_all_auths();

    crate::storage::BalanceStorage::add_balance(&env, &user1, &crate::types::ReflectorAsset::Stellar, 1000).unwrap();
    crate::storage::BalanceStorage::add_balance(&env, &loser, &crate::types::ReflectorAsset::Stellar, 1000).unwrap();

    DisputeManager::process_dispute(&env, user1.clone(), market_id.clone(), String::from_str(&env, "It was Yes!"), 200).unwrap();
    DisputeManager::vote_on_dispute(&env, loser.clone(), market_id.clone(), market_id.clone(), false, 50, String::from_str(&env, "It was No!")).unwrap();

    // Distribute fees
    DisputeManager::distribute_dispute_fees(&env, market_id.clone()).unwrap();

    // Loser tries to claim
    let err = DisputeManager::claim_dispute_winnings(&env, market_id.clone(), loser.clone()).unwrap_err();
    assert_eq!(err, Error::NothingToClaim);
}

#[test]
fn test_edge_case_zero_stake() {
    let (env, admin, market_id) = setup_env_and_market();
    let user1 = Address::generate(&env);

    env.mock_all_auths();

    crate::storage::BalanceStorage::add_balance(&env, &user1, &crate::types::ReflectorAsset::Stellar, 1000).unwrap();

    // Initiator stakes
    DisputeManager::process_dispute(&env, user1.clone(), market_id.clone(), String::from_str(&env, "Only me"), 100).unwrap();

    // Distributed with NO against votes
    DisputeManager::distribute_dispute_fees(&env, market_id.clone()).unwrap();

    // Claim
    let payout = DisputeManager::claim_dispute_winnings(&env, market_id.clone(), user1.clone()).unwrap();
    // 100 + (100 * 0) / 100 = 100
    assert_eq!(payout, 100);
}
