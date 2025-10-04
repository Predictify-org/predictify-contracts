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
                    market.claimed.set(user.clone(), true);
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

// Helper to build composite key prefix + symbol as soroban Symbol
// composite_symbol no longer required with new map-based storage approach
