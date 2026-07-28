#![cfg(test)]

use fees::ContractError;

/// Ensure error variant discriminants are stable across versions.
#[test]
fn test_fees_error_stability() {
    assert_eq!(ContractError::Unauthorized as u32, 1);
    assert_eq!(ContractError::AdminNotSet as u32, 2);
    assert_eq!(ContractError::InvalidInput as u32, 3);
    assert_eq!(ContractError::InvalidState as u32, 4);
    assert_eq!(ContractError::Overflow as u32, 5);
    assert_eq!(ContractError::FeesPaused as u32, 6);
    assert_eq!(ContractError::FeeConfigNotFound as u32, 7);
    assert_eq!(ContractError::FeePercentageTooHigh as u32, 8);
    assert_eq!(ContractError::BelowCollectionThreshold as u32, 9);
}

/// Ensure all variants are covered by the match.
#[test]
fn test_all_variants_accounted_for() {
    // If a new variant is added, this test must be updated.
    let errors = [
        ContractError::Unauthorized,
        ContractError::AdminNotSet,
        ContractError::InvalidInput,
        ContractError::InvalidState,
        ContractError::Overflow,
        ContractError::FeesPaused,
        ContractError::FeeConfigNotFound,
        ContractError::FeePercentageTooHigh,
        ContractError::BelowCollectionThreshold,
    ];
    assert_eq!(errors.len(), 9, "All 9 variants must be listed");
}
