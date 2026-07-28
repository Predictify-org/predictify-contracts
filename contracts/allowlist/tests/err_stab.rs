#![cfg(test)]

//! Ensures that [`AllowlistError`] variant discriminants remain stable across
//! contract versions.

use allowlist::AllowlistError;

#[test]
fn test_error_variant_stability() {
    assert_eq!(AllowlistError::Unauthorized as u32, 1);
    assert_eq!(AllowlistError::NotInitialized as u32, 2);
    assert_eq!(AllowlistError::AlreadyInitialized as u32, 3);
    assert_eq!(AllowlistError::AllowlistNotFound as u32, 4);
    assert_eq!(AllowlistError::AllowlistAlreadyExists as u32, 5);
    assert_eq!(AllowlistError::AddressAlreadyInAllowlist as u32, 6);
    assert_eq!(AllowlistError::AddressNotInAllowlist as u32, 7);
    assert_eq!(AllowlistError::AllowlistEmpty as u32, 8);
    assert_eq!(AllowlistError::InvalidInput as u32, 9);
    assert_eq!(AllowlistError::Overflow as u32, 10);
}

#[test]
fn test_debug_format_does_not_panic() {
    // Just ensuring the derived Debug impl works for all variants.
    let _ = format!("{:?}", AllowlistError::Unauthorized);
    let _ = format!("{:?}", AllowlistError::NotInitialized);
    let _ = format!("{:?}", AllowlistError::AlreadyInitialized);
    let _ = format!("{:?}", AllowlistError::AllowlistNotFound);
    let _ = format!("{:?}", AllowlistError::AllowlistAlreadyExists);
    let _ = format!("{:?}", AllowlistError::AddressAlreadyInAllowlist);
    let _ = format!("{:?}", AllowlistError::AddressNotInAllowlist);
    let _ = format!("{:?}", AllowlistError::AllowlistEmpty);
    let _ = format!("{:?}", AllowlistError::InvalidInput);
    let _ = format!("{:?}", AllowlistError::Overflow);
}
