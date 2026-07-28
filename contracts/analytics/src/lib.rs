//! Analytics contract.
//!
//! Provides on-chain aggregation and querying of market analytics data for
//! the Predictify platform. The contract records participation metrics,
//! market statistics, and fee summaries, and exposes them as read-only views
//! consumed by dashboards and off-chain indexers.
//!
//! # Auth matrix
//!
//! | Entrypoint                | Required role |
//! |---------------------------|---------------|
//! | `initialize`              | Admin         |
//! | `record_market_snapshot`  | Admin         |
//! | `pause_analytics`         | Admin         |
//! | `unpause_analytics`       | Admin         |
//! | `transfer_admin`          | Admin         |
//! | `get_snapshot`            | Anyone        |
//! | `is_paused`               | Anyone        |
//! | `admin`                   | Anyone        |
//! | `version`                 | Anyone        |

#![no_std]

extern crate std;

mod errors;
pub use errors::ContractError;

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

// ============================================================
// Storage key symbols
// ============================================================

const KEY_ADMIN: &str = "Admin";
const KEY_PAUSED: &str = "Paused";
const KEY_INITIALIZED: &str = "Init";

// ============================================================
// Types
// ============================================================

/// A snapshot of market participation metrics recorded at a point in time.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketSnapshot {
    /// On-chain ledger timestamp at which the snapshot was recorded.
    pub recorded_at: u64,
    /// Total number of unique participants across all tracked markets.
    pub total_participants: u32,
    /// Sum of all stakes placed, in stroops.
    pub total_stake: i128,
    /// Number of markets that reached resolution in this window.
    pub resolved_markets: u32,
    /// Number of active (open) markets at the time of the snapshot.
    pub active_markets: u32,
}

// ============================================================
// Contract
// ============================================================

/// The Analytics contract.
#[contract]
pub struct AnalyticsContract;

#[contractimpl]
impl AnalyticsContract {
    // ------------------------------------------------------------------
    // Initialisation
    // ------------------------------------------------------------------

    /// Initialise the contract with an `admin` address.
    ///
    /// May only be called once.
    ///
    /// # Errors
    /// - [`ContractError::AlreadyInitialized`] if the contract has already
    ///   been initialized.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();

        if env
            .storage()
            .instance()
            .has(&Symbol::new(&env, KEY_INITIALIZED))
        {
            return Err(ContractError::AlreadyInitialized);
        }

        env.storage()
            .instance()
            .set(&Symbol::new(&env, KEY_ADMIN), &admin);
        env.storage()
            .instance()
            .set(&Symbol::new(&env, KEY_INITIALIZED), &true);
        env.storage()
            .instance()
            .set(&Symbol::new(&env, KEY_PAUSED), &false);

        Ok(())
    }

    // ------------------------------------------------------------------
    // State-changing entrypoints
    // ------------------------------------------------------------------

    /// Record a market participation snapshot for the current ledger.
    ///
    /// Only the admin may call this entrypoint. The snapshot is stored under
    /// a key derived from `market_id` and can later be retrieved via
    /// [`Self::get_snapshot`].
    ///
    /// # Errors
    /// - [`ContractError::Unauthorized`] if the caller is not the admin.
    /// - [`ContractError::AnalyticsPaused`] if analytics collection is paused.
    /// - [`ContractError::InvalidState`] if the contract is not initialized.
    pub fn record_market_snapshot(
        env: Env,
        admin: Address,
        market_id: Symbol,
        snapshot: MarketSnapshot,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_initialized(&env)?;
        Self::assert_is_admin(&env, &admin)?;
        Self::assert_not_paused(&env)?;

        env.storage()
            .persistent()
            .set(&(symbol_short!("snap"), market_id), &snapshot);

        Ok(())
    }

    /// Pause analytics data collection.
    ///
    /// While paused, [`Self::record_market_snapshot`] will return
    /// [`ContractError::AnalyticsPaused`].
    ///
    /// # Errors
    /// - [`ContractError::Unauthorized`] if the caller is not the admin.
    pub fn pause_analytics(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_initialized(&env)?;
        Self::assert_is_admin(&env, &admin)?;

        env.storage()
            .instance()
            .set(&Symbol::new(&env, KEY_PAUSED), &true);

        Ok(())
    }

    /// Resume analytics data collection.
    ///
    /// # Errors
    /// - [`ContractError::Unauthorized`] if the caller is not the admin.
    pub fn unpause_analytics(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();
        Self::assert_initialized(&env)?;
        Self::assert_is_admin(&env, &admin)?;

        env.storage()
            .instance()
            .set(&Symbol::new(&env, KEY_PAUSED), &false);

        Ok(())
    }

    /// Transfer admin ownership to `new_admin`.
    ///
    /// # Errors
    /// - [`ContractError::Unauthorized`] if the caller is not the current admin.
    /// - [`ContractError::InvalidConfig`] if `new_admin` is the same as the
    ///   current admin.
    pub fn transfer_admin(
        env: Env,
        current_admin: Address,
        new_admin: Address,
    ) -> Result<(), ContractError> {
        current_admin.require_auth();
        Self::assert_initialized(&env)?;
        Self::assert_is_admin(&env, &current_admin)?;

        if new_admin == current_admin {
            return Err(ContractError::InvalidConfig);
        }

        env.storage()
            .instance()
            .set(&Symbol::new(&env, KEY_ADMIN), &new_admin);

        Ok(())
    }

    // ------------------------------------------------------------------
    // Read-only entrypoints
    // ------------------------------------------------------------------

    /// Return the snapshot recorded for `market_id`, or an error if absent.
    ///
    /// # Errors
    /// - [`ContractError::SnapshotNotFound`] if no snapshot has been recorded
    ///   for the given market.
    pub fn get_snapshot(env: Env, market_id: Symbol) -> Result<MarketSnapshot, ContractError> {
        env.storage()
            .persistent()
            .get(&(symbol_short!("snap"), market_id))
            .ok_or(ContractError::SnapshotNotFound)
    }

    /// Return `true` if analytics collection is currently paused.
    pub fn is_paused(env: Env) -> bool {
        env.storage()
            .instance()
            .get::<Symbol, bool>(&Symbol::new(&env, KEY_PAUSED))
            .unwrap_or(false)
    }

    /// Return the current admin address.
    ///
    /// # Errors
    /// - [`ContractError::AdminNotSet`] if the contract is not initialized.
    pub fn admin(env: Env) -> Result<Address, ContractError> {
        env.storage()
            .instance()
            .get::<Symbol, Address>(&Symbol::new(&env, KEY_ADMIN))
            .ok_or(ContractError::AdminNotSet)
    }

    /// Return the contract version.
    pub fn version(_env: Env) -> u32 {
        1
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    fn assert_initialized(env: &Env) -> Result<(), ContractError> {
        if !env
            .storage()
            .instance()
            .has(&Symbol::new(env, KEY_INITIALIZED))
        {
            return Err(ContractError::NotInitialized);
        }
        Ok(())
    }

    fn assert_is_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
        let stored: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(env, KEY_ADMIN))
            .ok_or(ContractError::AdminNotSet)?;
        if caller != &stored {
            return Err(ContractError::Unauthorized);
        }
        Ok(())
    }

    fn assert_not_paused(env: &Env) -> Result<(), ContractError> {
        let paused: bool = env
            .storage()
            .instance()
            .get(&Symbol::new(env, KEY_PAUSED))
            .unwrap_or(false);
        if paused {
            return Err(ContractError::AnalyticsPaused);
        }
        Ok(())
    }
}
