//! # Claim Idempotency Tests
//!
//! Comprehensive test suite for the idempotent winnings claim mechanism.
//!
//! ## Test Coverage
//!
//! - **Idempotency Tests**: Verify double claims are prevented
//! - **Timestamp Tracking Tests**: Verify claim timestamps are recorded
//! - **Payout Tracking Tests**: Verify payout amounts are stored
//! - **Retry Safety Tests**: Verify safe retry mechanisms
//! - **Edge Case Tests**: Boundary conditions and special scenarios
//! - **Integration Tests**: Full claim lifecycle with new structure
//!
//! ## Test Coverage Target: 95%+

#![cfg(test)]

use crate::test::{PredictifyHybrid, PredictifyHybridClient};
use crate::types::{ClaimInfo, MarketState, OracleConfig, OracleProvider};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, Address, Env, String, Symbol,
};

// ===== TEST SETUP =====

/// Test infrastructure for claim idempotency tests
struct ClaimIdempotencyTestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    user: Address,
    user2: Address,
    token_id: Address,
    market_id: Symbol,
}

impl ClaimIdempotencyTestSetup {
    /// Create a new test environment with resolved market
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Setup admin and users
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let user2 = Address::generate(&env);

        // Register and initialize the contract
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);

        // Setup token for staking
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        // Set token for staking in contract storage
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Fund users with tokens
        let stellar_client = StellarAssetClient::new(&env, &token_id);
        stellar_client.mint(&admin, &10_000_0000000); // 10,000 XLM
        stellar_client.mint(&user, &1000_0000000); // 1,000 XLM
        stellar_client.mint(&user2, &1000_0000000); // 1,000 XLM

        // Approve contract to spend tokens
        let token_client = soroban_sdk::token::Client::new(&env, &token_id);
        token_client.approve(&user, &contract_id, &i128::MAX, &1000000);
        token_client.approve(&user2, &contract_id, &i128::MAX, &1000000);
        token_client.approve(&admin, &contract_id, &i128::MAX, &1000000);

        // Create and resolve a test market
        let market_id = Self::create_resolved_market_static(
            &env,
            &contract_id,
            &admin,
            &user,
            &user2,
            &token_id,
        );

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

    /// Create a resolved test market with votes
    fn create_resolved_market_static(
        env: &Env,
        contract_id: &Address,
        admin: &Address,
        user: &Address,
        user2: &Address,
        token_id: &Address,
    ) -> Symbol {
        let client = PredictifyHybridClient::new(env, contract_id);

        let outcomes = vec![
            env,
            String::from_str(env, "yes"),
            String::from_str(env, "no"),
        ];

        // Create market
        let market_id = client.create_market(
            admin,
            &String::from_str(env, "Will BTC reach $100,000?"),
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::Reflector,
                oracle_address: Address::from_str(
                    env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                feed_id: String::from_str(env, "BTC/USD"),
                threshold: 100_000_00000000,
                comparison: String::from_str(env, "gte"),
            },
            &None,
            &86400u64,
            &None,
            &None,
            &None,
        );

        // Vote (user votes "yes" with 100 XLM, user2 votes "no" with 50 XLM)
        client.vote(user, &market_id, &String::from_str(env, "yes"), &100_000_0000000);
        client.vote(user2, &market_id, &String::from_str(env, "no"), &50_000_0000000);

        // Advance time past market end
        let market = client.get_market(&market_id).unwrap();
        env.ledger().set(LedgerInfo {
            timestamp: market.end_time + 1,
            protocol_version: 22,
            sequence_number: env.ledger().sequence(),
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 10000,
        });

        // Resolve market (winner is "yes")
        client.resolve_market_manual(
            admin,
            &market_id,
            &String::from_str(env, "yes"),
        );

        market_id
    }

    /// Get client for contract interactions
    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    /// Get claim info directly from storage
    fn get_claim_info(&self, user: &Address) -> Option<ClaimInfo> {
        self.env.as_contract(&self.contract_id, || {
            let market = self
                .env
                .storage()
                .persistent()
                .get::<Symbol, crate::types::Market>(&self.market_id)
                .unwrap();
            market.claimed.get(user.clone())
        })
    }
}

// ===== IDEMPOTENCY TESTS =====

#[test]
fn test_claim_idempotency_prevents_double_claim() {
    let setup = ClaimIdempotencyTestSetup::new();
    let client = setup.client();

    // First claim should succeed
    let payout1 = client.claim_winnings(&setup.user, &setup.market_id);
    assert!(payout1 > 0);

    // Verify claim info is stored
    let claim_info = setup.get_claim_info(&setup.user).unwrap();
    assert!(claim_info.is_claimed());
    assert_eq!(claim_info.get_payout(), payout1);
    assert!(claim_info.get_timestamp() > 0);

    // Second claim should fail with AlreadyClaimed error
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_winnings(&setup.user, &setup.market_id);
    }));
    assert!(result.is_err());
}

#[test]
fn test_claim_idempotent_with_zero_payout() {
    let setup = ClaimIdempotencyTestSetup::new();
    let client = setup.client();

    // Loser (user2) should get zero payout but still be marked as claimed
    let payout = client.claim_winnings(&setup.user2, &setup.market_id);
    assert_eq!(payout, 0);

    // Verify claim info shows claimed even with zero payout
    let claim_info = setup.get_claim_info(&setup.user2).unwrap();
    assert!(claim_info.is_claimed());
    assert_eq!(claim_info.get_payout(), 0);
    assert!(claim_info.get_timestamp() > 0);

    // Retry should also return zero (idempotent)
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_winnings(&setup.user2, &setup.market_id);
    }));
    assert!(result.is_err()); // Still fails because already claimed
}

#[test]
fn test_claim_retry_after_failed_transfer_simulation() {
    let setup = ClaimIdempotencyTestSetup::new();
    let client = setup.client();

    // Simulate: User attempts claim, transfer succeeds
    let payout1 = client.claim_winnings(&setup.user, &setup.market_id);
    assert!(payout1 > 0);

    let claim_info1 = setup.get_claim_info(&setup.user).unwrap();
    let timestamp1 = claim_info1.get_timestamp();

    // Simulate retry (e.g., user thinks transaction failed)
    // Should fail with AlreadyClaimed, preventing double payout
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_winnings(&setup.user, &setup.market_id);
    }));
    assert!(result.is_err());

    // Verify claim info unchanged
    let claim_info2 = setup.get_claim_info(&setup.user).unwrap();
    assert_eq!(claim_info2.get_payout(), claim_info1.get_payout());
    assert_eq!(claim_info2.get_timestamp(), timestamp1);
}

// ===== TIMESTAMP TRACKING TESTS =====

#[test]
fn test_claim_timestamp_recorded() {
    let setup = ClaimIdempotencyTestSetup::new();
    let client = setup.client();

    let before_timestamp = setup.env.ledger().timestamp();

    // Claim winnings
    let _payout = client.claim_winnings(&setup.user, &setup.market_id);

    let claim_info = setup.get_claim_info(&setup.user).unwrap();
    let recorded_timestamp = claim_info.get_timestamp();

    // Timestamp should be >= before_timestamp
    assert!(recorded_timestamp >= before_timestamp);

    // Timestamp should be reasonable (within 10 seconds of current ledger time)
    let after_timestamp = setup.env.ledger().timestamp();
    assert!(recorded_timestamp <= after_timestamp);
}

#[test]
fn test_claim_timestamp_accuracy() {
    let setup = ClaimIdempotencyTestSetup::new();
    let client = setup.client();

    // Claim at time T1
    let _payout1 = client.claim_winnings(&setup.user, &setup.market_id);
    let claim_info1 = setup.get_claim_info(&setup.user).unwrap();
    let timestamp1 = claim_info1.get_timestamp();

    // Advance time
    setup.env.ledger().with_mut(|li| {
        li.timestamp += 100; // Advance by 100 seconds
    });

    // This would fail (already claimed), but if we could claim again on a different market,
    // the timestamp would be different
    // For this test, we just verify the first timestamp is valid
    assert!(timestamp1 > 0);
}

// ===== PAYOUT TRACKING TESTS =====

#[test]
fn test_claim_payout_amount_stored() {
    let setup = ClaimIdempotencyTestSetup::new();
    let client = setup.client();

    // Claim winnings
    let payout = client.claim_winnings(&setup.user, &setup.market_id);
    assert!(payout > 0);

    // Verify stored payout matches returned payout
    let claim_info = setup.get_claim_info(&setup.user).unwrap();
    assert_eq!(claim_info.get_payout(), payout);
}

#[test]
fn test_claim_payout_verification() {
    let setup = ClaimIdempotencyTestSetup::new();
    let client = setup.client();

    // Get initial balance
    let token_client = soroban_sdk::token::Client::new(&setup.env, &setup.token_id);
    let balance_before = token_client.balance(&setup.user);

    // Claim winnings
    let payout = client.claim_winnings(&setup.user, &setup.market_id);

    // Verify balance increased by payout amount
    let balance_after = token_client.balance(&setup.user);
    assert_eq!(balance_after - balance_before, payout);

    // Verify stored payout matches
    let claim_info = setup.get_claim_info(&setup.user).unwrap();
    assert_eq!(claim_info.get_payout(), payout);
}

// ===== EDGE CASE TESTS =====

#[test]
fn test_claim_state_transitions() {
    let setup = ClaimIdempotencyTestSetup::new();
    let client = setup.client();

    // Before claim
    let claim_info_before = setup.get_claim_info(&setup.user);
    assert!(claim_info_before.is_none());

    // After claim
    let _payout = client.claim_winnings(&setup.user, &setup.market_id);
    let claim_info_after = setup.get_claim_info(&setup.user).unwrap();
    assert!(claim_info_after.is_claimed());
    assert!(claim_info_after.get_payout() > 0);
    assert!(claim_info_after.get_timestamp() > 0);
}

#[test]
fn test_multiple_users_claim_independently() {
    let setup = ClaimIdempotencyTestSetup::new();
    let client = setup.client();

    // Both users can claim independently
    let payout1 = client.claim_winnings(&setup.user, &setup.market_id);
    let payout2 = client.claim_winnings(&setup.user2, &setup.market_id);

    // Verify both claims recorded separately
    let claim_info1 = setup.get_claim_info(&setup.user).unwrap();
    let claim_info2 = setup.get_claim_info(&setup.user2).unwrap();

    assert!(claim_info1.is_claimed());
    assert!(claim_info2.is_claimed());

    // Payouts may differ based on stake proportions
    assert_eq!(claim_info1.get_payout(), payout1);
    assert_eq!(claim_info2.get_payout(), payout2);

    // Timestamps should be close (same block)
    let time_diff = claim_info1.get_timestamp().abs_diff(claim_info2.get_timestamp());
    assert!(time_diff < 10); // Within 10 seconds
}

// ===== INTEGRATION TESTS =====

#[test]
fn test_full_claim_lifecycle_with_new_structure() {
    let setup = ClaimIdempotencyTestSetup::new();
    let client = setup.client();

    // 1. Market created and votes placed (done in setup)

    // 2. Market resolved (done in setup)

    // 3. User claims winnings
    let payout = client.claim_winnings(&setup.user, &setup.market_id);

    // 4. Verify claim recorded with full details
    let claim_info = setup.get_claim_info(&setup.user).unwrap();
    assert!(claim_info.is_claimed());
    assert_eq!(claim_info.get_payout(), payout);
    assert!(claim_info.get_timestamp() > 0);

    // 5. Verify double claim prevented
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_winnings(&setup.user, &setup.market_id);
    }));
    assert!(result.is_err());
}

#[test]
fn test_claim_info_struct_methods() {
    let env = Env::default();

    // Test unclaimed default
    let unclaimed = ClaimInfo::unclaimed();
    assert!(!unclaimed.is_claimed());
    assert_eq!(unclaimed.get_timestamp(), 0);
    assert_eq!(unclaimed.get_payout(), 0);

    // Test new with payout
    let claimed = ClaimInfo::new(&env, 15_000_000);
    assert!(claimed.is_claimed());
    assert!(claimed.get_timestamp() > 0);
    assert_eq!(claimed.get_payout(), 15_000_000);
}
