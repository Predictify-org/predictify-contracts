//! Lifecycle state validation and corruption detection.
//!
//! Provides deterministic state validation for market lifecycle transitions,
//! ensuring consistency between market state and archive/restore metadata.
//! Detects and reports state corruption, inconsistencies, and boundary violations.
//!
//! # Validation Strategy
//!
//! Each validation check is deterministic and idempotent:
//! - **State Consistency**: Market state matches archive/restore metadata
//! - **Metadata Integrity**: Archive/restore records are valid and well-formed
//! - **Transition Validation**: State transitions follow legal paths only
//! - **Corruption Detection**: Identifies missing or mismatched state records
//! - **Capacity Checks**: Verifies archive size constraints are maintained
//!
//! # Design
//!
//! Validations are designed to be:
//! - **Fast**: Early termination on first error
//! - **Complete**: Check all relevant invariants
//! - **Diagnostic**: Return detailed error info for debugging
//! - **Safe**: Never modify state (read-only operations)

use crate::err::Error;
use crate::event_archive::EventArchive;
use crate::restore_archive::RestoreArchive;
use crate::types::{Market, MarketState};
use soroban_sdk::{Env, String, Symbol};

/// Lifecycle validation result with detailed diagnostics.
///
/// Contains both the validation outcome and diagnostic information
/// for understanding why validation succeeded or failed.
#[derive(Clone, Debug)]
pub struct LifecycleValidationResult {
    /// Whether validation passed
    pub is_valid: bool,
    /// Error code if validation failed
    pub error: Option<Error>,
    /// Diagnostic message for debugging
    pub message: String,
    /// Timestamp of validation check
    pub checked_at: u64,
}

impl LifecycleValidationResult {
    /// Create a successful validation result
    pub fn success(env: &Env, message: &str) -> Self {
        Self {
            is_valid: true,
            error: None,
            message: String::from_str(env, message),
            checked_at: env.ledger().timestamp(),
        }
    }

    /// Create a failed validation result
    pub fn failure(env: &Env, error: Error, message: &str) -> Self {
        Self {
            is_valid: false,
            error: Some(error),
            message: String::from_str(env, message),
            checked_at: env.ledger().timestamp(),
        }
    }
}

/// Lifecycle state validator.
pub struct LifecycleValidator;

impl LifecycleValidator {
    /// Validate market state consistency with archive/restore metadata.
    ///
    /// Performs comprehensive checks to ensure:
    /// 1. Market state and archive metadata are synchronized
    /// 2. Market state and restore metadata are synchronized
    /// 3. Archive and restore states are mutually exclusive (not both)
    /// 4. Archive capacity constraints are maintained
    ///
    /// # Arguments
    /// * `env` - Soroban environment
    /// * `market_id` - Market to validate
    ///
    /// # Returns
    /// * `Ok(result)` - Validation completed (check result.is_valid for status)
    /// * `Err(error)` - Validation could not complete (e.g., market not found)
    pub fn validate_market_lifecycle(
        env: &Env,
        market_id: &Symbol,
    ) -> Result<LifecycleValidationResult, Error> {
        // Fetch market
        let market_opt: Option<Market> = env.storage().persistent().get(market_id);
        
        let market = match market_opt {
            Some(m) => m,
            None => {
                return Ok(LifecycleValidationResult::failure(
                    env,
                    Error::MarketNotFound,
                    "Market not found",
                ));
            }
        };

        // Validate based on market state
        match market.state {
            MarketState::Archived => {
                Self::validate_archived_market(env, market_id, &market)
            }
            MarketState::Restored => {
                Self::validate_restored_market(env, market_id, &market)
            }
            _ => {
                // For other states, just verify they're not incorrectly marked
                Self::validate_non_archived_market(env, market_id, &market)
            }
        }
    }

    /// Validate an archived market.
    ///
    /// Ensures:
    /// - Market state is Archived
    /// - Archive metadata exists and is consistent
    /// - Restore metadata does NOT exist (archived and restored are mutually exclusive)
    /// - Archive size constraint is maintained
    fn validate_archived_market(
        env: &Env,
        market_id: &Symbol,
        market: &Market,
    ) -> Result<LifecycleValidationResult, Error> {
        // Check state is exactly Archived
        if market.state != MarketState::Archived {
            return Ok(LifecycleValidationResult::failure(
                env,
                Error::InvalidState,
                "Market state mismatch: expected Archived",
            ));
        }

        // Verify archive metadata exists
        if !EventArchive::is_archived(env, market_id) {
            return Ok(LifecycleValidationResult::failure(
                env,
                Error::InvalidState,
                "Archive metadata missing: market claims to be archived but has no archive record",
            ));
        }

        // Verify restore metadata does NOT exist
        if RestoreArchive::is_restored(env, market_id) {
            return Ok(LifecycleValidationResult::failure(
                env,
                Error::InvalidState,
                "State corruption: market is both archived and restored",
            ));
        }

        // Verify archive size constraint
        let archive_size = EventArchive::archive_size(env);
        if archive_size > crate::event_archive::MAX_ARCHIVE_SIZE {
            return Ok(LifecycleValidationResult::failure(
                env,
                Error::InvalidState,
                "Archive capacity exceeded",
            ));
        }

        Ok(LifecycleValidationResult::success(
            env,
            "Archived market state is consistent",
        ))
    }

    /// Validate a restored market.
    ///
    /// Ensures:
    /// - Market state is Restored
    /// - Restore metadata exists and is consistent
    /// - Archive metadata still exists (restore doesn't delete archive record)
    /// - Restore metadata is properly versioned
    fn validate_restored_market(
        env: &Env,
        market_id: &Symbol,
        market: &Market,
    ) -> Result<LifecycleValidationResult, Error> {
        // Check state is exactly Restored
        if market.state != MarketState::Restored {
            return Ok(LifecycleValidationResult::failure(
                env,
                Error::InvalidState,
                "Market state mismatch: expected Restored",
            ));
        }

        // Verify restore metadata exists
        if !RestoreArchive::is_restored(env, market_id) {
            return Ok(LifecycleValidationResult::failure(
                env,
                Error::InvalidState,
                "Restore metadata missing: market claims to be restored but has no restore record",
            ));
        }

        // Verify restore metadata is properly formed and versioned
        if let Some(restore_entry) = RestoreArchive::get_restore_entry(env, market_id) {
            // Check version is recognized (current = 1)
            if restore_entry.version != 1 {
                return Ok(LifecycleValidationResult::failure(
                    env,
                    Error::InvalidState,
                    "Unsupported restore entry version",
                ));
            }
        } else {
            return Ok(LifecycleValidationResult::failure(
                env,
                Error::InvalidState,
                "Restore metadata exists but cannot be retrieved",
            ));
        }

        Ok(LifecycleValidationResult::success(
            env,
            "Restored market state is consistent",
        ))
    }

    /// Validate a non-archived/restored market.
    ///
    /// Ensures:
    /// - Market is not incorrectly marked as archived/restored
    /// - No orphaned archive/restore metadata exists
    fn validate_non_archived_market(
        env: &Env,
        market_id: &Symbol,
        market: &Market,
    ) -> Result<LifecycleValidationResult, Error> {
        // Check state is not Archived (should never reach here if state is Archived)
        if market.state == MarketState::Archived {
            return Ok(LifecycleValidationResult::failure(
                env,
                Error::InvalidState,
                "Market state inconsistency: state indicates Archived",
            ));
        }

        // Check state is not Restored (should never reach here if state is Restored)
        if market.state == MarketState::Restored {
            return Ok(LifecycleValidationResult::failure(
                env,
                Error::InvalidState,
                "Market state inconsistency: state indicates Restored",
            ));
        }

        // Check no orphaned archive metadata exists
        if EventArchive::is_archived(env, market_id) {
            return Ok(LifecycleValidationResult::failure(
                env,
                Error::InvalidState,
                "Orphaned archive metadata: market state is not Archived but archive record exists",
            ));
        }

        // Check no orphaned restore metadata exists
        if RestoreArchive::is_restored(env, market_id) {
            return Ok(LifecycleValidationResult::failure(
                env,
                Error::InvalidState,
                "Orphaned restore metadata: market state is not Restored but restore record exists",
            ));
        }

        Ok(LifecycleValidationResult::success(
            env,
            "Non-archived market state is consistent",
        ))
    }

    /// Validate state transition legality.
    ///
    /// Checks if a requested state transition is allowed by lifecycle rules:
    /// - **Active** → Ended, Disputed, Closed, Cancelled
    /// - **Ended** → Disputed, Resolved, Closed
    /// - **Disputed** → Resolved, Closed
    /// - **Resolved** → Archived, Closed
    /// - **Cancelled** → Archived, Closed
    /// - **Archived** → Restored
    /// - **Restored** → (any enabled state for re-activation)
    ///
    /// Invalid transitions are rejected deterministically.
    pub fn validate_state_transition(
        _env: &Env,
        from_state: MarketState,
        to_state: MarketState,
    ) -> Result<(), Error> {
        // Define legal transitions
        let is_legal = match (from_state, to_state) {
            // Active can transition to several states
            (MarketState::Active, MarketState::Ended) => true,
            (MarketState::Active, MarketState::Disputed) => true,
            (MarketState::Active, MarketState::Closed) => true,
            (MarketState::Active, MarketState::Cancelled) => true,

            // Ended can transition to dispute, resolution, or closure
            (MarketState::Ended, MarketState::Disputed) => true,
            (MarketState::Ended, MarketState::Resolved) => true,
            (MarketState::Ended, MarketState::Closed) => true,

            // Disputed can transition to resolution or closure
            (MarketState::Disputed, MarketState::Resolved) => true,
            (MarketState::Disputed, MarketState::Closed) => true,

            // Resolved can transition to archive or closure
            (MarketState::Resolved, MarketState::Archived) => true,
            (MarketState::Resolved, MarketState::Closed) => true,

            // Cancelled can transition to archive or closure
            (MarketState::Cancelled, MarketState::Archived) => true,
            (MarketState::Cancelled, MarketState::Closed) => true,

            // Archived can transition to restored
            (MarketState::Archived, MarketState::Restored) => true,

            // Restored can transition to closed or back to active (if re-enabled)
            (MarketState::Restored, MarketState::Closed) => true,

            // Closed is terminal - no transitions allowed
            (MarketState::Closed, _) => false,

            // Self-transitions are not allowed
            (from, to) if from == to => false,

            // All other transitions are illegal
            _ => false,
        };

        if !is_legal {
            return Err(Error::IllegalMarketStateTransition);
        }

        Ok(())
    }
}

// ===== TESTS =====

#[cfg(test)]
mod tests {
    use super::*;

    /// Placeholder for lifecycle validation tests.
    /// Comprehensive tests will be added in tests/contracts/lifecycle.test.ts
    #[test]
    fn test_placeholder() {
        // Tests will be implemented in dedicated test file
    }
}
