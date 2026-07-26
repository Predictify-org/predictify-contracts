#![no_std]

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

extern crate alloc;

use soroban_sdk::{
    contract, contractimpl, panic_with_error, symbol_short, Address, BytesN, Map, String, Symbol, Vec,
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
mod events;
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
mod cross_oracle_staleness_tests;
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
mod extensions;
mod graceful_degradation;
mod market_id_generator;
mod metadata_limits;
mod queries;
mod recovery;
mod statistics;
mod tokens;
mod rate_limiter;
mod dispute_multisig;
mod event_topic_catalog;
mod storage_tier_audit;
mod leaderboard;
mod lists;
mod audit_trail;
mod monitor;
mod capabilities;

#[cfg(test)]
mod override_audit_tests;
#[cfg(test)]
mod market_audit_tests;
#[cfg(test)]
mod test_audit_trail;

mod bandprotocol {
    soroban_sdk::contractimport!(file = "./std_reference.wasm");
}

pub mod timelock;

use bets::BetStorage;
use gas::BudgetGuard;
use resolution::ResolutionOutcomeCache;
use storage::BalanceStorage;
use types::{Market, ReflectorAsset};

#[cfg(test)]
mod market_state_matrix_tests;
#[cfg(test)]
mod upgrade_manager_tests;
#[cfg(test)]
mod oracle_lifecycle_events_tests;
mod timelock_tests;

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

use admin::{
    AdminAnalyticsResult, AdminFunctions, AdminInitializer, AdminManager, AdminPermission,
    AdminRole, AdminSystemIntegration,
};
pub use admin::Severity;
pub use err::Error;
use crate::storage::{
    check_market_creation_rent, check_market_creation_rent_budget, DataKey, MARKET_TTL_LEDGERS,
    MARKETS_BUMP_AMOUNT, MARKETS_LIFETIME_THRESHOLD,
};
pub mod errors {
    pub use crate::err::*;
}
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
        let stored_admin: Option<Address> =
            env.storage().persistent().get(&Symbol::new(env, SYM_ADMIN));
        if stored_admin.is_none() {
            return Err(Error::AdminNotSet);
        }
        Ok(())
    }
}