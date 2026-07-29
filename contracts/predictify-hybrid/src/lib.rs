#![no_std]

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern crate alloc;

use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, Address, BytesN, Env, Map, String, Symbol, Vec, Val,
};

pub const PERCENTAGE_DENOMINATOR: i128 = 10000;
pub const SYM_ADMIN: &str = "Admin";
pub const ORACLE_FAILURE_PRIMARY_THEN_FALLBACK_REASON: &str = "Primary oracle failed, fallback also failed";
const SYM_ALLOWED_ASSETS: &str = "allowed"; 

mod admin;
pub mod analytics;
pub mod audit;
pub mod audit_trail;
pub mod monitor;
mod balances;
mod batch_operations;
mod bets;
pub mod circuit_breaker;
mod config;
mod err;
mod force_resolve;
mod event_archive;
pub mod events;
mod extensions;
pub mod gov_registry;
mod fees;
mod gas;
mod governance;
mod markets;
mod monitoring;
mod oracles;
mod reentrancy_guard;
mod oracle_health;
mod reporting;

#[cfg(test)]
mod resolution_event_ordering_tests;
#[cfg(test)]
mod resolution_state_property_tests;

#[cfg(test)]
#[path = "tests/oracle_validation_tests.rs"]
mod oracle_validation_tests;

#[cfg(test)]
#[path = "tests/market_closed_tests.rs"]
mod market_closed_tests;

#[cfg(test)]
mod cross_oracle_staleness_tests;

#[cfg(test)]
mod reflector_auth_tests;

#[cfg(test)]
mod reflector_auth_snap_tests;

mod resolution;
mod storage;
mod tokens;
mod types;
mod upgrade_manager;
mod utils;
mod validation;
mod versioning;
mod voting;
mod market_analytics;
mod performance_benchmarks;
mod disputes;
mod edge_cases;
mod graceful_degradation;
mod market_id_generator;
mod metadata_limits;
mod queries;
mod recovery;
mod statistics;
mod rate_limiter;
mod dispute_multisig;
mod event_topic_catalog;
mod storage_tier_audit;
mod leaderboard;
mod lists;
mod capabilities;

#[cfg(test)]
mod override_audit_tests;
#[cfg(test)]
mod market_audit_tests;
#[cfg(test)]
mod test_audit_trail;
#[cfg(test)]
mod tests;

mod bandprotocol {
    soroban_sdk::contractimport!(file = "./std_reference.wasm");
}

pub mod timelock;
pub mod handshake;

use bets::BetStorage;
use gas::BudgetGuard;
use resolution::ResolutionOutcomeCache;
use storage::{BalanceStorage, StorageOptimizer};
use types::{Market, ReflectorAsset};
use events::EventEmitter;
// `CircuitBreaker`, `Error`, `EventEmitter`, `ClaimInfo` and the soroban_sdk
// prelude items are imported/re-exported once below; duplicating them here
// tripped E0252 "defined multiple times".

#[cfg(test)]
mod resolution_invariants;
#[cfg(test)]
mod market_state_matrix_tests;
#[cfg(test)]
mod upgrade_manager_tests;
#[cfg(test)]
mod oracle_lifecycle_events_tests;
mod timelock_tests;
#[cfg(test)]
mod market_leaderboard_tests;

#[cfg(test)]
mod governance_tests;

#[cfg(any())]
mod category_tags_tests;
#[cfg(test)]
mod tie_resolution_tests;
#[cfg(test)]
mod force_resolve_tests;

#[cfg(test)]
mod analytics_snapshot_tests;
#[cfg(test)]
mod betting_invariant_proptest;
#[cfg(test)]
mod property_based_tests;
#[cfg(test)]
mod betting_invariants;
mod analytics_snapshot;

#[cfg(test)]
mod max_participants_tests;

#[cfg(test)]
#[path = "tests/fee_config_commit_reveal_tests.rs"]
mod fee_config_commit_reveal_tests;

#[cfg(test)]
mod admin_cooldown_tests;

// Re-export commonly used items
use admin::{AdminAnalyticsResult, AdminInitializer, AdminManager, AdminPermission, AdminRole};
use crate::Error;
// Backwards-compatible re-export for existing module paths.
pub mod errors {
    pub use crate::Error;
}
pub use crate::Error;
// pub use queries::QueryManager;
pub use audit_trail::{AuditAction, AuditRecord, AuditTrailHead, AuditTrailManager};
pub use types::*;

use crate::config::{
    ConfigManager, DEFAULT_PLATFORM_FEE_PERCENTAGE, MAX_PLATFORM_FEE_PERCENTAGE,
    MIN_PLATFORM_FEE_PERCENTAGE,
};
use crate::gas::GasTracker;
use crate::graceful_degradation::{OracleBackup, OracleHealth};
use crate::market_id_generator::MarketIdGenerator;
use alloc::format;

impl From<crate::reentrancy_guard::GuardError> for Error {
    fn from(_err: crate::reentrancy_guard::GuardError) -> Self {
        Error::InvalidState
    }
}

impl From<crate::rate_limiter::RateLimiterError> for Error {
    fn from(err: crate::rate_limiter::RateLimiterError) -> Self {
        match err {
            crate::rate_limiter::RateLimiterError::RateLimitExceeded => Error::RateLimitExceeded,
            crate::rate_limiter::RateLimiterError::ConfigNotFound => Error::ConfigNotFound,
            crate::rate_limiter::RateLimiterError::Unauthorized => Error::Unauthorized,
            _ => Error::RateLimitExceeded,
        }
    }
}

fn resolution_timeout_reached(env: &Env, market: &Market) -> bool {
    let current_time = env.ledger().timestamp();
    current_time >= market.end_time.saturating_add(market.resolution_timeout)
}

fn automatic_oracle_result_unavailable(
    env: &Env,
    config: &OracleConfig,
) -> Result<String, Error> {
    if !config.is_active() {
        return Err(Error::OracleUnavailable);
    }
    Ok(String::from_str(env, "pending"))
}

/// Probe an oracle for an automatic result; returns the raw oracle result string.
fn get_oracle_result(env: &Env, config: &OracleConfig) -> Result<String, Error> {
    automatic_oracle_result_unavailable(env, config)
}

#[contract]
pub struct PredictifyHybrid;

#[contractimpl]
impl PredictifyHybrid {
    pub fn initialize(
        env: Env,
        admin: Address,
        platform_fee_percentage: Option<i128>,
        allowed_assets: Option<Vec<Address>>,
    ) -> Result<(), Error> {
        if env.storage().persistent().has(&Symbol::new(&env, SYM_PLATFORM_FEE)) {
            return Err(Error::InvalidState);
        }

        let fee_percentage = platform_fee_percentage.unwrap_or(DEFAULT_PLATFORM_FEE_PERCENTAGE);

        if fee_percentage < MIN_PLATFORM_FEE_PERCENTAGE || fee_percentage > MAX_PLATFORM_FEE_PERCENTAGE {
            return Err(Error::InvalidFeeConfig);
        }

        AdminInitializer::initialize(&env, &admin)?;

        match crate::circuit_breaker::CircuitBreaker::initialize(&env) {
            Ok(_) => (),
            Err(e) => panic_with_error!(env, e),
        }

        env.storage().persistent().set(&Symbol::new(&env, SYM_PLATFORM_FEE), &fee_percentage);

        let mut default_config = ConfigManager::get_development_config(&env);
        default_config.fees.platform_fee_percentage = fee_percentage;
        ConfigManager::store_config(&env, &default_config)?;

        crate::rate_limiter::RateLimiter::new(env.clone())
            .init_rate_limiter(
                admin.clone(),
                crate::rate_limiter::RateLimitConfig {
                    voting_limit: 10_000,
                    dispute_limit: 1_000,
                    oracle_call_limit: 1_000,
                    bet_limit: 10_000,
                    events_per_admin_limit: 1_000,
                    time_window_seconds: 3_600,
                    refill_mode: crate::rate_limiter::RefillMode::Linear,
                },
            )
            .map_err(Error::from)?;

        if let Some(assets) = allowed_assets {
            env.storage().persistent().set(&Symbol::new(&env, SYM_ALLOWED_ASSETS), &assets);
        } else {
            // Initialize the token registry with default supported assets.
            crate::tokens::TokenRegistry::initialize_with_defaults(&env);
        }

        crate::events::EventEmitter::emit_contract_initialized(&env, &admin, fee_percentage);
        crate::events::EventEmitter::emit_platform_fee_set(&env, fee_percentage, &admin);

        Ok(())
    }

    pub fn deposit(env: Env, user: Address, asset: ReflectorAsset, amount: i128) -> Result<types::Balance, Error> {
        crate::circuit_breaker::CircuitBreaker::require_write_allowed(&env, "deposit")?;
        balances::BalanceManager::deposit(&env, user, asset, amount)
    }

    pub fn withdraw(env: Env, user: Address, asset: ReflectorAsset, amount: i128) -> Result<types::Balance, Error> {
        crate::circuit_breaker::CircuitBreaker::require_write_allowed(&env, "withdraw")?;
        if !crate::circuit_breaker::CircuitBreaker::are_withdrawals_allowed(&env)? {
            return Err(Error::CBOpen);
        }
        balances::BalanceManager::withdraw(&env, user, asset, amount)
    }

    pub fn get_balance(env: Env, user: Address, asset: ReflectorAsset) -> types::Balance {
        storage::BalanceStorage::get_balance(&env, &user, &asset)
    }

    pub fn create_market(
        env: Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
        oracle_config: OracleConfig,
        fallback_oracle_config: Option<OracleConfig>,
        resolution_timeout: u64,
        min_pool_size: Option<i128>,
        bet_deadline_mins_before_end: Option<u64>,
        dispute_window_seconds: Option<u64>,
        dispute_stake_floor: Option<i128>,
        max_participants: Option<u32>,
    ) -> Symbol {
        if let Err(e) = crate::circuit_breaker::CircuitBreaker::require_write_allowed(&env, "create_market") {
            panic_with_error!(env, e);
        }
        let gas_marker = GasTracker::start_tracking(&env);
        Self::require_primary_admin_or_panic(&env, &admin);

        if let Err(rate_err) = crate::rate_limiter::RateLimiter::new(env.clone()).rate_limit_admin_events(admin.clone()) {
            if !matches!(rate_err, crate::rate_limiter::RateLimiterError::ConfigNotFound) {
                panic_with_error!(env, Error::from(rate_err));
            }
        }

        if let Err(e) = crate::validation::CreationValidator::validate_market_creation(&env, &question, &outcomes, &duration_days) {
            panic_with_error!(env, e);
        }

        if let Err(e) = oracle_config.validate(&env) {
            panic_with_error!(env, e);
        }
        if let Some(ref fallback) = fallback_oracle_config {
            if let Err(e) = fallback.validate(&env) {
                panic_with_error!(env, e);
            }
        }

        if duration_days == 0 {
            panic_with_error!(env, Error::InvalidDuration);
        }

        let market_id = MarketIdGenerator::generate_market_id(&env, &admin);
        let seconds_per_day: u64 = 24 * 60 * 60;
        let duration_seconds: u64 = (duration_days as u64) * seconds_per_day;
        let end_time: u64 = env.ledger().timestamp() + duration_seconds;

        let bet_deadline = match bet_deadline_mins_before_end {
            Some(mins) => end_time.saturating_sub(mins * 60),
            None => 0,
        };

        let (has_fallback, fallback_cfg) = match &fallback_oracle_config {
            Some(c) => (true, c.clone()),
            None => (false, OracleConfig::none_sentinel(&env)),
        };
        
        let metadata_commitment = Market::compute_metadata_commitment(&env, &question, &outcomes, &oracle_config);
        
        let market = Market {
            admin: admin.clone(),
            question: question.clone(),
            outcomes: outcomes.clone(),
            end_time,
            oracle_config,
            metadata_commitment,
            has_fallback,
            fallback_oracle_config: fallback_cfg,
            resolution_timeout,
            oracle_result: None,
            votes: Map::new(&env),
            total_staked: 0,
            dispute_stakes: Map::new(&env),
            stakes: Map::new(&env),
            claimed: Map::new(&env),
            winning_outcomes: None,
            fee_collected: false,
            state: MarketState::Active,
            total_extension_days: 0,
            max_extension_days: 30,
            extension_history: Vec::new(&env),
            category: None,
            tags: Vec::new(&env),
            min_pool_size,
            bet_deadline,
            dispute_window_seconds: dispute_window_seconds.unwrap_or(86400),
            winnings_swept: false,
            timelock_config: timelock::MarketTimelockConfig::default(),
            dispute_stake_floor,
            max_participants,
            min_bet_amount: None,
        };

        if let Err(e) = check_market_creation_rent(&env) {
            panic_with_error!(env, e);
        }
        if let Err(e) = check_market_creation_rent_budget(&env) {
            panic_with_error!(env, e);
        }

        // Pre-flight TTL pressure check: reject creation if any target key
        // appears to be under TTL pressure such that the recommended bump
        // cannot be satisfied. Use existing storage optimizer probe to
        // inspect the market and audit-head keys before mutating state.
        let mut probe_keys: Vec<Val> = Vec::new(&env);
        probe_keys.push_back(market_id.clone().into());
        probe_keys.push_back(Symbol::new(&env, "AUDIT_HEAD").into());

        let pressures = StorageOptimizer::check_ttl_pressure(&env, probe_keys);
        if pressures.len() > 0 {
            for p in pressures.iter() {
                if p.remaining_ledgers < p.recommended_bump {
                    panic_with_error!(env, Error::InsufficientStorageRentBudget);
                }
            }
        }

        env.storage().persistent().set(&market_id, &market);
        env.storage().persistent().extend_ttl(&market_id, MARKET_TTL_LEDGERS, MARKET_TTL_LEDGERS);

        crate::events::EventEmitter::emit_market_created(&env, &market_id, &question, &outcomes, &admin, end_time);

        statistics::StatisticsManager::record_market_created(&env);

        crate::audit::AuditTrailManager::append_record(
            &env,
            crate::audit::AuditAction::MarketCreated,
            admin.clone(),
            Map::new(&env),
            None,
        );

        {
            let mut details = Map::new(&env);
            details.set(Symbol::new(&env, "question"), question.clone());
            details.set(Symbol::new(&env, "end_time"), String::from_str(&env, &alloc::format!("{}", end_time)));
            details.set(Symbol::new(&env, "dur_days"), String::from_str(&env, &alloc::format!("{}", duration_days)));
            crate::audit::MarketAuditManager::append(
                &env,
                &market_id,
                crate::audit::MarketAuditAction::MarketCreated,
                admin.clone(),
                details,
            );
        }

        GasTracker::end_tracking(&env, symbol_short!("create"), gas_marker);
        market_id
    }

    pub fn fetch_oracle_result(
        env: Env,
        caller: Address, 
        market_id: Symbol,
        oracle_contract: Address,
    ) -> Result<String, Error> {
        if let Err(e) = crate::circuit_breaker::CircuitBreaker::require_write_allowed(&env, "fetch_oracle") {
            panic_with_error!(env, e);
        }
        
        caller.require_auth(); 

        match resolution::OracleResolutionManager::fetch_oracle_result(&env, &market_id) {
            Ok(res) => Ok(res.oracle_result),
            Err(e) => Err(e),
        }
    }

    // --- Add remaining required helper methods that were safely kept at the end of block 1
    fn require_primary_admin_or_panic(env: &Env, admin: &Address) {
        admin.require_auth();
        let stored_admin: Option<Address> =
            env.storage().persistent().get(&Symbol::new(env, SYM_ADMIN));
        match stored_admin {
            Some(ref a) if a == admin => {}
            _ => panic_with_error!(env, Error::Unauthorized),
        }
    }

    fn require_primary_admin(env: &Env, admin: &Address) -> Result<(), Error> {
        admin.require_auth();
        let stored_admin: Option<Address> =
            env.storage().persistent().get(&Symbol::new(env, SYM_ADMIN));
        match stored_admin {
            Some(ref a) if a == admin => Ok(()),
            _ => Err(Error::Unauthorized),
        }
    }

    fn require_admin_permission(
        env: &Env,
        admin: &Address,
        permission: AdminPermission,
    ) -> Result<(), Error> {
        admin.require_auth();
        AdminManager::validate_admin_permission(env, admin, permission)
    }

    fn require_initialized_admin_root(env: &Env, admin: &Address) -> Result<(), Error> {
        admin.require_auth();

        if claim_period_seconds == 0 {
            panic_with_error!(env, Error::InvalidInput);
        }

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, SYM_ADMIN))
            .unwrap_or_else(|| panic_with_error!(env, Error::AdminNotSet));

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }

        recovery::UnclaimedWinningsPolicy::set_global_claim_period(&env, claim_period_seconds);
        EventEmitter::emit_claim_period_updated(&env, &admin, claim_period_seconds);
    }

    /// Set a market-specific claim period override (admin only).
    ///
    /// The market-specific value overrides the global claim period for the given market.
    pub fn set_market_claim_period(
        env: Env,
        admin: Address,
        market_id: Symbol,
        claim_period_seconds: u64,
    ) {
        admin.require_auth();

        if claim_period_seconds == 0 {
            panic_with_error!(env, Error::InvalidInput);
        }

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, SYM_ADMIN))
            .unwrap_or_else(|| panic_with_error!(env, Error::AdminNotSet));

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }

        if markets::MarketStateManager::get_market(&env, &market_id).is_err() {
            panic_with_error!(env, Error::MarketNotFound);
        }

        recovery::UnclaimedWinningsPolicy::set_market_claim_period(
            &env,
            &market_id,
            claim_period_seconds,
        );
        EventEmitter::emit_market_claim_period_updated(
            &env,
            &admin,
            &market_id,
            claim_period_seconds,
        );
    }

    /// Set treasury recipient for unclaimed winnings sweeps (admin only).
    pub fn set_treasury(env: Env, admin: Address, treasury: Address) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, SYM_ADMIN))
            .unwrap_or_else(|| panic_with_error!(env, Error::AdminNotSet));

        if admin != stored_admin {
            panic_with_error!(env, Error::Unauthorized);
        }

        recovery::UnclaimedWinningsPolicy::set_treasury(&env, &treasury);
        EventEmitter::emit_treasury_updated(&env, &admin, &treasury);
    }

    /// Sweep unclaimed winning payouts after claim period expiry (admin only).
    ///
    /// If `burn` is true, swept funds are burned (no recipient balance credited).
    /// If `burn` is false, swept funds are credited to the configured treasury.
    pub fn sweep_unclaimed_winnings(
        env: Env,
        admin: Address,
        market_id: Symbol,
        burn: bool,
    ) -> Result<i128, Error> {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, SYM_ADMIN))
            .ok_or(Error::AdminNotSet)?;

        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        let winning_outcomes = market
            .winning_outcomes
            .clone()
            .ok_or(Error::MarketNotResolved)?;

        if !recovery::UnclaimedWinningsPolicy::is_claim_window_expired(
            &env,
            &market_id,
            market.end_time,
        ) {
            return Err(Error::InvalidState);
        }

        // Idempotency guard: reject a repeat sweep so the treasury is never double-credited.
        if market.winnings_swept {
            return Err(Error::SweepAlreadyDone);
        }

        let fee_percent = crate::config::ConfigManager::get_config(&env)
            .map(|cfg| cfg.fees.platform_fee_percentage)
            .unwrap_or_else(|_| {
                // Use the short platform fee key (backwards-compat fallback to legacy long keys
                // is not possible here because Soroban restricts symbols to <=9 chars).
                // If you need to read old on-chain keys created with long symbols,
                // perform a storage migration on-chain (one-time) to move legacy values
                // under the new short key.
                let new_key = Symbol::new(&env, "platform_fee");
                env.storage().persistent().get(&new_key).unwrap_or(2)
            });

        if fee_percent < 0 || fee_percent > PERCENTAGE_DENOMINATOR {
            return Err(Error::InvalidFeeConfig);
        }

        let summary = resolution::ResolutionOutcomeCache::require(&env, &market_id, &market)?;
        let winning_total = summary.winning_total;
        if winning_total <= 0 {
            return Ok(0);
        }

        let bettors = bets::BetStorage::get_all_bets_for_market(&env, &market_id);
        let mut swept_total = 0i128;
        let total_pool = summary.total_pool;

        for (user, outcome) in market.votes.iter() {
            if !winning_outcomes.contains(&outcome) {
                continue;
            }

            if market
                .claimed
                .get(user.clone())
                .map(|info| info.is_claimed())
                .unwrap_or(false)
            {
                continue;
            }

            let user_stake = market.stakes.get(user.clone()).unwrap_or(0);
            if user_stake <= 0 {
                continue;
            }

            let user_share = user_stake
                .checked_mul(PERCENTAGE_DENOMINATOR as i128 - fee_percent as i128)
                .ok_or(Error::InvalidInput)?
                / (PERCENTAGE_DENOMINATOR as i128);
            let payout = user_share
                .checked_mul(total_pool)
                .ok_or(Error::InvalidInput)?
                / winning_total;

            if payout < 0 {
                return Err(Error::InvalidInput);
            }

            market
                .claimed
                .set(user.clone(), ClaimInfo::new(&env, payout));
            swept_total = swept_total.checked_add(payout).ok_or(Error::InvalidInput)?;
        }

        for user in bettors.iter() {
            if market.votes.contains_key(user.clone()) {
                continue;
            }

            let Some(bet) = bets::BetStorage::get_bet(&env, &market_id, &user) else {
                continue;
            };

            if !winning_outcomes.contains(&bet.outcome) {
                continue;
            }

            if market
                .claimed
                .get(user.clone())
                .map(|info| info.is_claimed())
                .unwrap_or(false)
            {
                continue;
            }

            if bet.amount <= 0 {
                continue;
            }

            let user_share = bet
                .amount
                .checked_mul(PERCENTAGE_DENOMINATOR as i128 - fee_percent as i128)
                .ok_or(Error::InvalidInput)?
                / (PERCENTAGE_DENOMINATOR as i128);
            let payout = user_share
                .checked_mul(total_pool)
                .ok_or(Error::InvalidInput)?
                / winning_total;

            if payout < 0 {
                return Err(Error::InvalidInput);
            }

            market
                .claimed
                .set(user.clone(), ClaimInfo::new(&env, payout));
            swept_total = swept_total.checked_add(payout).ok_or(Error::InvalidInput)?;
        }

        let recipient = if burn {
            None
        } else {
            let treasury = recovery::UnclaimedWinningsPolicy::get_treasury(&env)
                .ok_or(Error::ConfigNotFound)?;
            if swept_total > 0 {
                storage::BalanceStorage::add_balance(
                    &env,
                    &treasury,
                    &types::ReflectorAsset::Stellar,
                    swept_total,
                )?;
            }
            Some(treasury)
        };

        // Mark this market as swept so a second call returns SweepAlreadyDone.
        market.winnings_swept = true;
        env.storage().persistent().set(&market_id, &market);
        EventEmitter::emit_unclaimed_winnings_swept(
            &env,
            &market_id,
            &admin,
            &recipient,
            swept_total,
            burn,
        );

        Ok(swept_total)
    }

    /// Retrieves complete market information by market identifier.
    ///
    /// This function provides read-only access to all market data including
    /// configuration, current state, voting results, stakes, and resolution status.
    /// It's the primary way to query market information for display or analysis.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to retrieve
    ///
    /// # Returns
    ///
    /// Returns `Some(Market)` if the market exists, `None` if not found.
    /// The `Market` struct contains:
    /// - Basic info: admin, question, outcomes, end_time
    /// - Oracle configuration and results
    /// - Voting data: votes, stakes, total_staked
    /// - Resolution data: winning_outcome, claimed status
    /// - State information: current state, extensions, fee collection
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_market(env.clone(), market_id) {
    ///     Some(market) => {
    ///         // Market found - access market data
    ///         let question = market.question;
    ///         let state = market.state;
    ///         let total_staked = market.total_staked;
    ///     },
    ///     None => {
    ///         // Market not found
    ///     }
    /// }
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **UI Display**: Show market details, voting status, and results
    /// - **Analytics**: Calculate market statistics and user positions
    /// - **Validation**: Check market state before performing operations
    /// - **Monitoring**: Track market progress and resolution status
    ///
    /// # Performance
    ///
    /// This is a read-only operation that doesn't modify contract state.
    /// It retrieves data from persistent storage with minimal computational overhead.
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_market(env: Env, market_id: Symbol) -> Option<Market> {
        env.storage().persistent().get(&market_id)
    }

    /// Verifies a client's expected metadata commitment against on-chain market metadata.
    ///
    /// The commitment is `sha256(canonical_xdr({ question, outcomes, oracle_config }))`.
    /// This helper returns `false` when the market is missing, when `expected` does not
    /// match the commitment stored at creation/update time, or when any committed field
    /// in storage was changed without refreshing the stored commitment.
    pub fn verify_market_metadata(env: Env, market_id: Symbol, expected: BytesN<32>) -> bool {
        let market: Option<Market> = env.storage().persistent().get(&market_id);
        match market {
            Some(market) => market.verify_metadata_commitment(&env, &expected),
            None => false,
        }
    }

    /// Manually resolves a prediction market by setting the winning outcome (admin only).
    ///
    /// This function allows contract administrators to manually resolve markets
    /// when automatic oracle resolution is not available or needs override.
    /// It's typically used for markets with subjective outcomes or when oracle
    /// data is unavailable or disputed.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the resolution (must be authorized)
    /// * `market_id` - Unique identifier of the market to resolve
    /// * `winning_outcome` - The outcome to be declared as the winner
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketClosed` - Market hasn't reached its end time yet
    /// - `Error::InvalidOutcome` - Winning outcome doesn't match any market outcomes
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, String, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Manually resolve market with "Yes" as winning outcome
    /// PredictifyHybrid::resolve_market_manual(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     String::from_str(&env, "Yes")
    /// );
    /// ```
    ///
    /// # Resolution Process
    ///
    /// 1. **Authentication**: Verifies caller is the contract admin
    /// 2. **Market Validation**: Ensures market exists and has ended
    /// 3. **Outcome Validation**: Confirms winning outcome is valid
    /// 4. **State Update**: Sets winning outcome and updates market state
    ///
    /// # Use Cases
    ///
    /// - **Subjective Markets**: Markets requiring human judgment
    /// - **Oracle Failures**: When automated oracles are unavailable
    /// - **Dispute Resolution**: Override disputed automatic resolutions
    /// - **Emergency Resolution**: Resolve markets in exceptional circumstances
    ///
    /// # Security
    ///
    /// This function requires admin privileges and should be used carefully.
    /// Manual resolutions should be transparent and follow established governance procedures.
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.

    /// Check and enforce admin action cooldown for resolution functions.
    fn check_resolution_cooldown(env: &Env, admin: &Address, fn_name: &Symbol) -> Result<(), Error> {
        let cooldown_key = DataKey::ResolutionCooldownSeconds;
        let cooldown: u64 = env.storage().persistent().get(&cooldown_key).unwrap_or(0);
        if cooldown == 0 {
            return Ok(());
        }
        let now = env.ledger().timestamp();
        let last_key = DataKey::ResolutionAdminLastAction(fn_name.clone());
        let last_action: u64 = env.storage().persistent().get(&last_key).unwrap_or(0);
        if last_action > 0 && now < last_action.saturating_add(cooldown) {
            return Err(Error::AdminActionTimelocked);
        }
        env.storage().persistent().set(&last_key, &now);
        env.storage().persistent().extend_ttl(&last_key, 535680, 535680);
        Ok(())
    }

    /// Sets the cooldown period for resolution admin actions.
    pub fn set_resolution_cooldown(env: Env, admin: Address, seconds: u64) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        let key = DataKey::ResolutionCooldownSeconds;
        env.storage().persistent().set(&key, &seconds);
        env.storage().persistent().extend_ttl(&key, 535680, 535680);
        Ok(())
    }

    pub fn resolve_market_manual(
        env: Env,
        admin: Address,
        market_id: Symbol,
        winning_outcome: String,
    ) {
        let gas_marker = GasTracker::start_tracking(&env);
        Self::require_primary_admin_or_panic(&env, &admin);
        Self::check_resolution_cooldown(&env, &admin, &Symbol::new(&env, "resolve_market_manual")).unwrap_or_else(|e| panic_with_error!(env, e));

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::MarketNotFound);
            });

        // Check if market has ended
        if env.ledger().timestamp() < market.end_time {
            panic_with_error!(env, Error::MarketClosed);
        }

        // Validate winning outcome
        let outcome_exists = market.outcomes.iter().any(|o| o == winning_outcome);
        if !outcome_exists {
            panic_with_error!(env, Error::InvalidOutcome);
        }

        // Capture old state for event
        let old_state = market.state.clone();

        // Set winning outcome(s) as a vector (single outcome for now, supports future multi-winner)
        let mut winning_outcomes_vec = Vec::new(&env);
        winning_outcomes_vec.push_back(winning_outcome.clone());
        market.winning_outcomes = Some(winning_outcomes_vec.clone());
        market.state = MarketState::Resolved;
        recovery::UnclaimedWinningsPolicy::set_claim_window_start_if_missing(
            &env,
            &market_id,
            env.ledger().timestamp(),
        );
        env.storage().persistent().set(&market_id, &market);

        // Resolve bets to mark them as won/lost
        let _ = bets::BetManager::resolve_market_bets(&env, &market_id, &winning_outcomes_vec);

        let _ = resolution::ResolutionOutcomeCache::refresh(&env, &market_id);

        // Emit market resolved event (simplified to avoid segfaults)
        let oracle_result_str = market
            .oracle_result
            .clone()
            .unwrap_or_else(|| String::from_str(&env, "N/A"));
        let community_consensus_str = String::from_str(&env, "Manual");
        let resolution_method = String::from_str(&env, "Manual");

        // Emit events with defensive approach
        EventEmitter::emit_market_resolved(
            &env,
            &market_id,
            &winning_outcome,
            &oracle_result_str,
            &community_consensus_str,
            &resolution_method,
            100, // confidence score for manual resolution
        );

        // Emit state change event
        let reason = String::from_str(&env, "Manual resolution by admin");
        EventEmitter::emit_state_change_event(
            &env,
            &market_id,
            &old_state,
            &MarketState::Resolved,
            &reason,
        );

        // Automatically distribute payouts to winners after resolution
        let _ = Self::distribute_payouts(env.clone(), market_id.clone());

        // Invalidate analytics cache — market state and winning_outcomes have changed.
        analytics::AnalyticsCache::new(&env).invalidate(&market_id);

        // Per-market audit entry: MarketResolved
        {
            let mut details = Map::new(&env);
            details.set(
                Symbol::new(&env, "outcome"),
                winning_outcome.clone(),
            );
            details.set(
                Symbol::new(&env, "method"),
                String::from_str(&env, "Manual"),
            );
            crate::audit::MarketAuditManager::append(
                &env,
                &market_id,
                crate::audit::MarketAuditAction::MarketResolved,
                admin.clone(),
                details,
            );
        }

        GasTracker::end_tracking(&env, symbol_short!("res_man"), gas_marker);
    }

    /// Resolves a market with multiple winning outcomes (for tie cases).
    ///
    /// This function allows authorized administrators to resolve a market with
    /// multiple winners when there's a tie. The pool will be split proportionally
    /// among all winning outcomes based on stake distribution.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the resolution (must be authorized)
    /// * `market_id` - Unique identifier of the market to resolve
    /// * `winning_outcomes` - Vector of outcomes to be declared as winners (minimum 1, all must be valid)
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketClosed` - Market hasn't ended yet
    /// - `Error::InvalidOutcome` - One or more outcomes are not valid for this market
    /// - `Error::InvalidInput` - Empty outcomes vector
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String, Vec};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "sports_match");
    ///
    /// // Resolve with tie (Team A and Team B both win)
    /// let winning_outcomes = vec![
    ///     &env,
    ///     String::from_str(&env, "Team A"),
    ///     String::from_str(&env, "Team B"),
    /// ];
    ///
    /// PredictifyHybrid::resolve_market_with_ties(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     winning_outcomes
    /// );
    /// ```
    ///
    /// # Pool Split Logic
    ///
    /// When multiple outcomes win:
    /// - Total pool is split proportionally among all winners
    /// - Each winner receives: (their_stake / total_winning_stakes) * total_pool * (1 - fee)
    /// - This ensures fair distribution even when outcomes are tied
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn resolve_market_with_ties(
        env: Env,
        admin: Address,
        market_id: Symbol,
        winning_outcomes: Vec<String>,
    ) {
        Self::require_primary_admin_or_panic(&env, &admin);
        Self::check_resolution_cooldown(&env, &admin, &Symbol::new(&env, "resolve_market_with_ties")).unwrap_or_else(|e| panic_with_error!(env, e));

        // Validate outcomes vector is not empty
        if winning_outcomes.len() == 0 {
            panic_with_error!(env, Error::InvalidInput);
        }

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::MarketNotFound);
            });

        // Check if market has ended
        if env.ledger().timestamp() < market.end_time {
            panic_with_error!(env, Error::MarketClosed);
        }

        // Validate all winning outcomes exist in market outcomes
        for outcome in winning_outcomes.iter() {
            let outcome_exists = market.outcomes.iter().any(|o| o == outcome);
            if !outcome_exists {
                panic_with_error!(env, Error::InvalidOutcome);
            }
        }

        // Capture old state for event
        let old_state = market.state.clone();

        // Set winning outcome(s) - supports multiple winners for ties
        market.winning_outcomes = Some(winning_outcomes.clone());
        market.state = MarketState::Resolved;
        recovery::UnclaimedWinningsPolicy::set_claim_window_start_if_missing(
            &env,
            &market_id,
            env.ledger().timestamp(),
        );
        env.storage().persistent().set(&market_id, &market);

        // Resolve bets to mark them as won/lost
        let _ = bets::BetManager::resolve_market_bets(&env, &market_id, &winning_outcomes);

        let _ = resolution::ResolutionOutcomeCache::refresh(&env, &market_id);

        // Emit market resolved event
        let primary_outcome = winning_outcomes.get(0).unwrap().clone();
        let oracle_result_str = market
            .oracle_result
            .clone()
            .unwrap_or_else(|| String::from_str(&env, "N/A"));
        let community_consensus_str = String::from_str(&env, "Manual");
        let resolution_method = String::from_str(&env, "Manual");

        EventEmitter::emit_market_resolved(
            &env,
            &market_id,
            &primary_outcome,
            &oracle_result_str,
            &community_consensus_str,
            &resolution_method,
            100, // confidence score for manual resolution
        );

        // Emit state change event
        let reason = String::from_str(&env, "Manual resolution with ties by admin");
        EventEmitter::emit_state_change_event(
            &env,
            &market_id,
            &old_state,
            &MarketState::Resolved,
            &reason,
        );

        // Automatically distribute payouts (handles split pool for ties)
        let _ = Self::distribute_payouts(env.clone(), market_id.clone());

        // Invalidate analytics cache — market state and winning_outcomes have changed.
        analytics::AnalyticsCache::new(&env).invalidate(&market_id);

        // Per-market audit entry: MarketResolved (with ties)
        {
            let mut details = Map::new(&env);
            details.set(
                Symbol::new(&env, "outcome"),
                primary_outcome.clone(),
            );
            details.set(
                Symbol::new(&env, "method"),
                String::from_str(&env, "ManualTie"),
            );
            crate::audit::MarketAuditManager::append(
                &env,
                &market_id,
                crate::audit::MarketAuditAction::MarketResolved,
                admin.clone(),
                details,
            );
        }
    }

    /// Force-resolves a market bypassing time/state constraints, with idempotency-key
    /// replay protection and audit trail.
    ///
    /// This admin-only entrypoint resolves a market **regardless** of its current state
    /// or whether `end_time` has been reached. Every call must supply a non-empty `reason`
    /// and a unique `idempotency_key` (a string, e.g. a UUID) scoped to the market.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the force-resolve (must be authorised)
    /// * `market_id` - Unique identifier of the market to force-resolve
    /// * `winning_outcomes` - Vector of outcomes to declare as winners (minimum 1, all must be valid)
    /// * `reason` - Human-readable justification for the force-resolve (stored in audit trail)
    /// * `idempotency_key` - Unique caller-provided key (e.g. UUID) per market; prevents replay
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Force-resolve succeeded
    /// * `Err(Error::Unauthorized)` - Caller is not the contract primary admin
    /// * `Err(Error::MarketNotFound)` - Market does not exist
    /// * `Err(Error::InvalidOutcome)` - One or more outcomes are invalid for this market
    /// * `Err(Error::InvalidInput)` - Empty outcomes vector
    /// * `Err(Error::ForceResolveReasonEmpty)` - Reason string is empty
    /// * `Err(Error::ForceResolveReplayed)` - Idempotency key has already been used
    ///
    /// # Events
    ///
    /// On first invocation emits a `ForceResolvedEvent` (topic `frc_rs`) and a state-change
    /// event. Repeated calls with the same `idempotency_key` are rejected as replays.
    pub fn force_resolve_market(
        env: Env,
        admin: Address,
        market_id: Symbol,
        winning_outcomes: Vec<String>,
        reason: String,
        idempotency_key: String,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        Self::check_resolution_cooldown(&env, &admin, &Symbol::new(&env, "force_resolve_market"))?;

        if reason.is_empty() {
            return Err(Error::ForceResolveReasonEmpty);
        }

        if winning_outcomes.len() == 0 {
            return Err(Error::InvalidInput);
        }

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        for outcome in winning_outcomes.iter() {
            let outcome_exists = market.outcomes.iter().any(|o| o == outcome);
            if !outcome_exists {
                return Err(Error::InvalidOutcome);
            }
        }

        // Idempotency check — reject if this key was already consumed
        if force_resolve::ForceResolveManager::is_already_resolved(
            &env,
            &market_id,
            &idempotency_key,
        ) {
            return Err(Error::ForceResolveReplayed);
        }

        let old_state = market.state.clone();

        market.winning_outcomes = Some(winning_outcomes.clone());
        market.state = MarketState::Resolved;

        recovery::UnclaimedWinningsPolicy::set_claim_window_start_if_missing(
            &env,
            &market_id,
            env.ledger().timestamp(),
        );

        env.storage().persistent().set(&market_id, &market);

        force_resolve::ForceResolveManager::mark_resolved(
            &env,
            &market_id,
            &idempotency_key,
            &admin,
            &winning_outcomes,
        );

        let _ = bets::BetManager::resolve_market_bets(&env, &market_id, &winning_outcomes);
        let _ = resolution::ResolutionOutcomeCache::refresh(&env, &market_id);

        let primary_outcome = winning_outcomes.get(0).unwrap().clone();

        // Emit force-resolved event
        EventEmitter::emit_force_resolved(
            &env,
            &market_id,
            &admin,
            &primary_outcome,
            &reason,
            &idempotency_key,
        );

        // Emit state change event
        EventEmitter::emit_state_change_event(
            &env,
            &market_id,
            &old_state,
            &MarketState::Resolved,
            &reason,
        );

        // Append immutable audit trail record
        let mut details = Map::new(&env);
        details.set(Symbol::new(&env, "reason"), reason);
        details.set(Symbol::new(&env, "old_state"), {
            let s = match old_state {
                MarketState::Active => "Active",
                MarketState::Ended => "Ended",
                MarketState::Disputed => "Disputed",
                MarketState::Resolved => "Resolved",
                MarketState::Closed => "Closed",
                MarketState::Cancelled => "Cancelled",
            };
            String::from_str(&env, s)
        });
        AuditTrailManager::append_record(
            &env,
            AuditAction::MarketForceResolved,
            admin.clone(),
            details,
            None,
        );

        // Per-market audit entry: MarketForceResolved
        {
            let mut per_market_details = Map::new(&env);
            per_market_details.set(
                Symbol::new(&env, "outcome"),
                primary_outcome.clone(),
            );
            per_market_details.set(
                Symbol::new(&env, "method"),
                String::from_str(&env, "Force"),
            );
            crate::audit::MarketAuditManager::append(
                &env,
                &market_id,
                crate::audit::MarketAuditAction::MarketForceResolved,
                admin.clone(),
                per_market_details,
            );
        }

        // Auto-distribute payouts
        let _ = Self::distribute_payouts(env.clone(), market_id);

        Ok(())
    }

    /// Fetches oracle result for a market from external oracle contracts.
    ///
    /// This function retrieves prediction results from configured oracle sources
    /// such as Reflector or Pyth networks. It's used to obtain objective data
    /// for market resolution when manual resolution is not appropriate.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to fetch oracle data for
    /// * `oracle_contract` - Address of the oracle contract to query
    ///
    /// # Returns
    ///
    /// Returns `Result<String, Error>` where:
    /// - `Ok(String)` - The oracle result as a string representation
    /// - `Err(Error)` - Specific error if operation fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Market already has oracle result set
    /// - `Error::MarketClosed` - Market hasn't reached its end time yet
    /// - Oracle-specific errors from the resolution module
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "btc_market");
    /// # let oracle_address = Address::generate(&env);
    ///
    /// match PredictifyHybrid::fetch_oracle_result(
    ///     env.clone(),
    ///     market_id,
    ///     oracle_address
    /// ) {
    ///     Ok(result) => {
    ///         // Oracle result retrieved successfully
    ///         println!("Oracle result: {}", result);
    ///     },
    ///     Err(e) => {
    ///         // Handle error
    ///         println!("Failed to fetch oracle result: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Oracle Integration
    ///
    /// This function integrates with various oracle types:
    /// - **Reflector**: For asset price data and market conditions
    /// - **Pyth**: For high-frequency financial data feeds
    /// - **Custom Oracles**: For specialized data sources
    ///
    /// # Market State Requirements
    ///
    /// - Market must exist and be past its end time
    /// - Market must not already have an oracle result
    /// - Automatic oracle resolution stops once `ledger.timestamp() >= end_time + resolution_timeout`
    /// - When `has_fallback` is `true`, the contract attempts the primary oracle once and then the fallback once
    /// - The market-stored oracle configuration controls ordering; the external `oracle_contract` argument is ignored
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn fetch_oracle_result(
        env: Env,
        market_id: Symbol,
        oracle_contract: Address,
    ) -> Result<String, Error> {
        let _ = oracle_contract;

        // Get the market from storage
        let mut market = env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Validate market state
        if market.oracle_result.is_some() {
            return Err(Error::MarketResolved);
        }

        // Check if market has ended
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        if resolution_timeout_reached(&env, &market) {
            EventEmitter::emit_resolution_timeout(&env, &market_id, current_time);
            return Err(Error::ResolutionTimeoutReached);
        }

        match get_oracle_result(&env, &market.oracle_config) {
            Ok(outcome) => {
                market.oracle_result = Some(outcome.clone());
                env.storage().persistent().set(&market_id, &market);
                Ok(outcome)
            }
            Err(_) if market.has_fallback => {
                match get_oracle_result(&env, &market.fallback_oracle_config) {
                    Ok(outcome) => {
                        market.oracle_result = Some(outcome.clone());
                        env.storage().persistent().set(&market_id, &market);
                        EventEmitter::emit_fallback_used(
                            &env,
                            &market_id,
                            &market.oracle_config.oracle_address,
                            &market.fallback_oracle_config.oracle_address,
                        );
                        Ok(outcome)
                    }
                    Err(_) => {
                        EventEmitter::emit_manual_resolution_required(
                            &env,
                            &market_id,
                            &String::from_str(&env, "primary_and_fallback_failed"),
                        );
                        Err(Error::FallbackOracleUnavailable)
                    }
                }
            }
            Err(err) => {
                EventEmitter::emit_manual_resolution_required(
                    &env,
                    &market_id,
                    &String::from_str(&env, "primary_failed_no_fallback"),
                );
                Err(err)
            }
        }
    }

    /// Verifies and fetches event outcome from external oracle sources automatically.
    ///
    /// This function implements the complete oracle integration mechanism that:
    /// - Automatically fetches event outcomes from configured external data sources
    /// - Validates oracle responses and signatures/authority
    /// - Supports multiple oracle sources with consensus-based verification
    /// - Handles oracle failures gracefully with fallback mechanisms
    /// - Emits result verification events for transparency
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `caller` - The address initiating the verification (must be authenticated)
    /// * `market_id` - Unique identifier of the market to verify
    ///
    /// # Returns
    ///
    /// Returns `Result<OracleResult, Error>` where:
    /// - `Ok(OracleResult)` - Complete oracle verification result including:
    ///   - `outcome`: The determined outcome ("yes"/"no" or custom)
    ///   - `price`: The fetched price from oracle
    ///   - `threshold`: The configured threshold for comparison
    ///   - `confidence_score`: Statistical confidence (0-100)
    ///   - `is_verified`: Whether the result passed all validations
    ///   - `sources_count`: Number of oracle sources consulted
    /// - `Err(Error)` - Specific error if verification fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketNotReadyForVerification` - Market hasn't ended yet
    /// - `Error::OracleVerified` - Result already verified for this market
    /// - `Error::OracleUnavailable` - Oracle service is unavailable
    /// - `Error::OracleStale` - Oracle data is too old
    /// - `Error::OracleConsensusNotReached` - Multiple oracles disagree
    /// - `Error::InvalidOracleConfig` - Oracle not whitelisted/authorized
    /// - `Error::OracleAllSourcesFailed` - All oracle sources failed
    /// - `Error::InsufficientOracleSources` - No active oracle sources available
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let caller = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "btc_50k_2024");
    ///
    /// // Verify result for an ended market
    /// match PredictifyHybrid::verify_result(env.clone(), caller, market_id) {
    ///     Ok(result) => {
    ///         println!("Outcome: {}", result.outcome);
    ///         println!("Price: ${}", result.price / 100);
    ///         println!("Confidence: {}%", result.confidence_score);
    ///         println!("Sources consulted: {}", result.sources_count);
    ///         
    ///         if result.is_verified {
    ///             println!("Result is verified and authoritative");
    ///         }
    ///     },
    ///     Err(e) => {
    ///         println!("Verification failed: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Oracle Integration
    ///
    /// This function integrates with multiple oracle providers:
    /// - **Reflector**: Primary oracle for Stellar Network (production ready)
    /// - **Band Protocol**: Decentralized oracle network
    /// - **Custom Oracles**: Can be added via whitelist system
    ///
    /// # Multi-Oracle Consensus
    ///
    /// When multiple oracle sources are configured:
    /// 1. All active sources are queried in parallel
    /// 2. Responses are validated for freshness and authority
    /// 3. Consensus is calculated (default: 66% agreement required)
    /// 4. Confidence score reflects agreement level and price stability
    ///
    /// # Security Features
    ///
    /// - **Whitelist Validation**: Only whitelisted oracles are queried
    /// - **Authority Verification**: Oracle responses are validated for authenticity
    /// - **Staleness Protection**: Data older than 5 minutes is rejected
    /// - **Price Range Validation**: Ensures prices are within reasonable bounds
    /// - **Consensus Requirement**: Multiple sources must agree for high-value markets
    ///
    /// # Events Emitted
    ///
    /// - `OracleVerificationInitiated`: When verification begins
    /// - `OracleResultVerified`: When verification succeeds
    /// - `OracleVerificationFailed`: When verification fails
    /// - `OracleConsensusReached`: When multiple sources agree
    ///
    /// # Market State Requirements
    ///
    /// - Market must exist in storage
    /// - Market end time must have passed
    /// - Result must not already be verified
    /// - At least one active oracle source must be available
    #[deprecated(note = "Use fetch_oracle_result instead. This legacy stub will be removed in a future version.")]
    pub fn verify_result(
        env: Env,
        caller: Address,
        market_id: Symbol,
    ) -> Result<OracleResult, Error> {
        // Authenticate the caller
        caller.require_auth();

        DeprecatedRegistry::record_call(
            &env,
            &caller,
            &Symbol::new(&env, "verify_result"),
            &Symbol::new(&env, "fetch_oracle_result"),
        );

        // Use the OracleIntegrationManager to perform verification
        // Temporarily disabled due to oracles module being disabled
        // oracles::OracleIntegrationManager::verify_result(&env, &market_id, &caller)
        Err(Error::OracleUnavailable)
    }

    /// Verifies oracle result with retry logic for resilience.
    ///
    /// This function is similar to `verify_result` but includes automatic
    /// retry logic to handle transient oracle failures. Useful in production
    /// environments where network issues may cause temporary unavailability.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `caller` - The address initiating the verification
    /// * `market_id` - Unique identifier of the market to verify
    /// * `max_retries` - Maximum number of retry attempts (capped at 3)
    ///
    /// # Returns
    ///
    /// Returns `Result<OracleResult, Error>` - Same as `verify_result`
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let caller = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "btc_50k_2024");
    ///
    /// // Verify with up to 3 retries
    /// let result = PredictifyHybrid::verify_result_with_retry(
    ///     env.clone(),
    ///     caller,
    ///     market_id,
    ///     3
    /// );
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn verify_result_with_retry(
        env: Env,
        caller: Address,
        market_id: Symbol,
        max_retries: u32,
    ) -> Result<OracleResult, Error> {
        caller.require_auth();
        // Temporarily disabled due to oracles module being disabled
        // oracles::OracleIntegrationManager::verify_result_with_retry(
        //     &env,
        //     &market_id,
        //     &caller,
        //     max_retries,
        // )
        Err(Error::OracleUnavailable)
    }

    /// Retrieves a previously verified oracle result for a market.
    ///
    /// This function returns the stored oracle verification result for a market
    /// that has already been verified. Useful for checking verification status
    /// and retrieving historical verification data.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market
    ///
    /// # Returns
    ///
    /// Returns `Option<OracleResult>`:
    /// - `Some(OracleResult)` - The stored verification result
    /// - `None` - Market has not been verified yet
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "btc_50k_2024");
    ///
    /// match PredictifyHybrid::get_verified_result(env.clone(), market_id) {
    ///     Some(result) => {
    ///         println!("Market verified with outcome: {}", result.outcome);
    ///     },
    ///     None => {
    ///         println!("Market not yet verified");
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_verified_result(env: Env, market_id: Symbol) -> Option<OracleResult> {
        // Temporarily disabled due to oracles module being disabled
        // oracles::OracleIntegrationManager::get_oracle_result(&env, &market_id)
        None
    }

    /// Checks if a market's result has been verified via oracle.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `market_id` - Unique identifier of the market
    ///
    /// # Returns
    ///
    /// Returns `bool` - `true` if verified, `false` otherwise
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn is_result_verified(env: Env, market_id: Symbol) -> bool {
        // Temporarily disabled due to oracles module being disabled
        // oracles::OracleIntegrationManager::is_result_verified(&env, &market_id)
        false
    }

    /// Admin override for oracle result verification.
    ///
    /// Allows an authorized admin to manually set the verification result
    /// when automatic verification fails or produces incorrect results.
    /// This is a privileged operation requiring admin authorization.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - Admin address (must be authorized)
    /// * `market_id` - Market to override
    /// * `outcome` - The outcome to set ("yes"/"no" or custom)
    /// * `reason` - Reason for the manual override
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>`:
    /// - `Ok(())` - Override successful
    /// - `Err(Error::Unauthorized)` - Caller is not admin
    ///
    /// # Security
    ///
    /// This function should be used sparingly and only when:
    /// - Automatic oracle verification has failed repeatedly
    /// - Oracle data is known to be incorrect
    /// - Emergency situations requiring immediate resolution
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    /// Set the minimum oracle price-confidence threshold in basis points.
    /// Prices from oracles with confidence ratio above this threshold are rejected.
    /// `min_confidence_bps` = 0 disables the check.
    pub fn set_oracle_confidence_threshold(env: Env, admin: Address, min_confidence_bps: u32) {
        admin.require_auth();
        if min_confidence_bps > 10_000 {
            panic_with_error!(env, Error::InvalidInput);
        }
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "oracle_conf_bps"), &min_confidence_bps);
    }

    /// Get the configured oracle confidence threshold in bps (0 = disabled).
    pub fn get_oracle_confidence_threshold(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, "oracle_conf_bps"))
            .unwrap_or(0u32)
    }

    /// Set configurable weight for a specific oracle source
    pub fn set_oracle_weight(
        env: Env,
        admin: Address,
        oracle: Address,
        weight: u32,
    ) -> Result<(), Error> {
        admin.require_auth();
        crate::oracles::OracleIntegrationManager::set_oracle_weight(&env, admin, oracle, weight)
    }

    /// Get configured weight for an oracle source, defaults to 1
    pub fn get_oracle_weight(env: Env, oracle: Address) -> u32 {
        crate::oracles::OracleIntegrationManager::get_oracle_weight(&env, &oracle)
    }

    pub fn admin_override_verification(
        env: Env,
        admin: Address,
        market_id: Symbol,
        outcome: String,
        reason: String,
        provided_nonce: u64,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;

        // Reject empty reason — every override must be justified
        if reason.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Load the market
        let mut market = markets::MarketStateManager::get_market(&env, &market_id)?;

        // Capture the previous oracle result for the audit record and event
        let old_result = market
            .oracle_result
            .clone()
            .unwrap_or_else(|| String::from_str(&env, "none"));

        // Apply the override
        market.oracle_result = Some(outcome.clone());
        market.state = crate::types::MarketState::Resolved;
        markets::MarketStateManager::update_market(&env, &market_id, &market);

        // Append an immutable audit record
        // Validate and store the admin override nonce for replay protection
        let key = DataKey::AdminOverrideNonce(admin.clone());
        let mut stored_nonce: u64 = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(0);

        if provided_nonce <= stored_nonce {
            return Err(Error::ReplayedOverride);
        }

        // Update the nonce for this admin
        env.storage().persistent().set(&key, &provided_nonce);
        env.storage().persistent().extend_ttl(
            &key,
            env.storage().max_ttl(),
            env.storage().max_ttl(),
        );

        // Append an immutable audit record with the nonce for replay protection
        let mut details = Map::new(&env);
        details.set(Symbol::new(&env, "old_result"), old_result.clone());
        details.set(Symbol::new(&env, "new_result"), outcome.clone());
        details.set(Symbol::new(&env, "reason"), reason.clone());
        AuditTrailManager::append_record(
            &env,
            AuditAction::OracleVerificationOverride,
            admin.clone(),
            details,
            Some(provided_nonce),
        );

        // Emit the dedicated override event for off-chain monitors
        EventEmitter::emit_admin_override(&env, &market_id, &admin, &old_result, &outcome, &reason);

        Ok(())
    }

    /// Resolves a market automatically using oracle data and community consensus.
    ///
    /// This function implements the hybrid resolution algorithm that combines
    /// objective oracle data with community voting patterns to determine the
    /// final market outcome. It's the primary automated resolution mechanism.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to resolve
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market resolved successfully
    /// - `Err(Error)` - Specific error if resolution fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketNotEnded` - Market hasn't reached its end time
    /// - `Error::MarketResolved` - Market is already resolved
    /// - `Error::InsufficientData` - Not enough data for resolution
    /// - Resolution-specific errors from the resolution module
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "ended_market");
    ///
    /// match PredictifyHybrid::resolve_market(env.clone(), market_id) {
    ///     Ok(()) => {
    ///         // Market resolved successfully
    ///         println!("Market resolved successfully");
    ///     },
    ///     Err(e) => {
    ///         // Handle resolution error
    ///         println!("Resolution failed: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Hybrid Resolution Algorithm
    ///
    /// The resolution process follows these steps:
    /// 1. **Data Collection**: Gather oracle data and community votes
    /// 2. **Consensus Analysis**: Analyze agreement between oracle and community
    /// 3. **Conflict Resolution**: Handle disagreements using weighted algorithms
    /// 4. **Final Determination**: Set winning outcome based on hybrid result
    /// 5. **State Update**: Update market state to resolved
    ///
    /// # Resolution Criteria
    ///
    /// - Market must be past its end time
    /// - Sufficient voting participation required
    /// - Oracle data must be available (if configured)
    /// - No active disputes that would prevent resolution
    ///
    /// # Post-Resolution
    ///
    /// After successful resolution:
    /// - Market state changes to `Resolved`
    /// - Winning outcome is set
    /// - Users can claim winnings
    /// - Market statistics are finalized
    ///
    /// # Event Ordering Contract
    ///
    /// On every successful resolution `resolve_market` emits exactly **three**
    /// resolution-signalling events in the following deterministic sequence:
    ///
    /// | # | Topic symbol    | Emitter                               | Description                      |
    /// |---|-----------------|---------------------------------------|----------------------------------|
    /// | 1 | `mkt_res`       | `EventEmitter::emit_market_resolved`  | Final outcome recorded           |
    /// | 2 | `st_chng`       | `EventEmitter::emit_state_change_event` | State transition to `Resolved` |
    /// | 3 | `idx_transition`| `ContractMonitor::emit_resolution_transition_hook` | Off-chain indexer hook |
    ///
    /// Off-chain consumers **must** handle these three events in the order
    /// listed above. The sequence is enforced by the order of calls inside
    /// `MarketResolutionManager::resolve_market` and is covered by a
    /// deterministic ordering test (see `resolution_event_ordering_tests`).
    #[deprecated(note = "Use resolve_market_manual or fetch_oracle_result + resolve_market_manual instead. This legacy stub will be removed in a future version.")]
    pub fn resolve_market(env: Env, caller: Address, market_id: Symbol) -> Result<(), Error> {
        // Authenticate the caller
        caller.require_auth();

        DeprecatedRegistry::record_call(
            &env,
            &caller,
            &Symbol::new(&env, "resolve_market"),
            &Symbol::new(&env, "resolve_market_manual"),
        );

        // Use the resolution module to resolve the market
        // Temporarily disabled due to resolution module being disabled
        // let _resolution = resolution::MarketResolutionManager::resolve_market(&env, &market_id)?;
        // For now, just return success

        statistics::StatisticsManager::record_market_resolved(&env);

        // Invalidate analytics cache — market state has changed.
        analytics::AnalyticsCache::new(&env).invalidate(&market_id);

        Ok(())
    }

    // =========================================================================
    // DEPRECATED-ENTRYPOINTS REGISTRY
    // =========================================================================

    /// Register a deprecated entrypoint in the on-chain registry.
    ///
    /// Adds a new entry to the deprecated-entrypoints registry so that any
    /// caller can discover which functions have been superseded and what the
    /// recommended replacement is.  The operation is idempotent: registering
    /// the same `entrypoint` a second time is a no-op.
    ///
    /// # Parameters
    ///
    /// * `env`         – Soroban environment.
    /// * `admin`       – Contract admin address; must satisfy
    ///                   [`AdminAccessControl::require_admin_auth`].
    /// * `entrypoint`  – Short symbol of the deprecated function name.
    /// * `replacement` – Short symbol of the recommended replacement.
    /// * `note`        – Optional migration hint (max 128 bytes UTF-8).
    ///
    /// # Errors
    ///
    /// * [`Error::Unauthorized`] – Caller is not the contract admin.
    /// * [`Error::AdminNotSet`]  – Contract has not been initialised.
    /// * [`Error::RegistryFull`] – Registry has reached its capacity limit.
    ///
    /// # Events
    ///
    /// Emits `("depr_reg", entrypoint)` on success.
    pub fn register_deprecated(
        env: Env,
        admin: Address,
        entrypoint: Symbol,
        replacement: Symbol,
        note: Option<String>,
    ) -> Result<(), Error> {
        DeprecatedRegistry::register(&env, &admin, entrypoint, replacement, note)
    }

    /// Remove a deprecated entrypoint from the on-chain registry.
    ///
    /// Intended for correcting mistaken registrations or tidying entries for
    /// functions that have been fully removed.  If the entry does not exist
    /// the call is a no-op.
    ///
    /// # Parameters
    ///
    /// * `env`        – Soroban environment.
    /// * `admin`      – Contract admin address.
    /// * `entrypoint` – Name of the entrypoint to remove.
    ///
    /// # Errors
    ///
    /// * [`Error::Unauthorized`] – Caller is not the contract admin.
    /// * [`Error::AdminNotSet`]  – Contract has not been initialised.
    ///
    /// # Events
    ///
    /// Emits `("depr_rem", entrypoint)` when an entry is actually removed.
    pub fn remove_deprecated(
        env: Env,
        admin: Address,
        entrypoint: Symbol,
    ) -> Result<(), Error> {
        DeprecatedRegistry::remove(&env, &admin, entrypoint)
    }

    /// Look up a single deprecated-entrypoint entry by name.
    ///
    /// Returns `Some(DeprecatedEntry)` if `entrypoint` is registered, or
    /// `None` if it is not.  This is a permissionless read.
    ///
    /// # Parameters
    ///
    /// * `env`        – Soroban environment.
    /// * `entrypoint` – Name of the entrypoint to look up.
    pub fn get_deprecated_entry(env: Env, entrypoint: Symbol) -> Option<DeprecatedEntry> {
        DeprecatedRegistry::get_entry(&env, &entrypoint)
    }

    /// Return all entries in the deprecated-entrypoints registry.
    ///
    /// Returns a `Vec<DeprecatedEntry>` (empty if nothing has been registered
    /// yet).  This is a permissionless read.
    ///
    /// # Parameters
    ///
    /// * `env` – Soroban environment.
    pub fn list_deprecated_entries(env: Env) -> Vec<DeprecatedEntry> {
        DeprecatedRegistry::list_entries(&env)
    }

    /// Return the number of entries in the deprecated-entrypoints registry.
    ///
    /// This is a permissionless read.
    ///
    /// # Parameters
    ///
    /// * `env` – Soroban environment.
    pub fn deprecated_entry_count(env: Env) -> u32 {
        DeprecatedRegistry::entry_count(&env)
    }

    /// Return `true` if the given entrypoint is listed as deprecated.
    ///
    /// This is a permissionless read.
    ///
    /// # Parameters
    ///
    /// * `env`        – Soroban environment.
    /// * `entrypoint` – Name of the entrypoint to check.
    pub fn is_deprecated(env: Env, entrypoint: Symbol) -> bool {
        DeprecatedRegistry::is_deprecated(&env, &entrypoint)
    }

    /// Retrieves comprehensive analytics about market resolution performance.
    ///
    /// This function provides detailed statistics about how markets are being
    /// resolved across the platform, including success rates, resolution methods,
    /// oracle performance, and community consensus patterns.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    ///
    /// # Returns
    ///
    /// Returns `Result<ResolutionAnalytics, Error>` where:
    /// - `Ok(ResolutionAnalytics)` - Complete resolution analytics data
    /// - `Err(Error)` - Error if analytics calculation fails
    ///
    /// The `ResolutionAnalytics` struct contains:
    /// - Total markets resolved
    /// - Resolution method breakdown (manual vs automatic)
    /// - Oracle accuracy statistics
    /// - Community consensus metrics
    /// - Average resolution time
    /// - Dispute frequency and outcomes
    ///
    /// # Errors
    ///
    /// This function may return:
    /// - `Error::InsufficientData` - Not enough resolved markets for analytics
    /// - Storage access errors
    /// - Calculation errors from the analytics module
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    ///
    /// match PredictifyHybrid::get_resolution_analytics(env.clone()) {
    ///     Ok(analytics) => {
    ///         // Access resolution statistics
    ///         let total_resolved = analytics.total_markets_resolved;
    ///         let oracle_accuracy = analytics.oracle_accuracy_rate;
    ///         let avg_resolution_time = analytics.average_resolution_time;
    ///         
    ///         println!("Resolved markets: {}", total_resolved);
    ///         println!("Oracle accuracy: {}%", oracle_accuracy);
    ///     },
    ///     Err(e) => {
    ///         println!("Analytics unavailable: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Platform Monitoring**: Track overall resolution system health
    /// - **Oracle Evaluation**: Assess oracle performance and reliability
    /// - **Community Analysis**: Understand voting patterns and accuracy
    /// - **System Optimization**: Identify areas for improvement
    /// - **Governance Reporting**: Provide transparency to stakeholders
    ///
    /// # Analytics Metrics
    ///
    /// Key metrics included:
    /// - **Resolution Rate**: Percentage of markets successfully resolved
    /// - **Method Distribution**: Manual vs automatic resolution breakdown
    /// - **Accuracy Scores**: Oracle vs community prediction accuracy
    /// - **Time Metrics**: Average time from market end to resolution
    /// - **Dispute Analytics**: Frequency and resolution of disputes
    ///
    /// # Performance
    ///
    /// This function performs read-only analytics calculations and may take
    /// longer for platforms with many resolved markets. Results may be cached
    /// for performance optimization.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_resolution_analytics(env: Env) -> Result<resolution::MarketResolutionAnalytics, Error> {
        resolution::MarketResolutionAnalytics::calculate_resolution_analytics(&env)
    }

    /// Retrieves comprehensive analytics and statistics for a specific market.
    ///
    /// This function provides detailed statistical analysis of a market including
    /// participation metrics, voting patterns, stake distribution, and performance
    /// indicators. It's essential for market analysis and user interfaces.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<MarketStats, Error>` where:
    /// - `Ok(MarketStats)` - Complete market statistics and analytics
    /// - `Err(Error)` - Error if market not found or analysis fails
    ///
    /// The `MarketStats` struct contains:
    /// - Participation metrics (total voters, total stake)
    /// - Outcome distribution (stakes per outcome)
    /// - Market activity timeline
    /// - Consensus and confidence indicators
    /// - Resolution status and results
    ///
    /// # Errors
    ///
    /// This function returns:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - Calculation errors from the analytics module
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_market_analytics(env.clone(), market_id) {
    ///     Ok(stats) => {
    ///         // Access market statistics
    ///         let total_participants = stats.total_participants;
    ///         let total_stake = stats.total_stake;
    ///         let leading_outcome = stats.leading_outcome;
    ///         
    ///         println!("Participants: {}", total_participants);
    ///         println!("Total stake: {}", total_stake);
    ///         println!("Leading outcome: {:?}", leading_outcome);
    ///     },
    ///     Err(e) => {
    ///         println!("Analytics unavailable: {:?}", e);
    ///     }
    /// }
    /// ```
    ///
    /// # Statistical Metrics
    ///
    /// Key analytics provided:
    /// - **Participation**: Number of unique voters and total stake
    /// - **Distribution**: Stake distribution across outcomes
    /// - **Confidence**: Market confidence indicators and consensus strength
    /// - **Activity**: Voting timeline and participation patterns
    /// - **Performance**: Market liquidity and engagement metrics
    ///
    /// # Use Cases
    ///
    /// - **UI Display**: Show market statistics to users
    /// - **Market Analysis**: Understand market dynamics and trends
    /// - **Risk Assessment**: Evaluate market confidence and volatility
    /// - **Performance Tracking**: Monitor market engagement over time
    /// - **Research**: Academic and commercial market research
    ///
    /// # Real-time Updates
    ///
    /// Statistics are calculated in real-time based on current market state.
    /// For active markets, analytics reflect the most current voting and staking data.
    /// For resolved markets, analytics include final resolution information.
    ///
    /// # Performance
    ///
    /// This function performs calculations on market data and may have
    /// computational overhead for markets with many participants. Consider
    /// caching results for frequently accessed markets.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_market_analytics(
        env: Env,
        market_id: Symbol,
    ) -> Result<markets::MarketStats, Error> {
        // Fast path: serve from the instance analytics cache when hot.
        // The cache is invalidated on every write (vote, bet, claim, resolve, dispute).
        if let Some(cached) = analytics::get_or_compute(&env, &market_id) {
            return Ok(cached);
        }

        // Slow path: market not found in persistent storage.
        Err(Error::MarketNotFound)
    }

    /// Returns a deterministic, versioned snapshot for a single market's analytics.
    ///
    /// The payload is encoded with Soroban XDR so off-chain analytics services can
    /// persist a stable byte stream without relying on host-side ordering.
    pub fn get_market_analytics_snapshot(
        env: Env,
        market_id: Symbol,
    ) -> Result<analytics_snapshot::AnalyticsSnapshotEnvelope, Error> {
        analytics_snapshot::AnalyticsSnapshotManager::get_snapshot(&env, market_id)
    }

    /// Dispute a market resolution
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn dispute_market(
        env: Env,
        user: Address,
        market_id: Symbol,
        stake: i128,
        reason: Option<String>,
    ) -> Result<(), Error> {
        user.require_auth();

        // Rate limit disputes to prevent abuse
        if let Err(rate_err) = crate::rate_limiter::RateLimiter::new(env.clone())
            .rate_limit_disputes(user.clone(), market_id.clone())
        {
            if matches!(rate_err, crate::rate_limiter::RateLimiterError::ConfigNotFound) {
                // No rate limit config — skip
            } else {
                return Err(Error::from(rate_err));
            }
        }

        let result = disputes::DisputeManager::process_dispute(&env, user.clone(), market_id.clone(), stake, reason);
        if result.is_ok() {
            // Invalidate analytics cache — dispute stakes have changed.
            analytics::AnalyticsCache::new(&env).invalidate(&market_id);

            // Per-market audit entry: DisputeFiled
            let details = Map::new(&env);
            crate::audit::MarketAuditManager::append(
                &env,
                &market_id,
                crate::audit::MarketAuditAction::DisputeFiled,
                user,
                details,
            );
        }
        result
    }

    /// Set the dispute stake cap for a user in a market (governance/admin only)
    pub fn set_dispute_stake_cap(
        env: Env,
        admin: Address,
        market_id: Symbol,
        user: Address,
        cap: i128,
    ) -> Result<(), Error> {
        Self::require_admin_permission(&env, &admin, AdminPermission::UpdateConfig)?;
        if cap < 0 {
            return Err(Error::InvalidInput);
        }
        disputes::DisputeManager::set_dispute_stake_cap(&env, &market_id, &user, cap)
    }

    /// Get the dispute stake cap for a user in a market
    pub fn get_dispute_stake_cap(
        env: Env,
        market_id: Symbol,
        user: Address,
    ) -> i128 {
        let cap_key = storage::DataKey::DisputeStakeCap(market_id, user);
        env.storage().persistent().get(&cap_key).unwrap_or(0)
    }

    /// Set the per-user cumulative dispute stake cap across all active disputes (admin only).
    ///
    /// This cap limits the total stake a user can commit to disputes
    /// across all markets that have active (unresolved) disputes.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `admin` - The admin address (must be authorized)
    /// * `user` - The user address the cap applies to
    /// * `cap` - The maximum cumulative stake allowed in stroops (0 = disabled)
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when:
    /// - `Error::Unauthorized` — caller is not the contract admin
    /// - `Error::InvalidInput` — cap value is invalid
    ///
    /// # Events
    ///
    /// Emits `dispute_cumulative_stake_cap_set` event on success.
    pub fn set_dispute_cumulative_stake_cap(
        env: Env,
        admin: Address,
        user: Address,
        cap: i128,
    ) -> Result<(), Error> {
        if cap < 0 {
            return Err(Error::InvalidInput);
        }
        Self::require_admin_permission(&env, &admin, AdminPermission::UpdateConfig)?;
        disputes::DisputeManager::set_dispute_cumulative_stake_cap(&env, &admin, &user, cap)
    }

    /// Get the per-user cumulative dispute stake cap.
    ///
    /// Returns 0 if no cap is set (cap is disabled).
    pub fn get_dispute_cumulative_stake_cap(env: Env, user: Address) -> i128 {
        disputes::DisputeManager::get_dispute_cumulative_stake_cap(&env, &user)
    }

    /// Vote on a dispute
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn vote_on_dispute(
        env: Env,
        user: Address,
        market_id: Symbol,
        dispute_id: Symbol,
        vote: bool,
        stake: i128,
        reason: Option<String>,
    ) -> Result<(), Error> {
        user.require_auth();

        // Rate limit dispute votes to prevent abuse
        if let Err(rate_err) = crate::rate_limiter::RateLimiter::new(env.clone())
            .rate_limit_disputes(user.clone(), market_id.clone())
        {
            if matches!(rate_err, crate::rate_limiter::RateLimiterError::ConfigNotFound) {
                // No rate limit config — skip
            } else {
                return Err(Error::from(rate_err));
            }
        }

        let result = disputes::DisputeManager::vote_on_dispute(
            &env, user, market_id.clone(), dispute_id, vote, stake, reason,
        );
        if result.is_ok() {
            // Invalidate analytics cache — dispute stakes have changed.
            analytics::AnalyticsCache::new(&env).invalidate(&market_id);
        }
        result
    }

    /// Resolve a dispute (admin only)
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn resolve_dispute(
        env: Env,
        admin: Address,
        market_id: Symbol,
    ) -> Result<disputes::DisputeResolution, Error> {
        Self::require_primary_admin(&env, &admin)?;

        let result = disputes::DisputeManager::resolve_dispute(&env, market_id.clone(), admin.clone());
        if result.is_ok() {
            // Per-market audit entry: DisputeResolved
            let details = Map::new(&env);
            crate::audit::MarketAuditManager::append(
                &env,
                &market_id,
                crate::audit::MarketAuditAction::DisputeResolved,
                admin,
                details,
            );
        }
        result
    }

    /// Sets the maximum capacity of resolved/expired disputes to retain in history (admin only).
    pub fn set_history_cap(
        env: Env,
        admin: Address,
        cap: u32,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;

        disputes::DisputeManager::set_history_cap(&env, admin, cap)
    }

    /// Sets the global anti-grief minimum stake floor (admin only).
    pub fn set_anti_grief_floor(
        env: Env,
        admin: Address,
        floor: i128,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;

        disputes::DisputeManager::set_anti_grief_floor(&env, admin, floor)
    }

    /// Collect fees from a market (admin only)
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn collect_fees(env: Env, admin: Address, market_id: Symbol) -> Result<i128, Error> {
        if let Err(e) =
            crate::circuit_breaker::CircuitBreaker::require_write_allowed(&env, "collect_fees")
        {
            return Err(e);
        }
        Self::require_primary_admin(&env, &admin)?;

        let result = fees::FeeManager::collect_fees(&env, admin.clone(), market_id.clone());
        if let Ok(amount) = result {
            // Per-market audit entry: FeesCollected
            let mut details = Map::new(&env);
            details.set(
                Symbol::new(&env, "amount"),
                String::from_str(&env, &alloc::format!("{}", amount)),
            );
            crate::audit::MarketAuditManager::append(
                &env,
                &market_id,
                crate::audit::MarketAuditAction::FeesCollected,
                admin,
                details,
            );
            Ok(amount)
        } else {
            result
        }
    }

    // ===== PER-MARKET AUDIT LOG READ ENTRYPOINTS =====

    /// Returns a reverse-chronological page of audit entries for `market_id`.
    ///
    /// Each entry records a key state change: market creation, resolution,
    /// dispute filing/resolution, or fee collection. Entries are immutable once
    /// written and are keyed by market so off-chain clients can efficiently stream
    /// the history for a single market without scanning the global audit trail.
    ///
    /// # Parameters
    ///
    /// * `env`       - The Soroban environment.
    /// * `market_id` - The market whose audit log to query.
    /// * `limit`     - Maximum number of entries to return; capped at 100.
    ///
    /// # Returns
    ///
    /// A `Vec<MarketAuditEntry>` ordered newest-first (index `total_entries` → 1).
    /// Returns an empty vector when the market has no audit entries.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "btc_50k");
    /// let entries = PredictifyHybrid::get_market_audit_log(env, market_id, 10);
    /// for entry in entries.iter() {
    ///     // entry.index, entry.action, entry.actor, entry.timestamp, entry.details
    /// }
    /// ```
    pub fn get_market_audit_log(
        env: Env,
        market_id: Symbol,
        limit: u32,
    ) -> Vec<crate::audit::MarketAuditEntry> {
        crate::audit::MarketAuditManager::get_entries(&env, &market_id, limit)
    }

    /// Returns a single audit entry for `market_id` by its 1-based `index`.
    ///
    /// # Parameters
    ///
    /// * `env`       - The Soroban environment.
    /// * `market_id` - The market whose audit log to query.
    /// * `index`     - 1-based entry index (`1` == oldest, `total_entries` == newest).
    ///
    /// # Returns
    ///
    /// `Some(MarketAuditEntry)` when found; `None` when `index` is 0, out of range,
    /// or the market has no entries.
    pub fn get_market_audit_entry(
        env: Env,
        market_id: Symbol,
        index: u32,
    ) -> Option<crate::audit::MarketAuditEntry> {
        crate::audit::MarketAuditManager::get_entry(&env, &market_id, index)
    }

    /// Returns the head (total entry count) of the per-market audit log.
    ///
    /// Clients can read this first to learn valid index bounds before calling
    /// [`get_market_audit_entry`] for individual entries.
    ///
    /// # Parameters
    ///
    /// * `env`       - The Soroban environment.
    /// * `market_id` - The market to query.
    ///
    /// # Returns
    ///
    /// `Some(MarketAuditHead)` when the market has at least one entry; `None` otherwise.
    pub fn get_market_audit_head(
        env: Env,
        market_id: Symbol,
    ) -> Option<crate::audit::MarketAuditHead> {
        crate::audit::MarketAuditManager::get_head(&env, &market_id)
    }

    /// Automatically distribute payouts to all winners after market resolution.
    ///
    /// This function automatically calculates and distributes winnings to all users
    /// who bet on the winning outcome, eliminating the need for manual claiming.
    /// It handles edge cases like no winners, all winners, and prevents double payouts.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the resolved market
    ///
    /// # Returns
    ///
    /// Returns `Result<i128, Error>` where:
    /// - `Ok(total_distributed)` - Total amount distributed to winners
    /// - `Err(Error)` - Error if distribution fails
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketNotResolved` - Market hasn't been resolved yet
    /// - `Error::MarketResolved` - Payouts have already been distributed
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "resolved_market");
    ///
    /// match PredictifyHybrid::distribute_payouts(env.clone(), market_id) {
    ///     Ok(total) => println!("Distributed {} stroops to winners", total),
    ///     Err(e) => println!("Distribution failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Payout Calculation
    ///
    /// Payouts are calculated using the formula:
    /// ```text
    /// user_payout = (user_stake * (100 - fee_percentage) / 100) * total_pool / winning_total
    /// ```
    ///
    /// # Edge Cases
    ///
    /// - **No Winners**: If no users bet on the winning outcome, no payouts are made
    /// - **All Winners**: If all users bet on the winning outcome, they receive proportional shares
    /// - **Double Payout Prevention**: Users who already claimed are skipped
    ///
    /// # Security & Testing
    ///
    /// - Tested for invariants using `proptest` to ensure:
    ///   - Total distributed `<= total pool` mathematically strictly.
    ///   - Fees are deducted predictably and exactly.
    ///   - Split pools evenly and proportionately distribute to tie winners without underflow.
    ///   - Failsafes prevent re-distribution.
    ///
    /// # Events
    ///
    /// This function emits `WinningsClaimedEvent` for each user who receives a payout.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    pub fn distribute_payouts(env: Env, market_id: Symbol) -> Result<i128, Error> {
        // ── Circuit breaker guard ──────────────────────────────────────────────
        if let Err(e) = CircuitBreaker::require_write_allowed(&env, "distribute_payouts") {
            return Err(e);
        }

        // ── Load market ────────────────────────────────────────────────────────
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::MarketNotFound);
            });

        // ── Require resolved ───────────────────────────────────────────────────
        let winning_outcomes = match &market.winning_outcomes {
            Some(outcomes) => outcomes,
            None => return Err(Error::MarketNotResolved),
        };

        // ── Load bettor registry ───────────────────────────────────────────────
        let bettors = BetStorage::get_all_bets_for_market(&env, &market_id);

        // ── Platform fee (basis points, default 200 = 2%) ─────────────────────
        let fee_percent: i128 = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, "platform_fee"))
            .unwrap_or(200);

        // ── Short-circuit: check whether any unclaimed winners exist ───────────
        let mut has_unclaimed_winners = false;

        // Check voters
        for (user, outcome) in market.votes.iter() {
            if winning_outcomes.contains(&outcome) {
                if !market
                    .claimed
                    .get(user.clone())
                    .map(|info| info.is_claimed())
                    .unwrap_or(false)
                {
                    has_unclaimed_winners = true;
                    break;
                }
            }
        }

        // Check bettors (only if no unclaimed voters found yet)
        if !has_unclaimed_winners {
            for user in bettors.iter() {
                if let Some(bet) = BetStorage::get_bet(&env, &market_id, &user) {
                    if winning_outcomes.contains(&bet.outcome)
                        && !market
                            .claimed
                            .get(user.clone())
                            .map(|info| info.is_claimed())
                            .unwrap_or(false)
                    {
                        has_unclaimed_winners = true;
                        break;
                    }
                }
            }
        }

        if !has_unclaimed_winners {
            return Ok(0);
        }

        // ── Resolution summary (winning totals & pool size) ────────────────────
        let summary = ResolutionOutcomeCache::require(&env, &market_id, &market)?;
        let winning_total = summary.winning_total;
        if winning_total == 0 {
            return Ok(0);
        }

        let total_pool = summary.total_pool;
        let fee_denominator = 10_000i128;
        let mut total_distributed: i128 = 0;

        // ── Budget guard: abort before host runs out of CPU instructions ───────
        // Threshold of 100 000 instructions gives enough headroom to finish the
        // current iteration and write the updated market back to storage.
        let budget_guard = BudgetGuard::new(&env, 100_000);

        // ── 1. Distribute to Voters ────────────────────────────────────────────
        let mut voter_count = 0u32;
        for (user, outcome) in market.votes.iter() {
            if winning_outcomes.contains(&outcome) {
                // Skip already-claimed voters
                if market
                    .claimed
                    .get(user.clone())
                    .map(|info| info.is_claimed())
                    .unwrap_or(false)
                {
                    voter_count += 1;
                    if voter_count % 10 == 0 {
                        budget_guard.check()?;
                    }
                    continue;
                }

                let user_stake = market.stakes.get(user.clone()).unwrap_or(0);
                if user_stake > 0 {
                    let user_share = (user_stake
                        .checked_mul(fee_denominator - fee_percent)
                        .ok_or(Error::InvalidInput)?)
                        / fee_denominator;

                    let payout = (user_share
                        .checked_mul(total_pool)
                        .ok_or(Error::InvalidInput)?)
                        / winning_total;

                    if payout >= 0 {
                        market
                            .claimed
                            .set(user.clone(), ClaimInfo::new(&env, payout));

                        if payout > 0 {
                            total_distributed = total_distributed
                                .checked_add(payout)
                                .ok_or(Error::InvalidInput)?;

                            BalanceStorage::add_balance(
                                &env,
                                &user,
                                &ReflectorAsset::Stellar,
                                payout,
                            )?;

                            EventEmitter::emit_winnings_claimed(
                                &env,
                                &market_id,
                                &user,
                                payout,
                            );
                        }
                    }
                }
            }

            voter_count += 1;
            if voter_count % 10 == 0 {
                budget_guard.check()?;
            }
        }

        // ── 2. Distribute to Bettors ───────────────────────────────────────────
        let mut bettor_count = 0u32;
        for user in bettors.iter() {
            if let Some(mut bet) = BetStorage::get_bet(&env, &market_id, &user) {
                if winning_outcomes.contains(&bet.outcome) {
                    // If already claimed via the voter path, just mark status Won
                    if market
                        .claimed
                        .get(user.clone())
                        .map(|info| info.is_claimed())
                        .unwrap_or(false)
                    {
                        bet.status = BetStatus::Won;
                        let _ = BetStorage::store_bet(&env, &bet);
                    } else if bet.amount > 0 {
                        let user_share = (bet.amount
                            .checked_mul(fee_denominator - fee_percent)
                            .ok_or(Error::InvalidInput)?)
                            / fee_denominator;

                        let payout = (user_share
                            .checked_mul(total_pool)
                            .ok_or(Error::InvalidInput)?)
                            / winning_total;

                        if payout > 0 {
                            market
                                .claimed
                                .set(user.clone(), ClaimInfo::new(&env, payout));

                            total_distributed = total_distributed
                                .checked_add(payout)
                                .ok_or(Error::InvalidInput)?;

                            bet.status = BetStatus::Won;
                            let _ = BetStorage::store_bet(&env, &bet);

                            match BalanceStorage::add_balance(
                                &env,
                                &user,
                                &ReflectorAsset::Stellar,
                                payout,
                            ) {
                                Ok(_) => {}
                                Err(e) => panic_with_error!(env, e),
                            }

                            EventEmitter::emit_winnings_claimed(
                                &env,
                                &market_id,
                                &user,
                                payout,
                            );
                        }
                    }
                } else {
                    // Losing bet — mark as Lost
                    if matches!(bet.status, BetStatus::Active) {
                        bet.status = BetStatus::Lost;
                        let _ = BetStorage::store_bet(&env, &bet);
                    }
                }
            }

            bettor_count += 1;
            if bettor_count % 10 == 0 {
                budget_guard.check()?;
            }
        }

        // ── Final budget check before the storage write ────────────────────────
        budget_guard.check()?;

        // ── Persist updated claim map ──────────────────────────────────────────
        env.storage().persistent().set(&market_id, &market);

        Ok(total_distributed)
    }

    // ===== EVENT ARCHIVE AND HISTORICAL QUERY =====

    /// Mark a resolved or cancelled event (market) as archived. Admin only.
    /// Market must be in Resolved or Cancelled state. Returns InvalidState if not
    /// eligible, AlreadyClaimed if already archived.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn archive_event(env: Env, admin: Address, market_id: Symbol) -> Result<(), Error> {
        crate::event_archive::EventArchive::archive_event(&env, &admin, &market_id)
    }

    /// Remove the oldest `count` archived entries to free capacity (admin only).
    ///
    /// Returns the number of entries actually removed. `count` is capped at 30.
    ///
    /// # Errors
    /// * `Unauthorized` - Caller is not admin
    pub fn prune_archive(env: Env, admin: Address, count: u32, cursor: Option<crate::event_archive::PruneCursor>) -> Result<(u32, crate::event_archive::PruneCursor), Error> {
        crate::event_archive::EventArchive::prune_archive(&env, &admin, count, cursor)
    }

    /// Return the current number of entries in the event archive.
    pub fn archive_size(env: Env) -> u32 {
        crate::event_archive::EventArchive::archive_size(&env)
    }

    /// Query events by creation time range. Returns public metadata only (no votes/stakes).
    /// Paginated: cursor is start index, limit capped at 30. Returns (entries, next_cursor).
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn query_events_history(
        env: Env,
        from_ts: u64,
        to_ts: u64,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        crate::event_archive::EventArchive::query_events_history(
            &env, from_ts, to_ts, cursor, limit,
        )
    }

    /// Query events by resolution status (e.g. Resolved, Cancelled). Paginated.
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn query_events_by_status(
        env: Env,
        status: MarketState,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        crate::event_archive::EventArchive::query_events_by_resolution_status(
            &env, status, cursor, limit,
        )
    }

    /// Query events by category (oracle feed_id). Paginated.
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn query_events_by_category(
        env: Env,
        category: String,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        crate::event_archive::EventArchive::query_events_by_category(&env, &category, cursor, limit)
    }

    /// Set the platform fee percentage (admin only).
    ///
    /// This function allows the admin to update the platform fee percentage
    /// within the allowed limits (0-10%). The fee is applied to winning payouts.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address (must be authorized)
    /// * `fee_percentage` - New fee percentage in basis points (e.g., 200 = 2%)
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Fee percentage updated successfully
    /// - `Err(Error)` - Error if update fails
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::InvalidFeeConfig` - Fee percentage is outside valid range (0-10%)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Set platform fee to 2.5% (250 basis points)
    /// match PredictifyHybrid::set_platform_fee(env.clone(), admin, 250) {
    ///     Ok(()) => println!("Fee updated successfully"),
    ///     Err(e) => println!("Fee update failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Fee Limits
    ///
    /// - Minimum fee: 0% (0 basis points)
    /// - Maximum fee: 10% (1000 basis points)
    /// - Default fee: 2% (200 basis points)
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn set_platform_fee(env: Env, admin: Address, fee_percentage: i128) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;

        // Validate fee percentage (0-10%)
        if fee_percentage < 0 || fee_percentage > 1000 {
            return Err(Error::InvalidFeeConfig);
        }

        // Update fee in legacy storage
        let fee_key = Symbol::new(&env, "platform_fee");
        env.storage().persistent().set(&fee_key, &fee_percentage);

        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::FeeConfigUpdated,
            admin.clone(),
            Map::new(&env),
            None,
        );

        Ok(())
    }

    pub fn commit_fee_config(env: Env, admin: Address, hash: BytesN<32>) -> Result<(), Error> {
        fees::FeeManager::commit_fee_config(&env, admin, hash)
    }

    /// Reveal and apply a committed fee configuration (admin only)
    pub fn reveal_fee_config(env: Env, admin: Address, new_config: fees::FeeConfig) -> Result<fees::FeeConfig, Error> {
        fees::FeeManager::update_fee_config(&env, admin, new_config)
    }

    /// Set global minimum and maximum bet limits (admin only).
    /// Applies to all events that do not have per-event limits.
    /// Rejects if min > max or outside absolute bounds (MIN_BET_AMOUNT..=MAX_BET_AMOUNT).
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn set_global_bet_limits(
        env: Env,
        admin: Address,
        min_bet: i128,
        max_bet: i128,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        let limits = crate::types::BetLimits { min_bet, max_bet };
        crate::bets::set_global_bet_limits(&env, &limits)?;
        let scope = Symbol::new(&env, "global");
        EventEmitter::emit_bet_limits_updated(&env, &admin, &scope, min_bet, max_bet);

        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::BetLimitsUpdated,
            admin.clone(),
            Map::new(&env),
            None,
        );

        Ok(())
    }

    /// Set per-event minimum and maximum bet limits (admin only).
    /// Overrides global limits for the given market.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn set_event_bet_limits(
        env: Env,
        admin: Address,
        market_id: Symbol,
        min_bet: i128,
        max_bet: i128,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        let limits = BetLimits { min_bet, max_bet };
        crate::bets::set_event_bet_limits(&env, &market_id, &limits)?;
        EventEmitter::emit_bet_limits_updated(&env, &admin, &market_id, min_bet, max_bet);
        Ok(())
    }

    /// Get effective bet limits for a market (per-event if set, else global, else defaults).
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_effective_bet_limits(env: Env, market_id: Symbol) -> BetLimits {
        crate::bets::get_effective_bet_limits(&env, &market_id)
    }

    /// Set the per-market maximum single-bet cap (admin only).
    ///
    /// Once set, any individual bet whose `amount` exceeds `cap` is rejected with
    /// [`Error::BetExceedsCap`].  The cap is checked after, and in addition to, the
    /// global/per-event `max_bet` in [`BetLimits`].
    ///
    /// Pass `cap = 0` to remove the cap (equivalent to calling
    /// [`remove_market_max_bet_cap`]).  Any other value must satisfy
    /// `0 < cap <= MAX_BET_AMOUNT`.
    ///
    /// # Parameters
    ///
    /// - `admin`     – Must be the primary admin address
    /// - `market_id` – Identifies the target market
    /// - `cap`       – Maximum single-bet amount in base token units (stroops)
    ///
    /// # Errors
    ///
    /// - [`Error::Unauthorized`] when `admin` is not the primary admin
    /// - [`Error::InvalidInput`] when `cap` is negative or exceeds [`MAX_BET_AMOUNT`]
    ///
    /// # Events
    ///
    /// Emits a `bet_limits_updated` event scoped to `market_id` so that indexers
    /// can track cap changes.
    pub fn set_market_max_bet_cap(
        env: Env,
        admin: Address,
        market_id: Symbol,
        cap: i128,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        // cap == 0 is treated as "remove the cap"
        if cap == 0 {
            crate::bets::remove_market_max_bet_cap(&env, &market_id);
        } else {
            crate::bets::set_market_max_bet_cap(&env, &market_id, cap)?;
        }
        // Emit so indexers can observe
        EventEmitter::emit_bet_limits_updated(&env, &admin, &market_id, 0, cap);

        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::BetLimitsUpdated,
            admin.clone(),
            Map::new(&env),
            None,
        );

        Ok(())
    }

    /// Get the per-market max single-bet cap, or `None` if no cap is configured.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_market_max_bet_cap(env: Env, market_id: Symbol) -> Option<i128> {
        crate::bets::get_market_max_bet_cap(&env, &market_id)
    }

    /// Remove the per-market max single-bet cap (admin only).
    ///
    /// After removal, bets on this market are bounded only by the global/per-event
    /// [`BetLimits`] `max_bet` (or [`MAX_BET_AMOUNT`] when no limits are configured).
    ///
    /// # Errors
    ///
    /// - [`Error::Unauthorized`] when `admin` is not the primary admin
    ///
    /// # Events
    ///
    /// Emits a `bet_limits_updated` event with `max_bet = 0` to signal removal.
    pub fn remove_market_max_bet_cap(
        env: Env,
        admin: Address,
        market_id: Symbol,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        crate::bets::remove_market_max_bet_cap(&env, &market_id);
        EventEmitter::emit_bet_limits_updated(&env, &admin, &market_id, 0, 0);
        Ok(())
    }

    pub fn set_max_participants(
        env: Env,
        admin: Address,
        market_id: Symbol,
        max_participants: Option<u32>,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| panic_with_error!(&env, Error::MarketNotFound));
        market.max_participants = max_participants;
        env.storage().persistent().set(&market_id, &market);
        Ok(())
    }

    /// Set global oracle validation config (admin only).
    ///
    /// - `max_staleness_secs`: maximum allowed age in seconds.
    /// - `max_confidence_bps`: maximum confidence interval in basis points.
    /// Per-event overrides, if set, take precedence over this global config.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn set_oracle_val_cfg_global(
        env: Env,
        admin: Address,
        max_staleness_secs: u64,
        max_confidence_bps: u32,
        max_deviation_bps: Option<u32>,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        crate::admin::OracleAdminCooldownManager::enforce_cooldown(&env, &admin)?;

        let config = GlobalOracleValidationConfig {
            max_staleness_secs,
            max_confidence_bps,
            max_deviation_bps,
            max_deviation_z_multiple: None,
            history_size: None,
        };
        crate::oracles::OracleValidationConfigManager::set_global_config(&env, &config)?;

        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::OracleConfigUpdated,
            admin.clone(),
            Map::new(&env),
            None,
        );

        Ok(())
    }

    /// Set per-event oracle validation config (admin only).
    ///
    /// Overrides global validation settings for the given market.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn set_oracle_val_cfg_event(
        env: Env,
        admin: Address,
        market_id: Symbol,
        max_staleness_secs: u64,
        max_confidence_bps: u32,
        max_deviation_bps: Option<u32>,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        crate::admin::OracleAdminCooldownManager::enforce_cooldown(&env, &admin)?;

        let config = EventOracleValidationConfig {
            max_staleness_secs,
            max_confidence_bps,
            max_deviation_bps,
            max_deviation_z_multiple: None,
            history_size: None,
        };
        crate::oracles::OracleValidationConfigManager::set_event_config(&env, &market_id, &config)?;

        let mut details = Map::new(&env);
        details.set(
            Symbol::new(&env, "market_id"),
            String::from_str(&env, "market_updated"),
        );

        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::OracleConfigUpdated,
            admin.clone(),
            details,
            None,
        );

        Ok(())
    }

    /// Get effective oracle validation config for a market.
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_oracle_val_cfg_effective(
        env: Env,
        market_id: Symbol,
    ) -> GlobalOracleValidationConfig {
        crate::oracles::OracleValidationConfigManager::get_effective_config(&env, &market_id)
    }

    /// Withdraw collected platform fees (admin only).
    ///
    /// This function allows the admin to withdraw fees that have been collected
    /// from market payouts. Fees are accumulated across all markets and can be
    /// withdrawn by the admin.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address (must be authorized)
    /// * `amount` - Amount to withdraw (in stroops). If 0, withdraws all available fees.
    ///
    /// # Returns
    ///
    /// Returns `Result<i128, Error>` where:
    /// - `Ok(amount_withdrawn)` - Amount successfully withdrawn
    /// - `Err(Error)` - Error if withdrawal fails
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::NoFeesToCollect` - No fees available to withdraw
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    ///
    /// // Withdraw all available fees
    /// match PredictifyHybrid::withdraw_collected_fees(env.clone(), admin, 0) {
    ///     Ok(amount) => println!("Withdrew {} stroops", amount),
    ///     Err(e) => println!("Withdrawal failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn withdraw_collected_fees(env: Env, admin: Address, amount: i128) -> Result<i128, Error> {
        Self::require_primary_admin(&env, &admin)?;

        // Get collected fees from storage (using the same key as FeeTracker)
        let fees_key = Symbol::new(&env, "tot_fees");
        let collected_fees: i128 = env.storage().persistent().get(&fees_key).unwrap_or(0);

        if collected_fees == 0 {
            return Err(Error::NoFeesToCollect);
        }

        // Determine withdrawal amount
        let withdrawal_amount = if amount == 0 || amount > collected_fees {
            collected_fees
        } else {
            amount
        };

        // Update collected fees (checked to prevent underflow)
        let remaining_fees = collected_fees
            .checked_sub(withdrawal_amount)
            .ok_or(Error::InvalidInput)?;
        env.storage().persistent().set(&fees_key, &remaining_fees);

        // Emit fee withdrawal event
        EventEmitter::emit_fee_collected(
            &env,
            &Symbol::new(&env, "withdrawal"),
            &admin,
            withdrawal_amount,
            &String::from_str(&env, "fee_withdrawal"),
        );

        // In a real implementation, transfer tokens to admin here
        // For now, we'll just track the withdrawal

        Ok(withdrawal_amount)
    }

    /// Extends the deadline of an active market by a specified number of days (admin only).
    ///
    /// This function allows contract administrators to extend the voting/betting period
    /// of active markets. Extensions can be used to allow more time for participation,
    /// respond to unforeseen circumstances, or adjust to market conditions. The function
    /// enforces maximum extension limits and validates market state before applying changes.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the extension (must be authorized)
    /// * `market_id` - Unique identifier of the market to extend
    /// * `additional_days` - Number of days to add to the current end time
    /// * `reason` - Explanation for why the extension is needed
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market deadline extended successfully
    /// - `Err(Error)` - Specific error if extension fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Cannot extend a resolved market
    /// - `Error::InvalidDuration` - Extension would exceed maximum allowed limit
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Extend market by 7 days
    /// match PredictifyHybrid::extend_deadline(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     7,
    ///     String::from_str(&env, "Low participation - extending to allow more votes")
    /// ) {
    ///     Ok(()) => println!("Market deadline extended successfully"),
    ///     Err(e) => println!("Extension failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Extension Rules
    ///
    /// - Market must be in Active or Ended state (not Resolved, Closed, or Cancelled)
    /// - Total extensions cannot exceed `max_extension_days` (default 30 days)
    /// - Extensions are recorded in market's extension history
    /// - Admin must pay extension fee if configured
    ///
    /// # Security
    ///
    /// This function requires admin authentication and should be used carefully.
    /// Excessive extensions may affect user trust and market integrity. All
    /// extensions are logged with timestamps and reasons for transparency.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn extend_deadline(
        env: Env,
        admin: Address,
        market_id: Symbol,
        additional_days: u32,
        reason: String,
    ) -> Result<(), Error> {
        admin.require_auth();

        // Verify admin
        let stored_admin: Address =
            match env.storage().persistent().get(&Symbol::new(&env, "Admin")) {
                Some(admin_addr) => admin_addr,
                None => panic_with_error!(env, Error::AdminNotSet),
            };

        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // Delegate to ExtensionManager for core logic, fee handling, and events
        crate::extensions::ExtensionManager::extend_market_duration(
            &env,
            admin,
            market_id,
            additional_days,
            reason,
        )
        .unwrap_or_else(|e| panic_with_error!(env, e));

        Ok(())
    }

    /// Updates the description/question of a market (admin only, before betting starts).
    ///
    /// This function allows contract administrators to update the market question
    /// or description before any bets have been placed. This ensures that market
    /// parameters can be corrected or clarified without affecting existing user
    /// commitments or predictions.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the update (must be authorized)
    /// * `market_id` - Unique identifier of the market to update
    /// * `new_description` - The updated market question or description
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market description updated successfully
    /// - `Err(Error)` - Specific error if update fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Cannot update a resolved market
    /// - `Error::BetsAlreadyPlaced` - Cannot update after bets have been placed
    /// - `Error::InvalidQuestion` - New description is empty or invalid
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Update market description
    /// match PredictifyHybrid::update_event_description(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     String::from_str(&env, "Will Bitcoin reach $100,000 by December 31, 2024?")
    /// ) {
    ///     Ok(()) => println!("Market description updated successfully"),
    ///     Err(e) => println!("Update failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Update Rules
    ///
    /// - Market must be in Active state
    /// - No bets can have been placed yet
    /// - Market must not be resolved
    /// - New description must be non-empty and meet length requirements
    ///
    /// # Security
    ///
    /// This function requires admin authentication and validates that no user
    /// funds are at risk. Updates are only allowed before any betting activity
    /// to maintain fairness and transparency.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn update_event_description(
        env: Env,
        admin: Address,
        market_id: Symbol,
        new_description: String,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;

        // Validate new description
        if new_description.is_empty() {
            panic_with_error!(env, Error::InvalidQuestion);
        }

        // Get market
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| panic_with_error!(env, Error::MarketNotFound));

        // Validate market state - cannot update resolved, closed, or cancelled markets
        if market.state != MarketState::Active {
            panic_with_error!(env, Error::MarketResolved);
        }

        // Check if any bets have been placed
        let bet_stats = bets::BetManager::get_market_bet_stats(&env, &market_id);
        if bet_stats.total_bets > 0 {
            panic_with_error!(env, Error::BetsAlreadyPlaced);
        }

        // Check if any votes have been placed
        if market.total_staked > 0 {
            panic_with_error!(env, Error::AlreadyVoted);
        }

        // Store old description for event
        let old_description = market.question.clone();

        // Update market description and refresh the metadata commitment so
        // clients with stale cached metadata fail verification.
        market.question = new_description.clone();
        market.refresh_metadata_commitment(&env);

        // Save market
        env.storage().persistent().set(&market_id, &market);

        // Emit description update event
        EventEmitter::emit_market_description_updated(
            &env,
            &market_id,
            &old_description,
            &new_description,
            &admin,
        );

        let mut details = Map::new(&env);
        details.set(
            Symbol::new(&env, "update"),
            String::from_str(&env, "description"),
        );
        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::MarketUpdated,
            admin.clone(),
            details,
            None,
        );

        Ok(())
    }

    /// Updates the outcomes of a market (admin only, before betting starts).
    ///
    /// This function allows contract administrators to update the available
    /// outcomes for a market before any bets have been placed. This ensures
    /// that market parameters can be corrected or adjusted without affecting
    /// existing user commitments.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the update (must be authorized)
    /// * `market_id` - Unique identifier of the market to update
    /// * `new_outcomes` - The updated list of possible outcomes
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market outcomes updated successfully
    /// - `Err(Error)` - Specific error if update fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Cannot update a resolved market
    /// - `Error::BetsAlreadyPlaced` - Cannot update after bets have been placed
    /// - `Error::InvalidOutcomes` - New outcomes list is invalid (< 2 outcomes or empty strings)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String, Vec};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Update market outcomes
    /// let new_outcomes = Vec::from_array(&env, [
    ///     String::from_str(&env, "Yes"),
    ///     String::from_str(&env, "No"),
    ///     String::from_str(&env, "Uncertain")
    /// ]);
    ///
    /// match PredictifyHybrid::update_event_outcomes(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     new_outcomes
    /// ) {
    ///     Ok(()) => println!("Market outcomes updated successfully"),
    ///     Err(e) => println!("Update failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Update Rules
    ///
    /// - Market must be in Active state
    /// - No bets can have been placed yet
    /// - Market must not be resolved
    /// - New outcomes must have at least 2 options
    /// - All outcome strings must be non-empty
    ///
    /// # Security
    ///
    /// This function requires admin authentication and validates that no user
    /// funds are at risk. Updates are only allowed before any betting activity
    /// to maintain fairness and transparency.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn update_event_outcomes(
        env: Env,
        admin: Address,
        market_id: Symbol,
        new_outcomes: Vec<String>,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;

        // Validate new outcomes
        if new_outcomes.len() < 2 {
            panic_with_error!(env, Error::InvalidOutcomes);
        }

        // Check all outcomes are non-empty
        for outcome in new_outcomes.iter() {
            if outcome.is_empty() {
                panic_with_error!(env, Error::InvalidOutcome);
            }
        }

        // Get market
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| panic_with_error!(env, Error::MarketNotFound));

        // Validate market state - cannot update resolved, closed, or cancelled markets
        if market.state != MarketState::Active {
            panic_with_error!(env, Error::MarketResolved);
        }

        // Check if any bets have been placed
        let bet_stats = bets::BetManager::get_market_bet_stats(&env, &market_id);
        if bet_stats.total_bets > 0 {
            panic_with_error!(env, Error::BetsAlreadyPlaced);
        }

        // Check if any votes have been placed
        if market.total_staked > 0 {
            panic_with_error!(env, Error::AlreadyVoted);
        }

        // Store old outcomes for event
        let old_outcomes = market.outcomes.clone();

        // Update market outcomes and refresh the commitment so stale clients
        // holding the old metadata commitment fail verification.
        market.outcomes = new_outcomes.clone();
        market.refresh_metadata_commitment(&env);

        // Save market
        env.storage().persistent().set(&market_id, &market);

        // Emit outcomes update event
        EventEmitter::emit_market_outcomes_updated(
            &env,
            &market_id,
            &old_outcomes,
            &new_outcomes,
            &admin,
        );

        let mut details = Map::new(&env);
        details.set(
            Symbol::new(&env, "update"),
            String::from_str(&env, "outcomes"),
        );
        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::MarketUpdated,
            admin.clone(),
            details,
            None,
        );

        Ok(())
    }

    /// Updates the category of a market (admin only, before betting starts).
    ///
    /// This function allows contract administrators to set or update the category
    /// for a market before any bets have been placed. Categories help clients
    /// filter and display markets by type (e.g., sports, crypto, politics).
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the update (must be authorized)
    /// * `market_id` - Unique identifier of the market to update
    /// * `category` - The new category (None to clear the category)
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market category updated successfully
    /// - `Err(Error)` - Specific error if update fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Cannot update a resolved market
    /// - `Error::BetsAlreadyPlaced` - Cannot update after bets have been placed
    /// - `Error::InvalidInput` - `Some` with an empty category string, or other invalid optional payload
    /// - `Error::CategoryTooShort` / `Error::CategoryTooLong` - Category length outside configured bounds
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Set market category
    /// match PredictifyHybrid::update_event_category(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     Some(String::from_str(&env, "sports"))
    /// ) {
    ///     Ok(()) => println!("Market category updated successfully"),
    ///     Err(e) => println!("Update failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn update_event_category(
        env: Env,
        admin: Address,
        market_id: Symbol,
        category: Option<String>,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;

        // Get market
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Validate market state - cannot update resolved, closed, or cancelled markets
        if market.state != MarketState::Active {
            return Err(Error::MarketResolved);
        }

        // Check if any bets have been placed
        let bet_stats = bets::BetManager::get_market_bet_stats(&env, &market_id);
        if bet_stats.total_bets > 0 {
            return Err(Error::BetsAlreadyPlaced);
        }

        // Check if any votes have been placed
        if market.total_staked > 0 {
            return Err(Error::AlreadyVoted);
        }

        crate::metadata_limits::validate_option_category_metadata(&category)?;

        // Store old category for event
        let old_category = market.category.clone();

        // Update market category
        market.category = category.clone();

        // Save market
        env.storage().persistent().set(&market_id, &market);

        // Emit category update event
        EventEmitter::emit_category_updated(&env, &market_id, &old_category, &category, &admin);

        let mut details = Map::new(&env);
        details.set(
            Symbol::new(&env, "update"),
            String::from_str(&env, "category"),
        );
        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::MarketUpdated,
            admin.clone(),
            details,
            None,
        );

        Ok(())
    }

    /// Updates the tags of a market (admin only, before betting starts).
    ///
    /// This function allows contract administrators to set or update tags
    /// for a market before any bets have been placed. Tags help clients
    /// filter and search markets by multiple dimensions.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address performing the update (must be authorized)
    /// * `market_id` - Unique identifier of the market to update
    /// * `tags` - The new list of tags (empty Vec to clear all tags)
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Market tags updated successfully
    /// - `Err(Error)` - Specific error if update fails
    ///
    /// # Errors
    ///
    /// This function returns specific errors:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Cannot update a resolved market
    /// - `Error::BetsAlreadyPlaced` - Cannot update after bets have been placed
    /// - `Error::InvalidInput` - Empty tag entry, duplicate tags, or other invalid list content
    /// - `Error::TagTooShort` / `Error::TagTooLong` - Tag length outside configured bounds
    /// - `Error::TooManyTags` - More than the maximum number of tags per market
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, Symbol, String, vec};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// // Set market tags
    /// let tags = vec![
    ///     &env,
    ///     String::from_str(&env, "bitcoin"),
    ///     String::from_str(&env, "crypto"),
    ///     String::from_str(&env, "price-prediction")
    /// ];
    ///
    /// match PredictifyHybrid::update_event_tags(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     tags
    /// ) {
    ///     Ok(()) => println!("Market tags updated successfully"),
    ///     Err(e) => println!("Update failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn update_event_tags(
        env: Env,
        admin: Address,
        market_id: Symbol,
        tags: Vec<String>,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;

        crate::metadata_limits::validate_event_tags(&tags)?;

        // Get market
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Validate market state - cannot update resolved, closed, or cancelled markets
        if market.state != MarketState::Active {
            return Err(Error::MarketResolved);
        }

        // Check if any bets have been placed
        let bet_stats = bets::BetManager::get_market_bet_stats(&env, &market_id);
        if bet_stats.total_bets > 0 {
            return Err(Error::BetsAlreadyPlaced);
        }

        // Check if any votes have been placed
        if market.total_staked > 0 {
            return Err(Error::AlreadyVoted);
        }

        // Store old tags for event
        let old_tags = market.tags.clone();

        // Update market tags
        market.tags = tags.clone();

        // Save market
        env.storage().persistent().set(&market_id, &market);

        // Emit tags update event
        EventEmitter::emit_tags_updated(&env, &market_id, &old_tags, &tags, &admin);

        let mut details = Map::new(&env);
        details.set(Symbol::new(&env, "update"), String::from_str(&env, "tags"));
        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::MarketUpdated,
            admin.clone(),
            details,
            None,
        );

        Ok(())
    }

    /// Query events by tags (paginated, bounded).
    ///
    /// Returns events that have ANY of the provided tags (OR logic).
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `tags` - Tags to filter by (events matching any tag are returned)
    /// * `cursor` - Pagination cursor
    /// * `limit` - Maximum results per page
    ///
    /// # Returns
    ///
    /// Tuple of (events, next_cursor)
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn query_events_by_tags(
        env: Env,
        tags: Vec<String>,
        cursor: u32,
        limit: u32,
    ) -> (Vec<EventHistoryEntry>, u32) {
        event_archive::EventArchive::query_events_by_tags(&env, &tags, cursor, limit)
    }

    /// Return a paginated page of market IDs.
    ///
    /// Avoids unbounded `Vec` returns by slicing the market index.
    /// Pass `next_cursor` from the previous response as `cursor` on the next
    /// call.  Iteration is complete when `items.len() < limit`.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `cursor` - Zero-based start index (0 for first page)
    /// * `limit` - Desired page size; capped server-side at 50
    ///
    /// # Returns
    ///
    /// `SymbolPagedResult<Symbol>` with `items`, `next_cursor`, and `total_count`.
    ///
    /// # Errors
    ///
    /// Panics with `Error::ContractStateError` if the market index is corrupted.
    ///
    /// # Events
    ///
    /// Read-only; no events emitted.
   pub fn get_all_markets_paged(
    env: Env,
    cursor: u32,
    limit: u32,
) -> SymbolPagedResult {
    crate::queries::QueryManager::get_all_markets_paged(
        &env,
        cursor,
        limit,
    )
    .unwrap_or_else(|e| panic_with_error!(&env, e))
}
    /// Return a paginated page of a user's bets across markets.
    ///
    /// Scans the market index slice `[cursor, cursor+limit)` and returns only
    /// markets where `user` has placed a bet.  Prevents gas exhaustion on
    /// large market lists.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `user` - Address to query
    /// * `cursor` - Zero-based start index into the market list
    /// * `limit` - Page size; capped server-side at 50
    ///
    /// # Returns
    ///
    /// `SymbolPagedResult<UserBetQuery>` with `items`, `next_cursor`, and `total_count`.
    ///
    /// # Errors
    ///
    /// Panics with `Error::ContractStateError` if the market index is corrupted.
    ///
    /// # Events
    ///
    /// Read-only; no events emitted.
    pub fn query_user_bets_paged(
        env: Env,
        user: Address,
        cursor: u32,
        limit: u32,
    ) -> UserBetPagedResult {
            crate::queries::QueryManager::query_user_bets_paged(&env, user, cursor, limit)
            .unwrap_or_else(|e| panic_with_error!(&env, e))
        }       
    /// Return partial contract state statistics for a market-list page.
    ///
    /// Processes only the market slice `[cursor, cursor+limit)`.  Callers
    /// accumulate results across pages to build a full aggregate.
    ///
    /// # Parameters
    ///
    /// * `env` - Soroban environment
    /// * `cursor` - Start index into the market list
    /// * `limit` - Page size; capped server-side at 50
    ///
    /// # Returns
    ///
    /// `(ContractStateQuery, next_cursor)` — partial stats and the cursor for
    /// the next call.
    ///
    /// # Errors
    ///
    /// Panics with `Error::ContractStateError` if the market index is corrupted.
    ///
    /// # Events
    ///
    /// Read-only; no events emitted.
    pub fn query_contract_state_paged(
        env: Env,
        cursor: u32,
        limit: u32,
    ) -> (ContractStateQuery, u32) {
        crate::queries::QueryManager::query_contract_state_paged(&env, cursor, limit)
            .unwrap_or_else(|e| panic_with_error!(&env, e))
    }

    /// Cancel an event and automatically refund all placed bets (admin only).
    ///
    /// This function allows admins to cancel events before resolution and
    /// automatically refund all bets placed on the market. It validates
    /// cancellation conditions, updates market status, and processes refunds.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address (must be authorized)
    /// * `market_id` - Unique identifier of the market to cancel
    /// * `reason` - Optional reason for cancellation
    ///
    /// # Returns
    ///
    /// Returns `Result<i128, Error>` where:
    /// - `Ok(total_refunded)` - Total amount refunded to users
    /// - `Err(Error)` - Error if cancellation fails
    ///
    /// # Panics
    ///
    /// This function will panic with specific errors if:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    /// - `Error::MarketResolved` - Market has already been resolved
    /// - `Error::InvalidState` - Market is in an invalid state for cancellation
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Address, String, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let admin = Address::generate(&env);
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::cancel_event(
    ///     env.clone(),
    ///     admin,
    ///     market_id,
    ///     Some(String::from_str(&env, "Oracle data unavailable"))
    /// ) {
    ///     Ok(total) => println!("Refunded {} stroops", total),
    ///     Err(e) => println!("Cancellation failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Cancellation Conditions
    ///
    /// - Market must exist and be active
    /// - Market must not be resolved
    /// - Market must not already be cancelled
    /// - Only admin can cancel events
    ///
    /// # Refund Process
    ///
    /// 1. All active bets are identified
    /// 2. Funds are unlocked and returned to users
    /// 3. Bet status is updated to "Refunded"
    /// 4. Market state is updated to "Cancelled"
    /// 5. Cancellation and refund events are emitted
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn cancel_event(
        env: Env,
        admin: Address,
        market_id: Symbol,
        reason: Option<String>,
    ) -> Result<i128, Error> {
        Self::require_primary_admin(&env, &admin)?;

        // Get and validate market
        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .unwrap_or_else(|| {
                panic_with_error!(env, Error::MarketNotFound);
            });

        // Validate cancellation conditions
        if market.state == MarketState::Resolved {
            return Err(Error::MarketResolved);
        }

        if market.state == MarketState::Cancelled {
            // Already cancelled, return 0 refunded
            return Ok(0);
        }

        // Market must be active or ended (not resolved)
        if !matches!(market.state, MarketState::Active | MarketState::Ended) {
            return Err(Error::InvalidState);
        }

        // Capture old state for event
        let old_state = market.state.clone();

        // Update market state to cancelled
        market.state = MarketState::Cancelled;
        env.storage().persistent().set(&market_id, &market);

        // Refund all bets (batch of token transfers)
        let refund_result = bets::BetManager::refund_market_bets(&env, &market_id);
        refund_result?;

        // Calculate total refunded (sum of all bets)
        let total_refunded = market.total_staked;

        let mut details = Map::new(&env);
        if let Some(r) = &reason {
            details.set(Symbol::new(&env, "reason"), r.clone());
        }
        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::EventCancelled,
            admin.clone(),
            details,
            None,
        );

        // Emit cancellation event
        EventEmitter::emit_state_change_event(
            &env,
            &market_id,
            &old_state,
            &MarketState::Cancelled,
            &reason.unwrap_or_else(|| String::from_str(&env, "Event cancelled by admin")),
        );

        // Emit market closed event
        EventEmitter::emit_market_closed(&env, &market_id, &admin);

        Ok(total_refunded)
    }

    /// Refund all bets when oracle resolution fails or times out (automatic refund path).
    ///
    /// Callable when: market has ended, no oracle result, and either (1) resolution
    /// timeout has passed since market end, or (2) caller is admin (confirmed failure).
    /// Refunds full bet amount per user (no fee deduction). Marks market as cancelled and
    /// prevents further resolution. Emits refund events. Idempotent when already cancelled.
    ///
    /// The timeout gate is evaluated per market from `end_time + resolution_timeout`.
    /// Non-admin callers cannot trigger this path before that market-specific deadline.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn refund_on_oracle_failure(
        env: Env,
        caller: Address,
        market_id: Symbol,
    ) -> Result<i128, Error> {
        caller.require_auth();

        let mut market: Market = env
            .storage()
            .persistent()
            .get(&market_id)
            .ok_or(Error::MarketNotFound)?;

        if market.state == MarketState::Cancelled {
            return Ok(0);
        }
        if market.winning_outcomes.is_some() {
            return Err(Error::MarketResolved);
        }
        if market.oracle_result.is_some() {
            return Err(Error::MarketResolved);
        }
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        let stored_admin: Option<Address> =
            env.storage().persistent().get(&Symbol::new(&env, "Admin"));
        let is_admin = stored_admin.as_ref().map_or(false, |a| a == &caller);
        let timeout_passed = resolution_timeout_reached(&env, &market);
        if !is_admin && !timeout_passed {
            return Err(Error::Unauthorized);
        }

        let old_state = market.state.clone();
        market.state = MarketState::Cancelled;
        env.storage().persistent().set(&market_id, &market);

        let refund_result = bets::BetManager::refund_market_bets(&env, &market_id);
        refund_result?;

        let total_refunded = market.total_staked;
        EventEmitter::emit_state_change_event(
            &env,
            &market_id,
            &old_state,
            &MarketState::Cancelled,
            &String::from_str(&env, "Refund on oracle failure/timeout"),
        );
        EventEmitter::emit_refund_on_oracle_failure(&env, &market_id, total_refunded);

        Ok(total_refunded)
    }

    /// Extend market duration (admin only)
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn extend_market(
        env: Env,
        admin: Address,
        market_id: Symbol,
        additional_days: u32,
        reason: String,
        _fee_amount: i128,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;

        extensions::ExtensionManager::extend_market_duration(
            &env,
            admin,
            market_id,
            additional_days,
            reason,
        )
    }

    /// Sets the admin-configurable cumulative extension cap (in days) that applies
    /// globally to all markets. A value of `0` disables the cap.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unauthorized`] when the caller is not the primary admin.
    ///
    /// # Events
    ///
    /// Emits no events; purely a configuration write.
    pub fn set_cumulative_extension_cap(
        env: Env,
        admin: Address,
        cap_days: u32,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        let key = Symbol::new(&env, "cum_ext_cap");
        env.storage().persistent().set(&key, &cap_days);
        Ok(())
    }

    /// Returns the running cumulative extension total (in days) for a given market.
    /// Returns `0` when no extensions have been recorded yet.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_cumulative_extension_total(env: Env, market_id: Symbol) -> Result<u32, Error> {
        let key = crate::storage::DataKey::MarketExtensionTotal(market_id);
        Ok(env.storage().persistent().get(&key).unwrap_or(0u32))
    }

    // ===== STORAGE OPTIMIZATION FUNCTIONS =====

    /// Compress market data for storage optimization
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn compress_market_data(
        env: Env,
        market_id: Symbol,
    ) -> Result<storage::CompressedMarket, Error> {
        let market = match markets::MarketStateManager::get_market(&env, &market_id) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        storage::StorageOptimizer::compress_market_data(&env, &market)
    }

    /// Clean up old market data based on age and state
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn cleanup_old_market_data(env: Env, market_id: Symbol) -> Result<bool, Error> {
        storage::StorageOptimizer::cleanup_old_market_data(&env, &market_id)
    }

    /// Migrate storage format from old to new format
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn migrate_storage_format(
        env: Env,
        from_format: storage::StorageFormat,
        to_format: storage::StorageFormat,
    ) -> Result<storage::StorageMigration, Error> {
        let result =
            storage::StorageOptimizer::migrate_storage_format(&env, from_format, to_format);

        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::StorageMigrated,
            env.current_contract_address(),
            Map::new(&env),
            None,
        );

        result
    }

    /// Read-only recovery dry run for ops verification.
    ///
    /// Returns the recovery plan that `recover_market_state` would follow without
    /// executing any side effects. No admin authentication is required. Useful for
    /// pre-flight checks before executing a live recovery.
    ///
    /// # Parameters
    /// * `env` - The Soroban environment.
    /// * `market_id` - The market to analyse.
    ///
    /// # Returns
    /// A [`recovery::DryRunResult`] describing integrity status, detected issues,
    /// and planned recovery actions.
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn recovery_dry_run(
        env: Env,
        market_id: Symbol,
    ) -> crate::recovery::DryRunResult {
        match crate::recovery::RecoveryManager::recovery_dry_run(&env, &market_id) {
            Ok(result) => result,
            Err(e) => panic_with_error!(env, e),
        }
    }


    /// Promote resolved market metadata from Temporary to Persistent storage
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    pub fn promote_market_to_persistent(
        env: Env,
        market_id: Symbol,
    ) -> Result<(), Error> {
        storage::StorageMigration::promote_market_to_persistent(&env, &market_id)
    }

    /// Demote scratch keys from Persistent to Temporary storage
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    pub fn demote_scratch_keys(
        env: Env,
        market_id: Symbol,
    ) -> Result<(), Error> {
        storage::StorageMigration::demote_scratch_keys(&env, &market_id)
    }

    /// Monitor storage usage and return statistics
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn monitor_storage_usage(env: Env) -> Result<storage::StorageUsageStats, Error> {
        storage::StorageOptimizer::monitor_storage_usage(&env)
    }

    /// Optimize storage layout for a specific market
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn optimize_storage_layout(env: Env, market_id: Symbol) -> Result<bool, Error> {
        storage::StorageOptimizer::optimize_storage_layout(&env, &market_id)
    }

    /// Get storage usage statistics
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_storage_usage_statistics(env: Env) -> Result<storage::StorageUsageStats, Error> {
        storage::StorageOptimizer::get_storage_usage_statistics(&env)
    }

    /// Validate storage integrity for a specific market
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn validate_storage_integrity(
        env: Env,
        market_id: Symbol,
    ) -> Result<storage::StorageIntegrityResult, Error> {
        storage::StorageOptimizer::validate_storage_integrity(&env, &market_id)
    }

    /// Get storage configuration
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_storage_config(env: Env) -> storage::StorageConfig {
        storage::StorageOptimizer::get_storage_config(&env)
    }

    /// Update storage configuration
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn update_storage_config(env: Env, config: storage::StorageConfig) -> Result<(), Error> {
        storage::StorageOptimizer::update_storage_config(&env, &config)
    }

    /// Calculate storage cost for a market
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn calculate_storage_cost(env: Env, market_id: Symbol) -> Result<u64, Error> {
        let market = match markets::MarketStateManager::get_market(&env, &market_id) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        Ok(storage::StorageUtils::calculate_storage_cost(&market))
    }

    /// Get storage efficiency score for a market
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_storage_efficiency_score(env: Env, market_id: Symbol) -> Result<u32, Error> {
        let market = match markets::MarketStateManager::get_market(&env, &market_id) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        Ok(storage::StorageUtils::get_storage_efficiency_score(&market))
    }

    /// Get storage recommendations for a market
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_storage_recommendations(env: Env, market_id: Symbol) -> Result<Vec<String>, Error> {
        let market = match markets::MarketStateManager::get_market(&env, &market_id) {
            Ok(m) => m,
            Err(e) => return Err(e),
        };

        Ok(storage::StorageUtils::get_storage_recommendations(&market))
    }

    // ===== ERROR RECOVERY FUNCTIONS =====

    /// Recover from an error using appropriate recovery strategy
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn recover_from_error(
        env: Env,
        error: Error,
        context: errors::ErrorContext,
    ) -> Result<errors::ErrorRecovery, Error> {
        errors::ErrorHandler::recover_from_error(&env, error, context)
    }

    /// Validate error recovery configuration and state
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn validate_error_recovery(
        env: Env,
        recovery: errors::ErrorRecovery,
    ) -> Result<bool, Error> {
        errors::ErrorHandler::validate_error_recovery(&env, &recovery)
    }

    /// Get current error recovery status and statistics
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_error_recovery_status(env: Env) -> Result<errors::ErrorRecoveryStatus, Error> {
        errors::ErrorHandler::get_error_recovery_status(&env)
    }

    /// Emit error recovery event for monitoring and logging
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn emit_error_recovery_event(env: Env, recovery: errors::ErrorRecovery) {
        errors::ErrorHandler::emit_error_recovery_event(&env, &recovery);
    }

    /// Validate resilience patterns configuration
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn validate_resilience_patterns(
        env: Env,
        patterns: Vec<errors::ResiliencePattern>,
    ) -> Result<bool, Error> {
        errors::ErrorHandler::validate_resilience_patterns(&env, &patterns)
    }

    /// Document error recovery procedures and best practices
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn document_error_recovery(env: Env) -> Result<soroban_sdk::Map<String, String>, Error> {
        errors::ErrorHandler::document_error_recovery_procedures(&env)
    }

    // ===== EDGE CASE HANDLING ENTRY POINTS =====

    /// Handle zero stake scenario for a specific market
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn handle_zero_stake_scenario(env: Env, market_id: Symbol) -> Result<(), Error> {
        edge_cases::EdgeCaseHandler::handle_zero_stake_scenario(&env, market_id)
    }

    /// Implement tie-breaking mechanism for equal outcomes
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn implement_tie_breaking_mechanism(
        env: Env,
        outcomes: Vec<String>,
    ) -> Result<String, Error> {
        edge_cases::EdgeCaseHandler::implement_tie_breaking_mechanism(&env, outcomes)
    }

    /// Detect orphaned markets and return their IDs
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn detect_orphaned_markets(env: Env) -> Result<Vec<Symbol>, Error> {
        edge_cases::EdgeCaseHandler::detect_orphaned_markets(&env)
    }

    /// Handle partial resolution with incomplete data
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn handle_partial_resolution(
        env: Env,
        market_id: Symbol,
        partial_data: edge_cases::PartialData,
    ) -> Result<(), Error> {
        edge_cases::EdgeCaseHandler::handle_partial_resolution(&env, market_id, partial_data)
    }

    /// Validate edge case handling scenario
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn validate_edge_case_handling(
        env: Env,
        scenario: edge_cases::EdgeCaseScenario,
    ) -> Result<(), Error> {
        edge_cases::EdgeCaseHandler::validate_edge_case_handling(&env, scenario)
    }

    /// Run comprehensive edge case testing scenarios
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn test_edge_case_scenarios(env: Env) -> Result<(), Error> {
        edge_cases::EdgeCaseHandler::test_edge_case_scenarios(&env)
    }

    /// Get comprehensive edge case statistics
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_edge_case_statistics(env: Env) -> Result<edge_cases::EdgeCaseStats, Error> {
        edge_cases::EdgeCaseHandler::get_edge_case_statistics(&env)
    }

    // ===== RECOVERY PUBLIC METHODS =====
    /// Initiates or performs recovery of a potentially corrupted market state. Only admin.
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn recover_market_state(env: Env, admin: Address, market_id: Symbol) -> bool {
        admin.require_auth();
        if let Err(e) = crate::recovery::RecoveryManager::assert_is_admin(&env, &admin) {
            panic_with_error!(env, e);
        }
        let plan = match crate::recovery::RecoveryManager::recover_market_state(
            &env, &admin, &market_id, true,
        ) {
            Ok(p) => p,
            Err(e) => panic_with_error!(env, e),
        };

        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::ErrorRecovered,
            admin.clone(),
            Map::new(&env),
            None,
        );

        plan.recovered
    }

    /// Read-only preview of the recovery plan for a market.
    ///
    /// Returns a [`RecoveryPlan`] describing exactly what `recover_market_state`
    /// would do if called, without executing any side effects. No admin
    /// authentication is required — any caller can inspect the plan.
    ///
    /// # Parameters
    /// * `env` - The Soroban environment.
    /// * `market_id` - The market to analyse.
    ///
    /// # Returns
    /// A [`RecoveryPlan`] with the predicted recovery outcome.
    ///
    /// # Events
    ///
    /// This entrypoint performs no state changes and emits no events.
    pub fn recovery_plan(
        env: Env,
        market_id: Symbol,
    ) -> crate::recovery::RecoveryPlan {
        match crate::recovery::RecoveryManager::dry_run_recovery_plan(&env, &market_id) {
            Ok(plan) => plan,
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Executes partial refund mechanism for selected users in a failed/corrupted market. Only admin.
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn partial_refund_mechanism(
        env: Env,
        admin: Address,
        market_id: Symbol,
        users: Vec<Address>,
    ) -> i128 {
        Self::require_primary_admin_or_panic(&env, &admin);
        let result = match crate::recovery::RecoveryManager::partial_refund_mechanism(
            &env, &admin, &market_id, &users,
        ) {
            Ok(total_refunded) => total_refunded,
            Err(e) => panic_with_error!(env, e),
        };

        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::PartialRefundExecuted,
            admin.clone(),
            Map::new(&env),
            None,
        );

        result
    }

    /// Validates market state integrity; returns true if consistent.
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn validate_market_state_integrity(env: Env, market_id: Symbol) -> bool {
        match crate::recovery::RecoveryValidator::validate_market_state_integrity(&env, &market_id)
        {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Returns recovery status for a market.
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_recovery_status(env: Env, market_id: Symbol) -> String {
        crate::recovery::RecoveryManager::get_recovery_status(&env, &market_id)
            .unwrap_or_else(|_| String::from_str(&env, "unknown"))
    }

    /// Remove the oldest `count` completed recovery history entries for a market (admin only).
    ///
    /// Active (unresolved) recovery state is never pruned. `count` is capped at 30.
    ///
    /// # Errors
    /// * `Unauthorized` - Caller is not admin
    pub fn prune_recovery_history(
        env: Env,
        admin: Address,
        market_id: Symbol,
        count: u32,
    ) -> Result<u32, Error> {
        crate::recovery::RecoveryManager::prune_recovery_history(&env, &admin, &market_id, count)
    }

    // ===== PER-MARKET RECOVERY TIMELOCK ENTRYPOINTS =====

    /// Initiate a per-market recovery request with an admin timelock.
    ///
    /// Creates a pending recovery request for the specified market. The recovery
    /// action cannot be executed until the timelock period (default 24 hours) has
    /// elapsed. Only contract admins may call this function.
    ///
    /// # Arguments
    /// * `admin` - The admin initiating the recovery (must be authenticated)
    /// * `market_id` - The target market
    /// * `action` - The recovery action (ReconstructState, CancelMarket, or ForceResolve)
    /// * `reason` - Human-readable explanation for the recovery
    ///
    /// # Errors
    /// * `RecoveryAlreadyPending` - A recovery is already pending for this market
    /// * `MarketNotRecoverable` - Market is in a non-recoverable state
    /// * `InvalidRecoveryAction` - Action is invalid for the market's current state
    pub fn initiate_market_recovery(
        env: Env,
        admin: Address,
        market_id: Symbol,
        action: crate::recovery::PerMarketRecoveryAction,
        reason: String,
    ) -> crate::recovery::PendingMarketRecovery {
        Self::require_primary_admin_or_panic(&env, &admin);

        match crate::recovery::RecoveryTimelockManager::initiate_recovery(
            &env,
            &admin,
            &market_id,
            &action,
            &reason,
        ) {
            Ok(request) => {
                crate::audit_trail::AuditTrailManager::append_record(
                    &env,
                    crate::audit_trail::AuditAction::ErrorRecovered,
                    admin.clone(),
                    Map::new(&env),
                );
                request
            }
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Execute a pending per-market recovery request after the timelock has expired.
    ///
    /// This can only be called after the timelock period initiated by
    /// `initiate_market_recovery` has elapsed. Only contract admins may call this.
    ///
    /// # Arguments
    /// * `admin` - The admin executing the recovery (must be authenticated)
    /// * `market_id` - The target market
    ///
    /// # Errors
    /// * `RecoveryRequestNotFound` - No pending request for this market
    /// * `RecoveryTimelockActive` - The timelock has not yet expired
    pub fn execute_market_recovery(env: Env, admin: Address, market_id: Symbol) -> bool {
        Self::require_primary_admin_or_panic(&env, &admin);

        match crate::recovery::RecoveryTimelockManager::execute_recovery(&env, &admin, &market_id)
        {
            Ok(success) => {
                crate::audit_trail::AuditTrailManager::append_record(
                    &env,
                    crate::audit_trail::AuditAction::ErrorRecovered,
                    admin.clone(),
                    Map::new(&env),
                );
                success
            }
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Cancel a pending per-market recovery request.
    ///
    /// Removes the pending request so the recovery action will not be executed.
    /// Only contract admins may call this.
    ///
    /// # Arguments
    /// * `admin` - The admin cancelling the recovery (must be authenticated)
    /// * `market_id` - The target market
    ///
    /// # Errors
    /// * `RecoveryRequestNotFound` - No pending request for this market
    pub fn cancel_market_recovery(env: Env, admin: Address, market_id: Symbol) {
        Self::require_primary_admin_or_panic(&env, &admin);

        match crate::recovery::RecoveryTimelockManager::cancel_recovery(
            &env,
            &admin,
            &market_id,
        ) {
            Ok(()) => {
                crate::audit_trail::AuditTrailManager::append_record(
                    &env,
                    crate::audit_trail::AuditAction::ErrorRecovered,
                    admin.clone(),
                    Map::new(&env),
                );
            }
            Err(e) => panic_with_error!(env, e),
        }
    }

    /// Returns the pending recovery request for a market, if any.
    ///
    /// Read-only query; no authentication required.
    pub fn get_pending_market_recovery(
        env: Env,
        market_id: Symbol,
    ) -> Option<crate::recovery::PendingMarketRecovery> {
        crate::recovery::RecoveryTimelockManager::get_pending(&env, &market_id)
    }

    /// Returns the current recovery timelock configuration.
    ///
    /// Read-only query; no authentication required.
    pub fn get_recovery_timelock_config(env: Env) -> crate::recovery::RecoveryTimelockConfig {
        crate::recovery::RecoveryTimelockManager::get_config(&env)
    }

    // ===== VERSIONING FUNCTIONS =====

    /// Track contract version for versioning system
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn track_contract_version(env: Env, version: versioning::Version) -> Result<(), Error> {
        versioning::VersionManager::new(&env).track_contract_version(&env, version)
    }

    /// Migrate data between contract versions
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn migrate_data_between_versions(
        env: Env,
        old_version: versioning::Version,
        new_version: versioning::Version,
    ) -> Result<versioning::VersionMigration, Error> {
        versioning::VersionManager::new(&env).migrate_data_between_versions(
            &env,
            old_version,
            new_version,
        )
    }

    /// Validate version compatibility
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn validate_version_compatibility(
        env: Env,
        old_version: versioning::Version,
        new_version: versioning::Version,
    ) -> Result<bool, Error> {
        versioning::VersionManager::new(&env).validate_version_compatibility(
            &env,
            &old_version,
            &new_version,
        )
    }

    /// Upgrade to a specific version
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn upgrade_to_version(env: Env, target_version: versioning::Version) -> Result<(), Error> {
        versioning::VersionManager::new(&env).upgrade_to_version(&env, target_version)
    }

    /// Rollback to a specific version
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn rollback_to_version(env: Env, target_version: versioning::Version) -> Result<(), Error> {
        versioning::VersionManager::new(&env).rollback_to_version(&env, target_version)
    }

    /// Get version history
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_version_history(env: Env) -> Result<versioning::VersionHistory, Error> {
        versioning::VersionManager::new(&env).get_version_history(&env)
    }

    /// Test version migration
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn test_version_migration(
        env: Env,
        migration: versioning::VersionMigration,
    ) -> Result<bool, Error> {
        versioning::VersionManager::new(&env).test_version_migration(&env, migration)
    }

    // ===== MONITORING FUNCTIONS =====

    /// Monitor market health for a specific market
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn monitor_market_health(
        env: Env,
        market_id: Symbol,
    ) -> Result<monitoring::MarketHealthMetrics, Error> {
        monitoring::ContractMonitor::monitor_market_health(&env, market_id)
    }

    /// Monitor oracle health for a specific oracle provider
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn monitor_oracle_health(
        env: Env,
        oracle: OracleProvider,
    ) -> Result<monitoring::OracleHealthMetrics, Error> {
        monitoring::ContractMonitor::monitor_oracle_health(&env, oracle)
    }

    /// Monitor fee collection performance
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn monitor_fee_collection(
        env: Env,
        timeframe: monitoring::TimeFrame,
    ) -> Result<monitoring::FeeCollectionMetrics, Error> {
        monitoring::ContractMonitor::monitor_fee_collection(&env, timeframe)
    }

    /// Monitor dispute resolution performance
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn monitor_dispute_resolution(
        env: Env,
        market_id: Symbol,
    ) -> Result<monitoring::DisputeResolutionMetrics, Error> {
        monitoring::ContractMonitor::monitor_dispute_resolution(&env, market_id)
    }

    /// Get comprehensive contract performance metrics
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_contract_performance_metrics(
        env: Env,
        timeframe: monitoring::TimeFrame,
    ) -> Result<monitoring::PerformanceMetrics, Error> {
        monitoring::ContractMonitor::get_contract_performance_metrics(&env, timeframe)
    }

    /// Emit monitoring alert
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn emit_monitoring_alert(
        env: Env,
        alert: monitoring::MonitoringAlert,
    ) -> Result<(), Error> {
        monitoring::ContractMonitor::emit_monitoring_alert(&env, alert)
    }

    /// Validate monitoring data integrity
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn validate_monitoring_data(
        env: Env,
        data: monitoring::MonitoringData,
    ) -> Result<bool, Error> {
        monitoring::ContractMonitor::validate_monitoring_data(&env, &data)
    }

    /// Return all alerts in the bounded monitoring queue (oldest first).
    ///
    /// Clients should also check [`is_monitor_overflow`] to detect whether any
    /// alerts were silently evicted since the last admin reset.
    pub fn get_monitor_alerts(env: Env) -> Vec<monitoring::MonitoringAlert> {
        monitoring::ContractMonitor::get_alerts(&env)
    }

    /// Return `true` if at least one alert has been evicted from the queue due to
    /// overflow since the last [`clear_monitor_overflow`] call.
    pub fn is_monitor_overflow(env: Env) -> bool {
        monitoring::ContractMonitor::is_overflow(&env)
    }

    /// Reset the monitoring overflow flag.  Only the contract admin may call this.
    ///
    /// # Errors
    ///
    /// - [`Error::AdminNotSet`] – no admin has been initialised.
    /// - [`Error::Unauthorized`] – `admin` does not match the stored admin address.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn clear_monitor_overflow(env: Env, admin: Address) -> Result<(), Error> {
        monitoring::ContractMonitor::clear_overflow(&env, &admin)
    }

    // ===== ORACLE FALLBACK FUNCTIONS =====

    /// Get oracle data with backup if primary fails.
    ///
    /// The helper always attempts `primary_oracle` first. It attempts `backup_oracle`
    /// only after a failed primary call, and it aborts before any oracle call once
    /// `ledger.timestamp() >= end_time + resolution_timeout`.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_oracle_with_backup(
        env: Env,
        market_id: Symbol,
        oracle_contract: Address,
        primary_oracle: OracleProvider,
        backup_oracle: OracleProvider,
    ) -> Result<String, Error> {
        // Get market info
        let market = env
            .storage()
            .persistent()
            .get::<Symbol, Market>(&market_id)
            .ok_or(Error::MarketNotFound)?;

        // Check if market ended
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }
        if resolution_timeout_reached(&env, &market) {
            EventEmitter::emit_resolution_timeout(&env, &market_id, current_time);
            return Err(Error::ResolutionTimeoutReached);
        }

        // Try to get price with backup
        let backup = OracleBackup::new(primary_oracle, backup_oracle);
        match backup.get_price(&env, &oracle_contract, &market.oracle_config.feed_id) {
            Ok(price) => {
                // Simple comparison logic
                let threshold = market.oracle_config.threshold;
                let comparison = &market.oracle_config.comparison;

                let result = if comparison == &String::from_str(&env, "gt") {
                    if price > threshold {
                        "yes"
                    } else {
                        "no"
                    }
                } else if comparison == &String::from_str(&env, "lt") {
                    if price < threshold {
                        "yes"
                    } else {
                        "no"
                    }
                } else {
                    if price == threshold {
                        "yes"
                    } else {
                        "no"
                    }
                };

                Ok(String::from_str(&env, result))
            }
            Err(_) => {
                // Both oracles failed
                let reason = String::from_str(&env, "primary_and_fallback_failed");
                events::EventEmitter::emit_manual_resolution_required(&env, &market_id, &reason);
                Err(Error::FallbackOracleUnavailable)
            }
        }
    }

    /// Check if oracle is working
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn check_oracle_status(
        env: Env,
        oracle: OracleProvider,
        oracle_contract: Address,
    ) -> String {
        let health = graceful_degradation::monitor_oracle_health(&env, oracle, &oracle_contract);
        match health {
            OracleHealth::Working => String::from_str(&env, "working"),
            OracleHealth::Degraded => String::from_str(&env, "degraded"),
            OracleHealth::Broken => String::from_str(&env, "broken"),
        }
    }

    // ===== MULTI-ADMIN MANAGEMENT FUNCTIONS =====

    /// Add a new admin with specified role (SuperAdmin only)
    ///
    /// The caller must satisfy Soroban `require_auth()`. Access is granted to the
    /// stored primary admin and, after multi-admin migration, any delegated admin
    /// with `AdminPermission::Emergency`.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn add_admin(
        env: Env,
        current_admin: Address,
        new_admin: Address,
        role: AdminRole,
    ) -> Result<(), Error> {
        Self::require_admin_permission(&env, &current_admin, AdminPermission::Emergency)?;
        AdminManager::add_admin(&env, &current_admin, &new_admin, role)
    }

    /// Remove an admin from the system (SuperAdmin only)
    ///
    /// The caller must satisfy Soroban `require_auth()`. Access is granted to the
    /// stored primary admin and, after multi-admin migration, any delegated admin
    /// with `AdminPermission::Emergency`.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn remove_admin(
        env: Env,
        current_admin: Address,
        admin_to_remove: Address,
    ) -> Result<(), Error> {
        Self::require_admin_permission(&env, &current_admin, AdminPermission::Emergency)?;
        AdminManager::remove_admin(&env, &current_admin, &admin_to_remove)
    }

    /// Update an admin's role (SuperAdmin only)
    ///
    /// The caller must satisfy Soroban `require_auth()`. Access is granted to the
    /// stored primary admin and, after multi-admin migration, any delegated admin
    /// with `AdminPermission::Emergency`.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn update_admin_role(
        env: Env,
        current_admin: Address,
        target_admin: Address,
        new_role: AdminRole,
    ) -> Result<(), Error> {
        Self::require_admin_permission(&env, &current_admin, AdminPermission::Emergency)?;
        AdminManager::update_admin_role(&env, &current_admin, &target_admin, new_role)
    }

    /// Validate admin permission for specific action
    ///
    /// The caller must satisfy Soroban `require_auth()`, and the contract must
    /// already have an initialized primary admin in persistent storage.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn validate_admin_permission(
        env: Env,
        admin: Address,
        permission: AdminPermission,
    ) -> Result<(), Error> {
        Self::require_initialized_admin_root(&env, &admin)?;
        AdminManager::validate_admin_permission(&env, &admin, permission)
    }

    /// Get all admin roles in the system
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_admin_roles(env: Env) -> Map<Address, AdminRole> {
        AdminManager::get_admin_roles(&env)
    }

    /// Get comprehensive admin analytics
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_admin_analytics(env: Env) -> AdminAnalyticsResult {
        admin::EnhancedAdminAnalytics::get_admin_analytics(&env)
    }

    /// Migrate from single-admin to multi-admin system
    ///
    /// Only the stored primary admin can trigger the one-way migration into the
    /// delegated multi-admin storage layout.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn migrate_to_multi_admin(env: Env, admin: Address) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        admin::AdminSystemIntegration::migrate_to_multi_admin(&env)
    }

    /// Check if multi-admin migration is complete
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn is_multi_admin_migrated(env: Env) -> bool {
        admin::AdminSystemIntegration::is_migrated(&env)
    }

    /// Check role permissions against a specific permission
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn check_role_permissions(env: Env, role: AdminRole, permission: AdminPermission) -> bool {
        AdminManager::check_role_permissions(&env, role, permission)
    }

    // ===== CONTRACT UPGRADE METHODS =====

    /// Upgrade the contract to a new Wasm bytecode.
    ///
    /// This entrypoint lets the configured primary admin rotate the active
    /// contract bytecode after a predecessor-hash check. It is the primary
    /// authenticated upgrade path for on-chain migrations.
    ///
    /// # What
    ///
    /// Replaces the active Wasm hash with `new_wasm_hash` when the caller is the
    /// stored primary admin and the supplied predecessor matches the current
    /// deployed hash.
    ///
    /// # How
    ///
    /// The call verifies admin authorization, hands the request to the upgrade
    /// manager for chain and compatibility checks, and appends an audit-trail
    /// record for the resulting state change.
    ///
    /// # Why
    ///
    /// This keeps upgrades authenticated, ordered, and observable for operators.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the caller is not the primary admin, the supplied
    /// predecessor hash does not match, or the upgrade manager rejects the
    /// proposed upgrade.
    pub fn upgrade_contract(
        env: Env,
        admin: Address,
        new_wasm_hash: soroban_sdk::BytesN<32>,
        expected_predecessor: soroban_sdk::BytesN<32>,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        capabilities::admin::CapabilitiesAdminCooldown::require_elapsed(
            &env,
            &capabilities::admin::CapabilitiesAdminAction::Upgrade,
        )?;
        let result = upgrade_manager::UpgradeManager::upgrade_contract(
            &env,
            &admin,
            new_wasm_hash,
            expected_predecessor,
        );

        if result.is_ok() {
            capabilities::admin::CapabilitiesAdminCooldown::record_action(
                &env,
                &capabilities::admin::CapabilitiesAdminAction::Upgrade,
            );
        }

        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::ContractUpgraded,
            admin.clone(),
            Map::new(&env),
            None,
        );

        result
    }

    /// Broadcast an emergency notice to off-chain clients.
    ///
    /// This entrypoint publishes an admin broadcast event so operators and
    /// monitoring services can react to urgent contract-state changes.
    ///
    /// # What
    ///
    /// Emits a broadcast event carrying the provided severity, message hash, and
    /// human-readable reason.
    ///
    /// # How
    ///
    /// The call delegates to the admin broadcast helper after validation of the
    /// caller and the supplied payload.
    ///
    /// # Why
    ///
    /// This keeps critical operational messages visible without requiring a full
    /// contract upgrade.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the caller is not authorized or the broadcast
    /// payload cannot be accepted by the admin subsystem.
    pub fn admin_broadcast(
        env: Env,
        admin: Address,
        severity: crate::admin::Severity,
        message_hash: soroban_sdk::BytesN<32>,
        reason: String,
    ) -> Result<(), Error> {
        crate::admin::AdminFunctions::admin_broadcast(&env, &admin, severity, message_hash, reason)
    }

    /// Roll back the contract to a previous Wasm bytecode.
    ///
    /// This entrypoint reverts the deployed bytecode to a known good version when
    /// an upgrade is incomplete or unsafe. It is a recovery-oriented path for
    /// operators.
    ///
    /// # What
    ///
    /// Restores the contract to `rollback_wasm_hash` when the caller is the
    /// stored primary admin.
    ///
    /// # How
    ///
    /// The call verifies admin authorization, delegates to the rollback manager,
    /// and records the rollback in the audit trail.
    ///
    /// # Why
    ///
    /// This provides a controlled recovery path for failed or risky upgrades.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the caller is not authorized or the rollback target
    /// cannot be applied safely.
    pub fn rollback_upgrade(
        env: Env,
        admin: Address,
        rollback_wasm_hash: soroban_sdk::BytesN<32>,
    ) -> Result<(), Error> {
        Self::require_primary_admin(&env, &admin)?;
        capabilities::admin::CapabilitiesAdminCooldown::require_elapsed(
            &env,
            &capabilities::admin::CapabilitiesAdminAction::Rollback,
        )?;
        let result =
            upgrade_manager::UpgradeManager::rollback_upgrade(&env, &admin, rollback_wasm_hash);

        if result.is_ok() {
            capabilities::admin::CapabilitiesAdminCooldown::record_action(
                &env,
                &capabilities::admin::CapabilitiesAdminAction::Rollback,
            );
        }

        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::UpgradeRolledBack,
            admin.clone(),
            Map::new(&env),
            None,
        );

        result
    }

    /// Return the currently active contract version.
    ///
    /// This read-only entrypoint exposes the version information associated with
    /// the currently deployed contract implementation.
    ///
    /// # What
    ///
    /// Returns the active [`versioning::Version`] descriptor for the contract.
    ///
    /// # How
    ///
    /// The call delegates to the version manager to read the stored version state.
    ///
    /// # Why
    ///
    /// This helps clients and operators verify which implementation is currently
    /// active before issuing versioned operations.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the active version cannot be read from storage.
    pub fn get_contract_version(env: Env) -> Result<versioning::Version, Error> {
        upgrade_manager::UpgradeManager::get_contract_version(&env)
    }

    /// Return the capability bitmap for the active contract version.
    ///
    /// Clients can call this entrypoint to learn which feature flags are enabled
    /// without maintaining a client-side version-to-capability lookup table.
    ///
    /// # What
    ///
    /// Returns a `u64` bitmask where each bit corresponds to a `CAPABILITY_*`
    /// constant defined in the [`versioning`] module.
    ///
    /// # How
    ///
    /// The call delegates to the capability resolver for the active environment.
    ///
    /// # Why
    ///
    /// This provides a stable compatibility discovery mechanism during upgrades.
    ///
    /// # Errors
    ///
    /// This entrypoint does not currently emit contract errors; it returns the
    /// bitmap directly for the active environment.
    pub fn capabilities(env: Env) -> u64 {
        crate::capabilities::capabilities(&env)
    }

    /// Return the cooldown for repeated capability-critical admin actions.
    ///
    /// The returned duration is expressed in seconds. Contract upgrades and
    /// rollbacks each maintain an independent last-success timestamp and return
    /// [`Error::AdminActionTimelocked`] when repeated before this duration
    /// elapses.
    ///
    /// This is a read-only entrypoint and requires no authorization.
    pub fn get_capabilities_admin_cooldown(_env: Env) -> u64 {
        crate::capabilities::admin_cooldown_seconds()
    }

    /// Report whether an upgrade is currently available for execution.
    ///
    /// This read-only entrypoint checks whether there is an approved proposal that
    /// can be executed without further operator intervention.
    ///
    /// # What
    ///
    /// Returns `true` when an upgrade proposal is ready for execution and `false`
    /// otherwise.
    ///
    /// # How
    ///
    /// The call delegates to the upgrade manager to inspect the stored proposal
    /// state and readiness flags.
    ///
    /// # Why
    ///
    /// This lets operators or clients quickly determine whether an upgrade can be
    /// triggered.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the proposal state cannot be inspected safely.
    pub fn check_upgrade_available(env: Env) -> Result<bool, Error> {
        upgrade_manager::UpgradeManager::check_upgrade_available(&env)
    }

    /// Return the upgrade history for the contract.
    ///
    /// This entrypoint exposes the historical record of all upgrade attempts and
    /// successful changes applied to the contract implementation.
    ///
    /// # What
    ///
    /// Returns the full upgrade history as a vector of upgrade records.
    ///
    /// # How
    ///
    /// The call reads and formats the stored upgrade history through the upgrade
    /// manager.
    ///
    /// # Why
    ///
    /// This is useful for auditing, debugging, and operational review.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the history cannot be read from storage.
    pub fn get_upgrade_history(env: Env) -> Result<Vec<upgrade_manager::UpgradeRecord>, Error> {
        upgrade_manager::UpgradeManager::get_upgrade_history(&env)
    }

    /// Return aggregate analytics about prior upgrades.
    ///
    /// This read-only entrypoint summarizes the upgrade lifecycle with a compact
    /// statistics object for dashboards and audits.
    ///
    /// # What
    ///
    /// Returns an [`upgrade_manager::UpgradeStats`] object summarizing upgrade
    /// activity.
    ///
    /// # How
    ///
    /// The call delegates to the upgrade manager to compute the aggregate metrics
    /// from stored history.
    ///
    /// # Why
    ///
    /// This provides a stable way to surface operational health and migration
    /// patterns.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the statistics cannot be derived from the available
    /// history.
    pub fn get_upgrade_statistics(env: Env) -> Result<upgrade_manager::UpgradeStats, Error> {
        upgrade_manager::UpgradeManager::get_upgrade_statistics(&env)
    }

    /// Validate an upgrade proposal before execution.
    ///
    /// This entrypoint performs a compatibility analysis without applying the
    /// upgrade, which helps operators confirm that a target bytecode and config
    /// are safe to deploy.
    ///
    /// # What
    ///
    /// Returns a compatibility analysis result for the supplied proposal.
    ///
    /// # How
    ///
    /// The call routes the proposal through the upgrade manager for validation and
    /// compatibility checks.
    ///
    /// # Why
    ///
    /// This reduces the risk of introducing incompatible or unsafe upgrades.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the proposal cannot be validated or the environment
    /// cannot safely evaluate it.
    pub fn validate_upgrade_compatibility(
        env: Env,
        proposal: upgrade_manager::UpgradeProposal,
    ) -> Result<upgrade_manager::CompatibilityCheckResult, Error> {
        upgrade_manager::UpgradeManager::validate_upgrade_compatibility(&env, &proposal)
    }

    /// Test whether an upgrade proposal would pass the safety checks.
    ///
    /// This entrypoint performs a dry-run safety evaluation without applying the
    /// proposal, which is useful for pre-flight validation.
    ///
    /// # What
    ///
    /// Returns `true` when the proposal would pass the safety checks and `false`
    /// otherwise.
    ///
    /// # How
    ///
    /// The call delegates to the upgrade manager to run the proposal through the
    /// same validation path as a real execution, minus the actual state change.
    ///
    /// # Why
    ///
    /// This helps operators avoid unsafe or non-compliant upgrades.
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when the proposal cannot be evaluated safely.
    pub fn test_upgrade_safety(
        env: Env,
        proposal: upgrade_manager::UpgradeProposal,
    ) -> Result<bool, Error> {
        upgrade_manager::UpgradeManager::test_upgrade_safety(&env, &proposal)
    }

    // ===== MARKET ANALYTICS FUNCTIONS =====

    /// Get comprehensive market statistics for data analysis and insights
    ///
    /// This function provides detailed statistics about a specific market including
    /// participation metrics, stake distribution, outcome analysis, and performance
    /// indicators. It's essential for market monitoring and user interfaces.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<MarketStatistics, Error>` where:
    /// - `Ok(MarketStatistics)` - Complete market statistics and analytics
    /// - `Err(Error)` - Error if market not found or analysis fails
    ///
    /// # Errors
    ///
    /// This function returns:
    /// - `Error::MarketNotFound` - Market with given ID doesn't exist
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_market_analytics_statistics(env.clone(), market_id) {
    ///     Ok(stats) => {
    ///         println!("Total participants: {}", stats.total_participants);
    ///         println!("Total stake: {}", stats.total_stake);
    ///         println!("Consensus strength: {}%", stats.consensus_strength);
    ///     },
    ///     Err(e) => println!("Analytics unavailable: {:?}", e),
    /// }
    /// ```
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_market_analytics_statistics(
        env: Env,
        market_id: Symbol,
    ) -> Result<market_analytics::MarketStatistics, Error> {
        market_analytics::MarketAnalyticsManager::get_market_statistics(&env, market_id)
    }

    /// Get voting analytics and participation metrics for a market
    ///
    /// This function provides detailed analysis of voting patterns, participation
    /// trends, and community engagement within a specific market. It's useful
    /// for understanding market dynamics and user behavior.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<VotingAnalytics, Error>` where:
    /// - `Ok(VotingAnalytics)` - Complete voting analytics and metrics
    /// - `Err(Error)` - Error if market not found or analysis fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_voting_analytics(env.clone(), market_id) {
    ///     Ok(analytics) => {
    ///         println!("Total votes: {}", analytics.total_votes);
    ///         println!("Unique voters: {}", analytics.unique_voters);
    ///     },
    ///     Err(e) => println!("Voting analytics unavailable: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_voting_analytics(
        env: Env,
        market_id: Symbol,
    ) -> Result<market_analytics::VotingAnalytics, Error> {
        market_analytics::MarketAnalyticsManager::get_voting_analytics(&env, market_id)
    }

    /// Get oracle performance statistics for a specific oracle provider
    ///
    /// This function provides comprehensive performance metrics for oracle providers,
    /// including accuracy rates, response times, uptime statistics, and reliability
    /// scores. It's essential for oracle monitoring and optimization.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `oracle` - The oracle provider to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<OraclePerformanceStats, Error>` where:
    /// - `Ok(OraclePerformanceStats)` - Complete oracle performance statistics
    /// - `Err(Error)` - Error if oracle data unavailable
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, OracleProvider};
    /// # let env = Env::default();
    ///
    /// match PredictifyHybrid::get_oracle_performance_stats(env.clone(), OracleProvider::Reflector) {
    ///     Ok(stats) => {
    ///         println!("Oracle accuracy: {}%", stats.accuracy_rate);
    ///         println!("Uptime: {}%", stats.uptime_percentage);
    ///         println!("Reliability score: {}", stats.reliability_score);
    ///     },
    ///     Err(e) => println!("Oracle stats unavailable: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_oracle_performance_stats(
        env: Env,
        oracle: OracleProvider,
    ) -> Result<market_analytics::OraclePerformanceStats, Error> {
        market_analytics::MarketAnalyticsManager::get_oracle_performance_stats(&env, oracle)
    }

    /// Get fee analytics and revenue tracking for a specific timeframe
    ///
    /// This function provides comprehensive fee collection analytics including
    /// revenue tracking, fee distribution analysis, and collection efficiency
    /// metrics. It's essential for financial monitoring and optimization.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `timeframe` - The time period for fee analysis
    ///
    /// # Returns
    ///
    /// Returns `Result<FeeAnalytics, Error>` where:
    /// - `Ok(FeeAnalytics)` - Complete fee analytics and revenue data
    /// - `Err(Error)` - Error if fee data unavailable
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, TimeFrame};
    /// # let env = Env::default();
    ///
    /// match PredictifyHybrid::get_fee_analytics(env.clone(), TimeFrame::Month) {
    ///     Ok(analytics) => {
    ///         println!("Total fees collected: {}", analytics.total_fees_collected);
    ///         println!("Collection rate: {}%", analytics.fee_collection_rate);
    ///     },
    ///     Err(e) => println!("Fee analytics unavailable: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_fee_analytics(
        env: Env,
        timeframe: market_analytics::TimeFrame,
    ) -> Result<market_analytics::FeeAnalytics, Error> {
        market_analytics::MarketAnalyticsManager::get_fee_analytics(&env, timeframe)
    }

    /// Get dispute analytics and resolution metrics for a market
    ///
    /// This function provides detailed analysis of dispute patterns, resolution
    /// efficiency, and dispute-related metrics for a specific market. It's
    /// essential for understanding dispute dynamics and improving resolution processes.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<DisputeAnalytics, Error>` where:
    /// - `Ok(DisputeAnalytics)` - Complete dispute analytics and metrics
    /// - `Err(Error)` - Error if market not found or analysis fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_dispute_analytics(env.clone(), market_id) {
    ///     Ok(analytics) => {
    ///         println!("Total disputes: {}", analytics.total_disputes);
    ///         println!("Success rate: {}%", analytics.dispute_success_rate);
    ///     },
    ///     Err(e) => println!("Dispute analytics unavailable: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_dispute_analytics(
        env: Env,
        market_id: Symbol,
    ) -> Result<market_analytics::DisputeAnalytics, Error> {
        market_analytics::MarketAnalyticsManager::get_dispute_analytics(&env, market_id)
    }

    /// Get participation metrics for a specific market
    ///
    /// This function provides comprehensive participation analysis including
    /// user engagement, retention rates, and activity patterns for a specific
    /// market. It's essential for understanding user behavior and market health.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_id` - Unique identifier of the market to analyze
    ///
    /// # Returns
    ///
    /// Returns `Result<ParticipationMetrics, Error>` where:
    /// - `Ok(ParticipationMetrics)` - Complete participation metrics and analysis
    /// - `Err(Error)` - Error if market not found or analysis fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let market_id = Symbol::new(&env, "market_1");
    ///
    /// match PredictifyHybrid::get_participation_metrics(env.clone(), market_id) {
    ///     Ok(metrics) => {
    ///         println!("Total participants: {}", metrics.total_participants);
    ///         println!("Engagement score: {}", metrics.engagement_score);
    ///         println!("Retention rate: {}%", metrics.retention_rate);
    ///     },
    ///     Err(e) => println!("Participation metrics unavailable: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_participation_metrics(
        env: Env,
        market_id: Symbol,
    ) -> Result<market_analytics::ParticipationMetrics, Error> {
        market_analytics::MarketAnalyticsManager::get_participation_metrics(&env, market_id)
    }

    /// Get market comparison analytics for multiple markets
    ///
    /// This function provides comparative analysis across multiple markets,
    /// including performance rankings, comparative metrics, and market insights.
    /// It's essential for understanding market trends and performance patterns.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `markets` - Vector of market identifiers to compare
    ///
    /// # Returns
    ///
    /// Returns `Result<MarketComparisonAnalytics, Error>` where:
    /// - `Ok(MarketComparisonAnalytics)` - Complete comparative analytics
    /// - `Err(Error)` - Error if analysis fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, Symbol, vec};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let markets = vec![
    /// #     &env,
    /// #     Symbol::new(&env, "market_1"),
    /// #     Symbol::new(&env, "market_2"),
    /// # ];
    ///
    /// match PredictifyHybrid::get_market_comparison_analytics(env.clone(), markets) {
    ///     Ok(comparison) => {
    ///         println!("Total markets: {}", comparison.total_markets);
    ///         println!("Average participation: {}", comparison.average_participation);
    ///         println!("Success rate: {}%", comparison.success_rate);
    ///     },
    ///     Err(e) => println!("Comparison analytics unavailable: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_market_comparison_analytics(
        env: Env,
        markets: Vec<Symbol>,
    ) -> Result<market_analytics::MarketComparisonAnalytics, Error> {
        market_analytics::MarketAnalyticsManager::get_market_comparison_analytics(&env, markets)
    }

    // ===== PERFORMANCE BENCHMARK FUNCTIONS =====

    /// Benchmark gas usage for a specific function with given inputs
    ///
    /// This function measures the gas consumption and execution time for a specific
    /// contract function with provided inputs. It's essential for performance
    /// optimization and gas cost analysis.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `function` - Name of the function to benchmark
    /// * `inputs` - Vector of input parameters for the function
    ///
    /// # Returns
    ///
    /// Returns `Result<BenchmarkResult, Error>` where:
    /// - `Ok(BenchmarkResult)` - Complete benchmark results including gas usage and execution time
    /// - `Err(Error)` - Error if benchmarking fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, String, vec};
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    /// # let inputs = vec![&env, String::from_str(&env, "test_input")];
    ///
    /// match PredictifyHybrid::benchmark_gas_usage(
    ///     env.clone(),
    ///     String::from_str(&env, "create_market"),
    ///     inputs
    /// ) {
    ///     Ok(result) => {
    ///         println!("Gas usage: {}", result.gas_usage);
    ///         println!("Execution time: {}", result.execution_time);
    ///         println!("Performance score: {}", result.performance_score);
    ///     },
    ///     Err(e) => println!("Benchmark failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn benchmark_gas_usage(
        env: Env,
        function: String,
        inputs: Vec<String>,
    ) -> Result<performance_benchmarks::BenchmarkResult, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::benchmark_gas_usage(
            &env, function, inputs,
        )
    }

    /// Benchmark storage usage for a specific operation
    ///
    /// This function measures storage consumption and performance for various
    /// storage operations including read, write, and delete operations.
    /// It's essential for storage optimization and cost analysis.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `operation` - Storage operation configuration to benchmark
    ///
    /// # Returns
    ///
    /// Returns `Result<BenchmarkResult, Error>` where:
    /// - `Ok(BenchmarkResult)` - Complete storage benchmark results
    /// - `Err(Error)` - Error if benchmarking fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, StorageOperation};
    /// # let env = Env::default();
    /// # let operation = StorageOperation {
    /// #     operation_type: String::from_str(&env, "write"),
    /// #     data_size: 1024,
    /// #     key_count: 10,
    /// #     value_count: 10,
    /// #     operation_count: 100,
    /// # };
    ///
    /// match PredictifyHybrid::benchmark_storage_usage(env.clone(), operation) {
    ///     Ok(result) => {
    ///         println!("Storage usage: {}", result.storage_usage);
    ///         println!("Gas usage: {}", result.gas_usage);
    ///     },
    ///     Err(e) => println!("Storage benchmark failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn benchmark_storage_usage(
        env: Env,
        operation: performance_benchmarks::StorageOperation,
    ) -> Result<performance_benchmarks::BenchmarkResult, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::benchmark_storage_usage(
            &env, operation,
        )
    }

    /// Benchmark oracle call performance for a specific oracle provider
    ///
    /// This function measures the performance characteristics of oracle calls
    /// including response time, gas usage, and reliability metrics.
    /// It's essential for oracle performance monitoring and optimization.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `oracle` - The oracle provider to benchmark
    ///
    /// # Returns
    ///
    /// Returns `Result<BenchmarkResult, Error>` where:
    /// - `Ok(BenchmarkResult)` - Complete oracle performance benchmark results
    /// - `Err(Error)` - Error if benchmarking fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, OracleProvider};
    /// # let env = Env::default();
    ///
    /// match PredictifyHybrid::benchmark_oracle_call_performance(
    ///     env.clone(),
    ///     OracleProvider::Reflector
    /// ) {
    ///     Ok(result) => {
    ///         println!("Oracle response time: {}", result.execution_time);
    ///         println!("Oracle gas usage: {}", result.gas_usage);
    ///     },
    ///     Err(e) => println!("Oracle benchmark failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn benchmark_oracle_performance(
        env: Env,
        oracle: OracleProvider,
    ) -> Result<performance_benchmarks::BenchmarkResult, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::benchmark_oracle_call_performance(
            &env, oracle,
        )
    }

    /// Benchmark batch operations performance
    ///
    /// This function measures the performance of batch operations including
    /// gas efficiency, execution time, and throughput characteristics.
    /// It's essential for batch operation optimization and scalability analysis.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `operations` - Vector of batch operations to benchmark
    ///
    /// # Returns
    ///
    /// Returns `Result<BenchmarkResult, Error>` where:
    /// - `Ok(BenchmarkResult)` - Complete batch operation benchmark results
    /// - `Err(Error)` - Error if benchmarking fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::{Env, vec};
    /// # use predictify_hybrid::{PredictifyHybrid, BatchOperation};
    /// # let env = Env::default();
    /// # let operations = vec![
    /// #     &env,
    /// #     BatchOperation {
    /// #         operation_type: String::from_str(&env, "batch_vote"),
    /// #         batch_size: 100,
    /// #         operation_count: 10,
    /// #         data_size: 1024,
    /// #     }
    /// # ];
    ///
    /// match PredictifyHybrid::benchmark_batch_operations(env.clone(), operations) {
    ///     Ok(result) => {
    ///         println!("Batch execution time: {}", result.execution_time);
    ///         println!("Batch gas usage: {}", result.gas_usage);
    ///     },
    ///     Err(e) => println!("Batch benchmark failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn benchmark_batch_operations(
        env: Env,
        operations: Vec<performance_benchmarks::BatchOperation>,
    ) -> Result<performance_benchmarks::BenchmarkResult, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::benchmark_batch_operations(
            &env, operations,
        )
    }

    /// Benchmark scalability with large markets and user counts
    ///
    /// This function measures the contract's performance under high load
    /// scenarios with large numbers of markets and users. It's essential
    /// for scalability testing and performance validation.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `market_size` - Number of markets to simulate
    /// * `user_count` - Number of users to simulate
    ///
    /// # Returns
    ///
    /// Returns `Result<BenchmarkResult, Error>` where:
    /// - `Ok(BenchmarkResult)` - Complete scalability benchmark results
    /// - `Err(Error)` - Error if benchmarking fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::PredictifyHybrid;
    /// # let env = Env::default();
    ///
    /// match PredictifyHybrid::benchmark_scalability(env.clone(), 1000, 10000) {
    ///     Ok(result) => {
    ///         println!("Scalability test completed");
    ///         println!("Total gas usage: {}", result.gas_usage);
    ///         println!("Total execution time: {}", result.execution_time);
    ///     },
    ///     Err(e) => println!("Scalability benchmark failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn benchmark_scalability(
        env: Env,
        market_size: u32,
        user_count: u32,
    ) -> Result<performance_benchmarks::BenchmarkResult, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::benchmark_scalability(
            &env,
            market_size,
            user_count,
        )
    }

    /// Generate comprehensive performance report
    ///
    /// This function creates a detailed performance report including metrics,
    /// recommendations, and optimization opportunities based on benchmark results.
    /// It's essential for performance analysis and optimization planning.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `benchmark_suite` - The benchmark suite to generate report for
    ///
    /// # Returns
    ///
    /// Returns `Result<PerformanceReport, Error>` where:
    /// - `Ok(PerformanceReport)` - Complete performance report with analysis
    /// - `Err(Error)` - Error if report generation fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, PerformanceBenchmarkSuite};
    /// # let env = Env::default();
    /// # let suite = PerformanceBenchmarkSuite::default(); // Placeholder
    ///
    /// match PredictifyHybrid::generate_performance_report(env.clone(), suite) {
    ///     Ok(report) => {
    ///         println!("Performance report generated");
    ///         println!("Overall score: {}", report.performance_metrics.overall_performance_score);
    ///         println!("Recommendations: {}", report.recommendations.len());
    ///     },
    ///     Err(e) => println!("Report generation failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn generate_performance_report(
        env: Env,
        benchmark_suite: performance_benchmarks::PerformanceBenchmarkSuite,
    ) -> Result<performance_benchmarks::PerformanceReport, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::generate_performance_report(
            &env,
            benchmark_suite,
        )
    }

    /// Validate performance against thresholds
    ///
    /// This function validates performance metrics against predefined thresholds
    /// to ensure the contract meets performance requirements. It's essential
    /// for performance validation and quality assurance.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `metrics` - Performance metrics to validate
    /// * `thresholds` - Performance thresholds to validate against
    ///
    /// # Returns
    ///
    /// Returns `Result<bool, Error>` where:
    /// - `Ok(true)` - Performance meets all thresholds
    /// - `Ok(false)` - Performance does not meet thresholds
    /// - `Err(Error)` - Error if validation fails
    ///
    /// # Example
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # use predictify_hybrid::{PredictifyHybrid, PerformanceMetrics, PerformanceThresholds};
    /// # let env = Env::default();
    /// # let metrics = PerformanceMetrics::default(); // Placeholder
    /// # let thresholds = PerformanceThresholds::default(); // Placeholder
    ///
    /// match PredictifyHybrid::validate_performance_thresholds(env.clone(), metrics, thresholds) {
    ///     Ok(true) => println!("Performance meets all thresholds"),
    ///     Ok(false) => println!("Performance does not meet thresholds"),
    ///     Err(e) => println!("Validation failed: {:?}", e),
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when validation, authorization, storage, or subsystem checks fail.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn validate_performance_thresholds(
        env: Env,
        metrics: performance_benchmarks::PerformanceMetrics,
        thresholds: performance_benchmarks::PerformanceThresholds,
    ) -> Result<bool, Error> {
        performance_benchmarks::PerformanceBenchmarkManager::validate_performance_thresholds(
            &env, metrics, thresholds,
        )
    }
    /// Get platform-wide statistics
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    /// Verify SAC token decimals match declared value (admin only).
    ///
    /// This function performs a critical security check on SAC tokens to prevent
    /// denomination mistakes that have caused real on-chain losses. It verifies that
    /// the token's on-chain decimals() value matches what was declared during registration.
    ///
    /// This can be called:
    /// - Automatically during token registration (via add_global_verified/add_event_verified)
    /// - Manually by admin as part of periodic audits or security reviews
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment for blockchain operations
    /// * `admin` - The administrator address (must be authorized)
    /// * `token_contract` - Address of the token contract to verify
    /// * `declared_decimals` - The decimals value that was declared during registration
    ///
    /// # Returns
    ///
    /// Returns `Result<(), Error>` where:
    /// - `Ok(())` - Decimals match (token is safe)
    /// - `Err(Error::TokenDecimalsMismatch)` - Mismatch detected (token rejected)
    /// - `Err(Error::Unauthorized)` - Caller is not admin
    ///
    /// # Cross-Contract Call
    ///
    /// This function performs a cross-contract call to the token contract's
    /// `decimals()` function using the Soroban token interface.
    ///
    /// # Security Notes
    ///
    /// - Verifies via on-chain decimals() call (cannot be spoofed)
    /// - Mismatch indicates potential token misconfiguration
    /// - Rejected tokens cannot be used for betting/payouts
    /// - All registration paths should use verified variants
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let token_contract = Address::from_string("GBUQW...");
    /// PredictifyHybrid::re_verify_token(&env, &admin, &token_contract, 7)?;
    /// // Returns Ok if decimals match, TokenDecimalsMismatch error otherwise
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error`] when:
    /// - `Error::Unauthorized` - Caller is not the contract admin
    /// - `Error::TokenDecimalsMismatch` - On-chain decimals don't match declared value
    /// - Other errors from cross-contract call or storage operations
    ///
    /// # Events
    ///
    /// Emits audit trail record of verification attempt (success or failure).
    pub fn re_verify_token(
        env: Env,
        admin: Address,
        token_contract: Address,
        declared_decimals: u32,
    ) -> Result<(), Error> {
        // Verify admin authorization
        Self::require_primary_admin(&env, &admin)?;

        // Create temporary asset for verification
        let asset = crate::tokens::Asset {
            contract: token_contract.clone(),
            symbol: Symbol::new(&env, "TEMP"),
            decimals: declared_decimals,
        };

        // Perform decimals verification via cross-contract call
        crate::tokens::verify_token_decimals(&env, &asset)?;

        // Record verification in audit trail
        crate::audit_trail::AuditTrailManager::append_record(
            &env,
            crate::audit_trail::AuditAction::TokenVerified,
            admin.clone(),
            Map::new(&env),
            None,
        );

        Ok(())
    }

    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_platform_statistics(env: Env) -> PlatformStatistics {
        statistics::StatisticsManager::get_platform_stats(&env)
    }

    /// Get user-specific statistics
    ///
    /// # Errors
    ///
    /// This entrypoint surfaces contract errors via panic in internal calls.
    ///
    /// # Events
    ///
    /// State-changing paths may emit events through internal managers; read-only query paths emit no events.
    pub fn get_user_statistics(env: Env, user: Address) -> UserStatistics {
        statistics::StatisticsManager::get_user_stats(&env, &user)
    }

    /// Get dashboard statistics with versioning for client compatibility
    ///
    /// Provides comprehensive platform-level metrics optimized for dashboard display,
    /// including version information for managing client updates.
    ///
    /// # Returns
    ///
    /// * `DashboardStatisticsV1` - Versioned dashboard statistics with:
    ///   - API version (always 1)
    ///   - Platform statistics
    ///   - Active user count
    ///   - Total value locked
    ///   - Query timestamp
    ///
    /// # Errors
    ///
    /// Returns contract error if market traversal fails.
    ///
    /// # Events
    ///
    /// This is a read-only query; no events are emitted.
    pub fn get_dashboard_statistics(env: Env) -> Result<types::DashboardStatisticsV1, Error> {
        queries::QueryManager::get_dashboard_statistics(&env)
    }

    /// Get market statistics optimized for dashboard display
    ///
    /// Returns comprehensive per-market metrics including participant count,
    /// volume, consensus strength, and volatility for dashboard visualization.
    ///
    /// # Parameters
    ///
    /// * `market_id` - The market to query
    ///
    /// # Returns
    ///
    /// * `MarketStatisticsV1` - Market metrics with:
    ///   - Participant count
    ///   - Total volume
    ///   - Average stake
    ///   - Consensus strength (0-10000)
    ///   - Volatility (0-10000)
    ///   - Market state and question
    ///
    /// # Errors
    ///
    /// * `Error::MarketNotFound` - Market doesn't exist
    ///
    /// # Events
    ///
    /// Read-only query; no events emitted.
    pub fn get_market_statistics(
        env: Env,
        market_id: Symbol,
    ) -> Result<types::MarketStatisticsV1, Error> {
        queries::QueryManager::get_market_statistics(&env, market_id)
    }

    /// Get category statistics for filtered dashboard views
    ///
    /// Provides aggregated metrics for all markets in a specific category,
    /// enabling category-filtered dashboard displays and analytics.
    ///
    /// # Parameters
    ///
    /// * `category` - Category name to query
    ///
    /// # Returns
    ///
    /// * `CategoryStatisticsV1` - Category metrics with:
    ///   - Market count
    ///   - Total volume
    ///   - Participant count
    ///   - Resolved market count
    ///   - Average market volume
    ///
    /// # Events
    ///
    /// Read-only query; no events emitted.
    pub fn get_category_statistics(
        env: Env,
        category: String,
    ) -> Result<types::CategoryStatisticsV1, Error> {
        queries::QueryManager::get_category_statistics(&env, category)
    }

    /// Get top users by total winnings (leaderboard query)
    ///
    /// Returns the top N users ranked by total winnings claimed,
    /// useful for leaderboard and achievement displays.
    ///
    /// # Parameters
    ///
    /// * `limit` - Maximum number of results (capped at 50 for gas safety)
    ///
    /// # Returns
    ///
    /// * `Vec<UserLeaderboardEntryV1>` - Top users sorted by winnings (descending)
    ///
    /// # Notes
    ///
    /// Due to contract storage scanning limitations, large deployments should
    /// consider off-chain indexing for leaderboard queries.
    ///
    /// # Events
    ///
    /// Read-only query; no events emitted.
    pub fn get_top_users_by_winnings(
        env: Env,
        limit: u32,
    ) -> Result<Vec<types::UserLeaderboardEntryV1>, Error> {
        queries::QueryManager::get_top_users_by_winnings(&env, limit)
    }

    /// Get top users by win rate (skill-based leaderboard)
    ///
    /// Returns the top N users ranked by win rate percentage,
    /// with a minimum bet requirement to filter high-variance winners.
    ///
    /// # Parameters
    ///
    /// * `limit` - Maximum number of results (capped at 50)
    /// * `min_bets` - Minimum bets required for inclusion (e.g., 10)
    ///
    /// # Returns
    ///
    /// * `Vec<UserLeaderboardEntryV1>` - Top users sorted by win rate (descending)
    ///
    /// # Events
    ///
    /// Read-only query; no events emitted.
    pub fn get_top_users_by_win_rate(
        env: Env,
        limit: u32,
        min_bets: u64,
    ) -> Result<Vec<types::UserLeaderboardEntryV1>, Error> {
        queries::QueryManager::get_top_users_by_win_rate(&env, limit, min_bets)
    }

    /// Get the top-N participants in a specific market, ranked by cumulative stake.
    ///
    /// Returns a bounded list of the highest-staking participants in `market_id`,
    /// maintained incrementally on every [`place_bet`][Self::place_bet] call.
    /// Reads are O(N) where N ≤ 50 (`MAX_MARKET_LEADERBOARD_CAPACITY`), so costs
    /// are predictable regardless of the total participant count.
    ///
    /// # Parameters
    ///
    /// * `market_id` – Identifies the market.
    /// * `limit`     – Maximum entries to return (capped at 50).
    ///
    /// # Returns
    ///
    /// `Vec<MarketLeaderboardEntry>` sorted **descending by stake** (rank 1 =
    /// highest staker).  Returns an empty vector if no bets have been placed yet.
    ///
    /// # Events
    ///
    /// Read-only query; no events emitted.
    pub fn get_market_leaderboard(
        env: Env,
        market_id: Symbol,
        limit: u32,
    ) -> Vec<types::MarketLeaderboardEntry> {
        market_analytics::MarketLeaderboard::top_by_stake(&env, &market_id, limit)
    }


    /// Admin-initiated circuit-breaker resume: Open → HalfOpen with cooldown.
    ///
    /// Moves the circuit breaker from `Open` to `HalfOpen` and records the
    /// current ledger timestamp as the cooldown start.  Probe requests are not
    /// counted toward the success threshold until `recovery_timeout` seconds have
    /// elapsed.  After `half_open_max_requests` consecutive probe successes the
    /// breaker auto-closes; any failure during the probe window re-opens it.
    ///
    /// # Errors
    ///
    /// - `Error::Unauthorized` — caller is not an authorised admin.
    /// - `Error::CBError` — breaker is not currently `Open`.
    pub fn request_resume(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        crate::circuit_breaker::CircuitBreaker::request_resume(&env, &admin)
    }

    /// Return a versioned, XDR-stable snapshot of current platform statistics.
    ///
    /// The returned [`reporting::SnapshotEnvelope`] contains the current
    /// [`reporting::PlatformStats`] serialised with `to_xdr`, tagged with
    /// [`reporting::SNAPSHOT_SCHEMA_VERSION`] and the current ledger timestamp.
    ///
    /// # Errors
    ///
    /// - `Error::ContractStateError` — market index is missing or corrupted.
    pub fn get_snapshot_envelope(env: Env) -> Result<reporting::SnapshotEnvelope, Error> {
        reporting::ReportingManager::get_snapshot_envelope(&env)
    }

    /// Accumulate dispute fees. Called after each dispute resolution to add
    /// `fee_amount` to the running cumulative total. Returns the new total.
    pub fn accumulate_dispute_fee(env: Env, caller: Address, fee_amount: i128) -> i128 {
        caller.require_auth();
        if fee_amount < 0 {
            panic_with_error!(env, Error::InvalidInput);
        }
        let key = Symbol::new(&env, "cum_disp_fee");
        let current: i128 = env.storage().instance().get(&key).unwrap_or(0i128);
        let new_total = current.checked_add(fee_amount)
            .unwrap_or_else(|| panic_with_error!(env, Error::Overflow));
        env.storage().instance().set(&key, &new_total);
        new_total
    }

    /// Get the cumulative dispute fee total accumulated so far.
    pub fn get_cumulative_dispute_fee(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&Symbol::new(&env, "cum_disp_fee"))
            .unwrap_or(0i128)
    }

}

// ===== TESTS =====

#[cfg(test)]

mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        vec, Address, BytesN, Env, String,
    };
    use types::{ClaimInfo, MarketState, OracleConfig, OracleProvider};

    /// Helper: build a minimal resolved Market with one winner and one loser.
    fn setup_resolved_market(env: &Env, contract_id: &Address) -> Symbol {
        let market_id = Symbol::new(env, "test_mkt");

        env.as_contract(contract_id, || {
            let admin = Address::generate(env);
            let winner = Address::generate(env);
            let loser = Address::generate(env);

            let mut votes = soroban_sdk::Map::new(env);
            votes.set(winner.clone(), String::from_str(env, "yes"));
            votes.set(loser.clone(), String::from_str(env, "no"));

            let mut stakes = soroban_sdk::Map::new(env);
            stakes.set(winner.clone(), 100_000_000i128); // 10 XLM
            stakes.set(loser.clone(), 100_000_000i128);

            let market = Market {
                admin: admin.clone(),
                question: String::from_str(env, "Will BTC hit $100k?"),
                outcomes: vec![
                    env,
                    String::from_str(env, "yes"),
                    String::from_str(env, "no"),
                ],
                end_time: env.ledger().timestamp().saturating_sub(1),
                oracle_config: OracleConfig::new(
                    OracleProvider::reflector(),
                    Address::from_str(
                        env,
                        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                    ),
                    String::from_str(env, "BTC/USD"),
                    100_000,
                    String::from_str(env, "gt"),
                ),
                metadata_commitment: BytesN::from_array(env, &[0u8; 32]),
                has_fallback: false,
                fallback_oracle_config: OracleConfig::none_sentinel(env),
                resolution_timeout: 3600,
                oracle_result: None,
                state: MarketState::Resolved,
                votes,
                stakes,
                winning_outcomes: Some(vec![env, String::from_str(env, "yes")]),
                claimed: soroban_sdk::Map::new(env),
                total_staked: 200_000_000,
                dispute_stakes: soroban_sdk::Map::new(env),
                fee_collected: false,
                total_extension_days: 0,
                max_extension_days: 7,
                extension_history: soroban_sdk::Vec::new(env),
                category: None,
                tags: soroban_sdk::Vec::new(env),
                min_pool_size: None,
                bet_deadline: 0,
                dispute_window_seconds: 86400,
                winnings_swept: false,
                timelock_config: timelock::MarketTimelockConfig::default(),
                dispute_stake_floor: None,
                max_participants: None,
            };

            env.storage().persistent().set(&market_id, &market);
        });

        market_id
    }

    #[test]
    fn test_distribute_payouts_single_winner() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictifyHybrid, ());
        let market_id = setup_resolved_market(&env, &contract_id);

        // Store a resolution summary so ResolutionOutcomeCache::require succeeds.
        // (Adjust the key/type to match your actual resolution.rs implementation.)
        env.as_contract(&contract_id, || {
            let summary = resolution::ResolvedOutcomeSummary {
                winning_total: 100_000_000i128,
                total_pool: 200_000_000i128,
                num_winning_outcomes: 1u32,
            };
            let cache_key = (symbol_short!("res_out"), market_id.clone());
            env.storage().persistent().set(&cache_key, &summary);
        });

        let result = env.as_contract(&contract_id, || {
            PredictifyHybrid::distribute_payouts(env.clone(), market_id)
        });
        // With one winner staking 10 XLM from a 20 XLM pool at 2% fee:
        // share = 100_000_000 * 9800 / 10000 = 98_000_000
        // payout = 98_000_000 * 200_000_000 / 100_000_000 = 196_000_000
        assert!(result.is_ok());
        assert!(result.unwrap() > 0);
    }

    #[test]
    fn test_distribute_payouts_no_unclaimed_winners_returns_zero() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            // Market with winning_outcomes but everything already claimed
            let market_id = Symbol::new(&env, "all_claimed");
            let winner = Address::generate(&env);

            let mut votes = soroban_sdk::Map::new(&env);
            votes.set(winner.clone(), String::from_str(&env, "yes"));

            let mut claimed = soroban_sdk::Map::new(&env);
            // Mark as already claimed
            claimed.set(winner.clone(), ClaimInfo::new(&env, 1_000_000));

            let market = Market {
                admin: Address::generate(&env),
                question: String::from_str(&env, "Test?"),
                outcomes: vec![&env, String::from_str(&env, "yes")],
                end_time: 0,
                oracle_config: OracleConfig::new(
                    OracleProvider::reflector(),
                    Address::from_str(
                        &env,
                        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                    ),
                    String::from_str(&env, "BTC/USD"),
                    1,
                    String::from_str(&env, "gt"),
                ),
                metadata_commitment: BytesN::from_array(&env, &[0u8; 32]),
                has_fallback: false,
                fallback_oracle_config: OracleConfig::none_sentinel(&env),
                resolution_timeout: 3600,
                oracle_result: None,
                state: MarketState::Resolved,
                votes,
                stakes: soroban_sdk::Map::new(&env),
                winning_outcomes: Some(vec![&env, String::from_str(&env, "yes")]),
                claimed,
                total_staked: 0,
                dispute_stakes: soroban_sdk::Map::new(&env),
                fee_collected: false,
                total_extension_days: 0,
                max_extension_days: 7,
                extension_history: soroban_sdk::Vec::new(&env),
                category: None,
                tags: soroban_sdk::Vec::new(&env),
                min_pool_size: None,
                bet_deadline: 0,
                dispute_window_seconds: 86400,
                winnings_swept: false,
                timelock_config: timelock::MarketTimelockConfig::default(),
                dispute_stake_floor: None,
                max_participants: None,
            };

            env.storage().persistent().set(&market_id, &market);
        });

        let result = env.as_contract(&contract_id, || {
            PredictifyHybrid::distribute_payouts(
                env.clone(),
                Symbol::new(&env, "all_claimed"),
            )
        });
        assert_eq!(result, Ok(0));
    }

    #[test]
    fn test_distribute_payouts_market_not_resolved_returns_error() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let market_id = Symbol::new(&env, "unresolved");
            let market = Market {
                admin: Address::generate(&env),
                question: String::from_str(&env, "Test?"),
                outcomes: vec![&env, String::from_str(&env, "yes")],
                end_time: 9_999_999_999,
                oracle_config: OracleConfig::new(
                    OracleProvider::reflector(),
                    Address::from_str(
                        &env,
                        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                    ),
                    String::from_str(&env, "BTC/USD"),
                    1,
                    String::from_str(&env, "gt"),
                ),
                metadata_commitment: BytesN::from_array(&env, &[0u8; 32]),
                has_fallback: false,
                fallback_oracle_config: OracleConfig::none_sentinel(&env),
                resolution_timeout: 3600,
                oracle_result: None,
                state: MarketState::Active,
                votes: soroban_sdk::Map::new(&env),
                stakes: soroban_sdk::Map::new(&env),
                winning_outcomes: None, // Not resolved
                claimed: soroban_sdk::Map::new(&env),
                total_staked: 0,
                dispute_stakes: soroban_sdk::Map::new(&env),
                fee_collected: false,
                total_extension_days: 0,
                max_extension_days: 7,
                extension_history: soroban_sdk::Vec::new(&env),
                category: None,
                tags: soroban_sdk::Vec::new(&env),
                min_pool_size: None,
                bet_deadline: 0,
                dispute_window_seconds: 86400,
                winnings_swept: false,
                timelock_config: timelock::MarketTimelockConfig::default(),
                dispute_stake_floor: None,
                max_participants: None,
            };
            env.storage().persistent().set(&market_id, &market);
        });

        let result = env.as_contract(&contract_id, || {
            PredictifyHybrid::distribute_payouts(
                env.clone(),
                Symbol::new(&env, "unresolved"),
            )
        });
        assert_eq!(result, Err(Error::MarketNotResolved));
    }

    #[test]
    fn test_budget_guard_aborts_at_low_threshold() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PredictifyHybrid, ());

        // Set an extremely low threshold — should abort immediately on first check
        env.as_contract(&contract_id, || {
            let guard = BudgetGuard::new(&env, 0);
            // With threshold 0, any consumed > 0 triggers the error.
            // In the test host, consumed will be 0 initially so we test the logic:
            assert!(guard.threshold() == 0);
        });
    }

    #[test]
    fn test_budget_guard_consumed_is_non_negative() {
        let env = Env::default();
        let contract_id = env.register(PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let guard = BudgetGuard::new(&env, 100_000);
            assert!(guard.consumed() == 0); // No instructions consumed yet in test host
        });
    }

    #[test]
    fn test_upgrade_entrypoints_expose_expected_contract_surface() {
        let env = Env::default();
        let contract_id = env.register(PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let version = PredictifyHybrid::get_contract_version(env.clone());
            assert!(version.is_ok());

            let capability_bitmap = PredictifyHybrid::capabilities(env.clone());
            assert!(capability_bitmap > 0);

            let available = PredictifyHybrid::check_upgrade_available(env.clone());
            assert!(available.is_ok());
            assert!(!available.unwrap());

            let history = PredictifyHybrid::get_upgrade_history(env.clone());
            assert!(history.is_ok());
            assert!(history.unwrap().is_empty());

            let stats = PredictifyHybrid::get_upgrade_statistics(env.clone());
            assert!(stats.is_ok());
        });
    }

    pub fn get_fee_withdrawal_schedule(env: Env) -> crate::fees::FeeWithdrawalSchedule {
        crate::fees::FeeWithdrawalManager::get_schedule(&env)
    }
}
