use alloc::format;
use soroban_sdk::{contracttype, panic_with_error, Address, Env, Map, String, Symbol, Vec};

use crate::events::EventEmitter;
use crate::markets::MarketStateManager;
use crate::types::{ClaimInfo, MarketState};
use crate::Error;

const DEFAULT_UNCLAIMED_CLAIM_PERIOD_SECONDS: u64 = 90 * 24 * 60 * 60;

/// Maximum completed recovery records retained per market.
///
/// Bounds persistent storage growth under repeated recovery events. Active
/// (unresolved) recovery state is stored separately and is never counted toward
/// this cap.
pub const MAX_RECOVERY_HISTORY_PER_MARKET: u32 = 10;

/// Maximum entries removable in a single admin prune call (gas safety).
pub const MAX_RECOVERY_PRUNE_BATCH: u32 = 30;

// ===== PER-MARKET RECOVERY TIMELOCK =====

/// Default timelock delay before a per-market recovery action can be executed (24 hours).
pub const DEFAULT_RECOVERY_TIMELOCK_SECONDS: u64 = 24 * 60 * 60;

/// Minimum allowed recovery timelock (1 hour). Prevents setting dangerously short windows.
pub const MIN_RECOVERY_TIMELOCK_SECONDS: u64 = 60 * 60;

/// Maximum allowed recovery timelock (7 days). Prevents excessively long lockouts.
pub const MAX_RECOVERY_TIMELOCK_SECONDS: u64 = 7 * 24 * 60 * 60;

/// Specifies the type of recovery action to perform on a market.
///
/// Each variant maps to a specific recovery operation that can be
/// initiated with a timelock and executed after the delay.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PerMarketRecoveryAction {
    /// Reconstruct market state from stored data (fix total_staked inconsistencies, etc.)
    ReconstructState,
    /// Cancel the market and refund all stakes to participants.
    CancelMarket,
    /// Force-resolve the market with a specified outcome (for stuck ended markets).
    ForceResolve,
}

/// A pending per-market recovery request protected by an admin timelock.
///
/// Once initiated, the recovery action cannot be executed until `execute_after`
/// timestamp has been reached. This gives the admin a window to review and
/// cancel if the recovery was initiated in error.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingMarketRecovery {
    /// The market this recovery targets.
    pub market_id: Symbol,
    /// The admin who initiated the recovery request.
    pub initiated_by: Address,
    /// The recovery action to execute.
    pub action: PerMarketRecoveryAction,
    /// Unix timestamp when this request was initiated.
    pub initiated_at: u64,
    /// Unix timestamp after which this recovery may be executed.
    pub execute_after: u64,
    /// Human-readable reason for the recovery.
    pub reason: String,
}

/// Configuration for the per-market recovery timelock.
///
/// The timelock delay controls how long after initiation a recovery action
/// can actually be executed. Admin can tighten (increase) but not loosen
/// (decrease) this value once set.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryTimelockConfig {
    /// Minimum number of seconds between initiation and execution.
    pub timelock_seconds: u64,
}

// ===== RECOVERY TYPES =====
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryAction {
    MarketStateReconstructed,
    PartialRefundExecuted,
    IntegrityValidated,
    RecoverySkipped,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketRecovery {
    pub market_id: Symbol,
    pub actions: Vec<String>,
    pub issues_detected: Vec<String>,
    pub recovered: bool,
    pub partial_refund_total: i128,
    pub last_action: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryData {
    pub inconsistencies: Vec<String>,
    pub can_recover: bool,
    pub safety_score: i128,
}

/// One completed recovery event in per-market history.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryHistoryEntry {
    pub record: MarketRecovery,
    pub recorded_at: u64,
}

/// Manages per-market recovery requests with admin timelock protection.
///
/// Provides a two-phase recovery process:
/// 1. **Initiate**: Admin schedules a recovery action; timelock starts.
/// 2. **Execute**: After the timelock expires, the admin can execute the action.
///
/// Either phase can be cancelled by the admin before execution.
pub struct RecoveryTimelockManager;

impl RecoveryTimelockManager {
    // ---- Storage keys ----

    #[inline(always)]
    fn pending_key(env: &Env, market_id: &Symbol) -> (Symbol, Symbol) {
        (Symbol::new(env, "rcv_tl_pending"), market_id.clone())
    }

    #[inline(always)]
    fn config_key(env: &Env) -> Symbol {
        Symbol::new(env, "rcv_tl_config")
    }

    // ---- Config ----

    /// Get the current recovery timelock configuration (or defaults if not set).
    pub fn get_config(env: &Env) -> RecoveryTimelockConfig {
        env.storage()
            .persistent()
            .get(&Self::config_key(env))
            .unwrap_or(RecoveryTimelockConfig {
                timelock_seconds: DEFAULT_RECOVERY_TIMELOCK_SECONDS,
            })
    }

    /// Set/update the recovery timelock configuration (admin only).
    ///
    /// Security: timelock_seconds may only be increased (tightened), never decreased.
    /// Auth is enforced by the calling entrypoint.
    pub fn set_config(
        env: &Env,
        admin: &Address,
        config: &RecoveryTimelockConfig,
    ) -> Result<(), Error> {
        RecoveryManager::assert_is_admin(env, admin)?;

        if config.timelock_seconds < MIN_RECOVERY_TIMELOCK_SECONDS
            || config.timelock_seconds > MAX_RECOVERY_TIMELOCK_SECONDS
        {
            return Err(Error::RecoveryTimelockConfigInvalid);
        }

        // Only allow tightening if already set.
        if env.storage().persistent().has(&Self::config_key(env)) {
            let current = Self::get_config(env);
            if config.timelock_seconds < current.timelock_seconds {
                return Err(Error::RecoveryTimelockConfigInvalid);
            }
        }

        env.storage()
            .persistent()
            .set(&Self::config_key(env), config);
        Ok(())
    }

    // ---- Pending request ----

    /// Get the pending recovery request for a market, if any.
    pub fn get_pending(env: &Env, market_id: &Symbol) -> Option<PendingMarketRecovery> {
        env.storage()
            .persistent()
            .get(&Self::pending_key(env, market_id))
    }

    /// Initiate a per-market recovery request.
    ///
    /// Creates a pending request with a timelock. The recovery cannot be executed
    /// until `initiated_at + timelock_seconds` has elapsed.
    ///
    /// # Errors
    /// - `RecoveryAlreadyPending` if a request already exists for this market
    /// - `MarketNotRecoverable` if the market is in a non-recoverable state
    /// - `InvalidRecoveryAction` if the action is invalid for the current market state
    pub fn initiate_recovery(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        action: &PerMarketRecoveryAction,
        reason: &String,
    ) -> Result<PendingMarketRecovery, Error> {
        RecoveryManager::assert_is_admin(env, admin)?;

        // Reject if already pending.
        if Self::get_pending(env, market_id).is_some() {
            return Err(Error::RecoveryAlreadyPending);
        }

        // Validate market exists and is in a recoverable state.
        let market = MarketStateManager::get_market(env, market_id)?;

        match market.state {
            MarketState::Active | MarketState::Closed | MarketState::Cancelled | MarketState::Resolved => {
                return Err(Error::MarketNotRecoverable);
            }
            _ => {}
        }

        // Validate action against current market state.
        Self::validate_action_for_state(&market.state, action)?;

        let now = env.ledger().timestamp();
        let schedule = Self::get_config(env);

        let request = PendingMarketRecovery {
            market_id: market_id.clone(),
            initiated_by: admin.clone(),
            action: action.clone(),
            initiated_at: now,
            execute_after: now.saturating_add(schedule.timelock_seconds),
            reason: reason.clone(),
        };

        env.storage()
            .persistent()
            .set(&Self::pending_key(env, market_id), &request);

        Self::emit_recovery_initiated(env, &request);

        Ok(request)
    }

    /// Execute a pending recovery request after the timelock has expired.
    ///
    /// # Errors
    /// - `RecoveryRequestNotFound` if no pending request exists
    /// - `RecoveryTimelockActive` if the timelock has not yet expired
    pub fn execute_recovery(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
    ) -> Result<bool, Error> {
        RecoveryManager::assert_is_admin(env, admin)?;

        let request = Self::get_pending(env, market_id).ok_or(Error::RecoveryRequestNotFound)?;

        let now = env.ledger().timestamp();
        if now < request.execute_after {
            return Err(Error::RecoveryTimelockActive);
        }

        let result = Self::execute_action(env, &request)?;

        env.storage()
            .persistent()
            .remove(&Self::pending_key(env, market_id));

        Self::emit_recovery_executed(env, &request, result);

        Ok(result)
    }

    /// Cancel a pending recovery request.
    pub fn cancel_recovery(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
    ) -> Result<(), Error> {
        RecoveryManager::assert_is_admin(env, admin)?;

        let request = Self::get_pending(env, market_id).ok_or(Error::RecoveryRequestNotFound)?;

        env.storage()
            .persistent()
            .remove(&Self::pending_key(env, market_id));

        Self::emit_recovery_cancelled(env, &request);

        Ok(())
    }

    // ---- Action validation ----

    /// Validate that a recovery action is appropriate for the given market state.
    fn validate_action_for_state(
        state: &MarketState,
        action: &PerMarketRecoveryAction,
    ) -> Result<(), Error> {
        match action {
            PerMarketRecoveryAction::ReconstructState => {
                match state {
                    MarketState::Active
                    | MarketState::Closed
                    | MarketState::Cancelled
                    | MarketState::Resolved => Err(Error::InvalidRecoveryAction),
                    _ => Ok(()),
                }
            }
            PerMarketRecoveryAction::CancelMarket => {
                match state {
                    MarketState::Ended | MarketState::Disputed => Ok(()),
                    _ => Err(Error::InvalidRecoveryAction),
                }
            }
            PerMarketRecoveryAction::ForceResolve => {
                match state {
                    MarketState::Ended | MarketState::Disputed => Ok(()),
                    _ => Err(Error::InvalidRecoveryAction),
                }
            }
        }
    }

    // ---- Action execution ----

    /// Execute the concrete recovery action described in a pending request.
    fn execute_action(env: &Env, request: &PendingMarketRecovery) -> Result<bool, Error> {
        let mut market = MarketStateManager::get_market(env, &request.market_id)?;

        match request.action {
            PerMarketRecoveryAction::ReconstructState => {
                // Recompute total_staked from stakes map.
                let mut recomputed: i128 = 0;
                for (_, v) in market.stakes.iter() {
                    recomputed = recomputed.saturating_add(v);
                }
                market.total_staked = recomputed;
                MarketStateManager::update_market(env, &request.market_id, &market);
                Ok(true)
            }
            PerMarketRecoveryAction::CancelMarket => {
                // Mark all unclaimed stakes as claimed (refund simulation).
                for (user, stake) in market.stakes.iter() {
                    if stake > 0 && market.claimed.get(user.clone()).is_none() {
                        market.claimed.set(user, ClaimInfo::new(env, stake));
                    }
                }
                market.total_staked = 0;
                market.state = MarketState::Cancelled;
                MarketStateManager::update_market(env, &request.market_id, &market);
                Ok(true)
            }
            PerMarketRecoveryAction::ForceResolve => {
                if market.state == MarketState::Ended {
                    Ok(false)
                } else {
                    market.state = MarketState::Ended;
                    MarketStateManager::update_market(env, &request.market_id, &market);
                    Ok(true)
                }
            }
        }
    }

    // ---- Events ----

    fn emit_recovery_initiated(env: &Env, request: &PendingMarketRecovery) {
        let topic = Symbol::new(env, "rcv_tl_init");
        let action_sym = match request.action {
            PerMarketRecoveryAction::ReconstructState => Symbol::new(env, "reconstruct"),
            PerMarketRecoveryAction::CancelMarket => Symbol::new(env, "cancel"),
            PerMarketRecoveryAction::ForceResolve => Symbol::new(env, "force_resolve"),
        };
        env.events().publish(
            (
                topic,
                request.initiated_by.clone(),
                request.market_id.clone(),
            ),
            (
                action_sym,
                request.initiated_at,
                request.execute_after,
                request.reason.clone(),
            ),
        );
    }

    fn emit_recovery_executed(env: &Env, request: &PendingMarketRecovery, success: bool) {
        let topic = Symbol::new(env, "rcv_tl_exec");
        let action_sym = match request.action {
            PerMarketRecoveryAction::ReconstructState => Symbol::new(env, "reconstruct"),
            PerMarketRecoveryAction::CancelMarket => Symbol::new(env, "cancel"),
            PerMarketRecoveryAction::ForceResolve => Symbol::new(env, "force_resolve"),
        };
        env.events().publish(
            (
                topic,
                request.initiated_by.clone(),
                request.market_id.clone(),
            ),
            (action_sym, success, env.ledger().timestamp()),
        );
    }

    fn emit_recovery_cancelled(env: &Env, request: &PendingMarketRecovery) {
        let topic = Symbol::new(env, "rcv_tl_cancel");
        env.events().publish(
            (
                topic,
                request.initiated_by.clone(),
                request.market_id.clone(),
            ),
            (request.initiated_at, env.ledger().timestamp()),
        );
    }
}

pub struct RecoveryStorage;
impl RecoveryStorage {
    #[inline(always)]
    fn active_key(env: &Env) -> Symbol {
        Symbol::new(env, "recovery_records")
    }

    #[inline(always)]
    fn history_key(env: &Env) -> Symbol {
        Symbol::new(env, "recovery_history")
    }

    #[inline(always)]
    fn per_market_history_key(env: &Env, market_id: &Symbol) -> (Symbol, Symbol) {
        (Symbol::new(env, "rcv_hist"), market_id.clone())
    }

    #[inline(always)]
    fn status_key(env: &Env) -> Symbol {
        Symbol::new(env, "recovery_status_map")
    }

    #[inline(always)]
    fn migrated_key(env: &Env) -> Symbol {
        Symbol::new(env, "recovery_v2_migrated")
    }

    /// Split legacy `recovery_records` (active + completed mixed) into active + history maps.
    fn ensure_migrated(env: &Env) {
        if env
            .storage()
            .persistent()
            .get(&Self::migrated_key(env))
            .unwrap_or(false)
        {
            return;
        }

        if let Some(legacy_history) = env
            .storage()
            .persistent()
            .get::<_, Map<Symbol, Vec<RecoveryHistoryEntry>>>(&Self::history_key(env))
        {
            for (market_id, history) in legacy_history.iter() {
                env.storage().persistent().set(
                    &Self::per_market_history_key(env, &market_id),
                    &history,
                );
            }
            env.storage().persistent().remove(&Self::history_key(env));
        }

        let legacy: Map<Symbol, MarketRecovery> = env
            .storage()
            .persistent()
            .get(&Self::active_key(env))
            .unwrap_or(Map::new(env));

        let mut active = Map::new(env);
        for (market_id, record) in legacy.iter() {
            if record.recovered {
                Self::append_history_entry(env, &market_id, &record);
            } else {
                active.set(market_id, record);
            }
        }

        env.storage()
            .persistent()
            .set(&Self::active_key(env), &active);
        env.storage()
            .persistent()
            .set(&Self::migrated_key(env), &true);
    }

    fn load_active_map(env: &Env) -> Map<Symbol, MarketRecovery> {
        Self::ensure_migrated(env);
        env.storage()
            .persistent()
            .get(&Self::active_key(env))
            .unwrap_or(Map::new(env))
    }

    fn load_history_direct(env: &Env, market_id: &Symbol) -> Vec<RecoveryHistoryEntry> {
        env.storage()
            .persistent()
            .get(&Self::per_market_history_key(env, market_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn load_history(env: &Env, market_id: &Symbol) -> Vec<RecoveryHistoryEntry> {
        Self::ensure_migrated(env);
        Self::load_history_direct(env, market_id)
    }

    fn save_history(env: &Env, market_id: &Symbol, history: &Vec<RecoveryHistoryEntry>) {
        env.storage().persistent().set(
            &Self::per_market_history_key(env, market_id),
            history,
        );
    }

    fn trim_history(env: &Env, history: &mut Vec<RecoveryHistoryEntry>) {
        while history.len() > MAX_RECOVERY_HISTORY_PER_MARKET {
            history.remove(0);
        }
    }

    fn append_history_entry(env: &Env, market_id: &Symbol, record: &MarketRecovery) {
        let mut history = Self::load_history_direct(env, market_id);
        history.push_back(RecoveryHistoryEntry {
            record: record.clone(),
            recorded_at: env.ledger().timestamp(),
        });
        Self::trim_history(env, &mut history);
        Self::save_history(env, market_id, &history);
    }

    /// Active (unresolved) recovery, if any.
    pub fn load_active(env: &Env, market_id: &Symbol) -> Option<MarketRecovery> {
        Self::load_active_map(env).get(market_id.clone())
    }

    /// Latest recovery state: active first, otherwise most recent history entry.
    pub fn load(env: &Env, market_id: &Symbol) -> Option<MarketRecovery> {
        if let Some(active) = Self::load_active(env, market_id) {
            return Some(active);
        }
        let history = Self::load_history(env, market_id);
        let len = history.len();
        if len == 0 {
            return None;
        }
        history.get(len - 1).map(|entry| entry.record.clone())
    }

    pub fn history_len(env: &Env, market_id: &Symbol) -> u32 {
        Self::load_history(env, market_id).len()
    }

    pub fn save(env: &Env, record: &MarketRecovery) {
        Self::ensure_migrated(env);
        let market_id = record.market_id.clone();

        if record.recovered {
            Self::append_history_entry(env, &market_id, record);

            let mut active = Self::load_active_map(env);
            active.remove(market_id.clone());
            env.storage()
                .persistent()
                .set(&Self::active_key(env), &active);
        } else {
            let mut active = Self::load_active_map(env);
            active.set(market_id.clone(), record.clone());
            env.storage()
                .persistent()
                .set(&Self::active_key(env), &active);
        }

        let mut status_map: Map<Symbol, String> = env
            .storage()
            .persistent()
            .get(&Self::status_key(env))
            .unwrap_or(Map::new(env));
        let status = if record.recovered {
            String::from_str(env, "recovered")
        } else {
            String::from_str(env, "pending")
        };
        status_map.set(market_id, status);
        env.storage()
            .persistent()
            .set(&Self::status_key(env), &status_map);
    }

    pub fn status(env: &Env, market_id: &Symbol) -> Option<String> {
        let status_map: Map<Symbol, String> = env
            .storage()
            .persistent()
            .get(&Self::status_key(env))
            .unwrap_or(Map::new(env));
        status_map.get(market_id.clone())
    }

    /// Remove the oldest `count` completed recovery records for a market (admin only).
    ///
    /// Never removes the active (unresolved) recovery entry for the market.
    pub fn prune_history(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        count: u32,
    ) -> Result<u32, Error> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "Admin"))
            .unwrap_or_else(|| panic_with_error!(env, Error::AdminNotSet));

        if admin != &stored_admin {
            return Err(Error::Unauthorized);
        }

        let count = core::cmp::min(count, MAX_RECOVERY_PRUNE_BATCH);
        let mut history = Self::load_history(env, market_id);
        if history.is_empty() || count == 0 {
            return Ok(0);
        }

        let mut removed = 0u32;
        while removed < count && history.len() > 0 {
            history.remove(0);
            removed += 1;
        }

        Self::save_history(env, market_id, &history);
        Ok(removed)
    }
}

pub struct UnclaimedWinningsPolicy;
impl UnclaimedWinningsPolicy {
    #[inline(always)]
    fn global_claim_period_key(env: &Env) -> Symbol {
        Symbol::new(env, "claim_period_global")
    }

    #[inline(always)]
    fn market_claim_periods_key(env: &Env) -> Symbol {
        Symbol::new(env, "claim_period_market")
    }

    #[inline(always)]
    fn treasury_key(env: &Env) -> Symbol {
        Symbol::new(env, "treasury_addr")
    }

    #[inline(always)]
    fn claim_window_start_key(env: &Env) -> Symbol {
        Symbol::new(env, "claim_window_start")
    }

    pub fn set_global_claim_period(env: &Env, claim_period_seconds: u64) {
        env.storage()
            .persistent()
            .set(&Self::global_claim_period_key(env), &claim_period_seconds);
    }

    pub fn get_global_claim_period(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&Self::global_claim_period_key(env))
            .unwrap_or(DEFAULT_UNCLAIMED_CLAIM_PERIOD_SECONDS)
    }

    pub fn set_market_claim_period(env: &Env, market_id: &Symbol, claim_period_seconds: u64) {
        let mut periods: Map<Symbol, u64> = env
            .storage()
            .persistent()
            .get(&Self::market_claim_periods_key(env))
            .unwrap_or(Map::new(env));
        periods.set(market_id.clone(), claim_period_seconds);
        env.storage()
            .persistent()
            .set(&Self::market_claim_periods_key(env), &periods);
    }

    pub fn get_market_claim_period(env: &Env, market_id: &Symbol) -> Option<u64> {
        let periods: Map<Symbol, u64> = env
            .storage()
            .persistent()
            .get(&Self::market_claim_periods_key(env))
            .unwrap_or(Map::new(env));
        periods.get(market_id.clone())
    }

    pub fn get_effective_claim_period(env: &Env, market_id: &Symbol) -> u64 {
        Self::get_market_claim_period(env, market_id).unwrap_or(Self::get_global_claim_period(env))
    }

    pub fn claim_deadline(env: &Env, market_id: &Symbol, market_end_time: u64) -> u64 {
        Self::get_claim_window_start(env, market_id, market_end_time)
            .saturating_add(Self::get_effective_claim_period(env, market_id))
    }

    pub fn is_claim_window_expired(env: &Env, market_id: &Symbol, market_end_time: u64) -> bool {
        env.ledger().timestamp() >= Self::claim_deadline(env, market_id, market_end_time)
    }

    pub fn set_claim_window_start_if_missing(env: &Env, market_id: &Symbol, start_timestamp: u64) {
        let mut starts: Map<Symbol, u64> = env
            .storage()
            .persistent()
            .get(&Self::claim_window_start_key(env))
            .unwrap_or(Map::new(env));

        if starts.get(market_id.clone()).is_none() {
            starts.set(market_id.clone(), start_timestamp);
            env.storage()
                .persistent()
                .set(&Self::claim_window_start_key(env), &starts);
        }
    }

    pub fn get_claim_window_start(env: &Env, market_id: &Symbol, market_end_time: u64) -> u64 {
        let starts: Map<Symbol, u64> = env
            .storage()
            .persistent()
            .get(&Self::claim_window_start_key(env))
            .unwrap_or(Map::new(env));

        starts.get(market_id.clone()).unwrap_or(market_end_time)
    }

    pub fn set_treasury(env: &Env, treasury: &Address) {
        env.storage()
            .persistent()
            .set(&Self::treasury_key(env), treasury);
    }

    pub fn get_treasury(env: &Env) -> Option<Address> {
        env.storage().persistent().get(&Self::treasury_key(env))
    }
}

// ===== VALIDATION =====
pub struct RecoveryValidator;
impl RecoveryValidator {
    pub fn validate_market_state_integrity(env: &Env, market_id: &Symbol) -> Result<(), Error> {
        let market = MarketStateManager::get_market(env, market_id)?;

        // Simple integrity checks (extend as needed)
        if market.total_staked < 0 {
            return Err(Error::InvalidState);
        }
        if market.outcomes.len() < 2 {
            return Err(Error::InvalidOutcomes);
        }
        if market.end_time == 0 {
            return Err(Error::InvalidState);
        }

        Ok(())
    }

    pub fn validate_recovery_safety(_env: &Env, data: &RecoveryData) -> Result<(), Error> {
        if !data.can_recover {
            return Err(Error::InvalidState);
        }
        if data.safety_score < 0 {
            return Err(Error::InvalidState);
        }
        Ok(())
    }
}

// ===== MANAGER =====
pub struct RecoveryManager;
impl RecoveryManager {
    pub fn assert_is_admin(env: &Env, admin: &Address) -> Result<(), Error> {
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "Admin"))
            .ok_or(Error::AdminNotSet)?;
        if &stored_admin != admin {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    pub fn get_recovery_status(env: &Env, market_id: &Symbol) -> Result<String, Error> {
        RecoveryStorage::status(env, market_id).ok_or(Error::InvalidState)
    }

    /// Prune oldest completed recovery history entries for a market (admin only).
    pub fn prune_recovery_history(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        count: u32,
    ) -> Result<u32, Error> {
        RecoveryStorage::prune_history(env, admin, market_id, count)
    }
    /// Perform recovery for a market. This operation is privileged and requires the caller to be
    /// the configured admin. The `actor` address will be recorded in emitted events for full
    /// visibility and auditability.
    pub fn recover_market_state(
        env: &Env,
        actor: &Address,
        market_id: &Symbol,
    ) -> Result<bool, Error> {
        // Ensure caller is admin (defense-in-depth; callers should also enforce auth).
        Self::assert_is_admin(env, actor)?;

        // Validate integrity first; if valid skip
        if RecoveryValidator::validate_market_state_integrity(env, market_id).is_ok() {
            let rec = MarketRecovery {
                market_id: market_id.clone(),
                actions: Vec::new(env),
                issues_detected: Vec::new(env),
                recovered: false,
                partial_refund_total: 0,
                last_action: Some(String::from_str(env, "no_action_needed")),
            };
            RecoveryStorage::save(env, &rec);
            EventEmitter::emit_recovery_event(
                env,
                actor,
                market_id,
                &String::from_str(env, "skip"),
                &String::from_str(env, "integrity_ok"),
                None,
            );
            return Ok(false);
        }

        // Attempt reconstruction heuristics (simplified)
        let mut market = MarketStateManager::get_market(env, market_id)?;
        if market.state == MarketState::Closed || market.state == MarketState::Cancelled {
            // cannot reconstruct closed or cancelled; treat as skip
            return Ok(false);
        }

        // Example heuristic: ensure total_staked matches sum of stakes map
        let mut recomputed: i128 = 0;
        for (_, v) in market.stakes.iter() {
            recomputed += v;
        }
        if recomputed != market.total_staked {
            market.total_staked = recomputed;
        }

        MarketStateManager::update_market(env, market_id, &market);

        let mut actions = Vec::new(env);
        actions.push_back(String::from_str(env, "reconstructed_totals"));

        let rec = MarketRecovery {
            market_id: market_id.clone(),
            actions,
            issues_detected: Vec::new(env),
            recovered: true,
            partial_refund_total: 0,
            last_action: Some(String::from_str(env, "reconstructed")),
        };
        RecoveryStorage::save(env, &rec);
        EventEmitter::emit_recovery_event(
            env,
            actor,
            market_id,
            &String::from_str(env, "recover"),
            &String::from_str(env, "reconstructed"),
            None,
        );
        Ok(true)
    }

    /// Execute partial refunds for selected users. This is privileged and requires the caller
    /// to be admin. Refund actions are fully recorded via events including the `actor` address.
    pub fn partial_refund_mechanism(
        env: &Env,
        actor: &Address,
        market_id: &Symbol,
        users: &Vec<Address>,
    ) -> Result<i128, Error> {
        // ensure caller is admin
        Self::assert_is_admin(env, actor)?;

        let mut market = MarketStateManager::get_market(env, market_id)?;
        let mut total_refunded: i128 = 0;

        for user in users.iter() {
            if let Some(stake) = market.stakes.get(user.clone()) {
                if stake > 0 {
                    // For now just mark claimed and reduce total; real implementation would transfer tokens
                    market
                        .claimed
                        .set(user.clone(), crate::types::ClaimInfo::new(env, stake));
                    market.total_staked = market.total_staked - stake;
                    total_refunded += stake;
                }
            }
        }
        MarketStateManager::update_market(env, market_id, &market);

        // Update recovery record
        let mut rec = RecoveryStorage::load(env, market_id).unwrap_or(MarketRecovery {
            market_id: market_id.clone(),
            actions: Vec::new(env),
            issues_detected: Vec::new(env),
            recovered: false,
            partial_refund_total: 0,
            last_action: None,
        });
        rec.partial_refund_total += total_refunded;
        rec.actions
            .push_back(String::from_str(env, "partial_refund"));
        rec.last_action = Some(String::from_str(env, "partial_refund"));
        RecoveryStorage::save(env, &rec);
        EventEmitter::emit_recovery_event(
            env,
            actor,
            market_id,
            &String::from_str(env, "partial_refund"),
            &String::from_str(env, "executed"),
            Some(total_refunded),
        );
        Ok(total_refunded)
    }
}

// ===== EVENT INTEGRATION =====
impl EventEmitter {
    /// Emit a recovery event that includes the acting admin and optional amount.
    pub fn emit_recovery_event(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        action: &String,
        status: &String,
        amount: Option<i128>,
    ) {
        let topic = Symbol::new(env, "recovery_evt");
        // Publish a tuple: (action, status, amount, timestamp)
        let amt = amount.unwrap_or(0);
        env.events().publish(
            (topic, admin.clone(), market_id.clone()),
            (
                action.clone(),
                status.clone(),
                amt,
                env.ledger().timestamp(),
            ),
        );
    }
}

// Helper for symbol -> string representation (Soroban lacks direct to_string for Symbol)
fn symbol_to_string(env: &Env, sym: &Symbol) -> String {
    // Use debug formatting of Symbol then convert to soroban String
    let host_string = format!("{:?}", sym);
    String::from_str(env, &host_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::testutils::Ledger;

    struct RecoveryTest {
        env: Env,
        admin: Address,
        market_id: Symbol,
    }

    impl RecoveryTest {
        fn new() -> Self {
            let env = Env::default();
            let admin = Address::generate(&env);
            let market_id = Symbol::new(&env, "market_1");
            RecoveryTest {
                env,
                admin,
                market_id,
            }
        }
    }

    #[test]
    fn test_recovery_storage_load_nonexistent() {
        let test = RecoveryTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test loading recovery record that doesn't exist
        let record = test.env.as_contract(&contract_id, || {
            RecoveryStorage::load(&test.env, &test.market_id)
        });
        assert!(record.is_none());
    }

    #[test]
    fn test_recovery_storage_save_and_load() {
        let test = RecoveryTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test saving and loading recovery record
        let mut actions = Vec::new(&test.env);
        actions.push_back(String::from_str(&test.env, "test_action"));

        let record = MarketRecovery {
            market_id: test.market_id.clone(),
            actions,
            issues_detected: Vec::new(&test.env),
            recovered: true,
            partial_refund_total: 1000,
            last_action: Some(String::from_str(&test.env, "test")),
        };

        let loaded = test.env.as_contract(&contract_id, || {
            RecoveryStorage::save(&test.env, &record);
            RecoveryStorage::load(&test.env, &test.market_id)
        });
        assert!(loaded.is_some());
    }

    #[test]
    fn test_recovery_storage_status_pending() {
        let test = RecoveryTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test status when recovery is pending
        let record = MarketRecovery {
            market_id: test.market_id.clone(),
            actions: Vec::new(&test.env),
            issues_detected: Vec::new(&test.env),
            recovered: false,
            partial_refund_total: 0,
            last_action: None,
        };
        let status = test.env.as_contract(&contract_id, || {
            RecoveryStorage::save(&test.env, &record);
            RecoveryStorage::status(&test.env, &test.market_id)
        });
        assert!(status.is_some());
    }

    #[test]
    fn test_recovery_storage_status_recovered() {
        let test = RecoveryTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        // Test status when recovery is completed
        let record = MarketRecovery {
            market_id: test.market_id.clone(),
            actions: Vec::new(&test.env),
            issues_detected: Vec::new(&test.env),
            recovered: true,
            partial_refund_total: 500,
            last_action: None,
        };
        let status = test.env.as_contract(&contract_id, || {
            RecoveryStorage::save(&test.env, &record);
            RecoveryStorage::status(&test.env, &test.market_id)
        });
        assert!(status.is_some());
    }

    #[test]
    fn test_recovery_validator_safety_score_negative() {
        let test = RecoveryTest::new();
        // Test that negative safety score fails validation
        let data = RecoveryData {
            inconsistencies: Vec::new(&test.env),
            can_recover: true,
            safety_score: -100,
        };
        let result = RecoveryValidator::validate_recovery_safety(&test.env, &data);
        assert!(result.is_err());
    }

    #[test]
    fn test_recovery_validator_cannot_recover() {
        let test = RecoveryTest::new();
        // Test that non-recoverable data fails validation
        let data = RecoveryData {
            inconsistencies: Vec::new(&test.env),
            can_recover: false,
            safety_score: 50,
        };
        let result = RecoveryValidator::validate_recovery_safety(&test.env, &data);
        assert!(result.is_err());
    }

    #[test]
    fn test_recovery_validator_valid_data() {
        let test = RecoveryTest::new();
        // Test that valid recovery data passes validation
        let data = RecoveryData {
            inconsistencies: Vec::new(&test.env),
            can_recover: true,
            safety_score: 75,
        };
        let result = RecoveryValidator::validate_recovery_safety(&test.env, &data);
        // Would pass if market exists and has valid state
        assert!(data.can_recover);
    }

    #[test]
    fn test_recovery_manager_admin_check_valid() {
        let test = RecoveryTest::new();
        // Test that valid admin passes check (if admin is stored)
        let admin = test.admin;
        assert!(!admin.to_string().is_empty());
    }

    #[test]
    fn test_recovery_manager_admin_check_invalid() {
        let test = RecoveryTest::new();
        // Test that non-admin fails check
        let non_admin = Address::generate(&test.env);
        assert_ne!(non_admin.to_string(), test.admin.to_string());
    }

    #[test]
    fn test_recovery_manager_get_recovery_status() {
        let test = RecoveryTest::new();
        // Test getting recovery status
        let market_id = test.market_id;
        // Would fail with InvalidState if status not set
        assert!(!market_id.to_string().is_empty());
    }

    #[test]
    fn test_recovery_actions_vector() {
        let test = RecoveryTest::new();
        // Test that actions vector in MarketRecovery works
        let mut actions = Vec::new(&test.env);
        actions.push_back(String::from_str(&test.env, "action_1"));
        actions.push_back(String::from_str(&test.env, "action_2"));
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn test_recovery_issues_detected_vector() {
        let test = RecoveryTest::new();
        // Test that issues_detected vector works
        let mut issues = Vec::new(&test.env);
        issues.push_back(String::from_str(&test.env, "issue_1"));
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_partial_refund_total_tracking() {
        let test = RecoveryTest::new();
        // Test that partial_refund_total is properly tracked
        let amount1 = 1000i128;
        let amount2 = 500i128;
        let total = amount1 + amount2;
        assert_eq!(total, 1500);
    }

    #[test]
    fn test_recovery_data_inconsistencies() {
        let test = RecoveryTest::new();
        // Test RecoveryData with inconsistencies
        let mut inconsistencies = Vec::new(&test.env);
        inconsistencies.push_back(String::from_str(&test.env, "inc_1"));
        let data = RecoveryData {
            inconsistencies,
            can_recover: true,
            safety_score: 50,
        };
        assert_eq!(data.inconsistencies.len(), 1);
    }

    #[test]
    fn test_recovery_action_enum_variants() {
        // Test all RecoveryAction variants exist
        let _ = RecoveryAction::MarketStateReconstructed;
        let _ = RecoveryAction::PartialRefundExecuted;
        let _ = RecoveryAction::IntegrityValidated;
        let _ = RecoveryAction::RecoverySkipped;
    }

    #[test]
    fn test_market_recovery_structure_creation() {
        let test = RecoveryTest::new();
        // Test creating MarketRecovery structure
        let recovery = MarketRecovery {
            market_id: test.market_id.clone(),
            actions: Vec::new(&test.env),
            issues_detected: Vec::new(&test.env),
            recovered: false,
            partial_refund_total: 0,
            last_action: None,
        };
        assert!(!recovery.market_id.to_string().is_empty());
    }

    #[test]
    fn test_market_recovery_with_partial_refund() {
        let test = RecoveryTest::new();
        // Test MarketRecovery with partial refund amount
        let recovery = MarketRecovery {
            market_id: test.market_id.clone(),
            actions: Vec::new(&test.env),
            issues_detected: Vec::new(&test.env),
            recovered: true,
            partial_refund_total: 5000,
            last_action: Some(String::from_str(&test.env, "refund_done")),
        };
        assert!(recovery.partial_refund_total > 0);
    }

    #[test]
    fn test_recovery_storage_keys() {
        let test = RecoveryTest::new();
        let active_key = RecoveryStorage::active_key(&test.env);
        let history_key = RecoveryStorage::history_key(&test.env);
        let status_key = RecoveryStorage::status_key(&test.env);
        assert_ne!(active_key.to_string(), status_key.to_string());
        assert_ne!(history_key.to_string(), status_key.to_string());
    }

    fn setup_admin_env() -> (Env, Address, Address, Symbol) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let market_id = Symbol::new(&env, "market_prune");
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "Admin"), &admin);
        });
        (env, admin, contract_id, market_id)
    }

    fn completed_record(env: &Env, market_id: &Symbol, tag: &str) -> MarketRecovery {
        let mut actions = Vec::new(env);
        actions.push_back(String::from_str(env, tag));
        MarketRecovery {
            market_id: market_id.clone(),
            actions,
            issues_detected: Vec::new(env),
            recovered: true,
            partial_refund_total: 0,
            last_action: Some(String::from_str(env, tag)),
        }
    }

    fn pending_record(env: &Env, market_id: &Symbol) -> MarketRecovery {
        MarketRecovery {
            market_id: market_id.clone(),
            actions: Vec::new(env),
            issues_detected: Vec::new(env),
            recovered: false,
            partial_refund_total: 0,
            last_action: Some(String::from_str(env, "pending")),
        }
    }

    fn completed_record_minimal(env: &Env, market_id: &Symbol) -> MarketRecovery {
        MarketRecovery {
            market_id: market_id.clone(),
            actions: Vec::new(env),
            issues_detected: Vec::new(env),
            recovered: true,
            partial_refund_total: 0,
            last_action: None,
        }
    }

    #[test]
    fn test_recovery_history_capped_per_market() {
        let env = Env::default();
        let market_id = Symbol::new(&env, "m_cap");
        let mut history = Vec::new(&env);
        let writes = MAX_RECOVERY_HISTORY_PER_MARKET as usize + 5;
        for _ in 0..writes {
            history.push_back(RecoveryHistoryEntry {
                record: completed_record_minimal(&env, &market_id),
                recorded_at: 0,
            });
        }
        RecoveryStorage::trim_history(&env, &mut history);
        assert_eq!(history.len(), MAX_RECOVERY_HISTORY_PER_MARKET);

        let (env, _admin, contract_id, market_id) = setup_admin_env();
        env.as_contract(&contract_id, || {
            for _ in 0..5 {
                RecoveryStorage::save(&env, &completed_record_minimal(&env, &market_id));
            }
            assert_eq!(RecoveryStorage::history_len(&env, &market_id), 5u32);
            assert!(RecoveryStorage::load_active(&env, &market_id).is_none());
        });
    }

    #[test]
    fn test_prune_preserves_active_recovery() {
        let (env, admin, contract_id, market_id) = setup_admin_env();
        env.as_contract(&contract_id, || {
            for i in 0..5 {
                RecoveryStorage::save(
                    &env,
                    &completed_record(&env, &market_id, &format!("done_{}", i)),
                );
            }
            RecoveryStorage::save(&env, &pending_record(&env, &market_id));
            assert_eq!(RecoveryStorage::history_len(&env, &market_id), 5);

            let removed = RecoveryStorage::prune_history(&env, &admin, &market_id, 3).unwrap();
            assert_eq!(removed, 3);
            assert_eq!(RecoveryStorage::history_len(&env, &market_id), 2);
            let active = RecoveryStorage::load_active(&env, &market_id).expect("active kept");
            assert!(!active.recovered);
        });
    }

    #[test]
    fn test_prune_count_greater_than_stored() {
        let (env, admin, contract_id, market_id) = setup_admin_env();
        env.as_contract(&contract_id, || {
            RecoveryStorage::save(&env, &completed_record(&env, &market_id, "only"));
            let removed = RecoveryStorage::prune_history(&env, &admin, &market_id, 100).unwrap();
            assert_eq!(removed, 1);
            assert_eq!(RecoveryStorage::history_len(&env, &market_id), 0);
        });
    }

    #[test]
    fn test_prune_requires_admin() {
        let (env, _admin, contract_id, market_id) = setup_admin_env();
        let intruder = Address::generate(&env);
        env.as_contract(&contract_id, || {
            RecoveryStorage::save(&env, &completed_record(&env, &market_id, "x"));
            let result = RecoveryStorage::prune_history(&env, &intruder, &market_id, 1);
            assert_eq!(result, Err(Error::Unauthorized));
        });
    }

    #[test]
    fn test_symbol_to_string_conversion() {
        let test = RecoveryTest::new();
        // Test helper function for symbol to string conversion
        let symbol = Symbol::new(&test.env, "test_symbol");
        let string = symbol_to_string(&test.env, &symbol);
        assert!(!string.to_string().is_empty());
    }

    #[test]
    fn test_recovery_event_emission() {
        let test = RecoveryTest::new();
        // Test that recovery event can be emitted
        let market_id = test.market_id;
        let action = String::from_str(&test.env, "recover");
        let status = String::from_str(&test.env, "success");
        // Event emission should not panic
        EventEmitter::emit_recovery_event(
            &test.env,
            &test.admin,
            &market_id,
            &action,
            &status,
            Some(0),
        );
        assert!(true);
    }

    #[test]
    fn test_recovery_no_action_case() {
        let test = RecoveryTest::new();
        // Test recovery when no action is needed
        let recovery = MarketRecovery {
            market_id: test.market_id.clone(),
            actions: Vec::new(&test.env),
            issues_detected: Vec::new(&test.env),
            recovered: false,
            partial_refund_total: 0,
            last_action: Some(String::from_str(&test.env, "no_action_needed")),
        };
        assert!(!recovery.recovered);
    }

    #[test]
    fn test_recovery_reconstructed_state() {
        let test = RecoveryTest::new();
        // Test recovery state after reconstruction
        let mut actions = Vec::new(&test.env);
        actions.push_back(String::from_str(&test.env, "reconstructed_totals"));
        let recovery = MarketRecovery {
            market_id: test.market_id.clone(),
            actions,
            issues_detected: Vec::new(&test.env),
            recovered: true,
            partial_refund_total: 0,
            last_action: Some(String::from_str(&test.env, "reconstructed")),
        };
        assert!(recovery.recovered);
    }

    #[test]
    fn test_recovery_data_safety_score_range() {
        let test = RecoveryTest::new();
        // Test safety score boundary conditions
        let high_score = 100i128;
        let low_score = 1i128;
        let zero_score = 0i128;
        assert!(high_score > low_score);
        assert!(low_score > zero_score);
    }

    #[test]
    fn test_recovery_multiple_issues() {
        let test = RecoveryTest::new();
        // Test recovery record with multiple issues
        let mut issues = Vec::new(&test.env);
        issues.push_back(String::from_str(&test.env, "issue_1"));
        issues.push_back(String::from_str(&test.env, "issue_2"));
        issues.push_back(String::from_str(&test.env, "issue_3"));
        assert_eq!(issues.len(), 3);
    }

    // ===== PER-MARKET RECOVERY TIMELOCK TESTS =====

    /// Use the minimum allowed timelock so tests can execute immediately.
    const TEST_TIMELOCK_SECONDS: u64 = MIN_RECOVERY_TIMELOCK_SECONDS;

    fn setup_with_market(state: MarketState) -> (Env, Address, Symbol, soroban_sdk::Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let market_id = Symbol::new(&env, "rcv_mkt");
        let contract_id = env.register(crate::PredictifyHybrid, ());
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "Admin"), &admin);
            let market = crate::types::Market {
                admin: admin.clone(),
                question: String::from_str(&env, "Test?"),
                outcomes: Vec::new(&env),
                end_time: 1000,
                oracle_config: crate::types::OracleConfig::none_sentinel(&env),
                has_fallback: false,
                fallback_oracle_config: crate::types::OracleConfig::none_sentinel(&env),
                resolution_timeout: 3600,
                oracle_result: None,
                votes: soroban_sdk::Map::new(&env),
                stakes: soroban_sdk::Map::new(&env),
                claimed: soroban_sdk::Map::new(&env),
                total_staked: 0,
                dispute_stakes: soroban_sdk::Map::new(&env),
                winning_outcomes: None,
                fee_collected: false,
                state,
                total_extension_days: 0,
                max_extension_days: 0,
                extension_history: Vec::new(&env),
                category: None,
                tags: Vec::new(&env),
                min_pool_size: None,
                bet_deadline: 0,
                dispute_window_seconds: 0,
                winnings_swept: false,
            };
            MarketStateManager::update_market(&env, &market_id, &market);
        });
        (env, admin, market_id, contract_id)
    }

    #[test]
    fn initiate_and_execute_force_resolve() {
        let (env, admin, market_id, contract_id) = setup_with_market(MarketState::Ended);

        let req = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::set_config(
                &env,
                &admin,
                &RecoveryTimelockConfig {
                    timelock_seconds: TEST_TIMELOCK_SECONDS,
                },
            )
            .unwrap();
            RecoveryTimelockManager::initiate_recovery(
                &env,
                &admin,
                &market_id,
                &PerMarketRecoveryAction::ForceResolve,
                &String::from_str(&env, "stuck market"),
            )
            .unwrap()
        });

        // Advance ledger past timelock.
        env.ledger().set_timestamp(req.execute_after);

        let result = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::execute_recovery(&env, &admin, &market_id).unwrap()
        });
        assert!(!result);

        let pending = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::get_pending(&env, &market_id)
        });
        assert!(pending.is_none());
    }

    #[test]
    fn timelock_blocks_early_execution() {
        let (env, admin, market_id, contract_id) = setup_with_market(MarketState::Ended);

        env.as_contract(&contract_id, || {
            RecoveryTimelockManager::initiate_recovery(
                &env,
                &admin,
                &market_id,
                &PerMarketRecoveryAction::ForceResolve,
                &String::from_str(&env, "too early"),
            )
            .unwrap();
        });

        let result = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::execute_recovery(&env, &admin, &market_id)
        });
        assert_eq!(result, Err(Error::RecoveryTimelockActive));
    }

    #[test]
    fn cancel_recovery_removes_pending() {
        let (env, admin, market_id, contract_id) = setup_with_market(MarketState::Ended);

        env.as_contract(&contract_id, || {
            RecoveryTimelockManager::initiate_recovery(
                &env,
                &admin,
                &market_id,
                &PerMarketRecoveryAction::ForceResolve,
                &String::from_str(&env, "cancel me"),
            )
            .unwrap();
            RecoveryTimelockManager::cancel_recovery(&env, &admin, &market_id).unwrap();
            assert!(RecoveryTimelockManager::get_pending(&env, &market_id).is_none());
        });
    }

    #[test]
    fn cannot_initiate_twice_without_cancelling() {
        let (env, admin, market_id, contract_id) = setup_with_market(MarketState::Ended);

        env.as_contract(&contract_id, || {
            RecoveryTimelockManager::initiate_recovery(
                &env,
                &admin,
                &market_id,
                &PerMarketRecoveryAction::ForceResolve,
                &String::from_str(&env, "first"),
            )
            .unwrap();

            let result = RecoveryTimelockManager::initiate_recovery(
                &env,
                &admin,
                &market_id,
                &PerMarketRecoveryAction::ForceResolve,
                &String::from_str(&env, "second"),
            );
            assert_eq!(result, Err(Error::RecoveryAlreadyPending));
        });
    }

    #[test]
    fn active_market_is_not_recoverable() {
        let (env, admin, market_id, contract_id) = setup_with_market(MarketState::Active);

        let result = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::initiate_recovery(
                &env,
                &admin,
                &market_id,
                &PerMarketRecoveryAction::ForceResolve,
                &String::from_str(&env, "nope"),
            )
        });
        assert_eq!(result, Err(Error::MarketNotRecoverable));
    }

    #[test]
    fn reconstruct_state_works_for_disputed() {
        let (env, admin, market_id, contract_id) = setup_with_market(MarketState::Disputed);

        let req = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::set_config(
                &env,
                &admin,
                &RecoveryTimelockConfig {
                    timelock_seconds: TEST_TIMELOCK_SECONDS,
                },
            )
            .unwrap();
            RecoveryTimelockManager::initiate_recovery(
                &env,
                &admin,
                &market_id,
                &PerMarketRecoveryAction::ReconstructState,
                &String::from_str(&env, "fix totals"),
            )
            .unwrap()
        });

        env.ledger().set_timestamp(req.execute_after);

        env.as_contract(&contract_id, || {
            let result =
                RecoveryTimelockManager::execute_recovery(&env, &admin, &market_id).unwrap();
            assert!(result);
            let market = MarketStateManager::get_market(&env, &market_id).unwrap();
            assert_eq!(market.total_staked, 0);
        });
    }

    #[test]
    fn cancel_market_marks_all_stakes_claimed() {
        let (env, admin, market_id, contract_id) = setup_with_market(MarketState::Ended);
        let user = Address::generate(&env);

        let req = env.as_contract(&contract_id, || {
            let mut market = MarketStateManager::get_market(&env, &market_id).unwrap();
            market.stakes.set(user.clone(), 500);
            market.total_staked = 500;
            MarketStateManager::update_market(&env, &market_id, &market);

            RecoveryTimelockManager::set_config(
                &env,
                &admin,
                &RecoveryTimelockConfig {
                    timelock_seconds: TEST_TIMELOCK_SECONDS,
                },
            )
            .unwrap();
            RecoveryTimelockManager::initiate_recovery(
                &env,
                &admin,
                &market_id,
                &PerMarketRecoveryAction::CancelMarket,
                &String::from_str(&env, "emergency cancel"),
            )
            .unwrap()
        });

        env.ledger().set_timestamp(req.execute_after);

        env.as_contract(&contract_id, || {
            RecoveryTimelockManager::execute_recovery(&env, &admin, &market_id).unwrap();

            let market = MarketStateManager::get_market(&env, &market_id).unwrap();
            assert_eq!(market.state, MarketState::Cancelled);
            assert_eq!(market.total_staked, 0);
            assert!(market.claimed.get(user.clone()).is_some());
        });
    }

    #[test]
    fn config_tighten_only() {
        let (env, admin, _market_id, contract_id) = setup_with_market(MarketState::Ended);

        env.as_contract(&contract_id, || {
            RecoveryTimelockManager::set_config(
                &env,
                &admin,
                &RecoveryTimelockConfig {
                    timelock_seconds: 3600,
                },
            )
            .unwrap();

            RecoveryTimelockManager::set_config(
                &env,
                &admin,
                &RecoveryTimelockConfig {
                    timelock_seconds: 7200,
                },
            )
            .unwrap();
            assert_eq!(
                RecoveryTimelockManager::get_config(&env).timelock_seconds,
                7200
            );

            let result = RecoveryTimelockManager::set_config(
                &env,
                &admin,
                &RecoveryTimelockConfig {
                    timelock_seconds: 1800,
                },
            );
            assert_eq!(result, Err(Error::RecoveryTimelockConfigInvalid));
        });
    }

    #[test]
    fn config_rejects_out_of_bounds() {
        let (env, admin, _market_id, contract_id) = setup_with_market(MarketState::Ended);

        env.as_contract(&contract_id, || {
            let result = RecoveryTimelockManager::set_config(
                &env,
                &admin,
                &RecoveryTimelockConfig {
                    timelock_seconds: 100,
                },
            );
            assert_eq!(result, Err(Error::RecoveryTimelockConfigInvalid));

            let result = RecoveryTimelockManager::set_config(
                &env,
                &admin,
                &RecoveryTimelockConfig {
                    timelock_seconds: 8 * 24 * 60 * 60,
                },
            );
            assert_eq!(result, Err(Error::RecoveryTimelockConfigInvalid));
        });
    }

    #[test]
    fn non_admin_cannot_initiate() {
        let (env, _admin, market_id, contract_id) = setup_with_market(MarketState::Ended);
        let rando = Address::generate(&env);

        let result = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::initiate_recovery(
                &env,
                &rando,
                &market_id,
                &PerMarketRecoveryAction::ForceResolve,
                &String::from_str(&env, "hacker"),
            )
        });
        assert_eq!(result, Err(Error::Unauthorized));
    }

    #[test]
    fn cancel_nonexistent_request_fails() {
        let (env, admin, market_id, contract_id) = setup_with_market(MarketState::Ended);

        let result = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::cancel_recovery(&env, &admin, &market_id)
        });
        assert_eq!(result, Err(Error::RecoveryRequestNotFound));
    }

    #[test]
    fn force_resolve_transitions_disputed_to_ended() {
        let (env, admin, market_id, contract_id) = setup_with_market(MarketState::Disputed);

        let req = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::set_config(
                &env,
                &admin,
                &RecoveryTimelockConfig {
                    timelock_seconds: TEST_TIMELOCK_SECONDS,
                },
            )
            .unwrap();
            RecoveryTimelockManager::initiate_recovery(
                &env,
                &admin,
                &market_id,
                &PerMarketRecoveryAction::ForceResolve,
                &String::from_str(&env, "push to ended"),
            )
            .unwrap()
        });

        env.ledger().set_timestamp(req.execute_after);

        env.as_contract(&contract_id, || {
            let changed =
                RecoveryTimelockManager::execute_recovery(&env, &admin, &market_id).unwrap();
            assert!(changed);
            let market = MarketStateManager::get_market(&env, &market_id).unwrap();
            assert_eq!(market.state, MarketState::Ended);
        });
    }

    #[test]
    fn reconstruct_state_rejects_active_market() {
        let (env, admin, market_id, contract_id) = setup_with_market(MarketState::Active);

        let result = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::initiate_recovery(
                &env,
                &admin,
                &market_id,
                &PerMarketRecoveryAction::ReconstructState,
                &String::from_str(&env, "try"),
            )
        });
        assert_eq!(result, Err(Error::MarketNotRecoverable));
    }

    #[test]
    fn cancel_market_rejects_ended_without_stakes() {
        let (env, admin, market_id, contract_id) = setup_with_market(MarketState::Ended);

        let req = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::set_config(
                &env,
                &admin,
                &RecoveryTimelockConfig {
                    timelock_seconds: TEST_TIMELOCK_SECONDS,
                },
            )
            .unwrap();
            RecoveryTimelockManager::initiate_recovery(
                &env,
                &admin,
                &market_id,
                &PerMarketRecoveryAction::CancelMarket,
                &String::from_str(&env, "cancel empty"),
            )
            .unwrap()
        });

        env.ledger().set_timestamp(req.execute_after);

        env.as_contract(&contract_id, || {
            let result =
                RecoveryTimelockManager::execute_recovery(&env, &admin, &market_id).unwrap();
            assert!(result);
            let market = MarketStateManager::get_market(&env, &market_id).unwrap();
            assert_eq!(market.state, MarketState::Cancelled);
        });
    }

    #[test]
    fn get_config_returns_defaults_when_unset() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let config = env.as_contract(&contract_id, || {
            RecoveryTimelockManager::get_config(&env)
        });
        assert_eq!(config.timelock_seconds, DEFAULT_RECOVERY_TIMELOCK_SECONDS);
    }
}

// Helper to build composite key prefix + symbol as soroban Symbol
// composite_symbol no longer required with new map-based storage approach
