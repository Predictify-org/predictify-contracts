#![allow(dead_code)]

use crate::config::{ORACLE_HEALTH_DEGRADED_THRESHOLD, ORACLE_HEALTH_RECOVERY_THRESHOLD};
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
    consecutive_good: u32,
    last_reason: String,
    updated_at: u64,
}

fn degradation_key(oracle: &OracleProvider) -> DegradationStorageKey {
    DegradationStorageKey::OracleHealth(oracle.clone())
}

fn load_degradation_state(env: &Env, oracle: &OracleProvider) -> Option<OracleDegradationState> {
    env.storage().persistent().get(&degradation_key(oracle))
}

/// Hysteresis-based oracle health recorder.
///
/// Instead of flipping state on a single sample, we require `DEGRADED_THRESHOLD`
/// consecutive bad samples to enter Degraded and `RECOVERY_THRESHOLD` consecutive
/// good samples to return to Working.  An `OracleHealthStatusEvent` is emitted
/// **only** when the state actually transitions.
fn record_oracle_health(
    env: &Env,
    oracle: &OracleProvider,
    sample: OracleHealth,
    reason: &String,
) {
    let previous = load_degradation_state(env, oracle);
    let prev_health = previous
        .as_ref()
        .map(|s| s.health.clone())
        .unwrap_or(OracleHealth::Working);
    let prev_failures = previous
        .as_ref()
        .map(|s| s.consecutive_failures)
        .unwrap_or(0);
    let prev_good = previous.as_ref().map(|s| s.consecutive_good).unwrap_or(0);
    let (new_health, new_failures, new_good, changed) = match sample {
        // --- good sample ---
        OracleHealth::Working => {
            let good = prev_good.saturating_add(1);
            let failures = 0;
            if prev_health == OracleHealth::Working {
                // Already healthy — just update counters.
                (OracleHealth::Working, failures, good, false)
            } else if good >= ORACLE_HEALTH_RECOVERY_THRESHOLD {
                // Enough consecutive good samples to recover.
                (OracleHealth::Working, failures, good, true)
            } else {
                // Still below threshold — stay in current state.
                (prev_health.clone(), failures, good, false)
            }
        }
        // --- bad sample ---
        OracleHealth::Degraded | OracleHealth::Broken => {
            let failures = prev_failures.saturating_add(1);
            let good = 0;
            match prev_health {
                OracleHealth::Degraded | OracleHealth::Broken => {
                    // Already degraded / broken — stay there.
                    (prev_health.clone(), failures, good, false)
                }
                OracleHealth::Working => {
                    if failures >= ORACLE_HEALTH_DEGRADED_THRESHOLD {
                        // Enough consecutive failures to mark degraded.
                        (OracleHealth::Degraded, failures, good, true)
                    } else {
                        // Below threshold — still Working on paper.
                        (OracleHealth::Working, failures, good, false)
                    }
                }
            }
        }
    };

    // Emit event only on actual state change.
    if changed {
        let provider_str = String::from_str(env, oracle.as_str());
        // Generate a valid placeholder address since this code path only has
        // an OracleProvider enum (not a real contract address). The `provider`
        // field in the event carries the oracle identity.  Callers that have
        // the real address (e.g., OracleBackup methods) can emit separately.
        EventEmitter::emit_oracle_health_status(
            env,
            &Address::generate(env),
            &provider_str,
            prev_health == OracleHealth::Working,
            new_health == OracleHealth::Working,
            new_failures,
        );
    }

    let state = OracleDegradationState {
        health: new_health,
        consecutive_failures: new_failures,
        consecutive_good: new_good,
        last_reason: reason.clone(),
        updated_at: env.ledger().timestamp(),
    };
    env.storage()
        .persistent()
        .set(&degradation_key(oracle), &state);
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

    // Probe the oracle.
    let working = backup.is_working(env, oracle_address).unwrap_or(false);

    // Record the sample through the hysteresis gate.
    let sample = if working {
        OracleHealth::Working
    } else {
        OracleHealth::Degraded
    };
    let reason = if working {
        String::from_str(env, "Oracle probe succeeded")
    } else {
        String::from_str(env, "Oracle probe failed")
    };
    record_oracle_health(env, &oracle, sample, &reason);

    // Return the *stored* health (after hysteresis), not the raw sample.
    load_degradation_state(env, &oracle)
        .map(|s| s.health)
        .unwrap_or(OracleHealth::Working)
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

    // ------------------------------------------------------------------
    //  Legacy / existing behaviour tests (adapted for hysteresis)
    // ------------------------------------------------------------------

    #[test]
    fn can_create_backup() {
        let backup = OracleBackup::new(OracleProvider::reflector(), OracleProvider::pyth());
        assert_eq!(backup.primary, OracleProvider::reflector());
        assert_eq!(backup.backup, OracleProvider::pyth());
    }

    #[test]
    fn can_check_health() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let addr = Address::generate(&env);

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
        let contract_id = env.register(crate::PredictifyHybrid, ());

        let backup = OracleBackup::new(OracleProvider::pyth(), OracleProvider::dia());
        let oracle_address = Address::generate(&env);

        env.as_contract(&contract_id, || {
            let result = backup.is_working(&env, &oracle_address);
            assert!(result.is_err());
        });

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

    // ------------------------------------------------------------------
    //  Hysteresis tests
    // ------------------------------------------------------------------

    /// Helper: directly record a health sample (bypassing probe).
    fn record_sample(env: &Env, oracle: &OracleProvider, working: bool) {
        let sample = if working {
            OracleHealth::Working
        } else {
            OracleHealth::Degraded
        };
        let reason = String::from_str(env, if working { "ok" } else { "fail" });
        record_oracle_health(env, oracle, sample, &reason);
    }

    /// Helper: load the stored health for an oracle.
    fn stored_health(env: &Env, oracle: &OracleProvider) -> OracleHealth {
        load_degradation_state(env, oracle)
            .map(|s| s.health)
            .unwrap_or(OracleHealth::Working)
    }

    #[test]
    fn hysteresis_single_bad_sample_does_not_transition() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let oracle = OracleProvider::reflector();

        env.as_contract(&contract_id, || {
            // Start from default (Working).
            assert_eq!(stored_health(&env, &oracle), OracleHealth::Working);

            // 1 bad sample — should NOT flip to Degraded.
            record_sample(&env, &oracle, false);
            assert_eq!(stored_health(&env, &oracle), OracleHealth::Working);

            // 2 bad samples — still below threshold.
            record_sample(&env, &oracle, false);
            assert_eq!(stored_health(&env, &oracle), OracleHealth::Working);
        });
    }

    #[test]
    fn hysteresis_three_consecutive_bad_triggers_degraded() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let oracle = OracleProvider::reflector();

        env.as_contract(&contract_id, || {
            // 3 consecutive bad samples = transition to Degraded.
            record_sample(&env, &oracle, false);
            record_sample(&env, &oracle, false);
            record_sample(&env, &oracle, false);

            assert_eq!(stored_health(&env, &oracle), OracleHealth::Degraded);

            // Verify consecutive_failures counter.
            let state = load_degradation_state(&env, &oracle).unwrap();
            assert_eq!(state.consecutive_failures, 3);
        });
    }

    #[test]
    fn hysteresis_good_sample_resets_bad_counter() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let oracle = OracleProvider::reflector();

        env.as_contract(&contract_id, || {
            // 2 bad, then 1 good — should reset counter and stay Working.
            record_sample(&env, &oracle, false);
            record_sample(&env, &oracle, false);
            record_sample(&env, &oracle, true);

            assert_eq!(stored_health(&env, &oracle), OracleHealth::Working);
            let state = load_degradation_state(&env, &oracle).unwrap();
            assert_eq!(state.consecutive_failures, 0);
            assert_eq!(state.consecutive_good, 1);
        });
    }

    #[test]
    fn hysteresis_recovery_requires_three_consecutive_good() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let oracle = OracleProvider::reflector();

        env.as_contract(&contract_id, || {
            // Push to Degraded.
            record_sample(&env, &oracle, false);
            record_sample(&env, &oracle, false);
            record_sample(&env, &oracle, false);
            assert_eq!(stored_health(&env, &oracle), OracleHealth::Degraded);

            // 1 good — still Degraded.
            record_sample(&env, &oracle, true);
            assert_eq!(stored_health(&env, &oracle), OracleHealth::Degraded);

            // 2 good — still Degraded.
            record_sample(&env, &oracle, true);
            assert_eq!(stored_health(&env, &oracle), OracleHealth::Degraded);

            // 3 good — recovery!
            record_sample(&env, &oracle, true);
            assert_eq!(stored_health(&env, &oracle), OracleHealth::Working);
        });
    }

    #[test]
    fn hysteresis_event_emitted_only_on_state_transition() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let oracle = OracleProvider::reflector();

        env.as_contract(&contract_id, || {
            // Bad samples #1 and #2 — no transition, so no "orc_hlth" event.
            record_sample(&env, &oracle, false);
            record_sample(&env, &oracle, false);

            // No state change yet — confirm via state assertion.
            assert_eq!(stored_health(&env, &oracle), OracleHealth::Working);
        });

        // Check that the health-status event ("orc_hlth") was NOT emitted
        // (no transition happened). The oracle_degradation event is still
        // emitted by is_working, so we don't check total event count.
        let events = env.events().all();
        let health_events: soroban_sdk::Vec<_> = events
            .events()
            .iter()
            .filter(|e| e.0 == (soroban_sdk::symbol_short!("orc_hlth"),))
            .collect();
        assert!(
            health_events.is_empty(),
            "No OracleHealthStatusEvent should be emitted before transition"
        );

        // Now trigger the transition with a 3rd bad sample.
        env.as_contract(&contract_id, || {
            record_sample(&env, &oracle, false);
            assert_eq!(stored_health(&env, &oracle), OracleHealth::Degraded);
        });

        let events = env.events().all();
        let health_events: soroban_sdk::Vec<_> = events
            .events()
            .iter()
            .filter(|e| e.0 == (soroban_sdk::symbol_short!("orc_hlth"),))
            .collect();
        assert!(
            health_events.len() >= 1,
            "OracleHealthStatusEvent should be emitted on transition to Degraded"
        );
    }

    #[test]
    fn hysteresis_timeout_requires_three_calls_to_degrade() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let oracle = OracleProvider::reflector();
        let oracle_address = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // 1st timeout — should NOT flip immediately (needs 3).
            handle_oracle_timeout(oracle.clone(), 61, &env);
            let health = get_degradation_status(oracle.clone(), &env, &oracle_address);
            assert_eq!(health, OracleHealth::Working);

            // 2nd timeout — still not enough.
            handle_oracle_timeout(oracle.clone(), 61, &env);
            let health = get_degradation_status(oracle.clone(), &env, &oracle_address);
            assert_eq!(health, OracleHealth::Working);

            // 3rd timeout — now it degrades.
            handle_oracle_timeout(oracle.clone(), 61, &env);
            let health = get_degradation_status(oracle.clone(), &env, &oracle_address);
            assert_eq!(health, OracleHealth::Degraded);
        });
    }

    #[test]
    fn hysteresis_no_event_on_noop_transition() {
        // When already Degraded, another bad sample should NOT re-emit the event.
        // We verify by checking that the health status stays Degraded and the
        // consecutive_failures counter increments.
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let oracle = OracleProvider::reflector();

        env.as_contract(&contract_id, || {
            // Push to Degraded.
            record_sample(&env, &oracle, false);
            record_sample(&env, &oracle, false);
            record_sample(&env, &oracle, false);
            assert_eq!(stored_health(&env, &oracle), OracleHealth::Degraded);

            // Additional bad sample — stays Degraded, failure counter increments.
            record_sample(&env, &oracle, false);
            assert_eq!(stored_health(&env, &oracle), OracleHealth::Degraded);
            let state = load_degradation_state(&env, &oracle).unwrap();
            assert_eq!(state.consecutive_failures, 4);
        });
    }

    #[test]
    fn hysteresis_get_degradation_status_uses_hysteresis() {
        // `get_degradation_status` calls `monitor_oracle_health` which probes
        // the oracle. Since the oracle is unregistered, probes fail, but
        // the hysteresis gate should require 3 consecutive failures before
        // returning Degraded.
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let oracle = OracleProvider::reflector();
        let oracle_address = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // First probe — should still return Working (hysteresis gate).
            let health = get_degradation_status(oracle.clone(), &env, &oracle_address);
            assert_eq!(health, OracleHealth::Working);

            // Second probe — still Working.
            let health = get_degradation_status(oracle.clone(), &env, &oracle_address);
            assert_eq!(health, OracleHealth::Working);

            // Third probe — now Degraded.
            let health = get_degradation_status(oracle.clone(), &env, &oracle_address);
            assert_eq!(health, OracleHealth::Degraded);
        });
    }
}

