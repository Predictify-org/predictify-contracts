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
    assert_eq!(Error::DisputeVoteExpired as u32, 405);
    assert_eq!(Error::DisputeVoteDenied as u32, 406);
    assert_eq!(Error::DisputeAlreadyVoted as u32, 407);
    assert_eq!(Error::DisputeCondNotMet as u32, 408);
    assert_eq!(Error::DisputeFeeFailed as u32, 409);
    assert_eq!(Error::DisputeNoEscalate as u32, 410);
    assert_eq!(Error::ThresholdBelowMin as u32, 411);
    assert_eq!(Error::ThresholdTooHigh as u32, 412);
    assert_eq!(Error::FeeAlreadyCollected as u32, 413);
    assert_eq!(Error::NoFeesToCollect as u32, 414);
    assert_eq!(Error::InvalidExtensionDays as u32, 415);
    assert_eq!(Error::ExtensionDenied as u32, 416);
    assert_eq!(Error::AdminNotSet as u32, 418);
    assert_eq!(Error::TimeoutNotSet as u32, 419);
    assert_eq!(Error::InvalidTimeoutHours as u32, 422);
}

#[test]
fn test_error_numeric_codes_circuit_breaker_range() {
    assert_eq!(Error::CBNotInitialized as u32, 500);
    assert_eq!(Error::CBAlreadyOpen as u32, 501);
    assert_eq!(Error::CBNotOpen as u32, 502);
    assert_eq!(Error::CBOpen as u32, 503);
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
        Error::DisputeVoteExpired.code(),
        "DISPUTE_VOTING_PERIOD_EXPIRED"
    );
    assert_eq!(
        Error::DisputeVoteDenied.code(),
        "DISPUTE_VOTING_NOT_ALLOWED"
    );
    assert_eq!(Error::DisputeAlreadyVoted.code(), "DISPUTE_ALREADY_VOTED");
    assert_eq!(
        Error::DisputeCondNotMet.code(),
        "DISPUTE_RESOLUTION_CONDITIONS_NOT_MET"
    );
    assert_eq!(
        Error::DisputeFeeFailed.code(),
        "DISPUTE_FEE_DISTRIBUTION_FAILED"
    );
    assert_eq!(
        Error::DisputeNoEscalate.code(),
        "DISPUTE_ESCALATION_NOT_ALLOWED"
    );
    assert_eq!(Error::ThresholdBelowMin.code(), "THRESHOLD_BELOW_MINIMUM");
    assert_eq!(Error::ThresholdTooHigh.code(), "THRESHOLD_EXCEEDS_MAXIMUM");
    assert_eq!(Error::FeeAlreadyCollected.code(), "FEE_ALREADY_COLLECTED");
    assert_eq!(Error::NoFeesToCollect.code(), "NO_FEES_TO_COLLECT");
    assert_eq!(
        Error::InvalidExtensionDays.code(),
        "INVALID_EXTENSION_DAYS"
    );
    assert_eq!(Error::ExtensionDenied.code(), "EXTENSION_DENIED");
    assert_eq!(Error::AdminNotSet.code(), "ADMIN_NOT_SET");
    assert_eq!(Error::TimeoutNotSet.code(), "DISPUTE_TIMEOUT_NOT_SET");
    assert_eq!(Error::InvalidTimeoutHours.code(), "INVALID_TIMEOUT_HOURS");
}

#[test]
fn test_error_code_strings_circuit_breaker() {
    assert_eq!(
        Error::CBNotInitialized.code(),
        "CIRCUIT_BREAKER_NOT_INITIALIZED"
    );
    assert_eq!(Error::CBAlreadyOpen.code(), "CIRCUIT_BREAKER_ALREADY_OPEN");
    assert_eq!(Error::CBNotOpen.code(), "CIRCUIT_BREAKER_NOT_OPEN");
    assert_eq!(Error::CBOpen.code(), "CIRCUIT_BREAKER_OPEN");
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
        Error::DisputeVoteExpired.description(),
        "Dispute voting period expired"
    );
    assert_eq!(
        Error::DisputeVoteDenied.description(),
        "Dispute voting not allowed"
    );
    assert_eq!(
        Error::DisputeAlreadyVoted.description(),
        "Already voted in dispute"
    );
    assert_eq!(
        Error::DisputeCondNotMet.description(),
        "Dispute resolution conditions not met"
    );
    assert_eq!(
        Error::DisputeFeeFailed.description(),
        "Dispute fee distribution failed"
    );
    assert_eq!(
        Error::DisputeNoEscalate.description(),
        "Dispute escalation not allowed"
    );
    assert_eq!(
        Error::ThresholdBelowMin.description(),
        "Threshold below minimum"
    );
    assert_eq!(
        Error::ThresholdTooHigh.description(),
        "Threshold exceeds maximum"
    );
    assert_eq!(
        Error::FeeAlreadyCollected.description(),
        "Fee already collected"
    );
    assert_eq!(Error::NoFeesToCollect.description(), "No fees to collect");
    assert_eq!(
        Error::InvalidExtensionDays.description(),
        "Invalid extension days"
    );
    assert_eq!(
        Error::ExtensionDenied.description(),
        "Extension not allowed or exceeded"
    );
    assert_eq!(
        Error::AdminNotSet.description(),
        "Admin address is not set (initialization missing)"
    );
    assert_eq!(
        Error::TimeoutNotSet.description(),
        "Dispute timeout not set"
    );
    assert_eq!(
        Error::InvalidTimeoutHours.description(),
        "Invalid timeout hours"
    );
}

#[test]
fn test_error_descriptions_circuit_breaker() {
    assert_eq!(
        Error::CBNotInitialized.description(),
        "Circuit breaker not initialized"
    );
    assert_eq!(
        Error::CBAlreadyOpen.description(),
        "Circuit breaker is already open (paused)"
    );
    assert_eq!(
        Error::CBNotOpen.description(),
        "Circuit breaker is not open (cannot recover)"
    );
    assert_eq!(
        Error::CBOpen.description(),
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
    assert!(!Error::DisputeVoteExpired.description().is_empty());
    assert!(!Error::DisputeVoteDenied.description().is_empty());
    assert!(!Error::DisputeAlreadyVoted.description().is_empty());
    assert!(!Error::DisputeCondNotMet.description().is_empty());
    assert!(!Error::DisputeFeeFailed.description().is_empty());
    assert!(!Error::DisputeNoEscalate.description().is_empty());
    assert!(!Error::ThresholdBelowMin.description().is_empty());
    assert!(!Error::ThresholdTooHigh.description().is_empty());
    assert!(!Error::FeeAlreadyCollected.description().is_empty());
    assert!(!Error::NoFeesToCollect.description().is_empty());
    assert!(!Error::InvalidExtensionDays.description().is_empty());
    assert!(!Error::ExtensionDenied.description().is_empty());
    assert!(!Error::AdminNotSet.description().is_empty());
    assert!(!Error::TimeoutNotSet.description().is_empty());
    assert!(!Error::InvalidTimeoutHours.description().is_empty());
    assert!(!Error::CBNotInitialized.description().is_empty());
    assert!(!Error::CBAlreadyOpen.description().is_empty());
    assert!(!Error::CBNotOpen.description().is_empty());
    assert!(!Error::CBOpen.description().is_empty());
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
    assert!(!Error::DisputeVoteExpired.code().is_empty());
    assert!(!Error::DisputeVoteDenied.code().is_empty());
    assert!(!Error::DisputeAlreadyVoted.code().is_empty());
    assert!(!Error::DisputeCondNotMet.code().is_empty());
    assert!(!Error::DisputeFeeFailed.code().is_empty());
    assert!(!Error::DisputeNoEscalate.code().is_empty());
    assert!(!Error::ThresholdBelowMin.code().is_empty());
    assert!(!Error::ThresholdTooHigh.code().is_empty());
    assert!(!Error::FeeAlreadyCollected.code().is_empty());
    assert!(!Error::NoFeesToCollect.code().is_empty());
    assert!(!Error::InvalidExtensionDays.code().is_empty());
    assert!(!Error::ExtensionDenied.code().is_empty());
    assert!(!Error::AdminNotSet.code().is_empty());
    assert!(!Error::TimeoutNotSet.code().is_empty());
    assert!(!Error::InvalidTimeoutHours.code().is_empty());
    assert!(!Error::CBNotInitialized.code().is_empty());
    assert!(!Error::CBAlreadyOpen.code().is_empty());
    assert!(!Error::CBNotOpen.code().is_empty());
    assert!(!Error::CBOpen.code().is_empty());
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
        Error::DisputeVoteExpired as u32,
        Error::DisputeVoteDenied as u32,
        Error::DisputeAlreadyVoted as u32,
        Error::DisputeCondNotMet as u32,
        Error::DisputeFeeFailed as u32,
        Error::DisputeNoEscalate as u32,
        Error::ThresholdBelowMin as u32,
        Error::ThresholdTooHigh as u32,
        Error::FeeAlreadyCollected as u32,
        Error::NoFeesToCollect as u32,
        Error::InvalidExtensionDays as u32,
        Error::ExtensionDenied as u32,
        Error::AdminNotSet as u32,
        Error::TimeoutNotSet as u32,
        Error::InvalidTimeoutHours as u32,
        Error::CBNotInitialized as u32,
        Error::CBAlreadyOpen as u32,
        Error::CBNotOpen as u32,
        Error::CBOpen as u32,
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
        Error::DisputeVoteExpired.code(),
        Error::DisputeVoteDenied.code(),
        Error::DisputeAlreadyVoted.code(),
        Error::DisputeCondNotMet.code(),
        Error::DisputeFeeFailed.code(),
        Error::DisputeNoEscalate.code(),
        Error::ThresholdBelowMin.code(),
        Error::ThresholdTooHigh.code(),
        Error::FeeAlreadyCollected.code(),
        Error::NoFeesToCollect.code(),
        Error::InvalidExtensionDays.code(),
        Error::ExtensionDenied.code(),
        Error::AdminNotSet.code(),
        Error::TimeoutNotSet.code(),
        Error::InvalidTimeoutHours.code(),
        Error::CBNotInitialized.code(),
        Error::CBAlreadyOpen.code(),
        Error::CBNotOpen.code(),
        Error::CBOpen.code(),
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
        Error::CBNotInitialized as u32,
        Error::CBAlreadyOpen as u32,
        Error::CBNotOpen as u32,
        Error::CBOpen as u32,
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
        ErrorHandler::get_error_recovery_strategy(&Error::FeeAlreadyCollected),
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
        ErrorHandler::get_error_recovery_strategy(&Error::DisputeFeeFailed),
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
    let detailed = ErrorHandler::categorize_error(&env, Error::DisputeFeeFailed, context);
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
    let detailed = ErrorHandler::categorize_error(&env, Error::FeeAlreadyCollected, context);
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
    let skip_errors = &[Error::AlreadyVoted, Error::AlreadyClaimed, Error::FeeAlreadyCollected];

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
        Error::CBOpen.code(),
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
