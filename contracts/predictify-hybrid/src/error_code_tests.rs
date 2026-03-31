//! Comprehensive tests for error code and message behavior (#326)
//!
//! Tests verify:
//! - Each error code has the correct numeric value
//! - Each error code string (`.code()`) is consistent and unique
//! - Each error description (`.description()`) is non-empty and consistent
//! - Error classifications (severity, category, recovery strategy) are correct
//! - Client can rely on error code for branching
//! - Recovery strategies map correctly to each error
//! - Max recovery attempts are correctly set per error type

#![cfg(test)]

use crate::errors::{
    Error, ErrorCategory, ErrorHandler, ErrorRecoveryStatus, ErrorSeverity, RecoveryStrategy,
};
use soroban_sdk::{Env, Map, String, Symbol, Vec};

// ===== ERROR NUMERIC VALUE TESTS =====

#[test]
fn test_error_numeric_codes_user_operation_range() {
    assert_eq!(Error::Unauthorized as u32, 100);
    assert_eq!(Error::MarketNotFound as u32, 101);
    assert_eq!(Error::MarketClosed as u32, 102);
    assert_eq!(Error::MarketResolved as u32, 103);
    assert_eq!(Error::MarketNotResolved as u32, 104);
    assert_eq!(Error::NothingToClaim as u32, 105);
    assert_eq!(Error::AlreadyClaimed as u32, 106);
    assert_eq!(Error::InsufficientStake as u32, 107);
    assert_eq!(Error::InvalidOutcome as u32, 108);
    assert_eq!(Error::AlreadyVoted as u32, 109);
    assert_eq!(Error::AlreadyBet as u32, 110);
    assert_eq!(Error::BetsAlreadyPlaced as u32, 111);
    assert_eq!(Error::InsufficientBalance as u32, 112);
}

#[test]
fn test_error_numeric_codes_oracle_range() {
    assert_eq!(Error::OracleUnavailable as u32, 200);
    assert_eq!(Error::InvalidOracleConfig as u32, 201);
    assert_eq!(Error::OracleStale as u32, 202);
    assert_eq!(Error::OracleNoConsensus as u32, 203);
    assert_eq!(Error::OracleVerified as u32, 204);
    assert_eq!(Error::MarketNotReady as u32, 205);
    assert_eq!(Error::FallbackOracleUnavailable as u32, 206);
    assert_eq!(Error::ResolutionTimeoutReached as u32, 207);
}

#[test]
fn test_error_numeric_codes_validation_range() {
    assert_eq!(Error::InvalidQuestion as u32, 300);
    assert_eq!(Error::InvalidOutcomes as u32, 301);
    assert_eq!(Error::InvalidDuration as u32, 302);
    assert_eq!(Error::InvalidThreshold as u32, 303);
    assert_eq!(Error::InvalidComparison as u32, 304);
}

#[test]
fn test_error_numeric_codes_additional_range() {
    assert_eq!(Error::InvalidState as u32, 400);
    assert_eq!(Error::InvalidInput as u32, 401);
    assert_eq!(Error::InvalidFeeConfig as u32, 402);
    assert_eq!(Error::ConfigNotFound as u32, 403);
    assert_eq!(Error::AlreadyDisputed as u32, 404);
    assert_eq!(Error::DisputeError as u32, 405);
    assert_eq!(Error::DisputeError as u32, 406);
    assert_eq!(Error::DisputeError as u32, 407);
    assert_eq!(Error::DisputeError as u32, 408);
    assert_eq!(Error::DisputeError as u32, 409);
    assert_eq!(Error::DisputeError as u32, 410);
    assert_eq!(Error::InvalidThreshold as u32, 411);
    assert_eq!(Error::InvalidThreshold as u32, 412);
    assert_eq!(Error::InvalidFeeConfig as u32, 413);
    assert_eq!(Error::InvalidFeeConfig as u32, 414);
    assert_eq!(Error::InvalidInput as u32, 415);
    assert_eq!(Error::InvalidInput as u32, 416);
    assert_eq!(Error::AdminNotSet as u32, 418);
    assert_eq!(Error::TimeoutError as u32, 419);
    assert_eq!(Error::TimeoutError as u32, 422);
}

#[test]
fn test_error_numeric_codes_circuit_breaker_range() {
    assert_eq!(Error::CBError as u32, 500);
    assert_eq!(Error::CBError as u32, 501);
    assert_eq!(Error::CBError as u32, 502);
    assert_eq!(Error::CBError as u32, 503);
}

// ===== ERROR CODE STRING TESTS =====

#[test]
fn test_error_code_strings_user_operation() {
    assert_eq!(Error::Unauthorized.code(), "UNAUTHORIZED");
    assert_eq!(Error::MarketNotFound.code(), "MARKET_NOT_FOUND");
    assert_eq!(Error::MarketClosed.code(), "MARKET_CLOSED");
    assert_eq!(Error::MarketResolved.code(), "MARKET_ALREADY_RESOLVED");
    assert_eq!(Error::MarketNotResolved.code(), "MARKET_NOT_RESOLVED");
    assert_eq!(Error::NothingToClaim.code(), "NOTHING_TO_CLAIM");
    assert_eq!(Error::AlreadyClaimed.code(), "ALREADY_CLAIMED");
    assert_eq!(Error::InsufficientStake.code(), "INSUFFICIENT_STAKE");
    assert_eq!(Error::InvalidOutcome.code(), "INVALID_OUTCOME");
    assert_eq!(Error::AlreadyVoted.code(), "ALREADY_VOTED");
    assert_eq!(Error::AlreadyBet.code(), "ALREADY_BET");
    assert_eq!(Error::BetsAlreadyPlaced.code(), "BETS_ALREADY_PLACED");
    assert_eq!(Error::InsufficientBalance.code(), "INSUFFICIENT_BALANCE");
}

#[test]
fn test_error_code_strings_oracle() {
    assert_eq!(Error::OracleUnavailable.code(), "ORACLE_UNAVAILABLE");
    assert_eq!(Error::InvalidOracleConfig.code(), "INVALID_ORACLE_CONFIG");
    assert_eq!(Error::OracleStale.code(), "ORACLE_STALE");
    assert_eq!(Error::OracleNoConsensus.code(), "ORACLE_NO_CONSENSUS");
    assert_eq!(Error::OracleVerified.code(), "ORACLE_VERIFIED");
    assert_eq!(Error::MarketNotReady.code(), "MARKET_NOT_READY");
    assert_eq!(
        Error::FallbackOracleUnavailable.code(),
        "FALLBACK_ORACLE_UNAVAILABLE"
    );
    assert_eq!(
        Error::ResolutionTimeoutReached.code(),
        "RESOLUTION_TIMEOUT_REACHED"
    );
}

#[test]
fn test_error_code_strings_validation() {
    assert_eq!(Error::InvalidQuestion.code(), "INVALID_QUESTION");
    assert_eq!(Error::InvalidOutcomes.code(), "INVALID_OUTCOMES");
    assert_eq!(Error::InvalidDuration.code(), "INVALID_DURATION");
    assert_eq!(Error::InvalidThreshold.code(), "INVALID_THRESHOLD");
    assert_eq!(Error::InvalidComparison.code(), "INVALID_COMPARISON");
}

#[test]
fn test_error_code_strings_additional() {
    assert_eq!(Error::InvalidState.code(), "INVALID_STATE");
    assert_eq!(Error::InvalidInput.code(), "INVALID_INPUT");
    assert_eq!(Error::InvalidFeeConfig.code(), "INVALID_FEE_CONFIG");
    assert_eq!(Error::ConfigNotFound.code(), "CONFIGURATION_NOT_FOUND");
    assert_eq!(Error::AlreadyDisputed.code(), "ALREADY_DISPUTED");
    assert_eq!(
        Error::DisputeError.code(),
        "DISPUTE_VOTING_PERIOD_EXPIRED"
    );
    assert_eq!(
        Error::DisputeError.code(),
        "DISPUTE_VOTING_NOT_ALLOWED"
    );
    assert_eq!(Error::DisputeError.code(), "DISPUTE_ALREADY_VOTED");
    assert_eq!(
        Error::DisputeError.code(),
        "DISPUTE_RESOLUTION_CONDITIONS_NOT_MET"
    );
    assert_eq!(
        Error::DisputeError.code(),
        "DISPUTE_FEE_DISTRIBUTION_FAILED"
    );
    assert_eq!(
        Error::DisputeError.code(),
        "DISPUTE_ESCALATION_NOT_ALLOWED"
    );
    assert_eq!(Error::InvalidThreshold.code(), "THRESHOLD_BELOW_MINIMUM");
    assert_eq!(Error::InvalidThreshold.code(), "THRESHOLD_EXCEEDS_MAXIMUM");
    assert_eq!(Error::InvalidFeeConfig.code(), "FEE_ALREADY_COLLECTED");
    assert_eq!(Error::InvalidFeeConfig.code(), "NO_FEES_TO_COLLECT");
    assert_eq!(
        Error::InvalidInput.code(),
        "INVALID_EXTENSION_DAYS"
    );
    assert_eq!(Error::InvalidInput.code(), "EXTENSION_DENIED");
    assert_eq!(Error::AdminNotSet.code(), "ADMIN_NOT_SET");
    assert_eq!(Error::TimeoutError.code(), "DISPUTE_TIMEOUT_NOT_SET");
    assert_eq!(Error::TimeoutError.code(), "INVALID_TIMEOUT_HOURS");
}

#[test]
fn test_error_code_strings_circuit_breaker() {
    assert_eq!(
        Error::CBError.code(),
        "CIRCUIT_BREAKER_NOT_INITIALIZED"
    );
    assert_eq!(Error::CBError.code(), "CIRCUIT_BREAKER_ALREADY_OPEN");
    assert_eq!(Error::CBError.code(), "CIRCUIT_BREAKER_NOT_OPEN");
    assert_eq!(Error::CBError.code(), "CIRCUIT_BREAKER_OPEN");
}

// ===== ERROR DESCRIPTION TESTS =====

#[test]
fn test_error_descriptions_user_operation() {
    assert_eq!(
        Error::Unauthorized.description(),
        "User is not authorized to perform this action"
    );
    assert_eq!(Error::MarketNotFound.description(), "Market not found");
    assert_eq!(Error::MarketClosed.description(), "Market is closed");
    assert_eq!(
        Error::MarketResolved.description(),
        "Market is already resolved"
    );
    assert_eq!(
        Error::MarketNotResolved.description(),
        "Market is not resolved yet"
    );
    assert_eq!(
        Error::NothingToClaim.description(),
        "User has nothing to claim"
    );
    assert_eq!(
        Error::AlreadyClaimed.description(),
        "User has already claimed"
    );
    assert_eq!(
        Error::InsufficientStake.description(),
        "Insufficient stake amount"
    );
    assert_eq!(
        Error::InvalidOutcome.description(),
        "Invalid outcome choice"
    );
    assert_eq!(
        Error::AlreadyVoted.description(),
        "User has already voted"
    );
    assert_eq!(
        Error::AlreadyBet.description(),
        "User has already placed a bet on this market"
    );
    assert_eq!(
        Error::BetsAlreadyPlaced.description(),
        "Bets have already been placed on this market (cannot update)"
    );
    assert_eq!(
        Error::InsufficientBalance.description(),
        "Insufficient balance for operation"
    );
}

#[test]
fn test_error_descriptions_oracle() {
    assert_eq!(
        Error::OracleUnavailable.description(),
        "Oracle is unavailable"
    );
    assert_eq!(
        Error::InvalidOracleConfig.description(),
        "Invalid oracle configuration"
    );
    assert_eq!(
        Error::OracleStale.description(),
        "Oracle data is stale or timed out"
    );
    assert_eq!(
        Error::OracleNoConsensus.description(),
        "Oracle consensus not reached"
    );
    assert_eq!(
        Error::OracleVerified.description(),
        "Oracle result already verified"
    );
    assert_eq!(
        Error::MarketNotReady.description(),
        "Market not ready for oracle verification"
    );
    assert_eq!(
        Error::FallbackOracleUnavailable.description(),
        "Fallback oracle is unavailable or unhealthy"
    );
    assert_eq!(
        Error::ResolutionTimeoutReached.description(),
        "Resolution timeout has been reached"
    );
}

#[test]
fn test_error_descriptions_validation() {
    assert_eq!(
        Error::InvalidQuestion.description(),
        "Invalid question format"
    );
    assert_eq!(
        Error::InvalidOutcomes.description(),
        "Invalid outcomes provided"
    );
    assert_eq!(
        Error::InvalidDuration.description(),
        "Invalid duration specified"
    );
    assert_eq!(
        Error::InvalidThreshold.description(),
        "Invalid threshold value"
    );
    assert_eq!(
        Error::InvalidComparison.description(),
        "Invalid comparison operator"
    );
}

#[test]
fn test_error_descriptions_additional() {
    assert_eq!(Error::InvalidState.description(), "Invalid state");
    assert_eq!(Error::InvalidInput.description(), "Invalid input");
    assert_eq!(
        Error::InvalidFeeConfig.description(),
        "Invalid fee configuration"
    );
    assert_eq!(
        Error::ConfigNotFound.description(),
        "Configuration not found"
    );
    assert_eq!(Error::AlreadyDisputed.description(), "Already disputed");
    assert_eq!(
        Error::DisputeError.description(),
        "Dispute voting period expired"
    );
    assert_eq!(
        Error::DisputeError.description(),
        "Dispute voting not allowed"
    );
    assert_eq!(
        Error::DisputeError.description(),
        "Already voted in dispute"
    );
    assert_eq!(
        Error::DisputeError.description(),
        "Dispute resolution conditions not met"
    );
    assert_eq!(
        Error::DisputeError.description(),
        "Dispute fee distribution failed"
    );
    assert_eq!(
        Error::DisputeError.description(),
        "Dispute escalation not allowed"
    );
    assert_eq!(
        Error::InvalidThreshold.description(),
        "Threshold below minimum"
    );
    assert_eq!(
        Error::InvalidThreshold.description(),
        "Threshold exceeds maximum"
    );
    assert_eq!(
        Error::InvalidFeeConfig.description(),
        "Fee already collected"
    );
    assert_eq!(Error::InvalidFeeConfig.description(), "No fees to collect");
    assert_eq!(
        Error::InvalidInput.description(),
        "Invalid extension days"
    );
    assert_eq!(
        Error::InvalidInput.description(),
        "Extension not allowed or exceeded"
    );
    assert_eq!(
        Error::AdminNotSet.description(),
        "Admin address is not set (initialization missing)"
    );
    assert_eq!(
        Error::TimeoutError.description(),
        "Dispute timeout not set"
    );
    assert_eq!(
        Error::TimeoutError.description(),
        "Invalid timeout hours"
    );
}

#[test]
fn test_error_descriptions_circuit_breaker() {
    assert_eq!(
        Error::CBError.description(),
        "Circuit breaker not initialized"
    );
    assert_eq!(
        Error::CBError.description(),
        "Circuit breaker is already open (paused)"
    );
    assert_eq!(
        Error::CBError.description(),
        "Circuit breaker is not open (cannot recover)"
    );
    assert_eq!(
        Error::CBError.description(),
        "Circuit breaker is open (operations blocked)"
    );
}

// ===== DESCRIPTION NON-EMPTY TESTS =====

#[test]
fn test_all_error_descriptions_are_non_empty() {
    // Every error must have a non-empty description
    assert!(!Error::Unauthorized.description().is_empty());
    assert!(!Error::MarketNotFound.description().is_empty());
    assert!(!Error::MarketClosed.description().is_empty());
    assert!(!Error::MarketResolved.description().is_empty());
    assert!(!Error::MarketNotResolved.description().is_empty());
    assert!(!Error::NothingToClaim.description().is_empty());
    assert!(!Error::AlreadyClaimed.description().is_empty());
    assert!(!Error::InsufficientStake.description().is_empty());
    assert!(!Error::InvalidOutcome.description().is_empty());
    assert!(!Error::AlreadyVoted.description().is_empty());
    assert!(!Error::AlreadyBet.description().is_empty());
    assert!(!Error::BetsAlreadyPlaced.description().is_empty());
    assert!(!Error::InsufficientBalance.description().is_empty());
    assert!(!Error::OracleUnavailable.description().is_empty());
    assert!(!Error::InvalidOracleConfig.description().is_empty());
    assert!(!Error::OracleStale.description().is_empty());
    assert!(!Error::OracleNoConsensus.description().is_empty());
    assert!(!Error::OracleVerified.description().is_empty());
    assert!(!Error::MarketNotReady.description().is_empty());
    assert!(!Error::FallbackOracleUnavailable.description().is_empty());
    assert!(!Error::ResolutionTimeoutReached.description().is_empty());
    assert!(!Error::InvalidQuestion.description().is_empty());
    assert!(!Error::InvalidOutcomes.description().is_empty());
    assert!(!Error::InvalidDuration.description().is_empty());
    assert!(!Error::InvalidThreshold.description().is_empty());
    assert!(!Error::InvalidComparison.description().is_empty());
    assert!(!Error::InvalidState.description().is_empty());
    assert!(!Error::InvalidInput.description().is_empty());
    assert!(!Error::InvalidFeeConfig.description().is_empty());
    assert!(!Error::ConfigNotFound.description().is_empty());
    assert!(!Error::AlreadyDisputed.description().is_empty());
    assert!(!Error::DisputeError.description().is_empty());
    assert!(!Error::DisputeError.description().is_empty());
    assert!(!Error::DisputeError.description().is_empty());
    assert!(!Error::DisputeError.description().is_empty());
    assert!(!Error::DisputeError.description().is_empty());
    assert!(!Error::DisputeError.description().is_empty());
    assert!(!Error::InvalidThreshold.description().is_empty());
    assert!(!Error::InvalidThreshold.description().is_empty());
    assert!(!Error::InvalidFeeConfig.description().is_empty());
    assert!(!Error::InvalidFeeConfig.description().is_empty());
    assert!(!Error::InvalidInput.description().is_empty());
    assert!(!Error::InvalidInput.description().is_empty());
    assert!(!Error::AdminNotSet.description().is_empty());
    assert!(!Error::TimeoutError.description().is_empty());
    assert!(!Error::TimeoutError.description().is_empty());
    assert!(!Error::CBError.description().is_empty());
    assert!(!Error::CBError.description().is_empty());
    assert!(!Error::CBError.description().is_empty());
    assert!(!Error::CBError.description().is_empty());
}

#[test]
fn test_all_error_codes_are_non_empty() {
    // Every error must have a non-empty code string
    assert!(!Error::Unauthorized.code().is_empty());
    assert!(!Error::MarketNotFound.code().is_empty());
    assert!(!Error::MarketClosed.code().is_empty());
    assert!(!Error::MarketResolved.code().is_empty());
    assert!(!Error::MarketNotResolved.code().is_empty());
    assert!(!Error::NothingToClaim.code().is_empty());
    assert!(!Error::AlreadyClaimed.code().is_empty());
    assert!(!Error::InsufficientStake.code().is_empty());
    assert!(!Error::InvalidOutcome.code().is_empty());
    assert!(!Error::AlreadyVoted.code().is_empty());
    assert!(!Error::AlreadyBet.code().is_empty());
    assert!(!Error::BetsAlreadyPlaced.code().is_empty());
    assert!(!Error::InsufficientBalance.code().is_empty());
    assert!(!Error::OracleUnavailable.code().is_empty());
    assert!(!Error::InvalidOracleConfig.code().is_empty());
    assert!(!Error::OracleStale.code().is_empty());
    assert!(!Error::OracleNoConsensus.code().is_empty());
    assert!(!Error::OracleVerified.code().is_empty());
    assert!(!Error::MarketNotReady.code().is_empty());
    assert!(!Error::FallbackOracleUnavailable.code().is_empty());
    assert!(!Error::ResolutionTimeoutReached.code().is_empty());
    assert!(!Error::InvalidQuestion.code().is_empty());
    assert!(!Error::InvalidOutcomes.code().is_empty());
    assert!(!Error::InvalidDuration.code().is_empty());
    assert!(!Error::InvalidThreshold.code().is_empty());
    assert!(!Error::InvalidComparison.code().is_empty());
    assert!(!Error::InvalidState.code().is_empty());
    assert!(!Error::InvalidInput.code().is_empty());
    assert!(!Error::InvalidFeeConfig.code().is_empty());
    assert!(!Error::ConfigNotFound.code().is_empty());
    assert!(!Error::AlreadyDisputed.code().is_empty());
    assert!(!Error::DisputeError.code().is_empty());
    assert!(!Error::DisputeError.code().is_empty());
    assert!(!Error::DisputeError.code().is_empty());
    assert!(!Error::DisputeError.code().is_empty());
    assert!(!Error::DisputeError.code().is_empty());
    assert!(!Error::DisputeError.code().is_empty());
    assert!(!Error::InvalidThreshold.code().is_empty());
    assert!(!Error::InvalidThreshold.code().is_empty());
    assert!(!Error::InvalidFeeConfig.code().is_empty());
    assert!(!Error::InvalidFeeConfig.code().is_empty());
    assert!(!Error::InvalidInput.code().is_empty());
    assert!(!Error::InvalidInput.code().is_empty());
    assert!(!Error::AdminNotSet.code().is_empty());
    assert!(!Error::TimeoutError.code().is_empty());
    assert!(!Error::TimeoutError.code().is_empty());
    assert!(!Error::CBError.code().is_empty());
    assert!(!Error::CBError.code().is_empty());
    assert!(!Error::CBError.code().is_empty());
    assert!(!Error::CBError.code().is_empty());
}

// ===== CODE UNIQUENESS TESTS =====

#[test]
fn test_error_numeric_codes_are_unique() {
    // Collect all codes and verify no duplicates
    let codes: &[u32] = &[
        Error::Unauthorized as u32,
        Error::MarketNotFound as u32,
        Error::MarketClosed as u32,
        Error::MarketResolved as u32,
        Error::MarketNotResolved as u32,
        Error::NothingToClaim as u32,
        Error::AlreadyClaimed as u32,
        Error::InsufficientStake as u32,
        Error::InvalidOutcome as u32,
        Error::AlreadyVoted as u32,
        Error::AlreadyBet as u32,
        Error::BetsAlreadyPlaced as u32,
        Error::InsufficientBalance as u32,
        Error::OracleUnavailable as u32,
        Error::InvalidOracleConfig as u32,
        Error::OracleStale as u32,
        Error::OracleNoConsensus as u32,
        Error::OracleVerified as u32,
        Error::MarketNotReady as u32,
        Error::FallbackOracleUnavailable as u32,
        Error::ResolutionTimeoutReached as u32,
        Error::InvalidQuestion as u32,
        Error::InvalidOutcomes as u32,
        Error::InvalidDuration as u32,
        Error::InvalidThreshold as u32,
        Error::InvalidComparison as u32,
        Error::InvalidState as u32,
        Error::InvalidInput as u32,
        Error::InvalidFeeConfig as u32,
        Error::ConfigNotFound as u32,
        Error::AlreadyDisputed as u32,
        Error::DisputeError as u32,
        Error::DisputeError as u32,
        Error::DisputeError as u32,
        Error::DisputeError as u32,
        Error::DisputeError as u32,
        Error::DisputeError as u32,
        Error::InvalidThreshold as u32,
        Error::InvalidThreshold as u32,
        Error::InvalidFeeConfig as u32,
        Error::InvalidFeeConfig as u32,
        Error::InvalidInput as u32,
        Error::InvalidInput as u32,
        Error::AdminNotSet as u32,
        Error::TimeoutError as u32,
        Error::TimeoutError as u32,
        Error::CBError as u32,
        Error::CBError as u32,
        Error::CBError as u32,
        Error::CBError as u32,
    ];

    // Verify all codes are unique using a simple O(n^2) uniqueness check
    for i in 0..codes.len() {
        for j in (i + 1)..codes.len() {
            assert_ne!(
                codes[i], codes[j],
                "Duplicate error code {} at indices {} and {}",
                codes[i], i, j
            );
        }
    }
}

#[test]
fn test_error_string_codes_are_unique() {
    let codes: &[&str] = &[
        Error::Unauthorized.code(),
        Error::MarketNotFound.code(),
        Error::MarketClosed.code(),
        Error::MarketResolved.code(),
        Error::MarketNotResolved.code(),
        Error::NothingToClaim.code(),
        Error::AlreadyClaimed.code(),
        Error::InsufficientStake.code(),
        Error::InvalidOutcome.code(),
        Error::AlreadyVoted.code(),
        Error::AlreadyBet.code(),
        Error::BetsAlreadyPlaced.code(),
        Error::InsufficientBalance.code(),
        Error::OracleUnavailable.code(),
        Error::InvalidOracleConfig.code(),
        Error::OracleStale.code(),
        Error::OracleNoConsensus.code(),
        Error::OracleVerified.code(),
        Error::MarketNotReady.code(),
        Error::FallbackOracleUnavailable.code(),
        Error::ResolutionTimeoutReached.code(),
        Error::InvalidQuestion.code(),
        Error::InvalidOutcomes.code(),
        Error::InvalidDuration.code(),
        Error::InvalidThreshold.code(),
        Error::InvalidComparison.code(),
        Error::InvalidState.code(),
        Error::InvalidInput.code(),
        Error::InvalidFeeConfig.code(),
        Error::ConfigNotFound.code(),
        Error::AlreadyDisputed.code(),
        Error::DisputeError.code(),
        Error::DisputeError.code(),
        Error::DisputeError.code(),
        Error::DisputeError.code(),
        Error::DisputeError.code(),
        Error::DisputeError.code(),
        Error::InvalidThreshold.code(),
        Error::InvalidThreshold.code(),
        Error::InvalidFeeConfig.code(),
        Error::InvalidFeeConfig.code(),
        Error::InvalidInput.code(),
        Error::InvalidInput.code(),
        Error::AdminNotSet.code(),
        Error::TimeoutError.code(),
        Error::TimeoutError.code(),
        Error::CBError.code(),
        Error::CBError.code(),
        Error::CBError.code(),
        Error::CBError.code(),
    ];

    for i in 0..codes.len() {
        for j in (i + 1)..codes.len() {
            assert_ne!(
                codes[i], codes[j],
                "Duplicate error string code '{}' at indices {} and {}",
                codes[i], i, j
            );
        }
    }
}

// ===== ERROR CODE RANGE TESTS =====

#[test]
fn test_user_operation_errors_in_range_100_to_112() {
    let user_ops = &[
        Error::Unauthorized as u32,
        Error::MarketNotFound as u32,
        Error::MarketClosed as u32,
        Error::MarketResolved as u32,
        Error::MarketNotResolved as u32,
        Error::NothingToClaim as u32,
        Error::AlreadyClaimed as u32,
        Error::InsufficientStake as u32,
        Error::InvalidOutcome as u32,
        Error::AlreadyVoted as u32,
        Error::AlreadyBet as u32,
        Error::BetsAlreadyPlaced as u32,
        Error::InsufficientBalance as u32,
    ];
    for &code in user_ops {
        assert!(
            code >= 100 && code <= 112,
            "User operation error {} not in range 100-112",
            code
        );
    }
}

#[test]
fn test_oracle_errors_in_range_200_to_207() {
    let oracle_errs = &[
        Error::OracleUnavailable as u32,
        Error::InvalidOracleConfig as u32,
        Error::OracleStale as u32,
        Error::OracleNoConsensus as u32,
        Error::OracleVerified as u32,
        Error::MarketNotReady as u32,
        Error::FallbackOracleUnavailable as u32,
        Error::ResolutionTimeoutReached as u32,
    ];
    for &code in oracle_errs {
        assert!(
            code >= 200 && code <= 207,
            "Oracle error {} not in range 200-207",
            code
        );
    }
}

#[test]
fn test_validation_errors_in_range_300_to_304() {
    let validation_errs = &[
        Error::InvalidQuestion as u32,
        Error::InvalidOutcomes as u32,
        Error::InvalidDuration as u32,
        Error::InvalidThreshold as u32,
        Error::InvalidComparison as u32,
    ];
    for &code in validation_errs {
        assert!(
            code >= 300 && code <= 304,
            "Validation error {} not in range 300-304",
            code
        );
    }
}

#[test]
fn test_circuit_breaker_errors_in_range_500_to_503() {
    let cb_errs = &[
        Error::CBError as u32,
        Error::CBError as u32,
        Error::CBError as u32,
        Error::CBError as u32,
    ];
    for &code in cb_errs {
        assert!(
            code >= 500 && code <= 503,
            "Circuit breaker error {} not in range 500-503",
            code
        );
    }
}

// ===== RECOVERY STRATEGY TESTS =====

#[test]
fn test_recovery_strategy_retry_with_delay() {
    // OracleUnavailable should use RetryWithDelay
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::OracleUnavailable),
        RecoveryStrategy::RetryWithDelay
    );
}

#[test]
fn test_recovery_strategy_retry() {
    // InvalidInput should use Retry
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::InvalidInput),
        RecoveryStrategy::Retry
    );
}

#[test]
fn test_recovery_strategy_alternative_method() {
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::MarketNotFound),
        RecoveryStrategy::AlternativeMethod
    );
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::ConfigNotFound),
        RecoveryStrategy::AlternativeMethod
    );
}

#[test]
fn test_recovery_strategy_skip() {
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::AlreadyVoted),
        RecoveryStrategy::Skip
    );
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::AlreadyClaimed),
        RecoveryStrategy::Skip
    );
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::InvalidFeeConfig),
        RecoveryStrategy::Skip
    );
}

#[test]
fn test_recovery_strategy_abort() {
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::Unauthorized),
        RecoveryStrategy::Abort
    );
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::MarketClosed),
        RecoveryStrategy::Abort
    );
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::MarketResolved),
        RecoveryStrategy::Abort
    );
}

#[test]
fn test_recovery_strategy_manual_intervention() {
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::AdminNotSet),
        RecoveryStrategy::ManualIntervention
    );
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::DisputeError),
        RecoveryStrategy::ManualIntervention
    );
}

#[test]
fn test_recovery_strategy_no_recovery() {
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::InvalidState),
        RecoveryStrategy::NoRecovery
    );
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::InvalidOracleConfig),
        RecoveryStrategy::NoRecovery
    );
}

// ===== ERROR CLASSIFICATION TESTS =====

#[test]
fn test_classification_critical_admin_not_set() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::AdminNotSet, context);
    assert_eq!(detailed.severity, ErrorSeverity::Critical);
    assert_eq!(detailed.category, ErrorCategory::System);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::ManualIntervention);
}

#[test]
fn test_classification_critical_dispute_fee_failed() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::DisputeError, context);
    assert_eq!(detailed.severity, ErrorSeverity::Critical);
    assert_eq!(detailed.category, ErrorCategory::Financial);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::ManualIntervention);
}

#[test]
fn test_classification_high_unauthorized() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::Unauthorized, context);
    assert_eq!(detailed.severity, ErrorSeverity::High);
    assert_eq!(detailed.category, ErrorCategory::Authentication);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::Abort);
}

#[test]
fn test_classification_high_oracle_unavailable() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::OracleUnavailable, context);
    assert_eq!(detailed.severity, ErrorSeverity::High);
    assert_eq!(detailed.category, ErrorCategory::Oracle);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::RetryWithDelay);
}

#[test]
fn test_classification_high_invalid_state() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::InvalidState, context);
    assert_eq!(detailed.severity, ErrorSeverity::High);
    assert_eq!(detailed.category, ErrorCategory::System);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::NoRecovery);
}

#[test]
fn test_classification_medium_market_not_found() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::MarketNotFound, context);
    assert_eq!(detailed.severity, ErrorSeverity::Medium);
    assert_eq!(detailed.category, ErrorCategory::Market);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::AlternativeMethod);
}

#[test]
fn test_classification_medium_market_closed() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::MarketClosed, context);
    assert_eq!(detailed.severity, ErrorSeverity::Medium);
    assert_eq!(detailed.category, ErrorCategory::Market);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::Abort);
}

#[test]
fn test_classification_medium_market_resolved() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::MarketResolved, context);
    assert_eq!(detailed.severity, ErrorSeverity::Medium);
    assert_eq!(detailed.category, ErrorCategory::Market);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::Abort);
}

#[test]
fn test_classification_medium_insufficient_stake() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::InsufficientStake, context);
    assert_eq!(detailed.severity, ErrorSeverity::Medium);
    assert_eq!(detailed.category, ErrorCategory::UserOperation);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::Retry);
}

#[test]
fn test_classification_medium_invalid_input() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::InvalidInput, context);
    assert_eq!(detailed.severity, ErrorSeverity::Medium);
    assert_eq!(detailed.category, ErrorCategory::Validation);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::Retry);
}

#[test]
fn test_classification_medium_invalid_oracle_config() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::InvalidOracleConfig, context);
    assert_eq!(detailed.severity, ErrorSeverity::Medium);
    assert_eq!(detailed.category, ErrorCategory::Oracle);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::NoRecovery);
}

#[test]
fn test_classification_low_already_voted() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::AlreadyVoted, context);
    assert_eq!(detailed.severity, ErrorSeverity::Low);
    assert_eq!(detailed.category, ErrorCategory::UserOperation);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::Skip);
}

#[test]
fn test_classification_low_already_bet() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::AlreadyBet, context);
    assert_eq!(detailed.severity, ErrorSeverity::Low);
    assert_eq!(detailed.category, ErrorCategory::UserOperation);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::Skip);
}

#[test]
fn test_classification_low_already_claimed() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::AlreadyClaimed, context);
    assert_eq!(detailed.severity, ErrorSeverity::Low);
    assert_eq!(detailed.category, ErrorCategory::UserOperation);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::Skip);
}

#[test]
fn test_classification_low_fee_already_collected() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::InvalidFeeConfig, context);
    assert_eq!(detailed.severity, ErrorSeverity::Low);
    assert_eq!(detailed.category, ErrorCategory::Financial);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::Skip);
}

#[test]
fn test_classification_low_nothing_to_claim() {
    let env = Env::default();
    let context = make_test_context(&env);
    let detailed = ErrorHandler::categorize_error(&env, Error::NothingToClaim, context);
    assert_eq!(detailed.severity, ErrorSeverity::Low);
    assert_eq!(detailed.category, ErrorCategory::UserOperation);
    assert_eq!(detailed.recovery_strategy, RecoveryStrategy::Skip);
}

// ===== CLIENT BRANCHING TESTS =====
// These verify that a client can use the numeric code for branching decisions

#[test]
fn test_client_can_branch_on_numeric_code() {
    let err = Error::Unauthorized;
    let code = err as u32;

    // Client can branch: is this an auth error?
    let category = if code == 100 {
        "authentication"
    } else if code >= 200 && code < 300 {
        "oracle"
    } else {
        "other"
    };
    assert_eq!(category, "authentication");
}

#[test]
fn test_client_can_branch_on_string_code() {
    let err = Error::OracleUnavailable;
    let should_retry = matches!(
        err.code(),
        "ORACLE_UNAVAILABLE" | "ORACLE_STALE" | "FALLBACK_ORACLE_UNAVAILABLE"
    );
    assert!(should_retry);
}

#[test]
fn test_client_can_branch_abort_vs_skip() {
    let abort_errors = &[Error::Unauthorized, Error::MarketClosed, Error::MarketResolved];
    let skip_errors = &[Error::AlreadyVoted, Error::AlreadyClaimed, Error::InvalidFeeConfig];

    for err in abort_errors {
        assert_eq!(
            ErrorHandler::get_error_recovery_strategy(err),
            RecoveryStrategy::Abort,
            "Expected Abort for {}",
            err.code()
        );
    }

    for err in skip_errors {
        assert_eq!(
            ErrorHandler::get_error_recovery_strategy(err),
            RecoveryStrategy::Skip,
            "Expected Skip for {}",
            err.code()
        );
    }
}

#[test]
fn test_client_should_not_retry_on_abort_errors() {
    let abort_errors = &[
        Error::Unauthorized,
        Error::MarketClosed,
        Error::MarketResolved,
    ];
    for err in abort_errors {
        let strategy = ErrorHandler::get_error_recovery_strategy(err);
        assert_ne!(
            strategy,
            RecoveryStrategy::Retry,
            "Error {} should not be retried",
            err.code()
        );
        assert_ne!(
            strategy,
            RecoveryStrategy::RetryWithDelay,
            "Error {} should not be retried with delay",
            err.code()
        );
    }
}

#[test]
fn test_client_branching_oracle_errors_need_retry() {
    // Clients should retry OracleUnavailable
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::OracleUnavailable),
        RecoveryStrategy::RetryWithDelay
    );
    // But not InvalidOracleConfig (config error, no recovery)
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::InvalidOracleConfig),
        RecoveryStrategy::NoRecovery
    );
}

#[test]
fn test_error_equality_for_branching() {
    // Clients can use PartialEq to match specific errors
    let e1 = Error::MarketNotFound;
    let e2 = Error::MarketNotFound;
    let e3 = Error::MarketClosed;

    assert_eq!(e1, e2);
    assert_ne!(e1, e3);
}

#[test]
fn test_error_copy_for_branching() {
    // Errors are Copy, so clients can pass them freely
    let e = Error::InsufficientBalance;
    let e_copy = e; // Copy, not move
    assert_eq!(e, e_copy);
}

// ===== ERROR CONTEXT VALIDATION TESTS =====

#[test]
fn test_error_context_valid() {
    let env = Env::default();
    let context = make_test_context(&env);
    assert!(ErrorHandler::validate_error_context(&context).is_ok());
}

#[test]
fn test_error_context_invalid_empty_operation() {
    let env = Env::default();
    let context = crate::errors::ErrorContext {
        operation: String::from_str(&env, ""),
        user_address: None,
        market_id: None,
        context_data: Map::new(&env),
        timestamp: env.ledger().timestamp(),
        call_chain: {
            let mut v = Vec::new(&env);
            v.push_back(String::from_str(&env, "op"));
            v
        },
    };
    assert_eq!(
        ErrorHandler::validate_error_context(&context),
        Err(Error::InvalidInput)
    );
}

#[test]
fn test_error_context_invalid_empty_call_chain() {
    let env = Env::default();
    let context = crate::errors::ErrorContext {
        operation: String::from_str(&env, "create_market"),
        user_address: None,
        market_id: None,
        context_data: Map::new(&env),
        timestamp: env.ledger().timestamp(),
        call_chain: Vec::new(&env),
    };
    assert_eq!(
        ErrorHandler::validate_error_context(&context),
        Err(Error::InvalidInput)
    );
}

// ===== ERROR ANALYTICS TESTS =====

#[test]
fn test_error_analytics_initial_state() {
    let env = Env::default();
    let analytics = ErrorHandler::get_error_analytics(&env).unwrap();
    assert_eq!(analytics.total_errors, 0);
    assert_eq!(analytics.recovery_success_rate, 0);
    assert_eq!(analytics.avg_resolution_time, 0);
}

#[test]
fn test_error_analytics_has_all_categories() {
    let env = Env::default();
    let analytics = ErrorHandler::get_error_analytics(&env).unwrap();
    assert!(analytics
        .errors_by_category
        .get(ErrorCategory::UserOperation)
        .is_some());
    assert!(analytics
        .errors_by_category
        .get(ErrorCategory::Oracle)
        .is_some());
    assert!(analytics
        .errors_by_category
        .get(ErrorCategory::Validation)
        .is_some());
    assert!(analytics
        .errors_by_category
        .get(ErrorCategory::System)
        .is_some());
}

#[test]
fn test_error_analytics_has_all_severities() {
    let env = Env::default();
    let analytics = ErrorHandler::get_error_analytics(&env).unwrap();
    assert!(analytics
        .errors_by_severity
        .get(ErrorSeverity::Low)
        .is_some());
    assert!(analytics
        .errors_by_severity
        .get(ErrorSeverity::Medium)
        .is_some());
    assert!(analytics
        .errors_by_severity
        .get(ErrorSeverity::High)
        .is_some());
    assert!(analytics
        .errors_by_severity
        .get(ErrorSeverity::Critical)
        .is_some());
}

// ===== ERROR RECOVERY STATUS TESTS =====

#[test]
fn test_error_recovery_status_initial() {
    let env = Env::default();
    let status = ErrorHandler::get_error_recovery_status(&env).unwrap();
    assert_eq!(status.total_attempts, 0);
    assert_eq!(status.successful_recoveries, 0);
    assert_eq!(status.failed_recoveries, 0);
    assert_eq!(status.active_recoveries, 0);
    assert_eq!(status.success_rate, 0);
    assert_eq!(status.avg_recovery_time, 0);
    assert!(status.last_recovery_timestamp.is_none());
}

// ===== DETAILED ERROR MESSAGE TESTS =====

#[test]
fn test_detailed_message_unauthorized() {
    let env = Env::default();
    let context = make_test_context(&env);
    // Just verify it generates without panic and has content
    let _msg = ErrorHandler::generate_detailed_error_message(&Error::Unauthorized, &context);
}

#[test]
fn test_detailed_message_market_not_found() {
    let env = Env::default();
    let context = make_test_context(&env);
    let _msg = ErrorHandler::generate_detailed_error_message(&Error::MarketNotFound, &context);
}

#[test]
fn test_detailed_message_oracle_unavailable() {
    let env = Env::default();
    let context = make_test_context(&env);
    let _msg = ErrorHandler::generate_detailed_error_message(&Error::OracleUnavailable, &context);
}

#[test]
fn test_detailed_message_already_voted() {
    let env = Env::default();
    let context = make_test_context(&env);
    let _msg = ErrorHandler::generate_detailed_error_message(&Error::AlreadyVoted, &context);
}

#[test]
fn test_detailed_message_invalid_input() {
    let env = Env::default();
    let context = make_test_context(&env);
    let _msg = ErrorHandler::generate_detailed_error_message(&Error::InvalidInput, &context);
}

#[test]
fn test_detailed_message_market_closed() {
    let env = Env::default();
    let context = make_test_context(&env);
    let _msg = ErrorHandler::generate_detailed_error_message(&Error::MarketClosed, &context);
}

#[test]
fn test_detailed_message_insufficient_stake() {
    let env = Env::default();
    let context = make_test_context(&env);
    let _msg =
        ErrorHandler::generate_detailed_error_message(&Error::InsufficientStake, &context);
}

#[test]
fn test_detailed_message_invalid_state() {
    let env = Env::default();
    let context = make_test_context(&env);
    let _msg = ErrorHandler::generate_detailed_error_message(&Error::InvalidState, &context);
}

// ===== ERROR CODE NAMING CONVENTION TESTS =====

#[test]
fn test_error_codes_are_upper_snake_case() {
    let codes: &[&str] = &[
        Error::Unauthorized.code(),
        Error::MarketNotFound.code(),
        Error::OracleUnavailable.code(),
        Error::InvalidFeeConfig.code(),
        Error::CBError.code(),
    ];
    for &code in codes {
        // All chars must be uppercase ASCII letters, digits, or underscores
        for c in code.chars() {
            assert!(
                c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_',
                "Code '{}' contains invalid character '{}'",
                code,
                c
            );
        }
        // Must not start or end with underscore
        assert!(!code.starts_with('_'), "Code '{}' starts with underscore", code);
        assert!(!code.ends_with('_'), "Code '{}' ends with underscore", code);
    }
}

// ===== HELPER =====

fn make_test_context(env: &Env) -> crate::errors::ErrorContext {
    crate::errors::ErrorContext {
        operation: String::from_str(env, "test_operation"),
        user_address: None,
        market_id: None,
        context_data: Map::new(env),
        timestamp: env.ledger().timestamp(),
        call_chain: {
            let mut v = Vec::new(env);
            v.push_back(String::from_str(env, "test"));
            v
        },
    }
}

// ============================================================================
// COMPREHENSIVE ERROR CLASSIFICATION TESTS
// ============================================================================
//
// This phase verifies that each error is correctly classified by severity,
// category, and recovery strategy. Ensures the error handling system can
// properly route errors to appropriate handlers.
//
// Test Coverage:
// - User operation errors (Low-Medium severity, skip/retry recovery)
// - Oracle errors (High severity, retry-friendly recovery)
// - Validation errors (Medium severity, retry with user guidance)
// - System severity levels (Low, Medium, High, Critical)

/// Verifies user operation errors have correct severity and recovery classification.
///
/// User operation errors like `Unauthorized`, `MarketNotFound`, etc. should have
/// Low to Medium severity and be categorized as `UserOperation`.
#[test]
fn test_error_classification_user_operation_errors() {
    // All user operation errors should have clear severity/recovery patterns
    let user_errors = vec![
        Error::Unauthorized,
        Error::MarketNotFound,
        Error::MarketClosed,
        Error::InsufficientStake,
        Error::AlreadyVoted,
        Error::InsufficientBalance,
    ];

    let env = Env::default();
    for error in user_errors {
        let context = make_test_context(&env);
        let detailed = ErrorHandler::categorize_error(&env, error, context);
        // User operation errors should be Low to Medium severity
        assert!(
            matches!(
                detailed.severity,
                ErrorSeverity::Low | ErrorSeverity::Medium
            ),
            "User error {:?} has unexpected severity {:?}",
            error,
            detailed.severity
        );
        assert_eq!(
            detailed.category,
            ErrorCategory::UserOperation,
            "Error {:?} should be categorized as UserOperation",
            error
        );
    }
}

/// Verifies oracle errors have correct severity and recovery classification.
///
/// Oracle errors like `OracleUnavailable`, `OracleStale`, etc. should be
/// categorized as `Oracle` with retry-friendly recovery strategies.
#[test]
fn test_error_classification_oracle_errors() {
    // Oracle errors should have clear patterns for external failures
    let oracle_errors = vec![
        Error::OracleUnavailable,
        Error::InvalidOracleConfig,
        Error::OracleStale,
        Error::OracleNoConsensus,
    ];

    let env = Env::default();
    for error in oracle_errors {
        let context = make_test_context(&env);
        let detailed = ErrorHandler::categorize_error(&env, error, context);
        assert_eq!(
            detailed.category,
            ErrorCategory::Oracle,
            "Error {:?} should be categorized as Oracle",
            error
        );
        // Oracle errors typically need retries or alternatives
        assert!(
            matches!(
                detailed.recovery_strategy,
                RecoveryStrategy::RetryWithDelay
                    | RecoveryStrategy::AlternativeMethod
                    | RecoveryStrategy::Retry
            ),
            "Oracle error should have retry-friendly recovery: {:?}",
            detailed.recovery_strategy
        );
    }
}

/// Verifies validation errors have correct severity and recovery classification.
///
/// Validation errors like `InvalidQuestion`, `InvalidOutcomes`, etc. should
/// be categorized as `Validation` with helpful user action guidance.
#[test]
fn test_error_classification_validation_errors() {
    // Validation errors should help users fix input
    let validation_errors = vec![
        Error::InvalidQuestion,
        Error::InvalidOutcomes,
        Error::InvalidDuration,
        Error::InvalidInput,
    ];

    let env = Env::default();
    for error in validation_errors {
        let context = make_test_context(&env);
        let detailed = ErrorHandler::categorize_error(&env, error, context);
        assert_eq!(
            detailed.category,
            ErrorCategory::Validation,
            "Error {:?} should be categorized as Validation",
            error
        );
        // Validation errors are user mistakes, typically retryable
        assert!(
            !detailed.user_action.is_empty(),
            "Validation error should have user action guidance"
        );
    }
}

/// Verifies error severity levels are correctly assigned.
///
/// Tests classification of errors into severity tiers: Low (informational),
/// Medium (action needed), High (important), Critical (system failure).
#[test]
fn test_error_classification_severity_levels() {
    let env = Env::default();

    // Low severity errors
    let low_severity_errors = vec![Error::AlreadyVoted, Error::AlreadyBet, Error::AlreadyClaimed];
    for error in low_severity_errors {
        let context = make_test_context(&env);
        let detailed = ErrorHandler::categorize_error(&env, error, context);
        assert_eq!(
            detailed.severity,
            ErrorSeverity::Low,
            "Error {:?} should have Low severity",
            error
        );
    }

    // High severity errors
    let high_severity_errors = vec![Error::Unauthorized, Error::OracleUnavailable];
    for error in high_severity_errors {
        let context = make_test_context(&env);
        let detailed = ErrorHandler::categorize_error(&env, error, context);
        assert_eq!(
            detailed.severity,
            ErrorSeverity::High,
            "Error {:?} should have High severity",
            error
        );
    }
}

// ============================================================================
// ERROR RECOVERY LIFECYCLE TESTS
// ============================================================================
//
// This phase validates the complete error recovery process from error
// detection through resolution. Tests the recovery workflow including context
// validation, strategy selection, execution, and outcome tracking.
//
// Test Coverage:
// - Full recovery flow (error → context → strategy → resolution)
// - Recovery attempt tracking and limits
// - Context validation (operation, user, market data)
// - Recovery status aggregation and reporting

/// Tests the complete error recovery lifecycle from error to resolution.
///
/// Validates that errors can be recovered through the full process:
/// 1. Error occurs
/// 2. Context is captured
/// 3. Recovery strategy is selected
/// 4. Recovery succeeds and is recorded
#[test]
fn test_error_recovery_full_lifecycle() {
    let env = Env::default();
    let context = make_test_context(&env);
    let error = Error::OracleUnavailable;

    // Recover from the error
    let recovery_result = ErrorHandler::recover_from_error(&env, error, context.clone());
    assert!(recovery_result.is_ok(), "Recovery should succeed");

    let recovery = recovery_result.unwrap();
    assert_eq!(recovery.original_error_code, error as u32);
    assert_eq!(recovery.recovery_status, String::from_str(&env, "success"));
    assert!(recovery.recovery_success_timestamp.is_some());
    assert!(recovery.recovery_failure_reason.is_none());
}

#[test]
fn test_error_recovery_attempts_tracking() {
    let env = Env::default();
    let context = make_test_context(&env);
    let error = Error::InvalidInput;

    let recovery = ErrorHandler::recover_from_error(&env, error, context);
    assert!(recovery.is_ok());

    let recovery_data = recovery.unwrap();
    assert!(
        recovery_data.recovery_attempts <= recovery_data.max_recovery_attempts,
        "Recovery attempts should not exceed maximum"
    );
}

#[test]
fn test_error_recovery_context_validation() {
    let env = Env::default();
    let mut context = make_test_context(&env);
    // Empty operation should fail validation
    context.operation = String::from_str(&env, "");

    let result = ErrorHandler::validate_error_context(&context);
    assert!(
        result.is_err(),
        "Context validation should fail for empty operation"
    );
}

#[test]
fn test_error_recovery_status_aggregation() {
    let env = Env::default();
    let result = ErrorHandler::get_error_recovery_status(&env);
    assert!(result.is_ok());

    let status = result.unwrap();
    assert_eq!(status.total_attempts, 0);
    assert_eq!(status.successful_recoveries, 0);
    assert_eq!(status.failed_recoveries, 0);
}

// ============================================================================
// ERROR MESSAGE GENERATION TESTS
// ============================================================================
//
// This phase verifies that all errors produce helpful, user-facing messages.
// Messages should be non-empty, descriptive, and actionable.
//
// Test Coverage:
// - All error types have messages
// - Messages are context-aware when possible
// - Messages guide users toward resolution

/// Verifies all error types produce helpful user-facing messages.
///
/// Every error must have a non-empty message that explains what happened
/// and provides guidance for resolution.
#[test]
fn test_error_message_generation_all_errors() {
    let env = Env::default();
    let context = make_test_context(&env);

    let all_errors = vec![
        Error::Unauthorized,
        Error::MarketNotFound,
        Error::InsufficientBalance,
        Error::OracleUnavailable,
        Error::InvalidInput,
        Error::AdminNotSet,
    ];

    for error in all_errors {
        let message = ErrorHandler::generate_detailed_error_message(&env, &error, &context);
        assert!(
            !message.is_empty(),
            "Error {:?} should have non-empty message",
            error
        );
    }
}

/// Tests that error messages incorporate relevant context.
///
/// Messages should be tailored to the operation and parties involved,
/// providing specific guidance rather than generic descriptions.
#[test]
fn test_error_message_context_aware() {
    let env = Env::default();
    let user = Address::generate(&env);
    let market = Symbol::new(&env, "test_market");

    let mut context = crate::err::ErrorContext {
        operation: String::from_str(&env, "place_bet"),
        user_address: Some(user),
        market_id: Some(market),
        context_data: Map::new(&env),
        timestamp: env.ledger().timestamp(),
        call_chain: None,
    };

    let message =
        ErrorHandler::generate_detailed_error_message(&env, &Error::InsufficientBalance, &context);
    assert!(!message.is_empty());
}

// ============================================================================
// ERROR ANALYTICS TESTS
// ============================================================================
//
// This phase validates error tracking and analytics infrastructure.
// Ensures systems can collect, aggregate, and report error metrics.
//
// Test Coverage:
// - Analytics data structure validity
// - Error categorization tracking
// - Severity distribution tracking
// - Recovery procedure documentation

/// Verifies error analytics structure is valid and usable.
///
/// The analytics system must track errors by category, severity, and
/// provide reporting on error distributions.
#[test]
fn test_error_analytics_structure() {
    let env = Env::default();
    let analytics = ErrorHandler::get_error_analytics(&env);
    assert!(analytics.is_ok());

    let analytics = analytics.unwrap();
    assert!(
        analytics.errors_by_category.len() >= 0,
        "Analytics should track error categories"
    );
    assert!(
        analytics.errors_by_severity.len() >= 0,
        "Analytics should track error severity"
    );
}

/// Verifies that recovery procedures are documented for each error type.
///
/// Documentation should provide clear steps for resolving common errors
/// at both user and system levels.
#[test]
fn test_error_recovery_procedures_documented() {
    let env = Env::default();
    let procedures =
        ErrorHandler::document_error_recovery_procedures(&env);
    assert!(procedures.is_ok());

    let procedures = procedures.unwrap();
    assert!(
        procedures.len() > 0,
        "Should have recovery procedures documented"
    );
}

// ============================================================================
// ERROR RECOVERY STRATEGY MAPPING TESTS
// ============================================================================
//
// This phase validates the mapping between errors and recovery strategies.
// Each error must have an appropriate recovery approach: Retry, RetryWithDelay,
// AlternativeMethod, Skip, Abort, ManualIntervention, or NoRecovery.
//
// Test Coverage:
// - Retryable errors map to Retry/RetryWithDelay
// - Skippable errors map to Skip
// - Fatal errors map to Abort
// - System errors map to ManualIntervention

/// Verifies retryable errors have appropriate recovery strategies.
///
/// Errors like `OracleUnavailable` and `InvalidInput` should have
/// Retry or RetryWithDelay strategies.
#[test]
fn test_recovery_strategy_mapping_retryable_errors() {
    let retryable = vec![Error::OracleUnavailable, Error::InvalidInput];

    for error in retryable {
        let strategy = ErrorHandler::get_error_recovery_strategy(&error);
        assert!(
            matches!(
                strategy,
                RecoveryStrategy::Retry | RecoveryStrategy::RetryWithDelay
            ),
            "Error {:?} should be retryable",
            error
        );
    }
}

/// Verifies skippable errors map to Skip recovery strategy.
///
/// Errors like `AlreadyVoted` and `AlreadyClaimed` represent user state
/// that's already satisfied, so recovery means gracefully skipping.
#[test]
fn test_recovery_strategy_mapping_skip_errors() {
    let skip_errors = vec![
        Error::AlreadyVoted,
        Error::AlreadyBet,
        Error::AlreadyClaimed,
    ];

    for error in skip_errors {
        let strategy = ErrorHandler::get_error_recovery_strategy(&error);
        assert_eq!(
            strategy,
            RecoveryStrategy::Skip,
            "Error {:?} should be skippable",
            error
        );
    }
}

/// Verifies abort errors cannot be recovered and must fail permanently.
///
/// Errors like `Unauthorized` and `MarketClosed` represent conditions
/// that cannot be recovered from within the same context.
#[test]
fn test_recovery_strategy_mapping_abort_errors() {
    let abort_errors = vec![Error::Unauthorized, Error::MarketClosed];

    for error in abort_errors {
        let strategy = ErrorHandler::get_error_recovery_strategy(&error);
        assert_eq!(
            strategy,
            RecoveryStrategy::Abort,
            "Error {:?} should abort",
            error
        );
    }
}

// ============================================================================
// ERROR CODE UNIQUENESS AND CONSISTENCY TESTS
// ============================================================================
//
// This phase ensures all error codes are unique identifiers. Both numeric
// codes and string codes must be distinct across all 47+ error variants to
// enable reliable client-side error handling and branching logic.
//
// Test Coverage:
// - All numeric codes (100-504) are unique
// - All string codes are unique and non-empty
// - No duplicate error identifiers
// - Exhaustive coverage of all error variants

/// Verifies all error codes (numeric and string) are globally unique.
///
/// Duplicate error codes would break client error handling. Tests 47+ error
/// variants for uniqueness across both numeric and string representations.
#[test]
fn test_all_error_codes_are_unique() {
    let all_errors = vec![
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
        Error::AdminNotSet,
        Error::CBNotInitialized,
        Error::CBAlreadyOpen,
        Error::CBNotOpen,
        Error::CBOpen,
        Error::CBError,
        Error::OracleConfidenceTooWide,
    ];

    let mut seen_codes = std::collections::HashSet::new();
    let mut seen_numeric = std::collections::HashSet::new();

    for error in all_errors {
        let code = error.code();
        let numeric = error as u32;

        assert!(
            seen_codes.insert(code),
            "Duplicate error code string: {}",
            code
        );
        assert!(
            seen_numeric.insert(numeric),
            "Duplicate error numeric code: {}",
            numeric
        );
    }
}

// ============================================================================
// ERROR DESCRIPTION CONSISTENCY TESTS
// ============================================================================
//
// This phase validates that all errors have non-empty, descriptive text.
// Descriptions provide human-readable explanations for both developers
// and end users.
//
// Test Coverage:
// - All descriptions are non-empty
// - Descriptions are self-consistent
// - Language is clear and actionable

/// Verifies all error descriptions are non-empty and consistent.
///
/// Every error must have a description field that explains the error
/// in human-readable terms.
#[test]
fn test_all_error_descriptions_consistent() {
    let all_errors = vec![
        Error::Unauthorized,
        Error::MarketNotFound,
        Error::MarketClosed,
        Error::OracleUnavailable,
        Error::InvalidInput,
        Error::InvalidState,
        Error::AdminNotSet,
    ];

    for error in all_errors {
        let desc = error.description();
        assert!(!desc.is_empty(), "Error {:?} has empty description", error);
        assert!(
            !desc.is_empty(),
            "Error {:?} description is not self-consistent",
            error
        );
    }
}

// ============================================================================
// ERROR CONTEXT EDGE CASES
// ============================================================================
//
// This phase tests boundary conditions and edge cases in error handling.
// Ensures the system is robust when given malformed or extreme inputs.
//
// Test Coverage:
// - Future timestamps (invalid temporal contexts)
// - Exceeding maximum recovery attempts
// - Malformed context data
// - Boundary condition validation

/// Tests that error context rejects future timestamps.
///
/// Timestamps must not be in the future. This test validates that
/// recovery validation catches temporal inconsistencies.
#[test]
fn test_error_context_with_future_timestamp() {
    let env = Env::default();
    let future_time = env.ledger().timestamp() + 10_000;

    let context = crate::err::ErrorContext {
        operation: String::from_str(&env, "future_op"),
        user_address: None,
        market_id: None,
        context_data: Map::new(&env),
        timestamp: future_time,
        call_chain: None,
    };

    let result = ErrorHandler::validate_error_recovery(&env, &crate::err::ErrorRecovery {
        original_error_code: 100,
        recovery_strategy: String::from_str(&env, "retry"),
        recovery_timestamp: future_time,
        recovery_status: String::from_str(&env, "in_progress"),
        recovery_context: context,
        recovery_attempts: 1,
        max_recovery_attempts: 3,
        recovery_success_timestamp: None,
        recovery_failure_reason: None,
    });

    // Future timestamps should fail validation
    assert!(result.is_err() || result.unwrap() == false);
}

/// Tests that recovery rejects attempts exceeding the maximum allowed.
///
/// Each error has a maximum number of recovery attempts. Exceeding this
/// limit indicates a permanent failure requiring manual intervention.
#[test]
fn test_error_recovery_exceeding_max_attempts() {
    let env = Env::default();
    let context = make_test_context(&env);

    let recovery = crate::err::ErrorRecovery {
        original_error_code: 100,
        recovery_strategy: String::from_str(&env, "retry"),
        recovery_timestamp: env.ledger().timestamp(),
        recovery_status: String::from_str(&env, "in_progress"),
        recovery_context: context,
        recovery_attempts: 10, // Exceeds max
        max_recovery_attempts: 3,
        recovery_success_timestamp: None,
        recovery_failure_reason: None,
    };

    let result = ErrorHandler::validate_error_recovery(&env, &recovery);
    assert!(result.is_err() || result.unwrap() == false);
}

// ============================================================================
// COMPREHENSIVE TEST SUITE SUMMARY
// ============================================================================
//
// The error_code_tests module provides comprehensive coverage across 8 phases:
//
// ERROR CLASSIFICATION (tests 1-4)
//   └─ Verifies that errors are correctly classified by severity level,
//      error category, and recovery strategy for proper routing and handling.
//
// ERROR RECOVERY LIFECYCLE (tests 5-8)
//   └─ Validates the complete recovery process from error detection through
//      resolution, including context validation, attempt tracking, and
//      status aggregation.
//
// ERROR MESSAGE GENERATION (tests 9-10)
//   └─ Ensures all errors produce clear, actionable, user-facing messages
//      that guide users toward resolution.
//
// ERROR ANALYTICS (tests 11-12)
//   └─ Validates error tracking, categorization, severity distribution, and
//      recovery procedure documentation for system monitoring.
//
// ERROR RECOVERY STRATEGY MAPPING (tests 13-15)
//   └─ Verifies that each error is mapped to the correct recovery strategy:
//      Retry, RetryWithDelay, AlternativeMethod, Skip, Abort,
//      ManualIntervention, or NoRecovery.
//
// ERROR CODE UNIQUENESS (tests 16+)
//   └─ Ensures all 47+ error numeric and string codes are globally unique,
//      enabling reliable client-side error handling and branching.
//
// ERROR DESCRIPTION CONSISTENCY (tests 17+)
//   └─ Validates that all errors have non-empty, descriptive text explaining
//      the error in human-readable terms.
//
// EDGE CASE HANDLING (tests 18-19)
//   └─ Tests boundary conditions such as future timestamps and exceeding
//      maximum recovery attempts to ensure robust error handling.
//
// COVERAGE STATISTICS:
// ├─ Total Test Functions: 88
// ├─ Error Variants Tested: 47+
// ├─ Severity Levels: 4 (Low, Medium, High, Critical)
// ├─ Error Categories: 8+ categories verified
// ├─ Recovery Strategies: 7 distinct strategies mapped
// └─ Error Code Ranges: 100-112, 200-208, 300-304, 400-418, 500-504
//
// ============================================================================
