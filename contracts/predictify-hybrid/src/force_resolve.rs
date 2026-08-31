#![allow(dead_code)]

use soroban_sdk::{contracttype, panic_with_error, symbol_short, Address, Env, String, Symbol, Vec};

use crate::err::Error;

/// Record of a force-resolve operation, stored for idempotency.
///
/// Once stored, the same `(market_id, idempotency_key)` pair guarantees
/// that a subsequent force-resolve call is a safe no-op rather than
/// re-applying the resolution.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForceResolveRecord {
    pub resolved: bool,
    pub timestamp: u64,
    pub admin: Address,
    pub winning_outcomes: Vec<String>,
}

pub struct ForceResolveManager;

impl ForceResolveManager {
    /// Deterministic storage key for an idempotency record.
    fn idempotency_storage_key(market_id: &Symbol, key: &String) -> (Symbol, Symbol, String) {
        (symbol_short!("frc_rslv"), market_id.clone(), key.clone())
    }

    /// Returns `true` when the idempotency key has already been consumed for
    /// this market.
    pub fn is_already_resolved(env: &Env, market_id: &Symbol, key: &String) -> bool {
        let storage_key = Self::idempotency_storage_key(market_id, key);
        env.storage().persistent().has(&storage_key)
    }

    /// Consumes the idempotency key by persisting a `ForceResolveRecord`.
    ///
    /// # Panics
    ///
    /// Panics with `Error::ForceResolveAlreadyUsed` when the key has already
    /// been consumed (callers should check `is_already_resolved` first).
    pub fn mark_resolved(
        env: &Env,
        market_id: &Symbol,
        key: &String,
        admin: &Address,
        winning_outcomes: &Vec<String>,
    ) {
        if Self::is_already_resolved(env, market_id, key) {
            panic_with_error!(env, Error::ForceResolveAlreadyUsed);
        }

        let record = ForceResolveRecord {
            resolved: true,
            timestamp: env.ledger().timestamp(),
            admin: admin.clone(),
            winning_outcomes: winning_outcomes.clone(),
        };
        let storage_key = Self::idempotency_storage_key(market_id, key);
        env.storage().persistent().set(&storage_key, &record);
    }

    /// Returns the `ForceResolveRecord` for a given market and key, if one
    /// exists.
    pub fn get_record(
        env: &Env,
        market_id: &Symbol,
        key: &String,
    ) -> Option<ForceResolveRecord> {
        let storage_key = Self::idempotency_storage_key(market_id, key);
        env.storage().persistent().get(&storage_key)
    }
}
