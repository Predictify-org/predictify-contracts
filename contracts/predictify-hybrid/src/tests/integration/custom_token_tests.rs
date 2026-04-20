//! Tests for custom Stellar token/asset support in bets and payouts
//! Covers XLM-native and custom token flows, insufficient balance, and event emission
//! Extended for comprehensive ReflectorAsset coverage matrix testing

use super::super::super::*;
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};

#[test]
fn test_place_bet_with_custom_token() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = crate::tokens::Asset {
        contract: Address::generate(&env),
        symbol: Symbol::new(&env, "USDC"),
        decimals: 7,
    };

    env.as_contract(&contract_id, || {
        // Initialize contract with allowed asset
        PredictifyHybrid::initialize(
            env.clone(),
            admin.clone(),
            None,
            Some(vec![&env, asset.clone()]),
        );

        // Create market with custom asset
        let outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];
        let oracle_config = OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::generate(&env),
            feed_id: String::from_str(&env, "BTC/USD"),
            threshold: 10000000,
            comparison: String::from_str(&env, "gt"),
        };
        let market_id = PredictifyHybrid::create_market(
            env.clone(),
            admin.clone(),
            String::from_str(&env, "Will BTC exceed $100k?"),
            outcomes,
            30,
            oracle_config,
            None,
            3600,
        );

        // Place bet with custom token
        let bet = PredictifyHybrid::place_bet(
            env.clone(),
            user.clone(),
            market_id.clone(),
            String::from_str(&env, "yes"),
            1000000,
            Some(asset.clone()),
        );
        assert_eq!(bet.amount, 1000000);
        assert_eq!(bet.outcome, String::from_str(&env, "yes"));
    });
}

#[test]
fn test_place_bet_with_xlm_native() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Initialize contract (no custom asset)
        PredictifyHybrid::initialize(env.clone(), admin.clone(), None, None);

        // Create market (XLM-native)
        let outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];
        let oracle_config = OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::generate(&env),
            feed_id: String::from_str(&env, "BTC/USD"),
            threshold: 10000000,
            comparison: String::from_str(&env, "gt"),
        };
        let market_id = PredictifyHybrid::create_market(
            env.clone(),
            admin.clone(),
            String::from_str(&env, "Will BTC exceed $100k?"),
            outcomes,
            30,
            oracle_config,
            None,
            3600,
        );

        // Place bet with XLM-native
        let bet = PredictifyHybrid::place_bet(
            env.clone(),
            user.clone(),
            market_id.clone(),
            String::from_str(&env, "yes"),
            1000000,
            None,
        );
        assert_eq!(bet.amount, 1000000);
        assert_eq!(bet.outcome, String::from_str(&env, "yes"));
    });
}

#[test]
fn test_insufficient_balance_for_custom_token() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let asset = crate::tokens::Asset {
        contract: Address::generate(&env),
        symbol: Symbol::new(&env, "USDC"),
        decimals: 7,
    };

    env.as_contract(&contract_id, || {
        PredictifyHybrid::initialize(
            env.clone(),
            admin.clone(),
            None,
            Some(vec![&env, asset.clone()]),
        );
        let outcomes = vec![
            &env,
            String::from_str(&env, "yes"),
            String::from_str(&env, "no"),
        ];
        let oracle_config = OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::generate(&env),
            feed_id: String::from_str(&env, "BTC/USD"),
            threshold: 10000000,
            comparison: String::from_str(&env, "gt"),
        };
        let market_id = PredictifyHybrid::create_market(
            env.clone(),
            admin.clone(),
            String::from_str(&env, "Will BTC exceed $100k?"),
            outcomes,
            30,
            oracle_config,
            None,
            3600,
        );
        // Simulate insufficient balance (should panic or return error)
        let result = std::panic::catch_unwind(|| {
            PredictifyHybrid::place_bet(
                env.clone(),
                user.clone(),
                market_id.clone(),
                String::from_str(&env, "yes"),
                999999999999,
                Some(asset.clone()),
            );
        });
        assert!(result.is_err());
    });
}

// ===== REFLECTOR ASSET COVERAGE MATRIX TESTS =====

#[test]
fn test_reflector_asset_symbol_methods() {
    let env = Env::default();

    // Test all supported assets
    let xlm = crate::types::ReflectorAsset::Stellar;
    let btc = crate::types::ReflectorAsset::BTC;
    let eth = crate::types::ReflectorAsset::ETH;
    let custom = crate::types::ReflectorAsset::Other(Symbol::new(&env, "CUSTOM"));

    assert_eq!(xlm.symbol().to_string(), "XLM");
    assert_eq!(btc.symbol().to_string(), "BTC");
    assert_eq!(eth.symbol().to_string(), "ETH");
    assert_eq!(custom.symbol().to_string(), "CUSTOM");
}

#[test]
fn test_reflector_asset_name_methods() {
    let env = Env::default();

    let xlm = crate::types::ReflectorAsset::Stellar;
    let btc = crate::types::ReflectorAsset::BTC;
    let eth = crate::types::ReflectorAsset::ETH;
    let custom = crate::types::ReflectorAsset::Other(Symbol::new(&env, "CUSTOM"));

    assert_eq!(xlm.name().to_string(), "Stellar Lumens");
    assert_eq!(btc.name().to_string(), "Bitcoin");
    assert_eq!(eth.name().to_string(), "Ethereum");
    assert!(custom.name().to_string().contains("CUSTOM"));
}

#[test]
fn test_reflector_asset_decimals() {
    let xlm = crate::types::ReflectorAsset::Stellar;
    let btc = crate::types::ReflectorAsset::BTC;
    let eth = crate::types::ReflectorAsset::ETH;
    let custom = crate::types::ReflectorAsset::Other(Symbol::new(&Env::default(), "CUSTOM"));

    assert_eq!(xlm.decimals(), 7);
    assert_eq!(btc.decimals(), 8);
    assert_eq!(eth.decimals(), 18);
    assert_eq!(custom.decimals(), 7); // Default for custom assets
}

#[test]
fn test_reflector_asset_feed_ids() {
    let env = Env::default();

    let xlm = crate::types::ReflectorAsset::Stellar;
    let btc = crate::types::ReflectorAsset::BTC;
    let eth = crate::types::ReflectorAsset::ETH;
    let custom = crate::types::ReflectorAsset::Other(Symbol::new(&env, "CUSTOM"));

    assert_eq!(xlm.feed_id().to_string(), "XLM/USD");
    assert_eq!(btc.feed_id().to_string(), "BTC/USD");
    assert_eq!(eth.feed_id().to_string(), "ETH/USD");
    assert_eq!(custom.feed_id().to_string(), "CUSTOM/USD");
}

#[test]
fn test_reflector_asset_support() {
    let xlm = crate::types::ReflectorAsset::Stellar;
    let btc = crate::types::ReflectorAsset::BTC;
    let eth = crate::types::ReflectorAsset::ETH;
    let custom = crate::types::ReflectorAsset::Other(Symbol::new(&Env::default(), "CUSTOM"));

    assert!(xlm.is_supported());
    assert!(btc.is_supported());
    assert!(eth.is_supported());
    assert!(!custom.is_supported()); // Custom assets not supported by default

    assert!(xlm.is_known());
    assert!(btc.is_known());
    assert!(eth.is_known());
    assert!(custom.is_known()); // All variants are known
}

#[test]
fn test_reflector_asset_validation() {
    let env = Env::default();

    let xlm = crate::types::ReflectorAsset::Stellar;
    let btc = crate::types::ReflectorAsset::BTC;
    let eth = crate::types::ReflectorAsset::ETH;
    let custom = crate::types::ReflectorAsset::Other(Symbol::new(&env, "CUSTOM"));

    // Supported assets should validate successfully
    assert!(xlm.validate_for_market(&env).is_ok());
    assert!(btc.validate_for_market(&env).is_ok());
    assert!(eth.validate_for_market(&env).is_ok());

    // Custom assets should fail validation
    assert!(custom.validate_for_market(&env).is_err());
}

#[test]
fn test_reflector_asset_from_symbol() {
    let env = Env::default();

    let xlm = crate::types::ReflectorAsset::from_symbol(String::from_str(&env, "XLM"));
    let btc = crate::types::ReflectorAsset::from_symbol(String::from_str(&env, "BTC"));
    let eth = crate::types::ReflectorAsset::from_symbol(String::from_str(&env, "ETH"));
    let custom = crate::types::ReflectorAsset::from_symbol(String::from_str(&env, "CUSTOM"));

    assert_eq!(xlm, crate::types::ReflectorAsset::Stellar);
    assert_eq!(btc, crate::types::ReflectorAsset::BTC);
    assert_eq!(eth, crate::types::ReflectorAsset::ETH);
    assert_eq!(
        custom,
        crate::types::ReflectorAsset::Other(Symbol::new(&env, "CUSTOM"))
    );
}

#[test]
fn test_reflector_asset_all_supported() {
    let supported = crate::types::ReflectorAsset::all_supported();
    assert_eq!(supported.len(), 3);

    let env = Env::default();
    let expected = vec![
        crate::types::ReflectorAsset::Stellar,
        crate::types::ReflectorAsset::BTC,
        crate::types::ReflectorAsset::ETH,
    ];

    for (i, asset) in supported.iter().enumerate() {
        assert_eq!(asset, &expected[i]);
        assert!(asset.is_supported());
    }
}

#[test]
fn test_reflector_asset_all_known() {
    let known = crate::types::ReflectorAsset::all_known();
    assert_eq!(known.len(), 4); // Includes custom asset

    let env = Env::default();
    let expected = vec![
        crate::types::ReflectorAsset::Stellar,
        crate::types::ReflectorAsset::BTC,
        crate::types::ReflectorAsset::ETH,
        crate::types::ReflectorAsset::Other(Symbol::new(&env, "CUSTOM")),
    ];

    for (i, asset) in known.iter().enumerate() {
        assert_eq!(asset, &expected[i]);
        assert!(asset.is_known());
    }
}

#[test]
fn test_reflector_asset_is_xlm() {
    let xlm = crate::types::ReflectorAsset::Stellar;
    let btc = crate::types::ReflectorAsset::BTC;
    let eth = crate::types::ReflectorAsset::ETH;
    let custom = crate::types::ReflectorAsset::Other(Symbol::new(&Env::default(), "XLM"));

    assert!(xlm.is_xlm());
    assert!(!btc.is_xlm());
    assert!(!eth.is_xlm());
    assert!(!custom.is_xlm()); // Even if symbol is XLM, it's not the Stellar variant
}

// ===== ASSET AND TOKEN REGISTRY INTEGRATION TESTS =====

#[test]
fn test_asset_from_reflector_asset() {
    let env = Env::default();
    let contract_address = Address::generate(&env);

    let btc_reflector = crate::types::ReflectorAsset::BTC;
    let btc_asset =
        crate::tokens::Asset::from_reflector_asset(&env, &btc_reflector, contract_address.clone());

    assert_eq!(btc_asset.contract, contract_address);
    assert_eq!(btc_asset.symbol, Symbol::new(&env, "BTC"));
    assert_eq!(btc_asset.decimals, 8);
}

#[test]
fn test_asset_matches_reflector_asset() {
    let env = Env::default();
    let contract_address = Address::generate(&env);

    let btc_reflector = crate::types::ReflectorAsset::BTC;
    let btc_asset =
        crate::tokens::Asset::from_reflector_asset(&env, &btc_reflector, contract_address);
    let eth_reflector = crate::types::ReflectorAsset::ETH;

    assert!(btc_asset.matches_reflector_asset(&env, &btc_reflector));
    assert!(!btc_asset.matches_reflector_asset(&env, &eth_reflector));
}

#[test]
fn test_asset_name_methods() {
    let env = Env::default();

    let xlm_asset = crate::tokens::Asset {
        contract: Address::default(&env),
        symbol: Symbol::new(&env, "XLM"),
        decimals: 7,
    };

    let btc_asset = crate::tokens::Asset {
        contract: Address::generate(&env),
        symbol: Symbol::new(&env, "BTC"),
        decimals: 8,
    };

    let usdc_asset = crate::tokens::Asset {
        contract: Address::generate(&env),
        symbol: Symbol::new(&env, "USDC"),
        decimals: 7,
    };

    let custom_asset = crate::tokens::Asset {
        contract: Address::generate(&env),
        symbol: Symbol::new(&env, "CUSTOM"),
        decimals: 9,
    };

    assert_eq!(xlm_asset.name(&env).to_string(), "Stellar Lumens");
    assert_eq!(btc_asset.name(&env).to_string(), "Bitcoin");
    assert_eq!(usdc_asset.name(&env).to_string(), "USD Coin");
    assert!(custom_asset.name(&env).to_string().contains("CUSTOM"));
}

#[test]
fn test_asset_is_native_xlm() {
    let env = Env::default();

    let native_xlm = crate::tokens::Asset {
        contract: Address::default(&env),
        symbol: Symbol::new(&env, "XLM"),
        decimals: 7,
    };

    let token_xlm = crate::tokens::Asset {
        contract: Address::generate(&env),
        symbol: Symbol::new(&env, "XLM"),
        decimals: 7,
    };

    let btc = crate::tokens::Asset {
        contract: Address::default(&env),
        symbol: Symbol::new(&env, "BTC"),
        decimals: 8,
    };

    assert!(native_xlm.is_native_xlm(&env));
    assert!(!token_xlm.is_native_xlm(&env));
    assert!(!btc.is_native_xlm(&env));
}

#[test]
fn test_token_registry_initialization() {
    let env = Env::default();

    crate::tokens::TokenRegistry::initialize_with_defaults(&env);

    let global_assets = crate::tokens::TokenRegistry::get_global_assets(&env);
    assert_eq!(global_assets.len(), 3); // XLM, BTC, ETH

    // Check that XLM is native (default address)
    let xlm_asset = &global_assets[0];
    assert!(xlm_asset.is_native_xlm(&env));
    assert_eq!(xlm_asset.symbol.to_string(), "XLM");
}

#[test]
fn test_token_registry_add_remove_global() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Initialize with defaults
        crate::tokens::TokenRegistry::initialize_with_defaults(&env);

        let new_asset = crate::tokens::Asset {
            contract: Address::generate(&env),
            symbol: Symbol::new(&env, "USDC"),
            decimals: 7,
        };

        // Add new asset
        crate::tokens::TokenRegistry::add_global(&env, &new_asset);

        let global_assets = crate::tokens::TokenRegistry::get_global_assets(&env);
        assert_eq!(global_assets.len(), 4);
        assert!(global_assets
            .iter()
            .any(|a| a.symbol == Symbol::new(&env, "USDC")));

        // Remove asset
        assert!(crate::tokens::TokenRegistry::remove_global(&env, &new_asset).is_ok());

        let global_assets = crate::tokens::TokenRegistry::get_global_assets(&env);
        assert_eq!(global_assets.len(), 3);
        assert!(!global_assets
            .iter()
            .any(|a| a.symbol == Symbol::new(&env, "USDC")));
    });
}

#[test]
fn test_token_registry_validation() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        // Initialize with defaults
        crate::tokens::TokenRegistry::initialize_with_defaults(&env);

        let valid_asset = crate::tokens::Asset {
            contract: Address::generate(&env),
            symbol: Symbol::new(&env, "USDC"),
            decimals: 7,
        };

        let invalid_asset = crate::tokens::Asset {
            contract: Address::default(&env), // Default address but not XLM
            symbol: Symbol::new(&env, "INVALID"),
            decimals: 19, // Invalid decimals
        };

        // Add valid asset to registry
        crate::tokens::TokenRegistry::add_global(&env, &valid_asset);

        // Valid asset should pass validation
        assert!(crate::tokens::TokenRegistry::validate_asset(&env, &valid_asset, None).is_ok());

        // Invalid asset should fail validation
        assert!(crate::tokens::TokenRegistry::validate_asset(&env, &invalid_asset, None).is_err());

        // Non-registered asset should fail validation
        let non_registered = crate::tokens::Asset {
            contract: Address::generate(&env),
            symbol: Symbol::new(&env, "UNKNOWN"),
            decimals: 7,
        };

        assert!(crate::tokens::TokenRegistry::validate_asset(&env, &non_registered, None).is_err());
    });
}

#[test]
fn test_market_creation_with_reflector_assets() {
    let env = Env::default();
    let contract_id = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);

    env.as_contract(&contract_id, || {
        PredictifyHybrid::initialize(env.clone(), admin.clone(), None, None);

        // Test creating markets with different Reflector assets
        let assets = crate::types::ReflectorAsset::all_supported();

        for asset in assets.iter() {
            let oracle_config = OracleConfig {
                provider: OracleProvider::Reflector,
                oracle_address: Address::generate(&env),
                feed_id: asset.feed_id(),
                threshold: 10000000,
                comparison: String::from_str(&env, "gt"),
            };

            let outcomes = vec![
                &env,
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ];
            let market_id = PredictifyHybrid::create_market(
                env.clone(),
                admin.clone(),
                String::from_str(
                    &env,
                    &format!("Will {} reach new highs?", asset.name().to_string()),
                ),
                outcomes,
                30,
                oracle_config,
                None,
                3600,
            );

            // Verify market was created successfully
            let market = PredictifyHybrid::get_market(env.clone(), market_id.clone());
            assert_eq!(market.oracle_config.feed_id, asset.feed_id());
        }
    });
}

#[test]
fn test_comprehensive_reflector_asset_matrix() {
    let env = Env::default();

    // Test all combinations of supported assets with various operations
    let assets = crate::types::ReflectorAsset::all_supported();

    for asset in assets.iter() {
        // Test basic properties
        assert!(!asset.symbol().is_empty());
        assert!(!asset.name().is_empty());
        assert!(asset.decimals() >= 1 && asset.decimals() <= 18);
        assert!(!asset.feed_id().is_empty());

        // Test support status
        assert!(asset.is_supported());
        assert!(asset.is_known());

        // Test validation
        assert!(asset.validate_for_market(&env).is_ok());

        // Test round-trip conversion
        let symbol_str = asset.symbol().to_string();
        let reconstructed =
            crate::types::ReflectorAsset::from_symbol(String::from_str(&env, &symbol_str));
        assert_eq!(asset, &reconstructed);

        // Test feed ID format
        let feed_id = asset.feed_id().to_string();
        assert!(feed_id.contains("/USD"));
    }
}

// ===== SAC TOKEN INTEGRATION TESTS =====

#[test]
fn test_sac_token_operations() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let spender = Address::generate(&env);

    // Register a dummy token contract to simulate SAC
    let token_id = env.register_stellar_asset_contract(admin.clone());
    let token_client = token::Client::new(&env, &token_id);
    let asset = crate::tokens::Asset::new(token_id.clone(), Symbol::new(&env, "TEST"), 7);

    // 1. Test Mint & Balance (Setup)
    token_client.mint(&user1, &1000);
    assert_eq!(crate::tokens::get_token_balance(&env, &asset, &user1), 1000);

    // 2. Test Transfer
    crate::tokens::transfer_token(&env, &asset, &user1, &user2, 400);
    assert_eq!(crate::tokens::get_token_balance(&env, &asset, &user1), 600);
    assert_eq!(crate::tokens::get_token_balance(&env, &asset, &user2), 400);

    // 3. Test Approve & Allowance
    let expiration = env.ledger().sequence() + 100;
    crate::tokens::approve_token(&env, &asset, &user1, &spender, 200, expiration);
    assert_eq!(
        crate::tokens::get_token_allowance(&env, &asset, &user1, &spender),
        200
    );

    // 4. Test Transfer From
    crate::tokens::transfer_from_token(&env, &asset, &spender, &user1, &user2, 100);
    assert_eq!(crate::tokens::get_token_balance(&env, &asset, &user1), 500);
    assert_eq!(crate::tokens::get_token_balance(&env, &asset, &user2), 500);
    assert_eq!(
        crate::tokens::get_token_allowance(&env, &asset, &user1, &spender),
        100
    );
}

#[test]
fn test_sac_token_failure_modes() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let recipient = Address::generate(&env);

    let token_id = env.register_stellar_asset_contract(admin.clone());
    let token_client = token::Client::new(&env, &token_id);
    let asset = crate::tokens::Asset::new(token_id.clone(), Symbol::new(&env, "TEST"), 7);

    token_client.mint(&user, &100);

    // 1. Test insufficient balance with check_token_balance
    assert!(crate::tokens::check_token_balance(&env, &asset, &user, 101).is_err());
    assert!(crate::tokens::check_token_balance(&env, &asset, &user, 100).is_ok());

    // 2. Test transfer failing due to balance (panics in Soroban)
    let result = std::panic::catch_unwind(|| {
        crate::tokens::transfer_token(&env, &asset, &user, &recipient, 101);
    });
    assert!(result.is_err());

    // 3. Test validate_token_operation
    assert!(crate::tokens::validate_token_operation(&env, &asset, &user, 100).is_ok());
    assert!(crate::tokens::validate_token_operation(&env, &asset, &user, 0).is_err()); // Invalid amount
    assert!(crate::tokens::validate_token_operation(&env, &asset, &user, 101).is_err());
    // Insufficient balance
}

#[test]
fn test_asset_native_xlm_detection() {
    let env = Env::default();

    // Our is_native_xlm heuristic currently checks the symbol "XLM".
    let asset = crate::tokens::Asset::new(Address::generate(&env), Symbol::new(&env, "XLM"), 7);
    assert!(asset.is_native_xlm(&env));

    let btc = crate::tokens::Asset::new(Address::generate(&env), Symbol::new(&env, "BTC"), 8);
    assert!(!btc.is_native_xlm(&env));
}
