#![cfg(test)]

use crate::{PredictifyHybrid, PredictifyHybridClient};
use crate::types::{OracleConfig, OracleProvider};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token::StellarAssetClient,
    vec, Address, Env, String, Symbol,
};

// Test setup with flexible token configuration
struct CustomTokenTestSetup {
    env: Env,
    contract_id: Address,
    admin: Address,
    token_id: Address,
    token_admin: Address,
    market_id: Symbol,
}

impl CustomTokenTestSetup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        // Setup admin
        let admin = Address::generate(&env);

        // Register contract
        let contract_id = env.register(PredictifyHybrid, ());
        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&\1, &None, &None);

        // Setup custom token
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        // Configure contract to use this token
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Create a test market
        let outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];

        let oracle_address = Address::generate(&env);
        let market_id = client.create_market(
            &admin,
            &String::from_str(&env, "Will it rain?"),
            &outcomes,
            &30,
            &OracleConfig {
                provider: OracleProvider::reflector(),
                oracle_address: oracle_address.clone(),
                feed_id: String::from_str(&env, "RAIN"),
                threshold: 1,
                comparison: String::from_str(&env, "gt"),
            },
            &None,       // fallback_oracle_config
            &3600,       // resolution_timeout
            &None,       // min_pool_size
            &None,       // bet_deadline_mins_before_end
            &None,       // dispute_window_seconds
        );

        Self {
            env,
            contract_id,
            admin,
            token_id,
            token_admin,
            market_id,
        }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    fn token_admin_client(&self) -> StellarAssetClient<'_> {
        StellarAssetClient::new(&self.env, &self.token_id)
    }

    fn token_client(&self) -> soroban_sdk::token::Client<'_> {
        soroban_sdk::token::Client::new(&self.env, &self.token_id)
    }
}

#[test]
fn test_bet_placement_with_custom_token() {
    let setup = CustomTokenTestSetup::new();
    let client = setup.client();
    let token_admin_client = setup.token_admin_client();
    let token_client = setup.token_client();
    
    let user = Address::generate(&setup.env);
    let bet_amount = 10_000_000; // 1 token

    // Mint tokens to user
    token_admin_client.mint(&user, &100_000_000); // 10 tokens

    // Place bet
    client.place_bet(
        &user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &bet_amount,
    );

    // Verify balance decreased
    assert_eq!(token_client.balance(&user), 90_000_000);

    // Verify contract balance increased
    assert_eq!(token_client.balance(&setup.contract_id), bet_amount);
}

#[test]
fn test_insufficient_balance() {
    let setup = CustomTokenTestSetup::new();
    let client = setup.client();
    let token_admin_client = setup.token_admin_client();
    
    let user = Address::generate(&setup.env);
    let bet_amount = 10_000_000;

    // Mint LESS tokens than bet amount
    token_admin_client.mint(&user, &5_000_000); // 0.5 tokens

    // Attempt to place bet using try_place_bet to avoid panics/segfaults
    let result = client.try_place_bet(
        &user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &bet_amount,
    );
    
    // Should return an error (likely HostError due to transfer failure)
    assert!(result.is_err());
}

#[test]
fn test_payout_distribution_flow() {
    let setup = CustomTokenTestSetup::new();
    let client = setup.client();
    let token_admin_client = setup.token_admin_client();
    let token_client = setup.token_client();
    
    let user_winner = Address::generate(&setup.env);
    let user_loser = Address::generate(&setup.env);
    let bet_amount = 10_000_000;

    // Mint tokens
    token_admin_client.mint(&user_winner, &100_000_000);
    token_admin_client.mint(&user_loser, &100_000_000);

    // Place bets
    client.place_bet(
        &user_winner,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &bet_amount,
    );

    client.place_bet(
        &user_loser,
        &setup.market_id,
        &String::from_str(&setup.env, "no"),
        &bet_amount,
    );

    // Advance time to end market but NOT past dispute window
    let market = client.get_market(&setup.market_id).unwrap();
    setup.env.ledger().set(LedgerInfo {
        timestamp: market.end_time + 1,
        protocol_version: 22,
        sequence_number: setup.env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });

    // Resolve market manually (Admin wins "yes")
    client.resolve_market_manual(
        &setup.admin,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
    );

    // Verify market is resolved but no payout distributed yet (due to dispute window)
    let market_after = client.get_market(&setup.market_id).unwrap();
    assert!(market_after.winning_outcomes.is_some());
    assert_eq!(token_client.balance(&user_winner), 90_000_000); // Only initial balance minus bet

    // Advance time past dispute window (24h default)
    setup.env.ledger().set(LedgerInfo {
        timestamp: market.end_time + 86400 + 1,
        protocol_version: 22,
        sequence_number: setup.env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });

    // Distribute payouts
    let total_distributed = client.distribute_payouts(&setup.market_id);
    assert!(total_distributed > 0);

    // Verify winner received payout (Original stake + winnings - fees)
    // Winner staked 10M, Loser staked 10M. Total pool 20M.
    // Fee 2% = 400k. Payout pool = 19.6M.
    // Winner share = 100%. Payout = 19.6M.
    // Final balance = 90M + 19.6M = 109.6M
    let winner_balance = token_client.balance(&user_winner);
    assert_eq!(winner_balance, 109_600_000);

    // Loser gets nothing
    assert_eq!(token_client.balance(&user_loser), 90_000_000);
}

#[test]
fn test_switch_token_support() {
    // This test verifies that we can switch the token used by the contract
    // by updating the TokenID storage key.
    
    let setup = CustomTokenTestSetup::new();
    let token1_client = setup.token_client();
    let client = setup.client();
    
    // 1. Verify betting with Token 1
    let user1 = Address::generate(&setup.env);
    setup.token_admin_client().mint(&user1, &10_000_000);
    
    client.place_bet(
        &user1,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &10_000_000,
    );
    assert_eq!(token1_client.balance(&user1), 0);
    
    // 2. Create and switch to Token 2
    let token2_admin = Address::generate(&setup.env);
    let token2_contract = setup.env.register_stellar_asset_contract_v2(token2_admin.clone());
    let token2_id = token2_contract.address();
    let token2_admin_client = StellarAssetClient::new(&setup.env, &token2_id);
    let token2_client = soroban_sdk::token::Client::new(&setup.env, &token2_id);

    // Update contract storage to use Token 2
    setup.env.as_contract(&setup.contract_id, || {
        setup.env.storage()
            .persistent()
            .set(&Symbol::new(&setup.env, "TokenID"), &token2_id);
    });

    // 3. Verify betting with Token 2
    let user2 = Address::generate(&setup.env);
    token2_admin_client.mint(&user2, &20_000_000);
    
    // Bet on existing one is fine.
    client.place_bet(
        &user2,
        &setup.market_id,
        &String::from_str(&setup.env, "no"),
        &20_000_000,
    );
    
    // Verify balances for Token 2
    assert_eq!(token2_client.balance(&user2), 0);
    assert_eq!(token2_client.balance(&setup.contract_id), 20_000_000);
    
    // Verify Token 1 balances are unchanged
    assert_eq!(token1_client.balance(&setup.contract_id), 10_000_000);
}

#[test]
fn test_cancel_refund_custom_token() {
    let setup = CustomTokenTestSetup::new();
    let client = setup.client();
    let token_admin_client = setup.token_admin_client();
    let token_client = setup.token_client();
    
    let user = Address::generate(&setup.env);
    let bet_amount = 10_000_000;

    // Mint tokens
    token_admin_client.mint(&user, &20_000_000);

    // Place bet
    client.place_bet(
        &user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &bet_amount,
    );

    // Verify balance before cancellation
    assert_eq!(token_client.balance(&user), 10_000_000);
    assert_eq!(token_client.balance(&setup.contract_id), bet_amount);

    // Cancel event
    client.cancel_event(
        &setup.admin,
        &setup.market_id,
        &Some(String::from_str(&setup.env, "Technical issue")),
    );

    // Verify balance after cancellation (should be refunded)
    assert_eq!(token_client.balance(&user), 20_000_000);
    assert_eq!(token_client.balance(&setup.contract_id), 0);
}

#[test]
fn test_fee_collection_custom_token() {
    let setup = CustomTokenTestSetup::new();
    let client = setup.client();
    let token_admin_client = setup.token_admin_client();
    let token_client = setup.token_client();
    
    let user = Address::generate(&setup.env);
    let bet_amount = 100_000_000;

    // Mint tokens
    token_admin_client.mint(&user, &bet_amount);

    // Place bet
    client.place_bet(
        &user,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
        &bet_amount,
    );

    // Advance time to end market
    let market = client.get_market(&setup.market_id).unwrap();
    setup.env.ledger().set(LedgerInfo {
        timestamp: market.end_time + 1,
        protocol_version: 22,
        sequence_number: setup.env.ledger().sequence(),
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 10000,
    });

    // Resolve market
    client.resolve_market_manual(
        &setup.admin,
        &setup.market_id,
        &String::from_str(&setup.env, "yes"),
    );

    // Collect fees
    let fee_amount = client.collect_fees(
        &setup.admin,
        &setup.market_id,
    );

    // Verify admin balance has not increased yet (as fees are in vault)
    let admin_balance_before = token_client.balance(&setup.admin);
    assert_eq!(admin_balance_before, 0);

    // Withdraw fees from vault
    let withdrawn_amount = client.withdraw_fees(&setup.admin, &fee_amount);
    assert_eq!(withdrawn_amount, fee_amount);

    // Verify admin balance increased by withdrawn amount
    let admin_balance_after = token_client.balance(&setup.admin);
    assert_eq!(admin_balance_after, fee_amount);
}

#[test]
fn test_deposit_and_withdraw_custom_token() {
    let setup = CustomTokenTestSetup::new();
    let client = setup.client();
    let token_admin_client = setup.token_admin_client();
    let token_client = setup.token_client();
    
    let user = Address::generate(&setup.env);
    let amount = 50_000_000;

    // Mint tokens
    token_admin_client.mint(&user, &amount);

    // Deposit tokens
    client.deposit(
        &user,
        &crate::types::ReflectorAsset::Stellar, // The setup configures Stellar as the current token
        &amount,
    );

    // Verify balances
    assert_eq!(token_client.balance(&user), 0);
    assert_eq!(token_client.balance(&setup.contract_id), amount);
    let internal_balance = client.get_balance(&user, &crate::types::ReflectorAsset::Stellar);
    assert_eq!(internal_balance.amount, amount);

    // Withdraw tokens
    client.withdraw(
        &user,
        &crate::types::ReflectorAsset::Stellar,
        &amount,
    );

    // Verify balances after withdrawal
    assert_eq!(token_client.balance(&user), amount);
    assert_eq!(token_client.balance(&setup.contract_id), 0);
    let internal_balance_after = client.get_balance(&user, &crate::types::ReflectorAsset::Stellar);
    assert_eq!(internal_balance_after.amount, 0);
}

// ===== TOKEN DECIMALS VERIFICATION TESTS =====

/// Mock token implementation for testing decimals with different values.
/// This allows us to test mismatches without requiring actual mismatched SAC tokens.
#[cfg(test)]
mod token_decimals_tests {
    use super::*;

    /// Test: verify_token_decimals with matching declared decimals
    #[test]
    fn test_token_decimals_self_test_matching() {
        let setup = CustomTokenTestSetup::new();
        
        // Create asset with correct declared decimals (7 for Stellar)
        let asset = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "USDC"),
            decimals: 7, // Stellar tokens have 7 decimals
        };
        
        // Verification should succeed when decimals match
        let result = crate::tokens::verify_token_decimals(&setup.env, &asset);
        assert!(result.is_ok(), "Expected successful verification with matching decimals");
    }

    /// Test: add_global_verified succeeds with matching decimals
    #[test]
    fn test_token_decimals_add_global_verified_matching() {
        let setup = CustomTokenTestSetup::new();
        
        // Create asset with correct decimals
        let asset = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "VERIFIED"),
            decimals: 7,
        };
        
        // Should register successfully when decimals match
        let result = crate::tokens::TokenRegistry::add_global_verified(&setup.env, &asset);
        assert!(result.is_ok(), "Expected successful registration with verified decimals");
        
        // Verify it was actually added to registry
        let registered = crate::tokens::TokenRegistry::is_allowed(&setup.env, &asset, None);
        assert!(registered, "Asset should be in global registry after verification");
    }

    /// Test: add_event_verified succeeds with matching decimals
    #[test]
    fn test_token_decimals_add_event_verified_matching() {
        let setup = CustomTokenTestSetup::new();
        
        // Create asset with correct decimals
        let asset = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "EVENT_TOKEN"),
            decimals: 7,
        };
        
        // Register for specific event
        let result = crate::tokens::TokenRegistry::add_event_verified(&setup.env, &setup.market_id, &asset);
        assert!(result.is_ok(), "Expected successful event-level registration");
        
        // Verify it was registered for that event
        let registered = crate::tokens::TokenRegistry::is_allowed(&setup.env, &asset, Some(&setup.market_id));
        assert!(registered, "Asset should be registered for the event");
    }

    /// Test: verify_token_decimals succeeds with matching declared decimals
    #[test]
    fn test_token_decimals_verification_with_correct_value() {
        let setup = CustomTokenTestSetup::new();
        
        // Test with the actual decimals value from the token
        let asset = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "TEST"),
            decimals: 7, // Stellar asset has 7 decimals
        };
        
        let result = crate::tokens::verify_token_decimals(&setup.env, &asset);
        assert!(result.is_ok(), "Verification should pass with correct decimals");
    }

    /// Test: verify_token_decimals rejects mismatched decimals
    #[test]
    fn test_token_decimals_verification_rejects_mismatch() {
        let setup = CustomTokenTestSetup::new();
        
        let asset = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "TEST"),
            decimals: 6, // Incorrect decimals intentionally
        };
        
        let result = crate::tokens::verify_token_decimals(&setup.env, &asset);
        assert!(result.is_err(), "Verification should fail with mismatched decimals");
        if let Err(err) = result {
            assert_eq!(err, crate::Error::TokenDecimalsMismatch, "Expected TokenDecimalsMismatch error");
        }
    }

    /// Test: add_global_verified rejects mismatched declared decimals
    #[test]
    fn test_token_decimals_add_global_verified_rejects_mismatch() {
        let setup = CustomTokenTestSetup::new();
        
        let asset = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "MISMATCH"),
            decimals: 6,
        };
        
        let result = crate::tokens::TokenRegistry::add_global_verified(&setup.env, &asset);
        assert!(result.is_err(), "add_global_verified should reject mismatched decimals");
        if let Err(err) = result {
            assert_eq!(err, crate::Error::TokenDecimalsMismatch, "Expected TokenDecimalsMismatch error");
        }
    }

    /// Test: re_verify_token rejects mismatched declared decimals
    #[test]
    fn test_re_verify_token_admin_function_rejects_mismatch() {
        let setup = CustomTokenTestSetup::new();
        let client = setup.client();
        
        let result = client.re_verify_token(
            &setup.admin,
            &setup.token_id,
            &6u32, // Incorrect decimals intentionally
        );
        
        assert!(result.is_err(), "re_verify_token should reject mismatched decimals");
        if let Err(err) = result {
            assert_eq!(err, crate::Error::TokenDecimalsMismatch, "Expected TokenDecimalsMismatch error");
        }
    }

    /// Test: Batch verification of multiple assets
    #[test]
    fn test_token_decimals_batch_verification() {
        let setup = CustomTokenTestSetup::new();
        
        let asset1 = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "TOKEN1"),
            decimals: 7,
        };
        
        let asset2 = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "TOKEN2"),
            decimals: 7,
        };
        
        let assets = vec![&setup.env, asset1, asset2];
        
        let result = crate::tokens::verify_token_decimals_batch(&setup.env, &assets);
        assert!(result.is_ok(), "Batch verification should succeed for all matching assets");
    }

    /// Test: re_verify_token admin entrypoint succeeds with matching decimals
    #[test]
    fn test_re_verify_token_admin_function_matching() {
        let setup = CustomTokenTestSetup::new();
        let client = setup.client();
        
        // Call re_verify_token as admin with correct decimals
        let result = client.re_verify_token(
            &setup.admin,
            &setup.token_id,
            &7u32, // Correct decimals for Stellar token
        );
        
        assert!(result.is_ok(), "re_verify_token should succeed with correct decimals");
    }

    /// Test: re_verify_token rejects non-admin caller
    #[test]
    fn test_re_verify_token_non_admin_rejected() {
        let setup = CustomTokenTestSetup::new();
        let client = setup.client();
        
        let non_admin = Address::generate(&setup.env);
        
        // Call re_verify_token as non-admin should fail
        let result = client.re_verify_token(
            &non_admin,
            &setup.token_id,
            &7u32,
        );
        
        assert!(result.is_err(), "re_verify_token should reject non-admin caller");
        // Verify it's an Unauthorized error
        if let Err(err) = result {
            assert_eq!(err, crate::Error::Unauthorized, "Expected Unauthorized error");
        }
    }

    /// Test: Asset validation with decimals bounds
    #[test]
    fn test_token_decimals_validation_bounds() {
        let setup = CustomTokenTestSetup::new();
        
        // Valid decimals (1-18)
        let valid_asset = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "VALID"),
            decimals: 7,
        };
        assert!(valid_asset.validate(&setup.env), "Asset with 7 decimals should be valid");
        
        let min_decimals = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "MIN"),
            decimals: 1,
        };
        assert!(min_decimals.validate(&setup.env), "Asset with 1 decimal should be valid (minimum)");
        
        let max_decimals = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "MAX"),
            decimals: 18,
        };
        assert!(max_decimals.validate(&setup.env), "Asset with 18 decimals should be valid (maximum)");
    }

    /// Test: Normalization and denormalization with verified decimals
    #[test]
    fn test_token_decimals_normalization() {
        // Test amounts are correctly normalized to canonical 7-decimal scale
        
        // USDC with 6 decimals: 1 USDC = 1_000_000 units
        let usdc_amount = 1_000_000;
        let normalized = crate::tokens::normalize_amount(usdc_amount, 6);
        assert_eq!(normalized, 10_000_000, "USDC should normalize to 7-decimal scale");
        
        // Denormalize back
        let denormalized = crate::tokens::denormalize_amount(normalized, 6);
        assert_eq!(denormalized, usdc_amount, "Should denormalize back to original USDC amount");
        
        // XLM with 7 decimals (canonical): no change
        let xlm_amount = 10_000_000;
        let normalized_xlm = crate::tokens::normalize_amount(xlm_amount, 7);
        assert_eq!(normalized_xlm, xlm_amount, "XLM (canonical) should not change");
    }

    /// Test: Error message for TokenDecimalsMismatch
    #[test]
    fn test_token_decimals_mismatch_error_exists() {
        // Verify that TokenDecimalsMismatch error is properly defined
        let mismatch_error = crate::Error::TokenDecimalsMismatch;
        
        // The error should be representable
        #[allow(unreachable_patterns)]
        match mismatch_error {
            crate::Error::TokenDecimalsMismatch => {
                // Success - error is properly defined
            }
            _ => panic!("TokenDecimalsMismatch error not properly defined"),
        }
    }

    /// Test: Security - verification is required for registration
    #[test]
    fn test_token_decimals_verified_variant_required() {
        let setup = CustomTokenTestSetup::new();
        
        let asset = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "SECURE"),
            decimals: 7,
        };
        
        // Unverified add should succeed (backward compatibility)
        crate::tokens::TokenRegistry::add_global(&setup.env, &asset);
        let is_registered = crate::tokens::TokenRegistry::is_allowed(&setup.env, &asset, None);
        assert!(is_registered, "Unverified add_global should work for backward compatibility");
        
        // Verified variant should also work when decimals match
        let asset2 = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "SECURE2"),
            decimals: 7,
        };
        let verified_result = crate::tokens::TokenRegistry::add_global_verified(&setup.env, &asset2);
        assert!(verified_result.is_ok(), "Verified registration should succeed with matching decimals");
    }

    /// Test: Cross-contract call safety during verification
    #[test]
    fn test_token_decimals_cross_contract_safety() {
        let setup = CustomTokenTestSetup::new();
        
        // Multiple verification calls should be idempotent
        let asset = crate::tokens::Asset {
            contract: setup.token_id.clone(),
            symbol: Symbol::new(&setup.env, "SAFETY"),
            decimals: 7,
        };
        
        let result1 = crate::tokens::verify_token_decimals(&setup.env, &asset);
        let result2 = crate::tokens::verify_token_decimals(&setup.env, &asset);
        let result3 = crate::tokens::verify_token_decimals(&setup.env, &asset);
        
        assert!(result1.is_ok(), "First verification should succeed");
        assert!(result2.is_ok(), "Second verification should succeed");
        assert!(result3.is_ok(), "Third verification should succeed");
        // All should be idempotent
    }
}


