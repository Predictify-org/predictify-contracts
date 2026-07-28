//! Betting client-facing error-code stability tests.
//!
//! These assertions intentionally freeze the numeric values exposed by the
//! contract's betting-related errors.  Client applications (SDKs, indexers,
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

// ============================================================
// §1  User-Operation / Betting Errors  (100 – 113)
// ============================================================

/// Core betting errors exposed to every caller.
///
/// These codes surface in response to invalid user actions on betting
/// entrypoints (`place_bet`, `place_bets`, `cancel_bet`, `claim_winnings`).
/// Clients should surface these to end-users with localised messages keyed
/// on the numeric code.
#[test]
fn bet_user_operation_error_codes_are_stable() {
    assert_eq!(Error::Unauthorized as u32, 100);
    assert_eq!(Error::MarketNotFound as u32, 101);
    assert_eq!(Error::MarketClosed as u32, 102);
    assert_eq!(Error::MarketResolved as u32, 103);
    assert_eq!(Error::MarketNotResolved as u32, 104);
    assert_eq!(Error::NothingToClaim as u32, 105);
    assert_eq!(Error::AlreadyClaimed as u32, 106);
    assert_eq!(Error::InsufficientStake as u32, 107);
    assert_eq!(Error::InvalidOutcome as u32, 108);
    assert_eq!(Error::AlreadyBet as u32, 110);
    assert_eq!(Error::BetsAlreadyPlaced as u32, 111);
    assert_eq!(Error::InsufficientBalance as u32, 112);
    assert_eq!(Error::BetCoolOffActive as u32, 113);
}

// ============================================================
// §2  Fee and Cap Errors  (500 / 600-range)
// ============================================================

/// Fee-protection and bet-cap errors emitted during `place_bet` / `place_bets`.
///
/// `FeeExceedsMax` protects callers from unexpected fee increases: if the
/// effective platform fee (in basis points) has risen above the caller-supplied
/// `max_fee_bps` guard, the transaction is rejected rather than silently
/// overcharged.
///
/// `MaxBetCapExceeded` is returned when a user's cumulative stake across all
/// markets would surpass the per-address cap set by the admin.
///
/// `InvalidCap` is returned when an admin attempts to configure a cap value
/// that is zero or otherwise invalid.
#[test]
fn bet_fee_and_cap_error_codes_are_stable() {
    assert_eq!(Error::FeeExceedsMax as u32, 508);
    assert_eq!(Error::MaxBetCapExceeded as u32, 673);
    assert_eq!(Error::InvalidCap as u32, 674);
}

// ============================================================
// §3  Batch Idempotency Error  (500-range)
// ============================================================

/// Idempotency key reuse rejection for the `place_bets` batch entrypoint.
///
/// When a `place_bets` call is submitted with a key that was already
/// successfully consumed, this error is returned so that clients can
/// distinguish a network retry from a genuine duplicate.
#[test]
fn bet_batch_idempotency_error_codes_are_stable() {
    assert_eq!(Error::IdempotentBatchAlreadyApplied as u32, 509);
}

// ============================================================
// §4  General / State Errors Reachable from Betting Entrypoints
// ============================================================

/// General errors that may surface on the betting path.
///
/// These are not exclusive to betting, but they are part of the visible error
/// surface of the betting entrypoints and must remain stable.
#[test]
fn bet_general_error_codes_are_stable() {
    assert_eq!(Error::InvalidState as u32, 400);
    assert_eq!(Error::InvalidInput as u32, 401);
}

// ============================================================
// §5  Uniqueness Guard
// ============================================================

/// No two betting error codes share the same discriminant.
///
/// This test collects every code exercised by the suite above and asserts
/// they are pairwise distinct.  A failure here means a new error was added
/// with a discriminant that duplicates an existing one.
#[test]
fn bet_error_codes_are_unique() {
    let codes: &[u32] = &[
        // user-operation range
        Error::Unauthorized as u32,
        Error::MarketNotFound as u32,
        Error::MarketClosed as u32,
        Error::MarketResolved as u32,
        Error::MarketNotResolved as u32,
        Error::NothingToClaim as u32,
        Error::AlreadyClaimed as u32,
        Error::InsufficientStake as u32,
        Error::InvalidOutcome as u32,
        Error::AlreadyBet as u32,
        Error::BetsAlreadyPlaced as u32,
        Error::InsufficientBalance as u32,
        Error::BetCoolOffActive as u32,
        // fee / cap range
        Error::FeeExceedsMax as u32,
        Error::MaxBetCapExceeded as u32,
        Error::InvalidCap as u32,
        // batch idempotency
        Error::IdempotentBatchAlreadyApplied as u32,
        // general
        Error::InvalidState as u32,
        Error::InvalidInput as u32,
    ];

    for (i, &a) in codes.iter().enumerate() {
        for (j, &b) in codes.iter().enumerate() {
            if i != j {
                assert_ne!(
                    a, b,
                    "duplicate betting error code {a} found at positions {i} and {j}; \
                     each error must have a unique discriminant"
                );
            }
        }
    }
}