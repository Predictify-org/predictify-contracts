#![cfg(test)]

use crate::statistics::StatisticsManager;
use crate::PredictifyHybrid;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup_env() -> (Env, Address) {
    let env = Env::default();
    let contract_id = env.register_contract(None, PredictifyHybrid);
    (env, contract_id)
}

#[test]
fn test_platform_stats_initialization() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        let stats = StatisticsManager::get_platform_stats(&env);
        assert_eq!(stats.total_events_created, 0);
        assert_eq!(stats.total_bets_placed, 0);
        assert_eq!(stats.total_volume, 0);
        assert_eq!(stats.total_fees_collected, 0);
        assert_eq!(stats.active_events_count, 0);
    });
}

#[test]
fn test_record_market_created() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        StatisticsManager::record_market_created(&env);

        let stats = StatisticsManager::get_platform_stats(&env);
        assert_eq!(stats.total_events_created, 1);
        assert_eq!(stats.active_events_count, 1);

        StatisticsManager::record_market_created(&env);

        let stats2 = StatisticsManager::get_platform_stats(&env);
        assert_eq!(stats2.total_events_created, 2);
        assert_eq!(stats2.active_events_count, 2);
    });
}

#[test]
fn test_record_market_resolved() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        StatisticsManager::record_market_created(&env);
        StatisticsManager::record_market_created(&env);

        let before = StatisticsManager::get_platform_stats(&env);
        assert_eq!(before.active_events_count, 2);

        StatisticsManager::record_market_resolved(&env);

        let after = StatisticsManager::get_platform_stats(&env);
        assert_eq!(after.active_events_count, 1);
        assert_eq!(after.total_events_created, 2);
    });
}

#[test]
fn test_record_bet_placed() {
    let (env, contract_id) = setup_env();
    let user = Address::generate(&env);
    let amount = 100_000_000i128;

    env.as_contract(&contract_id, || {
        StatisticsManager::record_bet_placed(&env, &user, amount);

        let p_stats = StatisticsManager::get_platform_stats(&env);
        assert_eq!(p_stats.total_bets_placed, 1);
        assert_eq!(p_stats.total_volume, amount);

        let u_stats = StatisticsManager::get_user_stats(&env, &user);
        assert_eq!(u_stats.total_bets_placed, 1);
        assert_eq!(u_stats.total_amount_wagered, amount);
    });
}

#[test]
fn test_user_stats_initialization() {
    let (env, contract_id) = setup_env();
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        let stats = StatisticsManager::get_user_stats(&env, &user);
        assert_eq!(stats.total_bets_placed, 0);
        assert_eq!(stats.total_amount_wagered, 0);
        assert_eq!(stats.total_winnings, 0);
        assert_eq!(stats.total_bets_won, 0);
        assert_eq!(stats.win_rate, 0);
    });
}

#[test]
fn test_record_winnings_claimed() {
    let (env, contract_id) = setup_env();
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        StatisticsManager::record_bet_placed(&env, &user, 100);
        StatisticsManager::record_bet_placed(&env, &user, 100);

        StatisticsManager::record_winnings_claimed(&env, &user, 150);

        let u_stats = StatisticsManager::get_user_stats(&env, &user);
        assert_eq!(u_stats.total_winnings, 150);
        assert_eq!(u_stats.total_bets_won, 1);
        assert_eq!(u_stats.win_rate, 5000);
    });
}

#[test]
fn test_record_market_resolved_underflow_protection() {
    let (env, contract_id) = setup_env();

    env.as_contract(&contract_id, || {
        // Start with 0 active events
        let initial_stats = StatisticsManager::get_platform_stats(&env);
        assert_eq!(initial_stats.active_events_count, 0);

        // Try to resolve a market when none are active
        StatisticsManager::record_market_resolved(&env);

        // Should remain 0 (no underflow)
        let after_stats = StatisticsManager::get_platform_stats(&env);
        assert_eq!(after_stats.active_events_count, 0);

        // Now create one and resolve it
        StatisticsManager::record_market_created(&env);
        let created_stats = StatisticsManager::get_platform_stats(&env);
        assert_eq!(created_stats.active_events_count, 1);

        StatisticsManager::record_market_resolved(&env);
        let resolved_stats = StatisticsManager::get_platform_stats(&env);
        assert_eq!(resolved_stats.active_events_count, 0);
    });
}
