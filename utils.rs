//! Pure utility helpers for Predictify Hybrid contracts.
//! 
//! This module contains deterministic logic for financial calculations,
//! string formatting, and state mapping that does not rely on direct
//! storage access.

/// Calculate a percentage of an amount using basis points (1/10000).
///
/// # Arguments
/// * `amount` - The base amount (e.g., total pool in stroops).
/// * `bps` - Basis points (e.g., 250 for 2.5%).
///
/// # Returns
/// The calculated percentage amount, rounding down.
pub fn calculate_bps(amount: i128, bps: u32) -> i128 {
    if amount <= 0 {
        return 0;
    }
    amount.saturating_mul(bps as i128) / 10000
}

/// Calculates the proportional share of a pool for a specific stake.
///
/// Used for determining winning payouts: (total_pool * user_stake) / total_winning_stakes.
///
/// # Arguments
/// * `total_pool` - The total amount to be distributed.
/// * `user_stake` - The user's individual stake.
/// * `total_winning_stakes` - The aggregate stakes of all winners.
///
/// # Returns
/// The user's share of the pool. Returns 0 if total_winning_stakes is 0.
pub fn calculate_payout_share(
    total_pool: i128,
    user_stake: i128,
    total_winning_stakes: i128,
) -> i128 {
    if total_winning_stakes <= 0 || total_pool <= 0 || user_stake <= 0 {
        return 0;
    }
    total_pool.saturating_mul(user_stake) / total_winning_stakes
}