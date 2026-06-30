#! Error Code Stability Tests
//!
//! This test suite freezes the integer values of the client-facing Error enum
//! to detect accidental reordering or deletion of variants. The Error enum
//! discriminants are explicitly assigned (not auto-incremented) and marked with
//! a stability guarantee.
//!
//! If any of these assertions fail, it means a variant has been:
//! - Reordered
//! - Deleted
//! - Inserted without an explicit discriminant causing a shift
//! - Changed in name while the discriminant stayed the same
//!
//! See the documentation on the Error enum itself for the stability policy.

use predictify_hybrid::Error;

// ===== User Operation Errors (100-112) =====

#[test]
fn user_errors() {
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

// ===== Oracle Errors (200-214) =====

#[test]
fn oracle_errors() {
    assert_eq!(Error::OracleUnavailable as u32, 200);
    assert_eq!(Error::InvalidOracleConfig as u32, 201);
    assert_eq!(Error::OracleStale as u32, 202);
    assert_eq!(Error::OracleNoConsensus as u32, 203);
    assert_eq!(Error::OracleVerified as u32, 204);
    assert_eq!(Error::MarketNotReady as u32, 205);
    assert_eq!(Error::FallbackOracleUnavailable as u32, 206);
    assert_eq!(Error::ResolutionTimeoutReached as u32, 207);
    assert_eq!(Error::OracleConfidenceTooWide as u32, 208);
    assert_eq!(Error::InvalidOracleFeed as u32, 209);
    assert_eq!(Error::OracleCallbackAuthFailed as u32, 210);
    assert_eq!(Error::OracleCallbackUnauthorized as u32, 211);
    assert_eq!(Error::OracleCallbackInvalidSignature as u32, 212);
    assert_eq!(Error::OracleCallbackReplayDetected as u32, 213);
    assert_eq!(Error::OracleCallbackTimeout as u32, 214);
}

// ===== Validation Errors (300-304) =====

#[test]
fn validation_errors() {
    assert_eq!(Error::InvalidQuestion as u32, 300);
    assert_eq!(Error::InvalidOutcomes as u32, 301);
    assert_eq!(Error::InvalidDuration as u32, 302);
    assert_eq!(Error::InvalidThreshold as u32, 303);
    assert_eq!(Error::InvalidComparison as u32, 304);
}

// ===== General & State Errors (400-441) =====

#[test]
fn general_errors() {
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
    assert_eq!(Error::DisputeError as u32, 410);
    assert_eq!(Error::SweepAlreadyDone as u32, 411);
    assert_eq!(Error::FeeArithmeticOverflow as u32, 412);
    assert_eq!(Error::FeeAlreadyCollected as u32, 413);
    assert_eq!(Error::NoFeesToCollect as u32, 414);
    assert_eq!(Error::InvalidExtensionDays as u32, 415);
    assert_eq!(Error::ExtensionDenied as u32, 416);
    assert_eq!(Error::GasBudgetExceeded as u32, 417);
    assert_eq!(Error::AdminNotSet as u32, 418);
    assert_eq!(Error::QuestionTooLong as u32, 420);
    assert_eq!(Error::OutcomeTooLong as u32, 421);
    assert_eq!(Error::TooManyOutcomes as u32, 422);
    assert_eq!(Error::FeedIdTooLong as u32, 423);
    assert_eq!(Error::ComparisonTooLong as u32, 424);
    assert_eq!(Error::CategoryTooLong as u32, 425);
    assert_eq!(Error::TagTooLong as u32, 426);
    assert_eq!(Error::TooManyTags as u32, 427);
    assert_eq!(Error::ExtensionReasonTooLong as u32, 428);
    assert_eq!(Error::SourceTooLong as u32, 429);
    assert_eq!(Error::ErrorMessageTooLong as u32, 430);
    assert_eq!(Error::SignatureTooLong as u32, 431);
    assert_eq!(Error::TooManyExtensions as u32, 432);
    assert_eq!(Error::TooManyOracleResults as u32, 433);
    assert_eq!(Error::TooManyWinningOutcomes as u32, 434);
    assert_eq!(Error::CategoryTooShort as u32, 436);
    assert_eq!(Error::TagTooShort as u32, 437);
    assert_eq!(Error::DisputerCannotVote as u32, 438);
    assert_eq!(Error::ArchiveFull as u32, 440);
    assert_eq!(Error::DuplicateMarketId as u32, 441);
}

// ===== Circuit Breaker Errors (500-508) =====

#[test]
fn circuit_breaker_errors() {
    assert_eq!(Error::CBNotInitialized as u32, 500);
    assert_eq!(Error::CBAlreadyOpen as u32, 501);
    assert_eq!(Error::CBNotOpen as u32, 502);
    assert_eq!(Error::CBOpen as u32, 503);
    assert_eq!(Error::CBError as u32, 504);
    assert_eq!(Error::RateLimitExceeded as u32, 505);
    assert_eq!(Error::CumulativeExtensionCapHit as u32, 506);
    assert_eq!(Error::IllegalMarketStateTransition as u32, 507);
    assert_eq!(Error::FeeExceedsMax as u32, 508);
}

// ===== Asset decimals =====

#[test]
fn asset_decimals() {
    assert_eq!(Error::AssetDecimalsMismatch as u32, 439);
}

// ===== op-count test ensuring we noticed all variants =====

#[test]
fn total_variant_count() {
    // This is a smoke check: count the number of variants known to the client-facing
    // Error enum. If the count changes from the expected value, the developer will be
    // forced to verify whether the new/deleted variant has a proper frozen discriminant.
    //
    // This count may need to be updated after legitimate additions. The purpose is to
    // catch accidental insertions that could shift discriminants.
    // update this comment when updating the count.
    let expected = 93;
    assert_eq!(std::mem::variant_count::<Error>(), expected);
}