#![cfg(test)]

use crate::errors::Error;
use crate::types::{OracleConfig, OracleProvider};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{vec, Address, Env, String, Symbol, Vec};

// Test helper structure
struct TestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl TestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());
        
        // Initialize the contract
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);
        
        Self {
            env,
            contract_id,
            admin,
        }
    }
    
    fn create_user(&self) -> Address {
        Address::generate(&self.env)
    }
    
    fn create_market(&self, question: &str, outcomes: Vec<String>, duration_days: u32) -> Symbol {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        let oracle_config = OracleConfig::new(
            OracleProvider::Reflector,
            String::from_str(&self.env, "BTC/USD"),
            5000000,
            String::from_str(&self.env, "gt"),
        );

        client.create_market(
            &self.admin,
            &String::from_str(&self.env, question),
            &outcomes,
            &duration_days,
            &oracle_config,
        )
    }
}

// ===== EXTEND DEADLINE TESTS =====

#[test]
fn test_extend_deadline_success() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Test question?", outcomes, 30);
    
    // Get initial market state
    let market_before = client.get_market(&market_id).unwrap();
    let initial_end_time = market_before.end_time;
    
    // Extend deadline by 7 days
    let result = client.try_extend_deadline(
        &setup.admin,
        &market_id,
        &7u32,
        &String::from_str(&setup.env, "Low participation"),
    );
    
    assert!(result.is_ok());
    
    // Verify market was updated
    let market_after = client.get_market(&market_id).unwrap();
    assert_eq!(market_after.end_time, initial_end_time + (7 * 24 * 60 * 60));
    assert_eq!(market_after.total_extension_days, 7);
    assert_eq!(market_after.extension_history.len(), 1);
}

#[test]
fn test_extend_deadline_exceeds_maximum() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Test question?", outcomes, 30);
    
    // Try to extend by more than max_extension_days (default 30)
    let result = client.try_extend_deadline(
        &setup.admin,
        &market_id,
        &31u32,
        &String::from_str(&setup.env, "Too long"),
    );
    
    assert_eq!(result, Err(Ok(Error::InvalidDuration)));
}

#[test]
fn test_extend_deadline_resolved_market() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Test question?", outcomes, 30);
    
    // Move time forward past end time
    setup.env.ledger().with_mut(|li| {
        li.timestamp = li.timestamp + (31 * 24 * 60 * 60);
    });
    
    // Resolve the market
    let _ = client.try_resolve_market_manual(
        &setup.admin,
        &market_id,
        &String::from_str(&setup.env, "Yes"),
    );
    
    // Try to extend resolved market
    let result = client.try_extend_deadline(
        &setup.admin,
        &market_id,
        &7u32,
        &String::from_str(&setup.env, "Extension after resolution"),
    );
    
    assert_eq!(result, Err(Ok(Error::MarketAlreadyResolved)));
}

#[test]
fn test_extend_deadline_unauthorized() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    let unauthorized_user = setup.create_user();
    
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Test question?", outcomes, 30);
    
    // Try to extend as unauthorized user
    let result = client.try_extend_deadline(
        &unauthorized_user,
        &market_id,
        &7u32,
        &String::from_str(&setup.env, "Unauthorized extension"),
    );
    
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

// ===== UPDATE EVENT DESCRIPTION TESTS =====

#[test]
fn test_update_event_description_success() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Original question?", outcomes, 30);
    
    // Update description
    let new_description = String::from_str(&setup.env, "Updated question with more details?");
    let result = client.try_update_event_description(
        &setup.admin,
        &market_id,
        &new_description,
    );
    
    assert!(result.is_ok());
    
    // Verify market was updated
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.question, new_description);
}

#[test]
fn test_update_event_description_empty() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Original question?", outcomes, 30);
    
    // Try to update with empty description
    let result = client.try_update_event_description(
        &setup.admin,
        &market_id,
        &String::from_str(&setup.env, ""),
    );
    
    assert_eq!(result, Err(Ok(Error::InvalidQuestion)));
}

#[test]
fn test_update_event_description_after_votes() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    let user = setup.create_user();
    
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Original question?", outcomes, 30);
    
    // Place a vote
    client.vote(
        &user,
        &market_id,
        &String::from_str(&setup.env, "Yes"),
        &1000000i128,
    );
    
    // Try to update description after vote
    let result = client.try_update_event_description(
        &setup.admin,
        &market_id,
        &String::from_str(&setup.env, "Updated question?"),
    );
    
    assert_eq!(result, Err(Ok(Error::AlreadyVoted)));
}

// Note: This test validates that votes prevent description updates
// The BetsAlreadyPlaced error would also prevent updates, but requires token setup
#[test]
fn test_update_event_description_after_activity() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    let user = setup.create_user();
    
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Original question?", outcomes, 30);
    
    // Place a vote (testing that any activity prevents updates)
    client.vote(
        &user,
        &market_id,
        &String::from_str(&setup.env, "Yes"),
        &1000000i128,
    );
    
    // Try to update description after activity
    let result = client.try_update_event_description(
        &setup.admin,
        &market_id,
        &String::from_str(&setup.env, "Updated question?"),
    );
    
    // Should fail because votes have been placed
    assert_eq!(result, Err(Ok(Error::AlreadyVoted)));
}

#[test]
fn test_update_event_description_unauthorized() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    let unauthorized_user = setup.create_user();
    
    let outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Original question?", outcomes, 30);
    
    // Try to update as unauthorized user
    let result = client.try_update_event_description(
        &unauthorized_user,
        &market_id,
        &String::from_str(&setup.env, "Unauthorized update?"),
    );
    
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

// ===== UPDATE EVENT OUTCOMES TESTS =====

#[test]
fn test_update_event_outcomes_success() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let initial_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Test question?", initial_outcomes, 30);
    
    // Update outcomes
    let new_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
        String::from_str(&setup.env, "Maybe"),
    ];
    
    let result = client.try_update_event_outcomes(
        &setup.admin,
        &market_id,
        &new_outcomes,
    );
    
    assert!(result.is_ok());
    
    // Verify market was updated
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.outcomes.len(), 3);
    assert_eq!(market.outcomes.get(0).unwrap(), String::from_str(&setup.env, "Yes"));
    assert_eq!(market.outcomes.get(1).unwrap(), String::from_str(&setup.env, "No"));
    assert_eq!(market.outcomes.get(2).unwrap(), String::from_str(&setup.env, "Maybe"));
}

#[test]
fn test_update_event_outcomes_too_few() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let initial_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Test question?", initial_outcomes, 30);
    
    // Try to update with only one outcome
    let new_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
    ];
    
    let result = client.try_update_event_outcomes(
        &setup.admin,
        &market_id,
        &new_outcomes,
    );
    
    assert_eq!(result, Err(Ok(Error::InvalidOutcomes)));
}

#[test]
fn test_update_event_outcomes_empty_string() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let initial_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Test question?", initial_outcomes, 30);
    
    // Try to update with empty outcome string
    let new_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, ""),
    ];
    
    let result = client.try_update_event_outcomes(
        &setup.admin,
        &market_id,
        &new_outcomes,
    );
    
    assert_eq!(result, Err(Ok(Error::InvalidOutcome)));
}

#[test]
fn test_update_event_outcomes_after_votes() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    let user = setup.create_user();
    
    let initial_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Test question?", initial_outcomes, 30);
    
    // Place a vote
    client.vote(
        &user,
        &market_id,
        &String::from_str(&setup.env, "Yes"),
        &1000000i128,
    );
    
    // Try to update outcomes after vote
    let new_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
        String::from_str(&setup.env, "Maybe"),
    ];
    
    let result = client.try_update_event_outcomes(
        &setup.admin,
        &market_id,
        &new_outcomes,
    );
    
    assert_eq!(result, Err(Ok(Error::AlreadyVoted)));
}

// Note: This test validates that votes prevent outcome updates
// The BetsAlreadyPlaced error would also prevent updates, but requires token setup
#[test]
fn test_update_event_outcomes_after_activity() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    let user = setup.create_user();
    
    let initial_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Test question?", initial_outcomes, 30);
    
    // Place a vote (testing that any activity prevents updates)
    client.vote(
        &user,
        &market_id,
        &String::from_str(&setup.env, "Yes"),
        &1000000i128,
    );
    
    // Try to update outcomes after activity
    let new_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
        String::from_str(&setup.env, "Maybe"),
    ];
    
    let result = client.try_update_event_outcomes(
        &setup.admin,
        &market_id,
        &new_outcomes,
    );
    
    // Should fail because votes have been placed
    assert_eq!(result, Err(Ok(Error::AlreadyVoted)));
}

#[test]
fn test_update_event_outcomes_unauthorized() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    let unauthorized_user = setup.create_user();
    
    let initial_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Test question?", initial_outcomes, 30);
    
    // Try to update as unauthorized user
    let new_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
        String::from_str(&setup.env, "Maybe"),
    ];
    
    let result = client.try_update_event_outcomes(
        &unauthorized_user,
        &market_id,
        &new_outcomes,
    );
    
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn test_update_event_outcomes_resolved_market() {
    let setup = TestSetup::new();
    let client = PredictifyHybridClient::new(&setup.env, &setup.contract_id);
    
    let initial_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
    ];
    
    let market_id = setup.create_market("Test question?", initial_outcomes, 30);
    
    // Move time forward past end time
    setup.env.ledger().with_mut(|li| {
        li.timestamp = li.timestamp + (31 * 24 * 60 * 60);
    });
    
    // Resolve the market
    let _ = client.try_resolve_market_manual(
        &setup.admin,
        &market_id,
        &String::from_str(&setup.env, "Yes"),
    );
    
    // Try to update outcomes on resolved market
    let new_outcomes = vec![
        &setup.env,
        String::from_str(&setup.env, "Yes"),
        String::from_str(&setup.env, "No"),
        String::from_str(&setup.env, "Maybe"),
    ];
    
    let result = client.try_update_event_outcomes(
        &setup.admin,
        &market_id,
        &new_outcomes,
    );
    
    assert_eq!(result, Err(Ok(Error::MarketAlreadyResolved)));
}
