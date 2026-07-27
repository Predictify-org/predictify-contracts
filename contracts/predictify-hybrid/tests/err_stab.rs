//! Betting error-code stability tests.
//!
//! This test suite **freezes** the numeric discriminant values for every
//! `Error` variant that a client application can receive from a betting
//! entrypoint (`place_bet`, `place_bets`, `cancel_bet`, `claim_winnings`,
//! `set_max_bet_cap`).
//!
//! ## Why this matters
//!
//! Client applications (web front-ends, indexers, bots) typically pattern-match
//! on the raw `u32` error code returned by the Soroban host — not on the Rust
//! variant name.  Reordering, inserting without an explicit discriminant, or
//! renaming a variant while keeping its discriminant are all **client-visible
//! API changes** that must be treated as breaking changes.
//!
//! These assertions will fail if any of the following happen:
//!
//! * A variant is reordered relative to another (shifting the auto-incremented
//!   value assigned by the compiler).
//! * A variant is inserted before an existing one without an explicit
//!   discriminant, causing all subsequent variants to shift.
//! * A variant is deleted, causing later ones to shift.
//! * A variant is renamed but its discriminant stays the same (name change is
//!   still a source-level breaking change even though the wire value is stable).
//!
//! ## Stability policy
//!
//! Every variant in the `Error` enum carries an explicit `= <N>` discriminant.
//! New betting errors **must** choose a fresh number that has never been used
//! and must never reuse a retired discriminant.  See the enum-level doc comment
//! in `src/err.rs` for the full policy.

use predictify_hybrid::Error;

// ============================================================
// Betting path — user operation errors (100-range)
// ============================================================

/// Core user-operation errors reachable from every betting entrypoint.
///
/// These codes appear in `place_bet`, `place_bets`, `cancel_bet`,
/// `claim_winnings` and related flows.
#[test]
fn bet_user_operation_error_codes_are_stable() {
    let table: &[(Error, u32)] = &[
        // The user attempted to interact with a market that does not exist.
        (Error::MarketNotFound, 101),
        // The market deadline has passed; no new bets are accepted.
        (Error::MarketClosed, 102),
        // The market has already been resolved; no new bets are accepted.
        (Error::MarketResolved, 103),
        // The market has not yet been resolved; winnings cannot be claimed.
        (Error::MarketNotResolved, 104),
        // The user has no bet on this market to claim or cancel.
        (Error::NothingToClaim, 105),
        // Winnings have already been claimed; duplicate claim rejected.
        (Error::AlreadyClaimed, 106),
        // Bet amount is below the market's configured minimum.
        (Error::InsufficientStake, 107),
        // The chosen outcome does not exist in this market's outcome list.
        (Error::InvalidOutcome, 108),
        // The user already has an active bet on this market.
        (Error::AlreadyBet, 110),
        // Bets have been placed; further state-mutating operations blocked.
        (Error::BetsAlreadyPlaced, 111),
        // User's token balance is insufficient to cover the bet amount.
        (Error::InsufficientBalance, 112),
    ];

    for &(error, expected) in table {
        assert_eq!(
            error as u32,
            expected,
            "betting user-operation error code changed for {error:?}; \
             this is a client-facing API break"
        );
    }
}

// ============================================================
// Betting path — fee and cap errors (500/600-range)
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
    let table: &[(Error, u32)] = &[
        // Effective fee (bps) exceeds the caller-supplied max_fee_bps guard.
        (Error::FeeExceedsMax, 508),
        // Cumulative stake would exceed the per-address bet cap.
        (Error::MaxBetCapExceeded, 673),
        // Admin tried to set a bet cap that is zero or negative.
        (Error::InvalidCap, 674),
    ];

    for &(error, expected) in table {
        assert_eq!(
            error as u32,
            expected,
            "betting fee/cap error code changed for {error:?}; \
             this is a client-facing API break"
        );
    }
}

// ============================================================
// Betting path — batch idempotency errors (500-range)
// ============================================================

/// Idempotency key reuse rejection for the `place_bets` batch entrypoint.
///
/// When a `place_bets` call is submitted with a key that was already
/// successfully consumed, this error is returned so that clients can
/// distinguish a network retry from a genuine duplicate.
#[test]
fn bet_batch_idempotency_error_codes_are_stable() {
    // The 509 slot is the canonical client-facing code.
    assert_eq!(
        Error::IdempotentBatchAlreadyApplied as u32,
        509,
        "IdempotentBatchAlreadyApplied error code changed; \
         clients that match on 509 will break"
    );
}

// ============================================================
// General / state errors reachable from betting entrypoints
// ============================================================

/// General errors that may surface on the betting path.
///
/// These are not exclusive to betting, but they are part of the visible error
/// surface of the betting entrypoints and must remain stable.
#[test]
fn bet_general_error_codes_are_stable() {
    let table: &[(Error, u32)] = &[
        // Caller is not authorised for an admin-only operation (e.g. set_max_bet_cap).
        (Error::Unauthorized, 100),
        // Generic input validation failure (empty batch, invalid parameter, etc.).
        (Error::InvalidInput, 401),
        // Contract/market state is inconsistent; an invariant was violated.
        (Error::InvalidState, 400),
    ];

    for &(error, expected) in table {
        assert_eq!(
            error as u32,
            expected,
            "betting general error code changed for {error:?}; \
             this is a client-facing API break"
        );
    }
}

// ============================================================
// Uniqueness guard
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
        // fee / cap range
        Error::FeeExceedsMax as u32,
        Error::MaxBetCapExceeded as u32,
        Error::InvalidCap as u32,
        // batch idempotency
        Error::IdempotentBatchAlreadyApplied as u32,
        // general
        Error::Unauthorized as u32,
        Error::InvalidInput as u32,
        Error::InvalidState as u32,
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
