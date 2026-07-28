//! # Error discriminant stability tests
//!
//! These tests pin the numeric value of every [`MonitorError`] variant.
//!
//! They are intentionally brittle: if a discriminant changes, these tests
//! fail immediately, alerting the author that they have introduced a breaking
//! change to the on-chain ABI.

#![cfg(test)]

use monitor::MonitorError;

#[test]
fn test_error_discriminants_are_stable() {
    assert_eq!(MonitorError::Unauthorized as u32, 1);
    assert_eq!(MonitorError::NotInitialized as u32, 2);
    assert_eq!(MonitorError::AlreadyInitialized as u32, 3);
    assert_eq!(MonitorError::BetCapExceeded as u32, 4);
    assert_eq!(MonitorError::PositionCapExceeded as u32, 5);
    assert_eq!(MonitorError::SubscriptionCapExceeded as u32, 6);
    assert_eq!(MonitorError::InvalidInput as u32, 7);
    assert_eq!(MonitorError::Overflow as u32, 8);
    assert_eq!(MonitorError::Underflow as u32, 9);
}

#[test]
fn test_debug_format_does_not_panic() {
    let _ = format!("{:?}", MonitorError::Unauthorized);
    let _ = format!("{:?}", MonitorError::NotInitialized);
    let _ = format!("{:?}", MonitorError::AlreadyInitialized);
    let _ = format!("{:?}", MonitorError::BetCapExceeded);
    let _ = format!("{:?}", MonitorError::PositionCapExceeded);
    let _ = format!("{:?}", MonitorError::SubscriptionCapExceeded);
    let _ = format!("{:?}", MonitorError::InvalidInput);
    let _ = format!("{:?}", MonitorError::Overflow);
    let _ = format!("{:?}", MonitorError::Underflow);
}
