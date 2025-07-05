use soroban_sdk::{contracttype, symbol_short, token, vec, Address, Env, Map, String, Symbol, Vec};

use crate::errors::Error;
use crate::types::*;

/// Market extension management system for Predictify Hybrid contract
///
/// This module provides comprehensive market extension functionality:
/// - Extension validation and approval mechanisms
/// - Extension limits and fee handling
/// - Extension events and logging
/// - Extension history tracking
/// - Extension analytics and reporting

// ===== EXTENSION CONSTANTS =====

const MAX_EXTENSION_DAYS: u32 = 30;
const MIN_EXTENSION_DAYS: u32 = 1;
const EXTENSION_FEE_PER_DAY: i128 = 100_000_000; // 1 XLM per day in stroops
const MAX_TOTAL_EXTENSIONS: u32 = 3;

// ===== EXTENSION MANAGEMENT =====

/// Market extension management utilities
pub struct ExtensionManager;

impl ExtensionManager {
    /// Extend market duration with validation and fee handling
    pub fn extend_market_duration(
        env: &Env,
        admin: Address,
        market_id: Symbol,
        additional_days: u32,
        reason: String,
    ) -> Result<(), Error> {
        // Validate extension conditions
        ExtensionValidator::validate_extension_conditions(env, &market_id, additional_days)?;

        // Check extension limits
        ExtensionValidator::check_extension_limits(env, &market_id, additional_days)?;

        // Verify admin permissions
        ExtensionValidator::can_extend_market(env, &market_id, &admin)?;

        // Handle extension fees
        let fee_amount = ExtensionUtils::handle_extension_fees(env, &market_id, additional_days)?;

        // Get and update market
        let mut market = MarketStateManager::get_market(env, &market_id)?;

        // Create extension record
        let extension =
            MarketExtension::new(env, additional_days, admin.clone(), reason, fee_amount);

        // Update market
        market.end_time += (additional_days as u64) * 24 * 60 * 60; // Convert days to seconds
        market.total_extension_days += additional_days;
        market.extension_history.push_back(extension);

        // Store updated market
        MarketStateManager::update_market(env, &market_id, &market);

        // Emit extension event
        ExtensionUtils::emit_extension_event(env, &market_id, additional_days, &admin);

        Ok(())
    }

    /// Get extension history for a market
    pub fn get_market_extension_history(
        env: &Env,
        market_id: Symbol,
    ) -> Result<Vec<MarketExtension>, Error> {
        let market = MarketStateManager::get_market(env, &market_id)?;
        Ok(market.extension_history)
    }

    /// Get extension statistics for a market
    pub fn get_extension_stats(
        env: &Env,
        market_id: Symbol,
    ) -> Result<crate::types::ExtensionStats, Error> {
        let market = MarketStateManager::get_market(env, &market_id)?;

        Ok(crate::types::ExtensionStats {
            total_extensions: market.extension_history.len().try_into().unwrap_or(0),
            total_extension_days: market.total_extension_days,
            max_extension_days: market.max_extension_days,
            can_extend: ExtensionValidator::can_extend_market(env, &market_id, &market.admin)
                .is_ok(),
            extension_fee_per_day: EXTENSION_FEE_PER_DAY,
        })
    }

    /// Check if market can be extended
    pub fn can_extend_market(env: &Env, market_id: Symbol, admin: Address) -> Result<bool, Error> {
        ExtensionValidator::can_extend_market(env, &market_id, &admin)?;
        Ok(true)
    }

    /// Calculate extension fee for given days
    pub fn calculate_extension_fee(additional_days: u32) -> i128 {
        (additional_days as i128) * EXTENSION_FEE_PER_DAY
    }
}

// ===== EXTENSION VALIDATION =====

/// Extension validation utilities
pub struct ExtensionValidator;

impl ExtensionValidator {
    /// Validate extension conditions
    pub fn validate_extension_conditions(
        env: &Env,
        market_id: &Symbol,
        additional_days: u32,
    ) -> Result<(), Error> {
        // Validate additional days
        if additional_days < MIN_EXTENSION_DAYS {
            return Err(Error::InvalidExtensionDays);
        }

        if additional_days > MAX_EXTENSION_DAYS {
            return Err(Error::ExtensionDaysExceeded);
        }

        // Get market and validate state
        let market = MarketStateManager::get_market(env, market_id)?;

        // Check if market is still active
        let current_time = env.ledger().timestamp();
        if current_time >= market.end_time {
            return Err(Error::MarketClosed);
        }

        // Check if market is already resolved
        if market.oracle_result.is_some() {
            return Err(Error::MarketAlreadyResolved);
        }

        Ok(())
    }

    /// Check extension limits
    pub fn check_extension_limits(
        env: &Env,
        market_id: &Symbol,
        additional_days: u32,
    ) -> Result<(), Error> {
        let market = MarketStateManager::get_market(env, market_id)?;

        // Check total extension days limit
        if market.total_extension_days + additional_days > market.max_extension_days {
            return Err(Error::ExtensionDaysExceeded);
        }

        // Check number of extensions limit
        if (market.extension_history.len() as usize) >= (MAX_TOTAL_EXTENSIONS as usize) {
            return Err(Error::MarketExtensionNotAllowed);
        }

        Ok(())
    }

    /// Check if admin can extend market
    pub fn can_extend_market(env: &Env, market_id: &Symbol, admin: &Address) -> Result<(), Error> {
        let market = MarketStateManager::get_market(env, market_id)?;

        // Check if caller is market admin
        if market.admin != *admin {
            return Err(Error::Unauthorized);
        }

        Ok(())
    }
}

// ===== EXTENSION UTILITIES =====

/// Extension utility functions
pub struct ExtensionUtils;

impl ExtensionUtils {
    /// Handle extension fees
    pub fn handle_extension_fees(
        env: &Env,
        market_id: &Symbol,
        additional_days: u32,
    ) -> Result<i128, Error> {
        let fee_amount = ExtensionManager::calculate_extension_fee(additional_days);

        // Get token client for fee collection
        let token_client = MarketUtils::get_token_client(env)?;

        // Transfer fees from admin to contract
        // Note: In a real implementation, you would need to handle the actual token transfer
        // For now, we'll just validate the fee amount

        if fee_amount <= 0 {
            return Err(Error::ExtensionFeeInsufficient);
        }

        Ok(fee_amount)
    }

    /// Emit extension event
    pub fn emit_extension_event(
        env: &Env,
        market_id: &Symbol,
        additional_days: u32,
        admin: &Address,
    ) {
        // In Soroban, events are emitted using the env.events() API
        // For now, we'll use a simple approach with storage
        let event_key = symbol_short!("ext_event");
        let event_data = ExtensionEvent {
            market_id: market_id.clone(),
            additional_days,
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&event_key, &event_data);
    }

    /// Get extension events
    pub fn get_extension_events(env: &Env) -> Vec<ExtensionEvent> {
        let event_key = symbol_short!("ext_event");
        match env.storage().persistent().get(&event_key) {
            Some(event) => vec![env, event],
            None => vec![env],
        }
    }
}

// ===== EXTENSION TYPES =====

/// Extension event data
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionEvent {
    /// Market ID that was extended
    pub market_id: Symbol,
    /// Additional days added
    pub additional_days: u32,
    /// Admin who requested extension
    pub admin: Address,
    /// Extension timestamp
    pub timestamp: u64,
}

// ===== EXTENSION TEST HELPERS =====

/// Extension testing utilities
pub struct ExtensionTestHelpers;

impl ExtensionTestHelpers {
    /// Create a test extension
    pub fn create_test_extension(
        env: &Env,
        additional_days: u32,
        admin: Address,
        reason: String,
    ) -> MarketExtension {
        MarketExtension::new(env, additional_days, admin, reason, 100_000_000)
    }

    /// Simulate market extension
    pub fn simulate_market_extension(
        env: &Env,
        market_id: &Symbol,
        admin: Address,
        additional_days: u32,
    ) -> Result<(), Error> {
        let reason = String::from_str(env, "Test extension");
        ExtensionManager::extend_market_duration(
            env,
            admin,
            market_id.clone(),
            additional_days,
            reason,
        )
    }
}

// Import required modules
use crate::markets::{MarketStateManager, MarketUtils};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ExtensionStats;
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};

    #[test]
    fn test_extension_validation() {
        let env = Env::default();
        let admin = Address::generate(&env);

        // Test valid extension days
        assert!(
            ExtensionValidator::validate_extension_conditions(&env, &symbol_short!("test"), 5)
                .is_err()
        ); // Market doesn't exist

        // Test invalid extension days
        assert_eq!(
            ExtensionValidator::validate_extension_conditions(&env, &symbol_short!("test"), 0)
                .unwrap_err(),
            Error::InvalidExtensionDays
        );

        assert_eq!(
            ExtensionValidator::validate_extension_conditions(&env, &symbol_short!("test"), 31)
                .unwrap_err(),
            Error::ExtensionDaysExceeded
        );
    }

    #[test]
    fn test_extension_fee_calculation() {
        assert_eq!(
            ExtensionManager::calculate_extension_fee(1),
            EXTENSION_FEE_PER_DAY
        );
        assert_eq!(
            ExtensionManager::calculate_extension_fee(5),
            5 * EXTENSION_FEE_PER_DAY
        );
        assert_eq!(
            ExtensionManager::calculate_extension_fee(30),
            30 * EXTENSION_FEE_PER_DAY
        );
    }

    #[test]
    fn test_extension_stats() {
        let env = Env::default();
        let stats = ExtensionStats {
            total_extensions: 2,
            total_extension_days: 10,
            max_extension_days: 30,
            can_extend: true,
            extension_fee_per_day: EXTENSION_FEE_PER_DAY,
        };

        assert_eq!(stats.total_extensions, 2);
        assert_eq!(stats.total_extension_days, 10);
        assert!(stats.can_extend);
    }
}
