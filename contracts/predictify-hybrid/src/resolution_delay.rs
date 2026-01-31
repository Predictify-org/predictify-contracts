//! Resolution Delay and Dispute Window Module
//!
//! This module implements a configurable resolution delay (dispute window) after market
//! end during which disputes can be raised. Payouts occur only after the window closes
//! with no unresolved disputes.
//!
//! # Key Features
//!
//! - **Configurable Window Duration**: Admin can set global or per-market dispute window
//! - **Proposal-Based Resolution**: Resolution is proposed, not immediately finalized
//! - **Dispute Integration**: Disputes can only be filed during the open window
//! - **Finalization Gate**: Payouts blocked until window closes and disputes resolved
//!
//! # Resolution Flow
//!
//! ```text
//! Market Ends → Resolution Proposed → Dispute Window Open → Window Closes → Finalized → Payouts
//!                                           ↓
//!                                    Disputes Filed → Disputes Resolved ↗
//! ```
//!
//! # Usage
//!
//! ```rust
//! // Set global dispute window to 48 hours
//! ResolutionDelayManager::set_global_dispute_window(&env, &admin, 48)?;
//!
//! // Propose resolution for a market (opens dispute window)
//! ResolutionDelayManager::propose_resolution(&env, &market_id, outcome, source)?;
//!
//! // Check if window is still open
//! let is_open = ResolutionDelayManager::is_dispute_window_open(&env, &market_id);
//!
//! // After window closes and no disputes, finalize
//! let final_outcome = ResolutionDelayManager::finalize_resolution(&env, &market_id)?;
//! ```

#![allow(dead_code)]

use soroban_sdk::{Address, Env, String, Symbol};

use crate::errors::Error;
use crate::events::EventEmitter;
use crate::markets::MarketStateManager;
use crate::types::{Market, MarketState, ResolutionDelayConfig};

// ===== STORAGE KEYS =====

/// Storage key for global dispute window configuration
const GLOBAL_DISPUTE_WINDOW_KEY: &str = "global_dispute_window";

/// Default dispute window duration in hours
pub const DEFAULT_DISPUTE_WINDOW_HOURS: u32 = 48;

/// Minimum dispute window duration in hours
pub const MIN_DISPUTE_WINDOW_HOURS: u32 = 1;

/// Maximum dispute window duration in hours (1 week)
pub const MAX_DISPUTE_WINDOW_HOURS: u32 = 168;

// ===== RESOLUTION DELAY MANAGER =====

/// Manager for resolution delay and dispute window functionality.
///
/// This struct provides all methods needed to manage the dispute window lifecycle,
/// from configuration through resolution proposal, dispute filing, and finalization.
///
/// # Responsibilities
///
/// - **Configuration**: Set global and per-market dispute window durations
/// - **Proposal**: Open dispute window when resolution is proposed
/// - **Validation**: Check window state for dispute filing and finalization
/// - **Finalization**: Close window and enable payouts when conditions met
///
/// # Security
///
/// - Admin-only configuration changes
/// - Time-locked finalization (cannot finalize during open window)
/// - Dispute blocking (unresolved disputes block finalization)
pub struct ResolutionDelayManager;

impl ResolutionDelayManager {
    // ===== CONFIGURATION =====

    /// Set the global dispute window duration.
    ///
    /// This sets the default dispute window duration that applies to all markets
    /// unless overridden by a per-market configuration.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `admin`: Admin address (must be authenticated)
    /// - `hours`: Window duration in hours (1-168)
    ///
    /// # Errors
    ///
    /// - `Error::Unauthorized`: Caller is not admin
    /// - `Error::InvalidTimeoutHours`: Hours outside valid range
    ///
    /// # Example
    ///
    /// ```rust
    /// // Set global window to 72 hours
    /// ResolutionDelayManager::set_global_dispute_window(&env, &admin, 72)?;
    /// ```
    pub fn set_global_dispute_window(
        env: &Env,
        admin: &Address,
        hours: u32,
    ) -> Result<(), Error> {
        // Validate admin
        Self::validate_admin(env, admin)?;

        // Validate hours
        if hours < MIN_DISPUTE_WINDOW_HOURS || hours > MAX_DISPUTE_WINDOW_HOURS {
            return Err(Error::InvalidTimeoutHours);
        }

        // Create and store config
        let config = ResolutionDelayConfig {
            dispute_window_hours: hours,
            min_dispute_stake: 10_000_000, // 1 XLM default
            auto_finalize_enabled: true,
        };

        env.storage().persistent().set(
            &Symbol::new(env, GLOBAL_DISPUTE_WINDOW_KEY),
            &config,
        );

        Ok(())
    }

    /// Set the dispute window duration for a specific market.
    ///
    /// This overrides the global setting for this market only.
    /// Set to 0 to use the global default.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `admin`: Admin address (must be authenticated)
    /// - `market_id`: Market to configure
    /// - `hours`: Window duration in hours (0 for global, 1-168 for override)
    ///
    /// # Errors
    ///
    /// - `Error::Unauthorized`: Caller is not admin
    /// - `Error::MarketNotFound`: Market doesn't exist
    /// - `Error::InvalidTimeoutHours`: Hours outside valid range
    pub fn set_market_dispute_window(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        hours: u32,
    ) -> Result<(), Error> {
        // Validate admin
        Self::validate_admin(env, admin)?;

        // Validate hours (0 is valid = use global)
        if hours != 0 && (hours < MIN_DISPUTE_WINDOW_HOURS || hours > MAX_DISPUTE_WINDOW_HOURS) {
            return Err(Error::InvalidTimeoutHours);
        }

        // Get and update market
        let mut market = MarketStateManager::get_market(env, market_id)?;
        market.dispute_window_hours = hours;
        MarketStateManager::update_market(env, market_id, &market);

        Ok(())
    }

    /// Get the dispute window configuration for a market.
    ///
    /// Returns the per-market config if set, otherwise the global config.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `market_id`: Market to get config for
    ///
    /// # Returns
    ///
    /// The effective `ResolutionDelayConfig` for this market.
    pub fn get_dispute_window_config(env: &Env, market_id: &Symbol) -> ResolutionDelayConfig {
        // Try to get market-specific override
        if let Ok(market) = MarketStateManager::get_market(env, market_id) {
            if market.dispute_window_hours > 0 {
                return ResolutionDelayConfig {
                    dispute_window_hours: market.dispute_window_hours,
                    min_dispute_stake: 10_000_000,
                    auto_finalize_enabled: true,
                };
            }
        }

        // Fall back to global config
        Self::get_global_config(env)
    }

    /// Get the global dispute window configuration.
    pub fn get_global_config(env: &Env) -> ResolutionDelayConfig {
        env.storage()
            .persistent()
            .get(&Symbol::new(env, GLOBAL_DISPUTE_WINDOW_KEY))
            .unwrap_or_else(|| ResolutionDelayConfig::default_config())
    }

    // ===== RESOLUTION WORKFLOW =====

    /// Propose a resolution for a market, opening the dispute window.
    ///
    /// This function is called after the market ends to propose an outcome.
    /// It opens the dispute window during which community members can file disputes.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `market_id`: Market to propose resolution for
    /// - `outcome`: The proposed winning outcome
    /// - `resolution_source`: Source of the resolution (Oracle, Community, Admin, Hybrid)
    ///
    /// # Errors
    ///
    /// - `Error::MarketNotFound`: Market doesn't exist
    /// - `Error::MarketClosed`: Market hasn't ended yet (still active)
    /// - `Error::MarketAlreadyResolved`: Resolution already finalized
    ///
    /// # Events
    ///
    /// Emits `ResolutionProposedEvent` with window details.
    pub fn propose_resolution(
        env: &Env,
        market_id: &Symbol,
        outcome: String,
        resolution_source: String,
    ) -> Result<(), Error> {
        let mut market = MarketStateManager::get_market(env, market_id)?;

        // Validate market has ended
        let current_time = env.ledger().timestamp();
        if current_time < market.end_time {
            return Err(Error::MarketClosed);
        }

        // Check if already finalized
        if market.resolution_is_finalized {
            return Err(Error::MarketAlreadyResolved);
        }

        // Get dispute window duration for this market
        let config = Self::get_dispute_window_config(env, market_id);
        let window_hours = config.dispute_window_hours;
        let window_seconds = (window_hours as u64) * 60 * 60;
        let window_end_time = current_time + window_seconds;

        // Store the resolution window data directly in market fields
        market.oracle_result = Some(outcome.clone());
        market.resolution_proposed_outcome = Some(outcome.clone());
        market.resolution_proposed_at = current_time;
        market.resolution_window_end_time = window_end_time;
        market.resolution_is_finalized = false;
        market.resolution_dispute_count = 0;
        market.resolution_source = Some(resolution_source.clone());

        // Update market state
        MarketStateManager::update_market(env, market_id, &market);

        // Emit event
        EventEmitter::emit_resolution_proposed(
            env,
            market_id,
            &outcome,
            &resolution_source,
            window_end_time,
            window_hours,
        );

        Ok(())
    }

    /// Check if the dispute window is currently open for a market.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `market_id`: Market to check
    ///
    /// # Returns
    ///
    /// `true` if the window is open and disputes can be filed.
    pub fn is_dispute_window_open(env: &Env, market_id: &Symbol) -> bool {
        if let Ok(market) = MarketStateManager::get_market(env, market_id) {
            let current_time = env.ledger().timestamp();
            market.is_dispute_window_open(current_time)
        } else {
            false
        }
    }

    /// Get the remaining time in the dispute window.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `market_id`: Market to check
    ///
    /// # Returns
    ///
    /// Remaining seconds, or 0 if window is closed or doesn't exist.
    pub fn get_window_remaining_time(env: &Env, market_id: &Symbol) -> u64 {
        if let Ok(market) = MarketStateManager::get_market(env, market_id) {
            if market.resolution_proposed_at > 0 {
                let current_time = env.ledger().timestamp();
                if current_time < market.resolution_window_end_time {
                    return market.resolution_window_end_time - current_time;
                }
            }
        }
        0
    }

    /// Get the dispute window status for a market.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `market_id`: Market to check
    ///
    /// # Returns
    ///
    /// Tuple of (is_open, remaining_seconds, dispute_count)
    pub fn get_dispute_window_status(env: &Env, market_id: &Symbol) -> (bool, u64, u32) {
        if let Ok(market) = MarketStateManager::get_market(env, market_id) {
            let current_time = env.ledger().timestamp();
            if market.resolution_proposed_at > 0 {
                let is_open = market.is_dispute_window_open(current_time);
                let remaining = if current_time < market.resolution_window_end_time {
                    market.resolution_window_end_time - current_time
                } else {
                    0
                };
                return (is_open, remaining, market.resolution_dispute_count);
            }
        }
        (false, 0, 0)
    }

    // ===== FINALIZATION =====

    /// Finalize the resolution after the dispute window closes.
    ///
    /// This function finalizes the resolution, making the outcome permanent and
    /// enabling payouts. It can only be called after the window closes and all
    /// disputes are resolved.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `market_id`: Market to finalize
    ///
    /// # Returns
    ///
    /// The final winning outcome.
    ///
    /// # Errors
    ///
    /// - `Error::MarketNotFound`: Market doesn't exist
    /// - `Error::MarketNotResolved`: No resolution has been proposed
    /// - `Error::DisputeTimeoutNotExpired`: Window hasn't closed yet
    /// - `Error::DisputeResolutionConditionsNotMet`: There are unresolved disputes
    /// - `Error::MarketAlreadyResolved`: Already finalized
    ///
    /// # Events
    ///
    /// Emits `DisputeWindowClosedEvent` and `ResolutionFinalizedEvent`.
    pub fn finalize_resolution(env: &Env, market_id: &Symbol) -> Result<String, Error> {
        let mut market = MarketStateManager::get_market(env, market_id)?;
        let current_time = env.ledger().timestamp();

        // Check if resolution was proposed
        if market.resolution_proposed_at == 0 {
            return Err(Error::MarketNotResolved);
        }

        // Check if already finalized
        if market.resolution_is_finalized {
            return Err(Error::MarketAlreadyResolved);
        }

        // Check if window has closed
        if market.is_dispute_window_open(current_time) {
            return Err(Error::DisputeTimeoutNotExpired);
        }

        // Check for unresolved disputes
        let has_unresolved = Self::has_unresolved_disputes(env, market_id);
        if has_unresolved {
            return Err(Error::DisputeResolutionConditionsNotMet);
        }

        // Get resolution data
        let proposed_outcome = market.resolution_proposed_outcome.clone()
            .unwrap_or_else(|| String::from_str(env, "unknown"));
        let dispute_count = market.resolution_dispute_count;
        let was_disputed = dispute_count > 0;

        // Emit window closed event
        EventEmitter::emit_dispute_window_closed(
            env,
            market_id,
            dispute_count,
            false, // no unresolved disputes
        );

        // Finalize the resolution
        market.resolution_is_finalized = true;

        // Set the winning outcome
        market.winning_outcome = Some(proposed_outcome.clone());
        market.state = MarketState::Resolved;

        // Update market
        MarketStateManager::update_market(env, market_id, &market);

        // Emit finalized event
        EventEmitter::emit_resolution_finalized(
            env,
            market_id,
            &proposed_outcome,
            was_disputed,
            dispute_count,
        );

        Ok(proposed_outcome)
    }

    /// Check if the resolution can be finalized.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `market_id`: Market to check
    ///
    /// # Returns
    ///
    /// Tuple of (can_finalize, reason)
    pub fn can_finalize(env: &Env, market_id: &Symbol) -> (bool, String) {
        let market = match MarketStateManager::get_market(env, market_id) {
            Ok(m) => m,
            Err(_) => return (false, String::from_str(env, "Market not found")),
        };

        if market.resolution_proposed_at == 0 {
            return (false, String::from_str(env, "Resolution not proposed"));
        }

        if market.resolution_is_finalized {
            return (false, String::from_str(env, "Already finalized"));
        }

        let current_time = env.ledger().timestamp();
        if market.is_dispute_window_open(current_time) {
            return (false, String::from_str(env, "Window still open"));
        }

        if Self::has_unresolved_disputes(env, market_id) {
            return (false, String::from_str(env, "Unresolved disputes"));
        }

        (true, String::from_str(env, "Ready to finalize"))
    }

    /// Check if a resolution is finalized.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `market_id`: Market to check
    ///
    /// # Returns
    ///
    /// `true` if the resolution is finalized and payouts are enabled.
    pub fn is_resolution_finalized(env: &Env, market_id: &Symbol) -> bool {
        if let Ok(market) = MarketStateManager::get_market(env, market_id) {
            return market.is_resolution_finalized();
        }
        false
    }

    // ===== DISPUTE INTEGRATION =====

    /// Record that a dispute was filed during the window.
    ///
    /// This increments the dispute count for the resolution window.
    /// Called by the dispute system when a dispute is successfully created.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `market_id`: Market where dispute was filed
    ///
    /// # Errors
    ///
    /// - `Error::MarketNotFound`: Market doesn't exist
    /// - `Error::DisputeVotingNotAllowed`: Window is not open for disputes
    pub fn record_dispute(env: &Env, market_id: &Symbol) -> Result<(), Error> {
        let mut market = MarketStateManager::get_market(env, market_id)?;
        let current_time = env.ledger().timestamp();

        // Check if window is open
        if !market.is_dispute_window_open(current_time) {
            return Err(Error::DisputeVotingNotAllowed);
        }

        // Increment dispute count
        market.resolution_dispute_count += 1;

        MarketStateManager::update_market(env, market_id, &market);
        Ok(())
    }

    /// Check if there are unresolved disputes for a market.
    ///
    /// This function checks if any disputes filed during the window are still
    /// pending resolution. If so, finalization is blocked.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `market_id`: Market to check
    ///
    /// # Returns
    ///
    /// `true` if there are unresolved disputes.
    pub fn has_unresolved_disputes(env: &Env, market_id: &Symbol) -> bool {
        // Check market's dispute stakes - if there are active dispute stakes,
        // there are unresolved disputes
        if let Ok(market) = MarketStateManager::get_market(env, market_id) {
            // If the market state is Disputed, there are unresolved disputes
            if market.state == MarketState::Disputed {
                return true;
            }

            // Also check if dispute_stakes has any entries with non-zero stakes
            let total_dispute_stakes = market.total_dispute_stakes();
            if total_dispute_stakes > 0 {
                // Need to check if these disputes are resolved
                // For now, we check if market state indicates active dispute
                return market.state == MarketState::Disputed;
            }
        }
        false
    }

    /// Validate that a dispute can be filed for this market.
    ///
    /// Checks that the dispute window is open and resolution has been proposed.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `market_id`: Market to validate
    ///
    /// # Errors
    ///
    /// - `Error::MarketNotResolved`: No resolution proposed
    /// - `Error::DisputeVotingNotAllowed`: Window is closed
    /// - `Error::MarketAlreadyResolved`: Resolution already finalized
    pub fn validate_dispute_allowed(env: &Env, market_id: &Symbol) -> Result<(), Error> {
        let market = MarketStateManager::get_market(env, market_id)?;
        let current_time = env.ledger().timestamp();

        // Check if resolution was proposed
        if market.resolution_proposed_at == 0 {
            return Err(Error::MarketNotResolved);
        }

        // Check if already finalized
        if market.resolution_is_finalized {
            return Err(Error::MarketAlreadyResolved);
        }

        // Check if window is open
        if !market.is_dispute_window_open(current_time) {
            return Err(Error::DisputeVotingNotAllowed);
        }

        Ok(())
    }

    // ===== ADMIN UTILITIES =====

    /// Force finalize a resolution (admin override).
    ///
    /// This allows an admin to finalize a resolution even if there are
    /// unresolved disputes. Should only be used in emergency situations.
    ///
    /// # Parameters
    ///
    /// - `env`: Soroban environment
    /// - `admin`: Admin address (must be authenticated)
    /// - `market_id`: Market to force finalize
    /// - `outcome`: The final outcome to set (can override proposed outcome)
    ///
    /// # Errors
    ///
    /// - `Error::Unauthorized`: Caller is not admin
    /// - `Error::MarketNotFound`: Market doesn't exist
    pub fn force_finalize(
        env: &Env,
        admin: &Address,
        market_id: &Symbol,
        outcome: String,
    ) -> Result<(), Error> {
        // Validate admin
        Self::validate_admin(env, admin)?;

        let mut market = MarketStateManager::get_market(env, market_id)?;
        let current_time = env.ledger().timestamp();

        // Set up resolution data if not already set
        if market.resolution_proposed_at == 0 {
            market.resolution_proposed_outcome = Some(outcome.clone());
            market.resolution_proposed_at = current_time;
            market.resolution_window_end_time = current_time;
            market.resolution_dispute_count = 0;
            market.resolution_source = Some(String::from_str(env, "AdminOverride"));
        }

        // Finalize
        market.resolution_is_finalized = true;

        // Set winning outcome
        market.winning_outcome = Some(outcome.clone());
        market.state = MarketState::Resolved;

        MarketStateManager::update_market(env, market_id, &market);

        // Emit finalized event
        EventEmitter::emit_resolution_finalized(
            env,
            market_id,
            &outcome,
            false, // Admin override, not disputed
            0,
        );

        Ok(())
    }

    // ===== PRIVATE HELPERS =====

    /// Validate that the caller is an admin.
    fn validate_admin(env: &Env, admin: &Address) -> Result<(), Error> {
        let stored_admin: Option<Address> = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "Admin"));

        match stored_admin {
            Some(stored) if stored == *admin => Ok(()),
            _ => Err(Error::Unauthorized),
        }
    }
}

// ===== MODULE TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    fn setup_test_env() -> (Env, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);

        // Set up admin in storage
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "Admin"), &admin);

        (env, admin)
    }

    #[test]
    fn test_set_global_dispute_window() {
        let (env, admin) = setup_test_env();

        // Set global window to 72 hours
        let result = ResolutionDelayManager::set_global_dispute_window(&env, &admin, 72);
        assert!(result.is_ok());

        // Verify config
        let config = ResolutionDelayManager::get_global_config(&env);
        assert_eq!(config.dispute_window_hours, 72);
    }

    #[test]
    fn test_invalid_window_hours() {
        let (env, admin) = setup_test_env();

        // 0 hours should fail
        let result = ResolutionDelayManager::set_global_dispute_window(&env, &admin, 0);
        assert_eq!(result, Err(Error::InvalidTimeoutHours));

        // 200 hours should fail (> 168)
        let result = ResolutionDelayManager::set_global_dispute_window(&env, &admin, 200);
        assert_eq!(result, Err(Error::InvalidTimeoutHours));
    }

    #[test]
    fn test_default_config() {
        let env = Env::default();
        let config = ResolutionDelayManager::get_global_config(&env);

        assert_eq!(config.dispute_window_hours, DEFAULT_DISPUTE_WINDOW_HOURS);
        assert_eq!(config.min_dispute_stake, 10_000_000);
        assert!(config.auto_finalize_enabled);
    }
}
