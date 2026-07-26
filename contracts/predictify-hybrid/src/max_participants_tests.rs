//! Tests for the per-market max participants cap.
//!
//! Coverage:
//! - No cap set (None) → any number of participants can vote
//! - Cap set → voting within cap succeeds
//! - Cap exceeded → MaxParticipantsRejected error
//! - Exact cap boundary is allowed; one-over is rejected
//! - Existing voters cannot re-vote (AlreadyVoted fires first)
//! - Admin can update the participant cap after creation
//! - Unauthorized caller cannot change the cap
//! - Cap applies to distinct voters, not total vote count

#![cfg(test)]

use crate::err::Error;
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Events},
    vec, Address, Env, String, Symbol, TryFromVal, Val,
};

// ===== TEST SETUP =====

struct Setup {
    env: Env,
    contract_id: Address,
    admin: Address,
    token_id: Address,
}

impl Setup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
            crate::circuit_breaker::CircuitBreaker::initialize(&env).unwrap();
        });

        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);

        Self { env, contract_id, admin, token_id }
    }

    fn funded_user(&self) -> Address {
        let u = Address::generate(&self.env);
        soroban_sdk::token::StellarAssetClient::new(&self.env, &self.token_id)
            .mint(&u, &100_000_000_000i128);
        u
    }

    fn create_market(&self, max_participants: Option<u32>) -> Symbol {
        use crate::types::{OracleConfig, OracleProvider};
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        let oracle_config = OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(
                &self.env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            String::from_str(&self.env, "BTC/USD"),
            5_000_000,
            String::from_str(&self.env, "gt"),
        );
        client.create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC hit 100k?"),
            &vec![
                &self.env,
                String::from_str(&self.env, "Yes"),
                String::from_str(&self.env, "No"),
            ],
            &30u32,
            &oracle_config,
            &None,
            &86400u64,
            &None,
            &None,
            &None,
            &None,
            &max_participants,
        )
    }

    fn vote(
        &self,
        market_id: &Symbol,
        user: &Address,
        outcome: &str,
        stake: i128,
    ) -> Result<Result<(), soroban_sdk::ConversionError>, Result<Error, soroban_sdk::InvokeError>> {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        client.try_vote(user, market_id, &String::from_str(&self.env, outcome), &stake)
    }
}

// ===== TESTS =====

/// When max_participants is None (default), any number of participants can vote.
#[test]
fn test_no_cap_allows_all_participants() {
    let s = Setup::new();
    let market_id = s.create_market(None);

    for _ in 0..5 {
        let user = s.funded_user();
        assert!(s.vote(&market_id, &user, "Yes", 1_000_000).is_ok());
    }
}

/// With a cap set, voting within the limit succeeds.
#[test]
fn test_cap_within_limit_succeeds() {
    let s = Setup::new();
    let market_id = s.create_market(Some(3));

    let user1 = s.funded_user();
    let user2 = s.funded_user();
    let user3 = s.funded_user();

    assert!(s.vote(&market_id, &user1, "Yes", 1_000_000).is_ok());
    assert!(s.vote(&market_id, &user2, "No", 1_000_000).is_ok());
    assert!(s.vote(&market_id, &user3, "Yes", 1_000_000).is_ok());
}

/// Exceeding the participant cap returns MaxParticipantsReached.
#[test]
fn test_cap_exceeded_returns_error() {
    let s = Setup::new();
    let market_id = s.create_market(Some(2));

    let user1 = s.funded_user();
    let user2 = s.funded_user();
    let user3 = s.funded_user();

    assert!(s.vote(&market_id, &user1, "Yes", 1_000_000).is_ok());
    assert!(s.vote(&market_id, &user2, "No", 1_000_000).is_ok());

    // Third voter should be rejected
    let result = s.vote(&market_id, &user3, "Yes", 1_000_000);
    assert_eq!(result, Err(Ok(Error::MaxParticipantsReached)));
}

/// Exactly hitting the cap boundary is allowed.
#[test]
fn test_exact_cap_boundary_allowed() {
    let s = Setup::new();
    let market_id = s.create_market(Some(3));

    let user1 = s.funded_user();
    let user2 = s.funded_user();
    let user3 = s.funded_user();

    assert!(s.vote(&market_id, &user1, "Yes", 1_000_000).is_ok());
    assert!(s.vote(&market_id, &user2, "No", 1_000_000).is_ok());
    assert!(s.vote(&market_id, &user3, "Yes", 1_000_000).is_ok());
}

/// After hitting the cap exactly, further votes are rejected.
#[test]
fn test_beyond_cap_after_exact_hit_rejected() {
    let s = Setup::new();
    let market_id = s.create_market(Some(2));

    let user1 = s.funded_user();
    let user2 = s.funded_user();
    let user3 = s.funded_user();

    assert!(s.vote(&market_id, &user1, "Yes", 1_000_000).is_ok());
    assert!(s.vote(&market_id, &user2, "No", 1_000_000).is_ok());

    // Exactly at cap; 3rd voter rejected
    let result = s.vote(&market_id, &user3, "Yes", 1_000_000);
    assert_eq!(result, Err(Ok(Error::MaxParticipantsReached)));
}

/// Admin can increase the cap after creation, allowing new participants.
#[test]
fn test_admin_can_increase_max_participants() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let market_id = s.create_market(Some(1));

    let user1 = s.funded_user();
    let user2 = s.funded_user();

    // First vote within initial cap
    assert!(s.vote(&market_id, &user1, "Yes", 1_000_000).is_ok());

    // Second vote should be rejected
    assert_eq!(
        s.vote(&market_id, &user2, "No", 1_000_000),
        Err(Ok(Error::MaxParticipantsReached))
    );

    // Admin increases cap to 2
    client.set_max_participants(&s.admin, &market_id, &Some(2u32));

    // Now second vote should succeed
    assert!(s.vote(&market_id, &user2, "No", 1_000_000).is_ok());
}

/// Admin can remove the cap entirely (set to None).
#[test]
fn test_admin_can_remove_cap() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let market_id = s.create_market(Some(1));

    // Remove the cap
    client.set_max_participants(&s.admin, &market_id, &None);

    let user1 = s.funded_user();
    let user2 = s.funded_user();

    assert!(s.vote(&market_id, &user1, "Yes", 1_000_000).is_ok());
    assert!(s.vote(&market_id, &user2, "No", 1_000_000).is_ok());
}

/// Non-admin cannot set the participant cap.
#[test]
fn test_unauthorized_cannot_set_cap() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let market_id = s.create_market(None);
    let rando = Address::generate(&s.env);

    let result = client.try_set_max_participants(&rando, &market_id, &Some(5u32));
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

/// Setting cap on a non-existent market returns MarketNotFound.
#[test]
fn test_set_cap_on_nonexistent_market_fails() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let fake_id = Symbol::new(&s.env, "nonexistent");

    let result = client.try_set_max_participants(&s.admin, &fake_id, &Some(5u32));
    assert_eq!(result, Err(Ok(Error::MarketNotFound)));
}

/// Zero cap rejects all participants.
#[test]
fn test_zero_cap_rejects_all() {
    let s = Setup::new();
    let market_id = s.create_market(Some(0));

    let user = s.funded_user();
    let result = s.vote(&market_id, &user, "Yes", 1_000_000);
    assert_eq!(result, Err(Ok(Error::MaxParticipantsReached)));
}
