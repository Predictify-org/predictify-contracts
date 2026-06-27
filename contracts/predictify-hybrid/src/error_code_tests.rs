//! Tests for the error taxonomy refinement (#602).
//!
//! Acceptance criteria enforced here:
//! - No duplicate `client_code()` values across all variants.
//! - Every variant has a `recoverability()` label (enum is exhaustive).
//! - `client_code()` falls within the disjoint range reserved for its category.
//! - `client_code()` is exposed as a public method (off-chain mapping).
//! - No `unwrap()` in the mapping table (method returns `u32`, not `Option`).

#![cfg(test)]

use alloc::vec;
use alloc::vec::Vec as StdVec;

use crate::err::{
    Error, ErrorCategory, ErrorHandler, ErrorSeverity, Recoverability, RecoveryStrategy,
};
use soroban_sdk::{Env, Map, String};

// ─── helpers ────────────────────────────────────────────────────────────────

fn make_ctx(env: &Env) -> crate::err::ErrorContext {
    crate::err::ErrorContext {
        operation: String::from_str(env, "test_op"),
        user_address: None,
        market_id: None,
        context_data: Map::new(env),
        timestamp: env.ledger().timestamp(),
        call_chain: None,
    }
}

/// Every concrete `Error` variant — used for exhaustive property checks.
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
        Error::ArchiveFull,
        Error::CategoryTooShort,
        Error::TagTooShort,
        Error::CBNotInitialized,
        Error::CBAlreadyOpen,
        Error::CBNotOpen,
        Error::CBOpen,
        Error::CBError,
        Error::RateLimitExceeded,
    ]
}

/// O(n²) uniqueness check — sufficient for ~75 variants, avoids HashSet (no_std).
fn all_unique_u32(values: &[u32]) -> bool {
    for i in 0..values.len() {
        for j in (i + 1)..values.len() {
            if values[i] == values[j] {
                return false;
            }
        }
    }
    true
}

fn all_unique_str(values: &[&'static str]) -> bool {
    for i in 0..values.len() {
        for j in (i + 1)..values.len() {
            if values[i] == values[j] {
                return false;
            }
        }
    }
    true
}

// ═══════════════════════════════════════════════════════════════════════════
// AC1 – No duplicate client_code() values
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_client_codes_are_unique() {
    let codes: StdVec<u32> = all_errors().iter().map(|e| e.client_code()).collect();
    assert!(
        all_unique_u32(&codes),
        "Duplicate client_code detected"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// AC2 – Every variant has a Recoverability label
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_every_variant_has_recoverability() {
    for err in all_errors() {
        let r = err.recoverability();
        assert!(
            matches!(
                r,
                Recoverability::Retryable
                    | Recoverability::RequiresAdmin
                    | Recoverability::Terminal
            ),
            "{:?} returned an unexpected Recoverability value",
            err
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// AC3 – client_code() lies in the disjoint range for its ErrorCategory
// ═══════════════════════════════════════════════════════════════════════════

/// Returns the expected (lo, hi) client-code range for an ErrorCategory.
/// Errors in the catch-all `Unknown` category accept the full 1000-1999 span.
fn expected_range(cat: &ErrorCategory) -> (u32, u32) {
    match cat {
        ErrorCategory::Oracle => (1000, 1099),
        ErrorCategory::Market => (1100, 1199),
        ErrorCategory::Validation => (1200, 1299),
        ErrorCategory::Financial => (1300, 1399),
        ErrorCategory::Dispute => (1400, 1499),
        ErrorCategory::Authentication => (1500, 1599),
        // System encompasses both pure system (1700-1799) and CB (1600-1699).
        ErrorCategory::System => (1600, 1799),
        ErrorCategory::UserOperation => (1800, 1899),
        _ => (1000, 1999), // Unknown / Metadata – wide acceptance
    }
}

#[test]
fn test_client_code_in_disjoint_range_for_category() {
    for err in all_errors() {
        let (_, cat, _) = ErrorHandler::get_error_classification(&err);
        let (lo, hi) = expected_range(&cat);
        let code = err.client_code();
        assert!(
            code >= lo && code <= hi,
            "{:?}: client_code {} not in [{}, {}] (category {:?})",
            err, code, lo, hi, cat
        );
    }
}

#[test]
fn test_category_ranges_are_disjoint() {
    let ranges: &[(&str, u32, u32)] = &[
        ("Oracle", 1000, 1099),
        ("Market", 1100, 1199),
        ("Validation", 1200, 1299),
        ("Financial", 1300, 1399),
        ("Dispute", 1400, 1499),
        ("Auth", 1500, 1599),
        ("CircuitBreaker", 1600, 1699),
        ("System", 1700, 1799),
        ("UserOperation", 1800, 1899),
        ("Metadata", 1900, 1999),
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

// ─── Spot-check canonical codes per category ────────────────────────────────

#[test]
fn test_oracle_client_codes_in_1000_range() {
    for err in [
        Error::OracleUnavailable,
        Error::InvalidOracleConfig,
        Error::OracleStale,
        Error::FallbackOracleUnavailable,
        Error::OracleConfidenceTooWide,
    ] {
        let c = err.client_code();
        assert!(c >= 1000 && c <= 1099, "{:?} -> {}", err, c);
    }
}

#[test]
fn test_market_client_codes_in_1100_range() {
    for err in [
        Error::MarketNotFound,
        Error::MarketClosed,
        Error::MarketResolved,
        Error::MarketNotReady,
    ] {
        let c = err.client_code();
        assert!(c >= 1100 && c <= 1199, "{:?} -> {}", err, c);
    }
}

#[test]
fn test_validation_client_codes_in_1200_range() {
    for err in [
        Error::InvalidQuestion,
        Error::InvalidInput,
        Error::InvalidDuration,
    ] {
        let c = err.client_code();
        assert!(c >= 1200 && c <= 1299, "{:?} -> {}", err, c);
    }
}

#[test]
fn test_financial_client_codes_in_1300_range() {
    for err in [
        Error::FeeArithmeticOverflow,
        Error::FeeAlreadyCollected,
        Error::NoFeesToCollect,
        Error::InvalidFeeConfig,
        Error::SweepAlreadyDone,
        Error::DisputeFeeFailed,
    ] {
        let c = err.client_code();
        assert!(c >= 1300 && c <= 1399, "{:?} -> {}", err, c);
    }
}

#[test]
fn test_dispute_client_codes_in_1400_range() {
    for err in [
        Error::AlreadyDisputed,
        Error::DisputeVoteExpired,
        Error::DisputeCondNotMet,
        Error::DisputeError,
    ] {
        let c = err.client_code();
        assert!(c >= 1400 && c <= 1499, "{:?} -> {}", err, c);
    }
}

#[test]
fn test_auth_client_code_in_1500_range() {
    let c = Error::Unauthorized.client_code();
    assert!(c >= 1500 && c <= 1599, "Unauthorized -> {}", c);
}

#[test]
fn test_circuit_breaker_client_codes_in_1600_range() {
    for err in [
        Error::CBNotInitialized,
        Error::CBAlreadyOpen,
        Error::CBNotOpen,
        Error::CBOpen,
        Error::CBError,
        Error::RateLimitExceeded,
    ] {
        let c = err.client_code();
        assert!(c >= 1600 && c <= 1699, "{:?} -> {}", err, c);
    }
}

#[test]
fn test_system_client_codes_in_1700_range() {
    for err in [
        Error::InvalidState,
        Error::ConfigNotFound,
        Error::AdminNotSet,
        Error::GasBudgetExceeded,
    ] {
        let c = err.client_code();
        assert!(c >= 1700 && c <= 1799, "{:?} -> {}", err, c);
    }
}

#[test]
fn test_user_operation_client_codes_in_1800_range() {
    for err in [
        Error::AlreadyVoted,
        Error::AlreadyBet,
        Error::BetsAlreadyPlaced,
        Error::InvalidOutcome,
        Error::InsufficientStake,
        Error::InsufficientBalance,
        Error::NothingToClaim,
        Error::AlreadyClaimed,
    ] {
        let c = err.client_code();
        assert!(c >= 1800 && c <= 1899, "{:?} -> {}", err, c);
    }
}

#[test]
fn test_metadata_client_codes_in_1900_range() {
    for err in [
        Error::QuestionTooLong,
        Error::OutcomeTooLong,
        Error::TooManyOutcomes,
        Error::ArchiveFull,
        Error::CategoryTooShort,
        Error::TagTooShort,
    ] {
        let c = err.client_code();
        assert!(c >= 1900 && c <= 1999, "{:?} -> {}", err, c);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Recoverability spot-checks
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_retryable_errors() {
    for err in [
        Error::OracleUnavailable,
        Error::OracleStale,
        Error::FallbackOracleUnavailable,
        Error::OracleCallbackTimeout,
        Error::ResolutionTimeoutReached,
        Error::RateLimitExceeded,
        Error::InvalidInput,
        Error::InsufficientStake,
        Error::InsufficientBalance,
    ] {
        assert_eq!(
            err.recoverability(),
            Recoverability::Retryable,
            "{:?} should be Retryable",
            err
        );
    }
}

#[test]
fn test_requires_admin_errors() {
    for err in [
        Error::AdminNotSet,
        Error::DisputeFeeFailed,
        Error::CBNotInitialized,
        Error::InvalidOracleConfig,
        Error::InvalidFeeConfig,
        Error::ConfigNotFound,
        Error::CBOpen,
    ] {
        assert_eq!(
            err.recoverability(),
            Recoverability::RequiresAdmin,
            "{:?} should be RequiresAdmin",
            err
        );
    }
}

#[test]
fn test_terminal_errors() {
    for err in [
        Error::Unauthorized,
        Error::MarketClosed,
        Error::MarketResolved,
        Error::AlreadyVoted,
        Error::AlreadyBet,
        Error::AlreadyClaimed,
        Error::FeeAlreadyCollected,
        Error::InvalidState,
    ] {
        assert_eq!(
            err.recoverability(),
            Recoverability::Terminal,
            "{:?} should be Terminal",
            err
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Off-chain client branching
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_client_can_branch_on_client_code_category() {
    fn domain(code: u32) -> &'static str {
        match code {
            1000..=1099 => "oracle",
            1100..=1199 => "market",
            1200..=1299 => "validation",
            1300..=1399 => "financial",
            1400..=1499 => "dispute",
            1500..=1599 => "auth",
            1600..=1699 => "circuit_breaker",
            1700..=1799 => "system",
            1800..=1899 => "user_operation",
            1900..=1999 => "metadata",
            _ => "unknown",
        }
    }

    assert_eq!(domain(Error::OracleUnavailable.client_code()), "oracle");
    assert_eq!(domain(Error::MarketClosed.client_code()), "market");
    assert_eq!(domain(Error::InvalidInput.client_code()), "validation");
    assert_eq!(domain(Error::FeeAlreadyCollected.client_code()), "financial");
    assert_eq!(domain(Error::DisputeError.client_code()), "dispute");
    assert_eq!(domain(Error::Unauthorized.client_code()), "auth");
    assert_eq!(domain(Error::CBOpen.client_code()), "circuit_breaker");
    assert_eq!(domain(Error::AdminNotSet.client_code()), "system");
    assert_eq!(domain(Error::AlreadyVoted.client_code()), "user_operation");
    assert_eq!(domain(Error::QuestionTooLong.client_code()), "metadata");
}

#[test]
fn test_client_retry_decision_from_recoverability() {
    fn should_retry(err: Error) -> bool {
        err.recoverability() == Recoverability::Retryable
    }
    assert!(should_retry(Error::OracleUnavailable));
    assert!(should_retry(Error::RateLimitExceeded));
    assert!(!should_retry(Error::Unauthorized));
    assert!(!should_retry(Error::AdminNotSet));
    assert!(!should_retry(Error::CBNotInitialized));
}

// ═══════════════════════════════════════════════════════════════════════════
// Existing taxonomy: .code() strings, .description(), numeric variant codes
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_error_code_strings_are_unique() {
    let codes: StdVec<&'static str> = all_errors().iter().map(|e| e.code()).collect();
    assert!(all_unique_str(&codes), "Duplicate .code() string detected");
}

#[test]
fn test_all_code_strings_are_upper_snake_case() {
    for err in all_errors() {
        let code = err.code();
        assert!(!code.is_empty(), "{:?}.code() is empty", err);
        for c in code.chars() {
            assert!(
                c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_',
                "{:?}.code() has invalid char '{}' in \"{}\"",
                err, c, code
            );
        }
        assert!(!code.starts_with('_'), "{:?}.code() starts with _", err);
        assert!(!code.ends_with('_'), "{:?}.code() ends with _", err);
    }
}

#[test]
fn test_all_descriptions_are_non_empty() {
    for err in all_errors() {
        assert!(!err.description().is_empty(), "{:?}.description() is empty", err);
    }
}

#[test]
fn test_all_numeric_variant_codes_are_unique() {
    let codes: StdVec<u32> = all_errors().iter().map(|e| *e as u32).collect();
    assert!(all_unique_u32(&codes), "Duplicate variant numeric code detected");
}

// ═══════════════════════════════════════════════════════════════════════════
// ErrorHandler classification spot-checks
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_classification_critical_admin_not_set() {
    let env = Env::default();
    let d = ErrorHandler::categorize_error(&env, Error::AdminNotSet, make_ctx(&env));
    assert_eq!(d.severity, ErrorSeverity::Critical);
    assert_eq!(d.category, ErrorCategory::System);
    assert_eq!(d.recovery_strategy, RecoveryStrategy::ManualIntervention);
}

#[test]
fn test_classification_high_unauthorized() {
    let env = Env::default();
    let d = ErrorHandler::categorize_error(&env, Error::Unauthorized, make_ctx(&env));
    assert_eq!(d.severity, ErrorSeverity::High);
    assert_eq!(d.category, ErrorCategory::Authentication);
    assert_eq!(d.recovery_strategy, RecoveryStrategy::Abort);
}

#[test]
fn test_classification_high_oracle_unavailable() {
    let env = Env::default();
    let d = ErrorHandler::categorize_error(&env, Error::OracleUnavailable, make_ctx(&env));
    assert_eq!(d.severity, ErrorSeverity::High);
    assert_eq!(d.category, ErrorCategory::Oracle);
    assert_eq!(d.recovery_strategy, RecoveryStrategy::RetryWithDelay);
}

#[test]
fn test_classification_medium_market_not_found() {
    let env = Env::default();
    let d = ErrorHandler::categorize_error(&env, Error::MarketNotFound, make_ctx(&env));
    assert_eq!(d.severity, ErrorSeverity::Medium);
    assert_eq!(d.category, ErrorCategory::Market);
    assert_eq!(d.recovery_strategy, RecoveryStrategy::AlternativeMethod);
}

#[test]
fn test_classification_low_already_voted() {
    let env = Env::default();
    let d = ErrorHandler::categorize_error(&env, Error::AlreadyVoted, make_ctx(&env));
    assert_eq!(d.severity, ErrorSeverity::Low);
    assert_eq!(d.category, ErrorCategory::UserOperation);
    assert_eq!(d.recovery_strategy, RecoveryStrategy::Skip);
}

// ═══════════════════════════════════════════════════════════════════════════
// Recovery strategy spot-checks
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_recovery_strategy_oracle_unavailable_is_retry_with_delay() {
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::OracleUnavailable),
        RecoveryStrategy::RetryWithDelay
    );
}

#[test]
fn test_recovery_strategy_already_voted_is_skip() {
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::AlreadyVoted),
        RecoveryStrategy::Skip
    );
}

#[test]
fn test_recovery_strategy_unauthorized_is_abort() {
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::Unauthorized),
        RecoveryStrategy::Abort
    );
}

#[test]
fn test_recovery_strategy_admin_not_set_is_manual_intervention() {
    assert_eq!(
        ErrorHandler::get_error_recovery_strategy(&Error::AdminNotSet),
        RecoveryStrategy::ManualIntervention
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Analytics and context validation
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_error_analytics_initial_state() {
    let env = Env::default();
    let a = ErrorHandler::get_error_analytics(&env).unwrap();
    assert_eq!(a.total_errors, 0);
    assert_eq!(a.recovery_success_rate, 0);
    assert!(a.errors_by_category.get(ErrorCategory::UserOperation).is_some());
    assert!(a.errors_by_severity.get(ErrorSeverity::Low).is_some());
}

#[test]
fn test_error_recovery_status_initial() {
    let env = Env::default();
    let s = ErrorHandler::get_error_recovery_status(&env).unwrap();
    assert_eq!(s.total_attempts, 0);
    assert!(s.last_recovery_timestamp.is_none());
}

#[test]
fn test_context_valid() {
    let env = Env::default();
    assert!(ErrorHandler::validate_error_context(&make_ctx(&env)).is_ok());
}

#[test]
fn test_context_empty_operation_fails() {
    let env = Env::default();
    let mut ctx = make_ctx(&env);
    ctx.operation = String::from_str(&env, "");
    assert!(ErrorHandler::validate_error_context(&ctx).is_err());
}
