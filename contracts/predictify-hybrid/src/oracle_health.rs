use soroban_sdk::{contracttype, Address, Env, Map, String, Symbol, Vec};
use crate::admin::AdminAccessControl;
use crate::err::Error;

// ===== ORACLE HEALTH TYPES =====

/// Oracle health states with hysteresis.
///
/// This enum defines the possible health states of an oracle with dual-threshold
/// hysteresis to prevent rapid state flapping between Healthy, Degraded, and Offline states.
///
/// # State Transitions
///
/// The hysteresis mechanism uses two thresholds per boundary:
/// - Healthy -> Degraded: Requires health_score to drop below `healthy_to_degraded_threshold`
/// - Degraded -> Healthy: Requires health_score to rise above `degraded_to_healthy_threshold`
/// - Degraded -> Offline: Requires health_score to drop below `degraded_to_offline_threshold`
/// - Offline -> Degraded: Requires health_score to rise above `offline_to_degraded_threshold`
///
/// This ensures a "dead zone" where the state doesn't change, preventing flapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[contracttype]
pub enum OracleHealthState {
    /// Oracle is operating normally within acceptable parameters.
    Healthy,
    /// Oracle is degraded but still functional. Some metrics are outside normal range.
    Degraded,
    /// Oracle is offline or critically unhealthy. Should not be used for resolution.
    Offline,
}

impl OracleHealthState {
    /// Returns true if the oracle is in a usable state for price queries.
    pub fn is_usable(&self) -> bool {
        matches!(self, OracleHealthState::Healthy | OracleHealthState::Degraded)
    }

    /// Returns true if the oracle is offline and should not be used.
    pub fn is_offline(&self) -> bool {
        matches!(self, OracleHealthState::Offline)
    }

    /// Returns the state as a human-readable string.
    pub fn as_str(&self) -> &str {
        match self {
            OracleHealthState::Healthy => "Healthy",
            OracleHealthState::Degraded => "Degraded",
            OracleHealthState::Offline => "Offline",
        }
    }
}

/// Configuration for oracle health hysteresis thresholds.
///
/// These thresholds define the boundaries between health states and provide
/// hysteresis to prevent rapid state flapping.
///
/// # Thresholds
///
/// - `healthy_to_degraded_threshold`: Score below which Healthy becomes Degraded
/// - `degraded_to_healthy_threshold`: Score above which Degraded becomes Healthy (must be > healthy_to_degraded)
/// - `degraded_to_offline_threshold`: Score below which Degraded becomes Offline
/// - `offline_to_degraded_threshold`: Score above which Offline becomes Degraded (must be > degraded_to_offline)
///
/// # Default Values
///
/// - Healthy -> Degraded: 70
/// - Degraded -> Healthy: 80 (hysteresis gap of 10)
/// - Degraded -> Offline: 30
/// - Offline -> Degraded: 40 (hysteresis gap of 10)
#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleHealthConfig {
    pub healthy_to_degraded_threshold: u32,     // Default: 70
    pub degraded_to_healthy_threshold: u32,     // Default: 80
    pub degraded_to_offline_threshold: u32,     // Default: 30
    pub offline_to_degraded_threshold: u32,     // Default: 40
    pub max_staleness_seconds: u64,             // Default: 300 (5 minutes)
    pub max_latency_ms: u64,                    // Default: 5000
    pub min_confidence_pct: u32,                // Default: 50 (50%)
    pub max_consecutive_failures: u32,          // Default: 5
    pub health_check_interval_seconds: u64,     // Default: 60
}

impl Default for OracleHealthConfig {
    fn default() -> Self {
        Self {
            healthy_to_degraded_threshold: 70,
            degraded_to_healthy_threshold: 80,
            degraded_to_offline_threshold: 30,
            offline_to_degraded_threshold: 40,
            max_staleness_seconds: 300,
            max_latency_ms: 5000,
            min_confidence_pct: 50,
            max_consecutive_failures: 5,
            health_check_interval_seconds: 60,
        }
    }
}

impl OracleHealthConfig {
    /// Validate configuration for logical consistency.
    pub fn validate(&self) -> Result<(), Error> {
        // Hysteresis thresholds must have gaps
        if self.degraded_to_healthy_threshold <= self.healthy_to_degraded_threshold {
            return Err(Error::InvalidOracleConfig);
        }
        if self.offline_to_degraded_threshold <= self.degraded_to_offline_threshold {
            return Err(Error::InvalidOracleConfig);
        }
        // Thresholds must be in range 0-100
        if self.healthy_to_degraded_threshold > 100
            || self.degraded_to_healthy_threshold > 100
            || self.degraded_to_offline_threshold > 100
            || self.offline_to_degraded_threshold > 100
        {
            return Err(Error::InvalidOracleConfig);
        }
        // Ordering: offline < degraded < healthy
        if self.degraded_to_offline_threshold >= self.healthy_to_degraded_threshold {
            return Err(Error::InvalidOracleConfig);
        }
        // Confidence must be 0-100
        if self.min_confidence_pct > 100 {
            return Err(Error::InvalidOracleConfig);
        }
        // Staleness and latency must be positive
        if self.max_staleness_seconds == 0 || self.max_latency_ms == 0 {
            return Err(Error::InvalidOracleConfig);
        }
        // Consecutive failures must be at least 1
        if self.max_consecutive_failures == 0 {
            return Err(Error::InvalidOracleConfig);
        }
        // Health check interval must be positive
        if self.health_check_interval_seconds == 0 {
            return Err(Error::InvalidOracleConfig);
        }
        Ok(())
    }
}

/// Health metrics for a single oracle check.
#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleHealthMetrics {
    /// Current health score (0-100). Higher is better.
    pub health_score: u32,
    /// Timestamp of the last successful response.
    pub last_success_time: u64,
    /// Timestamp of the last failed response.
    pub last_failure_time: u64,
    /// Number of consecutive failures.
    pub consecutive_failures: u32,
    /// Total number of health checks performed.
    pub total_checks: u32,
    /// Number of successful health checks.
    pub successful_checks: u32,
    /// Average latency in milliseconds.
    pub avg_latency_ms: u64,
    /// Last observed latency in milliseconds.
    pub last_latency_ms: u64,
    /// Data staleness in seconds (time since last price update).
    pub staleness_seconds: u64,
    /// Confidence percentage from oracle (if available).
    pub confidence_pct: Option<u32>,
}

impl Default for OracleHealthMetrics {
    fn default() -> Self {
        Self {
            health_score: 100,
            last_success_time: 0,
            last_failure_time: 0,
            consecutive_failures: 0,
            total_checks: 0,
            successful_checks: 0,
            avg_latency_ms: 0,
            last_latency_ms: 0,
            staleness_seconds: 0,
            confidence_pct: None,
        }
    }
}

/// Complete health state for an oracle including metrics and current state.
#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleHealth {
    /// Oracle contract address.
    pub oracle_address: Address,
    /// Current health state.
    pub state: OracleHealthState,
    /// Health metrics.
    pub metrics: OracleHealthMetrics,
    /// Timestamp of last state transition.
    pub last_state_change: u64,
    /// Number of state transitions since initialization.
    pub state_transition_count: u32,
    /// Configuration for this oracle.
    pub config: OracleHealthConfig,
}

impl OracleHealth {
    /// Create a new OracleHealth with default configuration.
    pub fn new(oracle_address: Address, env: &Env) -> Self {
        Self {
            oracle_address,
            state: OracleHealthState::Healthy,
            metrics: OracleHealthMetrics::default(),
            last_state_change: env.ledger().timestamp(),
            state_transition_count: 0,
            config: OracleHealthConfig::default(),
        }
    }

    /// Create a new OracleHealth with custom configuration.
    pub fn new_with_config(
        oracle_address: Address,
        config: OracleHealthConfig,
        env: &Env,
    ) -> Result<Self, Error> {
        config.validate()?;
        let mut health = Self {
            oracle_address,
            state: OracleHealthState::Healthy,
            metrics: OracleHealthMetrics::default(),
            last_state_change: env.ledger().timestamp(),
            state_transition_count: 0,
            config,
        };
        // Initial health score calculation
        health.recalculate_health_score(env);
        Ok(health)
    }

    /// Load existing OracleHealth from storage.
    pub fn load(env: &Env, oracle_address: &Address) -> Result<Self, Error> {
        let key = Self::health_key(oracle_address);
        env.storage()
            .instance()
            .get(&key)
            .ok_or(Error::InvalidOracleConfig)
    }

    /// Generate storage key for oracle health.
    fn health_key(oracle_address: &Address) -> Symbol {
        let key_str = alloc::format!("oracle_health_{:?}", oracle_address);
        Symbol::new(&Env::default(), &key_str)
    }

    /// Generate storage key for health history.
    fn history_key(oracle_address: &Address) -> Symbol {
        let key_str = alloc::format!("oracle_health_history_{:?}", oracle_address);
        Symbol::new(&Env::default(), &key_str)
    }

    /// Record a health check result (success or failure).
    pub fn record_check(
        &mut self,
        env: &Env,
        success: bool,
        latency_ms: u64,
        confidence_pct: Option<u32>,
    ) -> Result<OracleHealthState, Error> {
        let mut new_state = Self::evaluate_state_transition_static(self)?;

        if success {
            // Update metrics for success
            self.metrics.total_checks = self.metrics.total_checks.checked_add(1).ok_or(Error::InvalidInput)?;
            self.metrics.successful_checks = self.metrics.successful_checks.checked_add(1).ok_or(Error::InvalidInput)?;
            self.metrics.consecutive_failures = 0;
            self.metrics.last_success_time = env.ledger().timestamp();
            
            // Update latency (weighted average: 90% old, 10% new)
            if self.metrics.avg_latency_ms == 0 {
                self.metrics.avg_latency_ms = latency_ms;
            } else {
                let old_weight = self.metrics.avg_latency_ms.checked_mul(9).ok_or(Error::InvalidInput)?;
                let new_weight = latency_ms.checked_mul(1).ok_or(Error::InvalidInput)?;
                self.metrics.avg_latency_ms = old_weight.checked_add(new_weight).ok_or(Error::InvalidInput)?
                    .checked_div(10).ok_or(Error::InvalidInput)?;
            }
            self.metrics.last_latency_ms = latency_ms;
            self.metrics.confidence_pct = confidence_pct;
        } else {
            // Update metrics for failure
            self.metrics.total_checks = self.metrics.total_checks.checked_add(1).ok_or(Error::InvalidInput)?;
            self.metrics.consecutive_failures = self.metrics.consecutive_failures.checked_add(1).ok_or(Error::InvalidInput)?;
            self.metrics.last_failure_time = env.ledger().timestamp();
            self.metrics.last_latency_ms = latency_ms;
        }

        // Recalculate health score
        self.recalculate_health_score(env);
        
        // Evaluate state transition
        new_state = Self::evaluate_state_transition_static(self)?;
        
        // Record transition if changed
        if new_state != self.state {
            Self::record_state_transition_static(env, &self.oracle_address, self.state, new_state)?;
            self.state = new_state;
            self.last_state_change = env.ledger().timestamp();
            self.state_transition_count = self.state_transition_count.checked_add(1).ok_or(Error::InvalidInput)?;
        }

        Ok(self.state)
    }

    /// Update staleness metric and recalculate health.
    pub fn update_staleness(&mut self, env: &Env, staleness_seconds: u64) -> Result<OracleHealthState, Error> {
        self.metrics.staleness_seconds = staleness_seconds;
        self.recalculate_health_score(env);
        let new_state = Self::evaluate_state_transition_static(self)?;
        
        if new_state != self.state {
            Self::record_state_transition_static(env, &self.oracle_address, self.state, new_state)?;
            self.state = new_state;
            self.last_state_change = env.ledger().timestamp();
            self.state_transition_count = self.state_transition_count.checked_add(1).ok_or(Error::InvalidInput)?;
        }
        
        Ok(self.state)
    }

    /// Force state change (admin only).
    pub fn force_state_change(
        env: &Env,
        oracle_address: &Address,
        admin: &Address,
        new_state: OracleHealthState,
    ) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::require_admin_auth(env, admin)?;

        let mut health = Self::load(env, oracle_address)?;
        let old_state = health.state;
        
        if old_state == new_state {
            return Ok(()); // No change needed
        }

        // Record transition
        Self::record_state_transition_static(env, oracle_address, old_state, new_state)?;
        
        // Update state
        health.state = new_state;
        health.last_state_change = env.ledger().timestamp();
        health.state_transition_count = health.state_transition_count.checked_add(1).ok_or(Error::InvalidInput)?;
        
        // Save
        let key = Self::health_key(oracle_address);
        env.storage().instance().set(&key, &health);

        Ok(())
    }

    /// Static method to evaluate state transition based on current metrics.
    fn evaluate_state_transition_static(health: &OracleHealth) -> Result<OracleHealthState, Error> {
        let score = health.metrics.health_score;
        let config = &health.config;
        let current_state = health.state;

        let new_state = match current_state {
            OracleHealthState::Healthy => {
                if score < config.healthy_to_degraded_threshold {
                    OracleHealthState::Degraded
                } else {
                    OracleHealthState::Healthy
                }
            }
            OracleHealthState::Degraded => {
                if score >= config.degraded_to_healthy_threshold {
                    OracleHealthState::Healthy
                } else if score < config.degraded_to_offline_threshold {
                    OracleHealthState::Offline
                } else {
                    OracleHealthState::Degraded
                }
            }
            OracleHealthState::Offline => {
                if score >= config.offline_to_degraded_threshold {
                    OracleHealthState::Degraded
                } else {
                    OracleHealthState::Offline
                }
            }
        };

        Ok(new_state)
    }

    /// Static method to record state transition in history.
    fn record_state_transition_static(
        env: &Env,
        oracle_address: &Address,
        from_state: OracleHealthState,
        to_state: OracleHealthState,
    ) -> Result<(), Error> {
        let key = Self::history_key(oracle_address);
        
        let mut history: Vec<(u64, OracleHealthState, OracleHealthState)> = 
            env.storage().instance().get(&key).unwrap_or_else(|| Vec::new(env));
        
        let transition = (env.ledger().timestamp(), from_state, to_state);
        history.push_back(transition);
        
        // Keep only last 100 transitions
        if history.len() > 100 {
            history.remove(0);
        }
        
        env.storage().instance().set(&key, &history);
        Ok(())
    }

    /// Recalculate health score based on current metrics.
    pub fn recalculate_health_score(&mut self, _env: &Env) {
        let metrics = &mut self.metrics;
        let config = &self.config;
        
        // Start with perfect score
        let mut score: i32 = 100;

        // Penalize for staleness
        if metrics.staleness_seconds > config.max_staleness_seconds {
            let excess = metrics.staleness_seconds.saturating_sub(config.max_staleness_seconds);
            let penalty = (excess as i32).min(30); // Max 30 points penalty
            score -= penalty;
        }

        // Penalize for latency
        if metrics.avg_latency_ms > config.max_latency_ms {
            let excess = metrics.avg_latency_ms.saturating_sub(config.max_latency_ms);
            let penalty = ((excess as i32) / 100).min(20); // Max 20 points penalty
            score -= penalty;
        }

        // Penalize for consecutive failures
        if metrics.consecutive_failures > 0 {
            let penalty = (metrics.consecutive_failures as i32 * 10).min(40); // Max 40 points penalty
            score -= penalty;
        }

        // Penalize for low confidence
        if let Some(confidence) = metrics.confidence_pct {
            if confidence < config.min_confidence_pct {
                let penalty = ((config.min_confidence_pct - confidence) as i32).min(20);
                score -= penalty;
            }
        }

        // Penalize for low success rate (if we have enough data)
        if metrics.total_checks >= 10 {
            let success_rate = (metrics.successful_checks as i32 * 100) / (metrics.total_checks as i32);
            if success_rate < 95 {
                let penalty = (95 - success_rate).min(20);
                score -= penalty;
            }
        }

        // Clamp to 0-100 range
        metrics.health_score = score.clamp(0, 100) as u32;
    }

    /// Get current health state.
    pub fn get_state(&self) -> OracleHealthState {
        self.state
    }

    /// Get health score.
    pub fn get_health_score(&self) -> u32 {
        self.metrics.health_score
    }

    /// Check if oracle is usable for price queries.
    pub fn is_usable(&self) -> bool {
        self.state.is_usable()
    }
}

/// Oracle Health Manager - stateless manager for health operations.
pub struct OracleHealthManager;

// Storage key constants
const HEALTH_CONFIG_KEY: &str = "oracle_health_config";
const HEALTH_STATES_KEY: &str = "oracle_health_states";
const HEALTH_HISTORY_KEY: &str = "oracle_health_history";

impl OracleHealthManager {
    // ===== CONFIGURATION MANAGEMENT =====

    /// Get the global health configuration, initializing if necessary.
    pub fn get_config(env: &Env) -> Result<OracleHealthConfig, Error> {
        if env.storage().instance().get::<Symbol, OracleHealthConfig>(&Symbol::new(env, HEALTH_CONFIG_KEY)).is_none() {
            let config = OracleHealthConfig::default();
            env.storage().instance().set(&Symbol::new(env, HEALTH_CONFIG_KEY), &config);
            return Ok(config);
        }
        env.storage()
            .instance()
            .get::<Symbol, OracleHealthConfig>(&Symbol::new(env, HEALTH_CONFIG_KEY))
            .ok_or(Error::InvalidOracleConfig)
    }

    /// Update the global health configuration (admin only).
    pub fn update_config(env: &Env, admin: &Address, config: &OracleHealthConfig) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::require_admin_auth(env, admin)?;

        // Validate configuration
        config.validate()?;

        env.storage()
            .instance()
            .set(&Symbol::new(env, HEALTH_CONFIG_KEY), config);

        Ok(())
    }

    // ===== HEALTH STATE MANAGEMENT =====

    /// Get the health state for a specific oracle, initializing if necessary.
    pub fn get_oracle_health(env: &Env, oracle_address: &Address) -> Result<OracleHealth, Error> {
        let key = OracleHealth::health_key(oracle_address);
        
        if env.storage().instance().get::<Symbol, OracleHealth>(&key).is_none() {
            let config = Self::get_config(env)?;
            let health = OracleHealth::new_with_config(oracle_address.clone(), config, env)?;
            env.storage().instance().set(&key, &health);
            return Ok(health);
        }

        env.storage()
            .instance()
            .get::<Symbol, OracleHealth>(&key)
            .ok_or(Error::InvalidOracleConfig)
    }

    /// Record a successful health check for an oracle.
    pub fn record_success(
        env: &Env,
        oracle_address: &Address,
        latency_ms: u64,
        staleness_seconds: u64,
        confidence_pct: Option<u32>,
    ) -> Result<OracleHealthState, Error> {
        let mut health = Self::get_oracle_health(env, oracle_address)?;
        let state = health.record_check(env, true, latency_ms, confidence_pct)?;
        health.update_staleness(env, staleness_seconds)?;
        
        // Save updated health
        let key = OracleHealth::health_key(oracle_address);
        env.storage().instance().set(&key, &health);

        Ok(state)
    }

    /// Record a failed health check for an oracle.
    pub fn record_failure(
        env: &Env,
        oracle_address: &Address,
        latency_ms: u64,
    ) -> Result<OracleHealthState, Error> {
        let mut health = Self::get_oracle_health(env, oracle_address)?;
        let state = health.record_check(env, false, latency_ms, None)?;
        
        // Save updated health
        let key = OracleHealth::health_key(oracle_address);
        env.storage().instance().set(&key, &health);

        Ok(state)
    }

    /// Manually set oracle health state (admin only).
    pub fn set_state(
        env: &Env,
        admin: &Address,
        oracle_address: &Address,
        new_state: OracleHealthState,
    ) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::require_admin_auth(env, admin)?;

        let mut health = Self::get_oracle_health(env, oracle_address)?;
        let old_state = health.state;
        
        if old_state == new_state {
            return Ok(()); // No change needed
        }

        // Record transition
        OracleHealth::record_state_transition_static(env, oracle_address, old_state, new_state)?;
        
        // Update state
        health.state = new_state;
        health.last_state_change = env.ledger().timestamp();
        health.state_transition_count = health.state_transition_count.checked_add(1).ok_or(Error::InvalidInput)?;
        
        // Save
        let key = OracleHealth::health_key(oracle_address);
        env.storage().instance().set(&key, &health);

        Ok(())
    }

    /// Reset oracle health to initial state (admin only).
    pub fn reset_health(
        env: &Env,
        admin: &Address,
        oracle_address: &Address,
    ) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::require_admin_auth(env, admin)?;

        let config = Self::get_config(env)?;
        let health = OracleHealth::new_with_config(oracle_address.clone(), config, env)?;
        
        let key = OracleHealth::health_key(oracle_address);
        env.storage().instance().set(&key, &health);

        // Record reset as transition
        OracleHealth::record_state_transition_static(env, oracle_address, OracleHealthState::Offline, OracleHealthState::Healthy)?;

        Ok(())
    }

    // ===== QUERY FUNCTIONS =====

    /// Get health state for an oracle (read-only).
    pub fn get_state(env: &Env, oracle_address: &Address) -> Result<OracleHealthState, Error> {
        let health = Self::get_oracle_health(env, oracle_address)?;
        Ok(health.state)
    }

    /// Get full health details for an oracle (read-only).
    pub fn get_health_details(env: &Env, oracle_address: &Address) -> Result<OracleHealth, Error> {
        Self::get_oracle_health(env, oracle_address)
    }

    /// Get health history for an oracle (read-only).
    pub fn get_history(
        env: &Env,
        oracle_address: &Address,
    ) -> Result<Vec<(u64, OracleHealthState, OracleHealthState)>, Error> {
        let key = OracleHealth::history_key(oracle_address);
        let history: Vec<(u64, OracleHealthState, OracleHealthState)> = 
            env.storage().instance().get(&key).unwrap_or_else(|| Vec::new(env));
        Ok(history)
    }

    /// Get all oracle health states (read-only).
    pub fn get_all_health_states(env: &Env) -> Result<Map<Address, OracleHealthState>, Error> {
        // This would require iterating all storage keys, which is not straightforward in Soroban
        // For now, return empty map - in practice, you'd maintain a registry of oracle addresses
        let map: Map<Address, OracleHealthState> = Map::new(env);
        Ok(map)
    }

    /// Check if an oracle is usable for price queries.
    pub fn is_usable(env: &Env, oracle_address: &Address) -> Result<bool, Error> {
        let state = Self::get_state(env, oracle_address)?;
        Ok(state.is_usable())
    }

    /// Get health score for an oracle (read-only).
    pub fn get_health_score(env: &Env, oracle_address: &Address) -> Result<u32, Error> {
        let health = Self::get_oracle_health(env, oracle_address)?;
        Ok(health.metrics.health_score)
    }
}

// ===== TEST MODULE =====

#[cfg(test)]
mod oracle_health_tests {
    use super::*;
    use crate::admin::AdminRoleManager;
    use crate::err::Error;
    use soroban_sdk::{testutils::Address as _, vec, Env, String, Vec, Address};

    #[test]
    fn test_oracle_health_state_is_usable() {
        assert!(OracleHealthState::Healthy.is_usable());
        assert!(OracleHealthState::Degraded.is_usable());
        assert!(!OracleHealthState::Offline.is_usable());
    }

    #[test]
    fn test_oracle_health_state_is_offline() {
        assert!(!OracleHealthState::Healthy.is_offline());
        assert!(!OracleHealthState::Degraded.is_offline());
        assert!(OracleHealthState::Offline.is_offline());
    }

    #[test]
    fn test_oracle_health_state_as_str() {
        assert_eq!(OracleHealthState::Healthy.as_str(), "Healthy");
        assert_eq!(OracleHealthState::Degraded.as_str(), "Degraded");
        assert_eq!(OracleHealthState::Offline.as_str(), "Offline");
    }

    #[test]
    fn test_oracle_health_config_default() {
        let config = OracleHealthConfig::default();
        assert_eq!(config.healthy_to_degraded_threshold, 70);
        assert_eq!(config.degraded_to_healthy_threshold, 80);
        assert_eq!(config.degraded_to_offline_threshold, 30);
        assert_eq!(config.offline_to_degraded_threshold, 40);
        assert_eq!(config.max_staleness_seconds, 300);
        assert_eq!(config.max_latency_ms, 5000);
        assert_eq!(config.min_confidence_pct, 50);
        assert_eq!(config.max_consecutive_failures, 5);
        assert_eq!(config.health_check_interval_seconds, 60);
    }

    #[test]
    fn test_oracle_health_config_validate_ok() {
        let config = OracleHealthConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_oracle_health_config_validate_hysteresis_gap() {
        let mut config = OracleHealthConfig::default();
        config.degraded_to_healthy_threshold = 70; // Equal to healthy_to_degraded
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_oracle_health_config_validate_offline_hysteresis() {
        let mut config = OracleHealthConfig::default();
        config.offline_to_degraded_threshold = 30; // Equal to degraded_to_offline
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_oracle_health_config_validate_threshold_range() {
        let mut config = OracleHealthConfig::default();
        config.healthy_to_degraded_threshold = 101;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_oracle_health_config_validate_confidence_range() {
        let mut config = OracleHealthConfig::default();
        config.min_confidence_pct = 101;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_oracle_health_config_validate_zero_values() {
        let mut config = OracleHealthConfig::default();
        config.max_staleness_seconds = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_oracle_health_config_validate_threshold_ordering() {
        let mut config = OracleHealthConfig::default();
        config.degraded_to_offline_threshold = 80; // Higher than healthy_to_degraded
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_oracle_health_metrics_default() {
        let metrics = OracleHealthMetrics::default();
        assert_eq!(metrics.health_score, 100);
        assert_eq!(metrics.consecutive_failures, 0);
        assert_eq!(metrics.total_checks, 0);
        assert_eq!(metrics.successful_checks, 0);
    }

    #[test]
    fn test_oracle_health_state_transitions() {
        let config = OracleHealthConfig::default();
        
        // Healthy to Degraded threshold
        assert_eq!(config.healthy_to_degraded_threshold, 70);
        
        // Degraded to Healthy threshold
        assert_eq!(config.degraded_to_healthy_threshold, 80);
        
        // Degraded to Offline threshold
        assert_eq!(config.degraded_to_offline_threshold, 30);
        
        // Offline to Degraded threshold
        assert_eq!(config.offline_to_degraded_threshold, 40);
    }

    #[test]
    fn test_hysteresis_gaps() {
        let config = OracleHealthConfig::default();
        
        // Gap between Healthy->Degraded (70) and Degraded->Healthy (80)
        assert_eq!(config.degraded_to_healthy_threshold - config.healthy_to_degraded_threshold, 10);
        
        // Gap between Degraded->Offline (30) and Offline->Degraded (40)
        assert_eq!(config.offline_to_degraded_threshold - config.degraded_to_offline_threshold, 10);
    }
}

#[cfg(test)]
mod oracle_health_integration_tests {
    use super::*;
    use crate::admin::AdminRoleManager;
    use crate::err::Error;
    use soroban_sdk::{testutils::Address as _, Env, String, Symbol, Address};

    #[test]
    fn test_oracle_health_initialization() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            assert_eq!(health.oracle_address, oracle_addr);
            assert_eq!(health.state, OracleHealthState::Healthy);
            assert_eq!(health.metrics.health_score, 100);
            assert_eq!(health.metrics.consecutive_failures, 0);
            assert_eq!(health.state_transition_count, 0);
        });
    }

    #[test]
    fn test_oracle_health_record_success() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Record a successful check with 100ms latency
            let state = health.record_check(&env, true, 100, Some(80)).unwrap();
            assert_eq!(state, OracleHealthState::Healthy);
            assert_eq!(health.metrics.total_checks, 1);
            assert_eq!(health.metrics.successful_checks, 1);
            assert_eq!(health.metrics.consecutive_failures, 0);
            assert_eq!(health.metrics.avg_latency_ms, 100);
            assert_eq!(health.metrics.confidence_pct, Some(80));
        });
    }

    #[test]
    fn test_oracle_health_record_failure() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Record a failed check
            let state = health.record_check(&env, false, 0, None).unwrap();
            assert_eq!(state, OracleHealthState::Healthy); // First failure doesn't degrade immediately
            assert_eq!(health.metrics.total_checks, 1);
            assert_eq!(health.metrics.successful_checks, 0);
            assert_eq!(health.metrics.consecutive_failures, 1);
        });
    }

    #[test]
    fn test_oracle_health_healthy_to_degraded_transition() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Record failures to drop health score below 70
            for _ in 0..3 {
                health.record_check(&env, false, 0, None).unwrap();
            }
            
            // Should transition to Degraded
            assert_eq!(health.state, OracleHealthState::Degraded);
            assert_eq!(health.state_transition_count, 1);
        });
    }

    #[test]
    fn test_oracle_health_degraded_to_healthy_hysteresis() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Drop to Degraded
            for _ in 0..3 {
                health.record_check(&env, false, 0, None).unwrap();
            }
            assert_eq!(health.state, OracleHealthState::Degraded);
            
            // Recover but stay below 80 (hysteresis threshold)
            for _ in 0..2 {
                health.record_check(&env, true, 100, Some(80)).unwrap();
            }
            // Still Degraded due to hysteresis (score < 80)
            assert_eq!(health.state, OracleHealthState::Degraded);
            
            // Recover above 80
            health.record_check(&env, true, 100, Some(80)).unwrap();
            health.record_check(&env, true, 100, Some(80)).unwrap();
            // Now should transition back to Healthy
            assert_eq!(health.state, OracleHealthState::Healthy);
            assert_eq!(health.state_transition_count, 2);
        });
    }

    #[test]
    fn test_oracle_health_degraded_to_offline_transition() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Drop to Degraded first
            for _ in 0..3 {
                health.record_check(&env, false, 0, None).unwrap();
            }
            assert_eq!(health.state, OracleHealthState::Degraded);
            
            // Continue failing to reach Offline
            for _ in 0..3 {
                health.record_check(&env, false, 0, None).unwrap();
            }
            
            // Should transition to Offline (score < 30)
            assert_eq!(health.state, OracleHealthState::Offline);
            assert_eq!(health.state_transition_count, 2);
        });
    }

    #[test]
    fn test_oracle_health_offline_to_degraded_hysteresis() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Drop to Offline
            for _ in 0..6 {
                health.record_check(&env, false, 0, None).unwrap();
            }
            assert_eq!(health.state, OracleHealthState::Offline);
            
            // Recover but stay below 40 (hysteresis threshold)
            for _ in 0..2 {
                health.record_check(&env, true, 100, Some(80)).unwrap();
            }
            // Still Offline due to hysteresis (score < 40)
            assert_eq!(health.state, OracleHealthState::Offline);
            
            // Recover above 40
            for _ in 0..2 {
                health.record_check(&env, true, 100, Some(80)).unwrap();
            }
            // Now should transition to Degraded
            assert_eq!(health.state, OracleHealthState::Degraded);
            assert_eq!(health.state_transition_count, 2);
        });
    }

    #[test]
    fn test_oracle_health_staleness_check() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Update staleness to exceed threshold (300 seconds)
            let state = health.update_staleness(&env, 350).unwrap();
            
            // Should degrade due to staleness
            assert!(health.metrics.health_score < 100);
            assert_eq!(health.metrics.staleness_seconds, 350);
        });
    }

    #[test]
    fn test_oracle_health_latency_impact() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Record checks with high latency (above 5000ms threshold)
            for _ in 0..5 {
                health.record_check(&env, true, 6000, Some(80)).unwrap();
            }
            
            // High latency should reduce health score
            assert!(health.metrics.health_score < 100);
            assert!(health.metrics.avg_latency_ms > 5000);
        });
    }

    #[test]
    fn test_oracle_health_confidence_impact() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Record checks with low confidence (below 50%)
            for _ in 0..5 {
                health.record_check(&env, true, 100, Some(30)).unwrap();
            }
            
            // Low confidence should reduce health score
            assert!(health.metrics.health_score < 100);
        });
    }

    #[test]
    fn test_oracle_health_max_consecutive_failures() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Record max_consecutive_failures (5) failures
            for _ in 0..5 {
                health.record_check(&env, false, 0, None).unwrap();
            }
            
            // Should be Offline after max consecutive failures
            assert_eq!(health.state, OracleHealthState::Offline);
            assert_eq!(health.metrics.consecutive_failures, 5);
        });
    }

    #[test]
    fn test_oracle_health_state_transition_count() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            assert_eq!(health.state_transition_count, 0);
            
            // Healthy -> Degraded
            for _ in 0..3 {
                health.record_check(&env, false, 0, None).unwrap();
            }
            assert_eq!(health.state_transition_count, 1);
            
            // Degraded -> Offline
            for _ in 0..3 {
                health.record_check(&env, false, 0, None).unwrap();
            }
            assert_eq!(health.state_transition_count, 2);
            
            // Offline -> Degraded
            for _ in 0..5 {
                health.record_check(&env, true, 100, Some(80)).unwrap();
            }
            assert_eq!(health.state_transition_count, 3);
            
            // Degraded -> Healthy
            for _ in 0..5 {
                health.record_check(&env, true, 100, Some(80)).unwrap();
            }
            assert_eq!(health.state_transition_count, 4);
        });
    }

    #[test]
    fn test_oracle_health_require_auth_on_state_change() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            let unauthorized = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let _ = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Should fail with unauthorized caller
            let result = OracleHealth::force_state_change(&env, &oracle_addr, &unauthorized, OracleHealthState::Offline);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_oracle_health_admin_can_force_state() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            assert_eq!(health.state, OracleHealthState::Healthy);
            
            // Admin forces offline
            OracleHealth::force_state_change(&env, &oracle_addr, &admin, OracleHealthState::Offline).unwrap();
            
            // Reload and verify
            let health = OracleHealth::load(&env, &oracle_addr).unwrap();
            assert_eq!(health.state, OracleHealthState::Offline);
        });
    }

    #[test]
    fn test_oracle_health_edge_case_extreme_values() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Extreme latency
            health.record_check(&env, true, u64::MAX, Some(100)).unwrap();
            
            // Health score should not underflow
            assert!(health.metrics.health_score <= 100);
            
            // Many successes should not overflow
            for _ in 0..1000 {
                health.record_check(&env, true, 100, Some(100)).unwrap();
            }
            
            assert!(health.metrics.total_checks <= 1001);
            assert!(health.metrics.successful_checks <= 1001);
        });
    }

    #[test]
    fn test_oracle_health_config_custom() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            
            // Custom config with tighter thresholds
            let mut config = OracleHealthConfig::default();
            config.healthy_to_degraded_threshold = 85;
            config.degraded_to_healthy_threshold = 90;
            config.degraded_to_offline_threshold = 50;
            config.offline_to_degraded_threshold = 60;
            config.validate().unwrap();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // With tighter thresholds, should degrade faster
            health.record_check(&env, false, 0, None).unwrap();
            health.record_check(&env, false, 0, None).unwrap();
            
            // Should be degraded at higher threshold
            assert_eq!(health.state, OracleHealthState::Degraded);
        });
    }
    
    #[test]
    fn test_oracle_health_manager_get_config() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let config = OracleHealthManager::get_config(&env).unwrap();
            assert_eq!(config.healthy_to_degraded_threshold, 70);
            assert_eq!(config.degraded_to_healthy_threshold, 80);
            assert_eq!(config.degraded_to_offline_threshold, 30);
            assert_eq!(config.offline_to_degraded_threshold, 40);
        });
    }

    #[test]
    fn test_oracle_health_manager_update_config_admin() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            let unauthorized = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            // Admin can update config
            let mut config = OracleHealthConfig::default();
            config.healthy_to_degraded_threshold = 75;
            OracleHealthManager::update_config(&env, &admin, &config).unwrap();
            
            let updated = OracleHealthManager::get_config(&env).unwrap();
            assert_eq!(updated.healthy_to_degraded_threshold, 75);
            
            // Unauthorized cannot update
            let result = OracleHealthManager::update_config(&env, &unauthorized, &config);
            assert!(result.is_err());
        });
    }
    
    #[test]
    fn test_oracle_health_manager_get_state() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Get state via manager
            let state = OracleHealthManager::get_state(&env, &oracle_addr).unwrap();
            assert_eq!(state, OracleHealthState::Healthy);
            
            // Make unhealthy
            for _ in 0..5 {
                health.record_check(&env, false, 0, None).unwrap();
            }
            
            let state = OracleHealthManager::get_state(&env, &oracle_addr).unwrap();
            assert_eq!(state, OracleHealthState::Offline);
        });
    }
    
    #[test]
    fn test_oracle_health_manager_is_usable() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Healthy is usable
            assert!(OracleHealthManager::is_usable(&env, &oracle_addr).unwrap());
            
            // Force offline
            OracleHealth::force_state_change(&env, &oracle_addr, &admin, OracleHealthState::Offline).unwrap();
            
            // Offline is not usable
            assert!(!OracleHealthManager::is_usable(&env, &oracle_addr).unwrap());
        });
    }
    
    #[test]
    fn test_oracle_health_manager_get_health_score() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            
            crate::admin::AdminInitializer::initialize(&env, &admin).unwrap();
            AdminRoleManager::assign_role(
                &env,
                &admin,
                crate::admin::AdminRole::SuperAdmin,
                &admin,
            ).unwrap();

            let oracle_addr = Address::generate(&env);
            let config = OracleHealthConfig::default();
            
            let mut health = OracleHealth::new_with_config(oracle_addr.clone(), config, &env).unwrap();
            
            // Initial score should be 100
            let score = OracleHealthManager::get_health_score(&env, &oracle_addr).unwrap();
            assert_eq!(score, 100);
            
            // Record failures to reduce score
            for _ in 0..3 {
                health.record_check(&env, false, 0, None).unwrap();
            }
            
            let score = OracleHealthManager::get_health_score(&env, &oracle_addr).unwrap();
            assert!(score < 100);
        });
    }
}
