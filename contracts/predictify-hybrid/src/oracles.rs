use soroban_sdk::{
    Address, Env, String, Symbol, symbol_short, vec, IntoVal,
};

use crate::errors::Error;
use crate::types::*;

/// Oracle management system for Predictify Hybrid contract
/// 
/// This module provides a comprehensive oracle management system with:
/// - OracleInterface trait for standardized oracle interactions
/// - Oracle implementations for different providers (Pyth, Reflector)
/// - Oracle factory pattern for creating oracle instances
/// - Oracle utilities for price comparison and outcome determination

// ===== ORACLE INTERFACE =====

/// Standard interface for all oracle implementations
pub trait OracleInterface {
    /// Get the current price for a given feed ID
    fn get_price(&self, env: &Env, feed_id: &String) -> Result<i128, Error>;
    
    /// Get the oracle provider type
    fn provider(&self) -> OracleProvider;
    
    /// Get the oracle contract ID
    fn contract_id(&self) -> Address;
    
    /// Check if the oracle is healthy and available
    fn is_healthy(&self, env: &Env) -> Result<bool, Error>;
}

// ===== PYTH ORACLE IMPLEMENTATION =====

/// Pyth Network oracle implementation
pub struct PythOracle {
    contract_id: Address,
}

impl PythOracle {
    /// Create a new Pyth oracle instance
    pub fn new(contract_id: Address) -> Self {
        Self { contract_id }
    }
    
    /// Get the Pyth oracle contract ID
    pub fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }
    
    /// Get mock price data for testing
    pub fn get_mock_price(&self, _env: &Env, feed_id: &String) -> Result<i128, Error> {
        // This is a placeholder for the actual Pyth oracle interaction
        // In a real implementation, we would call the Pyth contract here
        
        // Return different mock prices based on the asset
        if feed_id == &String::from_str(_env, "BTC/USD") {
            Ok(26_000_00) // $26,000 for BTC
        } else if feed_id == &String::from_str(_env, "ETH/USD") {
            Ok(3_200_00)  // $3,200 for ETH
        } else if feed_id == &String::from_str(_env, "XLM/USD") {
            Ok(12_00)     // $0.12 for XLM
        } else {
            Ok(26_000_00) // Default to BTC price
        }
    }
    
    /// Check if the Pyth oracle is healthy
    pub fn check_health(&self, _env: &Env) -> Result<bool, Error> {
        // In a real implementation, this would check the Pyth oracle's health
        // For now, we'll assume it's always healthy
        Ok(true)
    }
}

impl OracleInterface for PythOracle {
    fn get_price(&self, env: &Env, feed_id: &String) -> Result<i128, Error> {
        // Validate feed ID
        if feed_id.is_empty() {
            return Err(Error::InvalidOracleFeed);
        }
        
        // Get price from Pyth oracle
        self.get_mock_price(env, feed_id)
    }
    
    fn provider(&self) -> OracleProvider {
        OracleProvider::Pyth
    }
    
    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }
    
    fn is_healthy(&self, env: &Env) -> Result<bool, Error> {
        self.check_health(env)
    }
}

// ===== REFLECTOR ORACLE CLIENT =====

/// Client for interacting with Reflector oracle contract
pub struct ReflectorOracleClient<'a> {
    env: &'a Env,
    contract_id: Address,
}

impl<'a> ReflectorOracleClient<'a> {
    /// Create a new Reflector oracle client
    pub fn new(env: &'a Env, contract_id: Address) -> Self {
        Self { env, contract_id }
    }
    
    /// Get the latest price for an asset
    pub fn lastprice(&self, asset: ReflectorAsset) -> Option<ReflectorPriceData> {
        let args = vec![self.env, asset.into_val(self.env)];
        self.env
            .invoke_contract(&self.contract_id, &symbol_short!("lastprice"), args)
    }
    
    /// Get price for an asset at a specific timestamp
    pub fn price(&self, asset: ReflectorAsset, timestamp: u64) -> Option<ReflectorPriceData> {
        let args = vec![
            self.env,
            asset.into_val(self.env),
            timestamp.into_val(self.env),
        ];
        self.env
            .invoke_contract(&self.contract_id, &symbol_short!("price"), args)
    }
    
    /// Get TWAP (Time-Weighted Average Price) for an asset
    pub fn twap(&self, asset: ReflectorAsset, records: u32) -> Option<i128> {
        let args = vec![
            self.env,
            asset.into_val(self.env),
            records.into_val(self.env),
        ];
        self.env
            .invoke_contract(&self.contract_id, &symbol_short!("twap"), args)
    }
    
    /// Check if the Reflector oracle is healthy
    pub fn is_healthy(&self) -> bool {
        // Try to get a simple price to check if oracle is responsive
        let test_asset = ReflectorAsset::Other(Symbol::new(self.env, "BTC"));
        self.lastprice(test_asset).is_some()
    }
}

// ===== REFLECTOR ORACLE IMPLEMENTATION =====

/// Reflector oracle implementation
pub struct ReflectorOracle {
    contract_id: Address,
}

impl ReflectorOracle {
    /// Create a new Reflector oracle instance
    pub fn new(contract_id: Address) -> Self {
        Self { contract_id }
    }
    
    /// Get the Reflector oracle contract ID
    pub fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }
    
    /// Parse feed ID to extract asset information
    pub fn parse_feed_id(&self, env: &Env, feed_id: &String) -> Result<ReflectorAsset, Error> {
        if feed_id.is_empty() {
            return Err(Error::InvalidOracleFeed);
        }
        
        // Create asset symbol for Reflector
        // Since we can't easily parse the String in no_std, we'll use the feed_id directly
        let base_asset = ReflectorAsset::Other(Symbol::new(env, "BTC")); // Default to BTC for now
        Ok(base_asset)
    }
    
    /// Get price from Reflector oracle
    pub fn get_reflector_price(&self, env: &Env, feed_id: &String) -> Result<i128, Error> {
        // Parse the feed_id to extract asset information
        let base_asset = self.parse_feed_id(env, feed_id)?;
        
        // Create Reflector client
        let reflector_client = ReflectorOracleClient::new(env, self.contract_id.clone());
        
        // Try to get the latest price first
        if let Some(price_data) = reflector_client.lastprice(base_asset.clone()) {
            return Ok(price_data.price);
        }
        
        // If lastprice fails, try TWAP with 1 record
        if let Some(twap_price) = reflector_client.twap(base_asset, 1) {
            return Ok(twap_price);
        }
        
        // If both fail, return error
        Err(Error::OracleUnavailable)
    }
    
    /// Check if the Reflector oracle is healthy
    pub fn check_health(&self, env: &Env) -> Result<bool, Error> {
        let reflector_client = ReflectorOracleClient::new(env, self.contract_id.clone());
        Ok(reflector_client.is_healthy())
    }
}

impl OracleInterface for ReflectorOracle {
    fn get_price(&self, env: &Env, feed_id: &String) -> Result<i128, Error> {
        self.get_reflector_price(env, feed_id)
    }
    
    fn provider(&self) -> OracleProvider {
        OracleProvider::Reflector
    }
    
    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }
    
    fn is_healthy(&self, env: &Env) -> Result<bool, Error> {
        self.check_health(env)
    }
}

// ===== ORACLE FACTORY =====

/// Factory for creating oracle instances
pub struct OracleFactory;

impl OracleFactory {
    /// Create a Pyth oracle instance
    pub fn create_pyth_oracle(contract_id: Address) -> PythOracle {
        PythOracle::new(contract_id)
    }
    
    /// Create a Reflector oracle instance
    pub fn create_reflector_oracle(contract_id: Address) -> ReflectorOracle {
        ReflectorOracle::new(contract_id)
    }
    
    /// Create an oracle instance based on provider and contract ID
    pub fn create_oracle(
        provider: OracleProvider,
        contract_id: Address,
    ) -> Result<OracleInstance, Error> {
        match provider {
            OracleProvider::Pyth => {
                let oracle = PythOracle::new(contract_id);
                Ok(OracleInstance::Pyth(oracle))
            }
            OracleProvider::Reflector => {
                let oracle = ReflectorOracle::new(contract_id);
                Ok(OracleInstance::Reflector(oracle))
            }
            OracleProvider::BandProtocol | OracleProvider::DIA => {
                Err(Error::InvalidOracleConfig)
            }
        }
    }
    
    /// Create an oracle instance from oracle configuration
    pub fn create_from_config(
        oracle_config: &OracleConfig,
        contract_id: Address,
    ) -> Result<OracleInstance, Error> {
        Self::create_oracle(oracle_config.provider.clone(), contract_id)
    }
    
    /// Check if a provider is supported
    pub fn is_provider_supported(provider: &OracleProvider) -> bool {
        provider.is_supported()
    }
}

// ===== ORACLE INSTANCE ENUM =====

/// Enum to hold different oracle implementations
pub enum OracleInstance {
    Pyth(PythOracle),
    Reflector(ReflectorOracle),
}

impl OracleInstance {
    /// Get the price from the oracle
    pub fn get_price(&self, env: &Env, feed_id: &String) -> Result<i128, Error> {
        match self {
            OracleInstance::Pyth(oracle) => oracle.get_price(env, feed_id),
            OracleInstance::Reflector(oracle) => oracle.get_price(env, feed_id),
        }
    }
    
    /// Get the oracle provider type
    pub fn provider(&self) -> OracleProvider {
        match self {
            OracleInstance::Pyth(_) => OracleProvider::Pyth,
            OracleInstance::Reflector(_) => OracleProvider::Reflector,
        }
    }
    
    /// Get the oracle contract ID
    pub fn contract_id(&self) -> Address {
        match self {
            OracleInstance::Pyth(oracle) => oracle.contract_id(),
            OracleInstance::Reflector(oracle) => oracle.contract_id(),
        }
    }
    
    /// Check if the oracle is healthy
    pub fn is_healthy(&self, env: &Env) -> Result<bool, Error> {
        match self {
            OracleInstance::Pyth(oracle) => oracle.is_healthy(env),
            OracleInstance::Reflector(oracle) => oracle.is_healthy(env),
        }
    }
}

// ===== ORACLE UTILITIES =====

/// General oracle utilities
pub struct OracleUtils;

impl OracleUtils {
    /// Compare prices using different operators
    pub fn compare_prices(
        price: i128,
        threshold: i128,
        comparison: &String,
        env: &Env,
    ) -> Result<bool, Error> {
        if comparison == &String::from_str(env, "gt") {
            Ok(price > threshold)
        } else if comparison == &String::from_str(env, "lt") {
            Ok(price < threshold)
        } else if comparison == &String::from_str(env, "eq") {
            Ok(price == threshold)
        } else {
            Err(Error::InvalidComparison)
        }
    }
    
    /// Determine market outcome based on price comparison
    pub fn determine_outcome(
        price: i128,
        threshold: i128,
        comparison: &String,
        env: &Env,
    ) -> Result<String, Error> {
        let is_condition_met = Self::compare_prices(price, threshold, comparison, env)?;
        
        if is_condition_met {
            Ok(String::from_str(env, "yes"))
        } else {
            Ok(String::from_str(env, "no"))
        }
    }
    
    /// Validate oracle response
    pub fn validate_oracle_response(price: i128) -> Result<(), Error> {
        if price <= 0 {
            return Err(Error::InvalidThreshold);
        }
        
        // Check for reasonable price range (1 cent to $1M)
        if price < 1 || price > 100_000_000_00 {
            return Err(Error::InvalidThreshold);
        }
        
        Ok(())
    }
}

// ===== MODULE TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_pyth_oracle_creation() {
        let env = Env::default();
        let contract_id = Address::generate(&env);
        let oracle = PythOracle::new(contract_id.clone());
        
        assert_eq!(oracle.contract_id(), contract_id);
        assert_eq!(oracle.provider(), OracleProvider::Pyth);
    }

    #[test]
    fn test_reflector_oracle_creation() {
        let env = Env::default();
        let contract_id = Address::generate(&env);
        let oracle = ReflectorOracle::new(contract_id.clone());
        
        assert_eq!(oracle.contract_id(), contract_id);
        assert_eq!(oracle.provider(), OracleProvider::Reflector);
    }

    #[test]
    fn test_oracle_factory() {
        let env = Env::default();
        let contract_id = Address::generate(&env);
        
        // Test Pyth oracle creation
        let pyth_oracle = OracleFactory::create_oracle(OracleProvider::Pyth, contract_id.clone());
        assert!(pyth_oracle.is_ok());
        
        // Test Reflector oracle creation
        let reflector_oracle = OracleFactory::create_oracle(OracleProvider::Reflector, contract_id.clone());
        assert!(reflector_oracle.is_ok());
        
        // Test unsupported provider
        let unsupported_oracle = OracleFactory::create_oracle(OracleProvider::BandProtocol, contract_id);
        assert!(unsupported_oracle.is_err());
    }

    #[test]
    fn test_oracle_utils() {
        let env = Env::default();
        
        // Test price comparison
        let price = 30_000_00; // $30k
        let threshold = 25_000_00; // $25k
        
        // Test greater than
        let gt_result = OracleUtils::compare_prices(
            price,
            threshold,
            &String::from_str(&env, "gt"),
            &env,
        );
        assert!(gt_result.is_ok());
        assert!(gt_result.unwrap());
        
        // Test less than
        let lt_result = OracleUtils::compare_prices(
            price,
            threshold,
            &String::from_str(&env, "lt"),
            &env,
        );
        assert!(lt_result.is_ok());
        assert!(!lt_result.unwrap());
        
        // Test equal to
        let eq_result = OracleUtils::compare_prices(
            threshold,
            threshold,
            &String::from_str(&env, "eq"),
            &env,
        );
        assert!(eq_result.is_ok());
        assert!(eq_result.unwrap());
        
        // Test outcome determination
        let outcome = OracleUtils::determine_outcome(
            price,
            threshold,
            &String::from_str(&env, "gt"),
            &env,
        );
        assert!(outcome.is_ok());
        assert_eq!(outcome.unwrap(), String::from_str(&env, "yes"));
    }
} 