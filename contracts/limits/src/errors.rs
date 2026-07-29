use soroban_sdk::contracterror;

/// Semantic error variants for limit validations.
/// Each variant maps to a stable numeric code for client interpretation.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum LimitError {
    /// Bet amount is below the configured minimum.
    BetBelowMinimum = 1,
    /// Bet amount exceeds the configured maximum.
    BetExceedsMaximum = 2,
    /// Leverage must be greater than zero.
    LeverageMustBePositive = 3,
    /// Leverage exceeds the configured maximum.
    LeverageExceedsMax = 4,
    /// Fee in basis points exceeds the maximum allowed.
    FeeExceedsMax = 5,
    /// Total exposure would exceed pool limits.
    ExposureLimitReached = 6,
    /// Operation would exceed the per-user cap.
    UserCapExceeded = 7,
    /// Slippage tolerance outside acceptable range.
    SlippageOutOfRange = 8,
}

