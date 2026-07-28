//! Analytics `ContractError` stability snapshot.
//!
//! # Purpose
//!
//! `ContractError` discriminants are serialized by the Soroban SDK and
//! consumed directly by off-chain analytics dashboards, indexers, and
//! monitoring tooling.  Any change to an existing code number is therefore
//! a **client-facing API break** that requires an explicit migration decision
//! and a corresponding note in the PR description.
//!
//! # Rules for contributors
//!
//! * **Never renumber** an existing variant.
//! * **Never remove** a variant (deprecate instead, document in the PR).
//! * **Never reuse** a code that belonged to a removed variant.
//! * New variants must be **appended** with a fresh, previously-unused code.
//! * Adding a new variant requires updating **both** this file and the
//!   exhaustive `frozen_code` match inside [`error_code_snapshot!`].  The
//!   compiler will reject an incomplete match, preventing silent omissions.
//!
//! # Snapshot last updated
//!
//! v0.0.0 — initial freeze (15 variants).

use std::collections::BTreeSet;

use analytics::ContractError;

// ---------------------------------------------------------------------------
// Macro: error_code_snapshot!
// ---------------------------------------------------------------------------
//
// Generates two items:
//   1. `ERROR_CODE_SNAPSHOT` — a `&[(ContractError, u32)]` slice that pairs
//      every variant with its frozen public code.
//   2. `frozen_code(error: ContractError) -> u32` — an exhaustive match so
//      that adding a new variant without updating this table is a *compile
//      error*, not a silent regression.

macro_rules! error_code_snapshot {
    ($($variant:ident = $code:literal),+ $(,)?) => {
        const ERROR_CODE_SNAPSHOT: &[(ContractError, u32)] = &[
            $((ContractError::$variant, $code),)+
        ];

        /// Returns the frozen public code for an analytics error.
        ///
        /// The match is exhaustive: a newly added `ContractError` variant will
        /// cause a **compile error** here until its public code is registered
        /// in the snapshot.
        fn frozen_code(error: ContractError) -> u32 {
            match error {
                $(ContractError::$variant => $code,)+
            }
        }
    };
}

// ---------------------------------------------------------------------------
// THE SNAPSHOT  — edit this table to register a new variant
// ---------------------------------------------------------------------------
//
// Code ranges (must not overlap):
//   1–9   general / auth
//   10–19 data / query
//   20–29 metric / aggregation
//   30–39 configuration / admin
// ---------------------------------------------------------------------------

error_code_snapshot! {
    // -- general / auth (1-9) --
    Unauthorized       = 1,
    AdminNotSet        = 2,
    NotInitialized     = 3,
    AlreadyInitialized = 4,

    // -- data / query (10-19) --
    MarketNotFound    = 10,
    SnapshotNotFound  = 11,
    InvalidTimeRange  = 12,
    UnsupportedWindow = 13,

    // -- metric / aggregation (20-29) --
    Overflow        = 20,
    StoreFull       = 21,
    DuplicateEntry  = 22,
    ValueOutOfRange = 23,

    // -- configuration / admin (30-39) --
    InvalidConfig    = 30,
    AnalyticsPaused  = 31,
    InvalidState     = 32,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Every analytics error code matches its frozen snapshot value.
///
/// Failure here means a discriminant changed — this is a client-facing API
/// break.  Do NOT simply update the expected value; open an issue, document
/// the migration path, and update the snapshot deliberately.
#[test]
fn analytics_error_codes_are_stable() {
    assert_eq!(
        ERROR_CODE_SNAPSHOT.len(),
        15,
        "snapshot length changed — update this count when adding or removing variants"
    );

    for &(error, expected) in ERROR_CODE_SNAPSHOT {
        assert_eq!(
            error as u32,
            expected,
            "client-facing error code changed for {error:?}; this is an API break"
        );
        // Also verify the exhaustive match returns the same value.
        assert_eq!(
            frozen_code(error),
            expected,
            "frozen_code() disagrees with snapshot for {error:?}"
        );
    }
}

/// All analytics error codes in the snapshot are unique.
///
/// Failure here means two variants share the same numeric code, which would
/// make it impossible for callers to distinguish them.
#[test]
fn analytics_error_codes_are_unique() {
    let mut seen: BTreeSet<u32> = BTreeSet::new();

    for &(error, code) in ERROR_CODE_SNAPSHOT {
        assert!(
            seen.insert(code),
            "duplicate analytics error code {code} — {error:?} collides with a previously registered variant"
        );
    }
}

/// Error codes fall within their declared ranges and respect the range
/// boundaries defined in the module documentation.
///
/// This test catches accidental off-by-one errors when assigning codes to a
/// new variant.
#[test]
fn analytics_error_codes_respect_range_boundaries() {
    for &(error, code) in ERROR_CODE_SNAPSHOT {
        let in_range = matches!(code, 1..=9 | 10..=19 | 20..=29 | 30..=39);
        assert!(
            in_range,
            "error code {code} for {error:?} falls outside any declared range (1-9, 10-19, 20-29, 30-39)"
        );
    }
}

/// All known `ContractError` variants appear in the snapshot at least once.
///
/// Because `frozen_code` uses an exhaustive match, a missing variant causes a
/// compile error.  This test provides an additional run-time guard that the
/// snapshot slice itself is non-empty and is exercised by the other tests.
#[test]
fn snapshot_is_non_empty() {
    assert!(
        !ERROR_CODE_SNAPSHOT.is_empty(),
        "ERROR_CODE_SNAPSHOT must not be empty"
    );
}
