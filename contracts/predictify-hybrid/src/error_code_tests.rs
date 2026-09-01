#![cfg(test)]

use crate::err::{Error, ErrorCategory, ErrorSeverity, PublicErrorMapping, Recoverability};
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

fn all_errors() -> Vec<Error> {
    vec![
        Error::IdempotentBatchAlreadyApplied,
        Error::ReasonTableFull,
        Error::Overflow,
        Error::MaxBetCapExceeded,
        Error::InvalidCap,
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
        Error::InvalidInitializationParams,
        Error::DisputeError,
        Error::DisputerCannotVote,
        Error::SweepAlreadyDone,
        Error::FeeArithmeticOverflow,
        Error::FeeAlreadyCollected,
        Error::NoFeesToCollect,
        Error::InvalidExtensionDays,
        Error::ExtensionDenied,
        Error::GasBudgetExceeded,
        Error::AdminNotSet,
        Error::AssetDecimalsMismatch,
        Error::AdminActionTimelocked,
        Error::OperationWouldExceedBudget,
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
        Error::ArchiveFull,
        Error::CategoryTooShort,
        Error::TagTooShort,
        Error::DuplicateMarketId,
        Error::CannotArchiveFromState,
        Error::CannotRestoreFromState,
        Error::MarketAlreadyArchived,
        Error::MarketAlreadyRestored,
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
        Error::OracleQuoteOutlier,
        Error::MaxParticipantsReached,
        Error::BetExceedsCap,
        Error::ReplayedOverride,
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
        Error::BatchEmpty,
        Error::BatchSizeExceeded,
        Error::TreasuryUpdateTimelocked,
        Error::NoPendingTreasuryUpdate,
        Error::PendingTreasuryUpdateExists,
    ]
}

// =========================================================================
// 1. Acceptance Criteria: Existing codes remain stable & distinct
// =========================================================================

#[test]
fn test_all_variants_have_unique_contract_codes() {
    let mut seen = BTreeSet::new();
    for err in all_errors() {
        let code = err as u32;
        assert!(
            seen.insert(code),
            "Duplicate contract code {} found for error {:?}",
            code,
            err
        );
    }
}

#[test]
fn test_all_variants_have_unique_client_codes() {
    let mut seen = BTreeSet::new();
    for err in all_errors() {
        let client_code = err.client_code();
        assert_ne!(client_code, 0, "client_code for {:?} must not be 0", err);
        assert!(
            seen.insert(client_code),
            "Duplicate client_code {} found for error {:?}",
            client_code,
            err
        );
    }
}

#[test]
fn test_all_variants_have_unique_code_strings() {
    let mut seen = BTreeSet::new();
    for err in all_errors() {
        let s = err.code();
        assert!(!s.is_empty(), "code string for {:?} must not be empty", err);
        assert_ne!(s, "UNSPECIFIED_ERROR", "code string for {:?} must be specific", err);
        assert!(
            seen.insert(s),
            "Duplicate code string '{}' found for error {:?}",
            s,
            err
        );
    }
}

#[test]
fn test_contract_codes_match_documented_discriminants() {
    // Spot check core discriminants across all sections
    assert_eq!(Error::Unauthorized as u32, 100);
    assert_eq!(Error::MarketNotFound as u32, 101);
    assert_eq!(Error::NothingToClaim as u32, 105);
    assert_eq!(Error::AlreadyClaimed as u32, 106);
    assert_eq!(Error::OracleUnavailable as u32, 200);
    assert_eq!(Error::InvalidQuestion as u32, 300);
    assert_eq!(Error::InvalidState as u32, 400);
    assert_eq!(Error::CBNotInitialized as u32, 500);
    assert_eq!(Error::IdempotentBatchAlreadyApplied as u32, 660);
    assert_eq!(Error::InvalidInitializationParams as u32, 700);
}

// =========================================================================
// 2. Acceptance Criteria: New codes are unique and documented
// =========================================================================

#[test]
fn test_all_descriptions_are_non_empty_and_meaningful() {
    for err in all_errors() {
        let desc = err.description();
        assert!(
            !desc.is_empty(),
            "Error {:?} has an empty description",
            err
        );
        assert_ne!(
            desc,
            "An unspecified error occurred.",
            "Error {:?} has default unspecified description",
            err
        );
    }
}

#[test]
fn test_client_code_in_disjoint_range_for_category() {
    for err in all_errors() {
        let code = err.client_code();
        let cat = err.category();
        match cat {
            ErrorCategory::Oracle => {
                assert!(
                    (1000..=1099).contains(&code),
                    "Oracle error {:?} client_code {} not in 1000..=1099",
                    err,
                    code
                );
            }
            ErrorCategory::Market => {
                assert!(
                    (1100..=1199).contains(&code),
                    "Market error {:?} client_code {} not in 1100..=1199",
                    err,
                    code
                );
            }
            ErrorCategory::Validation => {
                assert!(
                    (1200..=1299).contains(&code) || (1900..=1999).contains(&code),
                    "Validation/Metadata error {:?} client_code {} not in valid range",
                    err,
                    code
                );
            }
            ErrorCategory::Financial => {
                assert!(
                    (1300..=1399).contains(&code),
                    "Financial error {:?} client_code {} not in 1300..=1399",
                    err,
                    code
                );
            }
            ErrorCategory::Dispute => {
                assert!(
                    (1400..=1499).contains(&code),
                    "Dispute error {:?} client_code {} not in 1400..=1499",
                    err,
                    code
                );
            }
            ErrorCategory::Authentication => {
                assert!(
                    (1500..=1599).contains(&code),
                    "Auth error {:?} client_code {} not in 1500..=1599",
                    err,
                    code
                );
            }
            ErrorCategory::System => {
                assert!(
                    (1600..=1799).contains(&code),
                    "System/CB error {:?} client_code {} not in 1600..=1799",
                    err,
                    code
                );
            }
            ErrorCategory::UserOperation => {
                assert!(
                    (1800..=1899).contains(&code),
                    "UserOp error {:?} client_code {} not in 1800..=1899",
                    err,
                    code
                );
            }
            ErrorCategory::Unknown => {
                panic!("Error {:?} returned ErrorCategory::Unknown", err);
            }
        }
    }
}

// =========================================================================
// 3. Acceptance Criteria: Unknown values decode safely
// =========================================================================

#[test]
fn test_decode_contract_code_known() {
    let mapping = Error::decode_contract_code(100);
    assert_eq!(mapping.contract_code, 100);
    assert_eq!(mapping.client_code, 1500);
    assert_eq!(mapping.code_str, "UNAUTHORIZED");
    assert_eq!(mapping.category, ErrorCategory::Authentication);
    assert!(mapping.is_known);
}

#[test]
fn test_decode_contract_code_unknown() {
    let unknown_codes = [0u32, 9999, 55555, u32::MAX];
    for code in unknown_codes {
        let mapping = Error::decode_contract_code(code);
        assert_eq!(mapping.contract_code, code);
        assert_eq!(mapping.client_code, 0);
        assert_eq!(mapping.code_str, "UNKNOWN_ERROR");
        assert_eq!(mapping.category, ErrorCategory::Unknown);
        assert_eq!(mapping.severity, ErrorSeverity::Medium);
        assert_eq!(mapping.recoverability, Recoverability::Terminal);
        assert!(!mapping.is_known);
    }
}

#[test]
fn test_decode_client_code_known() {
    let mapping = Error::decode_client_code(1000);
    assert_eq!(mapping.contract_code, 200);
    assert_eq!(mapping.client_code, 1000);
    assert_eq!(mapping.code_str, "ORACLE_UNAVAILABLE");
    assert_eq!(mapping.category, ErrorCategory::Oracle);
    assert_eq!(mapping.recoverability, Recoverability::Retryable);
    assert!(mapping.is_known);
}

#[test]
fn test_decode_client_code_unknown() {
    let unknown_client_codes = [0u32, 999, 9999, u32::MAX];
    for code in unknown_client_codes {
        let mapping = Error::decode_client_code(code);
        assert_eq!(mapping.contract_code, 0);
        assert_eq!(mapping.client_code, code);
        assert_eq!(mapping.code_str, "UNKNOWN_ERROR");
        assert_eq!(mapping.category, ErrorCategory::Unknown);
        assert_eq!(mapping.severity, ErrorSeverity::Medium);
        assert_eq!(mapping.recoverability, Recoverability::Terminal);
        assert!(!mapping.is_known);
    }
}

#[test]
fn test_from_code_str_roundtrip() {
    for err in all_errors() {
        let str_code = err.code();
        let resolved = Error::from_code_str(str_code);
        assert_eq!(
            resolved,
            Some(err),
            "Failed roundtrip for from_code_str('{}')",
            str_code
        );
    }
    assert_eq!(Error::from_code_str("NON_EXISTENT_ERROR"), None);
}

#[test]
fn test_from_contract_code_roundtrip() {
    for err in all_errors() {
        let code = err as u32;
        let resolved = Error::from_contract_code(code);
        assert_eq!(
            resolved,
            Some(err),
            "Failed roundtrip for from_contract_code({})",
            code
        );
    }
    assert_eq!(Error::from_contract_code(999999), None);
}

#[test]
fn test_from_client_code_roundtrip() {
    for err in all_errors() {
        let code = err.client_code();
        let resolved = Error::from_client_code(code);
        assert_eq!(
            resolved,
            Some(err),
            "Failed roundtrip for from_client_code({})",
            code
        );
    }
    assert_eq!(Error::from_client_code(999999), None);
}

// =========================================================================
// 4. Acceptance Criteria: Golden vectors cover public entrypoints
// =========================================================================

struct GoldenVector {
    error: Error,
    contract_code: u32,
    client_code: u32,
    code_str: &'static str,
    category: ErrorCategory,
    severity: ErrorSeverity,
    recoverability: Recoverability,
}

#[test]
fn test_golden_vectors() {
    let golden_vectors: &[GoldenVector] = &[
        // Core Entrypoint: Authentication
        GoldenVector {
            error: Error::Unauthorized,
            contract_code: 100,
            client_code: 1500,
            code_str: "UNAUTHORIZED",
            category: ErrorCategory::Authentication,
            severity: ErrorSeverity::High,
            recoverability: Recoverability::Terminal,
        },
        // Core Entrypoint: Market Lifecycle
        GoldenVector {
            error: Error::MarketNotFound,
            contract_code: 101,
            client_code: 1100,
            code_str: "MARKET_NOT_FOUND",
            category: ErrorCategory::Market,
            severity: ErrorSeverity::Medium,
            recoverability: Recoverability::Terminal,
        },
        GoldenVector {
            error: Error::MarketClosed,
            contract_code: 102,
            client_code: 1101,
            code_str: "MARKET_CLOSED",
            category: ErrorCategory::Market,
            severity: ErrorSeverity::Medium,
            recoverability: Recoverability::Terminal,
        },
        GoldenVector {
            error: Error::MarketResolved,
            contract_code: 103,
            client_code: 1102,
            code_str: "MARKET_ALREADY_RESOLVED",
            category: ErrorCategory::Market,
            severity: ErrorSeverity::Medium,
            recoverability: Recoverability::Terminal,
        },
        // Core Entrypoint: Claims / Winnings
        GoldenVector {
            error: Error::NothingToClaim,
            contract_code: 105,
            client_code: 1302,
            code_str: "NOTHING_TO_CLAIM",
            category: ErrorCategory::Financial,
            severity: ErrorSeverity::Low,
            recoverability: Recoverability::Terminal,
        },
        GoldenVector {
            error: Error::AlreadyClaimed,
            contract_code: 106,
            client_code: 1303,
            code_str: "ALREADY_CLAIMED",
            category: ErrorCategory::Financial,
            severity: ErrorSeverity::Low,
            recoverability: Recoverability::Terminal,
        },
        // Core Entrypoint: Betting / Staking
        GoldenVector {
            error: Error::InsufficientStake,
            contract_code: 107,
            client_code: 1300,
            code_str: "INSUFFICIENT_STAKE",
            category: ErrorCategory::Financial,
            severity: ErrorSeverity::Medium,
            recoverability: Recoverability::Retryable,
        },
        GoldenVector {
            error: Error::AlreadyBet,
            contract_code: 110,
            client_code: 1801,
            code_str: "ALREADY_BET",
            category: ErrorCategory::UserOperation,
            severity: ErrorSeverity::Low,
            recoverability: Recoverability::Terminal,
        },
        // Core Entrypoint: Oracle Integration
        GoldenVector {
            error: Error::OracleUnavailable,
            contract_code: 200,
            client_code: 1000,
            code_str: "ORACLE_UNAVAILABLE",
            category: ErrorCategory::Oracle,
            severity: ErrorSeverity::High,
            recoverability: Recoverability::Retryable,
        },
        GoldenVector {
            error: Error::OracleStale,
            contract_code: 202,
            client_code: 1002,
            code_str: "ORACLE_STALE",
            category: ErrorCategory::Oracle,
            severity: ErrorSeverity::Medium,
            recoverability: Recoverability::Retryable,
        },
        // Core Entrypoint: Validation
        GoldenVector {
            error: Error::InvalidQuestion,
            contract_code: 300,
            client_code: 1200,
            code_str: "INVALID_QUESTION",
            category: ErrorCategory::Validation,
            severity: ErrorSeverity::Medium,
            recoverability: Recoverability::Retryable,
        },
        // Core Entrypoint: Circuit Breaker
        GoldenVector {
            error: Error::CBOpen,
            contract_code: 503,
            client_code: 1603,
            code_str: "CIRCUIT_BREAKER_OPEN",
            category: ErrorCategory::System,
            severity: ErrorSeverity::High,
            recoverability: Recoverability::Retryable,
        },
        GoldenVector {
            error: Error::RateLimitExceeded,
            contract_code: 505,
            client_code: 1605,
            code_str: "RATE_LIMIT_EXCEEDED",
            category: ErrorCategory::System,
            severity: ErrorSeverity::Medium,
            recoverability: Recoverability::Retryable,
        },
        // Core Entrypoint: Dispute
        GoldenVector {
            error: Error::AlreadyDisputed,
            contract_code: 404,
            client_code: 1400,
            code_str: "ALREADY_DISPUTED",
            category: ErrorCategory::Dispute,
            severity: ErrorSeverity::Medium,
            recoverability: Recoverability::Terminal,
        },
    ];

    for gv in golden_vectors {
        let mapping = gv.error.public_mapping();
        assert_eq!(mapping.contract_code, gv.contract_code, "contract_code mismatch for {:?}", gv.error);
        assert_eq!(mapping.client_code, gv.client_code, "client_code mismatch for {:?}", gv.error);
        assert_eq!(mapping.code_str, gv.code_str, "code_str mismatch for {:?}", gv.error);
        assert_eq!(mapping.category, gv.category, "category mismatch for {:?}", gv.error);
        assert_eq!(mapping.severity, gv.severity, "severity mismatch for {:?}", gv.error);
        assert_eq!(mapping.recoverability, gv.recoverability, "recoverability mismatch for {:?}", gv.error);
        assert!(mapping.is_known);

        // Also verify decode functions produce exact same golden mapping
        let from_contract = Error::decode_contract_code(gv.contract_code);
        assert_eq!(from_contract, mapping);

        let from_client = Error::decode_client_code(gv.client_code);
        assert_eq!(from_client, mapping);
    }
}
