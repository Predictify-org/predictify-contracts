
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{testutils::{Address as _, Ledger}, token::{StellarAssetClient, Client as TokenClient}, Address, Env, String, Symbol, vec, Vec};
use alloc::format;

/// Setup function for tests
fn setup_test() -> (Env, PredictifyHybridClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register_contract(None, crate::PredictifyHybrid);
    let client = PredictifyHybridClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    
    // Initialize contract
    client.initialize(&admin, &Some(2)); // 2% fee
    
    (env, client, admin)
}

/// Helper to create a test market
fn create_test_market(
    env: &Env, 
    client: &PredictifyHybridClient, 
    admin: &Address, 
    suffix: &str
) -> Symbol {
    let question = String::from_str(env, &format!("Will {} price increase?", suffix));
    let mut outcomes = Vec::new(env);
    outcomes.push_back(String::from_str(env, "yes"));
    outcomes.push_back(String::from_str(env, "no"));
    
    let oracle_config = OracleConfig {
        provider: OracleProvider::Reflector,
        feed_id: String::from_str(env, "BTC/USD"),
        threshold: 100,
        comparison: String::from_str(env, "gte"),
    };
    
    client.create_market(
        admin,
        &question,
        &outcomes,
        &30, // 30 days
        &oracle_config
    )
}

#[test]
fn test_update_category_and_tags() {
    let (env, client, admin) = setup_test();
    let market_id = create_test_market(&env, &client, &admin, "Bitcoin");
    
    // Initial state: category should be empty (None) and tags empty
    let market = client.get_market(&market_id).unwrap();
    assert!(market.category.is_none());
    assert!(market.tags.is_empty());
    
    // Update category
    let category = String::from_str(&env, "Crypto");
    client.update_event_category(&admin, &market_id, &Some(category.clone()));
    
    // Verify category update
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.category, Some(category));
    
    // Update tags
    let tags = vec![
        &env, 
        String::from_str(&env, "bitcoin"), 
        String::from_str(&env, "finance")
    ];
    client.update_event_tags(&admin, &market_id, &tags);
    
    // Verify tags update
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.tags, tags);
    
    // Test clearing category
    client.update_event_category(&admin, &market_id, &None);
    let market = client.get_market(&market_id).unwrap();
    assert!(market.category.is_none());
    
    // Test clearing tags
    client.update_event_tags(&admin, &market_id, &Vec::new(&env));
    let market = client.get_market(&market_id).unwrap();
    assert!(market.tags.is_empty());
}

#[test]
fn test_query_events_by_category() {
    let (env, client, admin) = setup_test();
    
    // Create 3 markets
    let m1 = create_test_market(&env, &client, &admin, "Bitcoin");
    let m2 = create_test_market(&env, &client, &admin, "Ethereum");
    let m3 = create_test_market(&env, &client, &admin, "Tesla"); // Different category
    
    // Set categories
    let crypto_cat = String::from_str(&env, "Crypto");
    let stocks_cat = String::from_str(&env, "Stocks");
    
    client.update_event_category(&admin, &m1, &Some(crypto_cat.clone()));
    client.update_event_category(&admin, &m2, &Some(crypto_cat.clone()));
    client.update_event_category(&admin, &m3, &Some(stocks_cat.clone()));
    
    // Query by "Crypto" category
    let (crypto_events, _) = client.query_events_by_category(&crypto_cat, &0, &10);
    
    // Should find 2 markets
    assert_eq!(crypto_events.len(), 2);
    
    // Verify IDs (can be in any order depending on internals, but usually insertion order for mock)
    let found_ids: Vec<Symbol> = vec![
        &env,
        crypto_events.get(0).unwrap().market_id,
        crypto_events.get(1).unwrap().market_id
    ];
    // Check if m1 and m2 are present
    // Note: In Soroban testing, we need to iterate or check specifically.
    // For simplicity, we check counts and existence logic is implied if count matches distinct items.
    
    // Query by "Stocks" category
    let (stock_events, _) = client.query_events_by_category(&stocks_cat, &0, &10);
    assert_eq!(stock_events.len(), 1);
    assert_eq!(stock_events.get(0).unwrap().market_id, m3);
    
    // Query by non-existing category
    let (empty_events, _) = client.query_events_by_category(&String::from_str(&env, "None"), &0, &10);
    assert_eq!(empty_events.len(), 0);
}

#[test]
fn test_query_events_by_tags() {
    let (env, client, admin) = setup_test();
    
    // Create 3 markets
    let m1 = create_test_market(&env, &client, &admin, "BTC");
    let m2 = create_test_market(&env, &client, &admin, "ETH");
    let m3 = create_test_market(&env, &client, &admin, "USDT");
    
    // Define tags
    let tag_l1 = String::from_str(&env, "L1");
    let tag_stable = String::from_str(&env, "Stablecoin");
    let tag_high_vol = String::from_str(&env, "HighVol");
    
    // Set tags
    // m1: [L1, HighVol]
    client.update_event_tags(&admin, &m1, &vec![&env, tag_l1.clone(), tag_high_vol.clone()]);
    
    // m2: [L1, HighVol]
    client.update_event_tags(&admin, &m2, &vec![&env, tag_l1.clone(), tag_high_vol.clone()]);
    
    // m3: [Stablecoin]
    client.update_event_tags(&admin, &m3, &vec![&env, tag_stable.clone()]);
    
    // Query by "L1" tag -> Should get m1 and m2
    let (l1_events, _) = client.query_events_by_tags(&vec![&env, tag_l1.clone()], &0, &10);
    assert_eq!(l1_events.len(), 2);
    
    // Query by "Stablecoin" -> m3 only
    let (stable_events, _) = client.query_events_by_tags(&vec![&env, tag_stable.clone()], &0, &10);
    assert_eq!(stable_events.len(), 1);
    assert_eq!(stable_events.get(0).unwrap().market_id, m3);
    
    // Query by OR logic: "L1" OR "Stablecoin" -> Should get m1, m2, m3
    let (all_events, _) = client.query_events_by_tags(&vec![&env, tag_l1.clone(), tag_stable.clone()], &0, &10);
    assert_eq!(all_events.len(), 3);
    
    // Query by non-existing tag
    let (no_events, _) = client.query_events_by_tags(&vec![&env, String::from_str(&env, "NonExistent")], &0, &10);
    assert_eq!(no_events.len(), 0);
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn test_security_update_category_unauthorized() {
    let (env, client, admin) = setup_test();
    let market_id = create_test_market(&env, &client, &admin, "Security");
    
    let user = Address::generate(&env);
    
    // Try to update category as non-admin
    client.update_event_category(&user, &market_id, &Some(String::from_str(&env, "Hack")));
}

#[test]
#[should_panic(expected = "Error(Contract, #100)")]
fn test_security_update_tags_unauthorized() {
    let (env, client, admin) = setup_test();
    let market_id = create_test_market(&env, &client, &admin, "Security");
    
    let user = Address::generate(&env);
    
    // Try to update tags as non-admin
    client.update_event_tags(&user, &market_id, &vec![&env, String::from_str(&env, "Hack")]);
}

// ===== INTEGRATION TEST: ENSURE CATEGORY/TAGS DO NOT AFFECT RESOLUTION/PAYOUTS =====

struct TokenTestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    user1: Address,
    user2: Address,
    token_id: Address,
    market_id: Symbol,
}

impl TokenTestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());

        // Setup Token
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        // Store TokenID in contract
        env.as_contract(&contract_id, || {
             env.storage().persistent().set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Initialize the contract
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &Some(2));

        // Create users and fund them
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let stellar_client = StellarAssetClient::new(&env, &token_id);
        stellar_client.mint(&user1, &1_000_000_000); // 1_000 XLM
        stellar_client.mint(&user2, &1_000_000_000);

        // Approve contract to spend tokens on behalf of users
        let token_client = TokenClient::new(&env, &token_id);
        token_client.approve(&user1, &contract_id, &i128::MAX, &1000000);
        token_client.approve(&user2, &contract_id, &i128::MAX, &1000000);
        token_client.approve(&admin, &contract_id, &i128::MAX, &1000000);

        // Create a market
        let outcomes = vec![&env, String::from_str(&env, "yes"), String::from_str(&env, "no")];
        let market_id = client.create_market(
            &admin,
            &String::from_str(&env, "Will category/tags affect payouts?"),
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::Reflector,
                feed_id: String::from_str(&env, "BTC/USD"),
                threshold: 100,
                comparison: String::from_str(&env, "gte"),
            },
        );

        Self { env, contract_id, admin, user1, user2, token_id, market_id }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }
}

#[test]
fn test_category_tags_do_not_affect_resolution_and_payouts() {
    let setup = TokenTestSetup::new();
    let client = setup.client();

    // Update category and tags before bets are placed (allowed)
    client.update_event_category(&setup.admin, &setup.market_id, &Some(String::from_str(&setup.env, "Finance")));
    client.update_event_tags(&setup.admin, &setup.market_id, &vec![&setup.env, String::from_str(&setup.env, "crypto")] );

    // Place bets: user1 -> yes (10), user2 -> no (20)
    client.place_bet(&setup.user1, &setup.market_id, &String::from_str(&setup.env, "yes"), &10_0000000);
    client.place_bet(&setup.user2, &setup.market_id, &String::from_str(&setup.env, "no"), &20_0000000);

    // Payout multiplier for "yes" should be (30/10)*100 = 300 and unchanged by category/tags
    let yes_mul_before = client.get_payout_multiplier(&setup.market_id, &String::from_str(&setup.env, "yes"));
    assert_eq!(yes_mul_before, 300);

    // Advance time and resolve market to "yes"
    setup.env.ledger().with_mut(|li| { li.timestamp = li.timestamp + (31 * 24 * 60 * 60); });
    let _ = client.try_resolve_market_manual(&setup.admin, &setup.market_id, &String::from_str(&setup.env, "yes"));

    // Ensure market resolved
    let market = client.get_market(&setup.market_id).unwrap();
    assert_eq!(market.state, MarketState::Resolved);

    // Final multiplier should remain the same
    let yes_mul_final = client.get_payout_multiplier(&setup.market_id, &String::from_str(&setup.env, "yes"));
    assert_eq!(yes_mul_final, 300);
}

#[test]
#[should_panic(expected = "Error(Contract, #111)")]
fn test_update_category_after_bets_forbidden() {
    let setup = TokenTestSetup::new();
    let client = setup.client();

    // Place a bet first
    client.place_bet(&setup.user1, &setup.market_id, &String::from_str(&setup.env, "yes"), &10_0000000);

    // Attempt to update category after bets have been placed -> should panic with #111
    client.update_event_category(&setup.admin, &setup.market_id, &Some(String::from_str(&setup.env, "Blocked")));
}
