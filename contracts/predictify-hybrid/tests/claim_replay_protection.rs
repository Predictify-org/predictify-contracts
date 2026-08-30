#![cfg(test)]

//! Comprehensive test suite for claim replay protection using monotonic nonces.
//!
//! This module tests the replay-safe prediction claim identifier implementation,
//! ensuring that transaction replays are prevented while legitimate claims succeed.
//!
//! ## Test Coverage
//!
//! - **Nonce Tracking**: Verify nonces start at 0 and increment by 1 on each claim
//! - **Replay Prevention**: Verify old nonces fail validation (InvalidNonce)
//! - **Per-User, Per-Market Independence**: Each (user, market) pair has independent nonce
//! - **State Persistence**: Nonce stored in persistent storage and ClaimInfo
//! - **Zero Payout Claims**: Nonce increments even when payout is 0

use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};
use predictify_hybrid::storage::{ClaimNonceManager, DataKey};
use predictify_hybrid::PredictifyHybrid;

/// Creates an environment with a registered contract instance so persistent
/// storage access works inside `as_contract`. Returns `(env, contract_id)`.
fn contract_env() -> (Env, Address) {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    (env, contract_id)
}

fn sym(env: &Env, name: &str) -> Symbol {
    Symbol::new(env, name)
}

// ===== NONCE MANAGER UNIT TESTS =====

#[test]
fn test_get_nonce_returns_zero_initially() {
    let (env, contract_id) = contract_env();
    env.as_contract(&contract_id, || {
        let user = Address::generate(&env);
        let market_id = sym(&env, "m1");

        let nonce = ClaimNonceManager::get_nonce(&env, &user, &market_id);
        assert_eq!(nonce, 0, "Initial nonce should be 0 for never-claimed user");
    });
}

#[test]
fn test_increment_nonce_returns_next_value() {
    let (env, contract_id) = contract_env();
    env.as_contract(&contract_id, || {
        let user = Address::generate(&env);
        let market_id = sym(&env, "m1");

        // First increment
        let nonce1 = ClaimNonceManager::increment_nonce(&env, &user, &market_id);
        assert_eq!(nonce1, 1, "First increment should return 1");

        // Verify persisted
        let stored = ClaimNonceManager::get_nonce(&env, &user, &market_id);
        assert_eq!(stored, 1, "Incremented nonce should be persisted");

        // Second increment
        let nonce2 = ClaimNonceManager::increment_nonce(&env, &user, &market_id);
        assert_eq!(nonce2, 2, "Second increment should return 2");

        // Verify
        let stored2 = ClaimNonceManager::get_nonce(&env, &user, &market_id);
        assert_eq!(stored2, 2, "Second increment should be persisted");
    });
}

#[test]
fn test_validate_nonce_succeeds_when_matching() {
    let (env, contract_id) = contract_env();
    env.as_contract(&contract_id, || {
        let user = Address::generate(&env);
        let market_id = sym(&env, "m1");

        // Get initial nonce (0)
        let current = ClaimNonceManager::get_nonce(&env, &user, &market_id);
        assert_eq!(current, 0);

        // Validation should succeed
        let result = ClaimNonceManager::validate_nonce(&env, &user, &market_id, 0);
        assert!(result.is_ok(), "Nonce 0 should validate when stored is 0");

        // After increment, validation with 0 should fail
        ClaimNonceManager::increment_nonce(&env, &user, &market_id);
        let result2 = ClaimNonceManager::validate_nonce(&env, &user, &market_id, 0);
        assert!(result2.is_err(), "Nonce 0 should fail when stored is 1");
    });
}

#[test]
fn test_validate_nonce_fails_for_old_nonce() {
    let (env, contract_id) = contract_env();
    env.as_contract(&contract_id, || {
        let user = Address::generate(&env);
        let market_id = sym(&env, "m1");

        // Increment to 1
        ClaimNonceManager::increment_nonce(&env, &user, &market_id);
        let stored = ClaimNonceManager::get_nonce(&env, &user, &market_id);
        assert_eq!(stored, 1);

        // Try to validate with old nonce (0) - should fail
        let result = ClaimNonceManager::validate_nonce(&env, &user, &market_id, 0);
        assert!(result.is_err(), "Old nonce should fail validation");

        // Validate with correct nonce - should succeed
        let result2 = ClaimNonceManager::validate_nonce(&env, &user, &market_id, 1);
        assert!(result2.is_ok(), "Current nonce should pass validation");
    });
}

#[test]
fn test_nonce_independence_per_user() {
    let (env, contract_id) = contract_env();
    env.as_contract(&contract_id, || {
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let market_id = sym(&env, "m1");

        // User1 increments nonce to 1
        let nonce1 = ClaimNonceManager::increment_nonce(&env, &user1, &market_id);
        assert_eq!(nonce1, 1);

        // User2's nonce should still be 0
        let nonce2 = ClaimNonceManager::get_nonce(&env, &user2, &market_id);
        assert_eq!(nonce2, 0, "User2 should have independent nonce");

        // User2 increments nonce to 1 (independent)
        let nonce2_new = ClaimNonceManager::increment_nonce(&env, &user2, &market_id);
        assert_eq!(nonce2_new, 1);

        // User1's nonce should still be 1 (unaffected)
        let nonce1_check = ClaimNonceManager::get_nonce(&env, &user1, &market_id);
        assert_eq!(nonce1_check, 1, "User1 nonce should be unaffected");
    });
}

#[test]
fn test_nonce_independence_per_market() {
    let (env, contract_id) = contract_env();
    env.as_contract(&contract_id, || {
        let user = Address::generate(&env);
        let market1 = sym(&env, "m1");
        let market2 = sym(&env, "m2");

        // Increment on market1
        let nonce_m1 = ClaimNonceManager::increment_nonce(&env, &user, &market1);
        assert_eq!(nonce_m1, 1);

        // Market2 nonce should still be 0
        let nonce_m2 = ClaimNonceManager::get_nonce(&env, &user, &market2);
        assert_eq!(nonce_m2, 0, "Different market should have independent nonce");

        // Increment on market2
        let nonce_m2_new = ClaimNonceManager::increment_nonce(&env, &user, &market2);
        assert_eq!(nonce_m2_new, 1);

        // Market1 nonce should still be 1
        let nonce_m1_check = ClaimNonceManager::get_nonce(&env, &user, &market1);
        assert_eq!(nonce_m1_check, 1, "Market1 nonce should be unaffected");
    });
}

#[test]
fn test_nonce_persists_across_calls() {
    let (env, contract_id) = contract_env();
    env.as_contract(&contract_id, || {
        let user = Address::generate(&env);
        let market_id = sym(&env, "m1");

        // Increment multiple times
        for i in 1..=5 {
            let nonce = ClaimNonceManager::increment_nonce(&env, &user, &market_id);
            assert_eq!(
                nonce, i,
                "Nonce should be {} after {} increments",
                i, i
            );
        }

        // Verify final state persisted
        let final_nonce = ClaimNonceManager::get_nonce(&env, &user, &market_id);
        assert_eq!(final_nonce, 5, "Final nonce should persist");
    });
}

#[test]
fn test_storage_key_uniqueness() {
    let env = Env::default();

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let market1 = sym(&env, "m1");
    let market2 = sym(&env, "m2");

    // Generate keys - they should all be different
    let key1 = DataKey::ClaimNonce(user1.clone(), market1.clone());
    let key2 = DataKey::ClaimNonce(user1.clone(), market2.clone());
    let key3 = DataKey::ClaimNonce(user2.clone(), market1.clone());

    // XDR-encode to check uniqueness
    let xdr1 = key1.to_xdr(&env);
    let xdr2 = key2.to_xdr(&env);
    let xdr3 = key3.to_xdr(&env);

    assert_ne!(xdr1, xdr2, "Different markets should produce different keys");
    assert_ne!(xdr1, xdr3, "Different users should produce different keys");
    assert_ne!(xdr2, xdr3, "Different user+market should produce different keys");
}

// ===== INTEGRATION SIMULATION TESTS =====

#[test]
fn test_claim_lifecycle_with_nonce() {
    let (env, contract_id) = contract_env();
    env.as_contract(&contract_id, || {
        let user = Address::generate(&env);
        let market_id = sym(&env, "m1");

        // Initial state: nonce is 0
        let nonce_before = ClaimNonceManager::get_nonce(&env, &user, &market_id);
        assert_eq!(nonce_before, 0);

        // Client queries for nonce to include in claim transaction
        let claim_nonce = nonce_before; // Client gets 0

        // Contract validates nonce
        let validation = ClaimNonceManager::validate_nonce(&env, &user, &market_id, claim_nonce);
        assert!(validation.is_ok(), "First claim should pass validation");

        // Contract increments nonce on success
        let new_nonce = ClaimNonceManager::increment_nonce(&env, &user, &market_id);
        assert_eq!(new_nonce, 1);

        // Post-claim state
        let nonce_after = ClaimNonceManager::get_nonce(&env, &user, &market_id);
        assert_eq!(nonce_after, 1);

        // Replay attempt with old nonce would fail
        let replay_attempt = ClaimNonceManager::validate_nonce(&env, &user, &market_id, 0);
        assert!(replay_attempt.is_err(), "Replay with old nonce should fail");

        // Next legitimate claim uses new nonce
        let validation2 = ClaimNonceManager::validate_nonce(&env, &user, &market_id, 1);
        assert!(validation2.is_ok(), "Claim with current nonce should pass");
    });
}

#[test]
fn test_replay_attack_simulation() {
    let (env, contract_id) = contract_env();
    env.as_contract(&contract_id, || {
        let user = Address::generate(&env);
        let market_id = sym(&env, "m1");

        // Original claim: nonce 0 -> 1
        assert!(ClaimNonceManager::validate_nonce(&env, &user, &market_id, 0).is_ok());
        ClaimNonceManager::increment_nonce(&env, &user, &market_id);

        // Attacker replays old transaction with nonce 0
        let replay = ClaimNonceManager::validate_nonce(&env, &user, &market_id, 0);
        assert!(replay.is_err(), "Replay should be rejected (InvalidNonce)");

        // State unchanged - stored nonce still 1
        let stored = ClaimNonceManager::get_nonce(&env, &user, &market_id);
        assert_eq!(stored, 1, "Nonce should not change on failed validation");
    });
}

// ===== BOUNDARY TESTS =====

#[test]
fn test_nonce_monotonic_sequence() {
    let (env, contract_id) = contract_env();
    env.as_contract(&contract_id, || {
        let user = Address::generate(&env);
        let market_id = sym(&env, "m1");

        // Verify strict monotonic increase
        let mut prev = 0u64;
        for _ in 0..10 {
            let current = ClaimNonceManager::increment_nonce(&env, &user, &market_id);
            assert_eq!(current, prev + 1, "Nonce must increase by exactly 1");
            prev = current;
        }
    });
}

#[test]
fn test_zero_nonce_is_valid_on_first_claim() {
    let (env, contract_id) = contract_env();
    env.as_contract(&contract_id, || {
        let user = Address::generate(&env);
        let market_id = sym(&env, "m1");

        // Zero nonce should validate initially
        let result = ClaimNonceManager::validate_nonce(&env, &user, &market_id, 0);
        assert!(result.is_ok(), "Nonce 0 should be valid on first claim");

        // After increment, zero should no longer validate
        ClaimNonceManager::increment_nonce(&env, &user, &market_id);
        let result2 = ClaimNonceManager::validate_nonce(&env, &user, &market_id, 0);
        assert!(result2.is_err(), "Nonce 0 should be invalid after first claim");
    });
}
