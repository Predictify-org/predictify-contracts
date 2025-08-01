#![allow(dead_code)]

use soroban_sdk::contracterror;

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
        }
    }
}

