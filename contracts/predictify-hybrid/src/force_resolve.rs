#![allow(dead_code)]

use soroban_sdk::{
    contracttype, panic_with_error, symbol_short, Address, Env, String, Symbol, Vec,
};

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

/// Record of a payout-remainder allocation for a force-resolved market.
///
/// Kept separate from the force-resolve record so that exactly one remainder
/// allocation can be claimed per force-resolved market.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayoutRemainderAllocation {
    pub amount: u64,
    pub recipient: Address,
    pub allocated: bool,
}

/// Manager for the admin force-resolve feature and its idempotency story.
pub struct ForceResolveManager;

/// Compute the undistributed remainder from `total_amount` minus the sum of
/// `payout_amounts`.
///
/// # Panics
/// - If `payout_amounts` sum to more than `total_amount` (assertion failure).
/// - If any payout amount overflows `u64` in the running sum.
pub fn calculate_payout_remainder(total_amount: u64, payout_amounts: &Vec<u64>) -> u64 {
    let mut sum = 0u64;
    for i in 0..payout_amounts.len() {
        let amount = payout_amounts
            .get(i)
            .expect("payout amount index out of bounds");
        sum = sum
            .checked_add(amount)
            .expect("payout amount overflow");
    }
    assert!(
        sum <= total_amount,
        "payout amounts sum exceeds total amount"
    );
    total_amount - sum
}

impl ForceResolveManager {
    /// Deterministic storage key for a force-resolve idempotency record.
    fn idempotency_storage_key(market_id: &Symbol, key: &String) -> (Symbol, Symbol, String) {
        (symbol_short!("frc_rslv"), market_id.clone(), key.clone())
    }

    /// Deterministic storage key for a payout-remainder allocation.
    fn remainder_storage_key(market_id: &Symbol, key: &String) -> (Symbol, Symbol, String) {
        (symbol_short!("frc_rmndr"), market_id.clone(), key.clone())
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
    /// - `Error::ForceResolveAlreadyUsed` if the key was already consumed.
    pub fn mark_resolved(
        env: &Env,
        market_id: &Symbol,
        key: &String,
        admin: &Address,
        winning_outcomes: &Vec<String>,
    ) {
        admin.require_auth();
        assert!(key.len() > 0, "force resolve key must not be empty");
        assert!(
            !winning_outcomes.is_empty(),
            "winning outcomes must not be empty"
        );

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

    /// Retrieves the stored `ForceResolveRecord` for a market/key pair (if any).
    pub fn get_record(
        env: &Env,
        market_id: &Symbol,
        key: &String,
    ) -> Option<ForceResolveRecord> {
        let storage_key = Self::idempotency_storage_key(market_id, key);
        env.storage().persistent().get(&storage_key)
    }

    /// Allocates the payout remainder for a force-resolved market.
    ///
    /// Only the admin that performed the force resolve (recorded on the
    /// `ForceResolveRecord`) may allocate the remainder, and only once.
    ///
    /// # Panics
    /// - If no force-resolve record exists for the market/key pair.
    /// - If a remainder allocation was already recorded for the pair.
    /// - If `amount` is zero.
    pub fn allocate_payout_remainder(
        env: &Env,
        market_id: &Symbol,
        key: &String,
        amount: u64,
        recipient: &Address,
    ) {
        let record = Self::get_record(env, market_id, key)
            .expect("force resolve record not found; cannot allocate remainder");

        record.admin.require_auth();

        let storage_key = Self::remainder_storage_key(market_id, key);
        if env.storage().persistent().has(&storage_key) {
            panic!("payout remainder already allocated");
        }

        assert!(
            amount > 0,
            "payout remainder amount must be greater than zero"
        );

        let allocation = PayoutRemainderAllocation {
            amount,
            recipient: recipient.clone(),
            allocated: true,
        };
        env.storage().persistent().set(&storage_key, &allocation);
        env.events().publish(
            (symbol_short!("rmndr"), market_id.clone(), key.clone()),
            allocation,
        );
    }

    /// Retrieves the payout-remainder allocation for a market/key pair (if any).
    pub fn get_payout_remainder_allocation(
        env: &Env,
        market_id: &Symbol,
        key: &String,
    ) -> Option<PayoutRemainderAllocation> {
        let storage_key = Self::remainder_storage_key(market_id, key);
        env.storage().persistent().get(&storage_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_calculate_payout_remainder() {
        let env = Env::default();
        let mut payouts = Vec::new(&env);
        payouts.push_back(10u64);
        payouts.push_back(10u64);
        payouts.push_back(10u64);
        let remainder = calculate_payout_remainder(32, &payouts);
        assert_eq!(remainder, 2);
    }

    #[test]
    fn test_calculate_payout_remainder_no_remainder() {
        let env = Env::default();
        let mut payouts = Vec::new(&env);
        payouts.push_back(10u64);
        payouts.push_back(10u64);
        let remainder = calculate_payout_remainder(20, &payouts);
        assert_eq!(remainder, 0);
    }

    #[test]
    #[should_panic]
    fn test_calculate_payout_remainder_exceeds_total() {
        let env = Env::default();
        let mut payouts = Vec::new(&env);
        payouts.push_back(10u64);
        payouts.push_back(10u64);
        let _ = calculate_payout_remainder(15, &payouts);
    }
}