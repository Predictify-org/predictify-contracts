use soroban_sdk::contracterror;

/// A stable catalog of errors for the fees smart contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// Action is unauthorized; typically thrown when a non-admin invokes an admin action.
    Unauthorized = 1,
    /// Admin not set; thrown when no admin has been configured for the contract.
    AdminNotSet = 2,
    /// Invalid input; thrown when provided parameters are out of bounds or malformed.
    InvalidInput = 3,
    /// Invalid state; thrown when a state transition is not allowed.
    InvalidState = 4,
    /// Arithmetic overflow prevented.
    Overflow = 5,
    /// Fees are currently paused.
    FeesPaused = 6,
    /// Fee configuration not found.
    FeeConfigNotFound = 7,
    /// Fee percentage exceeds maximum allowed.
    FeePercentageTooHigh = 8,
    /// Fee collection threshold not met.
    BelowCollectionThreshold = 9,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_variant_stability() {
        // Ensure error variant discriminants do not accidentally change.
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
}
