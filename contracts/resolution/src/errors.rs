use soroban_sdk::contracterror;

/// A stable catalog of errors for the resolution smart contract.
///
/// # Stability
///
/// Each discriminant is part of the client-facing contract API. Clients may
/// persist these numbers or use them to decode failed invocations, so existing
/// values must not be renumbered or reused. Add new variants with an explicit,
/// previously unused value and update tests in the same change.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// Action is unauthorized; typically thrown when a non-admin invokes an admin action.
    Unauthorized = 1,
    /// Market not found; thrown when querying or resolving a non-existent market.
    MarketNotFound = 2,
    /// Market is closed; thrown when trying to resolve a market before its end time.
    MarketClosed = 3,
    /// Market already resolved; thrown when attempting to resolve a market more than once.
    MarketAlreadyResolved = 4,
    /// Invalid outcome; thrown when the provided outcome is not supported by the market.
    InvalidOutcome = 5,
    /// Invalid input; thrown when provided parameters are out of bounds or malformed.
    InvalidInput = 6,
    /// Invalid state; thrown when a state transition is not allowed.
    InvalidState = 7,
    /// Arithmetic overflow prevented.
    Overflow = 8,
    /// Resolution cooldown active; thrown when resolution is attempted too soon after previous resolution.
    ResolutionCooldownActive = 9,
    /// Oracle result not available; thrown when oracle data is missing or inaccessible.
    OracleResultNotAvailable = 10,
    /// Invalid winning outcomes; thrown when the winning outcomes list is empty or invalid.
    InvalidWinningOutcomes = 11,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_variant_stability() {
        // Ensure error variant discriminants do not accidentally change.
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
}
