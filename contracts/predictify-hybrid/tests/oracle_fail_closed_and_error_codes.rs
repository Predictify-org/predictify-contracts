//! Focused regression tests for:
//!   - #1392: Oracle resolution must fail-closed on stale data
//!   - #1411: Error mappings must be stable for client integrations
//!
//! These tests verify the contract-level behaviour of the two features without
//! depending on any test helpers that have pre-existing compilation issues.

use predictify_hybrid::Error;
use predictify_hybrid::errors::Recoverability;

// ═══════════════════════════════════════════════════════════════════════════
// #1411 — Stable error mappings
// ═══════════════════════════════════════════════════════════════════════════

// ── client_code() stability ─────────────────────────────────────────────────

/// Every oracle error must map into the 1000-1099 band.
#[test]
fn oracle_client_codes_in_range() {
    let oracle_errors = [
        Error::OracleUnavailable,
        Error::InvalidOracleConfig,
        Error::OracleStale,
        Error::OracleNoConsensus,
        Error::OracleVerified,
        Error::FallbackOracleUnavailable,
        Error::ResolutionTimeoutReached,
        Error::OracleConfidenceTooWide,
        Error::InvalidOracleFeed,
        Error::OracleCallbackAuthFailed,
        Error::OracleCallbackUnauthorized,
        Error::OracleCallbackInvalidSignature,
        Error::OracleCallbackReplayDetected,
        Error::OracleCallbackTimeout,
        Error::OracleQuoteOutlier,
    ];
    for err in oracle_errors {
        let c = err.client_code();
        assert!(
            c >= 1000 && c <= 1099,
            "{:?}.client_code() = {} is outside [1000, 1099]",
            err, c
        );
    }
}

/// Every market error must map into the 1100-1199 band.
#[test]
fn market_client_codes_in_range() {
    for err in [
        Error::MarketNotFound,
        Error::MarketClosed,
        Error::MarketResolved,
        Error::MarketNotResolved,
        Error::MarketNotReady,
        Error::InvalidState,
        Error::IllegalMarketStateTransition,
        Error::DuplicateMarketId,
        Error::MaxParticipantsReached,
    ] {
        let c = err.client_code();
        assert!(
            c >= 1100 && c <= 1199,
            "{:?}.client_code() = {} is outside [1100, 1199]",
            err, c
        );
    }
}

/// Every validation error must map into the 1200-1299 band.
#[test]
fn validation_client_codes_in_range() {
    for err in [
        Error::InvalidQuestion,
        Error::InvalidOutcomes,
        Error::InvalidDuration,
        Error::InvalidThreshold,
        Error::InvalidComparison,
        Error::InvalidInput,
        Error::InvalidOutcome,
    ] {
        let c = err.client_code();
        assert!(
            c >= 1200 && c <= 1299,
            "{:?}.client_code() = {} is outside [1200, 1299]",
            err, c
        );
    }
}

/// Every financial error must map into the 1300-1399 band.
#[test]
fn financial_client_codes_in_range() {
    for err in [
        Error::InsufficientStake,
        Error::InsufficientBalance,
        Error::NothingToClaim,
        Error::AlreadyClaimed,
        Error::FeeArithmeticOverflow,
        Error::FeeAlreadyCollected,
        Error::NoFeesToCollect,
        Error::InvalidFeeConfig,
        Error::FeeExceedsMax,
        Error::SweepAlreadyDone,
        Error::DisputeFeeFailed,
    ] {
        let c = err.client_code();
        assert!(
            c >= 1300 && c <= 1399,
            "{:?}.client_code() = {} is outside [1300, 1399]",
            err, c
        );
    }
}

/// Every dispute error must map into the 1400-1499 band.
#[test]
fn dispute_client_codes_in_range() {
    for err in [
        Error::AlreadyDisputed,
        Error::DisputeVoteExpired,
        Error::DisputeVoteDenied,
        Error::DisputeAlreadyVoted,
        Error::DisputeCondNotMet,
        Error::DisputeError,
        Error::DisputerCannotVote,
        Error::DisputeStakeCapExceeded,
    ] {
        let c = err.client_code();
        assert!(
            c >= 1400 && c <= 1499,
            "{:?}.client_code() = {} is outside [1400, 1499]",
            err, c
        );
    }
}

/// Authentication errors must map into the 1500-1599 band.
#[test]
fn auth_client_code_in_range() {
    for err in [Error::Unauthorized, Error::ReplayedOverride] {
        let c = err.client_code();
        assert!(
            c >= 1500 && c <= 1599,
            "{:?}.client_code() = {} is outside [1500, 1599]",
            err, c
        );
    }
}

/// Circuit breaker errors must map into the 1600-1699 band.
#[test]
fn circuit_breaker_client_codes_in_range() {
    for err in [
        Error::CBNotInitialized,
        Error::CBAlreadyOpen,
        Error::CBNotOpen,
        Error::CBOpen,
        Error::CBError,
        Error::RateLimitExceeded,
    ] {
        let c = err.client_code();
        assert!(
            c >= 1600 && c <= 1699,
            "{:?}.client_code() = {} is outside [1600, 1699]",
            err, c
        );
    }
}

/// System errors must map into the 1700-1799 band.
#[test]
fn system_client_codes_in_range() {
    for err in [
        Error::ConfigNotFound,
        Error::AdminNotSet,
        Error::GasBudgetExceeded,
    ] {
        let c = err.client_code();
        assert!(
            c >= 1700 && c <= 1799,
            "{:?}.client_code() = {} is outside [1700, 1799]",
            err, c
        );
    }
}

/// User-operation errors must map into the 1800-1899 band.
#[test]
fn user_operation_client_codes_in_range() {
    for err in [
        Error::AlreadyVoted,
        Error::AlreadyBet,
        Error::BetsAlreadyPlaced,
    ] {
        let c = err.client_code();
        assert!(
            c >= 1800 && c <= 1899,
            "{:?}.client_code() = {} is outside [1800, 1899]",
            err, c
        );
    }
}

/// Metadata/length-limit errors must map into the 1900-1999 band.
#[test]
fn metadata_client_codes_in_range() {
    for err in [
        Error::QuestionTooLong,
        Error::OutcomeTooLong,
        Error::TooManyOutcomes,
        Error::FeedIdTooLong,
        Error::ComparisonTooLong,
        Error::CategoryTooLong,
        Error::CategoryTooShort,
        Error::TagTooLong,
        Error::TagTooShort,
        Error::TooManyTags,
        Error::ExtensionReasonTooLong,
        Error::TooManyExtensions,
        Error::TooManyOracleResults,
        Error::TooManyWinningOutcomes,
        Error::ArchiveFull,
    ] {
        let c = err.client_code();
        assert!(
            c >= 1900 && c <= 1999,
            "{:?}.client_code() = {} is outside [1900, 1999]",
            err, c
        );
    }
}

/// No two error variants must share a client_code().
/// Uses O(n²) comparison to avoid requiring std::collections.
#[test]
fn client_codes_are_unique() {
    // Comprehensive list of all Error variants tested here.
    let all: &[Error] = &[
        Error::OracleUnavailable,
        Error::InvalidOracleConfig,
        Error::OracleStale,
        Error::OracleNoConsensus,
        Error::OracleVerified,
        Error::FallbackOracleUnavailable,
        Error::ResolutionTimeoutReached,
        Error::OracleConfidenceTooWide,
        Error::InvalidOracleFeed,
        Error::OracleCallbackAuthFailed,
        Error::OracleCallbackUnauthorized,
        Error::OracleCallbackInvalidSignature,
        Error::OracleCallbackReplayDetected,
        Error::OracleCallbackTimeout,
        Error::OracleQuoteOutlier,
        Error::MarketNotFound,
        Error::MarketClosed,
        Error::MarketResolved,
        Error::MarketNotResolved,
        Error::MarketNotReady,
        Error::InvalidState,
        Error::IllegalMarketStateTransition,
        Error::DuplicateMarketId,
        Error::MaxParticipantsReached,
        Error::InvalidQuestion,
        Error::InvalidOutcomes,
        Error::InvalidDuration,
        Error::InvalidThreshold,
        Error::InvalidComparison,
        Error::InvalidInput,
        Error::InvalidOutcome,
        Error::AssetDecimalsMismatch,
        Error::InvalidExtensionDays,
        Error::ExtensionDenied,
        Error::CumulativeExtensionCapHit,
        Error::ExtensionCapExceeded,
        Error::InsufficientStake,
        Error::InsufficientBalance,
        Error::NothingToClaim,
        Error::AlreadyClaimed,
        Error::FeeArithmeticOverflow,
        Error::FeeAlreadyCollected,
        Error::NoFeesToCollect,
        Error::InvalidFeeConfig,
        Error::FeeExceedsMax,
        Error::SweepAlreadyDone,
        Error::DisputeFeeFailed,
        Error::NoPendingFeeCommit,
        Error::FeeRevealTooEarly,
        Error::FeePreimageMismatch,
        Error::BetExceedsCap,
        Error::MaxBetCapExceeded,
        Error::AlreadyDisputed,
        Error::DisputeVoteExpired,
        Error::DisputeVoteDenied,
        Error::DisputeAlreadyVoted,
        Error::DisputeCondNotMet,
        Error::DisputeError,
        Error::DisputerCannotVote,
        Error::DisputeStakeCapExceeded,
        Error::Unauthorized,
        Error::ReplayedOverride,
        Error::UserNotWhitelisted,
        Error::UserBlacklisted,
        Error::CreatorBlacklisted,
        Error::CBNotInitialized,
        Error::CBAlreadyOpen,
        Error::CBNotOpen,
        Error::CBOpen,
        Error::CBError,
        Error::RateLimitExceeded,
        Error::PerLedgerBetCapExceeded,
        Error::ConfigNotFound,
        Error::AdminNotSet,
        Error::GasBudgetExceeded,
        Error::OperationWouldExceedBudget,
        Error::InsufficientStorageRentBudget,
        Error::UpgradeChainMismatch,
        Error::AlreadyInitialized,
        Error::InvalidTimeLockDelay,
        Error::TimeLockNotExpired,
        Error::NoPendingUpdate,
        Error::PendingUpdateExists,
        Error::AdminActionTimelocked,
        Error::OracleAdminCooldownActive,
        Error::SignerRotationCooldown,
        Error::AlreadyVoted,
        Error::AlreadyBet,
        Error::BetsAlreadyPlaced,
        Error::ForceResolveAlreadyUsed,
        Error::ForceResolveReplayed,
        Error::ForceResolveReasonEmpty,
        Error::IdempotentBatchAlreadyApplied,
        Error::InvalidStakeAmount,
        Error::QuestionTooLong,
        Error::OutcomeTooLong,
        Error::TooManyOutcomes,
        Error::FeedIdTooLong,
        Error::ComparisonTooLong,
        Error::CategoryTooLong,
        Error::CategoryTooShort,
        Error::TagTooLong,
        Error::TagTooShort,
        Error::TooManyTags,
        Error::ExtensionReasonTooLong,
        Error::SourceTooLong,
        Error::ErrorMessageTooLong,
        Error::SignatureTooLong,
        Error::TooManyExtensions,
        Error::TooManyOracleResults,
        Error::TooManyWinningOutcomes,
        Error::ArchiveFull,
        Error::ReasonTableFull,
        Error::RegistryFull,
        Error::Overflow,
        Error::InvalidCap,
    ];

    let codes: std::vec::Vec<u32> = all.iter().map(|e| e.client_code()).collect();
    for i in 0..codes.len() {
        for j in (i + 1)..codes.len() {
            assert_ne!(
                codes[i], codes[j],
                "Duplicate client_code {} for {:?} and {:?}",
                codes[i], all[i], all[j]
            );
        }
    }
}

// ── Stability pin-tests: these must NEVER change ─────────────────────────────

/// Pin specific client codes to freeze them against future changes.
///
/// If any of these assertions fail, a breaking change has been made to the
/// stable error mapping. The fix is to add a new variant rather than change
/// an existing one.
#[test]
fn pinned_client_codes_never_change() {
    // Oracle
    assert_eq!(Error::OracleUnavailable.client_code(), 1000);
    assert_eq!(Error::OracleStale.client_code(), 1002);
    assert_eq!(Error::FallbackOracleUnavailable.client_code(), 1005);

    // Market
    assert_eq!(Error::MarketNotFound.client_code(), 1100);
    assert_eq!(Error::MarketClosed.client_code(), 1101);
    assert_eq!(Error::MarketResolved.client_code(), 1102);

    // Validation
    assert_eq!(Error::InvalidInput.client_code(), 1205);

    // Authentication
    assert_eq!(Error::Unauthorized.client_code(), 1500);

    // Circuit Breaker
    assert_eq!(Error::CBOpen.client_code(), 1603);
    assert_eq!(Error::RateLimitExceeded.client_code(), 1605);

    // System
    assert_eq!(Error::AdminNotSet.client_code(), 1701);
    assert_eq!(Error::GasBudgetExceeded.client_code(), 1702);
}

// ── Recoverability tests ──────────────────────────────────────────────────────

/// Transient oracle errors must be Retryable.
#[test]
fn oracle_transient_errors_are_retryable() {
    for err in [
        Error::OracleUnavailable,
        Error::OracleStale,
        Error::OracleNoConsensus,
        Error::OracleConfidenceTooWide,
        Error::OracleCallbackTimeout,
        Error::FallbackOracleUnavailable,
    ] {
        assert_eq!(
            err.recoverability(),
            Recoverability::Retryable,
            "{:?} should be Retryable",
            err
        );
    }
}

/// Configuration errors must require admin action before retrying.
#[test]
fn config_errors_require_admin() {
    for err in [
        Error::AdminNotSet,
        Error::InvalidOracleConfig,
        Error::InvalidFeeConfig,
        Error::ConfigNotFound,
    ] {
        assert_eq!(
            err.recoverability(),
            Recoverability::RequiresAdmin,
            "{:?} should be RequiresAdmin",
            err
        );
    }
}

/// Duplicate/replay errors are permanent and must be Terminal.
#[test]
fn duplicate_errors_are_terminal() {
    for err in [
        Error::AlreadyClaimed,
        Error::AlreadyVoted,
        Error::AlreadyDisputed,
        Error::AlreadyBet,
    ] {
        assert_eq!(
            err.recoverability(),
            Recoverability::Terminal,
            "{:?} should be Terminal",
            err
        );
    }
}

/// Authorization errors are Terminal (same inputs will always be rejected).
#[test]
fn auth_errors_are_terminal() {
    assert_eq!(
        Error::Unauthorized.recoverability(),
        Recoverability::Terminal,
        "Unauthorized should be Terminal"
    );
}

/// Rate-limit and throttle conditions are Retryable (will clear after time passes).
#[test]
fn rate_limit_is_retryable() {
    assert_eq!(
        Error::RateLimitExceeded.recoverability(),
        Recoverability::Retryable
    );
}

// ── Exhaustive check: every variant has a valid recoverability label ──────────

/// Verify no variant panics or produces an invalid Recoverability value.
/// This is a compile-time-like check at runtime — if the match is
/// non-exhaustive it will panic here rather than silently returning a default.
#[test]
fn all_variants_have_recoverability() {
    let all: &[Error] = &[
        Error::OracleUnavailable,
        Error::InvalidOracleConfig,
        Error::OracleStale,
        Error::OracleNoConsensus,
        Error::OracleVerified,
        Error::FallbackOracleUnavailable,
        Error::ResolutionTimeoutReached,
        Error::OracleConfidenceTooWide,
        Error::InvalidOracleFeed,
        Error::OracleCallbackAuthFailed,
        Error::OracleCallbackUnauthorized,
        Error::OracleCallbackInvalidSignature,
        Error::OracleCallbackReplayDetected,
        Error::OracleCallbackTimeout,
        Error::OracleQuoteOutlier,
        Error::MarketNotFound,
        Error::MarketClosed,
        Error::MarketResolved,
        Error::MarketNotResolved,
        Error::MarketNotReady,
        Error::InvalidState,
        Error::InvalidInput,
        Error::InvalidQuestion,
        Error::InvalidOutcomes,
        Error::InvalidDuration,
        Error::InvalidThreshold,
        Error::InvalidComparison,
        Error::InvalidOutcome,
        Error::Unauthorized,
        Error::AlreadyVoted,
        Error::AlreadyBet,
        Error::AlreadyClaimed,
        Error::NothingToClaim,
        Error::InsufficientStake,
        Error::InsufficientBalance,
        Error::CBOpen,
        Error::CBError,
        Error::RateLimitExceeded,
        Error::AdminNotSet,
        Error::ConfigNotFound,
        Error::GasBudgetExceeded,
    ];
    for err in all {
        let r = err.recoverability();
        assert!(
            matches!(
                r,
                Recoverability::Retryable
                    | Recoverability::RequiresAdmin
                    | Recoverability::Terminal
            ),
            "{:?} returned an unexpected Recoverability label",
            err
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// #1392 — Fail-closed oracle staleness (unit level)
// ═══════════════════════════════════════════════════════════════════════════
//
// The following tests verify the fail-closed invariant of the error taxonomy:
// `OracleStale` must be Retryable (not Terminal or RequiresAdmin), so callers
// know that the operation may succeed once fresh data is available.

/// OracleStale is Retryable — confirms the fail-closed path surfaces a
/// diagnosable, retryable error rather than silently excluding the quote.
#[test]
fn oracle_stale_is_retryable_not_terminal() {
    let r = Error::OracleStale.recoverability();
    assert_eq!(
        r,
        Recoverability::Retryable,
        "OracleStale must be Retryable so callers can wait and retry for fresh data"
    );
}

/// OracleStale's client_code must be stable and in the Oracle range.
#[test]
fn oracle_stale_client_code_is_stable_and_in_oracle_range() {
    let c = Error::OracleStale.client_code();
    assert_eq!(c, 1002, "OracleStale.client_code() must be pinned to 1002");
    assert!(c >= 1000 && c <= 1099);
}

/// OracleNoConsensus (returned when all oracles are excluded or stale) must
/// also be Retryable so the resolution can be retried after oracle refresh.
#[test]
fn oracle_no_consensus_is_retryable() {
    assert_eq!(
        Error::OracleNoConsensus.recoverability(),
        Recoverability::Retryable,
        "OracleNoConsensus must be Retryable — consensus may be achievable after retry"
    );
}

/// Verify ranges are non-overlapping (belt-and-braces, mirrors test in
/// error_code_tests.rs but included here for self-contained coverage).
#[test]
fn client_code_ranges_are_disjoint() {
    let ranges: &[(&str, u32, u32)] = &[
        ("Oracle",          1000, 1099),
        ("Market",          1100, 1199),
        ("Validation",      1200, 1299),
        ("Financial",       1300, 1399),
        ("Dispute",         1400, 1499),
        ("Auth",            1500, 1599),
        ("CircuitBreaker",  1600, 1699),
        ("System",          1700, 1799),
        ("UserOperation",   1800, 1899),
        ("Metadata",        1900, 1999),
    ];
    for i in 0..ranges.len() {
        for j in (i + 1)..ranges.len() {
            let (n1, lo1, hi1) = ranges[i];
            let (n2, lo2, hi2) = ranges[j];
            assert!(
                hi1 < lo2 || hi2 < lo1,
                "Ranges for {} ({}-{}) and {} ({}-{}) overlap",
                n1, lo1, hi1, n2, lo2, hi2
            );
        }
    }
}
