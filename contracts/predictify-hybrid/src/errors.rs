#![allow(dead_code)]

use soroban_sdk::contracterror;

/// Comprehensive error codes for the Predictify Hybrid prediction market contract.
///
/// This enum defines all possible error conditions that can occur within the Predictify Hybrid
/// smart contract system. Each error is assigned a unique numeric code for efficient handling
/// and clear identification. The errors are organized into logical categories for better
/// understanding and maintenance.
///
/// # Error Categories
///
/// **User Operation Errors (100-199):**
/// - Authentication and authorization failures
/// - Market access and state violations
/// - User action conflicts and restrictions
///
/// **Oracle Errors (200-299):**
/// - Oracle connectivity and availability issues
/// - Oracle configuration and validation problems
///
/// **Validation Errors (300-399):**
/// - Input validation failures
/// - Parameter format and range violations
/// - Configuration validation errors
///
/// **System Errors (400-499):**
/// - System state and configuration issues
/// - Dispute and governance related errors
/// - Fee and extension management errors
///
/// # Example Usage
///
/// ```rust
/// # use predictify_hybrid::errors::Error;
///
/// // Handle specific error types
/// fn handle_market_operation_result(result: Result<(), Error>) {
///     match result {
///         Ok(()) => println!("Operation successful"),
///         Err(Error::Unauthorized) => {
///             println!("Error {}: {}", Error::Unauthorized as u32, Error::Unauthorized.description());
///         }
///         Err(Error::MarketNotFound) => {
///             println!("Market does not exist or has been removed");
///         }
///         Err(Error::InsufficientStake) => {
///             println!("Stake amount is below minimum requirement");
///         }
///         Err(e) => {
///             println!("Operation failed with error {}: {}", e as u32, e.description());
///         }
///     }
/// }
///
/// // Get error information
/// let error = Error::MarketClosed;
/// println!("Error Code: {}", error.code());           // "MARKET_CLOSED"
/// println!("Description: {}", error.description());   // "Market is closed"
/// println!("Numeric Code: {}", error as u32);         // 102
/// ```
///
/// # Error Handling Best Practices
///
/// 1. **Specific Handling**: Match specific error types for targeted error handling
/// 2. **User Feedback**: Use `description()` method for user-friendly error messages
/// 3. **Logging**: Use `code()` method for structured logging and monitoring
/// 4. **Recovery**: Implement appropriate recovery strategies for different error types
/// 5. **Validation**: Prevent errors through proper input validation
///
/// # Integration Points
///
/// Error enum integrates with:
/// - **All Contract Functions**: Every public function returns Result<T, Error>
/// - **Validation System**: Validation functions return specific error types
/// - **Event System**: Error events are emitted with error codes
/// - **Client Applications**: Error codes enable proper error handling in dApps
/// - **Monitoring Systems**: Error codes support operational monitoring and alerting
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
    /// Dispute timeout not set
    DisputeTimeoutNotSet = 421,
    /// Dispute timeout expired
    DisputeTimeoutExpired = 422,
    /// Dispute timeout not expired
    DisputeTimeoutNotExpired = 423,
    /// Invalid timeout hours
    InvalidTimeoutHours = 424,
    /// Dispute timeout extension not allowed
    DisputeTimeoutExtensionNotAllowed = 425,
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
            Error::DisputeTimeoutNotSet => "Dispute timeout not set",
            Error::DisputeTimeoutExpired => "Dispute timeout expired",
            Error::DisputeTimeoutNotExpired => "Dispute timeout not expired",
            Error::InvalidTimeoutHours => "Invalid timeout hours",
            Error::DisputeTimeoutExtensionNotAllowed => "Dispute timeout extension not allowed",
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
            Error::DisputeTimeoutNotSet => "DISPUTE_TIMEOUT_NOT_SET",
            Error::DisputeTimeoutExpired => "DISPUTE_TIMEOUT_EXPIRED",
            Error::DisputeTimeoutNotExpired => "DISPUTE_TIMEOUT_NOT_EXPIRED",
            Error::InvalidTimeoutHours => "INVALID_TIMEOUT_HOURS",
            Error::DisputeTimeoutExtensionNotAllowed => "DISPUTE_TIMEOUT_EXTENSION_NOT_ALLOWED",
        }
    }
}
