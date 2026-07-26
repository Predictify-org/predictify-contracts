//! Hybrid contract resolution-related client-facing error-code stability tests.
//!
//! These assertions intentionally freeze the numeric values exposed by the
//! contract's resolution errors. Client applications may persist or branch on
//! these values, so changing one is a visible API change and requires an
//! explicit migration or versioning decision.
//!
//! This test focuses on error codes used during market resolution, including:
//! - Oracle integration errors (200-series)
//! - Market state errors relevant to resolution
//! - Validation errors that block resolution
//! - Financial/settlement errors

use predictify_hybrid::Error;

/// Resolution-related error codes are reserved for specific ranges:
/// - Oracle errors: 200-214 (primary resolution integration point)
/// - Market state errors: 101-104 (resolution lifecycle)
/// - Configuration errors: 201-214 (oracle resolution specifics)
///
/// Keep this table explicit rather than deriving values from enum ordering:
/// the numeric assignments are part of the client-facing contract.
#[test]
fn resolution_oracle_error_codes_are_stable() {
    let resolution_errors = [
        // Oracle availability and configuration
        (Error::OracleUnavailable, 200u32),
        (Error::InvalidOracleConfig, 201u32),
        (Error::OracleStale, 202u32),
        (Error::OracleNoConsensus, 203u32),
        (Error::OracleVerified, 204u32),
        
        // Market readiness for resolution
        (Error::MarketNotReady, 205u32),
        
        // Fallback oracle configuration
        (Error::FallbackOracleUnavailable, 206u32),
        
        // Resolution timeout
        (Error::ResolutionTimeoutReached, 207u32),
        
        // Oracle data quality
        (Error::OracleConfidenceTooWide, 208u32),
        (Error::InvalidOracleFeed, 209u32),
        
        // Oracle callback (callback resolution path)
        (Error::OracleCallbackAuthFailed, 210u32),
        (Error::OracleCallbackUnauthorized, 211u32),
        (Error::OracleCallbackInvalidSignature, 212u32),
        (Error::OracleCallbackReplayDetected, 213u32),
        (Error::OracleCallbackTimeout, 214u32),
    ];

    for (error, expected_code) in resolution_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "resolution oracle error code changed for {error:?}; this is a client-facing API change"
        );
    }
}

/// Market state errors relevant to resolution are stable.
/// These errors determine whether a market can be resolved.
#[test]
fn resolution_market_state_error_codes_are_stable() {
    let market_state_errors = [
        (Error::MarketNotFound, 101u32),
        (Error::MarketClosed, 102u32),
        (Error::MarketResolved, 103u32),
        (Error::MarketNotResolved, 104u32),
    ];

    for (error, expected_code) in market_state_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "market state error code changed for {error:?}; this is a client-facing API change"
        );
    }
}

/// Validation errors that affect resolution workflow are stable.
#[test]
fn resolution_validation_error_codes_are_stable() {
    let validation_errors = [
        (Error::InvalidQuestion, 300u32),
        (Error::InvalidOutcomes, 301u32),
        (Error::InvalidDuration, 302u32),
        (Error::InvalidThreshold, 303u32),
        (Error::InvalidComparison, 304u32),
    ];

    for (error, expected_code) in validation_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "validation error code changed for {error:?}; this is a client-facing API change"
        );
    }
}

/// General system errors that can occur during resolution are stable.
#[test]
fn resolution_system_error_codes_are_stable() {
    let system_errors = [
        (Error::InvalidState, 400u32),
        (Error::InvalidInput, 401u32),
        (Error::InvalidFeeConfig, 402u32),
        (Error::ConfigNotFound, 403u32),
        (Error::Unauthorized, 100u32),
        (Error::AdminNotSet, 419u32),
    ];

    for (error, expected_code) in system_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "system error code changed for {error:?}; this is a client-facing API change"
        );
    }
}

/// Dispute-related errors during resolution are stable.
/// Disputes can extend or override resolution.
#[test]
fn resolution_dispute_error_codes_are_stable() {
    let dispute_errors = [
        (Error::AlreadyDisputed, 404u32),
        (Error::DisputeVoteExpired, 405u32),
        (Error::DisputeVoteDenied, 406u32),
        (Error::DisputeAlreadyVoted, 407u32),
        (Error::DisputeCondNotMet, 408u32),
        (Error::DisputeFeeFailed, 409u32),
        (Error::DisputeError, 410u32),
        (Error::DisputerCannotVote, 438u32),
        (Error::DisputeStakeCapExceeded, 522u32),
    ];

    for (error, expected_code) in dispute_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "dispute error code changed for {error:?}; this is a client-facing API change"
        );
    }
}

/// Fee and settlement errors during resolution are stable.
/// Resolution includes fee collection and payout distribution.
#[test]
fn resolution_financial_error_codes_are_stable() {
    let financial_errors = [
        (Error::InsufficientBalance, 112u32),
        (Error::InsufficientStake, 107u32),
        (Error::FeeAlreadyCollected, 413u32),
        (Error::NoFeesToCollect, 414u32),
        (Error::FeeArithmeticOverflow, 412u32),
        (Error::FeeExceedsMax, 508u32),
        (Error::NothingToClaim, 105u32),
        (Error::AlreadyClaimed, 106u32),
    ];

    for (error, expected_code) in financial_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "financial error code changed for {error:?}; this is a client-facing API change"
        );
    }
}

/// Force-resolve related errors are stable.
/// Force-resolve is an admin override path to resolution.
#[test]
fn resolution_force_resolve_error_codes_are_stable() {
    let force_resolve_errors = [
        (Error::ForceResolveAlreadyUsed, 435u32),
        (Error::ForceResolveReplayed, 517u32),
        (Error::ForceResolveReasonEmpty, 518u32),
    ];

    for (error, expected_code) in force_resolve_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "force-resolve error code changed for {error:?}; this is a client-facing API change"
        );
    }
}

/// Circuit breaker errors that can interrupt resolution are stable.
#[test]
fn resolution_circuit_breaker_error_codes_are_stable() {
    let circuit_breaker_errors = [
        (Error::CBNotInitialized, 500u32),
        (Error::CBAlreadyOpen, 501u32),
        (Error::CBNotOpen, 502u32),
        (Error::CBOpen, 503u32),
        (Error::CBError, 504u32),
        (Error::RateLimitExceeded, 505u32),
    ];

    for (error, expected_code) in circuit_breaker_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "circuit breaker error code changed for {error:?}; this is a client-facing API change"
        );
    }
}

/// State machine and transition errors are stable.
#[test]
fn resolution_state_transition_error_codes_are_stable() {
    let state_transition_errors = [
        (Error::IllegalMarketStateTransition, 507u32),
    ];

    for (error, expected_code) in state_transition_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "state transition error code changed for {error:?}; this is a client-facing API change"
        );
    }
}

/// Metadata limit errors that affect resolution are stable.
#[test]
fn resolution_metadata_limit_error_codes_are_stable() {
    let metadata_errors = [
        (Error::QuestionTooLong, 420u32),
        (Error::OutcomeTooLong, 421u32),
        (Error::TooManyOutcomes, 422u32),
        (Error::FeedIdTooLong, 423u32),
        (Error::ComparisonTooLong, 424u32),
        (Error::CategoryTooLong, 425u32),
        (Error::TagTooLong, 426u32),
        (Error::TooManyTags, 427u32),
        (Error::TooManyOracleResults, 433u32),
        (Error::TooManyWinningOutcomes, 434u32),
    ];

    for (error, expected_code) in metadata_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "metadata limit error code changed for {error:?}; this is a client-facing API change"
        );
    }
}

/// Oracle data quality errors are unique and contiguous within 200-214 range.
#[test]
fn resolution_oracle_error_codes_are_unique_and_contiguous() {
    let oracle_codes = [
        Error::OracleUnavailable as u32,
        Error::InvalidOracleConfig as u32,
        Error::OracleStale as u32,
        Error::OracleNoConsensus as u32,
        Error::OracleVerified as u32,
        Error::MarketNotReady as u32,
        Error::FallbackOracleUnavailable as u32,
        Error::ResolutionTimeoutReached as u32,
        Error::OracleConfidenceTooWide as u32,
        Error::InvalidOracleFeed as u32,
        Error::OracleCallbackAuthFailed as u32,
        Error::OracleCallbackUnauthorized as u32,
        Error::OracleCallbackInvalidSignature as u32,
        Error::OracleCallbackReplayDetected as u32,
        Error::OracleCallbackTimeout as u32,
    ];

    // All codes should be in 200-214 range
    for (index, code) in oracle_codes.iter().enumerate() {
        assert_eq!(*code, 200u32 + index as u32);
        assert!(*code >= 200 && *code <= 214);
        assert!(!oracle_codes[..index].contains(code));
    }
}

/// Market state errors are unique and contiguous within 101-104 range.
#[test]
fn resolution_market_state_error_codes_are_unique_and_contiguous() {
    let market_codes = [
        Error::MarketNotFound as u32,
        Error::MarketClosed as u32,
        Error::MarketResolved as u32,
        Error::MarketNotResolved as u32,
    ];

    // All codes should be in 101-104 range
    for (index, code) in market_codes.iter().enumerate() {
        assert_eq!(*code, 101u32 + index as u32);
        assert!(*code >= 101 && *code <= 104);
        assert!(!market_codes[..index].contains(code));
    }
}

/// Resolution error codes do not have duplicates across categories.
#[test]
fn resolution_error_codes_are_globally_unique() {
    let oracle_errors = [
        Error::OracleUnavailable as u32,
        Error::InvalidOracleConfig as u32,
        Error::OracleStale as u32,
        Error::OracleNoConsensus as u32,
        Error::OracleVerified as u32,
        Error::MarketNotReady as u32,
        Error::FallbackOracleUnavailable as u32,
        Error::ResolutionTimeoutReached as u32,
        Error::OracleConfidenceTooWide as u32,
        Error::InvalidOracleFeed as u32,
        Error::OracleCallbackAuthFailed as u32,
        Error::OracleCallbackUnauthorized as u32,
        Error::OracleCallbackInvalidSignature as u32,
        Error::OracleCallbackReplayDetected as u32,
        Error::OracleCallbackTimeout as u32,
    ];

    let market_errors = [
        Error::MarketNotFound as u32,
        Error::MarketClosed as u32,
        Error::MarketResolved as u32,
        Error::MarketNotResolved as u32,
    ];

    let validation_errors = [
        Error::InvalidQuestion as u32,
        Error::InvalidOutcomes as u32,
        Error::InvalidDuration as u32,
        Error::InvalidThreshold as u32,
        Error::InvalidComparison as u32,
    ];

    // Check no duplicates within categories
    for (i, code) in oracle_errors.iter().enumerate() {
        assert!(!oracle_errors[..i].contains(code));
    }
    for (i, code) in market_errors.iter().enumerate() {
        assert!(!market_errors[..i].contains(code));
    }
    for (i, code) in validation_errors.iter().enumerate() {
        assert!(!validation_errors[..i].contains(code));
    }

    // Check no cross-category duplicates for resolution errors
    for oracle_code in &oracle_errors {
        for market_code in &market_errors {
            assert_ne!(oracle_code, market_code);
        }
        for val_code in &validation_errors {
            assert_ne!(oracle_code, val_code);
        }
    }
    for market_code in &market_errors {
        for val_code in &validation_errors {
            assert_ne!(market_code, val_code);
        }
    }
}

/// Resolution-critical error codes remain stable across ranges.
/// These are the most important codes for resolution workflows.
#[test]
fn resolution_critical_error_codes_never_change() {
    // The absolute critical error codes for any resolution workflow
    assert_eq!(Error::MarketNotFound as u32, 101);
    assert_eq!(Error::MarketResolved as u32, 103);
    assert_eq!(Error::OracleUnavailable as u32, 200);
    assert_eq!(Error::InvalidOracleConfig as u32, 201);
    assert_eq!(Error::OracleVerified as u32, 204);
    assert_eq!(Error::InvalidInput as u32, 401);
}

/// Oracle error code ranges are isolated from other error ranges.
/// Oracle errors (200-214) must not overlap with:
/// - User operations (100-112)
/// - Validation (300-304)
/// - System (400+)
#[test]
fn resolution_oracle_error_range_does_not_overlap() {
    let oracle_min = 200u32;
    let oracle_max = 214u32;

    // User operation range
    assert!(Error::Unauthorized as u32 < oracle_min);
    assert!(Error::InsufficientBalance as u32 < oracle_min);

    // Validation range (starts after oracle range)
    assert!(Error::InvalidQuestion as u32 > oracle_max);
    assert!(Error::InvalidComparison as u32 > oracle_max);

    // System range (starts after validation range)
    assert!(Error::InvalidState as u32 > Error::InvalidComparison as u32);
}

/// All resolution error codes are non-negative and in valid u32 range.
#[test]
fn resolution_error_codes_are_valid_u32() {
    let resolution_errors = [
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
        Error::MarketNotFound,
        Error::MarketClosed,
        Error::MarketResolved,
        Error::MarketNotResolved,
    ];

    for error in &resolution_errors {
        let code = *error as u32;
        assert!(code > 0, "Error code should be positive");
        assert!(code < 1000, "Error code should be reasonable (< 1000)");
    }
}

/// Clients can reliably match on resolution error codes via bitwise comparison.
#[test]
fn resolution_error_codes_support_bitwise_comparison() {
    // Clients may use bitwise operations to categorize errors
    // Oracle range is 200-214
    let oracle_unavailable = Error::OracleUnavailable as u32;
    assert_eq!(oracle_unavailable & 0xF0, 0xC8); // 200 = 0xC8

    // Market range is 101-104
    let market_not_found = Error::MarketNotFound as u32;
    assert_eq!(market_not_found & 0xF0, 0x60); // 101 = 0x65

    // Validation range starts at 300
    let invalid_question = Error::InvalidQuestion as u32;
    assert_eq!(invalid_question & 0xF0, 0x120); // 300 = 0x12C
}
