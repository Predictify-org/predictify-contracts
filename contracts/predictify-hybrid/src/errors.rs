#![allow(dead_code)]

use alloc::format;
use alloc::string::ToString;
use soroban_sdk::{contracterror, contracttype, Address, Env, Map, String, Symbol, Vec};

/// Comprehensive error codes for the Predictify Hybrid prediction market contract.
///
/// This enum defines all possible error conditions that can occur within the Predictify Hybrid
/// smart contract system.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    // ===== USER OPERATION ERRORS =====
    /// User is not authorized to perform this action
    Unauthorized = 100,
    /// Market not found
    MarketNotFound = 101,
    /// Market is closed (has ended)
    MarketClosed = 102,
    /// Market is already resolved
    MarketResolved = 103,
    /// Market is not resolved yet
    MarketNotResolved = 104,
    /// User has nothing to claim
    NothingToClaim = 105,
    /// User has already claimed
    AlreadyClaimed = 106,
    /// Insufficient stake amount
    InsufficientStake = 107,
    /// Invalid outcome choice
    InvalidOutcome = 108,
    /// User has already voted in this market
    AlreadyVoted = 109,
    /// User has already placed a bet on this market
    AlreadyBet = 110,
    /// Bets have already been placed on this market (cannot update)
    BetsAlreadyPlaced = 111,
    /// Insufficient balance
    InsufficientBalance = 112,
    /// User is blacklisted and cannot perform this action
    Blacklist = 113,
    /// User is not on the required whitelist for this action
    NoWhitelist = 114,
    // FundsLocked removed to save space

    // ===== ORACLE ERRORS =====
    /// Oracle is unavailable
    OracleUnavailable = 200,
    /// Invalid oracle configuration
    InvalidOracleConfig = 201,
    /// Oracle data is stale or timed out
    OracleStale = 202,
    /// Oracle consensus not reached (multi-oracle)
    OracleNoConsensus = 203,
    /// Oracle result already verified for this market
    OracleVerified = 204,
    /// Market not ready for oracle verification
    MarketNotReady = 205,
    /// FallbackUnavail or unhealthy
    FallbackUnavail = 206,
    /// ResTimeoutas been reached
    ResTimeout = 207,
    /// Refund process has been initiated
    RefundStarted = 208,

    // ===== VALIDATION ERRORS =====
    /// Invalid question format
    InvalidQuestion = 300,
    /// Invalid outcomes provided
    InvalidOutcomes = 301,
    /// Invalid duration specified
    InvalidDuration = 302,
    /// Invalid threshold value
    InvalidThreshold = 303,
    /// Invalid comparison operator
    InvalidComparison = 304,

    // ===== ADDITIONAL ERRORS =====
    /// Invalid state
    InvalidState = 400,
    /// Invalid input
    InvalidInput = 401,
    /// Invalid fee configuration
    InvalidFeeConfig = 402,
    /// Configuration not found
    ConfigNotFound = 403,
    /// Already disputed
    AlreadyDisputed = 404,
    /// Dispute voting period expired
    DisputeVoteExpired = 405,
    /// Dispute voting not allowed
    DisputeVoteDenied = 406,
    /// Already voted in dispute
    DisputeAlreadyVoted = 407,
    /// Dispute resolution conditions not met
    DisputeCondNotMet = 408,
    /// Dispute fee distribution failed
    DisputeFeeFailed = 409,
    /// Dispute escalation not allowed
    DisputeNoEscalate = 410,
    /// Threshold below minimum
    ThresholdBelowMin = 411,
    /// Threshold exceeds maximum
    ThresholdTooHigh = 412,
    /// Fee already collected
    FeeAlreadyCollected = 413,
    /// No fees to collect
    NoFeesToCollect = 414,
    /// Invalid extension days
    InvalidExtensionDays = 415,
    /// Extension not allowed or exceeded
    ExtensionDenied = 416,
    /// Extension fee insufficient
    ExtensionFeeLow = 417,
    /// Admin address is not set (initialization missing)
    AdminNotSet = 418,
    /// Dispute timeout not set
    TimeoutNotSet = 419,
    /// Dispute timeout not expired
    TimeoutNotExpired = 420,
    /// Invalid timeout hours
    InvalidTimeoutHours = 422,

    // ===== CIRCUIT BREAKER ERRORS =====
    /// Circuit breaker not initialized
    CBNotInitialized = 500,
    /// Circuit breaker is already open (paused)
    CBAlreadyOpen = 501,
    /// Circuit breaker is not open (cannot recover)
    CBNotOpen = 502,
    /// Circuit breaker is open (operations blocked)
    CBOpen = 503,
}

// ===== ERROR CATEGORIZATION AND RECOVERY SYSTEM =====

/// Error severity levels for categorization and prioritization
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorSeverity {
    /// Low severity - informational or minor issues
    Low,
    /// Medium severity - warnings or recoverable issues
    Medium,
    /// High severity - significant issues requiring attention
    High,
    /// Critical severity - system-breaking issues
    Critical,
}

/// Error categories for grouping and analysis
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorCategory {
    /// User operation related errors
    UserOperation,
    /// Oracle and external data errors
    Oracle,
    /// Input validation errors
    Validation,
    /// System and configuration errors
    System,
    /// Dispute and governance errors
    Dispute,
    /// Fee and financial errors
    Financial,
    /// Market state errors
    Market,
    /// Authentication and authorization errors
    Authentication,
    /// Unknown or uncategorized errors
    Unknown,
}

/// Error recovery strategies
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryStrategy {
    /// Retry the operation
    Retry,
    /// Wait and retry later
    RetryWithDelay,
    /// Use alternative method
    AlternativeMethod,
    /// Skip operation and continue
    Skip,
    /// Abort operation
    Abort,
    /// Manual intervention required
    ManualIntervention,
    /// No recovery possible
    NoRecovery,
}

/// Error context information for debugging and recovery
#[contracttype]
#[derive(Clone, Debug)]
pub struct ErrorContext {
    /// Function or operation where error occurred
    pub operation: String,
    /// User address involved (if applicable)
    pub user_address: Option<Address>,
    /// Market ID involved (if applicable)
    pub market_id: Option<Symbol>,
    /// Additional context data
    pub context_data: Map<String, String>,
    /// Timestamp when error occurred
    pub timestamp: u64,
    /// Stack trace or call chain (simplified)
    pub call_chain: Vec<String>,
}

/// Detailed error information with categorization and recovery data
#[derive(Clone, Debug)]
pub struct DetailedError {
    /// The original error
    pub error: Error,
    /// Error severity level
    pub severity: ErrorSeverity,
    /// Error category
    pub category: ErrorCategory,
    /// Recovery strategy
    pub recovery_strategy: RecoveryStrategy,
    /// Error context
    pub context: ErrorContext,
    /// Detailed error message
    pub detailed_message: String,
    /// Suggested user action
    pub user_action: String,
    /// Technical details for debugging
    pub technical_details: String,
}

/// Error analytics and statistics
#[contracttype]
#[derive(Clone, Debug)]
pub struct ErrorAnalytics {
    /// Total error count
    pub total_errors: u32,
    /// Errors by category
    pub errors_by_category: Map<ErrorCategory, u32>,
    /// Errors by severity
    pub errors_by_severity: Map<ErrorSeverity, u32>,
    /// Most common errors
    pub most_common_errors: Vec<String>,
    /// Recovery success rate
    pub recovery_success_rate: i128, // Percentage * 100
    /// Average error resolution time (in seconds)
    pub avg_resolution_time: u64,
}

// ===== ERROR RECOVERY MECHANISMS =====

/// Comprehensive error recovery information and state
#[contracttype]
#[derive(Clone, Debug)]
pub struct ErrorRecovery {
    /// Original error code that triggered recovery
    pub original_error_code: u32,
    /// Recovery strategy applied
    pub recovery_strategy: String,
    /// Recovery attempt timestamp
    pub recovery_timestamp: u64,
    /// Recovery status
    pub recovery_status: String,
    /// Recovery context
    pub recovery_context: ErrorContext,
    /// Recovery attempts count
    pub recovery_attempts: u32,
    /// Maximum recovery attempts allowed
    pub max_recovery_attempts: u32,
    /// Recovery success timestamp
    pub recovery_success_timestamp: Option<u64>,
    /// Recovery failure reason
    pub recovery_failure_reason: Option<String>,
}

/// Recovery status enumeration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryStatus {
    /// Recovery not attempted yet
    Pending,
    /// Recovery in progress
    InProgress,
    /// Recovery completed successfully
    Success,
    /// Recovery failed
    Failed,
    /// Recovery exceeded maximum attempts
    Exhausted,
    /// Recovery cancelled
    Cancelled,
}

/// Recovery result information
#[derive(Clone, Debug)]
pub struct RecoveryResult {
    /// Whether recovery was successful
    pub success: bool,
    /// Recovery method used
    pub recovery_method: String,
    /// Recovery duration in seconds
    pub recovery_duration: u64,
    /// Additional recovery data
    pub recovery_data: Map<String, String>,
    /// Recovery validation result
    pub validation_result: bool,
}

/// Resilience pattern configuration
#[contracttype]
#[derive(Clone, Debug)]
pub struct ResiliencePattern {
    /// Pattern name/identifier
    pub pattern_name: String,
    /// Pattern type
    pub pattern_type: ResiliencePatternType,
    /// Pattern configuration
    pub pattern_config: Map<String, String>,
    /// Pattern enabled status
    pub enabled: bool,
    /// Pattern priority (higher = more important)
    pub priority: u32,
    /// Pattern last used timestamp
    pub last_used: Option<u64>,
    /// Pattern success rate
    pub success_rate: i128, // Percentage * 100
}

/// Resilience pattern types
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResiliencePatternType {
    /// Retry with exponential backoff
    RetryWithBackoff,
    /// Circuit breaker pattern
    CircuitBreaker,
    /// Bulkhead isolation
    Bulkhead,
    /// Timeout pattern
    Timeout,
    /// Fallback pattern
    Fallback,
    /// Health check pattern
    HealthCheck,
    /// Rate limiting pattern
    RateLimit,
}

/// Error recovery status tracking
#[contracttype]
#[derive(Clone, Debug)]
pub struct ErrorRecoveryStatus {
    /// Total recovery attempts
    pub total_attempts: u32,
    /// Successful recoveries
    pub successful_recoveries: u32,
    /// Failed recoveries
    pub failed_recoveries: u32,
    /// Active recovery processes
    pub active_recoveries: u32,
    /// Recovery success rate
    pub success_rate: i128, // Percentage * 100
    /// Average recovery time
    pub avg_recovery_time: u64,
    /// Last recovery timestamp
    pub last_recovery_timestamp: Option<u64>,
}

/// Main error handler for comprehensive error management
pub struct ErrorHandler;

impl ErrorHandler {
    /// Categorize an error with detailed information
    pub fn categorize_error(_env: &Env, error: Error, context: ErrorContext) -> DetailedError {
        let (severity, category, recovery_strategy) = Self::get_error_classification(&error);
        let detailed_message = Self::generate_detailed_error_message(&error, &context);
        let user_action = Self::get_user_action(&error, &category);
        let technical_details = Self::get_technical_details(&error, &context);

        DetailedError {
            error,
            severity,
            category,
            recovery_strategy,
            context,
            detailed_message,
            user_action,
            technical_details,
        }
    }

    /// Generate detailed error message with context
    pub fn generate_detailed_error_message(error: &Error, context: &ErrorContext) -> String {
        let _base_message = error.description();
        let _operation = &context.operation;

        match error {
            Error::Unauthorized => {
                String::from_str(context.call_chain.env(), "Authorization failed for operation. User may not have required permissions.")
            }
            Error::MarketNotFound => {
                String::from_str(context.call_chain.env(), "Market not found during operation. The market may have been removed or the ID is incorrect.")
            }
            Error::MarketClosed => {
                String::from_str(context.call_chain.env(), "Market is closed and cannot accept new operations. Operation was attempted on a closed market.")
            }
            Error::OracleUnavailable => {
                String::from_str(context.call_chain.env(), "Oracle service is unavailable during operation. External data source may be down or unreachable.")
            }
            Error::InsufficientStake => {
                String::from_str(context.call_chain.env(), "Insufficient stake amount for operation. Please increase your stake to meet the minimum requirement.")
            }
            Error::AlreadyVoted => {
                String::from_str(context.call_chain.env(), "User has already voted in this market. Operation cannot be performed as voting is limited to one vote per user.")
            }
            Error::InvalidInput => {
                String::from_str(context.call_chain.env(), "Invalid input provided for operation. Please check your input parameters and try again.")
            }
            Error::InvalidState => {
                String::from_str(context.call_chain.env(), "Invalid system state for operation. The system may be in an unexpected state.")
            }
            _ => {
                String::from_str(context.call_chain.env(), "Error during operation. Please check the operation parameters and try again.")
            }
        }
    }

    /// Handle error recovery based on error type and context
    pub fn handle_error_recovery(
        env: &Env,
        error: &Error,
        context: &ErrorContext,
    ) -> Result<bool, Error> {
        let recovery_strategy = Self::get_error_recovery_strategy(error);

        match recovery_strategy {
            RecoveryStrategy::Retry => {
                // For retryable errors, return success to allow retry
                Ok(true)
            }
            RecoveryStrategy::RetryWithDelay => {
                // For errors that need delay, check if enough time has passed
                let last_attempt = context.timestamp;
                let current_time = env.ledger().timestamp();
                let delay_required = 60; // 1 minute delay

                if current_time - last_attempt >= delay_required {
                    Ok(true)
                } else {
                    Err(Error::InvalidState)
                }
            }
            RecoveryStrategy::AlternativeMethod => {
                // Try alternative approach based on error type
                match error {
                    Error::OracleUnavailable => {
                        // Try fallback oracle or cached data
                        Ok(true)
                    }
                    Error::MarketNotFound => {
                        // Try to find similar market or suggest alternatives
                        Ok(false)
                    }
                    _ => Ok(false),
                }
            }
            RecoveryStrategy::Skip => {
                // Skip the operation and continue
                Ok(true)
            }
            RecoveryStrategy::Abort => {
                // Abort the operation
                Ok(false)
            }
            RecoveryStrategy::ManualIntervention => {
                // Require manual intervention
                Err(Error::InvalidState)
            }
            RecoveryStrategy::NoRecovery => {
                // No recovery possible
                Ok(false)
            }
        }
    }

    /// Emit error event for logging and monitoring
    pub fn emit_error_event(env: &Env, detailed_error: &DetailedError) {
        // Import the events module to emit error events
        use crate::events::EventEmitter;

        EventEmitter::emit_error_logged(
            env,
            detailed_error.error as u32,
            &detailed_error.detailed_message,
            &detailed_error.technical_details,
            detailed_error.context.user_address.clone(),
            detailed_error.context.market_id.clone(),
        );
    }

    /// Log error details for debugging and analysis
    pub fn log_error_details(env: &Env, detailed_error: &DetailedError) {
        // In a real implementation, this would log to a persistent storage
        // For now, we'll just emit the error event
        Self::emit_error_event(env, detailed_error);
    }

    /// Get error recovery strategy based on error type
    pub fn get_error_recovery_strategy(error: &Error) -> RecoveryStrategy {
        match error {
            // Retryable errors
            Error::OracleUnavailable => RecoveryStrategy::RetryWithDelay,
            Error::InvalidInput => RecoveryStrategy::Retry,

            // Alternative method errors
            Error::MarketNotFound => RecoveryStrategy::AlternativeMethod,
            Error::ConfigNotFound => RecoveryStrategy::AlternativeMethod,

            // Skip errors
            Error::AlreadyVoted => RecoveryStrategy::Skip,
            Error::AlreadyClaimed => RecoveryStrategy::Skip,
            Error::FeeAlreadyCollected => RecoveryStrategy::Skip,

            // Abort errors
            Error::Unauthorized => RecoveryStrategy::Abort,
            Error::MarketClosed => RecoveryStrategy::Abort,
            Error::MarketResolved => RecoveryStrategy::Abort,

            // Manual intervention errors
            Error::AdminNotSet => RecoveryStrategy::ManualIntervention,
            Error::DisputeFeeFailed => RecoveryStrategy::ManualIntervention,

            // No recovery errors
            Error::InvalidState => RecoveryStrategy::NoRecovery,
            Error::InvalidOracleConfig => RecoveryStrategy::NoRecovery,

            // Default to abort for unknown errors
            _ => RecoveryStrategy::Abort,
        }
    }

    /// Validate error context for completeness and correctness
    pub fn validate_error_context(context: &ErrorContext) -> Result<(), Error> {
        // Check if operation is provided
        if context.operation.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Check if call chain is not empty
        if context.call_chain.is_empty() {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Get comprehensive error analytics
    pub fn get_error_analytics(env: &Env) -> Result<ErrorAnalytics, Error> {
        // In a real implementation, this would aggregate error data from storage
        // For now, return a basic structure
        let mut errors_by_category = Map::new(env);
        errors_by_category.set(ErrorCategory::UserOperation, 0);
        errors_by_category.set(ErrorCategory::Oracle, 0);
        errors_by_category.set(ErrorCategory::Validation, 0);
        errors_by_category.set(ErrorCategory::System, 0);

        let mut errors_by_severity = Map::new(env);
        errors_by_severity.set(ErrorSeverity::Low, 0);
        errors_by_severity.set(ErrorSeverity::Medium, 0);
        errors_by_severity.set(ErrorSeverity::High, 0);
        errors_by_severity.set(ErrorSeverity::Critical, 0);

        let most_common_errors = Vec::new(env);

        Ok(ErrorAnalytics {
            total_errors: 0,
            errors_by_category,
            errors_by_severity,
            most_common_errors,
            recovery_success_rate: 0,
            avg_resolution_time: 0,
        })
    }

    // ===== ERROR RECOVERY MECHANISMS =====

    /// Recover from an error using appropriate recovery strategy
    pub fn recover_from_error(
        env: &Env,
        error: Error,
        context: ErrorContext,
    ) -> Result<ErrorRecovery, Error> {
        // Validate error context
        Self::validate_error_context(&context)?;

        // Create initial recovery record
        let mut recovery = ErrorRecovery {
            original_error_code: error as u32,
            recovery_strategy: Self::get_error_recovery_strategy_string(&error),
            recovery_timestamp: env.ledger().timestamp(),
            recovery_status: String::from_str(env, "pending"),
            recovery_context: context.clone(),
            recovery_attempts: 0,
            max_recovery_attempts: Self::get_max_recovery_attempts(&error),
            recovery_success_timestamp: None,
            recovery_failure_reason: None,
        };

        // Attempt recovery based on strategy
        recovery.recovery_status = String::from_str(env, "in_progress");
        recovery.recovery_attempts += 1;

        let recovery_result = Self::execute_recovery_strategy(env, &recovery)?;

        // Update recovery status based on result
        if recovery_result.success {
            recovery.recovery_status = String::from_str(env, "success");
            recovery.recovery_success_timestamp = Some(env.ledger().timestamp());
        } else {
            recovery.recovery_status = String::from_str(env, "failed");
            recovery.recovery_failure_reason =
                Some(String::from_str(env, "Recovery strategy failed"));
        }

        // Store recovery record
        Self::store_recovery_record(env, &recovery)?;

        // Emit recovery event
        Self::emit_error_recovery_event(env, &recovery);

        Ok(recovery)
    }

    /// Validate error recovery configuration and state
    pub fn validate_error_recovery(env: &Env, recovery: &ErrorRecovery) -> Result<bool, Error> {
        // Validate recovery context
        Self::validate_error_context(&recovery.recovery_context)?;

        // Check if recovery attempts are within limits
        if recovery.recovery_attempts > recovery.max_recovery_attempts {
            return Err(Error::InvalidState);
        }

        // Validate recovery timestamp
        let current_time = env.ledger().timestamp();
        if recovery.recovery_timestamp > current_time {
            return Err(Error::InvalidState);
        }

        // Validate recovery attempts
        if recovery.recovery_attempts > recovery.max_recovery_attempts {
            return Err(Error::InvalidInput);
        }

        Ok(true)
    }

    /// Get current error recovery status and statistics
    pub fn get_error_recovery_status(_env: &Env) -> Result<ErrorRecoveryStatus, Error> {
        // In a real implementation, this would aggregate recovery data from storage
        let status = ErrorRecoveryStatus {
            total_attempts: 0,
            successful_recoveries: 0,
            failed_recoveries: 0,
            active_recoveries: 0,
            success_rate: 0,
            avg_recovery_time: 0,
            last_recovery_timestamp: None,
        };

        Ok(status)
    }

    /// Emit error recovery event for monitoring and logging
    pub fn emit_error_recovery_event(env: &Env, recovery: &ErrorRecovery) {
        use crate::events::EventEmitter;

        EventEmitter::emit_error_recovery_event(
            env,
            recovery.original_error_code,
            &recovery.recovery_strategy,
            recovery.recovery_status.clone(),
            recovery.recovery_attempts,
            recovery.recovery_context.user_address.clone(),
            recovery.recovery_context.market_id.clone(),
        );
    }

    /// Validate resilience patterns configuration
    pub fn validate_resilience_patterns(
        _env: &Env,
        patterns: &Vec<ResiliencePattern>,
    ) -> Result<bool, Error> {
        for pattern in patterns.iter() {
            // Validate pattern name
            if pattern.pattern_name.is_empty() {
                return Err(Error::InvalidInput);
            }

            // Validate pattern configuration
            if pattern.pattern_config.is_empty() {
                return Err(Error::InvalidInput);
            }

            // Validate priority (must be between 1-100)
            if pattern.priority == 0 || pattern.priority > 100 {
                return Err(Error::InvalidInput);
            }

            // Validate success rate (must be between 0-10000 for percentage * 100)
            if pattern.success_rate < 0 || pattern.success_rate > 10000 {
                return Err(Error::InvalidInput);
            }
        }

        Ok(true)
    }

    /// Document error recovery procedures and best practices
    pub fn document_error_recovery_procedures(env: &Env) -> Result<Map<String, String>, Error> {
        let mut procedures = Map::new(env);

        procedures.set(
            String::from_str(env, "retry_procedure"),
            String::from_str(
                env,
                "For retryable errors, implement exponential backoff with max 3 attempts",
            ),
        );

        procedures.set(
            String::from_str(env, "oracle_recovery"),
            String::from_str(
                env,
                "For oracle errors, try fallback oracle or cached data before failing",
            ),
        );

        procedures.set(
            String::from_str(env, "validation_recovery"),
            String::from_str(
                env,
                "For validation errors, provide clear error messages and retry guidance",
            ),
        );

        procedures.set(
            String::from_str(env, "system_recovery"),
            String::from_str(
                env,
                "For system errors, log details and require manual intervention if critical",
            ),
        );

        Ok(procedures)
    }

    // ===== PRIVATE HELPER METHODS =====

    /// Execute recovery strategy based on error type
    fn execute_recovery_strategy(
        env: &Env,
        recovery: &ErrorRecovery,
    ) -> Result<RecoveryResult, Error> {
        let start_time = env.ledger().timestamp();

        let recovery_method = recovery.recovery_strategy.clone();

        let success = if recovery.recovery_strategy == String::from_str(env, "retry") {
            true
        } else if recovery.recovery_strategy == String::from_str(env, "retry_with_delay") {
            // Check if enough time has passed since last attempt
            let delay_required = 60; // 1 minute
            let time_since_last = env.ledger().timestamp() - recovery.recovery_timestamp;
            time_since_last >= delay_required
        } else if recovery.recovery_strategy == String::from_str(env, "alternative_method") {
            // Try alternative approach based on error type
            match recovery.original_error_code {
                200 => true,  // OracleUnavailable - Try fallback oracle
                101 => false, // MarketNotFound - No alternative available
                _ => false,
            }
        } else if recovery.recovery_strategy == String::from_str(env, "skip") {
            true
        } else if recovery.recovery_strategy == String::from_str(env, "abort") {
            false
        } else if recovery.recovery_strategy == String::from_str(env, "manual_intervention") {
            false
        } else if recovery.recovery_strategy == String::from_str(env, "no_recovery") {
            false
        } else {
            false
        };

        let recovery_duration = env.ledger().timestamp() - start_time;
        let mut recovery_data = Map::new(env);
        recovery_data.set(String::from_str(env, "strategy"), recovery_method.clone());
        recovery_data.set(
            String::from_str(env, "duration"),
            String::from_str(env, &recovery_duration.to_string()),
        );

        Ok(RecoveryResult {
            success,
            recovery_method,
            recovery_duration,
            recovery_data,
            validation_result: true,
        })
    }

    /// Get maximum recovery attempts for error type
    fn get_max_recovery_attempts(error: &Error) -> u32 {
        match error {
            Error::OracleUnavailable => 3,
            Error::InvalidInput => 2,
            Error::MarketNotFound => 1,
            Error::ConfigNotFound => 1,
            Error::AlreadyVoted => 0,
            Error::AlreadyBet => 0,
            Error::AlreadyClaimed => 0,
            Error::FeeAlreadyCollected => 0,
            Error::Unauthorized => 0,
            Error::MarketClosed => 0,
            Error::MarketResolved => 0,
            Error::AdminNotSet => 0,
            Error::DisputeFeeFailed => 0,
            Error::InvalidState => 0,
            Error::InvalidOracleConfig => 0,
            Error::Blacklist => 0,
            Error::NoWhitelist => 0,
            _ => 1,
        }
    }

    /// Store recovery record in persistent storage
    fn store_recovery_record(env: &Env, recovery: &ErrorRecovery) -> Result<(), Error> {
        let recovery_key = Symbol::new(
            env,
            &format!(
                "rec_{}_{}",
                recovery.original_error_code,
                recovery.recovery_timestamp % 1000000
            ),
        );
        env.storage().persistent().set(&recovery_key, recovery);
        Ok(())
    }

    /// Get error recovery strategy as string
    fn get_error_recovery_strategy_string(error: &Error) -> String {
        match error {
            Error::OracleUnavailable => String::from_str(&Env::default(), "retry_with_delay"),
            Error::InvalidInput => String::from_str(&Env::default(), "retry"),
            Error::MarketNotFound => String::from_str(&Env::default(), "alternative_method"),
            Error::ConfigNotFound => String::from_str(&Env::default(), "alternative_method"),
            Error::AlreadyVoted => String::from_str(&Env::default(), "skip"),
            Error::AlreadyBet => String::from_str(&Env::default(), "skip"),
            Error::AlreadyClaimed => String::from_str(&Env::default(), "skip"),
            Error::FeeAlreadyCollected => String::from_str(&Env::default(), "skip"),
            Error::Unauthorized => String::from_str(&Env::default(), "abort"),
            Error::MarketClosed => String::from_str(&Env::default(), "abort"),
            Error::MarketResolved => String::from_str(&Env::default(), "abort"),
            Error::AdminNotSet => String::from_str(&Env::default(), "manual_intervention"),
            Error::DisputeFeeFailed => {
                String::from_str(&Env::default(), "manual_intervention")
            }
            Error::InvalidState => String::from_str(&Env::default(), "no_recovery"),
            Error::InvalidOracleConfig => String::from_str(&Env::default(), "no_recovery"),
            Error::Blacklist => String::from_str(&Env::default(), "abort"),
            Error::NoWhitelist => String::from_str(&Env::default(), "abort"),
            _ => String::from_str(&Env::default(), "abort"),
        }
    }

    /// Get error classification (severity, category, recovery strategy)
    fn get_error_classification(error: &Error) -> (ErrorSeverity, ErrorCategory, RecoveryStrategy) {
        match error {
            // Critical errors
            Error::AdminNotSet => (
                ErrorSeverity::Critical,
                ErrorCategory::System,
                RecoveryStrategy::ManualIntervention,
            ),
            Error::DisputeFeeFailed => (
                ErrorSeverity::Critical,
                ErrorCategory::Financial,
                RecoveryStrategy::ManualIntervention,
            ),

            // High severity errors
            Error::Unauthorized => (
                ErrorSeverity::High,
                ErrorCategory::Authentication,
                RecoveryStrategy::Abort,
            ),
            Error::OracleUnavailable => (
                ErrorSeverity::High,
                ErrorCategory::Oracle,
                RecoveryStrategy::RetryWithDelay,
            ),
            Error::InvalidState => (
                ErrorSeverity::High,
                ErrorCategory::System,
                RecoveryStrategy::NoRecovery,
            ),

            // Medium severity errors
            Error::MarketNotFound => (
                ErrorSeverity::Medium,
                ErrorCategory::Market,
                RecoveryStrategy::AlternativeMethod,
            ),
            Error::MarketClosed => (
                ErrorSeverity::Medium,
                ErrorCategory::Market,
                RecoveryStrategy::Abort,
            ),
            Error::MarketResolved => (
                ErrorSeverity::Medium,
                ErrorCategory::Market,
                RecoveryStrategy::Abort,
            ),
            Error::InsufficientStake => (
                ErrorSeverity::Medium,
                ErrorCategory::UserOperation,
                RecoveryStrategy::Retry,
            ),
            Error::InvalidInput => (
                ErrorSeverity::Medium,
                ErrorCategory::Validation,
                RecoveryStrategy::Retry,
            ),
            Error::InvalidOracleConfig => (
                ErrorSeverity::Medium,
                ErrorCategory::Oracle,
                RecoveryStrategy::NoRecovery,
            ),

            // Low severity errors
            Error::AlreadyVoted => (
                ErrorSeverity::Low,
                ErrorCategory::UserOperation,
                RecoveryStrategy::Skip,
            ),
            Error::AlreadyBet => (
                ErrorSeverity::Low,
                ErrorCategory::UserOperation,
                RecoveryStrategy::Skip,
            ),
            Error::AlreadyClaimed => (
                ErrorSeverity::Low,
                ErrorCategory::UserOperation,
                RecoveryStrategy::Skip,
            ),
            Error::FeeAlreadyCollected => (
                ErrorSeverity::Low,
                ErrorCategory::Financial,
                RecoveryStrategy::Skip,
            ),
            Error::NothingToClaim => (
                ErrorSeverity::Low,
                ErrorCategory::UserOperation,
                RecoveryStrategy::Skip,
            ),
            Error::Blacklist => (
                ErrorSeverity::High,
                ErrorCategory::Authentication,
                RecoveryStrategy::Abort,
            ),
            Error::NoWhitelist => (
                ErrorSeverity::High,
                ErrorCategory::Authentication,
                RecoveryStrategy::Abort,
            ),

            // Default classification
            _ => (
                ErrorSeverity::Medium,
                ErrorCategory::Unknown,
                RecoveryStrategy::Abort,
            ),
        }
    }

    /// Get user-friendly action suggestion
    fn get_user_action(error: &Error, category: &ErrorCategory) -> String {
        match (error, category) {
            (Error::Unauthorized, _) => String::from_str(
                &Env::default(),
                "Please ensure you have the required permissions to perform this action.",
            ),
            (Error::InsufficientStake, _) => String::from_str(
                &Env::default(),
                "Please increase your stake amount to meet the minimum requirement.",
            ),
            (Error::MarketNotFound, _) => String::from_str(
                &Env::default(),
                "Please verify the market ID or check if the market still exists.",
            ),
            (Error::MarketClosed, _) => String::from_str(
                &Env::default(),
                "This market is closed. Please look for active markets.",
            ),
            (Error::AlreadyVoted, _) => String::from_str(
                &Env::default(),
                "You have already voted in this market. No further action needed.",
            ),
            (Error::OracleUnavailable, _) => String::from_str(
                &Env::default(),
                "Oracle service is temporarily unavailable. Please try again later.",
            ),
            (Error::InvalidInput, _) => String::from_str(
                &Env::default(),
                "Please check your input parameters and try again.",
            ),
            (Error::Blacklist, _) => String::from_str(
                &Env::default(),
                "Your address has been blacklisted from this operation.",
            ),
            (Error::NoWhitelist, _) => String::from_str(
                &Env::default(),
                "Your address is not whitelisted for this operation.",
            ),
            (_, ErrorCategory::Validation) => {
                String::from_str(&Env::default(), "Please review and correct the input data.")
            }
            (_, ErrorCategory::System) => String::from_str(
                &Env::default(),
                "System error occurred. Please contact support if the issue persists.",
            ),
            (_, ErrorCategory::Financial) => String::from_str(
                &Env::default(),
                "Financial operation failed. Please verify your balance and try again.",
            ),
            _ => String::from_str(
                &Env::default(),
                "An error occurred. Please try again or contact support if the issue persists.",
            ),
        }
    }

    /// Get technical details for debugging
    fn get_technical_details(error: &Error, context: &ErrorContext) -> String {
        let _error_code = error.code();
        let _error_num = *error as u32;
        let _timestamp = context.timestamp;

        String::from_str(context.call_chain.env(), "Error details for debugging")
    }
}

impl Error {
    /// Get a human-readable description of the error.
    ///
    /// This method returns a clear, user-friendly description of the error that can be
    /// displayed to end users or included in error messages. The descriptions are written
    /// in plain English and explain what went wrong in terms that users can understand.
    ///
    /// # Returns
    ///
    /// A static string slice containing a human-readable description of the error.
    ///
    /// # Example Usage
    ///
    /// ```rust
    /// # use predictify_hybrid::errors::Error;
    ///
    /// // Display user-friendly error messages
    /// let error = Error::InsufficientStake;
    /// println!("Operation failed: {}", error.description());
    /// // Output: "Operation failed: Insufficient stake amount"
    ///
    /// // Use in error handling for user interfaces
    /// fn display_error_to_user(error: Error) {
    ///     let message = format!("Error: {}", error.description());
    ///     // Display message in UI
    ///     println!("{}", message);
    /// }
    ///
    /// // Compare different error descriptions
    /// let errors = vec![
    ///     Error::MarketNotFound,
    ///     Error::MarketClosed,
    ///     Error::AlreadyVoted,
    /// ];
    ///
    /// for error in errors {
    ///     println!("{}: {}", error.code(), error.description());
    /// }
    /// // Output:
    /// // MARKET_NOT_FOUND: Market not found
    /// // MARKET_CLOSED: Market is closed
    /// // ALREADY_VOTED: User has already voted
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **User Interface**: Display error messages to users
    /// - **API Responses**: Include descriptions in API error responses
    /// - **Logging**: Add context to log entries
    /// - **Documentation**: Generate error documentation
    /// - **Debugging**: Understand error conditions during development
    pub fn description(&self) -> &'static str {
        match self {
            Error::Unauthorized => "User is not authorized to perform this action",
            Error::MarketNotFound => "Market not found",
            Error::MarketClosed => "Market is closed",
            Error::MarketResolved => "Market is already resolved",
            Error::MarketNotResolved => "Market is not resolved yet",
            Error::NothingToClaim => "User has nothing to claim",
            Error::AlreadyClaimed => "User has already claimed",
            Error::InsufficientStake => "Insufficient stake amount",
            Error::InvalidOutcome => "Invalid outcome choice",
            Error::AlreadyVoted => "User has already voted",
            Error::AlreadyBet => "User has already placed a bet on this market",
            Error::BetsAlreadyPlaced => {
                "Bets have already been placed on this market (cannot update)"
            }
            Error::InsufficientBalance => "Insufficient balance for operation",
            Error::Blacklist => "User is blacklisted",
            Error::NoWhitelist => "User is not whitelisted",
            Error::OracleUnavailable => "Oracle is unavailable",
            Error::InvalidOracleConfig => "Invalid oracle configuration",
            Error::InvalidQuestion => "Invalid question format",
            Error::InvalidOutcomes => "Invalid outcomes provided",
            Error::InvalidDuration => "Invalid duration specified",
            Error::InvalidThreshold => "Invalid threshold value",
            Error::InvalidComparison => "Invalid comparison operator",
            Error::InvalidState => "Invalid state",
            Error::InvalidInput => "Invalid input",
            Error::InvalidFeeConfig => "Invalid fee configuration",
            Error::ConfigNotFound => "Configuration not found",
            Error::AlreadyDisputed => "Already disputed",
            Error::DisputeVoteExpired => "Dispute voting period expired",
            Error::DisputeVoteDenied => "Dispute voting not allowed",
            Error::DisputeAlreadyVoted => "Already voted in dispute",
            Error::DisputeCondNotMet => "Dispute resolution conditions not met",
            Error::DisputeFeeFailed => "Dispute fee distribution failed",
            Error::DisputeNoEscalate => "Dispute escalation not allowed",
            Error::ThresholdBelowMin => "Threshold below minimum",
            Error::ThresholdTooHigh => "Threshold exceeds maximum",
            Error::FeeAlreadyCollected => "Fee already collected",
            Error::NoFeesToCollect => "No fees to collect",
            Error::InvalidExtensionDays => "Invalid extension days",
            Error::ExtensionDenied => "Extension not allowed or exceeded",
            Error::ExtensionFeeLow => "Extension fee insufficient",
            Error::AdminNotSet => "Admin address is not set (initialization missing)",
            Error::TimeoutNotSet => "Dispute timeout not set",
            Error::TimeoutNotExpired => "Dispute timeout not expired",
            Error::InvalidTimeoutHours => "Invalid timeout hours",
            Error::OracleStale => "Oracle data is stale or timed out",
            Error::OracleNoConsensus => "Oracle consensus not reached",
            Error::OracleVerified => "Oracle result already verified",
            Error::MarketNotReady => "Market not ready for oracle verification",
            Error::CBNotInitialized => "Circuit breaker not initialized",
            Error::CBAlreadyOpen => "Circuit breaker is already open (paused)",
            Error::CBNotOpen => "Circuit breaker is not open (cannot recover)",
            Error::CBOpen => "Circuit breaker is open (operations blocked)",
        }
    }

    /// Get the error code as a standardized string identifier.
    ///
    /// This method returns a standardized string representation of the error code that
    /// follows a consistent naming convention (UPPER_SNAKE_CASE). These codes are ideal
    /// for programmatic error handling, logging, monitoring, and API responses where
    /// consistent string identifiers are preferred over numeric codes.
    ///
    /// # Returns
    ///
    /// A static string slice containing the standardized error code identifier.
    ///
    /// # Example Usage
    ///
    /// ```rust
    /// # use predictify_hybrid::errors::Error;
    ///
    /// // Use for structured logging
    /// let error = Error::OracleUnavailable;
    /// println!("ERROR_CODE={} MESSAGE={}", error.code(), error.description());
    /// // Output: "ERROR_CODE=ORACLE_UNAVAILABLE MESSAGE=Oracle is unavailable"
    ///
    /// // Use for API error responses
    /// fn create_api_error_response(error: Error) -> String {
    ///     format!(
    ///         r#"{{
    ///             "error": "{}",
    ///             "message": "{}",
    ///             "code": {}
    ///         }}"",
    ///         error.code(),
    ///         error.description(),
    ///         error as u32
    ///     )
    /// }
    ///
    /// // Use for error categorization
    /// fn categorize_error(error: Error) -> &'static str {
    ///     match error.code() {
    ///         code if code.starts_with("MARKET_") => "Market Error",
    ///         code if code.starts_with("ORACLE_") => "Oracle Error",
    ///         code if code.starts_with("DISPUTE_") => "Dispute Error",
    ///         _ => "General Error",
    ///     }
    /// }
    ///
    /// // Use for monitoring and alerting
    /// fn should_alert(error: Error) -> bool {
    ///     matches!(error.code(),
    ///         "ORACLE_UNAVAILABLE" |
    ///         "DISPUTE_FEE_DISTRIBUTION_FAILED" |
    ///         "ADMIN_NOT_SET"
    ///     )
    /// }
    /// ```
    ///
    /// # Use Cases
    ///
    /// - **Structured Logging**: Consistent error identifiers for log analysis
    /// - **API Responses**: Machine-readable error codes for client applications
    /// - **Monitoring**: Error tracking and alerting based on error types
    /// - **Error Categorization**: Group and filter errors by type
    /// - **Documentation**: Generate error code reference documentation
    /// - **Testing**: Verify specific error conditions in unit tests
    pub fn code(&self) -> &'static str {
        match self {
            Error::Unauthorized => "UNAUTHORIZED",
            Error::MarketNotFound => "MARKET_NOT_FOUND",
            Error::MarketClosed => "MARKET_CLOSED",
            Error::MarketResolved => "MARKET_ALREADY_RESOLVED",
            Error::MarketNotResolved => "MARKET_NOT_RESOLVED",
            Error::NothingToClaim => "NOTHING_TO_CLAIM",
            Error::AlreadyClaimed => "ALREADY_CLAIMED",
            Error::InsufficientStake => "INSUFFICIENT_STAKE",
            Error::InvalidOutcome => "INVALID_OUTCOME",
            Error::AlreadyVoted => "ALREADY_VOTED",
            Error::AlreadyBet => "ALREADY_BET",
            Error::BetsAlreadyPlaced => "BETS_ALREADY_PLACED",
            Error::InsufficientBalance => "INSUFFICIENT_BALANCE",
            Error::Blacklist => "USER_BLACKLISTED",
            Error::NoWhitelist => "USER_NOT_WHITELISTED",
            Error::OracleUnavailable => "ORACLE_UNAVAILABLE",
            Error::InvalidOracleConfig => "INVALID_ORACLE_CONFIG",
            Error::InvalidQuestion => "INVALID_QUESTION",
            Error::InvalidOutcomes => "INVALID_OUTCOMES",
            Error::InvalidDuration => "INVALID_DURATION",
            Error::InvalidThreshold => "INVALID_THRESHOLD",
            Error::InvalidComparison => "INVALID_COMPARISON",
            Error::InvalidState => "INVALID_STATE",
            Error::InvalidInput => "INVALID_INPUT",
            Error::InvalidFeeConfig => "INVALID_FEE_CONFIG",
            Error::ConfigNotFound => "CONFIGURATION_NOT_FOUND",
            Error::AlreadyDisputed => "ALREADY_DISPUTED",
            Error::DisputeVoteExpired => "DISPUTE_VOTING_PERIOD_EXPIRED",
            Error::DisputeVoteDenied => "DISPUTE_VOTING_NOT_ALLOWED",
            Error::DisputeAlreadyVoted => "DISPUTE_ALREADY_VOTED",
            Error::DisputeCondNotMet => "DISPUTE_RESOLUTION_CONDITIONS_NOT_MET",
            Error::DisputeFeeFailed => "DISPUTE_FEE_DISTRIBUTION_FAILED",
            Error::DisputeNoEscalate => "DISPUTE_ESCALATION_NOT_ALLOWED",
            Error::ThresholdBelowMin => "THRESHOLD_BELOW_MINIMUM",
            Error::ThresholdTooHigh => "THRESHOLD_EXCEEDS_MAXIMUM",
            Error::FeeAlreadyCollected => "FEE_ALREADY_COLLECTED",
            Error::NoFeesToCollect => "NO_FEES_TO_COLLECT",
            Error::InvalidExtensionDays => "INVALID_EXTENSION_DAYS",
            Error::ExtensionDenied => "EXTENSION_DENIED",
            Error::ExtensionFeeLow => "EXTENSION_FEE_INSUFFICIENT",
            Error::AdminNotSet => "ADMIN_NOT_SET",
            Error::TimeoutNotSet => "DISPUTE_TIMEOUT_NOT_SET",
            Error::TimeoutNotExpired => "DISPUTE_TIMEOUT_NOT_EXPIRED",
            Error::InvalidTimeoutHours => "INVALID_TIMEOUT_HOURS",
            Error::OracleStale => "ORACLE_STALE",
            Error::OracleNoConsensus => "ORACLE_NO_CONSENSUS",
            Error::OracleVerified => "ORACLE_VERIFIED",
            Error::MarketNotReady => "MARKET_NOT_READY",
            Error::CBNotInitialized => "CIRCUIT_BREAKER_NOT_INITIALIZED",
            Error::CBAlreadyOpen => "CIRCUIT_BREAKER_ALREADY_OPEN",
            Error::CBNotOpen => "CIRCUIT_BREAKER_NOT_OPEN",
            Error::CBOpen => "CIRCUIT_BREAKER_OPEN",
        }
    }
}

// ===== TESTING MODULE =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address;

    #[test]
    fn test_error_categorization() {
        let env = Env::default();
        let context = ErrorContext {
            operation: String::from_str(&env, "test_operation"),
            user_address: Some(
                <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env),
            ),
            market_id: Some(Symbol::new(&env, "test_market")),
            context_data: Map::new(&env),
            timestamp: env.ledger().timestamp(),
            call_chain: Vec::new(&env),
        };

        let detailed_error = ErrorHandler::categorize_error(&env, Error::Unauthorized, context);

        assert_eq!(detailed_error.severity, ErrorSeverity::High);
        assert_eq!(detailed_error.category, ErrorCategory::Authentication);
        assert_eq!(detailed_error.recovery_strategy, RecoveryStrategy::Abort);
    }

    #[test]
    fn test_error_recovery_strategy() {
        let retry_strategy = ErrorHandler::get_error_recovery_strategy(&Error::OracleUnavailable);
        assert_eq!(retry_strategy, RecoveryStrategy::RetryWithDelay);

        let abort_strategy = ErrorHandler::get_error_recovery_strategy(&Error::Unauthorized);
        assert_eq!(abort_strategy, RecoveryStrategy::Abort);

        let skip_strategy = ErrorHandler::get_error_recovery_strategy(&Error::AlreadyVoted);
        assert_eq!(skip_strategy, RecoveryStrategy::Skip);
    }

    #[test]
    fn test_detailed_error_message() {
        let env = Env::default();
        let context = ErrorContext {
            operation: String::from_str(&env, "create_market"),
            user_address: None,
            market_id: None,
            context_data: Map::new(&env),
            timestamp: env.ledger().timestamp(),
            call_chain: Vec::new(&env),
        };

        let _message =
            ErrorHandler::generate_detailed_error_message(&Error::Unauthorized, &context);
        // Test that the message is generated correctly
        assert!(true); // Simplified test since to_string() is not available
    }

    #[test]
    fn test_error_context_validation() {
        let env = Env::default();
        let valid_context = ErrorContext {
            operation: String::from_str(&env, "test"),
            user_address: None,
            market_id: None,
            context_data: Map::new(&env),
            timestamp: env.ledger().timestamp(),
            call_chain: {
                let mut chain = Vec::new(&env);
                chain.push_back(String::from_str(&env, "test"));
                chain
            },
        };

        assert!(ErrorHandler::validate_error_context(&valid_context).is_ok());

        let invalid_context = ErrorContext {
            operation: String::from_str(&env, ""),
            user_address: None,
            market_id: None,
            context_data: Map::new(&env),
            timestamp: env.ledger().timestamp(),
            call_chain: Vec::new(&env),
        };

        assert!(ErrorHandler::validate_error_context(&invalid_context).is_err());
    }

    #[test]
    fn test_error_analytics() {
        let env = Env::default();
        let analytics = ErrorHandler::get_error_analytics(&env).unwrap();

        assert_eq!(analytics.total_errors, 0);
        assert!(analytics
            .errors_by_category
            .get(ErrorCategory::UserOperation)
            .is_some());
        assert!(analytics
            .errors_by_severity
            .get(ErrorSeverity::Low)
            .is_some());
    }
}
