//! Structured lifecycle events for the tokens account-state registry.
//!
//! Every event carries a frozen topic [`Symbol`] and a `schema_version: u32`
//! so off-chain indexers can route on `(topic, schema_version)` and detect a
//! deliberate ABI change independently of any new event type being added.

use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

use crate::{AccountLimits, AccountStateKind, AccountUsage};

/// Schema version for every event in this module. Frozen at v1.
pub const TOKENS_EVENT_SCHEMA_VERSION: u32 = 1;

pub const TOPIC_INITIALIZED: Symbol = symbol_short!("tk_init");
pub const TOPIC_LIMITS_SET: Symbol = symbol_short!("tk_lims");
pub const TOPIC_ITEM_TRACKED: Symbol = symbol_short!("tk_trkd");
pub const TOPIC_ITEM_UNTRACKED: Symbol = symbol_short!("tk_untr");

/// Emitted once the contract's admin and initial per-account limits are set.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokensInitializedEvent {
    pub admin: Address,
    pub account_limits: AccountLimits,
    pub schema_version: u32,
}

/// Emitted whenever the global per-account limits are replaced.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccountLimitsSetEvent {
    pub admin: Address,
    pub account_limits: AccountLimits,
    pub schema_version: u32,
}

/// Emitted after an item is successfully tracked for an account.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemTrackedEvent {
    pub account: Address,
    pub kind: AccountStateKind,
    pub usage: AccountUsage,
    pub schema_version: u32,
}

/// Emitted after an item is successfully untracked for an account.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemUntrackedEvent {
    pub account: Address,
    pub kind: AccountStateKind,
    pub usage: AccountUsage,
    pub schema_version: u32,
}

/// Emit a [`TokensInitializedEvent`] under [`TOPIC_INITIALIZED`].
pub fn emit_initialized(env: &Env, admin: &Address, account_limits: &AccountLimits) {
    let event = TokensInitializedEvent {
        admin: admin.clone(),
        account_limits: *account_limits,
        schema_version: TOKENS_EVENT_SCHEMA_VERSION,
    };
    env.events()
        .publish((TOPIC_INITIALIZED, TOKENS_EVENT_SCHEMA_VERSION), event);
}

/// Emit an [`AccountLimitsSetEvent`] under [`TOPIC_LIMITS_SET`].
pub fn emit_limits_set(env: &Env, admin: &Address, account_limits: &AccountLimits) {
    let event = AccountLimitsSetEvent {
        admin: admin.clone(),
        account_limits: *account_limits,
        schema_version: TOKENS_EVENT_SCHEMA_VERSION,
    };
    env.events()
        .publish((TOPIC_LIMITS_SET, TOKENS_EVENT_SCHEMA_VERSION), event);
}

/// Emit an [`ItemTrackedEvent`] under [`TOPIC_ITEM_TRACKED`].
pub fn emit_item_tracked(
    env: &Env,
    account: &Address,
    kind: AccountStateKind,
    usage: AccountUsage,
) {
    let event = ItemTrackedEvent {
        account: account.clone(),
        kind,
        usage,
        schema_version: TOKENS_EVENT_SCHEMA_VERSION,
    };
    env.events()
        .publish((TOPIC_ITEM_TRACKED, TOKENS_EVENT_SCHEMA_VERSION), event);
}

/// Emit an [`ItemUntrackedEvent`] under [`TOPIC_ITEM_UNTRACKED`].
pub fn emit_item_untracked(
    env: &Env,
    account: &Address,
    kind: AccountStateKind,
    usage: AccountUsage,
) {
    let event = ItemUntrackedEvent {
        account: account.clone(),
        kind,
        usage,
        schema_version: TOKENS_EVENT_SCHEMA_VERSION,
    };
    env.events()
        .publish((TOPIC_ITEM_UNTRACKED, TOKENS_EVENT_SCHEMA_VERSION), event);
}
