#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _},
    token::StellarAssetClient,
    Address, Env, String as SorobanString, Symbol, Vec as SorobanVec,
};
use predictify_hybrid::{
    PredictifyHybrid, PredictifyHybridClient, OracleConfig, OracleProvider,
    types::{BetStatus, BetLimits},
};

struct TestSetup {
    env: Env,
    admin: Address,
    user: Address,
    contract_id: Address,
    client: PredictifyHybridClient<'static>,
    token_id: Address,
}

impl TestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);

        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &Some(200), &None).unwrap();

        // Setup mock token for staking
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin);
        let token_id = token_contract.address();

        env.as_contract(&contract_id, || {
            env.storage().persistent().set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Fund user and approve contract spending
        let stellar_client = StellarAssetClient::new(&env, &token_id);
        stellar_client.mint(&user, &100_000_000_000);
        let token_client = soroban_sdk::token::Client::new(&env, &token_id);
        token_client.approve(&user, &contract_id, &i128::MAX, &100000);

        Self {
            env,
            admin,
            user,
            contract_id,
            client,
            token_id,
        }
    }

    fn create_market(&self) -> Symbol {
        let outcomes = soroban_sdk::vec![
            &self.env,
            SorobanString::from_str(&self.env, "yes"),
            SorobanString::from_str(&self.env, "no"),
        ];

        self.client.create_market(
            &self.admin,
            &SorobanString::from_str(&self.env, "Will prediction markets be popular?"),
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::reflector(),
                oracle_address: Address::from_str(
                    &self.env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                feed_id: SorobanString::from_str(&self.env, "BTC/USD"),
                threshold: 100_000_00000000,
                comparison: SorobanString::from_str(&self.env, "gt"),
            },
            &None,
            &86400u64,
            &None,
            &None,
            &None,
            &None,
            &None,
        )
    }
}

#[test]
fn test_place_bet_success() {
    let setup = TestSetup::new();
    let market_id = setup.create_market();

    let outcome = SorobanString::from_str(&setup.env, "yes");
    let amount = 10_000_000; // 10 stroops
    
    let bet = setup.client.place_bet(
        &setup.user,
        &market_id,
        &outcome,
        &amount,
        &250, // max fee 2.5%
    );

    assert_eq!(bet.amount, amount);
    assert_eq!(bet.status, BetStatus::Active);
    assert_eq!(bet.user, setup.user);
}

#[test]
fn test_place_bets_batch_success() {
    let setup = TestSetup::new();
    let market_id = setup.create_market();

    let mut bets_vec = SorobanVec::new(&setup.env);
    let outcome1 = SorobanString::from_str(&setup.env, "yes");
    let outcome2 = SorobanString::from_str(&setup.env, "no");
    bets_vec.push_back((market_id.clone(), outcome1, 5_000_000i128));
    bets_vec.push_back((market_id.clone(), outcome2, 8_000_000i128));

    let placed_bets = setup.client.place_bets(
        &setup.user,
        &bets_vec,
        &250,
        &None,
    );

    assert_eq!(placed_bets.len(), 2);
    assert_eq!(placed_bets.get(0).unwrap().amount, 5_000_000);
    assert_eq!(placed_bets.get(1).unwrap().amount, 8_000_000);
}

#[test]
fn test_configure_bet_limits() {
    let setup = TestSetup::new();
    let market_id = setup.create_market();

    // Set global limits
    setup.client.set_global_bet_limits(&setup.admin, &2_000_000, &50_000_000_000).unwrap();
    let effective = setup.client.get_effective_bet_limits(&market_id);
    assert_eq!(effective.min_bet, 2_000_000);
    assert_eq!(effective.max_bet, 50_000_000_000);

    // Set event limits
    setup.client.set_event_bet_limits(&setup.admin, &market_id, &5_000_000, &20_000_000_000).unwrap();
    let effective2 = setup.client.get_effective_bet_limits(&market_id);
    assert_eq!(effective2.min_bet, 5_000_000);
    assert_eq!(effective2.max_bet, 20_000_000_000);
}
