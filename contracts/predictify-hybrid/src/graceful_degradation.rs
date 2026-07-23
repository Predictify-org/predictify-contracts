#![allow(dead_code)]

use crate::err::Error;
use crate::events::EventEmitter;
// use crate::oracles::{OracleInterface, ReflectorOracle};
use crate::types::OracleProvider;
use soroban_sdk::{contracttype, Address, Env, String, Symbol};

const ORACLE_TIMEOUT_THRESHOLD_SECONDS: u32 = 60;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DegradationStorageKey {
    OracleHealth(OracleProvider),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
struct OracleDegradationState {
    health: OracleHealth,
    consecutive_failures: u32,
    last_reason: String,
    updated_at: u64,
}

fn degradation_key(oracle: &OracleProvider) -> DegradationStorageKey {
    DegradationStorageKey::OracleHealth(oracle.clone())
}

fn load_degradation_state(env: &Env, oracle: &OracleProvider) -> Option<OracleDegradationState> {
    env.storage().persistent().get(&degradation_key(oracle))
}

fn record_oracle_health(
    env: &Env,
    oracle: &OracleProvider,
    health: OracleHealth,
    reason: &String,
) {
    let consecutive_failures = match health {
        OracleHealth::Working => 0,
        OracleHealth::Degraded | OracleHealth::Broken => load_degradation_state(env, oracle)
            .map(|state| state.consecutive_failures.saturating_add(1))
            .unwrap_or(1),
    };

    let state = OracleDegradationState {
        health,
        consecutive_failures,
        last_reason: reason.clone(),
        updated_at: env.ledger().timestamp(),
    };
    env.storage().persistent().set(&degradation_key(oracle), &state);
}

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
    /// Retrieves the price from the primary oracle, falling back to the backup if necessary.
    ///
    /// Emits degradation events for both primary and backup failures to ensure
    /// operators have complete visibility into the oracle health lifecycle.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `oracle_address` - The contract address of the oracle
    /// * `feed_id` - The asset feed identifier
    pub fn get_price(
        &self,
        env: &Env,
        oracle_address: &Address,
        feed_id: &String,
    ) -> Result<i128, Error> {
        // Try primary oracle
        if let Ok(price) = self.call_oracle(env, &self.primary, oracle_address, feed_id) {
            let ok_msg = String::from_str(env, "Oracle healthy");
            record_oracle_health(env, &self.primary, OracleHealth::Working, &ok_msg);
            return Ok(price);
        }

        // Primary failed, notify and try backup
        let msg = String::from_str(env, "Primary oracle failed");
        EventEmitter::emit_oracle_degradation(env, &self.primary, &msg);

        let backup_result = self.call_oracle(env, &self.backup, oracle_address, feed_id);
        if backup_result.is_err() {
            let backup_msg = String::from_str(env, "Backup oracle failed");
            EventEmitter::emit_oracle_degradation(env, &self.backup, &backup_msg);
            return Err(Error::FallbackOracleUnavailable);
        }
        backup_result
    }

    // Call a single oracle
    fn call_oracle(
        &self,
        env: &Env,
        oracle: &OracleProvider,
        address: &Address,
        feed_id: &String,
    ) -> Result<i128, Error> {
        match oracle {
            oracle if oracle == &OracleProvider::reflector() => {
                // Temporarily disabled due to oracles module being disabled
                // let reflector = ReflectorOracle::new(address.clone());
                // reflector.get_price(env, feed_id)
                Err(Error::OracleUnavailable)
            }
            _ => Err(Error::OracleUnavailable),
        }
    }

    // Is oracle working?
    /// Checks if the primary oracle is currently operational.
    ///
    /// Rather than failing silently, this queries the oracle and emits an
    /// `OracleDegradationEvent` if the health check fails, providing operators
    /// with an immediate on-chain signal.
    ///
    /// # Returns
    /// * `Ok(true)` if the oracle responds successfully
    /// * `Err(Error)` if the oracle is unreachable or fails, surfacing the exact error
    pub fn is_working(&self, env: &Env, oracle_address: &Address) -> Result<bool, Error> {
        let test_feed = String::from_str(env, "BTC/USD");
        match self.call_oracle(env, &self.primary, oracle_address, &test_feed) {
            Ok(_) => Ok(true),
            Err(e) => {
                let msg =
                    String::from_str(env, "Oracle health check failed during is_working query");
                EventEmitter::emit_oracle_degradation(env, &self.primary, &msg);
                Err(e)
            }
        }
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
    if timeout_seconds > ORACLE_TIMEOUT_THRESHOLD_SECONDS {
        let msg = String::from_str(env, "Oracle timeout");
        record_oracle_health(env, &oracle, OracleHealth::Degraded, &msg);
        emit_degradation_event(env, oracle, msg);
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

pub fn monitor_oracle_health(
    env: &Env,
    oracle: OracleProvider,
    oracle_address: &Address,
) -> OracleHealth {
    let backup = OracleBackup::new(oracle.clone(), oracle);
    let stored_health = load_degradation_state(env, &backup.primary).map(|state| state.health);

    // Probe live first so a recovered oracle clears any prior degraded state.
    if backup.is_working(env, oracle_address).unwrap_or(false) {
        OracleHealth::Working
    } else {
        stored_health.unwrap_or(OracleHealth::Broken)
    }
}

pub fn get_degradation_status(
    oracle: OracleProvider,
    env: &Env,
    oracle_address: &Address,
) -> OracleHealth {
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
    Degraded,
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
    use soroban_sdk::testutils::Events;
    use soroban_sdk::Env;

    #[test]
    fn can_create_backup() {
        let backup = OracleBackup::new(OracleProvider::reflector(), OracleProvider::pyth());
        assert_eq!(backup.primary, OracleProvider::reflector());
        assert_eq!(backup.backup, OracleProvider::pyth());
    }

    #[test]
    fn can_check_health() {
        let env = Env::default();
        //1. register the contract so we have a context
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let addr = Address::generate(&env);

        //2. wrap the execution in the contract context
        env.as_contract(&contract_id, || {
            let health = monitor_oracle_health(&env, OracleProvider::reflector(), &addr);
            assert!(matches!(
                health,
                OracleHealth::Working | OracleHealth::Broken
            ));
        });
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

    #[test]
    fn test_is_working_propagates_error_and_emits_event() {
        let env = Env::default();
        // 1. Register the contract
        let contract_id = env.register(crate::PredictifyHybrid, ());

        let backup = OracleBackup::new(OracleProvider::pyth(), OracleProvider::dia());
        let oracle_address = Address::generate(&env);

        // 2. Wrap the execution in the contract context
        env.as_contract(&contract_id, || {
            let result = backup.is_working(&env, &oracle_address);
            assert!(result.is_err()); // No longer fails silently
        });

        // 3. Verify event emission
        let events = env.events().all();
        assert!(
            events.events().len() > 0,
            "Expected oracle degradation event to be emitted"
        );
    }

    #[test]
    fn test_oracle_fallback_both_oracles_down_returns_typed_error() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let backup = OracleBackup::new(OracleProvider::reflector(), OracleProvider::pyth());
        let oracle_address = Address::generate(&env);
        let feed_id = String::from_str(&env, "BTC/USD");

        env.as_contract(&contract_id, || {
            let result = backup.get_price(&env, &oracle_address, &feed_id);
            assert_eq!(result, Err(Error::FallbackOracleUnavailable));
        });

        let events = env.events().all();
        assert!(
            events.events().len() >= 2,
            "Expected degradation events for primary and backup oracle failures"
        );
    }

    #[test]
    fn test_oracle_fallback_timeout_marks_oracle_degraded() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let oracle = OracleProvider::reflector();
        let oracle_address = Address::generate(&env);

        env.as_contract(&contract_id, || {
            handle_oracle_timeout(oracle.clone(), 61, &env);
            let health = get_degradation_status(oracle.clone(), &env, &oracle_address);
            assert_eq!(health, OracleHealth::Degraded);
        });

        let events = env.events().all();
        assert!(
            events.events().len() > 0,
            "Expected timeout handling to emit a degradation event"
        );
    }
}
