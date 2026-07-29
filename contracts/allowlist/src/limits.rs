//! Per-account allowlist-membership limits.
//!
//! Every address may belong to only a bounded number of allowlists. The
//! configured cap applies independently to each address and is checked before
//! either single-address or batch additions mutate storage.
//!
//! Membership counts are cached in persistent storage. If a cache entry has
//! expired (or the contract was upgraded from a version without counters), the
//! count is rebuilt from the authoritative allowlist registry before it is
//! used. This prevents an expired counter from bypassing enforcement.

use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

use crate::{allowlist_storage_key, AllowlistError, DataKey};

/// Default maximum number of allowlists that may contain one address.
pub const DEFAULT_MAX_MEMBERSHIPS_PER_ACCOUNT: u32 = 50;

/// Hard upper bound for the configurable per-account membership cap.
///
/// The bound prevents an administrator from accidentally making the limit
/// ineffective while still allowing deployments to choose a suitable cap.
pub const MAX_CONFIGURABLE_ACCOUNT_LIMIT: u32 = 256;

const ACCOUNT_COUNT_TTL_THRESHOLD: u32 = 100_000;
const ACCOUNT_COUNT_TTL_TO: u32 = 518_400;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum LimitDataKey {
    MaxMembershipsPerAccount,
    AccountMembershipCount(Address),
}

/// Store the initial default cap during contract initialization.
pub(crate) fn initialize(env: &Env) {
    env.storage().instance().set(
        &LimitDataKey::MaxMembershipsPerAccount,
        &DEFAULT_MAX_MEMBERSHIPS_PER_ACCOUNT,
    );
}

/// Replace the cap applied independently to every account.
///
/// A zero cap is valid and disables new memberships without deleting existing
/// ones. Existing accounts above a lowered cap retain their memberships but
/// cannot be added to another allowlist until their usage falls below the cap.
pub(crate) fn set_account_limit(env: &Env, max_memberships: u32) -> Result<(), AllowlistError> {
    if max_memberships > MAX_CONFIGURABLE_ACCOUNT_LIMIT {
        return Err(AllowlistError::InvalidInput);
    }

    env.storage()
        .instance()
        .set(&LimitDataKey::MaxMembershipsPerAccount, &max_memberships);
    Ok(())
}

/// Return the configured cap, falling back to the default after an upgrade.
pub(crate) fn get_account_limit(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&LimitDataKey::MaxMembershipsPerAccount)
        .unwrap_or(DEFAULT_MAX_MEMBERSHIPS_PER_ACCOUNT)
}

/// Return the number of allowlists that currently contain `account`.
///
/// A missing cached count is rebuilt from the authoritative registry, which
/// supports upgrades and persistent-entry expiration without weakening the
/// limit.
pub(crate) fn get_account_usage(env: &Env, account: &Address) -> Result<u32, AllowlistError> {
    let key = LimitDataKey::AccountMembershipCount(account.clone());
    match env.storage().persistent().get(&key) {
        Some(count) => Ok(count),
        None => derive_account_usage(env, account),
    }
}

/// Add one membership to an account after enforcing its configured cap.
pub(crate) fn add_membership(env: &Env, account: &Address) -> Result<u32, AllowlistError> {
    let current = get_account_usage(env, account)?;
    let next = current.checked_add(1).ok_or(AllowlistError::Overflow)?;

    if next > get_account_limit(env) {
        return Err(AllowlistError::AccountLimitExceeded);
    }

    save_account_usage(env, account, next);
    Ok(next)
}

/// Remove one membership and release the corresponding account capacity.
pub(crate) fn remove_membership(env: &Env, account: &Address) -> Result<u32, AllowlistError> {
    let current = get_account_usage(env, account)?;
    let next = current.checked_sub(1).ok_or(AllowlistError::Underflow)?;
    let key = LimitDataKey::AccountMembershipCount(account.clone());

    if next == 0 {
        env.storage().persistent().remove(&key);
    } else {
        save_account_usage(env, account, next);
    }

    Ok(next)
}

fn derive_account_usage(env: &Env, account: &Address) -> Result<u32, AllowlistError> {
    let registry: Vec<Symbol> = env
        .storage()
        .persistent()
        .get(&DataKey::AllowlistRegistry)
        .unwrap_or_else(|| Vec::new(env));
    let mut count = 0u32;

    for allowlist_id in registry.iter() {
        let addresses: Option<Vec<Address>> = env
            .storage()
            .persistent()
            .get(&allowlist_storage_key(env, &allowlist_id));
        if addresses
            .map(|members| members.contains(account))
            .unwrap_or(false)
        {
            count = count.checked_add(1).ok_or(AllowlistError::Overflow)?;
        }
    }

    Ok(count)
}

fn save_account_usage(env: &Env, account: &Address, count: u32) {
    let key = LimitDataKey::AccountMembershipCount(account.clone());
    env.storage().persistent().set(&key, &count);

    let extend_to = ACCOUNT_COUNT_TTL_TO.min(env.storage().max_ttl());
    let threshold = ACCOUNT_COUNT_TTL_THRESHOLD.min(extend_to);
    env.storage()
        .persistent()
        .extend_ttl(&key, threshold, extend_to);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configurable_limit_validation_accepts_boundaries() {
        let env = Env::default();

        assert_eq!(set_account_limit(&env, 0), Ok(()));
        assert_eq!(
            set_account_limit(&env, MAX_CONFIGURABLE_ACCOUNT_LIMIT),
            Ok(())
        );
        assert_eq!(
            set_account_limit(&env, MAX_CONFIGURABLE_ACCOUNT_LIMIT + 1),
            Err(AllowlistError::InvalidInput)
        );
    }
}
