use soroban_sdk::{contracterror, contracttype, Address, Env, Map, String, Symbol, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Not enough multisig approvals yet
    ApprovalPending = 100,
    Unauthorized = 1,
    MarketNotFound = 2,
    MarketClosed = 3,
    MarketResolved = 4,
    MarketNotResolved = 5,
    NothingToClaim = 6,
    AlreadyClaimed = 7,
    InsufficientStake = 8,
    InvalidOutcome = 9,
    AlreadyVoted = 10,
    AlreadyBet = 11,
    BetsAlreadyPlaced = 12,
    InsufficientBalance = 13,
    OracleUnavailable = 14,
    InvalidOracleConfig = 15,
    OracleStale = 16,
    OracleNoConsensus = 17,
    OracleVerified = 18,
    MarketNotReady = 19,
    InvalidQuestion = 20,
    InvalidOutcomes = 21,
    InvalidDuration = 22,
    InvalidThreshold = 23,
    InvalidState = 24,
    InvalidInput = 25,
    ConfigNotFound = 26,
    AlreadyDisputed = 27,
    DisputeVoteExpired = 28,
    DisputeCondNotMet = 29,
    NotAuthorized = 30,
    /*
    AdminNotSet = 30,
    CBOpen = 31,
    ResolutionTimeoutReached = 32,
    CBAlreadyOpen = 33,
    CBNotOpen = 34,
    CBNotInitialized = 35,
    DisputeVoteDenied = 36,
    DisputeAlreadyVoted = 37,
    DisputeFeeFailed = 38,
    DisputeNoEscalate = 39,
    FeeAlreadyCollected = 40,
    NoFeesToCollect = 41,
    InvalidExtensionDays = 42,
    InvalidFeeConfig = 43,
    TimeoutNotSet = 44,
    TimeoutNotExpired = 45,
    InvalidTimeoutHours = 46,
    InvalidComparison = 47,
    ExtensionError = 48,
    InvalidResolutionWindow = 49,
    FallbackOracleUnavailable = 50,
    */
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorContext {
    pub operation: String,
    pub user_address: Option<Address>,
    pub market_id: Option<Symbol>,
    pub context_data: Map<String, String>,
    pub timestamp: u64,
    pub call_chain: Vec<String>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorRecovery {
    pub original_error_code: u32,
    pub recovery_method: String,
    pub recovery_timestamp: u64,
    pub recovery_data: Map<String, String>,
    pub success: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorRecoveryStatus {
    pub total_attempts: u32,
    pub successful_recoveries: u32,
    pub failed_recoveries: u32,
    pub active_recoveries: u32,
    pub success_rate: i128,
    pub avg_recovery_time: u64,
    pub last_recovery_timestamp: Option<u64>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
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
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ResiliencePatternType {
    RetryWithBackoff,
    CircuitBreaker,
    Bulkhead,
    Timeout,
    Fallback,
    HealthCheck,
    RateLimit,
}

pub struct ErrorHandler;

impl ErrorHandler {
    pub fn recover_from_error(_env: &Env, _error: Error, _context: ErrorContext) -> Result<ErrorRecovery, Error> {
        Err(Error::InvalidState)
    }
    pub fn validate_error_recovery(_env: &Env, _recovery: &ErrorRecovery) -> Result<bool, Error> {
        Ok(true)
    }
    pub fn get_error_recovery_status(_env: &Env) -> Result<ErrorRecoveryStatus, Error> {
        Err(Error::InvalidState)
    }
    pub fn emit_error_recovery_event(_env: &Env, _recovery: &ErrorRecovery) {}
    pub fn validate_resilience_patterns(_env: &Env, _patterns: &Vec<ResiliencePattern>) -> Result<bool, Error> {
        Ok(true)
    }
    pub fn document_error_recovery_procedures(_env: &Env) -> Result<Map<String, String>, Error> {
        Err(Error::InvalidState)
    }
}

impl Error {
    pub fn description(&self) -> &'static str {
        "User-friendly error description"
    }
    pub fn code(&self) -> &'static str {
        "ERROR_CODE"
    }
}
