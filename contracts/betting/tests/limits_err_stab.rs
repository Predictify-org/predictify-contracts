//! # Per-account limits — error-code stability tests
//!
//! These assertions freeze the numeric discriminants of the two error
//! variants introduced by the per-account limits module.  Clients (SDKs,
//! indexers, front-ends) may persist or branch on these values, so any
//! change to a discriminant is a **visible API break** requiring a
//! version bump and migration guide.
//!
//! ## Stability policy
//!
//! Once a discriminant appears here it is **frozen forever**.  Add new
//! variants with explicit, previously-unused `u32` discriminants and pin
//! them here in the same commit.  Never reuse a retired discriminant.
//!
//! | Variant                        | Code | Module               |
//! |--------------------------------|------|----------------------|
//! | `Error::PerAccountLimitExceeded`      | 677  | `betting::limits`    |
//! | `Error::PerAccountLimitInvalidConfig` | 678  | `betting::limits`    |

#![cfg(test)]

use predictify_hybrid::Error;

/// Both per-account limit error codes are frozen at their original values.
#[test]
fn per_account_limit_error_codes_are_stable() {
    assert_eq!(Error::PerAccountLimitExceeded as u32, 677);
    assert_eq!(Error::PerAccountLimitInvalidConfig as u32, 678);
}

/// The two new codes must not collide with any existing betting-path code.
#[test]
fn per_account_limit_error_codes_are_unique_among_betting_errors() {
    let all_betting_codes: &[u32] = &[
        // user-operation
        Error::Unauthorized as u32,
        Error::MarketNotFound as u32,
        Error::MarketClosed as u32,
        Error::MarketResolved as u32,
        Error::MarketNotResolved as u32,
        Error::NothingToClaim as u32,
        Error::AlreadyClaimed as u32,
        Error::InsufficientStake as u32,
        Error::InvalidOutcome as u32,
        Error::AlreadyBet as u32,
        Error::BetsAlreadyPlaced as u32,
        Error::InsufficientBalance as u32,
        Error::BetCoolOffActive as u32,
        // fee / cap
        Error::FeeExceedsMax as u32,
        Error::MaxBetCapExceeded as u32,
        Error::InvalidCap as u32,
        // batch idempotency
        Error::IdempotentBatchAlreadyApplied as u32,
        // general
        Error::InvalidState as u32,
        Error::InvalidInput as u32,
        // overflow
        Error::Overflow as u32,
        // NEW – per-account limits
        Error::PerAccountLimitExceeded as u32,
        Error::PerAccountLimitInvalidConfig as u32,
    ];

    for (i, &a) in all_betting_codes.iter().enumerate() {
        for (j, &b) in all_betting_codes.iter().enumerate() {
            if i != j {
                assert_ne!(
                    a, b,
                    "duplicate betting error code {a} at positions {i} and {j}"
                );
            }
        }
    }
}
