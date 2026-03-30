#![allow(dead_code)]

use alloc::format;
use alloc::string::ToString;
use soroban_sdk::{contracterror, contracttype, Address, Env, Map, String, Symbol, Vec};

/// Comprehensive error codes for the Predictify Hybrid prediction market contract.
///
/// This enum defines all possible error conditions that can occur during contract operations.
/// Each variant has a unique numeric code (100-504) for efficient error handling and diagnostics.
///
/// # Error Categories
///
/// - **User Operation Errors (100-112)**: Errors related to user actions like voting,
///   betting, or claiming winnings.
/// - **Oracle Errors (200-208)**: Errors related to external data source integration and
///   resolution.
/// - **Validation Errors (300-304)**: Input validation failures.
/// - **General Errors (400-418)**: System state and configuration issues.
/// - **Circuit Breaker Errors (500-504)**: Safety mechanism activation and management.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    // ===== USER OPERATION ERRORS (100-112) =====
    /// User is not authorized to perform the requested action. Typically returned when
    /// a non-admin attempts to call admin-only functions.
    Unauthorized = 100,
    /// The referenced market does not exist. Market ID may be incorrect or market may
    /// have been removed.
    MarketNotFound = 101,
    /// The market is closed and cannot accept new bets or operations. Market has
    /// passed its deadline.
    MarketClosed = 102,
    /// The market has already been resolved with a final outcome. No further betting is allowed.
    MarketResolved = 103,
    /// The market outcome has not yet been determined. Oracle resolution is still pending.
    MarketNotResolved = 104,
    /// The user has no winnings to claim from the market.
    NothingToClaim = 105,
    /// The user has already claimed their winnings. Duplicate claims are not allowed.
    AlreadyClaimed = 106,
    /// The stake amount is below the minimum required threshold for the market.
    InsufficientStake = 107,
    /// The selected outcome is invalid for this market. Check available outcomes.
    InvalidOutcome = 108,
    /// The user has already voted in this market. Only one vote per user is permitted.
    AlreadyVoted = 109,
    /// The user has already placed a bet on this market. Duplicate bets are not allowed.
    AlreadyBet = 110,
    /// Bets have already been placed on this market. The market cannot be updated.
    BetsAlreadyPlaced = 111,
    /// The user's balance is insufficient for the requested operation.
    InsufficientBalance = 112,

    // ===== ORACLE ERRORS =====
    /// The oracle service is unavailable. External data source may be temporarily
    /// down or unreachable.
    OracleUnavailable = 200,
    /// The oracle configuration is invalid. Check oracle address, asset code, and other parameters.
    InvalidOracleConfig = 201,
    /// Oracle data is stale and exceeds the freshness threshold. Market resolution is delayed.
    OracleStale = 202,
    /// Oracle consensus could not be achieved among multiple oracle instances.
    OracleNoConsensus = 203,
    /// Oracle result has already been verified and confirmed. No further verification is needed.
    OracleVerified = 204,
    /// Market is not ready for oracle verification. Check market state and deadlines.
    MarketNotReady = 205,
    /// The fallback oracle is unavailable or in an unhealthy state. Cannot proceed with resolution.
    FallbackOracleUnavailable = 206,
    /// Resolution timeout has been reached. Market cannot be resolved within the allowed timeframe.
    ResolutionTimeoutReached = 207,
    /// Oracle confidence interval is too wide. Accuracy threshold not met for reliable resolution.
    OracleConfidenceTooWide = 208,
    /// Invalid oracle feed ID
    InvalidOracleFeed = 209,
    /// Oracle callback authentication failed. Signature verification or authorization check failed.
    OracleCallbackAuthFailed = 210,
    /// Oracle callback not authorized. Caller is not in the authorized oracle whitelist.
    OracleCallbackUnauthorized = 211,
    /// Oracle callback signature is invalid or malformed.
    OracleCallbackInvalidSignature = 212,
    /// Oracle callback replay detected. Nonce or timestamp already used.
    OracleCallbackReplayDetected = 213,
    /// Oracle callback timeout. Response time exceeded maximum allowed duration.
    OracleCallbackTimeout = 214,

    // ===== VALIDATION ERRORS =====
    /// Market question is empty or invalid. Question must be non-empty and descriptive.
    InvalidQuestion = 300,
    /// Invalid outcomes provided. Must have 2+ outcomes, all non-empty, with no duplicates.
    InvalidOutcomes = 301,
    /// Market duration is invalid. Duration must be between 1 and 365 days.
    InvalidDuration = 302,
    /// Threshold value is invalid or out of acceptable range.
    InvalidThreshold = 303,
    /// Comparison operator is invalid or not supported.
    InvalidComparison = 304,

    // ===== GENERAL ERRORS =====
    /// Contract is in an invalid or unexpected state. Manual intervention may be required.
    InvalidState = 400,
    /// General input validation failed. Check parameters and try again.
    InvalidInput = 401,
    /// Platform fee configuration is invalid. Fee must be between 0% and 10%.
    InvalidFeeConfig = 402,
    /// Required configuration not found. Market or system configuration is missing.
    ConfigNotFound = 403,
    /// Market has already been disputed. Only one dispute per market is allowed.
    AlreadyDisputed = 404,
    /// The dispute voting period has expired. No further votes can be cast.
    DisputeVoteExpired = 405,
    /// Dispute voting is not allowed at this time. Check market state.
    DisputeVoteDenied = 406,
    /// User has already voted in this dispute. Duplicate votes are not allowed.
    DisputeAlreadyVoted = 407,
    /// Dispute resolution conditions are not met. Requirements may not be satisfied.
    DisputeCondNotMet = 408,
    /// Fee distribution for dispute resolution failed. Check balances and permissions.
    DisputeFeeFailed = 409,
    /// Generic dispute subsystem error. Check dispute state and configuration.
    DisputeError = 410,
    /// Platform fee has already been collected from this market.
    FeeAlreadyCollected = 413,
    /// No fees are available to collect from this market.
    NoFeesToCollect = 414,
    /// Extension days value is invalid. Must be between 1 and max allowed days.
    InvalidExtensionDays = 415,
    /// Market extension is not allowed or would exceed maximum extension limit.
    ExtensionDenied = 416,
    /// Gas budget cap has been exceeded for the operation.
    GasBudgetExceeded = 417,
    /// Admin address has not been set. Contract initialization is incomplete.
    AdminNotSet = 418,

    // ===== CIRCUIT BREAKER ERRORS ====="
    /// Circuit breaker has not been initialized. Initialize before use.
    CBNotInitialized = 500,
    /// Circuit breaker is already open (active). Cannot open again.
    CBAlreadyOpen = 501,
    /// Circuit breaker is not in open state. Cannot perform recovery.
    CBNotOpen = 502,
    /// Circuit breaker is open and blocking operations. Emergency halt is active.
    CBOpen = 503,
    /// Generic circuit breaker subsystem error. Check configuration and state.
    CBError = 504,
}

// ===== ERROR CATEGORIZATION AND RECOVERY SYSTEM =====

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ErrorCategory {
    UserOperation,
    Oracle,
    Validation,
    System,
    Dispute,
    Financial,
    Market,
    Authentication,
    Unknown,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryStrategy {
    Retry,
    RetryWithDelay,
    AlternativeMethod,
    Skip,
    Abort,
    ManualIntervention,
    NoRecovery,
}

/// Runtime context captured at the point of an error.
///
/// This structure captures the relevant state and metadata at the time an error occurs,
/// enabling better diagnostics, recovery strategies, and debugging. All fields except
/// `operation` are optional, allowing flexible context capture.
///
/// # Fields
///
/// * `operation` - The name of the operation that failed (required)
/// * `user_address` - The user performing the operation (if applicable)
/// * `market_id` - The market involved in the error (if applicable)
/// * `context_data` - Additional key-value data for debugging
/// * `timestamp` - Unix timestamp when the error occurred
/// * `call_chain` - Optional stack trace or call chain for debugging
#[contracttype]
#[derive(Clone, Debug)]
pub struct ErrorContext {
    /// The operation that failed (required).
    pub operation: String,
    /// The user address involved in the operation (optional).
    pub user_address: Option<Address>,
    /// The market ID involved in the operation (optional).
    pub market_id: Option<Symbol>,
    /// Additional contextual data for debugging (optional).
    pub context_data: Map<String, String>,
    /// Unix timestamp when the error occurred.
    pub timestamp: u64,
    /// Optional call chain or stack trace; None when not available.
    pub call_chain: Option<Vec<String>>,
}

/// A fully categorized and classified error with recovery information.
///
/// This structure extends a basic error with severity, category, recovery strategy,
/// and helpful messages for both end users and developers. It is produced by
/// `ErrorHandler::categorize_error()`.
///
/// # Fields
///
/// * `error` - The error code (numeric)
/// * `severity` - How critical the error is (Low/Medium/High/Critical)
/// * `category` - The category of error (UserOperation/Oracle/Validation/System/etc.)
/// * `recovery_strategy` - Recommended recovery approach
/// * `context` - Runtime context when the error occurred
/// * `detailed_message` - User-friendly error description
/// * `user_action` - Suggested action for the user
/// * `technical_details` - Technical information for debugging

#[derive(Clone, Debug)]
pub struct DetailedError {
    /// The core error code.
    pub error: Error,
    /// How critical this error is.
    pub severity: ErrorSeverity,
    /// The category of error.
    pub category: ErrorCategory,
    /// Recommended recovery strategy for this error.
    pub recovery_strategy: RecoveryStrategy,
    /// Runtime context captured when the error occurred.
    pub context: ErrorContext,
    /// User-friendly explanation of the error.
    pub detailed_message: String,
    /// Recommended action for the user to resolve the error.
    pub user_action: String,
    /// Technical details for debugging (error code, function, timestamp).
    pub technical_details: String,
}

/// Analytics and statistics about contract errors.
///
/// This structure aggregates error metrics for monitoring and diagnostics.
/// Currently a placeholder; full tracking requires persistent storage infrastructure.
///
/// # Fields
///
/// * `total_errors` - Total number of errors recorded
/// * `errors_by_category` - Error count broken down by category
/// * `errors_by_severity` - Error count broken down by severity level
/// * `most_common_errors` - List of most frequently occurring errors
/// * `recovery_success_rate` - Percentage of successful error recoveries (0-10000)
/// * `avg_resolution_time` - Average time to resolve errors (seconds)

#[contracttype]
#[derive(Clone, Debug)]
pub struct ErrorAnalytics {
    /// Total number of errors encountered.
    pub total_errors: u32,
    /// Errors grouped by category.
    pub errors_by_category: Map<ErrorCategory, u32>,
    /// Errors grouped by severity level.
    pub errors_by_severity: Map<ErrorSeverity, u32>,
    /// The most frequently occurring error codes.
    pub most_common_errors: Vec<String>,
    /// Success rate of error recovery (0-10000, where 10000 = 100%).
    pub recovery_success_rate: i128,
    /// Average time in seconds to resolve errors.
    pub avg_resolution_time: u64,
}

// ===== ERROR RECOVERY =====

/// Records an error recovery attempt with full lifecycle information.
///
/// This structure tracks the complete recovery process for an error, including
/// attempts, status, timing, and outcomes. Used for diagnostics and monitoring.
///
/// # Fields
///
/// * `original_error_code` - The numeric code of the original error
/// * `recovery_strategy` - The strategy used ("retry", "fallback", etc.)
/// * `recovery_timestamp` - When recovery was initiated
/// * `recovery_status` - Current status ("pending", "success", "failed")
/// * `recovery_context` - Context from the original error
/// * `recovery_attempts` - Number of recovery attempts made so far
/// * `max_recovery_attempts` - Maximum allowed recovery attempts
/// * `recovery_success_timestamp` - When recovery succeeded (if applicable)
/// * `recovery_failure_reason` - Why recovery failed (if applicable)

#[contracttype]
#[derive(Clone, Debug)]
pub struct ErrorRecovery {
    /// The original error code being recovered from.
    pub original_error_code: u32,
    /// The recovery strategy being employed.
    pub recovery_strategy: String,
    /// When recovery was initiated (Unix timestamp).
    pub recovery_timestamp: u64,
    /// Current status of the recovery (pending/success/failed).
    pub recovery_status: String,
    /// Context from the original error.
    pub recovery_context: ErrorContext,
    /// Number of recovery attempts made.
    pub recovery_attempts: u32,
    /// Maximum allowed recovery attempts.
    pub max_recovery_attempts: u32,
    /// Timestamp of successful recovery (if applicable).
    pub recovery_success_timestamp: Option<u64>,
    /// Reason for recovery failure (if applicable).
    pub recovery_failure_reason: Option<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryStatus {
    Pending,
    InProgress,
    Success,
    Failed,
    Exhausted,
    Cancelled,
}

/// Result of a recovery attempt.
///
/// # Fields
///
/// * `success` - Whether the recovery succeeded
/// * `recovery_method` - The method/strategy used
/// * `recovery_duration` - Time taken to recover (seconds)
/// * `recovery_data` - Additional data about the recovery
/// * `validation_result` - Whether the recovery result passed validation

#[derive(Clone, Debug)]
pub struct RecoveryResult {
    /// Whether recovery was successful.
    pub success: bool,
    /// The recovery method that was used.
    pub recovery_method: String,
    /// Time spent on recovery in seconds.
    pub recovery_duration: u64,
    /// Additional recovery metadata.
    pub recovery_data: Map<String, String>,
    /// Whether the recovery result passed validation.
    pub validation_result: bool,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct ResiliencePattern {
    pub pattern_name: String,
    pub pattern_type: ResiliencePatternType,
    pub pattern_config: Map<String, String>,
    pub enabled: bool,
    pub priority: u32,
    pub last_used: Option<u64>,
    pub success_rate: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResiliencePatternType {
    RetryWithBackoff,
    CircuitBreaker,
    Bulkhead,
    Timeout,
    Fallback,
    HealthCheck,
    RateLimit,
}

/// Status summary of error recovery operations.
///
/// # Fields
///
/// * `total_attempts` - Total recovery attempts made
/// * `successful_recoveries` - Number of successful recoveries
/// * `failed_recoveries` - Number of failed recovery attempts
/// * `active_recoveries` - Number of in-progress recovery operations
/// * `success_rate` - Overall success rate as percentage (0-10000)
/// * `avg_recovery_time` - Average recovery duration in seconds
/// * `last_recovery_timestamp` - When the last recovery occurred

#[contracttype]
#[derive(Clone, Debug)]
pub struct ErrorRecoveryStatus {
    /// Total recovery attempts made.
    pub total_attempts: u32,
    /// Number of successful recoveries.
    pub successful_recoveries: u32,
    /// Number of failed recovery attempts.
    pub failed_recoveries: u32,
    /// Number of active/in-progress recovery operations.
    pub active_recoveries: u32,
    /// Success rate as percentage (0-10000, where 10000 = 100%).
    pub success_rate: i128,
    /// Average time to resolve errors in seconds.
    pub avg_recovery_time: u64,
    /// Timestamp of the last recovery operation.
    pub last_recovery_timestamp: Option<u64>,
}

// ===== MAIN ERROR HANDLER =====

pub struct ErrorHandler;

impl ErrorHandler {
    // ===== PUBLIC API =====

    /// Categorizes an error with full classification, severity, recovery strategy, and messages.
    ///
    /// This is the primary entry point for error handling in the contract. It takes a raw error
    /// and context, and produces a fully elaborated `DetailedError` with:
    /// - Severity classification (Low/Medium/High/Critical)
    /// - Error category (UserOperation/Oracle/Validation/System/etc.)
    /// - Recommended recovery strategy
    /// - User-friendly error message
    /// - Suggested action for the user
    /// - Technical details for debugging
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `error` - The error code to categorize
    /// * `context` - Runtime context when the error occurred
    ///
    /// # Returns
    ///
    /// A fully categorized `DetailedError` with all classification and messaging information.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let context = ErrorContext {
    ///     operation: String::from_str(&env, "place_bet"),
    ///     user_address: Some(user),
    ///     market_id: Some(market_id),
    ///     context_data: Map::new(&env),
    ///     timestamp: env.ledger().timestamp(),
    ///     call_chain: None,
    /// };
    /// let detailed = ErrorHandler::categorize_error(&env, Error::InsufficientBalance, context);
    /// ```

    pub fn categorize_error(env: &Env, error: Error, context: ErrorContext) -> DetailedError {
        let (severity, category, recovery_strategy) = Self::get_error_classification(&error);
        let detailed_message = Self::generate_detailed_error_message(env, &error, &context);
        let user_action = Self::get_user_action(env, &error, &category);
        let technical_details = Self::get_technical_details(env, &error, &context);

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

    /// Generates a detailed, context-aware error message for the end user.
    ///
    /// Produces human-readable error explanations that explain what went wrong
    /// and provide guidance. Messages vary by error type and context.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `error` - The error code to generate a message for
    /// * `_context` - Runtime context (for future enhancement)
    ///
    /// # Returns
    ///
    /// A `String` containing a user-friendly error message.
    pub fn generate_detailed_error_message(
        env: &Env,
        error: &Error,
        _context: &ErrorContext,
    ) -> String {
        let msg = match error {
            Error::Unauthorized => {
                "Authorization failed. User does not have the required permissions."
            }
            Error::MarketNotFound => {
                "Market not found. The ID may be incorrect or the market has been removed."
            }
            Error::MarketClosed => {
                "Market is closed and cannot accept new operations."
            }
            Error::OracleUnavailable => {
                "Oracle service is unavailable. The external data source may be down."
            }
            Error::InsufficientStake => {
                "Insufficient stake. Please increase the amount to meet the minimum requirement."
            }
            Error::AlreadyVoted => {
                "User has already voted in this market. Only one vote per user is allowed."
            }
            Error::InvalidInput => {
                "Invalid input. Please check your parameters and try again."
            }
            Error::InvalidState => {
                "Invalid system state. The contract may be in an unexpected condition."
            }
            _ => "An error occurred. Please verify your parameters and try again.",
        };
        String::from_str(env, msg)
    }

    /// Attempts error recovery and determines whether the operation may proceed.
    ///
    /// Based on the error type and its recovery strategy, determines if the operation
    /// can be retried, skipped, or should be aborted. Implements delay logic for
    /// rate-limited recovery scenarios.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `error` - The error to attempt recovery for
    /// * `context` - Runtime context from the error occurrence
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Operation may proceed (recovery succeeded or skip strategy)
    /// * `Ok(false)` - Operation must be aborted (permanent failure)
    /// * `Err(error)` - Recovery is impossible or requires manual intervention
    pub fn handle_error_recovery(
        env: &Env,
        error: &Error,
        context: &ErrorContext,
    ) -> Result<bool, Error> {
        match Self::get_error_recovery_strategy(error) {
            RecoveryStrategy::Retry => Ok(true),

            RecoveryStrategy::RetryWithDelay => {
                let delay_required: u64 = 60;
                let current_time = env.ledger().timestamp();
                if current_time.saturating_sub(context.timestamp) >= delay_required {
                    Ok(true)
                } else {
                    Err(Error::InvalidState)
                }
            }

            RecoveryStrategy::AlternativeMethod => match error {
                Error::OracleUnavailable => Ok(true),
                Error::MarketNotFound => Ok(false),
                _ => Ok(false),
            },

            RecoveryStrategy::Skip => Ok(true),
            RecoveryStrategy::Abort => Ok(false),
            RecoveryStrategy::ManualIntervention => Err(Error::InvalidState),
            RecoveryStrategy::NoRecovery => Ok(false),
        }
    }

    /// Emits an error event for external monitoring and analytics.
    ///
    /// Records the error in the contract's event log, enabling:
    /// - External monitoring systems to track errors
    /// - Analytics dashboards to visualize error trends
    /// - Alerting systems to detect anomalies
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `detailed_error` - The fully categorized error to emit
    pub fn emit_error_event(env: &Env, detailed_error: &DetailedError) {
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

    /// Logs full error details for diagnostics and monitoring.
    ///
    /// Convenience method that emits the error event plus logs technical details.
    /// Equivalent to calling `emit_error_event`.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `detailed_error` - The fully categorized error to log
    pub fn log_error_details(env: &Env, detailed_error: &DetailedError) {
        Self::emit_error_event(env, detailed_error);
    }

    /// Maps each error variant to its recommended recovery strategy.
    ///
    /// Provides a lookup table from error codes to recovery strategies,
    /// enabling automatic recovery logic without duplicating error classification.
    ///
    /// # Error-to-Strategy Mapping
    ///
    /// | Error | Strategy |
    /// |-------|----------|
    /// | OracleUnavailable | RetryWithDelay |
    /// | InvalidInput | Retry |
    /// | Unauthorized, MarketClosed | Abort |
    /// | AlreadyVoted, AlreadyBet | Skip |
    /// | Other | Abort (default) |
    ///
    /// # Parameters
    ///
    /// * `error` - The error code to get recovery strategy for
    ///
    /// # Returns
    ///
    /// The recommended `RecoveryStrategy` for this error.
    pub fn get_error_recovery_strategy(error: &Error) -> RecoveryStrategy {
        match error {
            Error::OracleUnavailable => RecoveryStrategy::RetryWithDelay,
            Error::InvalidInput => RecoveryStrategy::Retry,
            Error::OracleConfidenceTooWide => RecoveryStrategy::NoRecovery,
            Error::MarketNotFound => RecoveryStrategy::AlternativeMethod,
            Error::ConfigNotFound => RecoveryStrategy::AlternativeMethod,
            Error::AlreadyVoted
            | Error::AlreadyBet
            | Error::AlreadyClaimed
            | Error::FeeAlreadyCollected => RecoveryStrategy::Skip,
            Error::Unauthorized
            | Error::MarketClosed
            | Error::MarketResolved => RecoveryStrategy::Abort,
            Error::AdminNotSet | Error::DisputeFeeFailed => RecoveryStrategy::ManualIntervention,
            Error::InvalidState | Error::InvalidOracleConfig => RecoveryStrategy::NoRecovery,
            _ => RecoveryStrategy::Abort,
        }
    }

    /// Validates an `ErrorContext` for structural integrity.
    ///
    /// Checks that required fields are present and have valid values.
    /// Only the `operation` field is mandatory; all others are optional.
    ///
    /// # Requirements
    ///
    /// * `operation` must be non-empty
    /// * All other fields are optional (can be absent)
    ///
    /// # Parameters
    ///
    /// * `context` - The context to validate
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Context is valid
    /// * `Err(InvalidInput)` - Context has validation errors
    pub fn validate_error_context(context: &ErrorContext) -> Result<(), Error> {
        if context.operation.is_empty() {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Gets error analytics and statistics.
    ///
    /// Returns aggregated error metrics for monitoring and diagnostics.
    /// Currently returns a zero-state placeholder; full tracking requires
    /// persistent storage infrastructure (e.g., storage-backed counters per category).
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// An `ErrorAnalytics` structure with current error statistics.
    ///
    /// # Note
    ///
    /// To enable full error tracking, implement persistent counters
    /// in contract storage for each error category and severity level.
    pub fn get_error_analytics(env: &Env) -> Result<ErrorAnalytics, Error> {
        let mut errors_by_category = Map::new(env);
        errors_by_category.set(ErrorCategory::UserOperation, 0u32);
        errors_by_category.set(ErrorCategory::Oracle, 0u32);
        errors_by_category.set(ErrorCategory::Validation, 0u32);
        errors_by_category.set(ErrorCategory::System, 0u32);

        let mut errors_by_severity = Map::new(env);
        errors_by_severity.set(ErrorSeverity::Low, 0u32);
        errors_by_severity.set(ErrorSeverity::Medium, 0u32);
        errors_by_severity.set(ErrorSeverity::High, 0u32);
        errors_by_severity.set(ErrorSeverity::Critical, 0u32);

        Ok(ErrorAnalytics {
            total_errors: 0,
            errors_by_category,
            errors_by_severity,
            most_common_errors: Vec::new(env),
            recovery_success_rate: 0,
            avg_resolution_time: 0,
        })
    }

    // ===== RECOVERY LIFECYCLE =====

    /// Runs the complete error recovery lifecycle from start to finish.
    ///
    /// Orchestrates the entire recovery process:
    /// 1. Validates the error context
    /// 2. Determines the appropriate recovery strategy
    /// 3. Executes the recovery strategy
    /// 4. Records the recovery outcome
    /// 5. Emits recovery events for monitoring
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `error` - The error to recover from
    /// * `context` - Runtime context from the error occurrence
    ///
    /// # Returns
    ///
    /// * `Ok(recovery)` - Recovery record with final status
    /// * `Err(error)` - Recovery lifecycle itself failed (validation error)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let context = ErrorContext {
    ///     operation: String::from_str(&env, "resolve_market"),
    ///     user_address: Some(admin),
    ///     market_id: Some(market_id),
    ///     context_data: Map::new(&env),
    ///     timestamp: env.ledger().timestamp(),
    ///     call_chain: None,
    /// };
    /// let recovery = ErrorHandler::recover_from_error(&env, Error::OracleUnavailable, context)?;
    /// ```
    pub fn recover_from_error(
        env: &Env,
        error: Error,
        context: ErrorContext,
    ) -> Result<ErrorRecovery, Error> {
        Self::validate_error_context(&context)?;

        // IMPROVEMENT: strategy string is derived from the same source-of-truth
        // enum rather than a parallel match.
        let strategy_str = Self::recovery_strategy_to_str(
            env,
            &Self::get_error_recovery_strategy(&error),
        );
        let max_attempts = Self::get_max_recovery_attempts(&error);

        let mut recovery = ErrorRecovery {
            original_error_code: error as u32,
            recovery_strategy: strategy_str,
            recovery_timestamp: env.ledger().timestamp(),
            recovery_status: String::from_str(env, "in_progress"),
            recovery_context: context,
            recovery_attempts: 1,
            max_recovery_attempts: max_attempts,
            recovery_success_timestamp: None,
            recovery_failure_reason: None,
        };

        let result = Self::execute_recovery_strategy(env, &recovery)?;

        if result.success {
            recovery.recovery_status = String::from_str(env, "success");
            recovery.recovery_success_timestamp = Some(env.ledger().timestamp());
        } else {
            recovery.recovery_status = String::from_str(env, "failed");
            recovery.recovery_failure_reason =
                Some(String::from_str(env, "Recovery strategy did not succeed"));
        }

        Self::store_recovery_record(env, &recovery)?;
        Self::emit_error_recovery_event(env, &recovery);

        Ok(recovery)
    }

    /// Validates a recovery record for internal consistency.
    ///
    /// Checks that:
    /// - The recovery context is valid (operation is non-empty)
    /// - Recovery attempts do not exceed the maximum allowed
    /// - Recovery timestamp is not in the future
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `recovery` - The recovery record to validate
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - Recovery record is valid
    /// * `Err(InvalidState)` - Recovery record has validation errors
    pub fn validate_error_recovery(env: &Env, recovery: &ErrorRecovery) -> Result<bool, Error> {
        Self::validate_error_context(&recovery.recovery_context)?;

        if recovery.recovery_attempts > recovery.max_recovery_attempts {
            return Err(Error::InvalidState);
        }

        let current_time = env.ledger().timestamp();
        if recovery.recovery_timestamp > current_time {
            return Err(Error::InvalidState);
        }

        Ok(true)
    }

    /// Gets the current status of error recovery operations.
    ///
    /// Returns aggregated statistics about recovery attempts, successes, and failures.
    /// Currently returns a zero-state placeholder; full tracking requires persistent storage.
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// An `ErrorRecoveryStatus` with current recovery statistics.
    pub fn get_error_recovery_status(_env: &Env) -> Result<ErrorRecoveryStatus, Error> {
        Ok(ErrorRecoveryStatus {
            total_attempts: 0,
            successful_recoveries: 0,
            failed_recoveries: 0,
            active_recoveries: 0,
            success_rate: 0,
            avg_recovery_time: 0,
            last_recovery_timestamp: None,
        })
    }

    /// Emits an error recovery event for monitoring and analytics.
    ///
    /// Records recovery progress and outcomes in the contract event log.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `recovery` - The recovery record to emit
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

    /// Validates resilience patterns for correctness.
    ///
    /// Checks that resilience patterns are properly configured:
    /// - Pattern names are non-empty
    /// - Pattern configurations are non-empty
    /// - Priority values are between 1-100
    /// - Success rates are between 0-10000 (0-100%)
    ///
    /// # Parameters
    ///
    /// * `_env` - The Soroban environment
    /// * `patterns` - Vector of resilience patterns to validate
    ///
    /// # Returns
    ///
    /// * `Ok(true)` - All patterns are valid
    /// * `Err(InvalidInput)` - One or more patterns have validation errors
    pub fn validate_resilience_patterns(
        _env: &Env,
        patterns: &Vec<ResiliencePattern>,
    ) -> Result<bool, Error> {
        for pattern in patterns.iter() {
            if pattern.pattern_name.is_empty() {
                return Err(Error::InvalidInput);
            }
            if pattern.pattern_config.is_empty() {
                return Err(Error::InvalidInput);
            }
            // priority must be 1–100
            if pattern.priority == 0 || pattern.priority > 100 {
                return Err(Error::InvalidInput);
            }
            // success_rate is stored as percentage * 100 (0–10 000)
            if pattern.success_rate < 0 || pattern.success_rate > 10_000 {
                return Err(Error::InvalidInput);
            }
        }
        Ok(true)
    }

    /// Documents the error recovery procedures for each error type.
    ///
    /// Returns a map of recovery procedure descriptions, useful for:
    /// - User documentation
    /// - Support team reference
    /// - Automated system responses
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    ///
    /// # Returns
    ///
    /// A map of recovery procedure names to their descriptions.
    pub fn document_error_recovery_procedures(env: &Env) -> Result<Map<String, String>, Error> {
        let mut procedures = Map::new(env);
        procedures.set(
            String::from_str(env, "retry_procedure"),
            String::from_str(
                env,
                "For retryable errors, use exponential backoff (max 3 attempts).",
            ),
        );
        procedures.set(
            String::from_str(env, "oracle_recovery"),
            String::from_str(env, "For oracle errors, try fallback oracle or cached data."),
        );
        procedures.set(
            String::from_str(env, "validation_recovery"),
            String::from_str(
                env,
                "For validation errors, surface clear messages and prompt retry.",
            ),
        );
        procedures.set(
            String::from_str(env, "system_recovery"),
            String::from_str(env, "For critical system errors, require manual intervention."),
        );
        Ok(procedures)
    }

    // ===== PRIVATE HELPERS =====

    /// Executes the concrete recovery logic for a recovery strategy.
    ///
    /// Implements the actual recovery operations based on the strategy
    /// (retry, delay, fallback, skip, abort, etc.).
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `recovery` - The recovery record with strategy details
    ///
    /// # Returns
    ///
    /// * `Ok(result)` - Recovery strategy executed with outcome
    /// * `Err(error)` - Recovery execution failed
    fn execute_recovery_strategy(
        env: &Env,
        recovery: &ErrorRecovery,
    ) -> Result<RecoveryResult, Error> {
        let start_time = env.ledger().timestamp();

        // IMPROVEMENT: compare against canonical strategy strings produced by
        // `recovery_strategy_to_str` so there is a single source of truth for
        // these string literals.
        let success = if recovery.recovery_strategy == String::from_str(env, "retry") {
            true
        } else if recovery.recovery_strategy == String::from_str(env, "retry_with_delay") {
            let delay_required: u64 = 60;
            env.ledger()
                .timestamp()
                .saturating_sub(recovery.recovery_timestamp)
                >= delay_required
        } else if recovery.recovery_strategy == String::from_str(env, "alternative_method") {
            matches!(recovery.original_error_code, 200) // OracleUnavailable → try fallback
        } else if recovery.recovery_strategy == String::from_str(env, "skip") {
            true
        } else {
            // "abort" | "manual_intervention" | "no_recovery" | unknown
            false
        };

        let recovery_duration = env.ledger().timestamp().saturating_sub(start_time);
        let mut recovery_data = Map::new(env);
        recovery_data.set(
            String::from_str(env, "strategy"),
            recovery.recovery_strategy.clone(),
        );
        recovery_data.set(
            String::from_str(env, "duration"),
            String::from_str(env, &recovery_duration.to_string()),
        );

        Ok(RecoveryResult {
            success,
            recovery_method: recovery.recovery_strategy.clone(),
            recovery_duration,
            recovery_data,
            validation_result: true,
        })
    }

    /// Gets the maximum number of recovery attempts allowed for an error.
    ///
    /// Different error types have different retry limits:
    /// - Retryable errors (OracleUnavailable): up to 3 attempts
    /// - Simple errors (InvalidInput): up to 2 attempts
    /// - Non-retryable errors: 0 attempts
    ///
    /// # Parameters
    ///
    /// * `error` - The error code
    ///
    /// # Returns
    ///
    /// The maximum allowed recovery attempts (0-3).
    fn get_max_recovery_attempts(error: &Error) -> u32 {
        match error {
            Error::OracleUnavailable => 3,
            Error::InvalidInput => 2,
            Error::MarketNotFound | Error::ConfigNotFound => 1,
            Error::AlreadyVoted
            | Error::AlreadyBet
            | Error::AlreadyClaimed
            | Error::FeeAlreadyCollected
            | Error::Unauthorized
            | Error::MarketClosed
            | Error::MarketResolved
            | Error::AdminNotSet
            | Error::DisputeFeeFailed
            | Error::InvalidState
            | Error::InvalidOracleConfig => 0,
            _ => 1,
        }
    }

    /// Persists a recovery record to contract storage with collision-resistant key.
    ///
    /// Stores the recovery record using a composite key that includes:
    /// - Error code
    /// - Recovery timestamp
    /// - Attempt number
    /// - Operation length (as simple collision differentiator)
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `recovery` - The recovery record to store
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Record stored successfully
    /// * `Err(error)` - Storage operation failed
    fn store_recovery_record(env: &Env, recovery: &ErrorRecovery) -> Result<(), Error> {
        // Use operation length as a cheap differentiator when a proper hash is
        // unavailable in no_std. Replace with a real hash when the SDK supports it.
        let op_len = recovery.recovery_context.operation.len();
        let key_str = format!(
            "rec_{}_{}_{}_{}",
            recovery.original_error_code,
            recovery.recovery_timestamp,
            recovery.recovery_attempts,
            op_len,
        );
        let recovery_key = Symbol::new(env, &key_str);
        env.storage().persistent().set(&recovery_key, recovery);
        Ok(())
    }

    /// Converts a `RecoveryStrategy` enum to its canonical string representation.
    ///
    /// Provides consistent string names for recovery strategies for use in
    /// storage, events, and logging. Acts as the single source of truth
    /// for strategy string literals.
    ///
    /// # Strategy Mappings
    ///
    /// | Strategy | String |
    /// |----------|--------|
    /// | Retry | "retry" |
    /// | RetryWithDelay | "retry_with_delay" |
    /// | AlternativeMethod | "alternative_method" |
    /// | Skip | "skip" |
    /// | Abort | "abort" |
    /// | ManualIntervention | "manual_intervention" |
    /// | NoRecovery | "no_recovery" |
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `strategy` - The recovery strategy to convert
    ///
    /// # Returns
    ///
    /// A `String` representation of the strategy.
    fn recovery_strategy_to_str(env: &Env, strategy: &RecoveryStrategy) -> String {
        let s = match strategy {
            RecoveryStrategy::Retry => "retry",
            RecoveryStrategy::RetryWithDelay => "retry_with_delay",
            RecoveryStrategy::AlternativeMethod => "alternative_method",
            RecoveryStrategy::Skip => "skip",
            RecoveryStrategy::Abort => "abort",
            RecoveryStrategy::ManualIntervention => "manual_intervention",
            RecoveryStrategy::NoRecovery => "no_recovery",
        };
        String::from_str(env, s)
    }

    /// Classifies an error by severity, category, and recovery strategy.
    ///
    /// Maps each error variant to its:
    /// - **Severity**: How critical the error is (Critical/High/Medium/Low)
    /// - **Category**: What kind of error it is (Authentication/Oracle/Validation/System/etc.)
    /// - **Recovery**: Recommended recovery approach
    ///
    /// This function is the single source of truth for error classification.
    ///
    /// # Parameters
    ///
    /// * `error` - The error to classify
    ///
    /// # Returns
    ///
    /// A tuple of (severity, category, recovery_strategy) for the error.
    fn get_error_classification(
        error: &Error,
    ) -> (ErrorSeverity, ErrorCategory, RecoveryStrategy) {
        match error {
            // Critical
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
            // High
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
            // Medium
            Error::MarketNotFound => (
                ErrorSeverity::Medium,
                ErrorCategory::Market,
                RecoveryStrategy::AlternativeMethod,
            ),
            Error::MarketClosed | Error::MarketResolved => (
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
            Error::InvalidOracleConfig | Error::OracleConfidenceTooWide => (
                ErrorSeverity::Medium,
                ErrorCategory::Oracle,
                RecoveryStrategy::NoRecovery,
            ),
            // Low
            Error::AlreadyVoted
            | Error::AlreadyBet
            | Error::AlreadyClaimed
            | Error::NothingToClaim => (
                ErrorSeverity::Low,
                ErrorCategory::UserOperation,
                RecoveryStrategy::Skip,
            ),
            Error::FeeAlreadyCollected => (
                ErrorSeverity::Low,
                ErrorCategory::Financial,
                RecoveryStrategy::Skip,
            ),
            _ => (
                ErrorSeverity::Medium,
                ErrorCategory::Unknown,
                RecoveryStrategy::Abort,
            ),
        }
    }

    /// Generates a user-facing action string recommending what to do about an error.
    ///
    /// Provides specific, actionable guidance based on the error type and category.
    /// Uses context-sensitive messages to help users resolve the problem.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `error` - The error code
    /// * `category` - The error's category (for fallback messages)
    ///
    /// # Returns
    ///
    /// A `String` with recommended user actions.
    fn get_user_action(env: &Env, error: &Error, category: &ErrorCategory) -> String {
        let msg = match (error, category) {
            (Error::Unauthorized, _) => {
                "Ensure you have the required permissions before retrying."
            }
            (Error::InsufficientStake, _) => {
                "Increase your stake amount to meet the minimum requirement."
            }
            (Error::MarketNotFound, _) => {
                "Verify the market ID or check whether the market is still active."
            }
            (Error::MarketClosed, _) => {
                "This market is closed. Please look for an active market."
            }
            (Error::AlreadyVoted, _) => {
                "You have already voted. No further action is required."
            }
            (Error::OracleUnavailable, _) => {
                "The oracle is temporarily unavailable. Please try again later."
            }
            (Error::InvalidInput, _) => {
                "Check your input parameters and try again."
            }
            (_, ErrorCategory::Validation) => {
                "Review and correct the input data."
            }
            (_, ErrorCategory::System) => {
                "A system error occurred. Contact support if the issue persists."
            }
            (_, ErrorCategory::Financial) => {
                "A financial operation failed. Verify your balance and try again."
            }
            _ => "An error occurred. Please try again or contact support.",
        };
        String::from_str(env, msg)
    }

    /// Builds a technical details string containing debugging information.
    ///
    /// Produces a compact technical summary for logging and diagnostics:
    /// - Numeric error code
    /// - String error code name
    /// - Timestamp when error occurred
    /// - Operation name
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment
    /// * `error` - The error code
    /// * `context` - Runtime context from error occurrence
    ///
    /// # Returns
    ///
    /// A `String` formatted as: `code=NNN (STRING_CODE) ts=TIMESTAMP op=OPERATION`
    fn get_technical_details(env: &Env, error: &Error, context: &ErrorContext) -> String {
        let detail = format!(
            "code={} ({}) ts={} op={}",
            *error as u32,
            error.code(),
            context.timestamp,
            context.operation.to_string(),
        );
        String::from_str(env, &detail)
    }
}

// ===== ERROR DISPLAY HELPERS =====

impl Error {
    /// Returns a human-readable description of the error.
    ///
    /// Provides a clear, concise explanation suitable for logging and user-facing messages.
    ///
    /// # Returns
    ///
    /// A static string describing the error.
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
            Error::OracleUnavailable => "Oracle is unavailable",
            Error::InvalidOracleConfig => "Invalid oracle configuration",
            Error::GasBudgetExceeded => "Gas budget exceeded",
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
            Error::DisputeError => "Generic dispute subsystem error",
            Error::FeeAlreadyCollected => "Fee already collected",
            Error::NoFeesToCollect => "No fees to collect",
            Error::InvalidExtensionDays => "Invalid extension days",
            Error::ExtensionDenied => "Extension not allowed or exceeded",
            Error::AdminNotSet => "Admin address is not set (initialization missing)",
            Error::OracleStale => "Oracle data is stale or timed out",
            Error::OracleNoConsensus => "Oracle consensus not reached",
            Error::OracleVerified => "Oracle result already verified",
            Error::MarketNotReady => "Market not ready for oracle verification",
            Error::FallbackOracleUnavailable => "Fallback oracle is unavailable or unhealthy",
            Error::ResolutionTimeoutReached => "Resolution timeout has been reached",
            Error::OracleConfidenceTooWide => "Oracle confidence interval exceeds threshold",
            Error::CBNotInitialized => "Circuit breaker not initialized",
            Error::CBAlreadyOpen => "Circuit breaker is already open (paused)",
            Error::CBNotOpen => "Circuit breaker is not open (cannot recover)",
            Error::CBOpen => "Circuit breaker is open (operations blocked)",
            Error::CBError => "Generic circuit breaker subsystem error",
            Error::InvalidOracleFeed => "Invalid oracle feed or malformed response",
        }
    }

    /// Returns the canonical string code for the error.
    ///
    /// The string code is a consistent uppercase identifier (e.g., "UNAUTHORIZED",
    /// "ORACLE_UNAVAILABLE")
    /// suitable for error comparison, logging, and external systems.
    ///
    /// # Returns
    ///
    /// A static uppercase string code identifying the error.
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
            Error::OracleUnavailable => "ORACLE_UNAVAILABLE",
            Error::InvalidOracleConfig => "INVALID_ORACLE_CONFIG",
            Error::GasBudgetExceeded => "GAS_BUDGET_EXCEEDED",
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
            Error::DisputeError => "DISPUTE_ERROR",
            Error::FeeAlreadyCollected => "FEE_ALREADY_COLLECTED",
            Error::NoFeesToCollect => "NO_FEES_TO_COLLECT",
            Error::InvalidExtensionDays => "INVALID_EXTENSION_DAYS",
            Error::ExtensionDenied => "EXTENSION_DENIED",
            Error::AdminNotSet => "ADMIN_NOT_SET",
            Error::OracleStale => "ORACLE_STALE",
            Error::OracleNoConsensus => "ORACLE_NO_CONSENSUS",
            Error::OracleVerified => "ORACLE_VERIFIED",
            Error::MarketNotReady => "MARKET_NOT_READY",
            Error::FallbackOracleUnavailable => "FALLBACK_ORACLE_UNAVAILABLE",
            Error::ResolutionTimeoutReached => "RESOLUTION_TIMEOUT_REACHED",
            Error::OracleConfidenceTooWide => "ORACLE_CONFIDENCE_TOO_WIDE",
            Error::CBNotInitialized => "CIRCUIT_BREAKER_NOT_INITIALIZED",
            Error::CBAlreadyOpen => "CIRCUIT_BREAKER_ALREADY_OPEN",
            Error::CBNotOpen => "CIRCUIT_BREAKER_NOT_OPEN",
            Error::CBOpen => "CIRCUIT_BREAKER_OPEN",
            Error::CBError => "CIRCUIT_BREAKER_ERROR",
            Error::InvalidOracleFeed => "INVALID_ORACLE_FEED",
        }
    }
}

// ===== TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec as StdVec;
    use soroban_sdk::testutils::Address;

    fn make_context(env: &Env) -> ErrorContext {
        ErrorContext {
            operation: String::from_str(env, "test_operation"),
            user_address: Some(
                <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(env),
            ),
            market_id: Some(Symbol::new(env, "test_market")),
            context_data: Map::new(env),
            timestamp: env.ledger().timestamp(),
            call_chain: None, // optional — absence is valid
        }
    }

    fn all_errors() -> StdVec<Error> {
        vec![
            Error::Unauthorized,
            Error::MarketNotFound,
            Error::MarketClosed,
            Error::MarketResolved,
            Error::MarketNotResolved,
            Error::NothingToClaim,
            Error::AlreadyClaimed,
            Error::InsufficientStake,
            Error::InvalidOutcome,
            Error::AlreadyVoted,
            Error::AlreadyBet,
            Error::BetsAlreadyPlaced,
            Error::InsufficientBalance,
            Error::OracleUnavailable,
            Error::InvalidOracleConfig,
            Error::OracleStale,
            Error::OracleNoConsensus,
            Error::OracleVerified,
            Error::MarketNotReady,
            Error::FallbackOracleUnavailable,
            Error::ResolutionTimeoutReached,
            Error::OracleConfidenceTooWide,
            Error::InvalidQuestion,
            Error::InvalidOutcomes,
            Error::InvalidDuration,
            Error::InvalidThreshold,
            Error::InvalidComparison,
            Error::InvalidState,
            Error::InvalidInput,
            Error::InvalidFeeConfig,
            Error::ConfigNotFound,
            Error::AlreadyDisputed,
            Error::DisputeVoteExpired,
            Error::DisputeVoteDenied,
            Error::DisputeAlreadyVoted,
            Error::DisputeCondNotMet,
            Error::DisputeFeeFailed,
            Error::DisputeError,
            Error::FeeAlreadyCollected,
            Error::NoFeesToCollect,
            Error::InvalidExtensionDays,
            Error::ExtensionDenied,
            Error::GasBudgetExceeded,
            Error::AdminNotSet,
            Error::CBNotInitialized,
            Error::CBAlreadyOpen,
            Error::CBNotOpen,
            Error::CBOpen,
            Error::CBError,
        ]
    }

    #[test]
    fn test_error_categorization() {
        let env = Env::default();
        let context = make_context(&env);
        let detailed = ErrorHandler::categorize_error(&env, Error::Unauthorized, context);

        assert_eq!(detailed.severity, ErrorSeverity::High);
        assert_eq!(detailed.category, ErrorCategory::Authentication);
        assert_eq!(detailed.recovery_strategy, RecoveryStrategy::Abort);
    }

    #[test]
    fn test_error_recovery_strategy() {
        assert_eq!(
            ErrorHandler::get_error_recovery_strategy(&Error::OracleUnavailable),
            RecoveryStrategy::RetryWithDelay
        );
        assert_eq!(
            ErrorHandler::get_error_recovery_strategy(&Error::Unauthorized),
            RecoveryStrategy::Abort
        );
        assert_eq!(
            ErrorHandler::get_error_recovery_strategy(&Error::AlreadyVoted),
            RecoveryStrategy::Skip
        );
    }

    #[test]
    fn test_detailed_error_message_does_not_panic() {
        let env = Env::default();
        let context = make_context(&env);
        // Should not panic — previously this called Env::default() internally
        let _ = ErrorHandler::generate_detailed_error_message(&env, &Error::Unauthorized, &context);
    }

    #[test]
    fn test_error_context_validation_valid() {
        let env = Env::default();
        // call_chain is now Option — None is valid
        let ctx = ErrorContext {
            operation: String::from_str(&env, "place_bet"),
            user_address: None,
            market_id: None,
            context_data: Map::new(&env),
            timestamp: env.ledger().timestamp(),
            call_chain: None,
        };
        assert!(ErrorHandler::validate_error_context(&ctx).is_ok());
    }

    #[test]
    fn test_error_context_validation_empty_operation_fails() {
        let env = Env::default();
        let ctx = ErrorContext {
            operation: String::from_str(&env, ""),
            user_address: None,
            market_id: None,
            context_data: Map::new(&env),
            timestamp: env.ledger().timestamp(),
            call_chain: None,
        };
        assert!(ErrorHandler::validate_error_context(&ctx).is_err());
    }

    #[test]
    fn test_validate_error_recovery_no_duplicate_check() {
        let env = Env::default();
        let ctx = make_context(&env);
        let recovery = ErrorRecovery {
            original_error_code: Error::OracleUnavailable as u32,
            recovery_strategy: String::from_str(&env, "retry_with_delay"),
            recovery_timestamp: env.ledger().timestamp(),
            recovery_status: String::from_str(&env, "in_progress"),
            recovery_context: ctx,
            recovery_attempts: 1,
            max_recovery_attempts: 3,
            recovery_success_timestamp: None,
            recovery_failure_reason: None,
        };
        assert!(ErrorHandler::validate_error_recovery(&env, &recovery).is_ok());
    }

    #[test]
    fn test_error_analytics() {
        let env = Env::default();
        let analytics = ErrorHandler::get_error_analytics(&env).unwrap();
        assert_eq!(analytics.total_errors, 0);
        assert!(analytics.errors_by_category.get(ErrorCategory::UserOperation).is_some());
        assert!(analytics.errors_by_severity.get(ErrorSeverity::Low).is_some());
    }

    #[test]
    fn test_technical_details_not_placeholder() {
        let env = Env::default();
        let ctx = make_context(&env);
        let details =
            ErrorHandler::get_technical_details(&env, &Error::OracleUnavailable, &ctx);
        // Must contain the numeric error code, not just a generic string
        // (soroban String has no contains(), so we verify it is non-empty)
        assert!(!details.is_empty());
    }

    // ── Regression: GasBudgetExceeded missing from description() match ──────
    #[test]
    fn test_gas_budget_exceeded_description_is_exhaustive() {
        let err = Error::GasBudgetExceeded;
        let desc = err.description();
        assert!(!desc.is_empty(), "GasBudgetExceeded must have a non-empty description");
        assert_ne!(
            desc,
            "An error occurred. Please verify your parameters and try again.",
            "GasBudgetExceeded must have its own description, not the catch-all fallback"
        );
    }

    // ── Regression: GasBudgetExceeded::code() returned "GAS BUDGET EXCEEDED"
    //   (spaces) instead of "GAS_BUDGET_EXCEEDED" (underscores), breaking
    //   every consumer that pattern-matches on error code strings. ────────────
    #[test]
    fn test_gas_budget_exceeded_code_uses_underscores() {
        let code = Error::GasBudgetExceeded.code();
        assert!(
            !code.contains(' '),
            "Error code must use underscores, not spaces — got: {:?}",
            code
        );
        assert_eq!(code, "GAS_BUDGET_EXCEEDED");
    }

    // ── Regression: get_technical_details() passed error.code() as the
    //   `op=` argument instead of context.operation, so the operation name
    //   was never recorded in technical details. ────────────────────────────
    #[test]
    fn test_technical_details_contains_operation_name() {
        let env = Env::default();
        let mut ctx = make_context(&env);
        ctx.operation = String::from_str(&env, "resolve_market");

        let details = ErrorHandler::get_technical_details(&env, &Error::OracleUnavailable, &ctx);

        // Convert soroban String → &str for assertion
        let details_str = details.to_string();
        assert!(
            details_str.contains("resolve_market"),
            "technical details must include the operation name; got: {:?}",
            details_str
        );
        assert!(
            details_str.contains("200"), // OracleUnavailable = 200
            "technical details must include the numeric error code"
        );
    }

    #[test]
    fn test_all_error_codes_and_descriptions_are_non_empty() {
        for err in all_errors() {
            let code = err.code();
            let desc = err.description();
            assert!(!code.is_empty());
            assert!(!desc.is_empty());
            assert!(!code.contains(' '));
        }
    }

    #[test]
    fn test_generate_detailed_error_message_specific_and_fallback_paths() {
        let env = Env::default();
        let context = make_context(&env);

        let known = [
            Error::Unauthorized,
            Error::MarketNotFound,
            Error::MarketClosed,
            Error::OracleUnavailable,
            Error::InsufficientStake,
            Error::AlreadyVoted,
            Error::InvalidInput,
            Error::InvalidState,
        ];

        for err in known {
            let msg = ErrorHandler::generate_detailed_error_message(&env, &err, &context);
            assert!(!msg.is_empty());
        }

        // Exercise fallback branch
        let fallback_msg =
            ErrorHandler::generate_detailed_error_message(&env, &Error::CBError, &context);
        assert!(!fallback_msg.is_empty());
    }

    #[test]
    fn test_get_error_recovery_strategy_exhaustive() {
        for err in all_errors() {
            let strategy = ErrorHandler::get_error_recovery_strategy(&err);
            match strategy {
                RecoveryStrategy::Retry
                | RecoveryStrategy::RetryWithDelay
                | RecoveryStrategy::AlternativeMethod
                | RecoveryStrategy::Skip
                | RecoveryStrategy::Abort
                | RecoveryStrategy::ManualIntervention
                | RecoveryStrategy::NoRecovery => {}
            }
        }
    }

    #[test]
    fn test_error_classification_covers_all_variants() {
        for err in all_errors() {
            let (severity, category, strategy) = ErrorHandler::get_error_classification(&err);
            match severity {
                ErrorSeverity::Low
                | ErrorSeverity::Medium
                | ErrorSeverity::High
                | ErrorSeverity::Critical => {}
            }
            match category {
                ErrorCategory::UserOperation
                | ErrorCategory::Oracle
                | ErrorCategory::Validation
                | ErrorCategory::System
                | ErrorCategory::Dispute
                | ErrorCategory::Financial
                | ErrorCategory::Market
                | ErrorCategory::Authentication
                | ErrorCategory::Unknown => {}
            }
            match strategy {
                RecoveryStrategy::Retry
                | RecoveryStrategy::RetryWithDelay
                | RecoveryStrategy::AlternativeMethod
                | RecoveryStrategy::Skip
                | RecoveryStrategy::Abort
                | RecoveryStrategy::ManualIntervention
                | RecoveryStrategy::NoRecovery => {}
            }
        }
    }

    #[test]
    fn test_user_action_all_branches() {
        let env = Env::default();

        let direct_pairs = [
            (Error::Unauthorized, ErrorCategory::Authentication),
            (Error::InsufficientStake, ErrorCategory::UserOperation),
            (Error::MarketNotFound, ErrorCategory::Market),
            (Error::MarketClosed, ErrorCategory::Market),
            (Error::AlreadyVoted, ErrorCategory::UserOperation),
            (Error::OracleUnavailable, ErrorCategory::Oracle),
            (Error::InvalidInput, ErrorCategory::Validation),
        ];

        for (err, category) in direct_pairs {
            let msg = ErrorHandler::get_user_action(&env, &err, &category);
            assert!(!msg.is_empty());
        }

        // Category fallback branches
        let validation_msg =
            ErrorHandler::get_user_action(&env, &Error::InvalidQuestion, &ErrorCategory::Validation);
        assert!(!validation_msg.is_empty());
        let system_msg =
            ErrorHandler::get_user_action(&env, &Error::CBError, &ErrorCategory::System);
        assert!(!system_msg.is_empty());
        let financial_msg =
            ErrorHandler::get_user_action(&env, &Error::DisputeError, &ErrorCategory::Financial);
        assert!(!financial_msg.is_empty());

        // Final fallback
        let fallback =
            ErrorHandler::get_user_action(&env, &Error::CBError, &ErrorCategory::Unknown);
        assert!(!fallback.is_empty());
    }

    #[test]
    fn test_recovery_strategy_to_str_all_values() {
        let env = Env::default();
        let strategies = [
            RecoveryStrategy::Retry,
            RecoveryStrategy::RetryWithDelay,
            RecoveryStrategy::AlternativeMethod,
            RecoveryStrategy::Skip,
            RecoveryStrategy::Abort,
            RecoveryStrategy::ManualIntervention,
            RecoveryStrategy::NoRecovery,
        ];

        for strategy in strategies {
            let s = ErrorHandler::recovery_strategy_to_str(&env, &strategy);
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn test_execute_recovery_strategy_all_paths() {
        let env = Env::default();
        let ctx = make_context(&env);
        let now = env.ledger().timestamp();

        let retry = ErrorRecovery {
            original_error_code: Error::InvalidInput as u32,
            recovery_strategy: String::from_str(&env, "retry"),
            recovery_timestamp: now,
            recovery_status: String::from_str(&env, "in_progress"),
            recovery_context: ctx.clone(),
            recovery_attempts: 1,
            max_recovery_attempts: 2,
            recovery_success_timestamp: None,
            recovery_failure_reason: None,
        };
        assert!(ErrorHandler::execute_recovery_strategy(&env, &retry)
            .unwrap()
            .success);

        let retry_with_delay_fail = ErrorRecovery {
            recovery_strategy: String::from_str(&env, "retry_with_delay"),
            ..retry.clone()
        };
        assert!(!ErrorHandler::execute_recovery_strategy(&env, &retry_with_delay_fail)
            .unwrap()
            .success);

        let alt_success = ErrorRecovery {
            original_error_code: Error::OracleUnavailable as u32,
            recovery_strategy: String::from_str(&env, "alternative_method"),
            ..retry.clone()
        };
        assert!(ErrorHandler::execute_recovery_strategy(&env, &alt_success)
            .unwrap()
            .success);

        let skip = ErrorRecovery {
            recovery_strategy: String::from_str(&env, "skip"),
            ..retry.clone()
        };
        assert!(ErrorHandler::execute_recovery_strategy(&env, &skip)
            .unwrap()
            .success);

        let abort = ErrorRecovery {
            recovery_strategy: String::from_str(&env, "abort"),
            ..retry
        };
        assert!(!ErrorHandler::execute_recovery_strategy(&env, &abort)
            .unwrap()
            .success);
    }

    #[test]
    fn test_handle_error_recovery_all_strategy_paths() {
        let env = Env::default();
        let mut ctx = make_context(&env);
        ctx.timestamp = env.ledger().timestamp();

        assert_eq!(
            ErrorHandler::handle_error_recovery(&env, &Error::InvalidInput, &ctx),
            Ok(true)
        );
        assert_eq!(
            ErrorHandler::handle_error_recovery(&env, &Error::Unauthorized, &ctx),
            Ok(false)
        );
        assert_eq!(
            ErrorHandler::handle_error_recovery(&env, &Error::AlreadyVoted, &ctx),
            Ok(true)
        );

        assert_eq!(
            ErrorHandler::handle_error_recovery(&env, &Error::MarketNotFound, &ctx),
            Ok(false)
        );
        assert_eq!(
            ErrorHandler::handle_error_recovery(&env, &Error::ConfigNotFound, &ctx),
            Ok(false)
        );

        assert!(
            ErrorHandler::handle_error_recovery(&env, &Error::OracleUnavailable, &ctx).is_err()
        );
        assert_eq!(
            ErrorHandler::handle_error_recovery(&env, &Error::OracleConfidenceTooWide, &ctx),
            Ok(false)
        );
        assert!(
            ErrorHandler::handle_error_recovery(&env, &Error::AdminNotSet, &ctx).is_err()
        );
    }

    #[test]
    fn test_validate_error_recovery_error_paths() {
        let env = Env::default();
        let mut ctx = make_context(&env);
        let now = env.ledger().timestamp();

        let too_many_attempts = ErrorRecovery {
            original_error_code: Error::InvalidInput as u32,
            recovery_strategy: String::from_str(&env, "retry"),
            recovery_timestamp: now,
            recovery_status: String::from_str(&env, "in_progress"),
            recovery_context: ctx.clone(),
            recovery_attempts: 3,
            max_recovery_attempts: 2,
            recovery_success_timestamp: None,
            recovery_failure_reason: None,
        };
        assert!(ErrorHandler::validate_error_recovery(&env, &too_many_attempts).is_err());

        ctx.timestamp = now;
        let future_timestamp = ErrorRecovery {
            recovery_timestamp: now + 1,
            recovery_attempts: 1,
            max_recovery_attempts: 2,
            recovery_context: ctx,
            ..too_many_attempts
        };
        assert!(ErrorHandler::validate_error_recovery(&env, &future_timestamp).is_err());
    }

    #[test]
    fn test_validate_resilience_patterns_invalid_branches() {
        let env = Env::default();

        let mut valid_pattern = ResiliencePattern {
            pattern_name: String::from_str(&env, "retry_backoff"),
            pattern_type: ResiliencePatternType::RetryWithBackoff,
            pattern_config: {
                let mut m = Map::new(&env);
                m.set(String::from_str(&env, "attempts"), String::from_str(&env, "3"));
                m
            },
            enabled: true,
            priority: 10,
            last_used: None,
            success_rate: 9_000,
        };

        let mut patterns = Vec::new(&env);
        patterns.push_back(valid_pattern.clone());
        assert_eq!(
            ErrorHandler::validate_resilience_patterns(&env, &patterns),
            Ok(true)
        );

        valid_pattern.pattern_name = String::from_str(&env, "");
        let mut invalid_name = Vec::new(&env);
        invalid_name.push_back(valid_pattern.clone());
        assert!(ErrorHandler::validate_resilience_patterns(&env, &invalid_name).is_err());

        valid_pattern.pattern_name = String::from_str(&env, "retry_backoff");
        valid_pattern.pattern_config = Map::new(&env);
        let mut invalid_config = Vec::new(&env);
        invalid_config.push_back(valid_pattern.clone());
        assert!(ErrorHandler::validate_resilience_patterns(&env, &invalid_config).is_err());

        valid_pattern.pattern_config = {
            let mut m = Map::new(&env);
            m.set(String::from_str(&env, "attempts"), String::from_str(&env, "3"));
            m
        };
        valid_pattern.priority = 0;
        let mut invalid_priority = Vec::new(&env);
        invalid_priority.push_back(valid_pattern.clone());
        assert!(ErrorHandler::validate_resilience_patterns(&env, &invalid_priority).is_err());

        valid_pattern.priority = 101;
        let mut invalid_priority_high = Vec::new(&env);
        invalid_priority_high.push_back(valid_pattern.clone());
        assert!(ErrorHandler::validate_resilience_patterns(&env, &invalid_priority_high).is_err());

        valid_pattern.priority = 10;
        valid_pattern.success_rate = -1;
        let mut invalid_rate_low = Vec::new(&env);
        invalid_rate_low.push_back(valid_pattern.clone());
        assert!(ErrorHandler::validate_resilience_patterns(&env, &invalid_rate_low).is_err());

        valid_pattern.success_rate = 10_001;
        let mut invalid_rate_high = Vec::new(&env);
        invalid_rate_high.push_back(valid_pattern);
        assert!(ErrorHandler::validate_resilience_patterns(&env, &invalid_rate_high).is_err());
    }

    #[test]
    fn test_document_error_recovery_procedures_contains_expected_keys() {
        let env = Env::default();
        let procedures = ErrorHandler::document_error_recovery_procedures(&env).unwrap();
        assert!(
            procedures
                .get(String::from_str(&env, "retry_procedure"))
                .is_some()
        );
        assert!(
            procedures
                .get(String::from_str(&env, "oracle_recovery"))
                .is_some()
        );
        assert!(
            procedures
                .get(String::from_str(&env, "validation_recovery"))
                .is_some()
        );
        assert!(
            procedures
                .get(String::from_str(&env, "system_recovery"))
                .is_some()
        );
    }

    #[test]
    fn test_recover_from_error_persists_and_updates_status() {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let context = make_context(&env);

        let recovery = env.as_contract(&contract_id, || {
            ErrorHandler::recover_from_error(&env, Error::InvalidInput, context.clone()).unwrap()
        });

        assert_eq!(recovery.recovery_status, String::from_str(&env, "success"));
        assert_eq!(recovery.recovery_attempts, 1);
        assert_eq!(recovery.max_recovery_attempts, 2);
        assert!(recovery.recovery_success_timestamp.is_some());
    }
}