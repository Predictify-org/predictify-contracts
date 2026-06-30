#![cfg(test)]
extern crate std;

use alloc::vec;
use crate::err::Error;

/// This snapshot test asserts a frozen mapping of variant names to numeric codes.
/// `err.rs` defines many error variants whose numeric codes are part of the client-facing API.
/// If you are adding a new variant, append it to this table explicitly.
/// If you remove an existing variant, remove it from this table.
/// This prevents accidental reordering or insertion in the middle of the enum
/// which would cause silent breaking changes to clients.
#[test]
fn test_error_code_stability() {
    let expected_codes = vec![
        (Error::Unauthorized, 100),
        (Error::MarketNotFound, 101),
        (Error::MarketClosed, 102),
        (Error::MarketResolved, 103),
        (Error::MarketNotResolved, 104),
        (Error::NothingToClaim, 105),
        (Error::AlreadyClaimed, 106),
        (Error::InsufficientStake, 107),
        (Error::InvalidOutcome, 108),
        (Error::AlreadyVoted, 109),
        (Error::AlreadyBet, 110),
        (Error::BetsAlreadyPlaced, 111),
        (Error::InsufficientBalance, 112),

        (Error::OracleUnavailable, 200),
        (Error::InvalidOracleConfig, 201),
        (Error::OracleStale, 202),
        (Error::OracleNoConsensus, 203),
        (Error::OracleVerified, 204),
        (Error::MarketNotReady, 205),
        (Error::FallbackOracleUnavailable, 206),
        (Error::ResolutionTimeoutReached, 207),
        (Error::OracleConfidenceTooWide, 208),
        (Error::InvalidOracleFeed, 209),
        (Error::OracleCallbackAuthFailed, 210),
        (Error::OracleCallbackUnauthorized, 211),
        (Error::OracleCallbackInvalidSignature, 212),
        (Error::OracleCallbackReplayDetected, 213),
        (Error::OracleCallbackTimeout, 214),

        (Error::InvalidQuestion, 300),
        (Error::InvalidOutcomes, 301),
        (Error::InvalidDuration, 302),
        (Error::InvalidThreshold, 303),
        (Error::InvalidComparison, 304),

        (Error::InvalidState, 400),
        (Error::InvalidInput, 401),
        (Error::InvalidFeeConfig, 402),
        (Error::ConfigNotFound, 403),
        (Error::AlreadyDisputed, 404),
        (Error::DisputeVoteExpired, 405),
        (Error::DisputeVoteDenied, 406),
        (Error::DisputeAlreadyVoted, 407),
        (Error::DisputeCondNotMet, 408),
        (Error::DisputeFeeFailed, 409),
        (Error::DisputeError, 410),
        (Error::SweepAlreadyDone, 411),
        (Error::FeeArithmeticOverflow, 412),
        (Error::FeeAlreadyCollected, 413),
        (Error::NoFeesToCollect, 414),
        (Error::InvalidExtensionDays, 415),
        (Error::ExtensionDenied, 416),
        (Error::GasBudgetExceeded, 417),
        (Error::OperationWouldExceedBudget, 418),
        (Error::AdminNotSet, 419),
        (Error::QuestionTooLong, 420),
        (Error::OutcomeTooLong, 421),
        (Error::TooManyOutcomes, 422),
        (Error::FeedIdTooLong, 423),
        (Error::ComparisonTooLong, 424),
        (Error::CategoryTooLong, 425),
        (Error::TagTooLong, 426),
        (Error::TooManyTags, 427),
        (Error::ExtensionReasonTooLong, 428),
        (Error::SourceTooLong, 429),
        (Error::ErrorMessageTooLong, 430),
        (Error::SignatureTooLong, 431),
        (Error::TooManyExtensions, 432),
        (Error::TooManyOracleResults, 433),
        (Error::TooManyWinningOutcomes, 434),
        (Error::ForceResolveAlreadyUsed, 435),
        (Error::CategoryTooShort, 436),
        (Error::TagTooShort, 437),
        (Error::DisputerCannotVote, 438),
        (Error::AssetDecimalsMismatch, 439),
        (Error::ArchiveFull, 440),
        (Error::DuplicateMarketId, 441),
        (Error::ReplayedOverride, 442),

        (Error::CBNotInitialized, 500),
        (Error::CBAlreadyOpen, 501),
        (Error::CBNotOpen, 502),
        (Error::CBOpen, 503),
        (Error::CBError, 504),
        (Error::RateLimitExceeded, 505),
        (Error::CumulativeExtensionCapHit, 506),
        (Error::IllegalMarketStateTransition, 507),
        (Error::FeeExceedsMax, 508),
        (Error::NoPendingFeeCommit, 519),
        (Error::FeeRevealTooEarly, 520),
        (Error::FeePreimageMismatch, 521),
        (Error::DisputeStakeCapExceeded, 522),
        (Error::InsufficientStorageRentBudget, 523),
        (Error::ExtensionCapExceeded, 524),
        (Error::UpgradeChainMismatch, 525),
        (Error::ReplayedAdminOverride, 526),
        (Error::OracleQuoteOutlier, 527),
    ];

    for (error, expected_code) in expected_codes {
        assert_eq!(
            error as u32,
            expected_code,
            "Error variant {:?} has a mismatched numeric code. Expected {}, but got {}. This breaks client-facing API stability.",
            error,
            expected_code,
            error as u32
        );
    }
}
