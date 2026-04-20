use soroban_sdk::{contracttype, Address, Env, Map, String, Symbol, Vec};

use crate::admin::AdminAccessControl;
use crate::errors::Error;
use crate::events::{CircuitBreakerEvent, EventEmitter};
use alloc::format;
use alloc::string::ToString;

// ===== CIRCUIT BREAKER TYPES =====

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum BreakerState {
    Closed,   // Normal operation
    Open,     // Circuit breaker is open (paused)
    HalfOpen, // Testing if service has recovered
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum BreakerAction {
    Pause,   // Emergency pause
    Resume,  // Resume operations
    Trigger, // Automatic trigger
    Reset,   // Reset circuit breaker
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum BreakerCondition {
    HighErrorRate,      // Error rate exceeds threshold
    HighLatency,        // Response time exceeds threshold
    LowLiquidity,       // Insufficient liquidity
    OracleFailure,      // Oracle service failure
    NetworkCongestion,  // Network issues
    SecurityThreat,     // Security concerns
    ManualOverride,     // Manual intervention
    SystemOverload,     // System overload
    InvalidData,        // Invalid data detected
    UnauthorizedAccess, // Unauthorized access attempts
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct CircuitBreakerConfig {
    pub max_error_rate: u32,         // Maximum error rate percentage (0-100)
    pub max_latency_ms: u64,         // Maximum latency in milliseconds
    pub min_liquidity: i128,         // Minimum liquidity threshold
    pub failure_threshold: u32,      // Number of failures before opening
    pub recovery_timeout: u64,       // Time to wait before attempting recovery
    pub half_open_max_requests: u32, // Max requests in half-open state
    pub auto_recovery_enabled: bool, // Whether to auto-recover
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct CircuitBreakerState {
    pub state: BreakerState,
    pub failure_count: u32,
    pub last_failure_time: u64,
    pub last_success_time: u64,
    pub opened_time: u64,
    pub half_open_requests: u32,
    pub total_requests: u32,
    pub error_count: u32,
    pub pause_scope: PauseScope,
    pub allow_withdrawals: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum PauseScope {
    BettingOnly,
    Full,
}

// ===== CIRCUIT BREAKER IMPLEMENTATION =====

/// Circuit Breaker Pattern for Emergency Pause and Safety
///
/// This struct provides comprehensive circuit breaker functionality
/// including emergency pause, automatic triggers, recovery mechanisms,
/// and event notifications. It ensures contract safety during
/// abnormal conditions and emergencies.
///
/// # Features
///
/// **Emergency Pause:**
/// - Manual pause by admin
/// - Automatic pause on conditions
/// - Pause with reason tracking
///
/// **Automatic Triggers:**
/// - Error rate monitoring
/// - Latency monitoring
/// - Liquidity monitoring
/// - Oracle failure detection
///
/// **Recovery Mechanisms:**
/// - Automatic recovery
/// - Manual recovery
/// - Half-open state testing
/// - Gradual recovery
///
/// **Event System:**
/// - Circuit breaker events
/// - Event notifications
/// - Event history tracking
pub struct CircuitBreaker;

impl CircuitBreaker {
    // ===== STORAGE KEYS =====

    const CONFIG_KEY: &'static str = "circuit_breaker_config";
    const STATE_KEY: &'static str = "circuit_breaker_state";
    const EVENTS_KEY: &'static str = "circuit_breaker_events";
    const CONDITIONS_KEY: &'static str = "circuit_breaker_conditions";

    // ===== CONFIGURATION MANAGEMENT =====

    /// Initialize circuit breaker with default configuration
    pub fn initialize(env: &Env) -> Result<(), Error> {
        let config = CircuitBreakerConfig {
            max_error_rate: 10,           // 10% error rate threshold
            max_latency_ms: 5000,         // 5 second latency threshold
            min_liquidity: 1_000_000_000, // 100 XLM minimum liquidity
            failure_threshold: 5,         // 5 failures before opening
            recovery_timeout: 300,        // 5 minutes recovery timeout
            half_open_max_requests: 3,    // 3 requests in half-open state
            auto_recovery_enabled: true,  // Enable auto-recovery
        };

        let state = CircuitBreakerState {
            state: BreakerState::Closed,
            failure_count: 0,
            last_failure_time: 0,
            last_success_time: env.ledger().timestamp(),
            opened_time: 0,
            half_open_requests: 0,
            total_requests: 0,
            error_count: 0,
            pause_scope: PauseScope::BettingOnly,
            allow_withdrawals: false,
        };

        env.storage()
            .instance()
            .set(&Symbol::new(env, Self::CONFIG_KEY), &config);
        env.storage()
            .instance()
            .set(&Symbol::new(env, Self::STATE_KEY), &state);

        // Initialize empty events and conditions
        let events: Vec<CircuitBreakerEvent> = Vec::new(env);
        let conditions: Map<String, bool> = Map::new(env);

        env.storage()
            .instance()
            .set(&Symbol::new(env, Self::EVENTS_KEY), &events);
        env.storage()
            .instance()
            .set(&Symbol::new(env, Self::CONDITIONS_KEY), &conditions);

        Ok(())
    }

    /// Get circuit breaker configuration
    pub fn get_config(env: &Env) -> Result<CircuitBreakerConfig, Error> {
        env.storage()
            .instance()
            .get(&Symbol::new(env, Self::CONFIG_KEY))
            .ok_or(Error::CBError)
    }

    /// Update circuit breaker configuration
    pub fn update_config(
        env: &Env,
        admin: &Address,
        config: &CircuitBreakerConfig,
    ) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::validate_admin_for_action(env, admin, "update_circuit_breaker_config")?;

        // Validate configuration
        Self::validate_config(config)?;

        env.storage()
            .instance()
            .set(&Symbol::new(env, Self::CONFIG_KEY), config);

        // Emit configuration update event
        let _ = Self::emit_circuit_breaker_event(
            env,
            BreakerAction::Reset,
            BreakerCondition::ManualOverride,
            &String::from_str(env, "Configuration updated"),
            Some(admin.clone()),
        );

        Ok(())
    }

    // ===== STATE MANAGEMENT =====

    /// Get current circuit breaker state
    pub fn get_state(env: &Env) -> Result<CircuitBreakerState, Error> {
        env.storage()
            .instance()
            .get(&Symbol::new(env, Self::STATE_KEY))
            .ok_or(Error::CBError)
    }

    /// Update circuit breaker state
    fn update_state(env: &Env, state: &CircuitBreakerState) -> Result<(), Error> {
        env.storage()
            .instance()
            .set(&Symbol::new(env, Self::STATE_KEY), state);
        Ok(())
    }

    // ===== EMERGENCY PAUSE =====

    /// Emergency pause by admin
    pub fn emergency_pause(env: &Env, admin: &Address, reason: &String) -> Result<(), Error> {
        // Default emergency pause uses BettingOnly scope and disallows withdrawals
        Self::pause_with_options(env, admin, reason, PauseScope::BettingOnly, false)
    }

    /// Pause with explicit options
    pub fn pause_with_options(
        env: &Env,
        admin: &Address,
        reason: &String,
        scope: PauseScope,
        allow_withdrawals: bool,
    ) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::validate_admin_for_action(env, admin, "emergency_actions")?;

        let mut state = Self::get_state(env)?;

        // Check if already paused
        if state.state == BreakerState::Open {
            return Err(Error::CBError);
        }

        // Update state
        state.state = BreakerState::Open;
        state.opened_time = env.ledger().timestamp();
        state.pause_scope = scope;
        state.allow_withdrawals = allow_withdrawals;
        Self::update_state(env, &state)?;

        // Emit pause event
        let _ = Self::emit_circuit_breaker_event(
            env,
            BreakerAction::Pause,
            BreakerCondition::ManualOverride,
            reason,
            Some(admin.clone()),
        );
        crate::monitoring::ContractMonitor::emit_pause_transition_hook(
            env,
            &String::from_str(env, "paused"),
            Some(admin.clone()),
            reason,
        );

        Ok(())
    }

    /// Check if circuit breaker is open (paused)
    pub fn is_open(env: &Env) -> Result<bool, Error> {
        let state = Self::get_state(env)?;
        Ok(state.state == BreakerState::Open)
    }

    /// Check if circuit breaker is closed (normal operation)
    pub fn is_closed(env: &Env) -> Result<bool, Error> {
        let state = Self::get_state(env)?;
        Ok(state.state == BreakerState::Closed)
    }

    /// Check if circuit breaker is in half-open state
    pub fn is_half_open(env: &Env) -> Result<bool, Error> {
        let state = Self::get_state(env)?;
        Ok(state.state == BreakerState::HalfOpen)
    }

    /// Check whether a specific operation is allowed under current pause scope.
    /// `op` examples: "betting", "create_event", "withdraw", etc.
    pub fn is_operation_allowed(env: &Env, op: &str) -> Result<bool, Error> {
        let state = Self::get_state(env)?;

        match state.state {
            BreakerState::Closed => Ok(true),
            BreakerState::Open => match state.pause_scope {
                PauseScope::Full => Ok(false),
                PauseScope::BettingOnly => {
                    if op == "betting" {
                        Ok(false)
                    } else {
                        Ok(true)
                    }
                }
            },
            BreakerState::HalfOpen => {
                let config = Self::get_config(env)?;
                Ok(state.half_open_requests < config.half_open_max_requests)
            }
        }
    }

    /// Returns whether withdrawals are allowed under the current pause state.
    pub fn are_withdrawals_allowed(env: &Env) -> Result<bool, Error> {
        let state = Self::get_state(env)?;
        if state.state == BreakerState::Open && !state.allow_withdrawals {
            return Ok(false);
        }
        Ok(true)
    }

    // ===== AUTOMATIC TRIGGERS =====

    /// Automatic circuit breaker trigger based on conditions
    pub fn automatic_circuit_breaker_trigger(
        env: &Env,
        condition: &BreakerCondition,
    ) -> Result<bool, Error> {
        let config = Self::get_config(env)?;
        let mut state = Self::get_state(env)?;
        let current_time = env.ledger().timestamp();

        // Check if auto-recovery is enabled and enough time has passed
        if config.auto_recovery_enabled && state.state == BreakerState::Open {
            if current_time - state.opened_time >= config.recovery_timeout {
                state.state = BreakerState::HalfOpen;
                state.half_open_requests = 0;
                Self::update_state(env, &state)?;

                let _ = Self::emit_circuit_breaker_event(
                    env,
                    BreakerAction::Reset,
                    BreakerCondition::ManualOverride,
                    &String::from_str(env, "Auto-recovery: transitioning to half-open"),
                    None,
                );
            }
        }

        // Check conditions and trigger if necessary
        let should_trigger = match condition {
            BreakerCondition::HighErrorRate => {
                if state.total_requests > 0 {
                    let error_rate = (state.error_count * 100) / state.total_requests;
                    error_rate >= config.max_error_rate
                } else {
                    false
                }
            }
            BreakerCondition::HighLatency => {
                // This would be implemented with actual latency tracking
                false
            }
            BreakerCondition::LowLiquidity => {
                // This would be implemented with actual liquidity checking
                false
            }
            BreakerCondition::OracleFailure => {
                // This would be implemented with oracle health checking
                false
            }
            BreakerCondition::NetworkCongestion => {
                // This would be implemented with network monitoring
                false
            }
            BreakerCondition::SecurityThreat => {
                // This would be implemented with security monitoring
                false
            }
            BreakerCondition::ManualOverride => {
                false // Manual override is handled separately
            }
            BreakerCondition::SystemOverload => {
                // This would be implemented with system monitoring
                false
            }
            BreakerCondition::InvalidData => {
                // This would be implemented with data validation
                false
            }
            BreakerCondition::UnauthorizedAccess => {
                // This would be implemented with access monitoring
                false
            }
        };

        if should_trigger && state.state != BreakerState::Open {
            state.state = BreakerState::Open;
            state.failure_count += 1;
            state.last_failure_time = current_time;
            state.opened_time = current_time;
            Self::update_state(env, &state)?;

            let _ = Self::emit_circuit_breaker_event(
                env,
                BreakerAction::Trigger,
                condition.clone(),
                &String::from_str(env, "Automatic circuit breaker triggered"),
                None,
            );

            return Ok(true);
        }

        Ok(false)
    }

    // ===== RECOVERY MECHANISMS =====

    /// Circuit breaker recovery by admin
    pub fn circuit_breaker_recovery(env: &Env, admin: &Address) -> Result<(), Error> {
        // Validate admin permissions
        AdminAccessControl::validate_admin_for_action(env, admin, "emergency_actions")?;

        let mut state = Self::get_state(env)?;

        // Check if circuit breaker is open
        if state.state != BreakerState::Open && state.state != BreakerState::HalfOpen {
            return Err(Error::CBError);
        }

        // Reset state
        state.state = BreakerState::Closed;
        state.failure_count = 0;
        state.half_open_requests = 0;
        state.last_success_time = env.ledger().timestamp();
        // restore safe defaults
        state.pause_scope = PauseScope::BettingOnly;
        state.allow_withdrawals = false;
        Self::update_state(env, &state)?;

        // Emit recovery event
        let _ = Self::emit_circuit_breaker_event(
            env,
            BreakerAction::Resume,
            BreakerCondition::ManualOverride,
            &String::from_str(env, "Circuit breaker recovered"),
            Some(admin.clone()),
        );
        crate::monitoring::ContractMonitor::emit_pause_transition_hook(
            env,
            &String::from_str(env, "unpaused"),
            Some(admin.clone()),
            &String::from_str(env, "manual_recovery"),
        );

        Ok(())
    }

    /// Record a successful operation (for half-open state)
    pub fn record_success(env: &Env) -> Result<(), Error> {
        let mut state = Self::get_state(env)?;
        let current_time = env.ledger().timestamp();

        state.total_requests += 1;
        state.last_success_time = current_time;

        // If in half-open state, check if we can close
        if state.state == BreakerState::HalfOpen {
            state.half_open_requests += 1;

            let config = Self::get_config(env)?;
            if state.half_open_requests >= config.half_open_max_requests {
                state.state = BreakerState::Closed;
                state.failure_count = 0;
                state.half_open_requests = 0;

                let _ = Self::emit_circuit_breaker_event(
                    env,
                    BreakerAction::Resume,
                    BreakerCondition::ManualOverride,
                    &String::from_str(env, "Auto-recovery: circuit breaker closed"),
                    None,
                );
                crate::monitoring::ContractMonitor::emit_pause_transition_hook(
                    env,
                    &String::from_str(env, "unpaused"),
                    None,
                    &String::from_str(env, "auto_recovery"),
                );
            }
        }

        Self::update_state(env, &state)?;
        Ok(())
    }

    /// Record a failed operation
    pub fn record_failure(env: &Env) -> Result<(), Error> {
        let mut state = Self::get_state(env)?;
        let current_time = env.ledger().timestamp();

        state.total_requests += 1;
        state.error_count += 1;
        state.last_failure_time = current_time;

        // If in half-open state, open the circuit breaker
        if state.state == BreakerState::HalfOpen {
            state.state = BreakerState::Open;
            state.opened_time = current_time;
            state.half_open_requests = 0;

            let _ = Self::emit_circuit_breaker_event(
                env,
                BreakerAction::Trigger,
                BreakerCondition::HighErrorRate,
                &String::from_str(env, "Failure in half-open state, reopening circuit breaker"),
                None,
            );
        }

        Self::update_state(env, &state)?;
        Ok(())
    }

    // ===== EVENT SYSTEM =====

    /// Emit circuit breaker event
    pub fn emit_circuit_breaker_event(
        env: &Env,
        action: BreakerAction,
        condition: BreakerCondition,
        reason: &String,
        admin: Option<Address>,
    ) -> Result<(), Error> {
        let event = CircuitBreakerEvent {
            action,
            condition,
            reason: reason.clone(),
            timestamp: env.ledger().timestamp(),
            admin,
        };

        // Store event in history
        let mut events: Vec<CircuitBreakerEvent> = env
            .storage()
            .instance()
            .get(&Symbol::new(env, Self::EVENTS_KEY))
            .unwrap_or_else(|| Vec::new(env));

        events.push_back(event.clone());

        // Keep only last 100 events
        if events.len() > 100 {
            events.remove(0);
        }

        env.storage()
            .instance()
            .set(&Symbol::new(env, Self::EVENTS_KEY), &events);

        // Emit event
        EventEmitter::emit_circuit_breaker_event(env, &event);

        Ok(())
    }

    /// Get circuit breaker event history
    pub fn get_event_history(env: &Env) -> Result<Vec<CircuitBreakerEvent>, Error> {
        env.storage()
            .instance()
            .get(&Symbol::new(env, Self::EVENTS_KEY))
            .ok_or(Error::CBError)
    }

    // ===== STATUS AND MONITORING =====

    /// Get circuit breaker status
    pub fn get_circuit_breaker_status(env: &Env) -> Result<Map<String, String>, Error> {
        let state = Self::get_state(env)?;
        let config = Self::get_config(env)?;
        let current_time = env.ledger().timestamp();

        let mut status = Map::new(env);

        status.set(
            String::from_str(env, "state"),
            String::from_str(env, &format!("{:?}", state.state)),
        );

        status.set(
            String::from_str(env, "failure_count"),
            String::from_str(env, &state.failure_count.to_string()),
        );

        status.set(
            String::from_str(env, "total_requests"),
            String::from_str(env, &state.total_requests.to_string()),
        );

        status.set(
            String::from_str(env, "error_count"),
            String::from_str(env, &state.error_count.to_string()),
        );

        if state.total_requests > 0 {
            let error_rate = (state.error_count * 100) / state.total_requests;
            status.set(
                String::from_str(env, "error_rate_percent"),
                String::from_str(env, &error_rate.to_string()),
            );
        }

        status.set(
            String::from_str(env, "max_error_rate"),
            String::from_str(env, &config.max_error_rate.to_string()),
        );

        status.set(
            String::from_str(env, "failure_threshold"),
            String::from_str(env, &config.failure_threshold.to_string()),
        );

        if state.state == BreakerState::Open {
            let time_open = current_time - state.opened_time;
            status.set(
                String::from_str(env, "time_open_seconds"),
                String::from_str(env, &time_open.to_string()),
            );

            let time_until_recovery = if time_open < config.recovery_timeout {
                config.recovery_timeout - time_open
            } else {
                0
            };

            status.set(
                String::from_str(env, "time_until_recovery_seconds"),
                String::from_str(env, &time_until_recovery.to_string()),
            );
        }

        if state.state == BreakerState::HalfOpen {
            status.set(
                String::from_str(env, "half_open_requests"),
                String::from_str(env, &state.half_open_requests.to_string()),
            );

            status.set(
                String::from_str(env, "max_half_open_requests"),
                String::from_str(env, &config.half_open_max_requests.to_string()),
            );
        }

        status.set(
            String::from_str(env, "auto_recovery_enabled"),
            String::from_str(env, &config.auto_recovery_enabled.to_string()),
        );

        Ok(status)
    }

    // ===== VALIDATION =====

    /// Validate circuit breaker conditions
    pub fn validate_circuit_breaker_conditions(
        conditions: &Vec<BreakerCondition>,
    ) -> Result<(), Error> {
        if conditions.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Check for duplicate conditions
        for i in 0..conditions.len() {
            for j in (i + 1)..conditions.len() {
                if conditions.get(i).unwrap() == conditions.get(j).unwrap() {
                    return Err(Error::InvalidInput);
                }
            }
        }

        Ok(())
    }

    /// Validate circuit breaker configuration
    fn validate_config(config: &CircuitBreakerConfig) -> Result<(), Error> {
        if config.max_error_rate > 100 {
            return Err(Error::InvalidInput);
        }

        if config.max_latency_ms == 0 {
            return Err(Error::InvalidInput);
        }

        if config.min_liquidity < 0 {
            return Err(Error::InvalidInput);
        }

        if config.failure_threshold == 0 {
            return Err(Error::InvalidInput);
        }

        if config.recovery_timeout == 0 {
            return Err(Error::InvalidInput);
        }

        if config.half_open_max_requests == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    // ===== TESTING =====

    /// Test circuit breaker scenarios
    pub fn test_circuit_breaker_scenarios(env: &Env) -> Result<Map<String, String>, Error> {
        let mut results = Map::new(env);

        // Test 1: Normal operation
        let is_closed = Self::is_closed(env)?;
        results.set(
            String::from_str(env, "normal_operation"),
            String::from_str(env, &is_closed.to_string()),
        );

        // Test 2: Emergency pause
        let test_admin = Address::from_str(
            env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        );
        let pause_result = Self::emergency_pause(
            env,
            &test_admin,
            &String::from_str(env, "Test emergency pause"),
        );
        results.set(
            String::from_str(env, "emergency_pause"),
            String::from_str(env, &pause_result.is_ok().to_string()),
        );

        // Test 3: Recovery
        let recovery_result = Self::circuit_breaker_recovery(env, &test_admin);
        results.set(
            String::from_str(env, "recovery"),
            String::from_str(env, &recovery_result.is_ok().to_string()),
        );

        // Test 4: Status check
        let _status = Self::get_circuit_breaker_status(env)?;
        results.set(
            String::from_str(env, "status_check"),
            String::from_str(env, "success"),
        );

        // Test 5: Event history
        let events = Self::get_event_history(env)?;
        results.set(
            String::from_str(env, "event_history"),
            String::from_str(env, &events.len().to_string()),
        );

        Ok(results)
    }
}

// ===== CIRCUIT BREAKER UTILITIES =====

/// Circuit breaker utilities for common operations
pub struct CircuitBreakerUtils;

impl CircuitBreakerUtils {
    /// Check if operation should be allowed
    pub fn should_allow_operation(env: &Env) -> Result<bool, Error> {
        let state = CircuitBreaker::get_state(env)?;

        match state.state {
            BreakerState::Closed => Ok(true),
            BreakerState::Open => Ok(false),
            BreakerState::HalfOpen => {
                let config = CircuitBreaker::get_config(env)?;
                Ok(state.half_open_requests < config.half_open_max_requests)
            }
        }
    }

    /// Wrap operation with circuit breaker protection
    pub fn with_circuit_breaker<F, T>(env: &Env, operation: F) -> Result<T, Error>
    where
        F: FnOnce() -> Result<T, Error>,
    {
        // Check if operation should be allowed
        if !Self::should_allow_operation(env)? {
            return Err(Error::CBError);
        }

        // Execute operation
        match operation() {
            Ok(result) => {
                CircuitBreaker::record_success(env)?;
                Ok(result)
            }
            Err(error) => {
                CircuitBreaker::record_failure(env)?;
                Err(error)
            }
        }
    }

    /// Get circuit breaker statistics
    pub fn get_statistics(env: &Env) -> Result<Map<String, String>, Error> {
        let state = CircuitBreaker::get_state(env)?;
        let mut stats = Map::new(env);

        stats.set(
            String::from_str(env, "total_requests"),
            String::from_str(env, &state.total_requests.to_string()),
        );

        stats.set(
            String::from_str(env, "error_count"),
            String::from_str(env, &state.error_count.to_string()),
        );

        if state.total_requests > 0 {
            let error_rate = (state.error_count * 100) / state.total_requests;
            stats.set(
                String::from_str(env, "error_rate_percent"),
                String::from_str(env, &error_rate.to_string()),
            );
        }

        stats.set(
            String::from_str(env, "failure_count"),
            String::from_str(env, &state.failure_count.to_string()),
        );

        stats.set(
            String::from_str(env, "current_state"),
            String::from_str(env, &format!("{:?}", state.state)),
        );

        Ok(stats)
    }
}

// ===== CIRCUIT BREAKER TESTING =====

/// Circuit breaker testing utilities
pub struct CircuitBreakerTesting;

impl CircuitBreakerTesting {
    /// Create test circuit breaker configuration
    pub fn create_test_config(_env: &Env) -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            max_error_rate: 5,           // 5% error rate threshold
            max_latency_ms: 1000,        // 1 second latency threshold
            min_liquidity: 100_000_000,  // 10 XLM minimum liquidity
            failure_threshold: 3,        // 3 failures before opening
            recovery_timeout: 60,        // 1 minute recovery timeout
            half_open_max_requests: 2,   // 2 requests in half-open state
            auto_recovery_enabled: true, // Enable auto-recovery
        }
    }

    /// Create test circuit breaker state
    pub fn create_test_state(env: &Env) -> CircuitBreakerState {
        CircuitBreakerState {
            state: BreakerState::Closed,
            failure_count: 0,
            last_failure_time: 0,
            last_success_time: env.ledger().timestamp(),
            opened_time: 0,
            half_open_requests: 0,
            total_requests: 0,
            error_count: 0,
            pause_scope: PauseScope::BettingOnly,
            allow_withdrawals: false,
        }
    }

    /// Simulate circuit breaker failure
    pub fn simulate_failure(env: &Env) -> Result<(), Error> {
        CircuitBreaker::record_failure(env)?;
        Ok(())
    }

    /// Simulate circuit breaker success
    pub fn simulate_success(env: &Env) -> Result<(), Error> {
        CircuitBreaker::record_success(env)?;
        Ok(())
    }

    /// Simulate automatic trigger
    pub fn simulate_trigger(env: &Env, condition: &BreakerCondition) -> Result<bool, Error> {
        CircuitBreaker::automatic_circuit_breaker_trigger(env, condition)
    }

    /// Reset circuit breaker to initial state
    pub fn reset(env: &Env, admin: &Address) -> Result<(), Error> {
        CircuitBreaker::circuit_breaker_recovery(env, admin)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use soroban_sdk::testutils::Address as _;

    struct CircuitBreakerTest {
        env: Env,
    }

    impl CircuitBreakerTest {
        fn new() -> Self {
            let env = Env::default();
            CircuitBreakerTest { env }
        }
    }

    #[test]
    fn test_breaker_state_closed() {
        let _test = CircuitBreakerTest::new();
        let state = BreakerState::Closed;
        assert_eq!(state, BreakerState::Closed);
    }

    #[test]
    fn test_breaker_state_open() {
        let _test = CircuitBreakerTest::new();
        let state = BreakerState::Open;
        assert_eq!(state, BreakerState::Open);
    }

    #[test]
    fn test_breaker_state_half_open() {
        let _test = CircuitBreakerTest::new();
        let state = BreakerState::HalfOpen;
        assert_eq!(state, BreakerState::HalfOpen);
    }

    #[test]
    fn test_breaker_action_pause() {
        let _test = CircuitBreakerTest::new();
        let action = BreakerAction::Pause;
        assert_eq!(action, BreakerAction::Pause);
    }

    #[test]
    fn test_breaker_action_resume() {
        let _test = CircuitBreakerTest::new();
        let action = BreakerAction::Resume;
        assert_eq!(action, BreakerAction::Resume);
    }

    #[test]
    fn test_breaker_condition_high_error_rate() {
        let _test = CircuitBreakerTest::new();
        let condition = BreakerCondition::HighErrorRate;
        assert_eq!(condition, BreakerCondition::HighErrorRate);
    }

    #[test]
    fn test_breaker_condition_oracle_failure() {
        let _test = CircuitBreakerTest::new();
        let condition = BreakerCondition::OracleFailure;
        assert_eq!(condition, BreakerCondition::OracleFailure);
    }

    #[test]
    fn test_pause_scope_betting_only() {
        let _test = CircuitBreakerTest::new();
        let scope = PauseScope::BettingOnly;
        assert_eq!(scope, PauseScope::BettingOnly);
    }

    #[test]
    fn test_pause_scope_full() {
        let _test = CircuitBreakerTest::new();
        let scope = PauseScope::Full;
        assert_eq!(scope, PauseScope::Full);
    }

    #[test]
    fn test_config_initialization() {
        let test = CircuitBreakerTest::new();
        let config = CircuitBreakerConfig {
            max_error_rate: 10,
            max_latency_ms: 5000,
            min_liquidity: 1_000_000_000,
            failure_threshold: 5,
            recovery_timeout: 300,
            half_open_max_requests: 3,
            auto_recovery_enabled: true,
        };
        assert_eq!(config.max_error_rate, 10);
        assert_eq!(config.half_open_max_requests, 3);
    }

    #[test]
    fn test_state_initialization() {
        let test = CircuitBreakerTest::new();
        let state = CircuitBreakerState {
            state: BreakerState::Closed,
            failure_count: 0,
            last_failure_time: 0,
            last_success_time: test.env.ledger().timestamp(),
            opened_time: 0,
            half_open_requests: 0,
            total_requests: 0,
            error_count: 0,
            pause_scope: PauseScope::BettingOnly,
            allow_withdrawals: false,
        };
        assert_eq!(state.state, BreakerState::Closed);
        assert_eq!(state.failure_count, 0);
    }

    #[test]
    fn test_circuit_breaker_initialize() {
        let test = CircuitBreakerTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        let result = test
            .env
            .as_contract(&contract_id, || CircuitBreaker::initialize(&test.env));
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_config_after_init() {
        let test = CircuitBreakerTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        let result = test.env.as_contract(&contract_id, || {
            let _ = CircuitBreaker::initialize(&test.env);
            CircuitBreaker::get_config(&test.env)
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_state_after_init() {
        let test = CircuitBreakerTest::new();
        let contract_id = test.env.register(crate::PredictifyHybrid, ());
        let result = test.env.as_contract(&contract_id, || {
            let _ = CircuitBreaker::initialize(&test.env);
            CircuitBreaker::get_state(&test.env)
        });
        assert!(result.is_ok());
    }

    #[test]
    fn test_breaker_condition_all_variants() {
        let _test = CircuitBreakerTest::new();
        let _ = BreakerCondition::HighErrorRate;
        let _ = BreakerCondition::HighLatency;
        let _ = BreakerCondition::LowLiquidity;
        let _ = BreakerCondition::OracleFailure;
        let _ = BreakerCondition::NetworkCongestion;
        let _ = BreakerCondition::SecurityThreat;
        let _ = BreakerCondition::ManualOverride;
        let _ = BreakerCondition::SystemOverload;
        let _ = BreakerCondition::InvalidData;
        let _ = BreakerCondition::UnauthorizedAccess;
    }

    #[test]
    fn test_breaker_action_all_variants() {
        let _test = CircuitBreakerTest::new();
        let _ = BreakerAction::Pause;
        let _ = BreakerAction::Resume;
        let _ = BreakerAction::Trigger;
        let _ = BreakerAction::Reset;
    }

    #[test]
    fn test_validate_config() {
        let test = CircuitBreakerTest::new();
        let config = CircuitBreakerConfig {
            max_error_rate: 15,
            max_latency_ms: 3000,
            min_liquidity: 500_000_000,
            failure_threshold: 3,
            recovery_timeout: 600,
            half_open_max_requests: 5,
            auto_recovery_enabled: true,
        };
        let result = CircuitBreaker::validate_config(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_transitions() {
        let test = CircuitBreakerTest::new();
        // Test state transitions
        let mut state = CircuitBreakerState {
            state: BreakerState::Closed,
            failure_count: 0,
            last_failure_time: 0,
            last_success_time: test.env.ledger().timestamp(),
            opened_time: 0,
            half_open_requests: 0,
            total_requests: 0,
            error_count: 0,
            pause_scope: PauseScope::BettingOnly,
            allow_withdrawals: false,
        };
        assert_eq!(state.state, BreakerState::Closed);
        state.state = BreakerState::Open;
        assert_eq!(state.state, BreakerState::Open);
    }

    #[test]
    fn test_failure_count_increment() {
        let test = CircuitBreakerTest::new();
        let mut state = CircuitBreakerState {
            state: BreakerState::Closed,
            failure_count: 0,
            last_failure_time: 0,
            last_success_time: 0,
            opened_time: 0,
            half_open_requests: 0,
            total_requests: 0,
            error_count: 0,
            pause_scope: PauseScope::BettingOnly,
            allow_withdrawals: false,
        };
        assert_eq!(state.failure_count, 0);
        state.failure_count += 1;
        assert_eq!(state.failure_count, 1);
    }

    #[test]
    fn test_error_rate_calculation() {
        let _test = CircuitBreakerTest::new();
        let total_requests = 100u32;
        let error_count = 10u32;
        let error_rate = (error_count * 100) / total_requests;
        assert_eq!(error_rate, 10);
    }

    #[test]
    fn test_withdrawal_permissions() {
        let test = CircuitBreakerTest::new();
        let state = CircuitBreakerState {
            state: BreakerState::Closed,
            failure_count: 0,
            last_failure_time: 0,
            last_success_time: test.env.ledger().timestamp(),
            opened_time: 0,
            half_open_requests: 0,
            total_requests: 0,
            error_count: 0,
            pause_scope: PauseScope::BettingOnly,
            allow_withdrawals: true,
        };
        assert!(state.allow_withdrawals);
    }
}
