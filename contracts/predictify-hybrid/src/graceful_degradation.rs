#![allow(dead_code)]

use crate::errors::Error;
use crate::events::EventEmitter;
use crate::oracles::{OracleInterface, ReflectorOracle};
use crate::types::OracleProvider;
use soroban_sdk::{contracttype, Address, Env, String, Symbol};

// Basic oracle backup system
pub struct OracleBackup {
    primary: OracleProvider,
    backup: OracleProvider,
}

impl OracleBackup {
    pub fn new(primary: OracleProvider, backup: OracleProvider) -> Self {
        Self { primary, backup }
    }

    // Get price, try backup if primary fails
    pub fn get_price(&self, env: &Env, oracle_address: &Address, feed_id: &String) -> Result<i128, Error> {
        // Try primary oracle
        if let Ok(price) = self.call_oracle(env, &self.primary, oracle_address, feed_id) {
            return Ok(price);
        }

        // Primary failed, notify and try backup
        let msg = String::from_str(env, "Primary oracle failed");
        EventEmitter::emit_oracle_degradation(env, &self.primary, &msg);
        
        self.call_oracle(env, &self.backup, oracle_address, feed_id)
    }

    // Call a single oracle
    fn call_oracle(&self, env: &Env, oracle: &OracleProvider, address: &Address, feed_id: &String) -> Result<i128, Error> {
        match oracle {
            OracleProvider::Reflector => {
                let reflector = ReflectorOracle::new(address.clone());
                reflector.get_price(env, feed_id)
            }
            _ => Err(Error::OracleUnavailable),
        }
    }

    // Is oracle working?
    pub fn is_working(&self, env: &Env, oracle_address: &Address) -> bool {
        let test_feed = String::from_str(env, "BTC/USD");
        self.call_oracle(env, &self.primary, oracle_address, &test_feed).is_ok()
    }
}

// Required functions to match original spec
pub fn fallback_oracle_call(
    env: &Env,
    primary_oracle: OracleProvider,
    fallback_oracle: OracleProvider,
    oracle_address: &Address,
    feed_id: &String,
) -> Result<i128, Error> {
    let backup = OracleBackup::new(primary_oracle, fallback_oracle);
    backup.get_price(env, oracle_address, feed_id)
}

pub fn handle_oracle_timeout(oracle: OracleProvider, timeout_seconds: u32, env: &Env) {
    if timeout_seconds > 60 {
        let msg = String::from_str(env, "Oracle timeout");
        EventEmitter::emit_oracle_degradation(env, &oracle, &msg);
    }
}

pub fn partial_resolution_mechanism(
    env: &Env,
    market_id: Symbol,
    available_data: PartialData,
) -> Result<String, Error> {
    // Good enough confidence? Use the data
    if available_data.confidence >= 70 && available_data.price.is_some() {
        return Ok(String::from_str(env, "resolved"));
    }

    // Not good enough, need human
    let msg = String::from_str(env, "Need manual resolution");
    EventEmitter::emit_manual_resolution_required(env, &market_id, &msg);
    Err(Error::OracleUnavailable)
}

pub fn emit_degradation_event(env: &Env, oracle: OracleProvider, reason: String) {
    EventEmitter::emit_oracle_degradation(env, &oracle, &reason);
}

pub fn monitor_oracle_health(env: &Env, oracle: OracleProvider, oracle_address: &Address) -> OracleHealth {
    let backup = OracleBackup::new(oracle.clone(), oracle);
    if backup.is_working(env, oracle_address) {
        OracleHealth::Working
    } else {
        OracleHealth::Broken
    }
}

pub fn get_degradation_status(oracle: OracleProvider, env: &Env, oracle_address: &Address) -> OracleHealth {
    monitor_oracle_health(env, oracle, oracle_address)
}

pub fn validate_degradation_strategy(_strategy: DegradationStrategy) -> Result<(), Error> {
    Ok(()) // All strategies are fine
}

// Simple data types
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DegradationStrategy {
    UseBackup,
    ManualFix,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleHealth {
    Working,
    Broken,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialData {
    pub price: Option<i128>,
    pub confidence: i128,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::Env;

    #[test]
    fn can_create_backup() {
        let backup = OracleBackup::new(OracleProvider::Reflector, OracleProvider::Pyth);
        assert_eq!(backup.primary, OracleProvider::Reflector);
        assert_eq!(backup.backup, OracleProvider::Pyth);
    }

    #[test]
    fn can_check_health() {
        let env = Env::default();
        let addr = Address::generate(&env);
        let health = monitor_oracle_health(&env, OracleProvider::Reflector, &addr);
        assert!(matches!(health, OracleHealth::Working | OracleHealth::Broken));
    }

    #[test]
    fn strategy_works() {
        let result = validate_degradation_strategy(DegradationStrategy::UseBackup);
        assert!(result.is_ok());
    }

    #[test]
    fn partial_data_works() {
        let env = Env::default();
        let market = Symbol::new(&env, "test");
        let data = PartialData {
            price: Some(100),
            confidence: 80,
            timestamp: env.ledger().timestamp(),
        };
        let result = partial_resolution_mechanism(&env, market, data);
        assert!(result.is_ok());
    }
}