//! Structured lifecycle events for the Allowlist contract.
//!
//! Each lifecycle operation emits a typed event with a stable topic symbol so
//! that off-chain indexers can filter, decode, and replay allowlist state
//! transitions deterministically.
//!
//! # Event Summary
//!
//! | Topic Symbol     | Emitted When                |
//! |------------------|-----------------------------|
//! | `alist_created`  | A new allowlist is created  |
//! | `alist_addr_add` | An address is added         |
//! | `alist_addr_rem` | An address is removed       |
//! | `alist_cleared`  | All addresses are removed   |
//! | `alist_deleted`  | An allowlist is deleted     |
//! | `alist_owner_xf` | Ownership is transferred    |
//! | `alist_init`     | Contract is initialized     |

use soroban_sdk::{Address, Env, Symbol, Vec};

/// Emit an `AllowlistCreated` event.
///
/// Published when a new allowlist is created via
/// [`crate::AllowlistContract::create_allowlist`].
pub fn emit_allowlist_created(env: &Env, admin: &Address, allowlist_id: &Symbol) {
    let topics = (
        Symbol::new(env, "alist_created"),
        admin,
        allowlist_id,
    );
    env.events().publish(topics, env.ledger().timestamp());
}

/// Emit an `AllowlistAddressAdded` event.
///
/// Published when an address is added to an allowlist via
/// [`crate::AllowlistContract::add_address`] or
/// [`crate::AllowlistContract::add_addresses`].
pub fn emit_allowlist_address_added(
    env: &Env,
    admin: &Address,
    allowlist_id: &Symbol,
    address: &Address,
) {
    let topics = (
        Symbol::new(env, "alist_addr_add"),
        admin,
        allowlist_id,
        address,
    );
    env.events().publish(topics, env.ledger().timestamp());
}

/// Emit an `AllowlistAddressRemoved` event.
///
/// Published when an address is removed from an allowlist via
/// [`crate::AllowlistContract::remove_address`] or
/// [`crate::AllowlistContract::remove_addresses`].
pub fn emit_allowlist_address_removed(
    env: &Env,
    admin: &Address,
    allowlist_id: &Symbol,
    address: &Address,
) {
    let topics = (
        Symbol::new(env, "alist_addr_rem"),
        admin,
        allowlist_id,
        address,
    );
    env.events().publish(topics, env.ledger().timestamp());
}

/// Emit an `AllowlistCleared` event.
///
/// Published when all addresses are removed from an allowlist via
/// [`crate::AllowlistContract::clear_allowlist`].
///
/// The `removed_count` is the number of addresses that were removed during
/// the clear operation.
pub fn emit_allowlist_cleared(
    env: &Env,
    admin: &Address,
    allowlist_id: &Symbol,
    removed_count: u32,
) {
    let topics = (
        Symbol::new(env, "alist_cleared"),
        admin,
        allowlist_id,
    );
    env.events()
        .publish(topics, (removed_count, env.ledger().timestamp()));
}

/// Emit an `AllowlistDeleted` event.
///
/// Published when an entire allowlist is removed via
/// [`crate::AllowlistContract::delete_allowlist`].
///
/// The `removed_count` is the number of addresses that were in the allowlist
/// at the time of deletion.
pub fn emit_allowlist_deleted(
    env: &Env,
    admin: &Address,
    allowlist_id: &Symbol,
    removed_count: u32,
) {
    let topics = (
        Symbol::new(env, "alist_deleted"),
        admin,
        allowlist_id,
    );
    env.events()
        .publish(topics, (removed_count, env.ledger().timestamp()));
}

/// Emit an `AllowlistOwnershipTransferred` event.
///
/// Published when contract ownership is transferred via
/// [`crate::AllowlistContract::transfer_ownership`].
pub fn emit_allowlist_ownership_transferred(
    env: &Env,
    previous_admin: &Address,
    new_admin: &Address,
) {
    let topics = (
        Symbol::new(env, "alist_owner_xf"),
        previous_admin,
        new_admin,
    );
    env.events().publish(topics, env.ledger().timestamp());
}

/// Emit an `AllowlistInitialized` event.
///
/// Published when the contract is initialized via
/// [`crate::AllowlistContract::initialize`].
pub fn emit_allowlist_initialized(env: &Env, admin: &Address) {
    let topics = (
        Symbol::new(env, "alist_init"),
        admin,
    );
    env.events().publish(topics, env.ledger().timestamp());
}
