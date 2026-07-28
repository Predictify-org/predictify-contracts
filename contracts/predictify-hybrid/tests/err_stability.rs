//! Client-facing contract error-code stability snapshot.
//!
//! `Error` discriminants are serialized by Soroban and consumed by clients.
//! Renaming, removing, or renumbering a variant is therefore an API change.
//! Additions must use a new explicit code and be reviewed into this snapshot.

use std::collections::BTreeSet;

use predictify_hybrid::Error;

macro_rules! error_code_snapshot {
    ($($variant:ident = $code:literal),+ $(,)?) => {
        const ERROR_CODE_SNAPSHOT: &[(Error, u32)] = &[
            $((Error::$variant, $code),)+
        ];

        // Keeping this match exhaustive makes a newly added enum variant fail
        // compilation until its public code is deliberately added here.
        fn frozen_code(error: Error) -> u32 {
            match error {
                $(Error::$variant => $code,)+
            }
        }
    };
}

error_code_snapshot! {
    Unauthorized = 100,
    MarketNotFound = 101,
    MarketClosed = 102,
    MarketResolved = 103,
    MarketNotResolved = 104,
    NothingToClaim = 105,
    AlreadyClaimed = 106,
    InsufficientStake = 107,
    InvalidOutcome = 108,
    AlreadyVoted = 109,
    AlreadyBet = 110,
    BetsAlreadyPlaced = 111,
    InsufficientBalance = 112,
    BetCoolOffActive = 113,

    OracleUnavailable = 200,
    InvalidOracleConfig = 201,
    OracleStale = 202,
    OracleNoConsensus = 203,
    OracleVerified = 204,
    MarketNotReady = 205,
    FallbackOracleUnavailable = 206,
    ResolutionTimeoutReached = 207,
    OracleConfidenceTooWide = 208,
    InvalidOracleFeed = 209,
    OracleCallbackAuthFailed = 210,
    OracleCallbackUnauthorized = 211,
    OracleCallbackInvalidSignature = 212,
    OracleCallbackReplayDetected = 213,
    OracleCallbackTimeout = 214,

    InvalidQuestion = 300,
    InvalidOutcomes = 301,
    InvalidDuration = 302,
    InvalidThreshold = 303,
    InvalidComparison = 304,

    InvalidInput = 401,
    InvalidFeeConfig = 402,
    ConfigNotFound = 403,
    AlreadyDisputed = 404,
    DisputeVoteExpired = 405,
    DisputeVoteDenied = 406,
    DisputeAlreadyVoted = 407,
    DisputeCondNotMet = 408,
    DisputeFeeFailed = 409,
    DisputeError = 410,
    SweepAlreadyDone = 411,
    FeeArithmeticOverflow = 412,
    FeeAlreadyCollected = 413,
    NoFeesToCollect = 414,
    InvalidExtensionDays = 415,
    ExtensionDenied = 416,
    GasBudgetExceeded = 417,
    OperationWouldExceedBudget = 418,
    AdminNotSet = 419,
    QuestionTooLong = 420,
    OutcomeTooLong = 421,
    TooManyOutcomes = 422,
    FeedIdTooLong = 423,
    ComparisonTooLong = 424,
    CategoryTooLong = 425,
    TagTooLong = 426,
    TooManyTags = 427,
    ExtensionReasonTooLong = 428,
    SourceTooLong = 429,
    ErrorMessageTooLong = 430,
    SignatureTooLong = 431,
    TooManyExtensions = 432,
    TooManyOracleResults = 433,
    TooManyWinningOutcomes = 434,
    ForceResolveAlreadyUsed = 435,
    CategoryTooShort = 436,
    TagTooShort = 437,
    DisputerCannotVote = 438,
    AssetDecimalsMismatch = 439,
    ArchiveFull = 440,
    DuplicateMarketId = 441,
    AdminActionTimelocked = 443,
    CooloffActive = 444,
    InvalidState = 494,
    NoDisputesFound = 496,
    OracleResultNotAvailable = 497,

    CBNotInitialized = 500,
    CBAlreadyOpen = 501,
    CBNotOpen = 502,
    CBOpen = 503,
    CBError = 504,
    RateLimitExceeded = 505,
    CumulativeExtensionCapHit = 506,
    IllegalMarketStateTransition = 507,
    FeeExceedsMax = 508,
    ForceResolveReplayed = 517,
    ForceResolveReasonEmpty = 518,
    NoPendingFeeCommit = 519,
    FeeRevealTooEarly = 520,
    FeePreimageMismatch = 521,
    DisputeStakeCapExceeded = 522,
    InsufficientStorageRentBudget = 523,
    ExtensionCapExceeded = 524,
    UpgradeChainMismatch = 525,
    ReplayedOverride = 526,
    OracleQuoteOutlier = 527,
    MaxParticipantsReached = 528,
    BetExceedsCap = 529,
    InvalidStakeAmount = 530,
    OracleAdminCooldownActive = 531,
    SignerRotationCooldown = 532,
    AlreadyInitialized = 533,
    InvalidTimeLockDelay = 534,
    PendingUpdateExists = 535,
    NoPendingUpdate = 536,
    TimeLockNotExpired = 537,
    RegistryFull = 538,
    PerLedgerBetCapExceeded = 539,
    UserBlacklisted = 540,
    UserNotWhitelisted = 541,
    CreatorBlacklisted = 542,

    ReasonTableFull = 670,
    Overflow = 672,
    MaxBetCapExceeded = 673,
    InvalidCap = 674,
    CoolOffPeriodActive = 675,
}

#[test]
fn contract_error_codes_are_stable() {
    assert_eq!(ERROR_CODE_SNAPSHOT.len(), 120);

    for &(error, expected) in ERROR_CODE_SNAPSHOT {
        assert_eq!(
            error as u32, expected,
            "client-facing error code changed for {error:?}; this requires an API migration"
        );
        assert_eq!(frozen_code(error), expected);
    }
}

#[test]
fn contract_error_codes_are_unique() {
    let mut seen = BTreeSet::new();

    for &(error, code) in ERROR_CODE_SNAPSHOT {
        assert!(
            seen.insert(code),
            "client-facing error code {code} is reused by {error:?}"
        );
    }
}
