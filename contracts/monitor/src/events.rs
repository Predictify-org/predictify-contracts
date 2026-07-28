//! Structured lifecycle events for the Monitor contract.
//!
//! Every observable state transition emits a typed event with a stable,
//! ≤9-character [`Symbol`] topic so that off-chain indexers can subscribe,
//! filter, and replay monitor state transitions deterministically.
//!
//! # Event Summary
//!
//! | Topic              | Emitted When                         |
//! |--------------------|--------------------------------------|
//! | `mon_init`         | Contract initialized                 |
//! | `mon_caps_set`     | Per-account caps updated             |
//! | `mon_bet_rec`      | A bet recorded for an account        |
//! | `mon_bet_rem`      | A bet removed for an account         |
//! | `mon_pos_rec`      | A position recorded for an account   |
//! | `mon_pos_rem`      | A position removed for an account    |
//! | `mon_sub_rec`      | A subscription recorded              |
//! | `mon_sub_rem`      | A subscription removed               |

use soroban_sdk::{Address, Env, Symbol};

// ---------------------------------------------------------------------------
// Initialization
// ---------------------------------------------------------------------------

/// Emit a `mon_init` event when the contract is initialized.
///
/// Published by [`crate::MonitorContract::initialize`].
pub fn emit_initialized(env: &Env, admin: &Address) {
    env.events().publish(
        (Symbol::new(env, "mon_init"), admin),
        env.ledger().timestamp(),
    );
}

// ---------------------------------------------------------------------------
// Cap management
// ---------------------------------------------------------------------------

/// Emit a `mon_caps_set` event when per-account caps are updated.
///
/// - `cap_type`: a short string identifying which cap was changed
///   (`"bets"`, `"positions"`, or `"subscriptions"`).
/// - `new_max`: the newly applied maximum.
///
/// Published by [`crate::MonitorContract::set_caps`].
pub fn emit_caps_set(env: &Env, admin: &Address, cap_type: &Symbol, new_max: u32) {
    env.events().publish(
        (Symbol::new(env, "mon_caps_set"), admin, cap_type),
        (new_max, env.ledger().timestamp()),
    );
}

// ---------------------------------------------------------------------------
// Bet events
// ---------------------------------------------------------------------------

/// Emit a `mon_bet_rec` event when a bet is recorded for `user`.
///
/// - `new_count`: the account's bet count after the increment.
///
/// Published by [`crate::MonitorContract::record_bet`].
pub fn emit_bet_recorded(env: &Env, user: &Address, new_count: u32) {
    env.events().publish(
        (Symbol::new(env, "mon_bet_rec"), user),
        (new_count, env.ledger().timestamp()),
    );
}

/// Emit a `mon_bet_rem` event when a bet is removed for `user`.
///
/// - `new_count`: the account's bet count after the decrement.
///
/// Published by [`crate::MonitorContract::remove_bet`].
pub fn emit_bet_removed(env: &Env, user: &Address, new_count: u32) {
    env.events().publish(
        (Symbol::new(env, "mon_bet_rem"), user),
        (new_count, env.ledger().timestamp()),
    );
}

// ---------------------------------------------------------------------------
// Position events
// ---------------------------------------------------------------------------

/// Emit a `mon_pos_rec` event when a position is recorded for `user`.
///
/// - `new_count`: the account's position count after the increment.
///
/// Published by [`crate::MonitorContract::record_position`].
pub fn emit_position_recorded(env: &Env, user: &Address, new_count: u32) {
    env.events().publish(
        (Symbol::new(env, "mon_pos_rec"), user),
        (new_count, env.ledger().timestamp()),
    );
}

/// Emit a `mon_pos_rem` event when a position is removed for `user`.
///
/// - `new_count`: the account's position count after the decrement.
///
/// Published by [`crate::MonitorContract::remove_position`].
pub fn emit_position_removed(env: &Env, user: &Address, new_count: u32) {
    env.events().publish(
        (Symbol::new(env, "mon_pos_rem"), user),
        (new_count, env.ledger().timestamp()),
    );
}

// ---------------------------------------------------------------------------
// Subscription events
// ---------------------------------------------------------------------------

/// Emit a `mon_sub_rec` event when a subscription is recorded for `user`.
///
/// - `new_count`: the account's subscription count after the increment.
///
/// Published by [`crate::MonitorContract::record_subscription`].
pub fn emit_subscription_recorded(env: &Env, user: &Address, new_count: u32) {
    env.events().publish(
        (Symbol::new(env, "mon_sub_rec"), user),
        (new_count, env.ledger().timestamp()),
    );
}

/// Emit a `mon_sub_rem` event when a subscription is removed for `user`.
///
/// - `new_count`: the account's subscription count after the decrement.
///
/// Published by [`crate::MonitorContract::remove_subscription`].
pub fn emit_subscription_removed(env: &Env, user: &Address, new_count: u32) {
    env.events().publish(
        (Symbol::new(env, "mon_sub_rem"), user),
        (new_count, env.ledger().timestamp()),
    );
}
