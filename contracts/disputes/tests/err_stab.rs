//! Disputes client-facing error-code stability tests.
//!
//! These assertions intentionally freeze the numeric values exposed by the
//! contract's dispute-related errors. Client applications may persist or branch on
//! these values, so changing one is a visible API change.

use predictify_hybrid::Error;

/// Dispute error codes are frozen to prevent accidental modifications or shifts.
#[test]
fn dispute_error_codes_are_stable() {
    let dispute_errors = [
        (Error::AlreadyDisputed, 404u32),
        (Error::DisputeVoteExpired, 405u32),
        (Error::DisputeVoteDenied, 406u32),
        (Error::DisputeAlreadyVoted, 407u32),
        (Error::DisputeCondNotMet, 408u32),
        (Error::DisputeFeeFailed, 409u32),
        (Error::DisputeError, 410u32),
        (Error::DisputerCannotVote, 438u32),
        (Error::DisputeStakeCapExceeded, 522u32),
    ];

    for (error, expected_code) in dispute_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "dispute error code changed for {error:?}; this is a client-facing API change"
        );
    }
}
