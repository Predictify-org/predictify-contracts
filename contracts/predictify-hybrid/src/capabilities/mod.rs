use soroban_sdk::Env;

pub(crate) mod admin;

/// Recovery feature capability flags (u64 bitmap).
///
/// Each bit represents a discrete feature the recovery subsystem supports.
/// Clients can compare the bitmap across contract versions to detect
/// capability deltas (features added or removed after an upgrade).
pub mod recovery {
    use super::*;

    /// Per-market timelock before a recovery action can be executed.
    pub const TIMELOCK_GUARD: u64 = 1 << 0;
    /// Admin-initiated market state reconstruction (fix total_staked mismatches, etc.).
    pub const STATE_RECONSTRUCTION: u64 = 1 << 1;
    /// Admin-initiated market cancellation with full stake refund.
    pub const CANCEL_MARKET: u64 = 1 << 2;
    /// Admin-initiated force-resolve for stuck ended markets.
    pub const FORCE_RESOLVE: u64 = 1 << 3;
    /// Partial refund mechanism for selected users.
    pub const PARTIAL_REFUND: u64 = 1 << 4;
    /// Read-only dry-run that analyses recoverability without side effects.
    pub const DRY_RUN: u64 = 1 << 5;
    /// Recovery status query per market.
    pub const STATUS_QUERY: u64 = 1 << 6;
    /// Recovery history with per-market capped retention.
    pub const HISTORY: u64 = 1 << 7;
    /// Admin pruning of completed recovery history.
    pub const PRUNE_HISTORY: u64 = 1 << 8;
    /// Unclaimed winnings policy (configurable claim periods, treasury address).
    pub const UNCLAIMED_WINNINGS_POLICY: u64 = 1 << 9;
    /// Recovery integrity validator for market state.
    pub const INTEGRITY_VALIDATOR: u64 = 1 << 10;
    /// Combined bitmap of all currently supported recovery features.
    pub const SUPPORTED: u64 = TIMELOCK_GUARD
        | STATE_RECONSTRUCTION
        | CANCEL_MARKET
        | FORCE_RESOLVE
        | PARTIAL_REFUND
        | DRY_RUN
        | STATUS_QUERY
        | HISTORY
        | PRUNE_HISTORY
        | UNCLAIMED_WINNINGS_POLICY
        | INTEGRITY_VALIDATOR;
}

/// Returns the recovery feature bitmap for the current contract version.
///
/// Clients can call this read-only view to discover which recovery
/// capabilities are available and detect changes after contract upgrades.
pub fn capabilities(_env: &Env) -> u64 {
    recovery::SUPPORTED
}

/// Returns the fixed cooldown between repeated capability-critical admin actions.
///
/// The cooldown is enforced independently for contract upgrades and rollbacks.
/// This keeps emergency rollback available after an upgrade while preventing
/// rapid repetition of either critical action.
pub fn admin_cooldown_seconds() -> u64 {
    admin::ADMIN_ACTION_COOLDOWN_SECONDS
}
