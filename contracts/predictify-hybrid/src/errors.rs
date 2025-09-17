#![allow(dead_code)]

use soroban_sdk::{
    contracterror, contracttype, Address, Env, Map, String, Symbol, Vec,
};

/// Essential error codes for Predictify Hybrid contract
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
    MarketAlreadyResolved = 103,
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

    // ===== ORACLE ERRORS =====
    /// Oracle is unavailable
    OracleUnavailable = 200,
    /// Invalid oracle configuration
    InvalidOracleConfig = 201,

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
    ConfigurationNotFound = 403,
    /// Already disputed
    AlreadyDisputed = 404,
    /// Dispute voting period expired
    DisputeVotingPeriodExpired = 405,
    /// Dispute voting not allowed
    DisputeVotingNotAllowed = 406,
    /// Already voted in dispute
    DisputeAlreadyVoted = 407,
    /// Dispute resolution conditions not met
    DisputeResolutionConditionsNotMet = 408,
    /// Dispute fee distribution failed
    DisputeFeeDistributionFailed = 409,
    /// Dispute escalation not allowed
    DisputeEscalationNotAllowed = 410,
    /// Threshold below minimum
    ThresholdBelowMinimum = 411,
    /// Threshold exceeds maximum
    ThresholdExceedsMaximum = 412,
    /// Fee already collected
    FeeAlreadyCollected = 413,
    /// Invalid oracle feed
    InvalidOracleFeed = 414,
    /// No fees to collect
    NoFeesToCollect = 415,
    /// Invalid extension days
    InvalidExtensionDays = 416,
    /// Extension days exceeded
    ExtensionDaysExceeded = 417,
    /// Market extension not allowed
    MarketExtensionNotAllowed = 418,
    /// Extension fee insufficient
    ExtensionFeeInsufficient = 419,
    /// Admin address is not set (initialization missing)
    AdminNotSet = 420,

    // ===== AUDIT SYSTEM ERRORS =====
    /// Audit not initialized
    AuditNotInitialized = 500,
    /// Audit item not found
    AuditItemNotFound = 501,
    /// Audit already completed
    AuditAlreadyCompleted = 502,
    /// Audit requirements not met
    AuditRequirementsNotMet = 503,
    /// Invalid audit category
    InvalidAuditCategory = 504,
    /// Invalid audit priority
    InvalidAuditPriority = 505,
    /// Audit permission denied
    AuditPermissionDenied = 506,
    /// Audit validation failed
    AuditValidationFailed = 507,

    // ===== DISPUTE TIMEOUT ERRORS =====
    /// Invalid timeout hours
    InvalidTimeoutHours = 600,
    /// Dispute timeout not expired
    DisputeTimeoutNotExpired = 601,
    /// Dispute timeout extension not allowed
    DisputeTimeoutExtensionNotAllowed = 602,
    /// Dispute timeout not set
    DisputeTimeoutNotSet = 603,

}

impl Error {
    /// Get a human-readable description of the error
    pub fn description(&self) -> &'static str {
        match self {
            Error::Unauthorized => "User is not authorized to perform this action",
            Error::MarketNotFound => "Market not found",
            Error::MarketClosed => "Market is closed",
            Error::MarketAlreadyResolved => "Market is already resolved",
            Error::MarketNotResolved => "Market is not resolved yet",
            Error::NothingToClaim => "User has nothing to claim",
            Error::AlreadyClaimed => "User has already claimed",
            Error::InsufficientStake => "Insufficient stake amount",
            Error::InvalidOutcome => "Invalid outcome choice",
            Error::AlreadyVoted => "User has already voted",
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
            Error::ConfigurationNotFound => "Configuration not found",
            Error::AlreadyDisputed => "Already disputed",
            Error::DisputeVotingPeriodExpired => "Dispute voting period expired",
            Error::DisputeVotingNotAllowed => "Dispute voting not allowed",
            Error::DisputeAlreadyVoted => "Already voted in dispute",
            Error::DisputeResolutionConditionsNotMet => "Dispute resolution conditions not met",
            Error::DisputeFeeDistributionFailed => "Dispute fee distribution failed",
            Error::DisputeEscalationNotAllowed => "Dispute escalation not allowed",
            Error::ThresholdBelowMinimum => "Threshold below minimum",
            Error::ThresholdExceedsMaximum => "Threshold exceeds maximum",
            Error::FeeAlreadyCollected => "Fee already collected",
            Error::InvalidOracleFeed => "Invalid oracle feed",
            Error::NoFeesToCollect => "No fees to collect",
            Error::InvalidExtensionDays => "Invalid extension days",
            Error::ExtensionDaysExceeded => "Extension days exceeded",
            Error::MarketExtensionNotAllowed => "Market extension not allowed",
            Error::ExtensionFeeInsufficient => "Extension fee insufficient",
            Error::AdminNotSet => "Admin address is not set (initialization missing)",
            Error::AuditNotInitialized => "Audit system is not initialized",
            Error::AuditItemNotFound => "Audit item not found",
            Error::AuditAlreadyCompleted => "Audit is already completed",
            Error::AuditRequirementsNotMet => "Audit requirements not met for deployment",
            Error::InvalidAuditCategory => "Invalid audit category",
            Error::InvalidAuditPriority => "Invalid audit priority",
            Error::AuditPermissionDenied => "Permission denied for audit operation",
            Error::AuditValidationFailed => "Audit validation failed",
            Error::InvalidTimeoutHours => "Invalid timeout hours specified",
            Error::DisputeTimeoutNotExpired => "Dispute timeout period has not expired yet",
            Error::DisputeTimeoutExtensionNotAllowed => "Dispute timeout extension is not allowed",
            Error::DisputeTimeoutNotSet => "Dispute timeout is not set",

        }
    }

    /// Get error code as string
    pub fn code(&self) -> &'static str {
        match self {
            Error::Unauthorized => "UNAUTHORIZED",
            Error::MarketNotFound => "MARKET_NOT_FOUND",
            Error::MarketClosed => "MARKET_CLOSED",
            Error::MarketAlreadyResolved => "MARKET_ALREADY_RESOLVED",
            Error::MarketNotResolved => "MARKET_NOT_RESOLVED",
            Error::NothingToClaim => "NOTHING_TO_CLAIM",
            Error::AlreadyClaimed => "ALREADY_CLAIMED",
            Error::InsufficientStake => "INSUFFICIENT_STAKE",
            Error::InvalidOutcome => "INVALID_OUTCOME",
            Error::AlreadyVoted => "ALREADY_VOTED",
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
            Error::ConfigurationNotFound => "CONFIGURATION_NOT_FOUND",
            Error::AlreadyDisputed => "ALREADY_DISPUTED",
            Error::DisputeVotingPeriodExpired => "DISPUTE_VOTING_PERIOD_EXPIRED",
            Error::DisputeVotingNotAllowed => "DISPUTE_VOTING_NOT_ALLOWED",
            Error::DisputeAlreadyVoted => "DISPUTE_ALREADY_VOTED",
            Error::DisputeResolutionConditionsNotMet => "DISPUTE_RESOLUTION_CONDITIONS_NOT_MET",
            Error::DisputeFeeDistributionFailed => "DISPUTE_FEE_DISTRIBUTION_FAILED",
            Error::DisputeEscalationNotAllowed => "DISPUTE_ESCALATION_NOT_ALLOWED",
            Error::ThresholdBelowMinimum => "THRESHOLD_BELOW_MINIMUM",
            Error::ThresholdExceedsMaximum => "THRESHOLD_EXCEEDS_MAXIMUM",
            Error::FeeAlreadyCollected => "FEE_ALREADY_COLLECTED",
            Error::InvalidOracleFeed => "INVALID_ORACLE_FEED",
            Error::NoFeesToCollect => "NO_FEES_TO_COLLECT",
            Error::InvalidExtensionDays => "INVALID_EXTENSION_DAYS",
            Error::ExtensionDaysExceeded => "EXTENSION_DAYS_EXCEEDED",
            Error::MarketExtensionNotAllowed => "MARKET_EXTENSION_NOT_ALLOWED",
            Error::ExtensionFeeInsufficient => "EXTENSION_FEE_INSUFFICIENT",
            Error::AdminNotSet => "ADMIN_NOT_SET",
            Error::AuditNotInitialized => "AUDIT_NOT_INITIALIZED",
            Error::AuditItemNotFound => "AUDIT_ITEM_NOT_FOUND",
            Error::AuditAlreadyCompleted => "AUDIT_ALREADY_COMPLETED",
            Error::AuditRequirementsNotMet => "AUDIT_REQUIREMENTS_NOT_MET",
            Error::InvalidAuditCategory => "INVALID_AUDIT_CATEGORY",
            Error::InvalidAuditPriority => "INVALID_AUDIT_PRIORITY",
            Error::AuditPermissionDenied => "AUDIT_PERMISSION_DENIED",
            Error::AuditValidationFailed => "AUDIT_VALIDATION_FAILED",
            Error::InvalidTimeoutHours => "INVALID_TIMEOUT_HOURS",
            Error::DisputeTimeoutNotExpired => "DISPUTE_TIMEOUT_NOT_EXPIRED",
            Error::DisputeTimeoutExtensionNotAllowed => "DISPUTE_TIMEOUT_EXTENSION_NOT_ALLOWED",

            Error::DisputeTimeoutNotSet => "DISPUTE_TIMEOUT_NOT_SET",

        }
    }
}


// ===== TESTING MODULE =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        // Test that error codes match expected values
        assert_eq!(Error::Unauthorized as u32, 100);
        assert_eq!(Error::MarketNotFound as u32, 101);
        assert_eq!(Error::MarketClosed as u32, 102);
        assert_eq!(Error::InsufficientStake as u32, 107);
        assert_eq!(Error::OracleUnavailable as u32, 200);
        assert_eq!(Error::InvalidQuestion as u32, 300);
        assert_eq!(Error::InvalidState as u32, 400);
        assert_eq!(Error::AuditNotInitialized as u32, 500);
    }

    #[test]
    fn test_error_ordering() {
        // Test that errors can be compared and ordered
        assert!(Error::Unauthorized < Error::MarketNotFound);
        assert!(Error::MarketNotFound < Error::OracleUnavailable);
        assert!(Error::OracleUnavailable < Error::InvalidQuestion);
    }

    // Note: Advanced error handling tests removed due to missing dependencies
    // These would require ErrorHandler, RecoveryStrategy, ErrorCategory, ErrorContext, etc.
    // which are not currently implemented in this codebase
}
