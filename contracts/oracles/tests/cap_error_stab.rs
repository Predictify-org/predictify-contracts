#![cfg(test)]

//! Stability tests for capabilities-related error codes in the oracles
//! contract.  These tests verify that error variants specific to the
//! capabilities subsystem retain their assigned numeric codes and
//! correct `Debug` formatting across refactors.

use oracles::Error;

// ---------------------------------------------------------------------------
// Individual code stability
// ---------------------------------------------------------------------------

#[test]
fn capability_not_supported_code() {
    let err = Error::CapabilityNotSupported;
    assert_eq!(err as u32, 220);
}

#[test]
fn capability_bitmap_corrupt_code() {
    let err = Error::CapabilityBitmapCorrupt;
    assert_eq!(err as u32, 221);
}

#[test]
fn reserved_capability_set_code() {
    let err = Error::ReservedCapabilitySet;
    assert_eq!(err as u32, 222);
}

// ---------------------------------------------------------------------------
// Debug formatting
// ---------------------------------------------------------------------------

#[test]
fn debug_format_capability_not_supported() {
    let s = format!("{:?}", Error::CapabilityNotSupported);
    assert!(
        !s.is_empty(),
        "Debug output must not be empty"
    );
}

#[test]
fn debug_format_capability_bitmap_corrupt() {
    let s = format!("{:?}", Error::CapabilityBitmapCorrupt);
    assert!(
        !s.is_empty(),
        "Debug output must not be empty"
    );
}

#[test]
fn debug_format_reserved_capability_set() {
    let s = format!("{:?}", Error::ReservedCapabilitySet);
    assert!(
        !s.is_empty(),
        "Debug output must not be empty"
    );
}

// ---------------------------------------------------------------------------
// Uniqueness across the entire Error enum
// ---------------------------------------------------------------------------

#[test]
fn all_error_codes_are_unique() {
    let variants: &[Error] = &[
        Error::OracleUnavailable,
        Error::InvalidOracleConfig,
        Error::OracleStale,
        Error::OracleNoConsensus,
        Error::OracleVerified,
        Error::MarketNotReady,
        Error::FallbackOracleUnavailable,
        Error::ResolutionTimeoutReached,
        Error::OracleConfidenceTooWide,
        Error::InvalidOracleFeed,
        Error::OracleCallbackAuthFailed,
        Error::OracleCallbackUnauthorized,
        Error::OracleCallbackInvalidSignature,
        Error::OracleCallbackReplayDetected,
        Error::OracleCallbackTimeout,
        Error::CapabilityNotSupported,
        Error::CapabilityBitmapCorrupt,
        Error::ReservedCapabilitySet,
    ];
    let mut codes: Vec<u32> = variants.iter().map(|v| *v as u32).collect();
    codes.sort();
    codes.dedup();
    assert_eq!(
        codes.len(),
        variants.len(),
        "Duplicate error codes detected"
    );
}
