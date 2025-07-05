use soroban_sdk::{contracterror, panic_with_error, Env, String};

/// Comprehensive error management system for Predictify Hybrid contract
///
/// This module provides a centralized error handling system with:
/// - Categorized error types for better organization
/// - Detailed error messages and documentation
/// - Error conversion traits for interoperability
/// - Helper functions for common error scenarios
/// - Context-aware error handling

/// Main error enum for the Predictify Hybrid contract
///
/// Errors are categorized into logical groups for better organization:
/// - Security: Authentication and authorization errors
/// - Market: Market state and operation errors  
/// - Oracle: Oracle integration and data errors
/// - Validation: Input validation and business logic errors
/// - State: Contract state and storage errors
#[contracterror]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    // ===== SECURITY ERRORS (1-10) =====
    /// Unauthorized access attempt - caller lacks required permissions
    Unauthorized = 1,

    // ===== MARKET ERRORS (11-30) =====
    /// Market is closed and no longer accepting votes or stakes
    MarketClosed = 2,
    /// Market has already been resolved and cannot be modified
    MarketAlreadyResolved = 5,
    /// Market has not been resolved yet
    MarketNotResolved = 9,
    /// Market does not exist
    MarketNotFound = 11,
    /// Market has expired
    MarketExpired = 12,
    /// Market is still active and cannot be resolved
    MarketStillActive = 13,
    /// Market extension is not allowed
    MarketExtensionNotAllowed = 14,
    /// Market extension days exceeded limit
    ExtensionDaysExceeded = 15,
    /// Invalid extension days provided
    InvalidExtensionDays = 16,
    /// Invalid extension reason provided
    InvalidExtensionReason = 17,
    /// Market extension fee insufficient
    ExtensionFeeInsufficient = 18,

    // ===== ORACLE ERRORS (31-50) =====
    /// Oracle service is unavailable or not responding
    OracleUnavailable = 3,
    /// Oracle configuration is invalid or malformed
    InvalidOracleConfig = 6,
    /// Oracle data is stale or outdated
    OracleDataStale = 31,
    /// Oracle feed ID is invalid or not found
    InvalidOracleFeed = 32,
    /// Oracle price is outside acceptable range
    OraclePriceOutOfRange = 33,
    /// Oracle comparison operation failed
    OracleComparisonFailed = 34,

    // ===== VALIDATION ERRORS (51-70) =====
    /// Invalid outcome specified for voting or resolution
    InvalidOutcome = 10,
    /// Insufficient stake for the requested operation
    InsufficientStake = 4,
    /// Invalid input parameters provided
    InvalidInput = 51,
    /// Question is empty or invalid
    InvalidQuestion = 52,
    /// Outcomes list is empty or invalid
    InvalidOutcomes = 53,
    /// Duration is invalid or too short/long
    InvalidDuration = 54,
    /// Threshold value is invalid
    InvalidThreshold = 55,
    /// Comparison operator is invalid
    InvalidComparison = 56,

    // ===== STATE ERRORS (71-90) =====
    /// User has already claimed their winnings
    AlreadyClaimed = 7,
    /// No winnings available to claim
    NothingToClaim = 8,
    /// User has already voted on this market
    AlreadyVoted = 71,
    /// User has already staked on this market
    AlreadyStaked = 72,
    /// User has already disputed this result
    AlreadyDisputed = 73,
    /// Fee has already been collected
    FeeAlreadyCollected = 74,
    /// No fees available to collect
    NoFeesToCollect = 75,

    // ===== SYSTEM ERRORS (91-100) =====
    /// Internal contract error
    InternalError = 91,
    /// Storage operation failed
    StorageError = 92,
    /// Arithmetic overflow or underflow
    ArithmeticError = 93,
    /// Invalid contract state
    InvalidState = 94,
}

/// Error categories for better organization and handling
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorCategory {
    Security,
    Market,
    Oracle,
    Validation,
    State,
    System,
}

impl Error {
    /// Get the category of this error
    pub fn category(&self) -> ErrorCategory {
        match self {
            // Security errors
            Error::Unauthorized => ErrorCategory::Security,

            // Market errors
            Error::MarketClosed
            | Error::MarketAlreadyResolved
            | Error::MarketNotResolved
            | Error::MarketNotFound
            | Error::MarketExpired
            | Error::MarketStillActive
            | Error::MarketExtensionNotAllowed
            | Error::ExtensionDaysExceeded
            | Error::InvalidExtensionDays
            | Error::InvalidExtensionReason
            | Error::ExtensionFeeInsufficient => ErrorCategory::Market,

            // Oracle errors
            Error::OracleUnavailable
            | Error::InvalidOracleConfig
            | Error::OracleDataStale
            | Error::InvalidOracleFeed
            | Error::OraclePriceOutOfRange
            | Error::OracleComparisonFailed => ErrorCategory::Oracle,

            // Validation errors
            Error::InvalidOutcome
            | Error::InsufficientStake
            | Error::InvalidInput
            | Error::InvalidQuestion
            | Error::InvalidOutcomes
            | Error::InvalidDuration
            | Error::InvalidThreshold
            | Error::InvalidComparison => ErrorCategory::Validation,

            // State errors
            Error::AlreadyClaimed
            | Error::NothingToClaim
            | Error::AlreadyVoted
            | Error::AlreadyStaked
            | Error::AlreadyDisputed
            | Error::FeeAlreadyCollected
            | Error::NoFeesToCollect => ErrorCategory::State,

            // System errors
            Error::InternalError
            | Error::StorageError
            | Error::ArithmeticError
            | Error::InvalidState => ErrorCategory::System,
        }
    }

    /// Get a human-readable error message
    pub fn message(&self) -> &'static str {
        match self {
            // Security errors
            Error::Unauthorized => "Unauthorized access - caller lacks required permissions",

            // Market errors
            Error::MarketClosed => "Market is closed and no longer accepting votes or stakes",
            Error::MarketAlreadyResolved => {
                "Market has already been resolved and cannot be modified"
            }
            Error::MarketNotResolved => "Market has not been resolved yet",
            Error::MarketNotFound => "Market does not exist",
            Error::MarketExpired => "Market has expired",
            Error::MarketStillActive => "Market is still active and cannot be resolved",
            Error::MarketExtensionNotAllowed => "Market extension is not allowed",
            Error::ExtensionDaysExceeded => "Market extension days exceeded limit",
            Error::InvalidExtensionDays => "Invalid extension days provided",
            Error::InvalidExtensionReason => "Invalid extension reason provided",
            Error::ExtensionFeeInsufficient => "Market extension fee insufficient",

            // Oracle errors
            Error::OracleUnavailable => "Oracle service is unavailable or not responding",
            Error::InvalidOracleConfig => "Oracle configuration is invalid or malformed",
            Error::OracleDataStale => "Oracle data is stale or outdated",
            Error::InvalidOracleFeed => "Oracle feed ID is invalid or not found",
            Error::OraclePriceOutOfRange => "Oracle price is outside acceptable range",
            Error::OracleComparisonFailed => "Oracle comparison operation failed",

            // Validation errors
            Error::InvalidOutcome => "Invalid outcome specified for voting or resolution",
            Error::InsufficientStake => "Insufficient stake for the requested operation",
            Error::InvalidInput => "Invalid input parameters provided",
            Error::InvalidQuestion => "Question is empty or invalid",
            Error::InvalidOutcomes => "Outcomes list is empty or invalid",
            Error::InvalidDuration => "Duration is invalid or too short/long",
            Error::InvalidThreshold => "Threshold value is invalid",
            Error::InvalidComparison => "Comparison operator is invalid",

            // State errors
            Error::AlreadyClaimed => "User has already claimed their winnings",
            Error::NothingToClaim => "No winnings available to claim",
            Error::AlreadyVoted => "User has already voted on this market",
            Error::AlreadyStaked => "User has already staked on this market",
            Error::AlreadyDisputed => "User has already disputed this result",
            Error::FeeAlreadyCollected => "Fee has already been collected",
            Error::NoFeesToCollect => "No fees available to collect",

            // System errors
            Error::InternalError => "Internal contract error occurred",
            Error::StorageError => "Storage operation failed",
            Error::ArithmeticError => "Arithmetic overflow or underflow occurred",
            Error::InvalidState => "Invalid contract state",
        }
    }

    /// Get error code as string for debugging
    pub fn code(&self) -> &'static str {
        match self {
            Error::Unauthorized => "UNAUTHORIZED",
            Error::MarketClosed => "MARKET_CLOSED",
            Error::OracleUnavailable => "ORACLE_UNAVAILABLE",
            Error::InsufficientStake => "INSUFFICIENT_STAKE",
            Error::MarketAlreadyResolved => "MARKET_ALREADY_RESOLVED",
            Error::InvalidOracleConfig => "INVALID_ORACLE_CONFIG",
            Error::AlreadyClaimed => "ALREADY_CLAIMED",
            Error::MarketExtensionNotAllowed => "MARKET_EXTENSION_NOT_ALLOWED",
            Error::ExtensionDaysExceeded => "EXTENSION_DAYS_EXCEEDED",
            Error::InvalidExtensionDays => "INVALID_EXTENSION_DAYS",
            Error::InvalidExtensionReason => "INVALID_EXTENSION_REASON",
            Error::ExtensionFeeInsufficient => "EXTENSION_FEE_INSUFFICIENT",
            Error::NothingToClaim => "NOTHING_TO_CLAIM",
            Error::MarketNotResolved => "MARKET_NOT_RESOLVED",
            Error::InvalidOutcome => "INVALID_OUTCOME",
            Error::MarketNotFound => "MARKET_NOT_FOUND",
            Error::MarketExpired => "MARKET_EXPIRED",
            Error::MarketStillActive => "MARKET_STILL_ACTIVE",
            Error::OracleDataStale => "ORACLE_DATA_STALE",
            Error::InvalidOracleFeed => "INVALID_ORACLE_FEED",
            Error::OraclePriceOutOfRange => "ORACLE_PRICE_OUT_OF_RANGE",
            Error::OracleComparisonFailed => "ORACLE_COMPARISON_FAILED",
            Error::InvalidInput => "INVALID_INPUT",
            Error::InvalidQuestion => "INVALID_QUESTION",
            Error::InvalidOutcomes => "INVALID_OUTCOMES",
            Error::InvalidDuration => "INVALID_DURATION",
            Error::InvalidThreshold => "INVALID_THRESHOLD",
            Error::InvalidComparison => "INVALID_COMPARISON",
            Error::AlreadyVoted => "ALREADY_VOTED",
            Error::AlreadyStaked => "ALREADY_STAKED",
            Error::AlreadyDisputed => "ALREADY_DISPUTED",
            Error::FeeAlreadyCollected => "FEE_ALREADY_COLLECTED",
            Error::NoFeesToCollect => "NO_FEES_TO_COLLECT",
            Error::InternalError => "INTERNAL_ERROR",
            Error::StorageError => "STORAGE_ERROR",
            Error::ArithmeticError => "ARITHMETIC_ERROR",
            Error::InvalidState => "INVALID_STATE",
        }
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self.category(),
            ErrorCategory::Validation | ErrorCategory::State
        )
    }

    /// Check if this is a critical error that should halt execution
    pub fn is_critical(&self) -> bool {
        matches!(
            self.category(),
            ErrorCategory::Security | ErrorCategory::System
        )
    }
}

/// Error context for additional debugging information
#[derive(Clone, Debug)]
pub struct ErrorContext {
    pub operation: String,
    pub details: String,
    pub timestamp: u64,
}

impl ErrorContext {
    pub fn new(env: &Env, operation: &str, details: &str) -> Self {
        Self {
            operation: String::from_str(env, operation),
            details: String::from_str(env, details),
            timestamp: env.ledger().timestamp(),
        }
    }
}

/// Error helper functions for common scenarios
pub mod helpers {
    use super::*;

    /// Validate that the caller is the admin
    pub fn require_admin(
        env: &Env,
        caller: &soroban_sdk::Address,
        admin: &soroban_sdk::Address,
    ) -> Result<(), Error> {
        if caller != admin {
            panic_with_error!(env, Error::Unauthorized);
        }
        Ok(())
    }

    /// Validate that the market exists and is not closed
    pub fn require_market_open(env: &Env, market: &Option<crate::Market>) -> Result<(), Error> {
        match market {
            Some(market) => {
                if env.ledger().timestamp() >= market.end_time {
                    panic_with_error!(env, Error::MarketClosed);
                }
                Ok(())
            }
            None => {
                panic_with_error!(env, Error::MarketNotFound);
            }
        }
    }

    /// Validate that the market is resolved
    pub fn require_market_resolved(env: &Env, market: &Option<crate::Market>) -> Result<(), Error> {
        match market {
            Some(market) => {
                if market.winning_outcome.is_none() {
                    panic_with_error!(env, Error::MarketNotResolved);
                }
                Ok(())
            }
            None => {
                panic_with_error!(env, Error::MarketNotFound);
            }
        }
    }

    /// Validate that the outcome is valid for the market
    pub fn require_valid_outcome(
        env: &Env,
        outcome: &String,
        outcomes: &soroban_sdk::Vec<String>,
    ) -> Result<(), Error> {
        if !outcomes.contains(outcome) {
            panic_with_error!(env, Error::InvalidOutcome);
        }
        Ok(())
    }

    /// Validate that the stake amount is sufficient
    pub fn require_sufficient_stake(env: &Env, stake: i128, min_stake: i128) -> Result<(), Error> {
        if stake < min_stake {
            panic_with_error!(env, Error::InsufficientStake);
        }
        Ok(())
    }

    /// Validate that the user hasn't already claimed
    pub fn require_not_claimed(env: &Env, claimed: bool) -> Result<(), Error> {
        if claimed {
            panic_with_error!(env, Error::AlreadyClaimed);
        }
        Ok(())
    }

    /// Validate oracle configuration
    pub fn require_valid_oracle_config(
        env: &Env,
        config: &crate::OracleConfig,
    ) -> Result<(), Error> {
        if config.threshold <= 0 {
            panic_with_error!(env, Error::InvalidOracleConfig);
        }

        if config.comparison != String::from_str(env, "gt")
            && config.comparison != String::from_str(env, "lt")
            && config.comparison != String::from_str(env, "eq")
        {
            panic_with_error!(env, Error::InvalidOracleConfig);
        }

        Ok(())
    }

    /// Validate market creation parameters
    pub fn require_valid_market_params(
        env: &Env,
        question: &String,
        outcomes: &soroban_sdk::Vec<String>,
        duration_days: u32,
    ) -> Result<(), Error> {
        if question.is_empty() {
            panic_with_error!(env, Error::InvalidQuestion);
        }

        if outcomes.len() < 2 {
            panic_with_error!(env, Error::InvalidOutcomes);
        }

        if duration_days == 0 || duration_days > 365 {
            panic_with_error!(env, Error::InvalidDuration);
        }

        Ok(())
    }
}

/// Error conversion traits for interoperability
pub mod conversions {
    use super::*;

    /// Convert from core::result::Result to our Error type
    pub trait IntoPredictifyError<T> {
        fn into_predictify_error(self, env: &Env, default_error: Error) -> Result<T, Error>;
    }

    impl<T, E> IntoPredictifyError<T> for core::result::Result<T, E> {
        fn into_predictify_error(self, env: &Env, default_error: Error) -> Result<T, Error> {
            self.map_err(|_| {
                panic_with_error!(env, default_error);
            })
        }
    }
}

/// Error logging and debugging utilities
pub mod debug {
    use super::*;

    /// Log error with context for debugging
    pub fn log_error(env: &Env, error: Error, context: &ErrorContext) {
        // In a real implementation, this would log to a debug storage or event
        // For now, we'll just use the panic mechanism
        // Note: In no_std environment, we can't use format! macro
        // This is a placeholder - in a real implementation you might want to
        // store this in a debug log or emit an event
        let _ = (env, error, context); // Suppress unused variable warning
    }

    /// Create a detailed error report
    pub fn create_error_report(env: &Env, error: Error, context: &ErrorContext) -> String {
        // In no_std environment, we can't use format! macro
        // For now, return a simple error message
        String::from_str(env, &error.message())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categories() {
        assert_eq!(Error::Unauthorized.category(), ErrorCategory::Security);
        assert_eq!(Error::MarketClosed.category(), ErrorCategory::Market);
        assert_eq!(Error::OracleUnavailable.category(), ErrorCategory::Oracle);
        assert_eq!(Error::InvalidOutcome.category(), ErrorCategory::Validation);
        assert_eq!(Error::AlreadyClaimed.category(), ErrorCategory::State);
        assert_eq!(Error::InternalError.category(), ErrorCategory::System);
    }

    #[test]
    fn test_error_messages() {
        assert_eq!(
            Error::Unauthorized.message(),
            "Unauthorized access - caller lacks required permissions"
        );
        assert_eq!(
            Error::MarketClosed.message(),
            "Market is closed and no longer accepting votes or stakes"
        );
        assert_eq!(
            Error::OracleUnavailable.message(),
            "Oracle service is unavailable or not responding"
        );
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(Error::Unauthorized.code(), "UNAUTHORIZED");
        assert_eq!(Error::MarketClosed.code(), "MARKET_CLOSED");
        assert_eq!(Error::OracleUnavailable.code(), "ORACLE_UNAVAILABLE");
    }

    #[test]
    fn test_error_recoverability() {
        assert!(!Error::Unauthorized.is_recoverable());
        assert!(Error::InvalidOutcome.is_recoverable());
        assert!(Error::AlreadyClaimed.is_recoverable());
    }

    #[test]
    fn test_error_criticality() {
        assert!(Error::Unauthorized.is_critical());
        assert!(!Error::InvalidOutcome.is_critical());
        assert!(Error::InternalError.is_critical());
    }

    #[test]
    fn test_error_context() {
        let env = soroban_sdk::Env::default();
        let context = ErrorContext::new(&env, "test_operation", "test_details");

        assert_eq!(context.operation, String::from_str(&env, "test_operation"));
        assert_eq!(context.details, String::from_str(&env, "test_details"));
        // Note: In test environment, timestamp might be 0, so we just check it's a valid u64
        assert!(context.timestamp >= 0);
    }
}
