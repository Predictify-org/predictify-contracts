//! Test utilities for ReflectorAsset and token testing
//! Provides helper functions and common test patterns for comprehensive asset testing

use crate::tokens::Asset;
use crate::types::{OracleConfig, OracleProvider, ReflectorAsset};
use soroban_sdk::{Address, Env, String, Symbol, Vec};

/// Test utility builder for creating test assets
pub struct AssetTestBuilder {
    env: Env,
}

impl AssetTestBuilder {
    pub fn new(env: &Env) -> Self {
        Self { env: env.clone() }
    }

    /// Create a test asset with minimal validation
    pub fn create_test_asset(&self, symbol: &str, decimals: u8) -> Asset {
        Asset::new(
            Address::generate(&self.env),
            Symbol::new(&self.env, symbol),
            decimals,
        )
    }

    /// Create a native XLM asset
    pub fn native_xlm(&self) -> Asset {
        Asset::new(
            Address::default(&self.env),
            Symbol::new(&self.env, "XLM"),
            7,
        )
    }

    /// Create a test asset from a ReflectorAsset
    pub fn from_reflector_asset(&self, reflector_asset: &ReflectorAsset) -> Asset {
        let contract_address = if reflector_asset.is_xlm() {
            Address::default(&self.env)
        } else {
            Address::generate(&self.env)
        };

        Asset::from_reflector_asset(&self.env, reflector_asset, contract_address)
    }

    /// Create all supported Reflector assets as tokens
    pub fn all_supported_assets(&self) -> Vec<Asset> {
        let reflector_assets = ReflectorAsset::all_supported();
        let mut assets = Vec::new(&self.env);

        for reflector_asset in reflector_assets.iter() {
            assets.push_back(self.from_reflector_asset(reflector_asset));
        }

        assets
    }
}

/// Oracle configuration test builder
pub struct OracleConfigTestBuilder {
    env: Env,
}

impl OracleConfigTestBuilder {
    pub fn new(env: &Env) -> Self {
        Self { env: env.clone() }
    }

    /// Create a basic oracle config for a ReflectorAsset
    pub fn for_reflector_asset(
        &self,
        asset: &ReflectorAsset,
        threshold: i128,
        comparison: &str,
    ) -> OracleConfig {
        OracleConfig::new(
            OracleProvider::reflector(),
            Address::generate(&self.env),
            asset.feed_id(),
            threshold,
            String::from_str(&self.env, comparison),
        )
    }

    /// Create oracle config for "greater than" comparison
    pub fn greater_than(&self, asset: &ReflectorAsset, threshold: i128) -> OracleConfig {
        self.for_reflector_asset(asset, threshold, "gt")
    }

    /// Create oracle config for "less than" comparison
    pub fn less_than(&self, asset: &ReflectorAsset, threshold: i128) -> OracleConfig {
        self.for_reflector_asset(asset, threshold, "lt")
    }

    /// Create oracle config for "equal to" comparison
    pub fn equal_to(&self, asset: &ReflectorAsset, threshold: i128) -> OracleConfig {
        self.for_reflector_asset(asset, threshold, "eq")
    }
}

/// Matrix test utilities for comprehensive ReflectorAsset testing
pub struct ReflectorAssetMatrixTest {
    env: Env,
}

impl ReflectorAssetMatrixTest {
    pub fn new(env: &Env) -> Self {
        Self { env: env.clone() }
    }

    /// Test all supported assets with a validation function
    pub fn test_all_supported<F>(&self, mut test_fn: F)
    where
        F: FnMut(&ReflectorAsset),
    {
        let assets = ReflectorAsset::all_supported();
        for asset in assets.iter() {
            test_fn(asset);
        }
    }

    /// Test all known assets (including unsupported) with a validation function
    pub fn test_all_known<F>(&self, mut test_fn: F)
    where
        F: FnMut(&ReflectorAsset),
    {
        let assets = ReflectorAsset::all_known();
        for asset in assets.iter() {
            test_fn(asset);
        }
    }

    /// Test asset property consistency across all assets
    pub fn test_property_consistency(&self) {
        self.test_all_supported(|asset| {
            // Basic property validation
            assert!(!asset.symbol().is_empty(), "Symbol should not be empty");
            assert!(!asset.name().is_empty(), "Name should not be empty");
            assert!(asset.decimals() >= 1, "Decimals should be >= 1");
            assert!(asset.decimals() <= 18, "Decimals should be <= 18");
            assert!(!asset.feed_id().is_empty(), "Feed ID should not be empty");

            // Support status consistency
            assert!(
                asset.is_supported(),
                "All tested assets should be supported"
            );
            assert!(asset.is_known(), "All tested assets should be known");

            // Validation consistency
            assert!(
                asset.validate_for_market(&self.env).is_ok(),
                "All supported assets should validate successfully"
            );
        });
    }

    /// Test symbol round-trip conversion
    pub fn test_symbol_round_trip(&self) {
        self.test_all_supported(|asset| {
            let symbol_str = asset.symbol().to_string();
            let reconstructed =
                ReflectorAsset::from_symbol(String::from_str(&self.env, &symbol_str));
            assert_eq!(
                asset, &reconstructed,
                "Round-trip conversion should preserve asset"
            );
        });
    }

    /// Test feed ID format consistency
    pub fn test_feed_id_format(&self) {
        self.test_all_supported(|asset| {
            let feed_id = asset.feed_id().to_string();
            assert!(feed_id.contains("/USD"), "Feed ID should contain '/USD'");
            assert!(
                !feed_id.starts_with("/"),
                "Feed ID should not start with '/'"
            );
            assert!(!feed_id.ends_with("/"), "Feed ID should not end with '/'");
        });
    }

    /// Test asset-specific properties
    pub fn test_asset_specific_properties(&self) {
        let xlm = ReflectorAsset::Stellar;
        let btc = ReflectorAsset::BTC;
        let eth = ReflectorAsset::ETH;

        // XLM-specific tests
        assert!(xlm.is_xlm(), "Stellar variant should be XLM");
        assert_eq!(xlm.decimals(), 7, "XLM should have 7 decimals");
        assert_eq!(
            xlm.symbol().to_string(),
            "XLM",
            "XLM symbol should be 'XLM'"
        );

        // BTC-specific tests
        assert!(!btc.is_xlm(), "BTC should not be XLM");
        assert_eq!(btc.decimals(), 8, "BTC should have 8 decimals");
        assert_eq!(
            btc.symbol().to_string(),
            "BTC",
            "BTC symbol should be 'BTC'"
        );

        // ETH-specific tests
        assert!(!eth.is_xlm(), "ETH should not be XLM");
        assert_eq!(eth.decimals(), 18, "ETH should have 18 decimals");
        assert_eq!(
            eth.symbol().to_string(),
            "ETH",
            "ETH symbol should be 'ETH'"
        );
    }
}

/// Token registry test utilities
pub struct TokenRegistryTestUtils {
    env: Env,
}

impl TokenRegistryTestUtils {
    pub fn new(env: &Env) -> Self {
        Self { env: env.clone() }
    }

    /// Initialize registry with test assets
    pub fn setup_test_registry(&self) {
        crate::tokens::TokenRegistry::initialize_with_defaults(&self.env);
    }

    /// Add a test asset to the global registry
    pub fn add_test_asset(&self, symbol: &str, decimals: u8) -> Asset {
        let asset = Asset::new(
            Address::generate(&self.env),
            Symbol::new(&self.env, symbol),
            decimals,
        );

        crate::tokens::TokenRegistry::add_global(&self.env, &asset);
        asset
    }

    /// Verify registry contains expected assets
    pub fn verify_registry_contains(&self, expected_symbols: Vec<&str>) {
        let global_assets = crate::tokens::TokenRegistry::get_global_assets(&self.env);

        for symbol in expected_symbols.iter() {
            let symbol_found = global_assets
                .iter()
                .any(|asset| asset.symbol.to_string() == *symbol);
            assert!(
                symbol_found,
                "Registry should contain asset with symbol: {}",
                symbol
            );
        }
    }

    /// Test asset validation in registry context
    pub fn test_asset_validation(&self) {
        let valid_asset = self.add_test_asset("VALID", 7);
        let invalid_asset = Asset::new(
            Address::default(&self.env), // Invalid for non-XLM
            Symbol::new(&self.env, "INVALID"),
            19, // Invalid decimals
        );

        // Valid asset should pass
        assert!(
            crate::tokens::TokenRegistry::validate_asset(&self.env, &valid_asset, None).is_ok(),
            "Valid asset should pass registry validation"
        );

        // Invalid asset should fail
        assert!(
            crate::tokens::TokenRegistry::validate_asset(&self.env, &invalid_asset, None).is_err(),
            "Invalid asset should fail registry validation"
        );
    }
}

/// Integration test utilities for end-to-end testing
pub struct IntegrationTestUtils {
    env: Env,
    asset_builder: AssetTestBuilder,
    oracle_builder: OracleConfigTestBuilder,
}

impl IntegrationTestUtils {
    pub fn new(env: &Env) -> Self {
        Self {
            env: env.clone(),
            asset_builder: AssetTestBuilder::new(env),
            oracle_builder: OracleConfigTestBuilder::new(env),
        }
    }

    /// Create a complete market setup for a ReflectorAsset
    pub fn create_market_setup(&self, asset: &ReflectorAsset) -> (Asset, OracleConfig) {
        let token = self.asset_builder.from_reflector_asset(asset);
        let oracle_config = self.oracle_builder.greater_than(asset, 10000000);
        (token, oracle_config)
    }

    /// Test market creation with all supported assets
    pub fn test_market_creation_with_all_assets(&self, contract_id: &soroban_sdk::BytesN<32>) {
        use crate::PredictifyHybrid;

        let admin = Address::generate(&self.env);

        self.env.as_contract(contract_id, || {
            PredictifyHybrid::initialize(self.env.clone(), admin.clone(), None, None);

            let assets = ReflectorAsset::all_supported();
            for asset in assets.iter() {
                let (token, oracle_config) = self.create_market_setup(asset);

                let outcomes = Vec::from_array(
                    &self.env,
                    [
                        String::from_str(&self.env, "yes"),
                        String::from_str(&self.env, "no"),
                    ],
                );

                let market_id = PredictifyHybrid::create_market(
                    self.env.clone(),
                    admin.clone(),
                    String::from_str(
                        &self.env,
                        &format!("Test market for {}", asset.name().to_string()),
                    ),
                    outcomes,
                    30,
                    oracle_config,
                    None,
                    3600,
                );

                // Verify market was created
                let market = PredictifyHybrid::get_market(self.env.clone(), market_id);
                assert_eq!(market.oracle_config.feed_id, asset.feed_id());
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_test_builder() {
        let env = Env::default();
        let builder = AssetTestBuilder::new(&env);

        let xlm = builder.native_xlm();
        assert!(xlm.is_native_xlm(&env));
        assert_eq!(xlm.symbol.to_string(), "XLM");

        let btc_token = builder.from_reflector_asset(&ReflectorAsset::BTC);
        assert_eq!(btc_token.symbol.to_string(), "BTC");
        assert_eq!(btc_token.decimals(), 8);
    }

    #[test]
    fn test_oracle_config_builder() {
        let env = Env::default();
        let builder = OracleConfigTestBuilder::new(&env);

        let btc = ReflectorAsset::BTC;
        let config = builder.greater_than(&btc, 50000000);

        assert_eq!(config.feed_id.to_string(), "BTC/USD");
        assert_eq!(config.threshold, 50000000);
        assert_eq!(config.comparison.to_string(), "gt");
    }

    #[test]
    fn test_reflector_asset_matrix() {
        let env = Env::default();
        let matrix = ReflectorAssetMatrixTest::new(&env);

        matrix.test_property_consistency();
        matrix.test_symbol_round_trip();
        matrix.test_feed_id_format();
        matrix.test_asset_specific_properties();
    }

    #[test]
    fn test_token_registry_utils() {
        let env = Env::default();
        let utils = TokenRegistryTestUtils::new(&env);

        utils.setup_test_registry();
        utils.verify_registry_contains(vec!["XLM", "BTC", "ETH"]);
        utils.test_asset_validation();
    }
}
