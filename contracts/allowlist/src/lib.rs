//! Allowlist smart contract with structured lifecycle events.
//!
//! Manages per-ID allowlists of `Address` values. Each allowlist is identified
//! by a unique `Symbol` and stores an ordered `Vec<Address>`. All state-changing
//! operations require `admin` authentication and emit a typed, structured event
//! for off-chain indexers.
//!
//! # Overview
//!
//! The contract supports creating, populating, querying, and deleting
//! independent allowlists. Every lifecycle transition — creation, address
//! addition/removal, clear, deletion, and ownership transfer — emits a
//! distinct event with a stable topic symbol and all relevant payload data.
//!
//! # Authorization
//!
//! - All state-changing entrypoints require `require_auth()` from the
//!   registered admin address.
//! - Read-only entrypoints (`is_allowed`, `get_allowlist`, `list_allowlists`,
//!   `get_admin`, `version`) do not require authentication.
//!
//! # Event Topics
//!
//! | Topic               | Description                          |
//! |---------------------|--------------------------------------|
//! | `alist_init`        | Contract initialized                 |
//! | `alist_created`     | New allowlist created                |
//! | `alist_addr_add`    | Single address added                 |
//! | `alist_addr_rem`    | Single address removed               |
//! | `alist_cleared`     | All addresses cleared                |
//! | `alist_deleted`     | Entire allowlist deleted             |
//! | `alist_owner_xf`    | Ownership transferred                |

#![no_std]

mod err;
mod events;

pub use err::AllowlistError;

use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, Symbol, Vec,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Minimum remaining ledgers before an allowlist entry is bumped on read.
const ALLOWLIST_TTL_THRESHOLD: u32 = 100_000;
/// Extended TTL ledgers for allowlist entries.
const ALLOWLIST_TTL_TO: u32 = 518_400;

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------

/// Persistent storage keys used by the Allowlist contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Whether the contract has been initialized (`bool`).
    Initialized,
    /// The current admin address (`Address`).
    Admin,
    /// Registry of all created allowlist IDs (`Vec<Symbol>`).
    AllowlistRegistry,
}

/// Underlying storage key for an individual allowlist's address vector.
///
/// Constructed at runtime as a tuple `(Symbol("Allowlist"), allowlist_id)` so
/// that each allowlist is stored under a unique composite key.
fn allowlist_storage_key(env: &Env, allowlist_id: &Symbol) -> (Symbol, Symbol) {
    (Symbol::new(env, "Allowlist"), allowlist_id.clone())
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct AllowlistContract;

#[contractimpl]
impl AllowlistContract {
    // =======================================================================
    // Initialisation
    // =======================================================================

    /// Initialise the contract with an `admin`.
    ///
    /// May only be called once. Emits an `alist_init` event.
    ///
    /// # Authorization
    /// The `admin` must authenticate via `require_auth()`.
    ///
    /// # Errors
    /// - [`AllowlistError::AlreadyInitialized`] if already initialized.
    pub fn initialize(env: Env, admin: Address) -> Result<(), AllowlistError> {
        admin.require_auth();

        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(AllowlistError::AlreadyInitialized);
        }

        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&DataKey::AllowlistRegistry, &Vec::<Symbol>::new(&env));

        events::emit_allowlist_initialized(&env, &admin);

        Ok(())
    }

    // =======================================================================
    // State-changing entrypoints (require admin auth)
    // =======================================================================

    /// Create a new allowlist identified by `allowlist_id`.
    ///
    /// Emits an `alist_created` event.
    ///
    /// # Authorization
    /// The `admin` must authenticate via `require_auth()` and be the
    /// registered admin.
    ///
    /// # Errors
    /// - [`AllowlistError::NotInitialized`] if contract not initialized.
    /// - [`AllowlistError::Unauthorized`] if caller is not the admin.
    /// - [`AllowlistError::AllowlistAlreadyExists`] if the ID is taken.
    pub fn create_allowlist(
        env: Env,
        admin: Address,
        allowlist_id: Symbol,
    ) -> Result<(), AllowlistError> {
        admin.require_auth();
        Self::require_initialized(&env)?;
        Self::require_admin(&env, &admin)?;

        let key = allowlist_storage_key(&env, &allowlist_id);
        if env.storage().persistent().has(&key) {
            return Err(AllowlistError::AllowlistAlreadyExists);
        }

        // Store and persist the empty allowlist (with TTL extension)
        Self::save_allowlist(&env, &allowlist_id, &Vec::<Address>::new(&env));

        // Register the allowlist ID
        let mut registry: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&DataKey::AllowlistRegistry)
            .unwrap_or_else(|| Vec::new(&env));

        if !registry.contains(&allowlist_id) {
            registry.push_back(allowlist_id.clone());
            env.storage()
                .persistent()
                .set(&DataKey::AllowlistRegistry, &registry);
        }

        events::emit_allowlist_created(&env, &admin, &allowlist_id);

        Ok(())
    }

    /// Add an `address` to an existing allowlist.
    ///
    /// Emits an `alist_addr_add` event.
    ///
    /// # Authorization
    /// The `admin` must authenticate via `require_auth()` and be the
    /// registered admin.
    ///
    /// # Errors
    /// - [`AllowlistError::NotInitialized`] if contract not initialized.
    /// - [`AllowlistError::Unauthorized`] if caller is not the admin.
    /// - [`AllowlistError::AllowlistNotFound`] if the allowlist does not exist.
    /// - [`AllowlistError::AddressAlreadyInAllowlist`] if the address is already present.
    pub fn add_address(
        env: Env,
        admin: Address,
        allowlist_id: Symbol,
        address: Address,
    ) -> Result<(), AllowlistError> {
        admin.require_auth();
        Self::require_initialized(&env)?;
        Self::require_admin(&env, &admin)?;

        let mut addrs = Self::load_allowlist(&env, &allowlist_id)?;

        if addrs.contains(&address) {
            return Err(AllowlistError::AddressAlreadyInAllowlist);
        }

        addrs.push_back(address.clone());
        Self::save_allowlist(&env, &allowlist_id, &addrs);

        events::emit_allowlist_address_added(&env, &admin, &allowlist_id, &address);

        Ok(())
    }

    /// Remove an `address` from an existing allowlist.
    ///
    /// Emits an `alist_addr_rem` event.
    ///
    /// # Authorization
    /// The `admin` must authenticate via `require_auth()` and be the
    /// registered admin.
    ///
    /// # Errors
    /// - [`AllowlistError::NotInitialized`] if contract not initialized.
    /// - [`AllowlistError::Unauthorized`] if caller is not the admin.
    /// - [`AllowlistError::AllowlistNotFound`] if the allowlist does not exist.
    /// - [`AllowlistError::AddressNotInAllowlist`] if the address is not present.
    pub fn remove_address(
        env: Env,
        admin: Address,
        allowlist_id: Symbol,
        address: Address,
    ) -> Result<(), AllowlistError> {
        admin.require_auth();
        Self::require_initialized(&env)?;
        Self::require_admin(&env, &admin)?;

        let addrs = Self::load_allowlist(&env, &allowlist_id)?;

        if !addrs.contains(&address) {
            return Err(AllowlistError::AddressNotInAllowlist);
        }

        let mut filtered: Vec<Address> = Vec::new(&env);
        for a in addrs.iter() {
            if a != address {
                filtered.push_back(a);
            }
        }
        Self::save_allowlist(&env, &allowlist_id, &filtered);

        events::emit_allowlist_address_removed(&env, &admin, &allowlist_id, &address);

        Ok(())
    }

    /// Batch-add multiple `addresses` to an existing allowlist.
    ///
    /// Uniqueness is enforced per address: addresses already present are
    /// silently skipped (idempotent). An `alist_addr_add` event is emitted
    /// for each address that is actually added.
    ///
    /// # Authorization
    /// The `admin` must authenticate via `require_auth()` and be the
    /// registered admin.
    ///
    /// # Errors
    /// - [`AllowlistError::NotInitialized`] if contract not initialized.
    /// - [`AllowlistError::Unauthorized`] if caller is not the admin.
    /// - [`AllowlistError::AllowlistNotFound`] if the allowlist does not exist.
    pub fn add_addresses(
        env: Env,
        admin: Address,
        allowlist_id: Symbol,
        addresses: Vec<Address>,
    ) -> Result<(), AllowlistError> {
        admin.require_auth();
        Self::require_initialized(&env)?;
        Self::require_admin(&env, &admin)?;

        let mut addrs = Self::load_allowlist(&env, &allowlist_id)?;

        for addr in addresses.iter() {
            if !addrs.contains(&addr) {
                addrs.push_back(addr.clone());
                events::emit_allowlist_address_added(&env, &admin, &allowlist_id, &addr);
            }
        }

        Self::save_allowlist(&env, &allowlist_id, &addrs);

        Ok(())
    }

    /// Batch-remove multiple `addresses` from an existing allowlist.
    ///
    /// Addresses not already in the allowlist are silently skipped (idempotent).
    /// An `alist_addr_rem` event is emitted for each address that is actually
    /// removed.
    ///
    /// # Authorization
    /// The `admin` must authenticate via `require_auth()` and be the
    /// registered admin.
    ///
    /// # Errors
    /// - [`AllowlistError::NotInitialized`] if contract not initialized.
    /// - [`AllowlistError::Unauthorized`] if caller is not the admin.
    /// - [`AllowlistError::AllowlistNotFound`] if the allowlist does not exist.
    pub fn remove_addresses(
        env: Env,
        admin: Address,
        allowlist_id: Symbol,
        addresses: Vec<Address>,
    ) -> Result<(), AllowlistError> {
        admin.require_auth();
        Self::require_initialized(&env)?;
        Self::require_admin(&env, &admin)?;

        let addrs = Self::load_allowlist(&env, &allowlist_id)?;

        // Separate addresses to keep vs. remove, emitting an event for each removal.
        let mut remaining = Vec::new(&env);
        for a in addrs.iter() {
            if addresses.contains(&a) {
                events::emit_allowlist_address_removed(&env, &admin, &allowlist_id, &a);
            } else {
                remaining.push_back(a.clone());
            }
        }

        Self::save_allowlist(&env, &allowlist_id, &remaining);

        Ok(())
    }

    /// Remove all addresses from an allowlist.
    ///
    /// Emits an `alist_cleared` event with the count of removed addresses.
    ///
    /// # Authorization
    /// The `admin` must authenticate via `require_auth()` and be the
    /// registered admin.
    ///
    /// # Errors
    /// - [`AllowlistError::NotInitialized`] if contract not initialized.
    /// - [`AllowlistError::Unauthorized`] if caller is not the admin.
    /// - [`AllowlistError::AllowlistNotFound`] if the allowlist does not exist.
    pub fn clear_allowlist(
        env: Env,
        admin: Address,
        allowlist_id: Symbol,
    ) -> Result<(), AllowlistError> {
        admin.require_auth();
        Self::require_initialized(&env)?;
        Self::require_admin(&env, &admin)?;

        let addrs = Self::load_allowlist(&env, &allowlist_id)?;
        let count: u32 = addrs.len();

        // Replace with empty allowlist (via save_allowlist for TTL extension)
        Self::save_allowlist(&env, &allowlist_id, &Vec::<Address>::new(&env));

        if count > 0 {
            events::emit_allowlist_cleared(&env, &admin, &allowlist_id, count);
        }

        Ok(())
    }

    /// Delete an entire allowlist, removing it from the registry.
    ///
    /// Emits an `alist_deleted` event with the count of addresses that were
    /// in the allowlist at deletion time.
    ///
    /// # Authorization
    /// The `admin` must authenticate via `require_auth()` and be the
    /// registered admin.
    ///
    /// # Errors
    /// - [`AllowlistError::NotInitialized`] if contract not initialized.
    /// - [`AllowlistError::Unauthorized`] if caller is not the admin.
    /// - [`AllowlistError::AllowlistNotFound`] if the allowlist does not exist.
    pub fn delete_allowlist(
        env: Env,
        admin: Address,
        allowlist_id: Symbol,
    ) -> Result<(), AllowlistError> {
        admin.require_auth();
        Self::require_initialized(&env)?;
        Self::require_admin(&env, &admin)?;

        let addrs = Self::load_allowlist(&env, &allowlist_id)?;
        let count: u32 = addrs.len();

        // Remove the allowlist data
        env.storage()
            .persistent()
            .remove(&allowlist_storage_key(&env, &allowlist_id));

        // Remove from registry
        let registry: Vec<Symbol> = env
            .storage()
            .persistent()
            .get(&DataKey::AllowlistRegistry)
            .unwrap_or_else(|| Vec::new(&env));

        let mut filtered: Vec<Symbol> = Vec::new(&env);
        for id in registry.iter() {
            if id != allowlist_id {
                filtered.push_back(id);
            }
        }
        env.storage()
            .persistent()
            .set(&DataKey::AllowlistRegistry, &filtered);

        events::emit_allowlist_deleted(&env, &admin, &allowlist_id, count);

        Ok(())
    }

    /// Transfer contract ownership to a new admin.
    ///
    /// Emits an `alist_owner_xf` event.
    ///
    /// # Authorization
    /// The `current_admin` must authenticate via `require_auth()` and be the
    /// registered admin.
    ///
    /// # Errors
    /// - [`AllowlistError::NotInitialized`] if contract not initialized.
    /// - [`AllowlistError::Unauthorized`] if caller is not the admin.
    /// - [`AllowlistError::InvalidInput`] if new_admin is the same as current.
    pub fn transfer_ownership(
        env: Env,
        current_admin: Address,
        new_admin: Address,
    ) -> Result<(), AllowlistError> {
        current_admin.require_auth();
        Self::require_initialized(&env)?;
        Self::require_admin(&env, &current_admin)?;

        if new_admin == current_admin {
            return Err(AllowlistError::InvalidInput);
        }

        env.storage()
            .instance()
            .set(&DataKey::Admin, &new_admin);

        events::emit_allowlist_ownership_transferred(&env, &current_admin, &new_admin);

        Ok(())
    }

    // =======================================================================
    // Read-only entrypoints (no auth required)
    // =======================================================================

    /// Return the contract's numeric version.
    pub fn version(_env: Env) -> u32 {
        1
    }

    /// Return the current admin address.
    ///
    /// # Errors
    /// - [`AllowlistError::NotInitialized`] if contract not initialized.
    pub fn get_admin(env: Env) -> Result<Address, AllowlistError> {
        Self::require_initialized(&env)?;
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .ok_or(AllowlistError::NotInitialized)
    }

    /// Check whether an `address` is present in the allowlist identified by
    /// `allowlist_id`.
    ///
    /// # Errors
    /// - [`AllowlistError::AllowlistNotFound`] if the allowlist does not exist.
    pub fn is_allowed(env: Env, allowlist_id: Symbol, address: Address) -> Result<bool, AllowlistError> {
        let addrs = Self::load_allowlist(&env, &allowlist_id)?;
        Ok(addrs.contains(&address))
    }

    /// Return all addresses in the allowlist identified by `allowlist_id`.
    ///
    /// # Errors
    /// - [`AllowlistError::AllowlistNotFound`] if the allowlist does not exist.
    pub fn get_allowlist(env: Env, allowlist_id: Symbol) -> Result<Vec<Address>, AllowlistError> {
        Self::load_allowlist(&env, &allowlist_id)
    }

    /// Return all registered allowlist IDs.
    pub fn list_allowlists(env: Env) -> Vec<Symbol> {
        env.storage()
            .persistent()
            .get(&DataKey::AllowlistRegistry)
            .unwrap_or_else(|| Vec::new(&env))
    }

    // =======================================================================
    // Internal helpers
    // =======================================================================

    /// Assert the contract is initialized.
    fn require_initialized(env: &Env) -> Result<(), AllowlistError> {
        if !env.storage().instance().has(&DataKey::Initialized) {
            return Err(AllowlistError::NotInitialized);
        }
        Ok(())
    }

    /// Assert that `caller` is the registered admin.
    fn require_admin(env: &Env, caller: &Address) -> Result<(), AllowlistError> {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(AllowlistError::NotInitialized)?;
        if caller != &stored {
            return Err(AllowlistError::Unauthorized);
        }
        Ok(())
    }

    /// Load an allowlist's address vector from storage.
    ///
    /// # Errors
    /// - [`AllowlistError::AllowlistNotFound`] if the allowlist does not exist.
    fn load_allowlist(env: &Env, allowlist_id: &Symbol) -> Result<Vec<Address>, AllowlistError> {
        let key = allowlist_storage_key(env, allowlist_id);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(AllowlistError::AllowlistNotFound)
    }

    /// Persist an allowlist's address vector and extend its TTL.
    fn save_allowlist(env: &Env, allowlist_id: &Symbol, addrs: &Vec<Address>) {
        let key = allowlist_storage_key(env, allowlist_id);
        env.storage().persistent().set(&key, addrs);
        // Extend TTL so the allowlist record does not expire prematurely.
        env.storage()
            .persistent()
            .extend_ttl(&key, ALLOWLIST_TTL_THRESHOLD, ALLOWLIST_TTL_TO);
    }
}
