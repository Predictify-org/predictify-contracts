use resolution::errors::ContractError;

#[test]
fn test_error_discriminant_stability() {
    // Ensure error variant discriminants are stable and match expected values.
    assert_eq!(ContractError::Unauthorized as u32, 1);
    assert_eq!(ContractError::MarketNotFound as u32, 2);
    assert_eq!(ContractError::MarketClosed as u32, 3);
    assert_eq!(ContractError::MarketAlreadyResolved as u32, 4);
    assert_eq!(ContractError::InvalidOutcome as u32, 5);
    assert_eq!(ContractError::InvalidInput as u32, 6);
    assert_eq!(ContractError::InvalidState as u32, 7);
    assert_eq!(ContractError::Overflow as u32, 8);
    assert_eq!(ContractError::ResolutionCooldownActive as u32, 9);
    assert_eq!(ContractError::OracleResultNotAvailable as u32, 10);
    assert_eq!(ContractError::InvalidWinningOutcomes as u32, 11);
}

#[test]
fn test_error_equality() {
    // Test that error variants compare correctly.
    assert_eq!(ContractError::Unauthorized, ContractError::Unauthorized);
    assert_eq!(ContractError::MarketNotFound, ContractError::MarketNotFound);
    assert_ne!(ContractError::Unauthorized, ContractError::MarketNotFound);
}

#[test]
fn test_error_ordering() {
    // Test that error variants have correct ordering based on discriminants.
    assert!(ContractError::Unauthorized < ContractError::MarketNotFound);
    assert!(ContractError::MarketNotFound < ContractError::MarketClosed);
    assert!(ContractError::MarketClosed < ContractError::MarketAlreadyResolved);
    assert!(ContractError::MarketAlreadyResolved < ContractError::InvalidOutcome);
    assert!(ContractError::InvalidOutcome < ContractError::InvalidInput);
    assert!(ContractError::InvalidInput < ContractError::InvalidState);
    assert!(ContractError::InvalidState < ContractError::Overflow);
    assert!(ContractError::Overflow < ContractError::ResolutionCooldownActive);
    assert!(ContractError::ResolutionCooldownActive < ContractError::OracleResultNotAvailable);
    assert!(ContractError::OracleResultNotAvailable < ContractError::InvalidWinningOutcomes);
}

#[test]
fn test_error_copy() {
    // Test that error variants implement Copy correctly.
    let error1 = ContractError::Unauthorized;
    let error2 = error1;
    assert_eq!(error1, error2);
}

#[test]
fn test_error_clone() {
    // Test that error variants implement Clone correctly.
    let error1 = ContractError::MarketNotFound;
    let error2 = error1.clone();
    assert_eq!(error1, error2);
}

#[test]
fn test_error_debug() {
    // Test that error variants implement Debug correctly.
    let error = ContractError::InvalidOutcome;
    let debug_str = format!("{:?}", error);
    assert!(debug_str.contains("InvalidOutcome"));
}

#[test]
fn test_all_error_variants_distinct() {
    // Ensure all error variants have distinct discriminants.
    let errors = vec![
        ContractError::Unauthorized,
        ContractError::MarketNotFound,
        ContractError::MarketClosed,
        ContractError::MarketAlreadyResolved,
        ContractError::InvalidOutcome,
        ContractError::InvalidInput,
        ContractError::InvalidState,
        ContractError::Overflow,
        ContractError::ResolutionCooldownActive,
        ContractError::OracleResultNotAvailable,
        ContractError::InvalidWinningOutcomes,
    ];
    
    let discriminants: Vec<u32> = errors.iter().map(|e| *e as u32).collect();
    let unique_discriminants: std::collections::HashSet<_> = discriminants.iter().collect();
    
    assert_eq!(discriminants.len(), unique_discriminants.len(), 
        "All error variants must have distinct discriminants");
}
