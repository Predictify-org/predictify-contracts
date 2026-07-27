//! Stability snapshot for client-facing dispute error codes.
//!
//! This test freezes the `#[repr(u32)]` discriminant of every dispute-related
//! variant in `predictify_hybrid::Error`.  Renumbering, reordering, removing,
//! or splitting these variants is a **client-facing API breaking change**
//! and the snapshot below will fail CI to surface it explicitly.
//!
//! The source of truth for each numeric code is the `Error` enum in
//! `predictify-hybrid/src/err.rs`.  Values here must match one-for-one.
//!
//! ## Coverage
//!
//! | Variant name                     | Frozen discriminant |
//! |----------------------------------|---------------------|
//! | `AlreadyDisputed`                | 404                 |
//! | `DisputeVoteExpired`             | 405                 |
//! | `DisputeVoteDenied`              | 406                 |
//! | `DisputeAlreadyVoted`            | 407                 |
//! | `DisputeCondNotMet`              | 408                 |
//! | `DisputeFeeFailed`               | 409                 |
//! | `DisputeError`                   | 410                 |
//! | `DisputerCannotVote`             | 438                 |
//! | `DisputeStakeCapExceeded`        | 522                 |
//! | `NoDisputesFound`                | 496                 |

use predictify_hybrid::Error;

/// Asserts that every dispute-related `Error` variant still maps to its
/// long-stable client-facing numeric discriminant.
///
/// Failures on this test must be treated as API-review required, not as
/// "fix the test" mechanical changes — downstream clients and event
/// dashboards match on these codes directly.
#[test]
fn dispute_error_discriminants_stable() {
    let snapshot: &[(Error, u32)] = &[
        (Error::AlreadyDisputed, 404),
        (Error::DisputeVoteExpired, 405),
        (Error::DisputeVoteDenied, 406),
        (Error::DisputeAlreadyVoted, 407),
        (Error::DisputeCondNotMet, 408),
        (Error::DisputeFeeFailed, 409),
        (Error::DisputeError, 410),
        (Error::DisputerCannotVote, 438),
        (Error::DisputeStakeCapExceeded, 522),
        (Error::NoDisputesFound, 496),
    ];

    for (err, expected) in snapshot {
        assert_eq!(
            *err as u32,
            *expected,
            "client-facing API break: discriminant for {:?} changed from {} to {}; \
             downstream clients rely on the stable numeric codes documented above",
            err,
            expected,
            *err as u32,
        );
    }
}
