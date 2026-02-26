#![cfg(test)]

use crate::config::ConfigManager;
use crate::types::{Market, MarketState, OracleConfig, OracleProvider, ReflectorAsset};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
use soroban_sdk::{vec, Address, Env, String, Symbol};

struct TimeoutSweepSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    treasury: Address,
    winner_1: Address,
    winner_2: Address,
    loser: Address,
    market_id: Symbol,
    end_time: u64,
}

impl TimeoutSweepSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        let winner_1 = Address::generate(&env);
        let winner_2 = Address::generate(&env);
        let loser = Address::generate(&env);

        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);

        env.as_contract(&contract_id, || {
            let cfg = ConfigManager::get_development_config(&env);
            ConfigManager::store_config(&env, &cfg).unwrap();
        });

        let market_id = Symbol::new(&env, "claim_to_1");
        let end_time = 10_000;

        env.ledger().set(LedgerInfo {
            timestamp: 9_000,
            protocol_version: 22,
            sequence_number: env.ledger().sequence() + 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 16,
            max_entry_ttl: 6_312_000,
        });

        env.as_contract(&contract_id, || {
            let outcomes = vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ];

            let mut market = Market::new(
                &env,
                admin.clone(),
                String::from_str(&env, "Will outcome be yes?"),
                outcomes,
                end_time,
                OracleConfig {
                    provider: OracleProvider::Reflector,
                    oracle_address: Address::from_str(
                        &env,
                        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                    ),
                    feed_id: String::from_str(&env, "TEST/YES"),
                    threshold: 1,
                    comparison: String::from_str(&env, "gt"),
                },
                None,
                86_400,
                MarketState::Resolved,
            );

            market
                .votes
                .set(winner_1.clone(), String::from_str(&env, "yes"));
            market
                .votes
                .set(winner_2.clone(), String::from_str(&env, "yes"));
            market
                .votes
                .set(loser.clone(), String::from_str(&env, "no"));

            market.stakes.set(winner_1.clone(), 1_000_000);
            market.stakes.set(winner_2.clone(), 1_000_000);
            market.stakes.set(loser.clone(), 1_000_000);
            market.total_staked = 3_000_000;

            let mut winning_outcomes = soroban_sdk::Vec::new(&env);
            winning_outcomes.push_back(String::from_str(&env, "yes"));
            market.winning_outcomes = Some(winning_outcomes);

            env.storage().persistent().set(&market_id, &market);
        });

        Self {
            env,
            contract_id,
            admin,
            treasury,
            winner_1,
            winner_2,
            loser,
            market_id,
            end_time,
        }
    }

    fn set_time(&self, timestamp: u64) {
        self.env.ledger().set(LedgerInfo {
            timestamp,
            protocol_version: 22,
            sequence_number: self.env.ledger().sequence() + 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 16,
            max_entry_ttl: 6_312_000,
        });
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }
}

#[test]
#[should_panic(expected = "Error(Contract, #400)")]
fn test_sweep_blocked_before_claim_period_end() {
    let setup = TimeoutSweepSetup::new();

    setup
        .client()
        .set_global_claim_period(&setup.admin, &1_000u64);

    setup.set_time(setup.end_time + 999);

    setup
        .client()
        .sweep_unclaimed_winnings(&setup.admin, &setup.market_id, &false);
}

#[test]
fn test_sweep_after_timeout_only_unclaimed_to_treasury() {
    let setup = TimeoutSweepSetup::new();

    setup
        .client()
        .set_global_claim_period(&setup.admin, &100u64);
    setup.client().set_treasury(&setup.admin, &setup.treasury);

    setup.set_time(setup.end_time + 50);
    setup
        .client()
        .claim_winnings(&setup.winner_1, &setup.market_id);

    setup.set_time(setup.end_time + 100);
    let swept = setup
        .client()
        .sweep_unclaimed_winnings(&setup.admin, &setup.market_id, &false);

    assert!(swept > 0);

    let treasury_balance = setup
        .client()
        .get_balance(&setup.treasury, &ReflectorAsset::Stellar)
        .amount;
    assert_eq!(treasury_balance, swept);

    let market = setup.client().get_market(&setup.market_id).unwrap();
    assert_eq!(
        market.claimed.get(setup.winner_1.clone()).unwrap_or(false),
        true
    );
    assert_eq!(
        market.claimed.get(setup.winner_2.clone()).unwrap_or(false),
        true
    );
    assert_eq!(
        market.claimed.get(setup.loser.clone()).unwrap_or(false),
        false
    );
}

#[test]
#[should_panic(expected = "Error(Contract, #207)")]
fn test_claim_blocked_after_claim_period_expired() {
    let setup = TimeoutSweepSetup::new();

    setup.client().set_global_claim_period(&setup.admin, &10u64);
    setup.set_time(setup.end_time + 10);

    setup
        .client()
        .claim_winnings(&setup.winner_1, &setup.market_id);
}

#[test]
fn test_market_specific_claim_period_override_used() {
    let setup = TimeoutSweepSetup::new();

    setup
        .client()
        .set_global_claim_period(&setup.admin, &1_000u64);
    setup
        .client()
        .set_market_claim_period(&setup.admin, &setup.market_id, &5u64);

    setup.set_time(setup.end_time + 5);
    let swept = setup
        .client()
        .sweep_unclaimed_winnings(&setup.admin, &setup.market_id, &true);

    assert!(swept > 0);

    let treasury_balance = setup
        .client()
        .get_balance(&setup.treasury, &ReflectorAsset::Stellar)
        .amount;
    assert_eq!(treasury_balance, 0);
}
