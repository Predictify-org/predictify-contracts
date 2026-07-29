//! Per-account state limits for token-adjacent records.
//!
//! This module bounds the number of bets, positions, and subscriptions that a
//! single account can keep in contract storage. Counts are derived directly
//! from authenticated, bounded membership maps, so a caller cannot bypass a
//! cap by replaying an add or fabricating a removal.

use soroban_sdk::{contracterror, contracttype, Address, BytesN, Env, Map};

/// Maximum value accepted for any configurable per-account category cap.
///
/// This hard ceiling prevents an administrator mistake from turning a bounded
/// category into effectively unbounded per-account storage while keeping each
/// encoded membership map at a predictable size.
pub const MAX_CONFIGURABLE_ACCOUNT_LIMIT: u32 = 256;

const STATE_TTL_THRESHOLD: u32 = 100_000;
const STATE_TTL_EXTEND_TO: u32 = 6_307_200;

/// Categories of token-adjacent state that are capped per account.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccountStateKind {
    /// A bet identifier owned by the account.
    Bet,
    /// An open or historical position identifier owned by the account.
    Position,
    /// A subscription identifier owned by the account.
    Subscription,
}

/// Global caps applied independently to each account.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccountLimits {
    /// Maximum number of tracked bet identifiers per account.
    pub bets: u32,
    /// Maximum number of tracked position identifiers per account.
    pub positions: u32,
    /// Maximum number of tracked subscription identifiers per account.
    pub subscriptions: u32,
}

/// Current per-category storage usage for one account.
#[contracttype]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AccountUsage {
    /// Number of tracked bet identifiers.
    pub bets: u32,
    /// Number of tracked position identifiers.
    pub positions: u32,
    /// Number of tracked subscription identifiers.
    pub subscriptions: u32,
}

/// Stable semantic errors returned by token account-limit operations.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TokenLimitError {
    /// The contract has already been initialized.
    AlreadyInitialized = 1,
    /// The contract has not been initialized.
    NotInitialized = 2,
    /// The authenticated address is not the configured administrator.
    Unauthorized = 3,
    /// A configured category cap exceeds the hard maximum.
    InvalidLimit = 4,
    /// The exact `(account, kind, item_id)` tuple is already tracked.
    ItemAlreadyTracked = 5,
    /// The exact `(account, kind, item_id)` tuple is not tracked.
    ItemNotFound = 6,
    /// The account has reached its configured bet cap.
    BetLimitExceeded = 7,
    /// The account has reached its configured position cap.
    PositionLimitExceeded = 8,
    /// The account has reached its configured subscription cap.
    SubscriptionLimitExceeded = 9,
    /// A checked counter increment overflowed.
    Overflow = 10,
    /// A checked membership-length decrement underflowed.
    Underflow = 11,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum DataKey {
    Initialized,
    Admin,
    AccountLimits,
    AccountItems(Address, AccountStateKind),
}

pub(crate) fn initialize(
    env: &Env,
    admin: &Address,
    account_limits: &AccountLimits,
) -> Result<(), TokenLimitError> {
    if env.storage().instance().has(&DataKey::Initialized) {
        return Err(TokenLimitError::AlreadyInitialized);
    }

    validate_limits(account_limits)?;

    env.storage().instance().set(&DataKey::Admin, admin);
    env.storage()
        .instance()
        .set(&DataKey::AccountLimits, account_limits);
    env.storage().instance().set(&DataKey::Initialized, &true);

    Ok(())
}

pub(crate) fn set_account_limits(
    env: &Env,
    admin: &Address,
    account_limits: &AccountLimits,
) -> Result<(), TokenLimitError> {
    require_initialized(env)?;
    require_admin(env, admin)?;
    validate_limits(account_limits)?;

    env.storage()
        .instance()
        .set(&DataKey::AccountLimits, account_limits);

    Ok(())
}

pub(crate) fn track_account_item(
    env: &Env,
    account: &Address,
    kind: &AccountStateKind,
    item_id: &BytesN<32>,
) -> Result<AccountUsage, TokenLimitError> {
    require_initialized(env)?;

    let mut items = get_account_items(env, account, kind);
    if items.contains_key(item_id.clone()) {
        return Err(TokenLimitError::ItemAlreadyTracked);
    }

    let account_limits = get_account_limits(env)?;
    let next_count = increment_count(items.len())?;
    enforce_limit(kind, next_count, limit_for_kind(&account_limits, kind))?;

    items.set(item_id.clone(), true);
    set_account_items(env, account, kind, &items);

    Ok(get_account_usage(env, account))
}

pub(crate) fn untrack_account_item(
    env: &Env,
    account: &Address,
    kind: &AccountStateKind,
    item_id: &BytesN<32>,
) -> Result<AccountUsage, TokenLimitError> {
    require_initialized(env)?;

    let mut items = get_account_items(env, account, kind);
    if !items.contains_key(item_id.clone()) {
        return Err(TokenLimitError::ItemNotFound);
    }

    decrement_count(items.len())?;
    items.remove(item_id.clone());

    let items_key = account_items_key(account, kind);
    if items.is_empty() {
        env.storage().persistent().remove(&items_key);
    } else {
        set_account_items(env, account, kind, &items);
    }

    Ok(get_account_usage(env, account))
}

pub(crate) fn get_account_limits(env: &Env) -> Result<AccountLimits, TokenLimitError> {
    env.storage()
        .instance()
        .get(&DataKey::AccountLimits)
        .ok_or(TokenLimitError::NotInitialized)
}

pub(crate) fn get_account_usage(env: &Env, account: &Address) -> AccountUsage {
    AccountUsage {
        bets: get_account_items(env, account, &AccountStateKind::Bet).len(),
        positions: get_account_items(env, account, &AccountStateKind::Position).len(),
        subscriptions: get_account_items(env, account, &AccountStateKind::Subscription).len(),
    }
}

pub(crate) fn get_remaining_capacity(
    env: &Env,
    account: &Address,
) -> Result<AccountLimits, TokenLimitError> {
    let account_limits = get_account_limits(env)?;
    let usage = get_account_usage(env, account);

    Ok(AccountLimits {
        bets: account_limits.bets.saturating_sub(usage.bets),
        positions: account_limits.positions.saturating_sub(usage.positions),
        subscriptions: account_limits
            .subscriptions
            .saturating_sub(usage.subscriptions),
    })
}

pub(crate) fn is_account_item_tracked(
    env: &Env,
    account: &Address,
    kind: &AccountStateKind,
    item_id: &BytesN<32>,
) -> bool {
    get_account_items(env, account, kind).contains_key(item_id.clone())
}

pub(crate) fn get_admin(env: &Env) -> Result<Address, TokenLimitError> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(TokenLimitError::NotInitialized)
}

fn require_initialized(env: &Env) -> Result<(), TokenLimitError> {
    if env
        .storage()
        .instance()
        .get::<DataKey, bool>(&DataKey::Initialized)
        .unwrap_or(false)
    {
        Ok(())
    } else {
        Err(TokenLimitError::NotInitialized)
    }
}

fn require_admin(env: &Env, admin: &Address) -> Result<(), TokenLimitError> {
    let stored_admin = get_admin(env)?;
    if stored_admin == *admin {
        Ok(())
    } else {
        Err(TokenLimitError::Unauthorized)
    }
}

fn validate_limits(account_limits: &AccountLimits) -> Result<(), TokenLimitError> {
    if account_limits.bets > MAX_CONFIGURABLE_ACCOUNT_LIMIT
        || account_limits.positions > MAX_CONFIGURABLE_ACCOUNT_LIMIT
        || account_limits.subscriptions > MAX_CONFIGURABLE_ACCOUNT_LIMIT
    {
        return Err(TokenLimitError::InvalidLimit);
    }

    Ok(())
}

fn account_items_key(account: &Address, kind: &AccountStateKind) -> DataKey {
    DataKey::AccountItems(account.clone(), *kind)
}

fn get_account_items(
    env: &Env,
    account: &Address,
    kind: &AccountStateKind,
) -> Map<BytesN<32>, bool> {
    env.storage()
        .persistent()
        .get(&account_items_key(account, kind))
        .unwrap_or_else(|| Map::new(env))
}

fn set_account_items(
    env: &Env,
    account: &Address,
    kind: &AccountStateKind,
    items: &Map<BytesN<32>, bool>,
) {
    let key = account_items_key(account, kind);
    env.storage().persistent().set(&key, items);
    bump_persistent_ttl(env, &key);
}

fn limit_for_kind(account_limits: &AccountLimits, kind: &AccountStateKind) -> u32 {
    match kind {
        AccountStateKind::Bet => account_limits.bets,
        AccountStateKind::Position => account_limits.positions,
        AccountStateKind::Subscription => account_limits.subscriptions,
    }
}

fn enforce_limit(
    kind: &AccountStateKind,
    next_count: u32,
    limit: u32,
) -> Result<(), TokenLimitError> {
    if next_count <= limit {
        return Ok(());
    }

    match kind {
        AccountStateKind::Bet => Err(TokenLimitError::BetLimitExceeded),
        AccountStateKind::Position => Err(TokenLimitError::PositionLimitExceeded),
        AccountStateKind::Subscription => Err(TokenLimitError::SubscriptionLimitExceeded),
    }
}

fn increment_count(current: u32) -> Result<u32, TokenLimitError> {
    current.checked_add(1).ok_or(TokenLimitError::Overflow)
}

fn decrement_count(current: u32) -> Result<u32, TokenLimitError> {
    current.checked_sub(1).ok_or(TokenLimitError::Underflow)
}

fn bump_persistent_ttl(env: &Env, key: &DataKey) {
    let extend_to = STATE_TTL_EXTEND_TO.min(env.storage().max_ttl());
    let threshold = STATE_TTL_THRESHOLD.min(extend_to);
    env.storage()
        .persistent()
        .extend_ttl(key, threshold, extend_to);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_counter_math_reports_overflow_and_underflow() {
        assert_eq!(increment_count(u32::MAX), Err(TokenLimitError::Overflow));
        assert_eq!(decrement_count(0), Err(TokenLimitError::Underflow));
    }

    #[test]
    fn every_kind_maps_to_its_semantic_limit_error() {
        assert_eq!(
            enforce_limit(&AccountStateKind::Bet, 2, 1),
            Err(TokenLimitError::BetLimitExceeded)
        );
        assert_eq!(
            enforce_limit(&AccountStateKind::Position, 2, 1),
            Err(TokenLimitError::PositionLimitExceeded)
        );
        assert_eq!(
            enforce_limit(&AccountStateKind::Subscription, 2, 1),
            Err(TokenLimitError::SubscriptionLimitExceeded)
        );
    }

    #[test]
    fn error_discriminants_are_stable() {
        assert_eq!(TokenLimitError::AlreadyInitialized as u32, 1);
        assert_eq!(TokenLimitError::NotInitialized as u32, 2);
        assert_eq!(TokenLimitError::Unauthorized as u32, 3);
        assert_eq!(TokenLimitError::InvalidLimit as u32, 4);
        assert_eq!(TokenLimitError::ItemAlreadyTracked as u32, 5);
        assert_eq!(TokenLimitError::ItemNotFound as u32, 6);
        assert_eq!(TokenLimitError::BetLimitExceeded as u32, 7);
        assert_eq!(TokenLimitError::PositionLimitExceeded as u32, 8);
        assert_eq!(TokenLimitError::SubscriptionLimitExceeded as u32, 9);
        assert_eq!(TokenLimitError::Overflow as u32, 10);
        assert_eq!(TokenLimitError::Underflow as u32, 11);
    }
}
