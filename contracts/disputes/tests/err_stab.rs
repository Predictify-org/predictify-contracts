//! Stability snapshot for client-facing dispute error codes.
//!
//! Dispute errors are serialized into contract failures and consumed by client
//! SDKs, indexers, and frontends. Their numeric discriminants are therefore a
//! visible API. Renumbering, reordering, removing, or splitting one of these
//! variants requires an API review and a client migration plan.
//!
//! The values below are the frozen GrantFox FWC26 dispute-error snapshot. New
//! dispute errors must use a previously unused explicit discriminant and be
//! reviewed into this snapshot deliberately.

use std::collections::BTreeSet;

use predictify_hybrid::Error;

/// Frozen numeric codes for every dispute-related client-facing error.
///
/// The authoritative definitions are in `predictify_hybrid::Error`. Keep this
/// snapshot synchronized only through an intentional API change review.
const DISPUTE_ERROR_SNAPSHOT: &[(Error, u32)] = &[
    (Error::AlreadyDisputed, 404),
    (Error::DisputeVoteExpired, 405),
    (Error::DisputeVoteDenied, 406),
    (Error::DisputeAlreadyVoted, 407),
    (Error::DisputeCondNotMet, 408),
    (Error::DisputeFeeFailed, 409),
    (Error::DisputeError, 410),
    (Error::DisputerCannotVote, 438),
    (Error::NoDisputesFound, 496),
    (Error::DisputeStakeCapExceeded, 522),
];

/// Asserts that every frozen dispute error retains its client-facing code.
#[test]
fn dispute_error_discriminants_stable() {
    assert_eq!(
        DISPUTE_ERROR_SNAPSHOT.len(),
        10,
        "the dispute error snapshot must cover every frozen dispute error"
    );

    for &(error, expected) in DISPUTE_ERROR_SNAPSHOT {
        assert_eq!(
            error as u32,
            expected,
            "client-facing dispute error code changed for {error:?}; this requires an API migration"
        );
    }
}

/// Ensures that no two dispute errors expose the same numeric code.
#[test]
fn dispute_error_discriminants_are_unique() {
    let mut codes = BTreeSet::new();

    for &(error, code) in DISPUTE_ERROR_SNAPSHOT {
        assert!(
            codes.insert(code),
            "client-facing dispute error code {code} is reused by {error:?}"
        );
    }
}
