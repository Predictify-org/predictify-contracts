//! Markets client-facing error-code stability tests.
//!
//! These assertions **freeze** the integer discriminants of every market-facing
//! [`predictify_hybrid::Error`] variant.  Client applications (SDKs, indexers,
//! front-ends) may persist or branch on these numeric values, so any change to
//! a discriminant is a **visible API break** that requires a version bump and a
//! migration guide.
//!
//! # What this test catches
//!
//! | Change | Detected? |
//! |--------|-----------|
//! | Variant reordered (auto-increment shift) | ✅ |
//! | Variant deleted | ✅ (compile error on next build) |
//! | Variant renamed while discriminant stays the same | ✅ (compile error) |
//! | New variant inserted without an explicit discriminant | ✅ |
//!
//! # Stability policy
//!
//! Once a discriminant appears in this file it is **frozen forever**.  To add a
//! new error variant, assign it an explicit, previously-unused `u32` discriminant
//! in [`predictify_hybrid::Error`] and add a corresponding `assert_eq!` here.
//! Never reuse a retired discriminant.
//!
//! See [`predictify_hybrid::Error`] for the authoritative enum definition.

use predictify_hybrid::Error;

// ─────────────────────────────────────────────────────────────────────────────
// §1  User-Operation / Market-Lifecycle Errors  (100 – 112)
// ─────────────────────────────────────────────────────────────────────────────

/// Core market-lifecycle errors exposed to every caller.
///
/// These codes surface in response to invalid user actions (wrong market state,
/// duplicate operations, insufficient funds, etc.).  Clients should surface
/// these to end-users with localised messages keyed on the numeric code.
#[test]
fn market_user_errors_are_stable() {
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

// ─────────────────────────────────────────────────────────────────────────────
// §2  Oracle / Resolution Errors  (200 – 214)
// ─────────────────────────────────────────────────────────────────────────────

/// Oracle and resolution errors that surface when the on-chain data pipeline
/// cannot produce or verify a market outcome.
#[test]
fn market_oracle_errors_are_stable() {
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

// ─────────────────────────────────────────────────────────────────────────────
// §3  Market-Creation Validation Errors  (300 – 304)
// ─────────────────────────────────────────────────────────────────────────────

/// Input-validation failures that occur during market creation or update.
/// Front-ends use these to drive field-level error messages in the UI.
#[test]
fn market_validation_errors_are_stable() {
    assert_eq!(Error::InvalidQuestion as u32, 300);
    assert_eq!(Error::InvalidOutcomes as u32, 301);
    assert_eq!(Error::InvalidDuration as u32, 302);
    assert_eq!(Error::InvalidThreshold as u32, 303);
    assert_eq!(Error::InvalidComparison as u32, 304);
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  General / State Errors  (400 – 443)
// ─────────────────────────────────────────────────────────────────────────────

/// System-state and configuration errors.  Includes admin-only codes that
/// off-chain operators monitor via event streams.
///
/// Note: 404–410, 438, and 522 are dispute-subsystem codes and are covered
/// separately in `contracts/disputes/tests/err_stab.rs`.
#[test]
fn market_general_errors_are_stable() {
    assert_eq!(Error::InvalidState as u32, 400);
    assert_eq!(Error::InvalidInput as u32, 401);
    assert_eq!(Error::InvalidFeeConfig as u32, 402);
    assert_eq!(Error::ConfigNotFound as u32, 403);
    // 404-410 → dispute subsystem (see contracts/disputes)
    assert_eq!(Error::SweepAlreadyDone as u32, 411);
    assert_eq!(Error::FeeArithmeticOverflow as u32, 412);
    assert_eq!(Error::FeeAlreadyCollected as u32, 413);
    assert_eq!(Error::NoFeesToCollect as u32, 414);
    assert_eq!(Error::InvalidExtensionDays as u32, 415);
    assert_eq!(Error::ExtensionDenied as u32, 416);
    assert_eq!(Error::GasBudgetExceeded as u32, 417);
    assert_eq!(Error::OperationWouldExceedBudget as u32, 418);
    assert_eq!(Error::AdminNotSet as u32, 419);
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
    assert_eq!(Error::ForceResolveAlreadyUsed as u32, 435);
    assert_eq!(Error::CategoryTooShort as u32, 436);
    assert_eq!(Error::TagTooShort as u32, 437);
    // 438 → DisputerCannotVote (dispute subsystem)
    assert_eq!(Error::AssetDecimalsMismatch as u32, 439);
    assert_eq!(Error::ArchiveFull as u32, 440);
    assert_eq!(Error::DuplicateMarketId as u32, 441);
    assert_eq!(Error::AdminActionTimelocked as u32, 443);
}

// ─────────────────────────────────────────────────────────────────────────────
// §5  Circuit-Breaker / Safety-Guard Errors  (500 – 528)
// ─────────────────────────────────────────────────────────────────────────────

/// Runtime safety-guard errors.  Circuit-breaker codes (500–504) indicate an
/// emergency halt; remaining codes cover idempotency, fee-commit, and
/// resource-cap guards.
#[test]
fn market_circuit_breaker_errors_are_stable() {
    assert_eq!(Error::CBNotInitialized as u32, 500);
    assert_eq!(Error::CBAlreadyOpen as u32, 501);
    assert_eq!(Error::CBNotOpen as u32, 502);
    assert_eq!(Error::CBOpen as u32, 503);
    assert_eq!(Error::CBError as u32, 504);
    assert_eq!(Error::RateLimitExceeded as u32, 505);
    assert_eq!(Error::CumulativeExtensionCapHit as u32, 506);
    assert_eq!(Error::IllegalMarketStateTransition as u32, 507);
    assert_eq!(Error::FeeExceedsMax as u32, 508);
    assert_eq!(Error::IdempotentBatchAlreadyApplied as u32, 509);
    // Slots 510–516 are currently unallocated; do not reuse retired codes.
    assert_eq!(Error::ForceResolveReplayed as u32, 517);
    assert_eq!(Error::ForceResolveReasonEmpty as u32, 518);
    assert_eq!(Error::NoPendingFeeCommit as u32, 519);
    assert_eq!(Error::FeeRevealTooEarly as u32, 520);
    assert_eq!(Error::FeePreimageMismatch as u32, 521);
    // 522 → DisputeStakeCapExceeded (dispute subsystem)
    assert_eq!(Error::InsufficientStorageRentBudget as u32, 523);
    assert_eq!(Error::ExtensionCapExceeded as u32, 524);
    assert_eq!(Error::UpgradeChainMismatch as u32, 525);
    // 526 → ReplayedOverride (admin-override subsystem; reserved)
    assert_eq!(Error::OracleQuoteOutlier as u32, 527);
    assert_eq!(Error::MaxParticipantsReached as u32, 528);
}

// ─────────────────────────────────────────────────────────────────────────────
// §6  Audit / Admin-Op Errors  (660 – 674)
// ─────────────────────────────────────────────────────────────────────────────

/// High-numbered codes reserved for admin-operation and audit facilities.
/// These are typically only seen by operators, not end-users.
#[test]
fn market_audit_errors_are_stable() {
    assert_eq!(Error::ReasonTableFull as u32, 670);
    assert_eq!(Error::Overflow as u32, 672);
    assert_eq!(Error::MaxBetCapExceeded as u32, 673);
    assert_eq!(Error::InvalidCap as u32, 674);
}

// ─────────────────────────────────────────────────────────────────────────────
// §7  Boundary smoke-test
// ─────────────────────────────────────────────────────────────────────────────

/// Verifies that the lowest and highest documented market error codes have not
/// moved.  Update `highest` if a new variant is intentionally added above 674.
#[test]
fn market_error_code_boundaries_are_stable() {
    // Lowest market error code.
    assert_eq!(
        Error::Unauthorized as u32,
        100,
        "lowest market error code (Unauthorized=100) has shifted",
    );

    // Highest currently-documented market error code.
    assert_eq!(
        Error::InvalidCap as u32,
        674,
        "highest market error code (InvalidCap=674) has shifted; update this assertion \
         if a new variant was intentionally added above 674",
    );
}
