#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

pub mod errors;

/// Persistent storage keys for the resolution contract.
#[contracttype]
pub enum DataKey {
    /// Per-market resolution outcome (`u32`). Market identified by `u64`.
    ResolutionResult(u64),
    /// Minimum votes required for a proposal to be accepted.
    MinVotes,
}

/// ---------------------------------------------------------------
///  TTL constants
/// ---------------------------------------------------------------
///
/// Threshold is set so that a key whose remaining TTL has dropped
/// below ~7 days (at ~5 s / ledger) is extended again.  The extend
/// amount refreshes the entry to ~30 days, keeping hot resolution
/// data alive across the typical market lifecycle without excessive
/// rent overhead.

/// Bump only if remaining TTL is below this (≈7 days).
pub const TTL_THRESHOLD: u32 = 120_960;
/// Extend TTL to this many ledgers on bump (≈30 days).
pub const TTL_EXTEND: u32 = 518_400;

/// Bump the persistent TTL for a [`DataKey`] if it already exists.
fn bump_ttl(env: &Env, key: &DataKey) {
    if env.storage().persistent().has(key) {
        env.storage()
            .persistent()
            .extend_ttl(key, TTL_THRESHOLD, TTL_EXTEND);
    }
}

#[contract]
pub struct ResolutionContract;

#[contractimpl]
impl ResolutionContract {
    /// Returns the contract version for the resolution subsystem.
    pub fn version(_env: Env) -> u32 {
        7
    }

    /// Store a resolution outcome for a market.
    ///
    /// Bumps TTL on the storage key **before** the read-guard so that
    /// an existing entry is kept alive, then extends TTL after storing.
    pub fn resolve(env: Env, admin: Address, market_id: u64, outcome: u32) {
        admin.require_auth();
        let key = DataKey::ResolutionResult(market_id);
        bump_ttl(&env, &key);
        env.storage().persistent().set(&key, &outcome);
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND);
    }

    /// Propose a resolution for a market.
    ///
    /// Requires `user` authorization.  (Storage-backed proposal
    /// tracking can be added in a follow-up; the auth snapshot and
    /// entrypoint signature are established here.)
    pub fn propose(_env: Env, user: Address, _market_id: u64) {
        user.require_auth();
    }

    /// Update the minimum votes required for proposals.
    ///
    /// Bumps TTL before reading the current value so that stale
    /// config entries are refreshed, then extends TTL after writing.
    pub fn update_config(env: Env, admin: Address, new_min_votes: u32) {
        admin.require_auth();
        let key = DataKey::MinVotes;
        bump_ttl(&env, &key);
        env.storage().persistent().set(&key, &new_min_votes);
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_THRESHOLD, TTL_EXTEND);
    }
}


