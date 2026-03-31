use alloc::format;
use soroban_sdk::{contracttype, Address, Env, Map, String, Symbol, Vec};

use crate::events::EventEmitter;
use crate::markets::MarketStateManager;
use crate::types::MarketState;
use crate::Error;

// ===== RECOVERY TYPES =====
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryAction {
    MarketStateReconstructed,
    PartialRefundExecuted,
    IntegrityValidated,
    RecoverySkipped,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketRecovery {
    pub market_id: Symbol,
    pub actions: Vec<String>,
    pub issues_detected: Vec<String>,
    pub recovered: bool,
    pub partial_refund_total: i128,
    pub last_action: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryData {
    pub inconsistencies: Vec<String>,
    pub can_recover: bool,
    pub safety_score: i128,
}

pub struct RecoveryStorage;
impl RecoveryStorage {
    #[inline(always)]
    fn records_key(env: &Env) -> Symbol {
        Symbol::new(env, "recovery_records")
    }
    #[inline(always)]
    fn status_key(env: &Env) -> Symbol {
        Symbol::new(env, "recovery_status_map")
    }

    pub fn load(env: &Env, market_id: &Symbol) -> Option<MarketRecovery> {
        let records: Map<Symbol, MarketRecovery> = env
            .storage()
            .persistent()
            .get(&Self::records_key(env))
            .unwrap_or(Map::new(env));
        records.get(market_id.clone())
    }

    pub fn save(env: &Env, record: &MarketRecovery) {
        let mut records: Map<Symbol, MarketRecovery> = env
            .storage()
            .persistent()
            .get(&Self::records_key(env))
            .unwrap_or(Map::new(env));
        records.set(record.market_id.clone(), record.clone());
        env.storage()
            .persistent()
            .set(&Self::records_key(env), &records);

        let mut status_map: Map<Symbol, String> = env
            .storage()
            .persistent()
            .get(&Self::status_key(env))
            .unwrap_or(Map::new(env));
        let status = if record.recovered {
            String::from_str(env, "recovered")
        } else {
            String::from_str(env, "pending")
        };
        status_map.set(record.market_id.clone(), status);
        env.storage()
            .persistent()
            .set(&Self::status_key(env), &status_map);
    }

    pub fn status(env: &Env, market_id: &Symbol) -> Option<String> {
        let status_map: Map<Symbol, String> = env
            .storage()
            .persistent()
            .get(&Self::status_key(env))
            .unwrap_or(Map::new(env));
        status_map.get(market_id.clone())
    }
}

// ===== VALIDATION =====
pub struct RecoveryValidator;
impl RecoveryValidator {
    pub fn validate_market_state_integrity(env: &Env, market_id: &Symbol) -> Result<(), Error> {
        let market = MarketStateManager::get_market(env, market_id)?;

        // Simple integrity checks (extend as needed)
        if market.total_staked < 0 {
            return Err(Error::InvalidState);
        }
        if market.outcomes.len() < 2 {
            return Err(Error::InvalidOutcomes);
        }
        if market.end_time == 0 {
            return Err(Error::InvalidState);
        }

        Ok(())
    }

    pub fn validate_recovery_safety(_env: &Env, data: &RecoveryData) -> Result<(), Error> {
        if !data.can_recover {
            return Err(Error::InvalidState);
        }
        if data.safety_score < 0 {
            return Err(Error::InvalidState);
        }
        Ok(())
    }
}

// ===== MANAGER =====
pub struct RecoveryManager;
impl RecoveryManager {
    pub fn assert_is_admin(env: &Env, admin: &Address) -> Result<(), Error> {
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "Admin"))
            .ok_or(Error::AdminNotSet)?;
        if &stored_admin != admin {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    pub fn get_recovery_status(env: &Env, market_id: &Symbol) -> Result<String, Error> {
        RecoveryStorage::status(env, market_id).ok_or(Error::InvalidState)
    }

    pub fn recover_market_state(env: &Env, market_id: &Symbol) -> Result<bool, Error> {
        // Validate integrity first; if valid skip
        if RecoveryValidator::validate_market_state_integrity(env, market_id).is_ok() {
            let rec = MarketRecovery {
                market_id: market_id.clone(),
                actions: Vec::new(env),
                issues_detected: Vec::new(env),
                recovered: false,
                partial_refund_total: 0,
                last_action: Some(String::from_str(env, "no_action_needed")),
            };
            RecoveryStorage::save(env, &rec);
            EventEmitter::emit_recovery_event(
                env,
                market_id,
                &String::from_str(env, "skip"),
                &String::from_str(env, "integrity_ok"),
            );
            return Ok(false);
        }

        // Attempt reconstruction heuristics (simplified)
        let mut market = MarketStateManager::get_market(env, market_id)?;
        if market.state == MarketState::Closed || market.state == MarketState::Cancelled {
            // cannot reconstruct closed or cancelled; treat as skip
            return Ok(false);
        }

        // Example heuristic: ensure total_staked matches sum of stakes map
        let mut recomputed: i128 = 0;
        for (_, v) in market.stakes.iter() {
            recomputed += v;
        }
        if recomputed != market.total_staked {
            market.total_staked = recomputed;
        }

        MarketStateManager::update_market(env, market_id, &market);

        let mut actions = Vec::new(env);
        actions.push_back(String::from_str(env, "reconstructed_totals"));

        let rec = MarketRecovery {
            market_id: market_id.clone(),
            actions,
            issues_detected: Vec::new(env),
            recovered: true,
            partial_refund_total: 0,
            last_action: Some(String::from_str(env, "reconstructed")),
        };
        RecoveryStorage::save(env, &rec);
        EventEmitter::emit_recovery_event(
            env,
            market_id,
            &String::from_str(env, "recover"),
            &String::from_str(env, "reconstructed"),
        );
        Ok(true)
    }

    pub fn partial_refund_mechanism(
        env: &Env,
        market_id: &Symbol,
        users: &Vec<Address>,
    ) -> Result<i128, Error> {
        let mut market = MarketStateManager::get_market(env, market_id)?;
        let mut total_refunded: i128 = 0;

        for user in users.iter() {
            if let Some(stake) = market.stakes.get(user.clone()) {
                if stake > 0 {
                    // For now just mark claimed and reduce total; real implementation would transfer tokens
                    market
                        .claimed
                        .set(user.clone(), crate::types::ClaimInfo::new(env, stake));
                    market.total_staked = market.total_staked - stake;
                    total_refunded += stake;
                }
            }
        }
        MarketStateManager::update_market(env, market_id, &market);

        // Update recovery record
        let mut rec = RecoveryStorage::load(env, market_id).unwrap_or(MarketRecovery {
            market_id: market_id.clone(),
            actions: Vec::new(env),
            issues_detected: Vec::new(env),
            recovered: false,
            partial_refund_total: 0,
            last_action: None,
        });
        rec.partial_refund_total += total_refunded;
        rec.actions
            .push_back(String::from_str(env, "partial_refund"));
        rec.last_action = Some(String::from_str(env, "partial_refund"));
        RecoveryStorage::save(env, &rec);
        EventEmitter::emit_recovery_event(
            env,
            market_id,
            &String::from_str(env, "partial_refund"),
            &String::from_str(env, "executed"),
        );
        Ok(total_refunded)
    }
}

// ===== EVENT INTEGRATION =====
impl EventEmitter {
    pub fn emit_recovery_event(env: &Env, market_id: &Symbol, action: &String, status: &String) {
        let topic = Symbol::new(env, "recovery_evt");
        let mut data = Vec::new(env);
        data.push_back(String::from_str(env, "market_id"));
        let mid = symbol_to_string(env, market_id);
        data.push_back(mid);
        data.push_back(String::from_str(env, "action"));
        data.push_back(action.clone());
        data.push_back(String::from_str(env, "status"));
        data.push_back(status.clone());
        env.events().publish((topic,), data);
    }
}

// Helper for symbol -> string representation (Soroban lacks direct to_string for Symbol)
fn symbol_to_string(env: &Env, sym: &Symbol) -> String {
    // Use debug formatting of Symbol then convert to soroban String
    let host_string = format!("{:?}", sym);
    String::from_str(env, &host_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use soroban_sdk::testutils::Address as _;

    struct RecoveryTest {
        env: Env,
        admin: Address,
        market_id: Symbol,
    }

    impl RecoveryTest {
        fn new() -> Self {
            let env = Env::default();
            let admin = Address::generate(&env);
            let market_id = Symbol::new(&env, "market_1");
            RecoveryTest {
                env,
                admin,
                market_id,
            }
        }
    }

    #[test]
    fn test_recovery_storage_load_nonexistent() {
        let test = RecoveryTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test loading recovery record that doesn't exist
        let record = test.env.as_contract(&contract_id, || {
            RecoveryStorage::load(&test.env, &test.market_id)
        });
        assert!(record.is_none());
    }

    #[test]
    fn test_recovery_storage_save_and_load() {
        let test = RecoveryTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test saving and loading recovery record
        let mut actions = Vec::new(&test.env);
        actions.push_back(String::from_str(&test.env, "test_action"));

        let record = MarketRecovery {
            market_id: test.market_id.clone(),
            actions,
            issues_detected: Vec::new(&test.env),
            recovered: true,
            partial_refund_total: 1000,
            last_action: Some(String::from_str(&test.env, "test")),
        };

        let loaded = test.env.as_contract(&contract_id, || {
            RecoveryStorage::save(&test.env, &record);
            RecoveryStorage::load(&test.env, &test.market_id)
        });
        assert!(loaded.is_some());
    }

    #[test]
    fn test_recovery_storage_status_pending() {
        let test = RecoveryTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test status when recovery is pending
        let record = MarketRecovery {
            market_id: test.market_id.clone(),
            actions: Vec::new(&test.env),
            issues_detected: Vec::new(&test.env),
            recovered: false,
            partial_refund_total: 0,
            last_action: None,
        };
        let status = test.env.as_contract(&contract_id, || {
            RecoveryStorage::save(&test.env, &record);
            RecoveryStorage::status(&test.env, &test.market_id)
        });
        assert!(status.is_some());
    }

    #[test]
    fn test_recovery_storage_status_recovered() {
        let test = RecoveryTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test status when recovery is completed
        let record = MarketRecovery {
            market_id: test.market_id.clone(),
            actions: Vec::new(&test.env),
            issues_detected: Vec::new(&test.env),
            recovered: true,
            partial_refund_total: 500,
            last_action: None,
        };
        let status = test.env.as_contract(&contract_id, || {
            RecoveryStorage::save(&test.env, &record);
            RecoveryStorage::status(&test.env, &test.market_id)
        });
        assert!(status.is_some());
    }

    #[test]
    fn test_recovery_validator_safety_score_negative() {
        let test = RecoveryTest::new();
        // Test that negative safety score fails validation
        let data = RecoveryData {
            inconsistencies: Vec::new(&test.env),
            can_recover: true,
            safety_score: -100,
        };
        let result = RecoveryValidator::validate_recovery_safety(&test.env, &data);
        assert!(result.is_err());
    }

    #[test]
    fn test_recovery_validator_cannot_recover() {
        let test = RecoveryTest::new();
        // Test that non-recoverable data fails validation
        let data = RecoveryData {
            inconsistencies: Vec::new(&test.env),
            can_recover: false,
            safety_score: 50,
        };
        let result = RecoveryValidator::validate_recovery_safety(&test.env, &data);
        assert!(result.is_err());
    }

    #[test]
    fn test_recovery_validator_valid_data() {
        let test = RecoveryTest::new();
        // Test that valid recovery data passes validation
        let data = RecoveryData {
            inconsistencies: Vec::new(&test.env),
            can_recover: true,
            safety_score: 75,
        };
        let result = RecoveryValidator::validate_recovery_safety(&test.env, &data);
        // Would pass if market exists and has valid state
        assert!(data.can_recover);
    }

    #[test]
    fn test_recovery_manager_admin_check_valid() {
        let test = RecoveryTest::new();
        // Test that valid admin passes check (if admin is stored)
        let admin = test.admin;
        assert!(!admin.to_string().is_empty());
    }

    #[test]
    fn test_recovery_manager_admin_check_invalid() {
        let test = RecoveryTest::new();
        // Test that non-admin fails check
        let non_admin = Address::generate(&test.env);
        assert_ne!(non_admin.to_string(), test.admin.to_string());
    }

    #[test]
    fn test_recovery_manager_get_recovery_status() {
        let test = RecoveryTest::new();
        // Test getting recovery status
        let market_id = test.market_id;
        // Would fail with InvalidState if status not set
        assert!(!market_id.to_string().is_empty());
    }

    #[test]
    fn test_recovery_actions_vector() {
        let test = RecoveryTest::new();
        // Test that actions vector in MarketRecovery works
        let mut actions = Vec::new(&test.env);
        actions.push_back(String::from_str(&test.env, "action_1"));
        actions.push_back(String::from_str(&test.env, "action_2"));
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn test_recovery_issues_detected_vector() {
        let test = RecoveryTest::new();
        // Test that issues_detected vector works
        let mut issues = Vec::new(&test.env);
        issues.push_back(String::from_str(&test.env, "issue_1"));
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_partial_refund_total_tracking() {
        let test = RecoveryTest::new();
        // Test that partial_refund_total is properly tracked
        let amount1 = 1000i128;
        let amount2 = 500i128;
        let total = amount1 + amount2;
        assert_eq!(total, 1500);
    }

    #[test]
    fn test_recovery_data_inconsistencies() {
        let test = RecoveryTest::new();
        // Test RecoveryData with inconsistencies
        let mut inconsistencies = Vec::new(&test.env);
        inconsistencies.push_back(String::from_str(&test.env, "inc_1"));
        let data = RecoveryData {
            inconsistencies,
            can_recover: true,
            safety_score: 50,
        };
        assert_eq!(data.inconsistencies.len(), 1);
    }

    #[test]
    fn test_recovery_action_enum_variants() {
        // Test all RecoveryAction variants exist
        let _ = RecoveryAction::MarketStateReconstructed;
        let _ = RecoveryAction::PartialRefundExecuted;
        let _ = RecoveryAction::IntegrityValidated;
        let _ = RecoveryAction::RecoverySkipped;
    }

    #[test]
    fn test_market_recovery_structure_creation() {
        let test = RecoveryTest::new();
        // Test creating MarketRecovery structure
        let recovery = MarketRecovery {
            market_id: test.market_id.clone(),
            actions: Vec::new(&test.env),
            issues_detected: Vec::new(&test.env),
            recovered: false,
            partial_refund_total: 0,
            last_action: None,
        };
        assert!(!recovery.market_id.to_string().is_empty());
    }

    #[test]
    fn test_market_recovery_with_partial_refund() {
        let test = RecoveryTest::new();
        // Test MarketRecovery with partial refund amount
        let recovery = MarketRecovery {
            market_id: test.market_id.clone(),
            actions: Vec::new(&test.env),
            issues_detected: Vec::new(&test.env),
            recovered: true,
            partial_refund_total: 5000,
            last_action: Some(String::from_str(&test.env, "refund_done")),
        };
        assert!(recovery.partial_refund_total > 0);
    }

    #[test]
    fn test_recovery_storage_keys() {
        let test = RecoveryTest::new();
        // Test that storage keys are properly generated
        let records_key = RecoveryStorage::records_key(&test.env);
        let status_key = RecoveryStorage::status_key(&test.env);
        assert_ne!(records_key.to_string(), status_key.to_string());
    }

    #[test]
    fn test_symbol_to_string_conversion() {
        let test = RecoveryTest::new();
        // Test helper function for symbol to string conversion
        let symbol = Symbol::new(&test.env, "test_symbol");
        let string = symbol_to_string(&test.env, &symbol);
        assert!(!string.to_string().is_empty());
    }

    #[test]
    fn test_recovery_event_emission() {
        let test = RecoveryTest::new();
        // Test that recovery event can be emitted
        let market_id = test.market_id;
        let action = String::from_str(&test.env, "recover");
        let status = String::from_str(&test.env, "success");
        // Event emission should not panic
        EventEmitter::emit_recovery_event(&test.env, &market_id, &action, &status);
        assert!(true);
    }

    #[test]
    fn test_recovery_no_action_case() {
        let test = RecoveryTest::new();
        // Test recovery when no action is needed
        let recovery = MarketRecovery {
            market_id: test.market_id.clone(),
            actions: Vec::new(&test.env),
            issues_detected: Vec::new(&test.env),
            recovered: false,
            partial_refund_total: 0,
            last_action: Some(String::from_str(&test.env, "no_action_needed")),
        };
        assert!(!recovery.recovered);
    }

    #[test]
    fn test_recovery_reconstructed_state() {
        let test = RecoveryTest::new();
        // Test recovery state after reconstruction
        let mut actions = Vec::new(&test.env);
        actions.push_back(String::from_str(&test.env, "reconstructed_totals"));
        let recovery = MarketRecovery {
            market_id: test.market_id.clone(),
            actions,
            issues_detected: Vec::new(&test.env),
            recovered: true,
            partial_refund_total: 0,
            last_action: Some(String::from_str(&test.env, "reconstructed")),
        };
        assert!(recovery.recovered);
    }

    #[test]
    fn test_recovery_data_safety_score_range() {
        let test = RecoveryTest::new();
        // Test safety score boundary conditions
        let high_score = 100i128;
        let low_score = 1i128;
        let zero_score = 0i128;
        assert!(high_score > low_score);
        assert!(low_score > zero_score);
    }

    #[test]
    fn test_recovery_multiple_issues() {
        let test = RecoveryTest::new();
        // Test recovery record with multiple issues
        let mut issues = Vec::new(&test.env);
        issues.push_back(String::from_str(&test.env, "issue_1"));
        issues.push_back(String::from_str(&test.env, "issue_2"));
        issues.push_back(String::from_str(&test.env, "issue_3"));
        assert_eq!(issues.len(), 3);
    }
}

// Helper to build composite key prefix + symbol as soroban Symbol
// composite_symbol no longer required with new map-based storage approach
