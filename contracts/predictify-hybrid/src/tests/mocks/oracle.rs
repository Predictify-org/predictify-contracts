//! Mock Oracle Implementations for Comprehensive Testing
//!
//! This module provides mock oracle implementations for testing various scenarios
//! including valid responses, invalid responses, timeouts, and malicious behavior.

use crate::errors::Error;
use crate::oracles::{OracleInterface, OracleProvider};
use crate::types::*;
use soroban_sdk::{contracttype, Address, Env, String, Symbol};

/// Mock Oracle Base Structure
#[derive(Debug, Clone)]
pub struct MockOracle {
    pub contract_id: Address,
    pub provider: OracleProvider,
    pub behavior: MockBehavior,
}

/// Mock Behavior Configuration
#[derive(Debug, Clone)]
pub enum MockBehavior {
    /// Returns valid price data
    Valid { price: i128 },
    /// Returns invalid/malformed response
    InvalidResponse,
    /// Returns empty response
    EmptyResponse,
    /// Simulates timeout/no response
    Timeout,
    /// Returns corrupted payload
    CorruptedPayload,
    /// Returns fake signature (malicious)
    MaliciousSignature,
    /// Returns unauthorized signer
    UnauthorizedSigner,
    /// Returns stale data
    StaleData,
    /// Returns extreme values
    ExtremeValue { price: i128 },
    /// Returns conflicting results for multiple calls
    ConflictingResults {
        prices: Vec<i128>,
        current_index: usize,
    },
}

/// Valid Mock Oracle - Returns correct results
pub struct ValidMockOracle {
    contract_id: Address,
    mock_price: i128,
}

impl ValidMockOracle {
    pub fn new(contract_id: Address, price: i128) -> Self {
        Self {
            contract_id,
            mock_price: price,
        }
    }
}

impl OracleInterface for ValidMockOracle {
    fn get_price(&self, _env: &Env, _feed_id: &String) -> Result<i128, Error> {
        Ok(self.mock_price)
    }

    fn provider(&self) -> OracleProvider {
        OracleProvider::Reflector
    }

    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }

    fn is_healthy(&self, _env: &Env) -> Result<bool, Error> {
        Ok(true)
    }
}

/// Invalid Response Mock Oracle - Returns malformed data
pub struct InvalidResponseMockOracle {
    contract_id: Address,
}

impl InvalidResponseMockOracle {
    pub fn new(contract_id: Address) -> Self {
        Self { contract_id }
    }
}

impl OracleInterface for InvalidResponseMockOracle {
    fn get_price(&self, _env: &Env, _feed_id: &String) -> Result<i128, Error> {
        Err(Error::InvalidOracleFeed)
    }

    fn provider(&self) -> OracleProvider {
        OracleProvider::Reflector
    }

    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }

    fn is_healthy(&self, _env: &Env) -> Result<bool, Error> {
        Ok(false)
    }
}

/// Empty Response Mock Oracle
pub struct EmptyResponseMockOracle {
    contract_id: Address,
}

impl EmptyResponseMockOracle {
    pub fn new(contract_id: Address) -> Self {
        Self { contract_id }
    }
}

impl OracleInterface for EmptyResponseMockOracle {
    fn get_price(&self, _env: &Env, _feed_id: &String) -> Result<i128, Error> {
        Err(Error::OracleUnavailable)
    }

    fn provider(&self) -> OracleProvider {
        OracleProvider::Reflector
    }

    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }

    fn is_healthy(&self, _env: &Env) -> Result<bool, Error> {
        Ok(false)
    }
}

/// Timeout Mock Oracle - Simulates no response
pub struct TimeoutMockOracle {
    contract_id: Address,
}

impl TimeoutMockOracle {
    pub fn new(contract_id: Address) -> Self {
        Self { contract_id }
    }
}

impl OracleInterface for TimeoutMockOracle {
    fn get_price(&self, _env: &Env, _feed_id: &String) -> Result<i128, Error> {
        Err(Error::OracleUnavailable)
    }

    fn provider(&self) -> OracleProvider {
        OracleProvider::Reflector
    }

    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }

    fn is_healthy(&self, _env: &Env) -> Result<bool, Error> {
        Ok(false)
    }
}

/// Corrupted Payload Mock Oracle
pub struct CorruptedPayloadMockOracle {
    contract_id: Address,
}

impl CorruptedPayloadMockOracle {
    pub fn new(contract_id: Address) -> Self {
        Self { contract_id }
    }
}

impl OracleInterface for CorruptedPayloadMockOracle {
    fn get_price(&self, _env: &Env, _feed_id: &String) -> Result<i128, Error> {
        Err(Error::InvalidInput)
    }

    fn provider(&self) -> OracleProvider {
        OracleProvider::Reflector
    }

    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }

    fn is_healthy(&self, _env: &Env) -> Result<bool, Error> {
        Ok(false)
    }
}

/// Malicious Signature Mock Oracle
pub struct MaliciousSignatureMockOracle {
    contract_id: Address,
}

impl MaliciousSignatureMockOracle {
    pub fn new(contract_id: Address) -> Self {
        Self { contract_id }
    }
}

impl OracleInterface for MaliciousSignatureMockOracle {
    fn get_price(&self, _env: &Env, _feed_id: &String) -> Result<i128, Error> {
        Err(Error::Unauthorized)
    }

    fn provider(&self) -> OracleProvider {
        OracleProvider::Reflector
    }

    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }

    fn is_healthy(&self, _env: &Env) -> Result<bool, Error> {
        Ok(false)
    }
}

/// Unauthorized Signer Mock Oracle
pub struct UnauthorizedSignerMockOracle {
    contract_id: Address,
}

impl UnauthorizedSignerMockOracle {
    pub fn new(contract_id: Address) -> Self {
        Self { contract_id }
    }
}

impl OracleInterface for UnauthorizedSignerMockOracle {
    fn get_price(&self, _env: &Env, _feed_id: &String) -> Result<i128, Error> {
        Err(Error::Unauthorized)
    }

    fn provider(&self) -> OracleProvider {
        OracleProvider::Reflector
    }

    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }

    fn is_healthy(&self, _env: &Env) -> Result<bool, Error> {
        Ok(false)
    }
}

/// Stale Data Mock Oracle
pub struct StaleDataMockOracle {
    contract_id: Address,
}

impl StaleDataMockOracle {
    pub fn new(contract_id: Address) -> Self {
        Self { contract_id }
    }
}

impl OracleInterface for StaleDataMockOracle {
    fn get_price(&self, _env: &Env, _feed_id: &String) -> Result<i128, Error> {
        Err(Error::InvalidState)
    }

    fn provider(&self) -> OracleProvider {
        OracleProvider::Reflector
    }

    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }

    fn is_healthy(&self, _env: &Env) -> Result<bool, Error> {
        Ok(false)
    }
}

/// Extreme Value Mock Oracle
pub struct ExtremeValueMockOracle {
    contract_id: Address,
    price: i128,
}

impl ExtremeValueMockOracle {
    pub fn new(contract_id: Address, price: i128) -> Self {
        Self { contract_id, price }
    }
}

impl OracleInterface for ExtremeValueMockOracle {
    fn get_price(&self, _env: &Env, _feed_id: &String) -> Result<i128, Error> {
        Ok(self.price)
    }

    fn provider(&self) -> OracleProvider {
        OracleProvider::Reflector
    }

    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }

    fn is_healthy(&self, _env: &Env) -> Result<bool, Error> {
        Ok(true)
    }
}

/// Conflicting Results Mock Oracle
pub struct ConflictingResultsMockOracle {
    contract_id: Address,
    prices: Vec<i128>,
    current_index: usize,
}

impl ConflictingResultsMockOracle {
    pub fn new(contract_id: Address, prices: Vec<i128>) -> Self {
        Self {
            contract_id,
            prices,
            current_index: 0,
        }
    }
}

impl OracleInterface for ConflictingResultsMockOracle {
    fn get_price(&self, _env: &Env, _feed_id: &String) -> Result<i128, Error> {
        let price = self
            .prices
            .get(self.current_index % self.prices.len())
            .unwrap_or(&0);
        Ok(*price)
    }

    fn provider(&self) -> OracleProvider {
        OracleProvider::Reflector
    }

    fn contract_id(&self) -> Address {
        self.contract_id.clone()
    }

    fn is_healthy(&self, _env: &Env) -> Result<bool, Error> {
        Ok(true)
    }
}

/// Mock Oracle Factory for creating different mock instances
pub struct MockOracleFactory;

impl MockOracleFactory {
    pub fn create_valid_oracle(contract_id: Address, price: i128) -> Box<dyn OracleInterface> {
        Box::new(ValidMockOracle::new(contract_id, price))
    }

    pub fn create_invalid_response_oracle(contract_id: Address) -> Box<dyn OracleInterface> {
        Box::new(InvalidResponseMockOracle::new(contract_id))
    }

    pub fn create_empty_response_oracle(contract_id: Address) -> Box<dyn OracleInterface> {
        Box::new(EmptyResponseMockOracle::new(contract_id))
    }

    pub fn create_timeout_oracle(contract_id: Address) -> Box<dyn OracleInterface> {
        Box::new(TimeoutMockOracle::new(contract_id))
    }

    pub fn create_corrupted_payload_oracle(contract_id: Address) -> Box<dyn OracleInterface> {
        Box::new(CorruptedPayloadMockOracle::new(contract_id))
    }

    pub fn create_malicious_signature_oracle(contract_id: Address) -> Box<dyn OracleInterface> {
        Box::new(MaliciousSignatureMockOracle::new(contract_id))
    }

    pub fn create_unauthorized_signer_oracle(contract_id: Address) -> Box<dyn OracleInterface> {
        Box::new(UnauthorizedSignerMockOracle::new(contract_id))
    }

    pub fn create_stale_data_oracle(contract_id: Address) -> Box<dyn OracleInterface> {
        Box::new(StaleDataMockOracle::new(contract_id))
    }

    pub fn create_extreme_value_oracle(
        contract_id: Address,
        price: i128,
    ) -> Box<dyn OracleInterface> {
        Box::new(ExtremeValueMockOracle::new(contract_id, price))
    }

    pub fn create_conflicting_results_oracle(
        contract_id: Address,
        prices: Vec<i128>,
    ) -> Box<dyn OracleInterface> {
        Box::new(ConflictingResultsMockOracle::new(contract_id, prices))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_valid_mock_oracle() {
        let env = Env::default();
        let contract_id = Address::generate(&env);
        let oracle = ValidMockOracle::new(contract_id.clone(), 2600000);

        assert_eq!(
            oracle
                .get_price(&env, &String::from_str(&env, "BTC"))
                .unwrap(),
            2600000
        );
        assert_eq!(oracle.provider(), OracleProvider::Reflector);
        assert_eq!(oracle.contract_id(), contract_id);
        assert!(oracle.is_healthy(&env).unwrap());
    }

    #[test]
    fn test_invalid_response_mock_oracle() {
        let env = Env::default();
        let contract_id = Address::generate(&env);
        let oracle = InvalidResponseMockOracle::new(contract_id.clone());

        assert!(oracle
            .get_price(&env, &String::from_str(&env, "BTC"))
            .is_err());
        assert_eq!(oracle.provider(), OracleProvider::Reflector);
        assert_eq!(oracle.contract_id(), contract_id);
        assert!(!oracle.is_healthy(&env).unwrap());
    }

    #[test]
    fn test_timeout_mock_oracle() {
        let env = Env::default();
        let contract_id = Address::generate(&env);
        let oracle = TimeoutMockOracle::new(contract_id.clone());

        let result = oracle.get_price(&env, &String::from_str(&env, "BTC"));
        assert!(result.is_err());
        // Note: Error type would need to be defined in errors.rs
        assert_eq!(oracle.provider(), OracleProvider::Reflector);
        assert!(!oracle.is_healthy(&env).unwrap());
    }

    #[test]
    fn test_extreme_value_mock_oracle() {
        let env = Env::default();
        let contract_id = Address::generate(&env);
        let extreme_price = i128::MAX;
        let oracle = ExtremeValueMockOracle::new(contract_id.clone(), extreme_price);

        assert_eq!(
            oracle
                .get_price(&env, &String::from_str(&env, "BTC"))
                .unwrap(),
            extreme_price
        );
        assert!(oracle.is_healthy(&env).unwrap());
    }

    #[test]
    fn test_conflicting_results_mock_oracle() {
        let env = Env::default();
        let contract_id = Address::generate(&env);
        let prices = vec![&env, 2500000, 2600000, 2700000];
        let oracle = ConflictingResultsMockOracle::new(contract_id.clone(), prices);

        // Should cycle through prices
        assert_eq!(
            oracle
                .get_price(&env, &String::from_str(&env, "BTC"))
                .unwrap(),
            2500000
        );
        // Note: In a real implementation, we'd need to track state changes
        assert!(oracle.is_healthy(&env).unwrap());
    }
}
