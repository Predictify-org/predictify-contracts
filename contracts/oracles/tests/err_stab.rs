//! Oracle client-facing error-code stability tests.
//!
//! These assertions intentionally freeze the numeric values exposed by the
//! contract's oracle errors. Client applications may persist or branch on
//! these values, so changing one is a visible API change and requires an
//! explicit migration or versioning decision.

use oracles::Error;

/// Oracle error codes are reserved for the 200-series range.
///
/// Keep this table explicit rather than deriving values from enum ordering:
/// the numeric assignments are part of the client-facing contract.
#[test]
fn oracle_error_codes_are_stable() {
    let oracle_errors = [
        (Error::OracleUnavailable, 200u32),
        (Error::InvalidOracleConfig, 201u32),
        (Error::OracleStale, 202u32),
        (Error::OracleNoConsensus, 203u32),
        (Error::OracleVerified, 204u32),
        (Error::MarketNotReady, 205u32),
        (Error::FallbackOracleUnavailable, 206u32),
        (Error::ResolutionTimeoutReached, 207u32),
        (Error::OracleConfidenceTooWide, 208u32),
        (Error::InvalidOracleFeed, 209u32),
        (Error::OracleCallbackAuthFailed, 210u32),
        (Error::OracleCallbackUnauthorized, 211u32),
        (Error::OracleCallbackInvalidSignature, 212u32),
        (Error::OracleCallbackReplayDetected, 213u32),
        (Error::OracleCallbackTimeout, 214u32),
    ];

    for (error, expected_code) in oracle_errors {
        assert_eq!(
            error as u32,
            expected_code,
            "oracle error code changed for {error:?}; this is a client-facing API change"
        );
    }
}

/// Oracle error codes remain contiguous and contain no duplicates.
#[test]
fn oracle_error_codes_are_unique_and_contiguous() {
    let codes = [
        Error::OracleUnavailable as u32,
        Error::InvalidOracleConfig as u32,
        Error::OracleStale as u32,
        Error::OracleNoConsensus as u32,
        Error::OracleVerified as u32,
        Error::MarketNotReady as u32,
        Error::FallbackOracleUnavailable as u32,
        Error::ResolutionTimeoutReached as u32,
        Error::OracleConfidenceTooWide as u32,
        Error::InvalidOracleFeed as u32,
        Error::OracleCallbackAuthFailed as u32,
        Error::OracleCallbackUnauthorized as u32,
        Error::OracleCallbackInvalidSignature as u32,
        Error::OracleCallbackReplayDetected as u32,
        Error::OracleCallbackTimeout as u32,
    ];

    for (index, code) in codes.iter().enumerate() {
        assert_eq!(*code, 200u32 + index as u32);
        assert!(!codes[..index].contains(code));
    }
}
