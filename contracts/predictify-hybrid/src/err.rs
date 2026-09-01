#![allow(dead_code)]

use alloc::format;
use alloc::string::{String as StdString, ToString};
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

/// Public mapping struct containing all metadata and classification for an error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicErrorMapping {
    pub contract_code: u32,
    pub client_code: u32,
    pub code_str: &'static str,
    pub description: &'static str,
    pub category: ErrorCategory,
    pub severity: ErrorSeverity,
    pub recoverability: Recoverability,
    pub is_known: bool,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    IdempotentBatchAlreadyApplied = 660,
    /// Reason table has reached its maximum capacity of 256 entries.
    ReasonTableFull = 670,
    Overflow = 672,
    MaxBetCapExceeded = 673,
    InvalidCap = 674,
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
    /// The provided claim nonce does not match the expected nonce for replay protection.
    /// Each claim must include the correct nonce to prevent transaction replays.
    InvalidNonce = 113,
    /// Bet amount exceeds the effective maximum allowed for the market.
    BetAboveMaximum = 114,
    /// Bet amount is below the per-market minimum threshold.
    BetBelowMarketMin = 115,
    /// Per-market bet limits are inverted (min > max).
    BetLimitsInverted = 116,
    /// Per-market bet limit exceeds the absolute maximum.
    BetLimitAboveMaximum = 117,
    /// Per-market max single-bet cap is out of range (zero, negative, or above absolute max).
    BetCapOutOfRange = 118,

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
    /// Initialization parameters must be validated atomically. If any parameter
    /// is invalid, the entire initialization is rejected and no state is changed.
    InvalidInitializationParams = 700,
    /// Generic dispute subsystem error. Check dispute state and configuration.
    DisputeError = 410,
    /// The dispute opener cannot vote on their own dispute.
    DisputerCannotVote = 438,
    /// Unclaimed winnings have already been swept for this market. Repeat sweeps are not allowed.
    SweepAlreadyDone = 411,
    /// Fee arithmetic overflowed during checked platform-fee calculation.
    FeeArithmeticOverflow = 412,
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
    /// Asset decimals mismatch. Stored decimals differ from the live SAC decimals.
    /// This prevents silently inflated or deflated stakes via normalize_amount.
    AssetDecimalsMismatch = 439,
    /// A per-market admin action was attempted before the configured timelock period elapsed.
    AdminActionTimelocked = 443,
    /// The operation would exceed the remaining CPU instruction budget.
    /// This is a pre-emptive guard that aborts before the host runs out of resources.
    ///
    /// Discriminant 444: AdminNotSet was pinned at 418 in the stability test so
    /// OperationWouldExceedBudget is placed after the frozen metadata range.
    OperationWouldExceedBudget = 444,

    // ===== METADATA LENGTH LIMIT ERRORS (420-434) =====
    /// Market question exceeds maximum allowed length.
    QuestionTooLong = 420,
    /// Outcome label exceeds maximum allowed length.
    OutcomeTooLong = 421,
    /// Too many outcomes specified for the market.
    TooManyOutcomes = 422,
    /// Oracle feed ID exceeds maximum allowed length.
    FeedIdTooLong = 423,
    /// Comparison operator exceeds maximum allowed length.
    ComparisonTooLong = 424,
    /// Category string exceeds maximum allowed length.
    CategoryTooLong = 425,
    /// Tag string exceeds maximum allowed length.
    TagTooLong = 426,
    /// Too many tags specified for the market.
    TooManyTags = 427,
    /// Extension reason exceeds maximum allowed length.
    ExtensionReasonTooLong = 428,
    /// Source identifier exceeds maximum allowed length.
    SourceTooLong = 429,
    /// Error message exceeds maximum allowed length.
    ErrorMessageTooLong = 430,
    /// Signature string exceeds maximum allowed length.
    SignatureTooLong = 431,
    /// Too many extension history entries.
    TooManyExtensions = 432,
    /// Too many oracle results in multi-oracle aggregation.
    TooManyOracleResults = 433,
    /// Too many winning outcomes specified.
    TooManyWinningOutcomes = 434,
    /// Force-resolve idempotency key has already been used for this market.
    ///
    /// The same `(market_id, idempotency_key)` pair was already consumed by a
    /// previous `force_resolve_market` call. The operation is safe to treat as
    /// a no-op; no resolution was re-applied.
    ForceResolveAlreadyUsed = 435,
    /// The event archive has reached its maximum capacity. Prune old entries before archiving more.
    ArchiveFull = 440,
    /// Category string is shorter than the minimum allowed length (when a category is set).
    CategoryTooShort = 436,
    /// Tag string is shorter than the minimum allowed length (non-empty tags only).
    TagTooShort = 437,

    // ===== VALIDATION ERRORS (435-437) =====
    /// Market ID already exists in the registry. Cannot create duplicate market IDs.
    DuplicateMarketId = 441,
    /// Market cannot be archived from current state. Archive only allowed from Resolved or Cancelled.
    CannotArchiveFromState = 442,
    /// Market cannot be restored from current state. Restore only allowed from Archived.
    CannotRestoreFromState = 447,
    /// Market is already archived. Cannot perform modification operations on archived markets.
    MarketAlreadyArchived = 445,
    /// Market is already restored. Cannot restore a market that is not archived.
    MarketAlreadyRestored = 446,
    // `ReplayedOverride` is defined once below (= 526); the duplicate that lived
    // here (= 442) was removed to fix E0428.

    // ===== CIRCUIT BREAKER ERRORS =====
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
    /// Rate limit exceeded. Too many requests in the time window.
    RateLimitExceeded = 505,
    /// Cumulative extension cap reached; no further extensions allowed for this market.
    CumulativeExtensionCapHit = 506,
    /// A market state transition was attempted that is not permitted by the state machine.
    ///
    /// This error is returned by `MarketStateLogic::validate_state_transition` whenever the
    /// requested `(from, to)` pair is not in the set of legal edges.  Callers should treat
    /// this as a terminal error — the transition will never succeed without first moving the
    /// market through intermediate states that are part of the legal path.
    ///
    /// # Examples of illegal transitions
    ///
    /// * `Resolved → Active`  (cannot reopen a resolved market)
    /// * `Closed → Ended`     (terminal state, no transitions allowed)
    /// * `Active → Active`    (self-loops are not valid transitions)
    IllegalMarketStateTransition = 507,
    /// The effective fee (in basis points) exceeds the maximum the caller is willing to accept.
    /// The bet is rejected to protect the caller from unexpected fee changes.
    FeeExceedsMax = 508,
    /// Force-resolve idempotency key has already been used. Use a new unique key.
    ForceResolveReplayed = 517,
    /// Force-resolve reason is empty. Every force-resolve must be justified.
    ForceResolveReasonEmpty = 518,
    /// No pending fee config commit was found for reveal or apply.
    NoPendingFeeCommit = 519,
    /// Fee config reveal was attempted too early (before timelock expiry).
    FeeRevealTooEarly = 520,
    /// Preimage does not match the committed hash during fee reveal.
    FeePreimageMismatch = 521,
    /// Dispute stake cap has been exceeded for this address.
    DisputeStakeCapExceeded = 522,
    /// Storage rent budget is insufficient for the requested operation.
    InsufficientStorageRentBudget = 523,
    /// The cumulative extension cap for this market has been reached.
    ExtensionCapExceeded = 524,
    /// The upgrade chain predecessor hash does not match the expected value.
    UpgradeChainMismatch = 525,
    /// An admin override nonce was replayed; reject to prevent replay attacks.
    /// Oracle quote is an outlier relative to the rolling median history.
    OracleQuoteOutlier = 527,
    /// Maximum number of unique participants has been reached for this market.
    MaxParticipantsReached = 528,
    /// The bet amount exceeds the maximum cap for this user/market.
    BetExceedsCap = 675,
    /// An admin override was replayed; reject to prevent replay attacks.
    ReplayedOverride = 526,
    /// Oracle admin cooldown is currently active.
    OracleAdminCooldownActive = 676,
    /// Signer rotation cooldown is currently active.
    SignerRotationCooldown = 677,
    /// User is not whitelisted for this operation.
    UserNotWhitelisted = 678,
    /// User has been blacklisted.
    UserBlacklisted = 679,
    /// Creator has been blacklisted.
    CreatorBlacklisted = 680,
    /// Contract is already initialized.
    AlreadyInitialized = 681,
    /// Invalid timelock delay.
    InvalidTimeLockDelay = 682,
    /// Timelock has not yet expired.
    TimeLockNotExpired = 683,
    /// No pending update found.
    NoPendingUpdate = 684,
    /// A pending update already exists.
    PendingUpdateExists = 685,
    /// Invalid stake amount.
    InvalidStakeAmount = 686,
    /// Per-ledger bet cap exceeded.
    PerLedgerBetCapExceeded = 687,
    /// Registry is full.
    RegistryFull = 688,
    /// Batch contains no entries; at least one bet is required.
    BatchEmpty = 545,
    /// Batch exceeds the maximum allowed number of entries.
    BatchSizeExceeded = 546,
    /// Treasury update timelock has not yet expired.
    TreasuryUpdateTimelocked = 689,
    /// No pending treasury update found.
    NoPendingTreasuryUpdate = 690,
    /// A pending treasury update already exists.
    PendingTreasuryUpdateExists = 691,
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

/// Off-chain recoverability annotation for each error variant.
///
/// # Client Guidance
///
/// | Label | Meaning | Off-chain action |
/// |-------|---------|-----------------|
/// | `Retryable` | Transient condition; the call may succeed if retried | Exponential back-off, max 3 attempts |
/// | `RequiresAdmin` | Needs privileged intervention before retrying | Alert ops team; do not auto-retry |
/// | `Terminal` | Permanent failure; retrying is futile | Surface to user; no retry |
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Recoverability {
    /// Transient condition. The operation may succeed on a subsequent attempt.
    Retryable,
    /// Requires an administrator action before the operation can proceed.
    RequiresAdmin,
    /// Permanent failure. Further attempts will not succeed.
    Terminal,
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
    fn soroban_string_to_host_string(value: &String) -> StdString {
        let mut bytes = alloc::vec![0u8; value.len() as usize];
        value.copy_into_slice(&mut bytes);
        StdString::from_utf8(bytes).unwrap_or_else(|_| StdString::from("invalid_utf8"))
    }

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
            Error::MarketClosed => "Market is closed and cannot accept new operations.",
            Error::OracleUnavailable => {
                "Oracle service is unavailable. The external data source may be down."
            }
            Error::InsufficientStake => {
                "Insufficient stake. Please increase the amount to meet the minimum requirement."
            }
            Error::AlreadyVoted => {
                "User has already voted in this market. Only one vote per user is allowed."
            }
            Error::InvalidInput => "Invalid input. Please check your parameters and try again.",
            Error::InvalidState => {
                "Invalid system state. The contract may be in an unexpected condition."
            }
            Error::ForceResolveAlreadyUsed => {
                "Force-resolve idempotency key already used. The operation is a safe no-op."
            }
            Error::ForceResolveReplayed => {
                "Force-resolve idempotency key already used. Use a new unique key."
            }
            Error::ForceResolveReasonEmpty => {
                "Force-resolve reason is empty. Provide a non-empty reason string."
            }
            Error::CannotArchiveFromState => {
                "Market cannot be archived from its current state. Archive is only allowed from Resolved or Cancelled states."
            }
            Error::CannotRestoreFromState => {
                "Market cannot be restored from its current state. Restore is only allowed from Archived state."
            }
            Error::MarketAlreadyArchived => {
                "Market is already archived. Archived markets are immutable and cannot be modified."
            }
            Error::MarketAlreadyRestored => {
                "Market is already restored. Cannot restore a market that is not archived."
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
            | Error::FeeAlreadyCollected
            | Error::ForceResolveAlreadyUsed => RecoveryStrategy::Skip,
            Error::ForceResolveReplayed | Error::ForceResolveReasonEmpty => {
                RecoveryStrategy::Retry
            }
            Error::Unauthorized | Error::MarketClosed | Error::MarketResolved => {
                RecoveryStrategy::Abort
            }
            Error::AdminNotSet | Error::DisputeFeeFailed => RecoveryStrategy::ManualIntervention,
            Error::InvalidState | Error::InvalidOracleConfig => RecoveryStrategy::NoRecovery,
            Error::CannotArchiveFromState
            | Error::CannotRestoreFromState
            | Error::MarketAlreadyArchived
            | Error::MarketAlreadyRestored => RecoveryStrategy::Abort,
            Error::FeeExceedsMax => RecoveryStrategy::Retry,
            Error::BetExceedsCap => RecoveryStrategy::NoRecovery,
            Error::OperationWouldExceedBudget => RecoveryStrategy::NoRecovery,
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
        let strategy_str =
            Self::recovery_strategy_to_str(env, &Self::get_error_recovery_strategy(&error));
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
            String::from_str(
                env,
                "For oracle errors, try fallback oracle or cached data.",
            ),
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
            String::from_str(
                env,
                "For critical system errors, require manual intervention.",
            ),
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
            | Error::ForceResolveAlreadyUsed
            | Error::FeeAlreadyCollected
            | Error::Unauthorized
            | Error::MarketClosed
            | Error::MarketResolved
            | Error::AdminNotSet
            | Error::DisputeFeeFailed
            | Error::InvalidState
            | Error::InvalidOracleConfig
            | Error::OperationWouldExceedBudget => 0,
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
    pub(crate) fn get_error_classification(error: &Error) -> (ErrorSeverity, ErrorCategory, RecoveryStrategy) {
        match error {
            Error::IdempotentBatchAlreadyApplied => (ErrorSeverity::Low, error.category(), RecoveryStrategy::Skip),
            Error::ReasonTableFull => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::Overflow => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::MaxBetCapExceeded => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::InvalidCap => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::Unauthorized => (ErrorSeverity::High, error.category(), RecoveryStrategy::Abort),
            Error::MarketNotFound => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::AlternativeMethod),
            Error::MarketClosed => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::MarketResolved => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::MarketNotResolved => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::NothingToClaim => (ErrorSeverity::Low, error.category(), RecoveryStrategy::Skip),
            Error::AlreadyClaimed => (ErrorSeverity::Low, error.category(), RecoveryStrategy::Skip),
            Error::InsufficientStake => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::InvalidOutcome => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::AlreadyVoted => (ErrorSeverity::Low, error.category(), RecoveryStrategy::Skip),
            Error::AlreadyBet => (ErrorSeverity::Low, error.category(), RecoveryStrategy::Skip),
            Error::BetsAlreadyPlaced => (ErrorSeverity::Low, error.category(), RecoveryStrategy::Skip),
            Error::InsufficientBalance => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::InvalidNonce => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::BetAboveMaximum => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::BetBelowMarketMin => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::BetLimitsInverted => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::BetLimitAboveMaximum => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::BetCapOutOfRange => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::OracleUnavailable => (ErrorSeverity::High, error.category(), RecoveryStrategy::RetryWithDelay),
            Error::InvalidOracleConfig => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::NoRecovery),
            Error::OracleStale => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::OracleNoConsensus => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::OracleVerified => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::MarketNotReady => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::FallbackOracleUnavailable => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::ResolutionTimeoutReached => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::RetryWithDelay),
            Error::OracleConfidenceTooWide => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::NoRecovery),
            Error::InvalidOracleFeed => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::ManualIntervention),
            Error::OracleCallbackAuthFailed => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::OracleCallbackUnauthorized => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::OracleCallbackInvalidSignature => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::OracleCallbackReplayDetected => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::OracleCallbackTimeout => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::RetryWithDelay),
            Error::InvalidQuestion => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::InvalidOutcomes => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::InvalidDuration => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::InvalidThreshold => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::InvalidComparison => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::InvalidState => (ErrorSeverity::High, error.category(), RecoveryStrategy::NoRecovery),
            Error::InvalidInput => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::InvalidFeeConfig => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::ManualIntervention),
            Error::ConfigNotFound => (ErrorSeverity::High, error.category(), RecoveryStrategy::ManualIntervention),
            Error::AlreadyDisputed => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Skip),
            Error::DisputeVoteExpired => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::DisputeVoteDenied => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::DisputeAlreadyVoted => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Skip),
            Error::DisputeCondNotMet => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::DisputeFeeFailed => (ErrorSeverity::Critical, error.category(), RecoveryStrategy::ManualIntervention),
            Error::InvalidInitializationParams => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::DisputeError => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::DisputerCannotVote => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::SweepAlreadyDone => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Skip),
            Error::FeeArithmeticOverflow => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::FeeAlreadyCollected => (ErrorSeverity::Low, error.category(), RecoveryStrategy::Skip),
            Error::NoFeesToCollect => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::InvalidExtensionDays => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::ExtensionDenied => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::GasBudgetExceeded => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::AdminNotSet => (ErrorSeverity::Critical, error.category(), RecoveryStrategy::ManualIntervention),
            Error::AssetDecimalsMismatch => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::ManualIntervention),
            Error::AdminActionTimelocked => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::ManualIntervention),
            Error::OperationWouldExceedBudget => (ErrorSeverity::Critical, error.category(), RecoveryStrategy::NoRecovery),
            Error::QuestionTooLong => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::OutcomeTooLong => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::TooManyOutcomes => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::FeedIdTooLong => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::ComparisonTooLong => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::CategoryTooLong => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::TagTooLong => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::TooManyTags => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::ExtensionReasonTooLong => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::SourceTooLong => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::ErrorMessageTooLong => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::SignatureTooLong => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::TooManyExtensions => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::TooManyOracleResults => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::TooManyWinningOutcomes => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::ForceResolveAlreadyUsed => (ErrorSeverity::Low, error.category(), RecoveryStrategy::Skip),
            Error::ArchiveFull => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::CategoryTooShort => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::TagTooShort => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::DuplicateMarketId => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::CannotArchiveFromState => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::CannotRestoreFromState => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::MarketAlreadyArchived => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Skip),
            Error::MarketAlreadyRestored => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Skip),
            Error::CBNotInitialized => (ErrorSeverity::High, error.category(), RecoveryStrategy::ManualIntervention),
            Error::CBAlreadyOpen => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::ManualIntervention),
            Error::CBNotOpen => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::ManualIntervention),
            Error::CBOpen => (ErrorSeverity::High, error.category(), RecoveryStrategy::Retry),
            Error::CBError => (ErrorSeverity::High, error.category(), RecoveryStrategy::ManualIntervention),
            Error::RateLimitExceeded => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::CumulativeExtensionCapHit => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::IllegalMarketStateTransition => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::FeeExceedsMax => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::ForceResolveReplayed => (ErrorSeverity::Low, error.category(), RecoveryStrategy::Retry),
            Error::ForceResolveReasonEmpty => (ErrorSeverity::Low, error.category(), RecoveryStrategy::Retry),
            Error::NoPendingFeeCommit => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::ManualIntervention),
            Error::FeeRevealTooEarly => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::FeePreimageMismatch => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::DisputeStakeCapExceeded => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::InsufficientStorageRentBudget => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::ExtensionCapExceeded => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::UpgradeChainMismatch => (ErrorSeverity::High, error.category(), RecoveryStrategy::ManualIntervention),
            Error::OracleQuoteOutlier => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::MaxParticipantsReached => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::BetExceedsCap => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::NoRecovery),
            Error::ReplayedOverride => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::OracleAdminCooldownActive => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::RetryWithDelay),
            Error::SignerRotationCooldown => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::RetryWithDelay),
            Error::UserNotWhitelisted => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::UserBlacklisted => (ErrorSeverity::High, error.category(), RecoveryStrategy::Abort),
            Error::CreatorBlacklisted => (ErrorSeverity::High, error.category(), RecoveryStrategy::Abort),
            Error::AlreadyInitialized => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Skip),
            Error::InvalidTimeLockDelay => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::TimeLockNotExpired => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::NoPendingUpdate => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::ManualIntervention),
            Error::PendingUpdateExists => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::ManualIntervention),
            Error::InvalidStakeAmount => (ErrorSeverity::Low, error.category(), RecoveryStrategy::Retry),
            Error::PerLedgerBetCapExceeded => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::RegistryFull => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::BatchEmpty => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::BatchSizeExceeded => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Retry),
            Error::TreasuryUpdateTimelocked => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::NoPendingTreasuryUpdate => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
            Error::PendingTreasuryUpdateExists => (ErrorSeverity::Medium, error.category(), RecoveryStrategy::Abort),
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
            (Error::Unauthorized, _) => "Ensure you have the required permissions before retrying.",
            (Error::InsufficientStake, _) => {
                "Increase your stake amount to meet the minimum requirement."
            }
            (Error::MarketNotFound, _) => {
                "Verify the market ID or check whether the market is still active."
            }
            (Error::MarketClosed, _) => "This market is closed. Please look for an active market.",
            (Error::AlreadyVoted, _) => "You have already voted. No further action is required.",
            (Error::OracleUnavailable, _) => {
                "The oracle is temporarily unavailable. Please try again later."
            }
            (Error::InvalidInput, _) => "Check your input parameters and try again.",
            (Error::OperationWouldExceedBudget, _) => {
                "The operation requires too much CPU time. Try with fewer winners or split across multiple transactions."
            }
            (_, ErrorCategory::Validation) => "Review and correct the input data.",
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
        let operation = Self::soroban_string_to_host_string(&context.operation);
        let detail = format!(
            "code={} ({}) ts={} op={}",
            *error as u32,
            error.code(),
            context.timestamp,
            operation,
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
            Error::IdempotentBatchAlreadyApplied => "Idempotent Batch Already Applied",
            Error::ReasonTableFull => "Reason table has reached its maximum capacity of 256 entries.",
            Error::Overflow => "Overflow",
            Error::MaxBetCapExceeded => "Max Bet Cap Exceeded",
            Error::InvalidCap => "Invalid Cap",
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
            Error::BetsAlreadyPlaced => "{",
            Error::InsufficientBalance => "Insufficient balance for operation",
            Error::InvalidNonce => "The provided claim nonce does not match the expected nonce for replay protection. Each claim must include the correct nonce to prevent transaction replays.",
            Error::BetAboveMaximum => "Bet amount exceeds the effective maximum allowed for the market.",
            Error::BetBelowMarketMin => "Bet amount is below the per-market minimum threshold.",
            Error::BetLimitsInverted => "Per-market bet limits are inverted (min > max).",
            Error::BetLimitAboveMaximum => "Per-market bet limit exceeds the absolute maximum.",
            Error::BetCapOutOfRange => "Per-market max single-bet cap is out of range (zero, negative, or above absolute max).",
            Error::OracleUnavailable => "Oracle is unavailable",
            Error::InvalidOracleConfig => "Invalid oracle configuration",
            Error::OracleStale => "Oracle data is stale",
            Error::OracleNoConsensus => "Oracle consensus not reached",
            Error::OracleVerified => "Oracle result already verified",
            Error::MarketNotReady => "Market not ready for verification",
            Error::FallbackOracleUnavailable => "Fallback oracle unavailable",
            Error::ResolutionTimeoutReached => "Resolution timeout reached",
            Error::OracleConfidenceTooWide => "Oracle confidence interval too wide",
            Error::InvalidOracleFeed => "Invalid oracle feed ID",
            Error::OracleCallbackAuthFailed => "Oracle callback authentication failed",
            Error::OracleCallbackUnauthorized => "Oracle callback unauthorized",
            Error::OracleCallbackInvalidSignature => "Oracle callback signature invalid",
            Error::OracleCallbackReplayDetected => "Oracle callback replay detected",
            Error::OracleCallbackTimeout => "Oracle callback timed out",
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
            Error::InvalidInitializationParams => "Initialization parameters must be validated atomically. If any parameter is invalid, the entire initialization is rejected and no state is changed.",
            Error::DisputeError => "Generic dispute subsystem error",
            Error::DisputerCannotVote => "Dispute opener cannot vote on their own dispute",
            Error::SweepAlreadyDone => "Unclaimed winnings already swept for this market",
            Error::FeeArithmeticOverflow => "Fee arithmetic overflowed",
            Error::FeeAlreadyCollected => "Platform fee already collected",
            Error::NoFeesToCollect => "No fees available to collect",
            Error::InvalidExtensionDays => "Invalid extension days value",
            Error::ExtensionDenied => "Market extension not allowed",
            Error::GasBudgetExceeded => "Gas budget exceeded",
            Error::AdminNotSet => "Admin address not set",
            Error::AssetDecimalsMismatch => "Asset decimals mismatch between stored and SAC decimals",
            Error::AdminActionTimelocked => "A per-market admin action was attempted before the configured timelock period elapsed.",
            Error::OperationWouldExceedBudget => "The operation would exceed the remaining CPU instruction budget. This is a pre-emptive guard that aborts before the host runs out of resources.  Discriminant 444: AdminNotSet was pinned at 418 in the stability test so OperationWouldExceedBudget is placed after the frozen metadata range.",
            Error::QuestionTooLong => "Market question exceeds maximum allowed length",
            Error::OutcomeTooLong => "Outcome label exceeds maximum allowed length",
            Error::TooManyOutcomes => "Too many outcomes specified for the market",
            Error::FeedIdTooLong => "Oracle feed ID exceeds maximum allowed length",
            Error::ComparisonTooLong => "Comparison operator exceeds maximum allowed length",
            Error::CategoryTooLong => "Category string exceeds maximum allowed length",
            Error::TagTooLong => "Tag string exceeds maximum allowed length",
            Error::TooManyTags => "Too many tags specified for the market",
            Error::ExtensionReasonTooLong => "Extension reason exceeds maximum allowed length",
            Error::SourceTooLong => "Source identifier exceeds maximum allowed length",
            Error::ErrorMessageTooLong => "Error message exceeds maximum allowed length",
            Error::SignatureTooLong => "Signature string exceeds maximum allowed length",
            Error::TooManyExtensions => "Too many extension history entries",
            Error::TooManyOracleResults => "Too many oracle results in multi-oracle aggregation",
            Error::TooManyWinningOutcomes => "Too many winning outcomes specified",
            Error::ForceResolveAlreadyUsed => "Force-resolve idempotency key has already been used for this market.  The same `(market_id, idempotency_key)` pair was already consumed by a previous `force_resolve_market` call. The operation is safe to treat as a no-op; no resolution was re-applied.",
            Error::ArchiveFull => "Event archive is full; maximum archive capacity reached",
            Error::CategoryTooShort => "Category string is shorter than the minimum allowed length",
            Error::TagTooShort => "Tag string is shorter than the minimum allowed length",
            Error::DuplicateMarketId => "Market ID already exists in the registry",
            Error::CannotArchiveFromState => "Market cannot be archived from current state. Archive only allowed from Resolved or Cancelled.",
            Error::CannotRestoreFromState => "Market cannot be restored from current state. Restore only allowed from Archived.",
            Error::MarketAlreadyArchived => "Market is already archived. Cannot perform modification operations on archived markets.",
            Error::MarketAlreadyRestored => "Market is already restored. Cannot restore a market that is not archived.",
            Error::CBNotInitialized => "Circuit breaker not initialized",
            Error::CBAlreadyOpen => "Circuit breaker is already open (paused)",
            Error::CBNotOpen => "Circuit breaker is not open (cannot recover)",
            Error::CBOpen => "Circuit breaker is open (operations blocked)",
            Error::CBError => "Generic circuit breaker subsystem error",
            Error::RateLimitExceeded => "Rate limit exceeded; too many requests in the time window",
            Error::CumulativeExtensionCapHit => "Cumulative extension cap reached; no further extensions allowed",
            Error::IllegalMarketStateTransition => "Illegal market state transition attempted",
            Error::FeeExceedsMax => "Fee is above the acceptable threshold",
            Error::ForceResolveReplayed => "Force-resolve idempotency key has already been used. Use a new unique key.",
            Error::ForceResolveReasonEmpty => "Force-resolve reason is empty. Every force-resolve must be justified.",
            Error::NoPendingFeeCommit => "No pending fee config commit found",
            Error::FeeRevealTooEarly => "Fee config reveal attempted too early",
            Error::FeePreimageMismatch => "Preimage does not match the committed hash",
            Error::DisputeStakeCapExceeded => "Dispute stake cap exceeded for this address",
            Error::InsufficientStorageRentBudget => "{",
            Error::ExtensionCapExceeded => "Cumulative extension cap for this market has been reached",
            Error::UpgradeChainMismatch => "Upgrade chain predecessor hash mismatch",
            Error::OracleQuoteOutlier => "Oracle quote is an outlier relative to the rolling median",
            Error::MaxParticipantsReached => "Maximum number of unique participants has been reached for this market.",
            Error::BetExceedsCap => "Bet amount exceeds the per-market maximum bet cap",
            Error::ReplayedOverride => "Admin override nonce replayed; rejected",
            Error::OracleAdminCooldownActive => "Oracle admin cooldown is currently active.",
            Error::SignerRotationCooldown => "Signer rotation cooldown is currently active.",
            Error::UserNotWhitelisted => "User is not whitelisted for this operation.",
            Error::UserBlacklisted => "User has been blacklisted.",
            Error::CreatorBlacklisted => "Creator has been blacklisted.",
            Error::AlreadyInitialized => "Contract is already initialized.",
            Error::InvalidTimeLockDelay => "Invalid timelock delay.",
            Error::TimeLockNotExpired => "Timelock has not yet expired.",
            Error::NoPendingUpdate => "No pending update found.",
            Error::PendingUpdateExists => "A pending update already exists.",
            Error::InvalidStakeAmount => "Invalid stake amount.",
            Error::PerLedgerBetCapExceeded => "Per-ledger bet cap exceeded.",
            Error::RegistryFull => "Registry is full.",
            Error::BatchEmpty => "Batch contains no entries; at least one bet is required.",
            Error::BatchSizeExceeded => "Batch exceeds the maximum allowed number of entries.",
            Error::TreasuryUpdateTimelocked => "Treasury update timelock has not yet expired",
            Error::NoPendingTreasuryUpdate => "No pending treasury update found",
            Error::PendingTreasuryUpdateExists => "A pending treasury update already exists",
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
            Error::IdempotentBatchAlreadyApplied => "IDEMPOTENT_BATCH_ALREADY_APPLIED",
            Error::ReasonTableFull => "REASON_TABLE_FULL",
            Error::Overflow => "OVERFLOW",
            Error::MaxBetCapExceeded => "MAX_BET_CAP_EXCEEDED",
            Error::InvalidCap => "INVALID_CAP",
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
            Error::InvalidNonce => "INVALID_NONCE",
            Error::BetAboveMaximum => "BET_ABOVE_MAXIMUM",
            Error::BetBelowMarketMin => "BET_BELOW_MARKET_MIN",
            Error::BetLimitsInverted => "BET_LIMITS_INVERTED",
            Error::BetLimitAboveMaximum => "BET_LIMIT_ABOVE_MAXIMUM",
            Error::BetCapOutOfRange => "BET_CAP_OUT_OF_RANGE",
            Error::OracleUnavailable => "ORACLE_UNAVAILABLE",
            Error::InvalidOracleConfig => "INVALID_ORACLE_CONFIG",
            Error::OracleStale => "ORACLE_STALE",
            Error::OracleNoConsensus => "ORACLE_NO_CONSENSUS",
            Error::OracleVerified => "ORACLE_VERIFIED",
            Error::MarketNotReady => "MARKET_NOT_READY",
            Error::FallbackOracleUnavailable => "FALLBACK_ORACLE_UNAVAILABLE",
            Error::ResolutionTimeoutReached => "RESOLUTION_TIMEOUT_REACHED",
            Error::OracleConfidenceTooWide => "ORACLE_CONFIDENCE_TOO_WIDE",
            Error::InvalidOracleFeed => "INVALID_ORACLE_FEED",
            Error::OracleCallbackAuthFailed => "ORACLE_CALLBACK_AUTH_FAILED",
            Error::OracleCallbackUnauthorized => "ORACLE_CALLBACK_UNAUTHORIZED",
            Error::OracleCallbackInvalidSignature => "ORACLE_CALLBACK_INVALID_SIGNATURE",
            Error::OracleCallbackReplayDetected => "ORACLE_CALLBACK_REPLAY_DETECTED",
            Error::OracleCallbackTimeout => "ORACLE_CALLBACK_TIMEOUT",
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
            Error::InvalidInitializationParams => "INVALID_INITIALIZATION_PARAMS",
            Error::DisputeError => "DISPUTE_ERROR",
            Error::DisputerCannotVote => "DISPUTER_CANNOT_VOTE",
            Error::SweepAlreadyDone => "SWEEP_ALREADY_DONE",
            Error::FeeArithmeticOverflow => "FEE_ARITHMETIC_OVERFLOW",
            Error::FeeAlreadyCollected => "FEE_ALREADY_COLLECTED",
            Error::NoFeesToCollect => "NO_FEES_TO_COLLECT",
            Error::InvalidExtensionDays => "INVALID_EXTENSION_DAYS",
            Error::ExtensionDenied => "EXTENSION_DENIED",
            Error::GasBudgetExceeded => "GAS_BUDGET_EXCEEDED",
            Error::AdminNotSet => "ADMIN_NOT_SET",
            Error::AssetDecimalsMismatch => "ASSET_DECIMALS_MISMATCH",
            Error::AdminActionTimelocked => "ADMIN_ACTION_TIMELOCKED",
            Error::OperationWouldExceedBudget => "OPERATION_WOULD_EXCEED_BUDGET",
            Error::QuestionTooLong => "QUESTION_TOO_LONG",
            Error::OutcomeTooLong => "OUTCOME_TOO_LONG",
            Error::TooManyOutcomes => "TOO_MANY_OUTCOMES",
            Error::FeedIdTooLong => "FEED_ID_TOO_LONG",
            Error::ComparisonTooLong => "COMPARISON_TOO_LONG",
            Error::CategoryTooLong => "CATEGORY_TOO_LONG",
            Error::TagTooLong => "TAG_TOO_LONG",
            Error::TooManyTags => "TOO_MANY_TAGS",
            Error::ExtensionReasonTooLong => "EXTENSION_REASON_TOO_LONG",
            Error::SourceTooLong => "SOURCE_TOO_LONG",
            Error::ErrorMessageTooLong => "ERROR_MESSAGE_TOO_LONG",
            Error::SignatureTooLong => "SIGNATURE_TOO_LONG",
            Error::TooManyExtensions => "TOO_MANY_EXTENSIONS",
            Error::TooManyOracleResults => "TOO_MANY_ORACLE_RESULTS",
            Error::TooManyWinningOutcomes => "TOO_MANY_WINNING_OUTCOMES",
            Error::ForceResolveAlreadyUsed => "FORCE_RESOLVE_ALREADY_USED",
            Error::ArchiveFull => "ARCHIVE_FULL",
            Error::CategoryTooShort => "CATEGORY_TOO_SHORT",
            Error::TagTooShort => "TAG_TOO_SHORT",
            Error::DuplicateMarketId => "DUPLICATE_MARKET_ID",
            Error::CannotArchiveFromState => "CANNOT_ARCHIVE_FROM_STATE",
            Error::CannotRestoreFromState => "CANNOT_RESTORE_FROM_STATE",
            Error::MarketAlreadyArchived => "MARKET_ALREADY_ARCHIVED",
            Error::MarketAlreadyRestored => "MARKET_ALREADY_RESTORED",
            Error::CBNotInitialized => "CIRCUIT_BREAKER_NOT_INITIALIZED",
            Error::CBAlreadyOpen => "CIRCUIT_BREAKER_ALREADY_OPEN",
            Error::CBNotOpen => "CIRCUIT_BREAKER_NOT_OPEN",
            Error::CBOpen => "CIRCUIT_BREAKER_OPEN",
            Error::CBError => "CIRCUIT_BREAKER_ERROR",
            Error::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            Error::CumulativeExtensionCapHit => "CUMULATIVE_EXTENSION_CAP_HIT",
            Error::IllegalMarketStateTransition => "ILLEGAL_MARKET_STATE_TRANSITION",
            Error::FeeExceedsMax => "FEE_ABOVE_ACCEPTABLE",
            Error::ForceResolveReplayed => "FORCE_RESOLVE_REPLAYED",
            Error::ForceResolveReasonEmpty => "FORCE_RESOLVE_REASON_EMPTY",
            Error::NoPendingFeeCommit => "NO_PENDING_FEE_COMMIT",
            Error::FeeRevealTooEarly => "FEE_REVEAL_TOO_EARLY",
            Error::FeePreimageMismatch => "FEE_PREIMAGE_MISMATCH",
            Error::DisputeStakeCapExceeded => "DISPUTE_STAKE_CAP_EXCEEDED",
            Error::InsufficientStorageRentBudget => "INSUFFICIENT_STORAGE_RENT_BUDGET",
            Error::ExtensionCapExceeded => "EXTENSION_CAP_EXCEEDED",
            Error::UpgradeChainMismatch => "UPGRADE_CHAIN_MISMATCH",
            Error::OracleQuoteOutlier => "ORACLE_QUOTE_OUTLIER",
            Error::MaxParticipantsReached => "MAX_PARTICIPANTS_REACHED",
            Error::BetExceedsCap => "BET_EXCEEDS_CAP",
            Error::ReplayedOverride => "REPLAYED_OVERRIDE",
            Error::OracleAdminCooldownActive => "ORACLE_ADMIN_COOLDOWN_ACTIVE",
            Error::SignerRotationCooldown => "SIGNER_ROTATION_COOLDOWN",
            Error::UserNotWhitelisted => "USER_NOT_WHITELISTED",
            Error::UserBlacklisted => "USER_BLACKLISTED",
            Error::CreatorBlacklisted => "CREATOR_BLACKLISTED",
            Error::AlreadyInitialized => "ALREADY_INITIALIZED",
            Error::InvalidTimeLockDelay => "INVALID_TIME_LOCK_DELAY",
            Error::TimeLockNotExpired => "TIME_LOCK_NOT_EXPIRED",
            Error::NoPendingUpdate => "NO_PENDING_UPDATE",
            Error::PendingUpdateExists => "PENDING_UPDATE_EXISTS",
            Error::InvalidStakeAmount => "INVALID_STAKE_AMOUNT",
            Error::PerLedgerBetCapExceeded => "PER_LEDGER_BET_CAP_EXCEEDED",
            Error::RegistryFull => "REGISTRY_FULL",
            Error::BatchEmpty => "BATCH_EMPTY",
            Error::BatchSizeExceeded => "BATCH_SIZE_EXCEEDED",
            Error::TreasuryUpdateTimelocked => "TREASURY_UPDATE_TIMELOCKED",
            Error::NoPendingTreasuryUpdate => "NO_PENDING_TREASURY_UPDATE",
            Error::PendingTreasuryUpdateExists => "PENDING_TREASURY_UPDATE_EXISTS",
        }
    }

    /// Returns a **stable** numeric client code for use in off-chain error
    /// handling, SDK integrations, and monitoring dashboards.
    ///
    /// # Stability guarantee
    ///
    /// Once a variant is assigned a client code, that assignment **must not
    /// change** across releases. Clients SHOULD use these codes rather than
    /// the raw `Error as u32` discriminant, whose value may shift when new
    /// variants are inserted.
    ///
    /// # Range allocation
    ///
    /// | Range      | Category          |
    /// |------------|-------------------|
    /// | 1000–1099  | Oracle            |
    /// | 1100–1199  | Market            |
    /// | 1200–1299  | Validation        |
    /// | 1300–1399  | Financial         |
    /// | 1400–1499  | Dispute           |
    /// | 1500–1599  | Authentication    |
    /// | 1600–1699  | Circuit Breaker   |
    /// | 1700–1799  | System            |
    /// | 1800–1899  | User Operation    |
    /// | 1900–1999  | Metadata / Limits |
    pub fn client_code(&self) -> u32 {
        match self {
            Error::OracleUnavailable => 1000,
            Error::InvalidOracleConfig => 1001,
            Error::OracleStale => 1002,
            Error::OracleNoConsensus => 1003,
            Error::OracleVerified => 1004,
            Error::FallbackOracleUnavailable => 1005,
            Error::ResolutionTimeoutReached => 1006,
            Error::OracleConfidenceTooWide => 1007,
            Error::InvalidOracleFeed => 1008,
            Error::OracleCallbackAuthFailed => 1009,
            Error::OracleCallbackUnauthorized => 1010,
            Error::OracleCallbackInvalidSignature => 1011,
            Error::OracleCallbackReplayDetected => 1012,
            Error::OracleCallbackTimeout => 1013,
            Error::OracleQuoteOutlier => 1014,
            Error::MarketNotFound => 1100,
            Error::MarketClosed => 1101,
            Error::MarketResolved => 1102,
            Error::MarketNotResolved => 1103,
            Error::MarketNotReady => 1104,
            Error::InvalidState => 1105,
            Error::IllegalMarketStateTransition => 1106,
            Error::DuplicateMarketId => 1107,
            Error::MaxParticipantsReached => 1108,
            Error::CannotArchiveFromState => 1109,
            Error::CannotRestoreFromState => 1110,
            Error::MarketAlreadyArchived => 1111,
            Error::MarketAlreadyRestored => 1112,
            Error::InvalidQuestion => 1200,
            Error::InvalidOutcomes => 1201,
            Error::InvalidDuration => 1202,
            Error::InvalidThreshold => 1203,
            Error::InvalidComparison => 1204,
            Error::InvalidInput => 1205,
            Error::InvalidOutcome => 1206,
            Error::AssetDecimalsMismatch => 1207,
            Error::InvalidExtensionDays => 1208,
            Error::ExtensionDenied => 1209,
            Error::CumulativeExtensionCapHit => 1210,
            Error::ExtensionCapExceeded => 1211,
            Error::InvalidInitializationParams => 1212,
            Error::InvalidNonce => 1213,
            Error::BetLimitsInverted => 1214,
            Error::BetLimitAboveMaximum => 1215,
            Error::BetCapOutOfRange => 1216,
            Error::BetAboveMaximum => 1217,
            Error::BetBelowMarketMin => 1218,
            Error::InsufficientStake => 1300,
            Error::InsufficientBalance => 1301,
            Error::NothingToClaim => 1302,
            Error::AlreadyClaimed => 1303,
            Error::FeeArithmeticOverflow => 1304,
            Error::FeeAlreadyCollected => 1305,
            Error::NoFeesToCollect => 1306,
            Error::InvalidFeeConfig => 1307,
            Error::FeeExceedsMax => 1308,
            Error::SweepAlreadyDone => 1309,
            Error::DisputeFeeFailed => 1310,
            Error::NoPendingFeeCommit => 1311,
            Error::FeeRevealTooEarly => 1312,
            Error::FeePreimageMismatch => 1313,
            Error::BetExceedsCap => 1314,
            Error::MaxBetCapExceeded => 1315,
            Error::TreasuryUpdateTimelocked => 1316,
            Error::NoPendingTreasuryUpdate => 1317,
            Error::PendingTreasuryUpdateExists => 1318,
            Error::AlreadyDisputed => 1400,
            Error::DisputeVoteExpired => 1401,
            Error::DisputeVoteDenied => 1402,
            Error::DisputeAlreadyVoted => 1403,
            Error::DisputeCondNotMet => 1404,
            Error::DisputeError => 1405,
            Error::DisputerCannotVote => 1406,
            Error::DisputeStakeCapExceeded => 1407,
            Error::Unauthorized => 1500,
            Error::ReplayedOverride => 1501,
            Error::UserNotWhitelisted => 1502,
            Error::UserBlacklisted => 1503,
            Error::CreatorBlacklisted => 1504,
            Error::CBNotInitialized => 1600,
            Error::CBAlreadyOpen => 1601,
            Error::CBNotOpen => 1602,
            Error::CBOpen => 1603,
            Error::CBError => 1604,
            Error::RateLimitExceeded => 1605,
            Error::PerLedgerBetCapExceeded => 1606,
            Error::ConfigNotFound => 1700,
            Error::AdminNotSet => 1701,
            Error::GasBudgetExceeded => 1702,
            Error::OperationWouldExceedBudget => 1703,
            Error::InsufficientStorageRentBudget => 1704,
            Error::UpgradeChainMismatch => 1705,
            Error::AlreadyInitialized => 1706,
            Error::InvalidTimeLockDelay => 1707,
            Error::TimeLockNotExpired => 1708,
            Error::NoPendingUpdate => 1709,
            Error::PendingUpdateExists => 1710,
            Error::AdminActionTimelocked => 1711,
            Error::OracleAdminCooldownActive => 1712,
            Error::SignerRotationCooldown => 1713,
            Error::Overflow => 1714,
            Error::InvalidCap => 1715,
            Error::BatchEmpty => 1716,
            Error::BatchSizeExceeded => 1717,
            Error::AlreadyVoted => 1800,
            Error::AlreadyBet => 1801,
            Error::BetsAlreadyPlaced => 1802,
            Error::ForceResolveAlreadyUsed => 1803,
            Error::ForceResolveReplayed => 1804,
            Error::ForceResolveReasonEmpty => 1805,
            Error::IdempotentBatchAlreadyApplied => 1806,
            Error::InvalidStakeAmount => 1807,
            Error::QuestionTooLong => 1900,
            Error::OutcomeTooLong => 1901,
            Error::TooManyOutcomes => 1902,
            Error::FeedIdTooLong => 1903,
            Error::ComparisonTooLong => 1904,
            Error::CategoryTooLong => 1905,
            Error::CategoryTooShort => 1906,
            Error::TagTooLong => 1907,
            Error::TagTooShort => 1908,
            Error::TooManyTags => 1909,
            Error::ExtensionReasonTooLong => 1910,
            Error::SourceTooLong => 1911,
            Error::ErrorMessageTooLong => 1912,
            Error::SignatureTooLong => 1913,
            Error::TooManyExtensions => 1914,
            Error::TooManyOracleResults => 1915,
            Error::TooManyWinningOutcomes => 1916,
            Error::ArchiveFull => 1917,
            Error::ReasonTableFull => 1918,
            Error::RegistryFull => 1919,
        }
    }

    /// Returns the off-chain recoverability label for this error.
    ///
    /// # Stability guarantee
    ///
    /// The label for each variant is part of the public API and **must not
    /// change** once assigned. Client SDKs use these labels to decide retry
    /// policy without having to maintain their own copy of the mapping.
    ///
    /// # Labels
    ///
    /// | Label           | Meaning                                     |
    /// |-----------------|---------------------------------------------|
    /// | `Retryable`     | Transient; caller MAY retry with back-off.  |
    /// | `RequiresAdmin` | Needs operator/admin action before retry.   |
    /// | `Terminal`      | Permanent; caller MUST NOT retry.           |
    pub fn recoverability(&self) -> Recoverability {
        match self {
            Error::ReasonTableFull
            | Error::InsufficientStake
            | Error::InvalidOutcome
            | Error::OracleUnavailable
            | Error::OracleStale
            | Error::OracleNoConsensus
            | Error::MarketNotReady
            | Error::FallbackOracleUnavailable
            | Error::ResolutionTimeoutReached
            | Error::OracleConfidenceTooWide
            | Error::OracleCallbackTimeout
            | Error::InvalidQuestion
            | Error::InvalidOutcomes
            | Error::InvalidDuration
            | Error::InvalidThreshold
            | Error::InvalidComparison
            | Error::InvalidInput
            | Error::QuestionTooLong
            | Error::OutcomeTooLong
            | Error::TooManyOutcomes
            | Error::FeedIdTooLong
            | Error::ComparisonTooLong
            | Error::CategoryTooLong
            | Error::TagTooLong
            | Error::TooManyTags
            | Error::ExtensionReasonTooLong
            | Error::SourceTooLong
            | Error::ErrorMessageTooLong
            | Error::SignatureTooLong
            | Error::TooManyExtensions
            | Error::TooManyOracleResults
            | Error::TooManyWinningOutcomes
            | Error::ArchiveFull
            | Error::CategoryTooShort
            | Error::TagTooShort
            | Error::CBOpen
            | Error::RateLimitExceeded
            | Error::FeeExceedsMax
            | Error::ForceResolveReplayed
            | Error::ForceResolveReasonEmpty
            | Error::FeeRevealTooEarly
            | Error::InsufficientStorageRentBudget
            | Error::OracleAdminCooldownActive
            | Error::SignerRotationCooldown
            | Error::TimeLockNotExpired
            | Error::InvalidStakeAmount
            | Error::PerLedgerBetCapExceeded
            | Error::RegistryFull
            | Error::BatchEmpty
            | Error::BatchSizeExceeded => Recoverability::Retryable,

            Error::InvalidOracleConfig
            | Error::InvalidOracleFeed
            | Error::InvalidFeeConfig
            | Error::ConfigNotFound
            | Error::DisputeFeeFailed
            | Error::AdminNotSet
            | Error::AssetDecimalsMismatch
            | Error::AdminActionTimelocked
            | Error::CBNotInitialized
            | Error::CBAlreadyOpen
            | Error::CBNotOpen
            | Error::CBError
            | Error::NoPendingFeeCommit
            | Error::UpgradeChainMismatch
            | Error::NoPendingUpdate
            | Error::PendingUpdateExists => Recoverability::RequiresAdmin,

            _ => Recoverability::Terminal,
        }
    }

    /// Resolves an `Error` variant from a raw on-chain contract error code (`u32`).
    /// Resolves an `Error` variant from a raw on-chain contract error code (`u32`).
    /// Resolves an `Error` variant from a raw on-chain contract error code (`u32`).
    pub fn from_contract_code(code: u32) -> Option<Error> {
        match code {
            100 => Some(Error::Unauthorized),
            101 => Some(Error::MarketNotFound),
            102 => Some(Error::MarketClosed),
            103 => Some(Error::MarketResolved),
            104 => Some(Error::MarketNotResolved),
            105 => Some(Error::NothingToClaim),
            106 => Some(Error::AlreadyClaimed),
            107 => Some(Error::InsufficientStake),
            108 => Some(Error::InvalidOutcome),
            109 => Some(Error::AlreadyVoted),
            110 => Some(Error::AlreadyBet),
            111 => Some(Error::BetsAlreadyPlaced),
            112 => Some(Error::InsufficientBalance),
            113 => Some(Error::InvalidNonce),
            114 => Some(Error::BetAboveMaximum),
            115 => Some(Error::BetBelowMarketMin),
            116 => Some(Error::BetLimitsInverted),
            117 => Some(Error::BetLimitAboveMaximum),
            118 => Some(Error::BetCapOutOfRange),
            200 => Some(Error::OracleUnavailable),
            201 => Some(Error::InvalidOracleConfig),
            202 => Some(Error::OracleStale),
            203 => Some(Error::OracleNoConsensus),
            204 => Some(Error::OracleVerified),
            205 => Some(Error::MarketNotReady),
            206 => Some(Error::FallbackOracleUnavailable),
            207 => Some(Error::ResolutionTimeoutReached),
            208 => Some(Error::OracleConfidenceTooWide),
            209 => Some(Error::InvalidOracleFeed),
            210 => Some(Error::OracleCallbackAuthFailed),
            211 => Some(Error::OracleCallbackUnauthorized),
            212 => Some(Error::OracleCallbackInvalidSignature),
            213 => Some(Error::OracleCallbackReplayDetected),
            214 => Some(Error::OracleCallbackTimeout),
            300 => Some(Error::InvalidQuestion),
            301 => Some(Error::InvalidOutcomes),
            302 => Some(Error::InvalidDuration),
            303 => Some(Error::InvalidThreshold),
            304 => Some(Error::InvalidComparison),
            400 => Some(Error::InvalidState),
            401 => Some(Error::InvalidInput),
            402 => Some(Error::InvalidFeeConfig),
            403 => Some(Error::ConfigNotFound),
            404 => Some(Error::AlreadyDisputed),
            405 => Some(Error::DisputeVoteExpired),
            406 => Some(Error::DisputeVoteDenied),
            407 => Some(Error::DisputeAlreadyVoted),
            408 => Some(Error::DisputeCondNotMet),
            409 => Some(Error::DisputeFeeFailed),
            410 => Some(Error::DisputeError),
            411 => Some(Error::SweepAlreadyDone),
            412 => Some(Error::FeeArithmeticOverflow),
            413 => Some(Error::FeeAlreadyCollected),
            414 => Some(Error::NoFeesToCollect),
            415 => Some(Error::InvalidExtensionDays),
            416 => Some(Error::ExtensionDenied),
            417 => Some(Error::GasBudgetExceeded),
            418 => Some(Error::AdminNotSet),
            420 => Some(Error::QuestionTooLong),
            421 => Some(Error::OutcomeTooLong),
            422 => Some(Error::TooManyOutcomes),
            423 => Some(Error::FeedIdTooLong),
            424 => Some(Error::ComparisonTooLong),
            425 => Some(Error::CategoryTooLong),
            426 => Some(Error::TagTooLong),
            427 => Some(Error::TooManyTags),
            428 => Some(Error::ExtensionReasonTooLong),
            429 => Some(Error::SourceTooLong),
            430 => Some(Error::ErrorMessageTooLong),
            431 => Some(Error::SignatureTooLong),
            432 => Some(Error::TooManyExtensions),
            433 => Some(Error::TooManyOracleResults),
            434 => Some(Error::TooManyWinningOutcomes),
            435 => Some(Error::ForceResolveAlreadyUsed),
            436 => Some(Error::CategoryTooShort),
            437 => Some(Error::TagTooShort),
            438 => Some(Error::DisputerCannotVote),
            439 => Some(Error::AssetDecimalsMismatch),
            440 => Some(Error::ArchiveFull),
            441 => Some(Error::DuplicateMarketId),
            442 => Some(Error::CannotArchiveFromState),
            443 => Some(Error::AdminActionTimelocked),
            444 => Some(Error::OperationWouldExceedBudget),
            445 => Some(Error::MarketAlreadyArchived),
            446 => Some(Error::MarketAlreadyRestored),
            447 => Some(Error::CannotRestoreFromState),
            500 => Some(Error::CBNotInitialized),
            501 => Some(Error::CBAlreadyOpen),
            502 => Some(Error::CBNotOpen),
            503 => Some(Error::CBOpen),
            504 => Some(Error::CBError),
            505 => Some(Error::RateLimitExceeded),
            506 => Some(Error::CumulativeExtensionCapHit),
            507 => Some(Error::IllegalMarketStateTransition),
            508 => Some(Error::FeeExceedsMax),
            517 => Some(Error::ForceResolveReplayed),
            518 => Some(Error::ForceResolveReasonEmpty),
            519 => Some(Error::NoPendingFeeCommit),
            520 => Some(Error::FeeRevealTooEarly),
            521 => Some(Error::FeePreimageMismatch),
            522 => Some(Error::DisputeStakeCapExceeded),
            523 => Some(Error::InsufficientStorageRentBudget),
            524 => Some(Error::ExtensionCapExceeded),
            525 => Some(Error::UpgradeChainMismatch),
            526 => Some(Error::ReplayedOverride),
            527 => Some(Error::OracleQuoteOutlier),
            528 => Some(Error::MaxParticipantsReached),
            545 => Some(Error::BatchEmpty),
            546 => Some(Error::BatchSizeExceeded),
            660 => Some(Error::IdempotentBatchAlreadyApplied),
            670 => Some(Error::ReasonTableFull),
            672 => Some(Error::Overflow),
            673 => Some(Error::MaxBetCapExceeded),
            674 => Some(Error::InvalidCap),
            675 => Some(Error::BetExceedsCap),
            676 => Some(Error::OracleAdminCooldownActive),
            677 => Some(Error::SignerRotationCooldown),
            678 => Some(Error::UserNotWhitelisted),
            679 => Some(Error::UserBlacklisted),
            680 => Some(Error::CreatorBlacklisted),
            681 => Some(Error::AlreadyInitialized),
            682 => Some(Error::InvalidTimeLockDelay),
            683 => Some(Error::TimeLockNotExpired),
            684 => Some(Error::NoPendingUpdate),
            685 => Some(Error::PendingUpdateExists),
            686 => Some(Error::InvalidStakeAmount),
            687 => Some(Error::PerLedgerBetCapExceeded),
            688 => Some(Error::RegistryFull),
            689 => Some(Error::TreasuryUpdateTimelocked),
            690 => Some(Error::NoPendingTreasuryUpdate),
            691 => Some(Error::PendingTreasuryUpdateExists),
            700 => Some(Error::InvalidInitializationParams),
            _ => None,
        }
    }

    /// Resolves an `Error` variant from an off-chain `client_code` (`u32`).
    pub fn from_client_code(code: u32) -> Option<Error> {
        match code {
            1000 => Some(Error::OracleUnavailable),
            1001 => Some(Error::InvalidOracleConfig),
            1002 => Some(Error::OracleStale),
            1003 => Some(Error::OracleNoConsensus),
            1004 => Some(Error::OracleVerified),
            1005 => Some(Error::FallbackOracleUnavailable),
            1006 => Some(Error::ResolutionTimeoutReached),
            1007 => Some(Error::OracleConfidenceTooWide),
            1008 => Some(Error::InvalidOracleFeed),
            1009 => Some(Error::OracleCallbackAuthFailed),
            1010 => Some(Error::OracleCallbackUnauthorized),
            1011 => Some(Error::OracleCallbackInvalidSignature),
            1012 => Some(Error::OracleCallbackReplayDetected),
            1013 => Some(Error::OracleCallbackTimeout),
            1014 => Some(Error::OracleQuoteOutlier),
            1100 => Some(Error::MarketNotFound),
            1101 => Some(Error::MarketClosed),
            1102 => Some(Error::MarketResolved),
            1103 => Some(Error::MarketNotResolved),
            1104 => Some(Error::MarketNotReady),
            1105 => Some(Error::InvalidState),
            1106 => Some(Error::IllegalMarketStateTransition),
            1107 => Some(Error::DuplicateMarketId),
            1108 => Some(Error::MaxParticipantsReached),
            1109 => Some(Error::CannotArchiveFromState),
            1110 => Some(Error::CannotRestoreFromState),
            1111 => Some(Error::MarketAlreadyArchived),
            1112 => Some(Error::MarketAlreadyRestored),
            1200 => Some(Error::InvalidQuestion),
            1201 => Some(Error::InvalidOutcomes),
            1202 => Some(Error::InvalidDuration),
            1203 => Some(Error::InvalidThreshold),
            1204 => Some(Error::InvalidComparison),
            1205 => Some(Error::InvalidInput),
            1206 => Some(Error::InvalidOutcome),
            1207 => Some(Error::AssetDecimalsMismatch),
            1208 => Some(Error::InvalidExtensionDays),
            1209 => Some(Error::ExtensionDenied),
            1210 => Some(Error::CumulativeExtensionCapHit),
            1211 => Some(Error::ExtensionCapExceeded),
            1212 => Some(Error::InvalidInitializationParams),
            1213 => Some(Error::InvalidNonce),
            1214 => Some(Error::BetLimitsInverted),
            1215 => Some(Error::BetLimitAboveMaximum),
            1216 => Some(Error::BetCapOutOfRange),
            1217 => Some(Error::BetAboveMaximum),
            1218 => Some(Error::BetBelowMarketMin),
            1300 => Some(Error::InsufficientStake),
            1301 => Some(Error::InsufficientBalance),
            1302 => Some(Error::NothingToClaim),
            1303 => Some(Error::AlreadyClaimed),
            1304 => Some(Error::FeeArithmeticOverflow),
            1305 => Some(Error::FeeAlreadyCollected),
            1306 => Some(Error::NoFeesToCollect),
            1307 => Some(Error::InvalidFeeConfig),
            1308 => Some(Error::FeeExceedsMax),
            1309 => Some(Error::SweepAlreadyDone),
            1310 => Some(Error::DisputeFeeFailed),
            1311 => Some(Error::NoPendingFeeCommit),
            1312 => Some(Error::FeeRevealTooEarly),
            1313 => Some(Error::FeePreimageMismatch),
            1314 => Some(Error::BetExceedsCap),
            1315 => Some(Error::MaxBetCapExceeded),
            1316 => Some(Error::TreasuryUpdateTimelocked),
            1317 => Some(Error::NoPendingTreasuryUpdate),
            1318 => Some(Error::PendingTreasuryUpdateExists),
            1400 => Some(Error::AlreadyDisputed),
            1401 => Some(Error::DisputeVoteExpired),
            1402 => Some(Error::DisputeVoteDenied),
            1403 => Some(Error::DisputeAlreadyVoted),
            1404 => Some(Error::DisputeCondNotMet),
            1405 => Some(Error::DisputeError),
            1406 => Some(Error::DisputerCannotVote),
            1407 => Some(Error::DisputeStakeCapExceeded),
            1500 => Some(Error::Unauthorized),
            1501 => Some(Error::ReplayedOverride),
            1502 => Some(Error::UserNotWhitelisted),
            1503 => Some(Error::UserBlacklisted),
            1504 => Some(Error::CreatorBlacklisted),
            1600 => Some(Error::CBNotInitialized),
            1601 => Some(Error::CBAlreadyOpen),
            1602 => Some(Error::CBNotOpen),
            1603 => Some(Error::CBOpen),
            1604 => Some(Error::CBError),
            1605 => Some(Error::RateLimitExceeded),
            1606 => Some(Error::PerLedgerBetCapExceeded),
            1700 => Some(Error::ConfigNotFound),
            1701 => Some(Error::AdminNotSet),
            1702 => Some(Error::GasBudgetExceeded),
            1703 => Some(Error::OperationWouldExceedBudget),
            1704 => Some(Error::InsufficientStorageRentBudget),
            1705 => Some(Error::UpgradeChainMismatch),
            1706 => Some(Error::AlreadyInitialized),
            1707 => Some(Error::InvalidTimeLockDelay),
            1708 => Some(Error::TimeLockNotExpired),
            1709 => Some(Error::NoPendingUpdate),
            1710 => Some(Error::PendingUpdateExists),
            1711 => Some(Error::AdminActionTimelocked),
            1712 => Some(Error::OracleAdminCooldownActive),
            1713 => Some(Error::SignerRotationCooldown),
            1714 => Some(Error::Overflow),
            1715 => Some(Error::InvalidCap),
            1716 => Some(Error::BatchEmpty),
            1717 => Some(Error::BatchSizeExceeded),
            1800 => Some(Error::AlreadyVoted),
            1801 => Some(Error::AlreadyBet),
            1802 => Some(Error::BetsAlreadyPlaced),
            1803 => Some(Error::ForceResolveAlreadyUsed),
            1804 => Some(Error::ForceResolveReplayed),
            1805 => Some(Error::ForceResolveReasonEmpty),
            1806 => Some(Error::IdempotentBatchAlreadyApplied),
            1807 => Some(Error::InvalidStakeAmount),
            1900 => Some(Error::QuestionTooLong),
            1901 => Some(Error::OutcomeTooLong),
            1902 => Some(Error::TooManyOutcomes),
            1903 => Some(Error::FeedIdTooLong),
            1904 => Some(Error::ComparisonTooLong),
            1905 => Some(Error::CategoryTooLong),
            1906 => Some(Error::CategoryTooShort),
            1907 => Some(Error::TagTooLong),
            1908 => Some(Error::TagTooShort),
            1909 => Some(Error::TooManyTags),
            1910 => Some(Error::ExtensionReasonTooLong),
            1911 => Some(Error::SourceTooLong),
            1912 => Some(Error::ErrorMessageTooLong),
            1913 => Some(Error::SignatureTooLong),
            1914 => Some(Error::TooManyExtensions),
            1915 => Some(Error::TooManyOracleResults),
            1916 => Some(Error::TooManyWinningOutcomes),
            1917 => Some(Error::ArchiveFull),
            1918 => Some(Error::ReasonTableFull),
            1919 => Some(Error::RegistryFull),
            _ => None,
        }
    }

    /// Resolves an `Error` variant from its canonical uppercase string code.
    pub fn from_code_str(code: &str) -> Option<Error> {
        match code {
            "IDEMPOTENT_BATCH_ALREADY_APPLIED" => Some(Error::IdempotentBatchAlreadyApplied),
            "REASON_TABLE_FULL" => Some(Error::ReasonTableFull),
            "OVERFLOW" => Some(Error::Overflow),
            "MAX_BET_CAP_EXCEEDED" => Some(Error::MaxBetCapExceeded),
            "INVALID_CAP" => Some(Error::InvalidCap),
            "UNAUTHORIZED" => Some(Error::Unauthorized),
            "MARKET_NOT_FOUND" => Some(Error::MarketNotFound),
            "MARKET_CLOSED" => Some(Error::MarketClosed),
            "MARKET_ALREADY_RESOLVED" => Some(Error::MarketResolved),
            "MARKET_NOT_RESOLVED" => Some(Error::MarketNotResolved),
            "NOTHING_TO_CLAIM" => Some(Error::NothingToClaim),
            "ALREADY_CLAIMED" => Some(Error::AlreadyClaimed),
            "INSUFFICIENT_STAKE" => Some(Error::InsufficientStake),
            "INVALID_OUTCOME" => Some(Error::InvalidOutcome),
            "ALREADY_VOTED" => Some(Error::AlreadyVoted),
            "ALREADY_BET" => Some(Error::AlreadyBet),
            "BETS_ALREADY_PLACED" => Some(Error::BetsAlreadyPlaced),
            "INSUFFICIENT_BALANCE" => Some(Error::InsufficientBalance),
            "INVALID_NONCE" => Some(Error::InvalidNonce),
            "BET_ABOVE_MAXIMUM" => Some(Error::BetAboveMaximum),
            "BET_BELOW_MARKET_MIN" => Some(Error::BetBelowMarketMin),
            "BET_LIMITS_INVERTED" => Some(Error::BetLimitsInverted),
            "BET_LIMIT_ABOVE_MAXIMUM" => Some(Error::BetLimitAboveMaximum),
            "BET_CAP_OUT_OF_RANGE" => Some(Error::BetCapOutOfRange),
            "ORACLE_UNAVAILABLE" => Some(Error::OracleUnavailable),
            "INVALID_ORACLE_CONFIG" => Some(Error::InvalidOracleConfig),
            "ORACLE_STALE" => Some(Error::OracleStale),
            "ORACLE_NO_CONSENSUS" => Some(Error::OracleNoConsensus),
            "ORACLE_VERIFIED" => Some(Error::OracleVerified),
            "MARKET_NOT_READY" => Some(Error::MarketNotReady),
            "FALLBACK_ORACLE_UNAVAILABLE" => Some(Error::FallbackOracleUnavailable),
            "RESOLUTION_TIMEOUT_REACHED" => Some(Error::ResolutionTimeoutReached),
            "ORACLE_CONFIDENCE_TOO_WIDE" => Some(Error::OracleConfidenceTooWide),
            "INVALID_ORACLE_FEED" => Some(Error::InvalidOracleFeed),
            "ORACLE_CALLBACK_AUTH_FAILED" => Some(Error::OracleCallbackAuthFailed),
            "ORACLE_CALLBACK_UNAUTHORIZED" => Some(Error::OracleCallbackUnauthorized),
            "ORACLE_CALLBACK_INVALID_SIGNATURE" => Some(Error::OracleCallbackInvalidSignature),
            "ORACLE_CALLBACK_REPLAY_DETECTED" => Some(Error::OracleCallbackReplayDetected),
            "ORACLE_CALLBACK_TIMEOUT" => Some(Error::OracleCallbackTimeout),
            "INVALID_QUESTION" => Some(Error::InvalidQuestion),
            "INVALID_OUTCOMES" => Some(Error::InvalidOutcomes),
            "INVALID_DURATION" => Some(Error::InvalidDuration),
            "INVALID_THRESHOLD" => Some(Error::InvalidThreshold),
            "INVALID_COMPARISON" => Some(Error::InvalidComparison),
            "INVALID_STATE" => Some(Error::InvalidState),
            "INVALID_INPUT" => Some(Error::InvalidInput),
            "INVALID_FEE_CONFIG" => Some(Error::InvalidFeeConfig),
            "CONFIGURATION_NOT_FOUND" => Some(Error::ConfigNotFound),
            "ALREADY_DISPUTED" => Some(Error::AlreadyDisputed),
            "DISPUTE_VOTING_PERIOD_EXPIRED" => Some(Error::DisputeVoteExpired),
            "DISPUTE_VOTING_NOT_ALLOWED" => Some(Error::DisputeVoteDenied),
            "DISPUTE_ALREADY_VOTED" => Some(Error::DisputeAlreadyVoted),
            "DISPUTE_RESOLUTION_CONDITIONS_NOT_MET" => Some(Error::DisputeCondNotMet),
            "DISPUTE_FEE_DISTRIBUTION_FAILED" => Some(Error::DisputeFeeFailed),
            "INVALID_INITIALIZATION_PARAMS" => Some(Error::InvalidInitializationParams),
            "DISPUTE_ERROR" => Some(Error::DisputeError),
            "DISPUTER_CANNOT_VOTE" => Some(Error::DisputerCannotVote),
            "SWEEP_ALREADY_DONE" => Some(Error::SweepAlreadyDone),
            "FEE_ARITHMETIC_OVERFLOW" => Some(Error::FeeArithmeticOverflow),
            "FEE_ALREADY_COLLECTED" => Some(Error::FeeAlreadyCollected),
            "NO_FEES_TO_COLLECT" => Some(Error::NoFeesToCollect),
            "INVALID_EXTENSION_DAYS" => Some(Error::InvalidExtensionDays),
            "EXTENSION_DENIED" => Some(Error::ExtensionDenied),
            "GAS_BUDGET_EXCEEDED" => Some(Error::GasBudgetExceeded),
            "ADMIN_NOT_SET" => Some(Error::AdminNotSet),
            "ASSET_DECIMALS_MISMATCH" => Some(Error::AssetDecimalsMismatch),
            "ADMIN_ACTION_TIMELOCKED" => Some(Error::AdminActionTimelocked),
            "OPERATION_WOULD_EXCEED_BUDGET" => Some(Error::OperationWouldExceedBudget),
            "QUESTION_TOO_LONG" => Some(Error::QuestionTooLong),
            "OUTCOME_TOO_LONG" => Some(Error::OutcomeTooLong),
            "TOO_MANY_OUTCOMES" => Some(Error::TooManyOutcomes),
            "FEED_ID_TOO_LONG" => Some(Error::FeedIdTooLong),
            "COMPARISON_TOO_LONG" => Some(Error::ComparisonTooLong),
            "CATEGORY_TOO_LONG" => Some(Error::CategoryTooLong),
            "TAG_TOO_LONG" => Some(Error::TagTooLong),
            "TOO_MANY_TAGS" => Some(Error::TooManyTags),
            "EXTENSION_REASON_TOO_LONG" => Some(Error::ExtensionReasonTooLong),
            "SOURCE_TOO_LONG" => Some(Error::SourceTooLong),
            "ERROR_MESSAGE_TOO_LONG" => Some(Error::ErrorMessageTooLong),
            "SIGNATURE_TOO_LONG" => Some(Error::SignatureTooLong),
            "TOO_MANY_EXTENSIONS" => Some(Error::TooManyExtensions),
            "TOO_MANY_ORACLE_RESULTS" => Some(Error::TooManyOracleResults),
            "TOO_MANY_WINNING_OUTCOMES" => Some(Error::TooManyWinningOutcomes),
            "FORCE_RESOLVE_ALREADY_USED" => Some(Error::ForceResolveAlreadyUsed),
            "ARCHIVE_FULL" => Some(Error::ArchiveFull),
            "CATEGORY_TOO_SHORT" => Some(Error::CategoryTooShort),
            "TAG_TOO_SHORT" => Some(Error::TagTooShort),
            "DUPLICATE_MARKET_ID" => Some(Error::DuplicateMarketId),
            "CANNOT_ARCHIVE_FROM_STATE" => Some(Error::CannotArchiveFromState),
            "CANNOT_RESTORE_FROM_STATE" => Some(Error::CannotRestoreFromState),
            "MARKET_ALREADY_ARCHIVED" => Some(Error::MarketAlreadyArchived),
            "MARKET_ALREADY_RESTORED" => Some(Error::MarketAlreadyRestored),
            "CIRCUIT_BREAKER_NOT_INITIALIZED" => Some(Error::CBNotInitialized),
            "CIRCUIT_BREAKER_ALREADY_OPEN" => Some(Error::CBAlreadyOpen),
            "CIRCUIT_BREAKER_NOT_OPEN" => Some(Error::CBNotOpen),
            "CIRCUIT_BREAKER_OPEN" => Some(Error::CBOpen),
            "CIRCUIT_BREAKER_ERROR" => Some(Error::CBError),
            "RATE_LIMIT_EXCEEDED" => Some(Error::RateLimitExceeded),
            "CUMULATIVE_EXTENSION_CAP_HIT" => Some(Error::CumulativeExtensionCapHit),
            "ILLEGAL_MARKET_STATE_TRANSITION" => Some(Error::IllegalMarketStateTransition),
            "FEE_ABOVE_ACCEPTABLE" => Some(Error::FeeExceedsMax),
            "FORCE_RESOLVE_REPLAYED" => Some(Error::ForceResolveReplayed),
            "FORCE_RESOLVE_REASON_EMPTY" => Some(Error::ForceResolveReasonEmpty),
            "NO_PENDING_FEE_COMMIT" => Some(Error::NoPendingFeeCommit),
            "FEE_REVEAL_TOO_EARLY" => Some(Error::FeeRevealTooEarly),
            "FEE_PREIMAGE_MISMATCH" => Some(Error::FeePreimageMismatch),
            "DISPUTE_STAKE_CAP_EXCEEDED" => Some(Error::DisputeStakeCapExceeded),
            "INSUFFICIENT_STORAGE_RENT_BUDGET" => Some(Error::InsufficientStorageRentBudget),
            "EXTENSION_CAP_EXCEEDED" => Some(Error::ExtensionCapExceeded),
            "UPGRADE_CHAIN_MISMATCH" => Some(Error::UpgradeChainMismatch),
            "ORACLE_QUOTE_OUTLIER" => Some(Error::OracleQuoteOutlier),
            "MAX_PARTICIPANTS_REACHED" => Some(Error::MaxParticipantsReached),
            "BET_EXCEEDS_CAP" => Some(Error::BetExceedsCap),
            "REPLAYED_OVERRIDE" => Some(Error::ReplayedOverride),
            "ORACLE_ADMIN_COOLDOWN_ACTIVE" => Some(Error::OracleAdminCooldownActive),
            "SIGNER_ROTATION_COOLDOWN" => Some(Error::SignerRotationCooldown),
            "USER_NOT_WHITELISTED" => Some(Error::UserNotWhitelisted),
            "USER_BLACKLISTED" => Some(Error::UserBlacklisted),
            "CREATOR_BLACKLISTED" => Some(Error::CreatorBlacklisted),
            "ALREADY_INITIALIZED" => Some(Error::AlreadyInitialized),
            "INVALID_TIME_LOCK_DELAY" => Some(Error::InvalidTimeLockDelay),
            "TIME_LOCK_NOT_EXPIRED" => Some(Error::TimeLockNotExpired),
            "NO_PENDING_UPDATE" => Some(Error::NoPendingUpdate),
            "PENDING_UPDATE_EXISTS" => Some(Error::PendingUpdateExists),
            "INVALID_STAKE_AMOUNT" => Some(Error::InvalidStakeAmount),
            "PER_LEDGER_BET_CAP_EXCEEDED" => Some(Error::PerLedgerBetCapExceeded),
            "REGISTRY_FULL" => Some(Error::RegistryFull),
            "BATCH_EMPTY" => Some(Error::BatchEmpty),
            "BATCH_SIZE_EXCEEDED" => Some(Error::BatchSizeExceeded),
            "TREASURY_UPDATE_TIMELOCKED" => Some(Error::TreasuryUpdateTimelocked),
            "NO_PENDING_TREASURY_UPDATE" => Some(Error::NoPendingTreasuryUpdate),
            "PENDING_TREASURY_UPDATE_EXISTS" => Some(Error::PendingTreasuryUpdateExists),
            _ => None,
        }
    }

    /// Generates full public error mapping metadata for this error.
    pub fn public_mapping(&self) -> PublicErrorMapping {
        PublicErrorMapping {
            contract_code: *self as u32,
            client_code: self.client_code(),
            code_str: self.code(),
            description: self.description(),
            category: self.category(),
            severity: self.severity(),
            recoverability: self.recoverability(),
            is_known: true,
        }
    }

    /// Returns the category for this error based on its client code range.
    pub fn category(&self) -> ErrorCategory {
        match self.client_code() {
            1000..=1099 => ErrorCategory::Oracle,
            1100..=1199 => ErrorCategory::Market,
            1200..=1299 => ErrorCategory::Validation,
            1300..=1399 => ErrorCategory::Financial,
            1400..=1499 => ErrorCategory::Dispute,
            1500..=1599 => ErrorCategory::Authentication,
            1600..=1799 => ErrorCategory::System,
            1800..=1899 => ErrorCategory::UserOperation,
            1900..=1999 => ErrorCategory::Validation,
            _ => ErrorCategory::Unknown,
        }
    }

    /// Returns the severity level for this error.
    pub fn severity(&self) -> ErrorSeverity {
        ErrorHandler::get_error_classification(self).0
    }

    /// Safely decodes a contract code into a `PublicErrorMapping`.
    pub fn decode_contract_code(code: u32) -> PublicErrorMapping {
        match Self::from_contract_code(code) {
            Some(err) => err.public_mapping(),
            None => PublicErrorMapping {
                contract_code: code,
                client_code: 0,
                code_str: "UNKNOWN_ERROR",
                description: "Unknown error code",
                category: ErrorCategory::Unknown,
                severity: ErrorSeverity::Medium,
                recoverability: Recoverability::Terminal,
                is_known: false,
            },
        }
    }

    /// Safely decodes an off-chain client code into a `PublicErrorMapping`.
    pub fn decode_client_code(code: u32) -> PublicErrorMapping {
        match Self::from_client_code(code) {
            Some(err) => err.public_mapping(),
            None => PublicErrorMapping {
                contract_code: 0,
                client_code: code,
                code_str: "UNKNOWN_ERROR",
                description: "Unknown client code",
                category: ErrorCategory::Unknown,
                severity: ErrorSeverity::Medium,
                recoverability: Recoverability::Terminal,
                is_known: false,
            },
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
            Error::InvalidNonce,
            Error::BetAboveMaximum,
            Error::BetBelowMarketMin,
            Error::BetLimitsInverted,
            Error::BetLimitAboveMaximum,
            Error::BetCapOutOfRange,
            Error::OracleUnavailable,
            Error::InvalidOracleConfig,
            Error::OracleStale,
            Error::OracleNoConsensus,
            Error::OracleVerified,
            Error::MarketNotReady,
            Error::FallbackOracleUnavailable,
            Error::ResolutionTimeoutReached,
            Error::OracleConfidenceTooWide,
            Error::InvalidOracleFeed,
            Error::OracleCallbackAuthFailed,
            Error::OracleCallbackUnauthorized,
            Error::OracleCallbackInvalidSignature,
            Error::OracleCallbackReplayDetected,
            Error::OracleCallbackTimeout,
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
            Error::SweepAlreadyDone,
            Error::FeeArithmeticOverflow,
            Error::FeeAlreadyCollected,
            Error::NoFeesToCollect,
            Error::InvalidExtensionDays,
            Error::ExtensionDenied,
            Error::GasBudgetExceeded,
            Error::AdminNotSet,
            Error::QuestionTooLong,
            Error::OutcomeTooLong,
            Error::TooManyOutcomes,
            Error::FeedIdTooLong,
            Error::ComparisonTooLong,
            Error::CategoryTooLong,
            Error::TagTooLong,
            Error::TooManyTags,
            Error::ExtensionReasonTooLong,
            Error::SourceTooLong,
            Error::ErrorMessageTooLong,
            Error::SignatureTooLong,
            Error::TooManyExtensions,
            Error::TooManyOracleResults,
            Error::TooManyWinningOutcomes,
            Error::ForceResolveAlreadyUsed,
            Error::CategoryTooShort,
            Error::TagTooShort,
            Error::DisputerCannotVote,
            Error::AssetDecimalsMismatch,
            Error::ArchiveFull,
            Error::DuplicateMarketId,
            Error::CannotArchiveFromState,
            Error::AdminActionTimelocked,
            Error::OperationWouldExceedBudget,
            Error::MarketAlreadyArchived,
            Error::MarketAlreadyRestored,
            Error::CannotRestoreFromState,
            Error::CBNotInitialized,
            Error::CBAlreadyOpen,
            Error::CBNotOpen,
            Error::CBOpen,
            Error::CBError,
            Error::RateLimitExceeded,
            Error::CumulativeExtensionCapHit,
            Error::IllegalMarketStateTransition,
            Error::FeeExceedsMax,
            Error::ForceResolveReplayed,
            Error::ForceResolveReasonEmpty,
            Error::NoPendingFeeCommit,
            Error::FeeRevealTooEarly,
            Error::FeePreimageMismatch,
            Error::DisputeStakeCapExceeded,
            Error::InsufficientStorageRentBudget,
            Error::ExtensionCapExceeded,
            Error::UpgradeChainMismatch,
            Error::ReplayedOverride,
            Error::OracleQuoteOutlier,
            Error::MaxParticipantsReached,
            Error::BatchEmpty,
            Error::BatchSizeExceeded,
            Error::IdempotentBatchAlreadyApplied,
            Error::ReasonTableFull,
            Error::Overflow,
            Error::MaxBetCapExceeded,
            Error::InvalidCap,
            Error::BetExceedsCap,
            Error::OracleAdminCooldownActive,
            Error::SignerRotationCooldown,
            Error::UserNotWhitelisted,
            Error::UserBlacklisted,
            Error::CreatorBlacklisted,
            Error::AlreadyInitialized,
            Error::InvalidTimeLockDelay,
            Error::TimeLockNotExpired,
            Error::NoPendingUpdate,
            Error::PendingUpdateExists,
            Error::InvalidStakeAmount,
            Error::PerLedgerBetCapExceeded,
            Error::RegistryFull,
            Error::TreasuryUpdateTimelocked,
            Error::NoPendingTreasuryUpdate,
            Error::PendingTreasuryUpdateExists,
            Error::InvalidInitializationParams,
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
        assert!(analytics
            .errors_by_category
            .get(ErrorCategory::UserOperation)
            .is_some());
        assert!(analytics
            .errors_by_severity
            .get(ErrorSeverity::Low)
            .is_some());
    }

    #[test]
    fn test_technical_details_not_placeholder() {
        let env = Env::default();
        let ctx = make_context(&env);
        let details = ErrorHandler::get_technical_details(&env, &Error::OracleUnavailable, &ctx);
        // Must contain the numeric error code, not just a generic string
        // (soroban String has no contains(), so we verify it is non-empty)
        assert!(!details.is_empty());
    }

    // ── Regression: GasBudgetExceeded missing from description() match ──────
    #[test]
    fn test_gas_budget_exceeded_description_is_exhaustive() {
        let err = Error::GasBudgetExceeded;
        let desc = err.description();
        assert!(
            !desc.is_empty(),
            "GasBudgetExceeded must have a non-empty description"
        );
        assert_ne!(
            desc, "An error occurred. Please verify your parameters and try again.",
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
        let validation_msg = ErrorHandler::get_user_action(
            &env,
            &Error::InvalidQuestion,
            &ErrorCategory::Validation,
        );
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
        assert!(
            ErrorHandler::execute_recovery_strategy(&env, &retry)
                .unwrap()
                .success
        );

        let retry_with_delay_fail = ErrorRecovery {
            recovery_strategy: String::from_str(&env, "retry_with_delay"),
            ..retry.clone()
        };
        assert!(
            !ErrorHandler::execute_recovery_strategy(&env, &retry_with_delay_fail)
                .unwrap()
                .success
        );

        let alt_success = ErrorRecovery {
            original_error_code: Error::OracleUnavailable as u32,
            recovery_strategy: String::from_str(&env, "alternative_method"),
            ..retry.clone()
        };
        assert!(
            ErrorHandler::execute_recovery_strategy(&env, &alt_success)
                .unwrap()
                .success
        );

        let skip = ErrorRecovery {
            recovery_strategy: String::from_str(&env, "skip"),
            ..retry.clone()
        };
        assert!(
            ErrorHandler::execute_recovery_strategy(&env, &skip)
                .unwrap()
                .success
        );

        let abort = ErrorRecovery {
            recovery_strategy: String::from_str(&env, "abort"),
            ..retry
        };
        assert!(
            !ErrorHandler::execute_recovery_strategy(&env, &abort)
                .unwrap()
                .success
        );
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
        assert!(ErrorHandler::handle_error_recovery(&env, &Error::AdminNotSet, &ctx).is_err());
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
                m.set(
                    String::from_str(&env, "attempts"),
                    String::from_str(&env, "3"),
                );
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
            m.set(
                String::from_str(&env, "attempts"),
                String::from_str(&env, "3"),
            );
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
        assert!(procedures
            .get(String::from_str(&env, "retry_procedure"))
            .is_some());
        assert!(procedures
            .get(String::from_str(&env, "oracle_recovery"))
            .is_some());
        assert!(procedures
            .get(String::from_str(&env, "validation_recovery"))
            .is_some());
        assert!(procedures
            .get(String::from_str(&env, "system_recovery"))
            .is_some());
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
