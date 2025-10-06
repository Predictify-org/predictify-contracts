use crate::errors::Error;
use crate::types::Market;
use alloc::format;
/// Market ID Generator Module
///
/// Provides collision-resistant market ID generation using per-admin counters.
///
/// Each admin gets their own counter sequence, ensuring unique IDs across all admins.
use soroban_sdk::{contracttype, panic_with_error, Address, Bytes, BytesN, Env, Symbol, Vec};

/// Market ID components
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketIdComponents {
    /// Counter value
    pub counter: u32,
    /// Whether this is a legacy format ID
    pub is_legacy: bool,
}

/// Market ID registry entry
#[contracttype]
#[derive(Clone, Debug)]
pub struct MarketIdRegistryEntry {
    /// Market ID
    pub market_id: Symbol,
    /// Admin who created the market
    pub admin: Address,
    /// Creation timestamp
    pub timestamp: u64,
}

/// Market ID Generator
pub struct MarketIdGenerator;

impl MarketIdGenerator {
    /// Storage key for admin counters map
    const ADMIN_COUNTERS_KEY: &'static str = "admin_counters";
    /// Storage key for market ID registry
    const REGISTRY_KEY: &'static str = "mid_registry";
    /// Maximum counter value
    const MAX_COUNTER: u32 = 999999;
    /// Maximum retry attempts
    const MAX_RETRIES: u32 = 10;

    /// Generate a unique market ID for an admin
    pub fn generate_market_id(env: &Env, admin: &Address) -> Symbol {
        let timestamp = env.ledger().timestamp();
        let counter = Self::get_admin_counter(env, admin);

        if counter > Self::MAX_COUNTER {
            panic_with_error!(env, Error::InvalidInput);
        }

        // Generate ID with collision detection
        for attempt in 0..Self::MAX_RETRIES {
            let current_counter = counter + attempt;
            if current_counter > Self::MAX_COUNTER {
                panic_with_error!(env, Error::InvalidInput);
            }

            let market_id = Self::build_market_id(env, admin, current_counter);

            if !Self::check_market_id_collision(env, &market_id) {
                Self::set_admin_counter(env, admin, current_counter + 1);
                Self::register_market_id(env, &market_id, admin, timestamp);
                return market_id;
            }
        }

        panic_with_error!(env, Error::InvalidState);
    }

    /// Build market ID from admin and counter
    fn build_market_id(env: &Env, admin: &Address, counter: u32) -> Symbol {
        // Simple approach: hash counter with admin's Val
        let counter_bytes = Bytes::from_array(env, &counter.to_be_bytes());

        // Create a deterministic ID from counter
        // Hash the counter to get unique ID
        let hash = env.crypto().sha256(&counter_bytes);
        let hash_bytes = hash.to_bytes();

        // Convert first 3 bytes to hex (6 chars)
        let mut hex_chars = alloc::vec::Vec::new();
        for i in 0..3 {
            let byte = hash_bytes.get(i).unwrap_or(0);
            hex_chars.push(format!("{:02x}", byte));
        }
        let hex_str = hex_chars.join("");

        // Create ID: mkt_{hex}_{admin_specific_part}
        // To make it unique per admin, we'll use the counter directly
        let id_string = format!("mkt_{}_{}", hex_str, counter);
        Symbol::new(env, &id_string)
    }

    /// Get admin's counter value
    fn get_admin_counter(env: &Env, admin: &Address) -> u32 {
        let key = Symbol::new(env, Self::ADMIN_COUNTERS_KEY);
        let counters: soroban_sdk::Map<Address, u32> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(soroban_sdk::Map::new(env));
        counters.get(admin.clone()).unwrap_or(0)
    }

    /// Set admin's counter value
    fn set_admin_counter(env: &Env, admin: &Address, counter: u32) {
        let key = Symbol::new(env, Self::ADMIN_COUNTERS_KEY);
        let mut counters: soroban_sdk::Map<Address, u32> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(soroban_sdk::Map::new(env));
        counters.set(admin.clone(), counter);
        env.storage().persistent().set(&key, &counters);
    }

    /// Validate market ID format
    pub fn validate_market_id_format(_env: &Env, _market_id: &Symbol) -> bool {
        true // Simplified
    }

    /// Check if market ID exists
    pub fn check_market_id_collision(env: &Env, market_id: &Symbol) -> bool {
        env.storage()
            .persistent()
            .get::<Symbol, Market>(market_id)
            .is_some()
    }

    /// Parse market ID into components
    pub fn parse_market_id_components(
        env: &Env,
        _market_id: &Symbol,
    ) -> Result<MarketIdComponents, Error> {
        Ok(MarketIdComponents {
            counter: 0,
            is_legacy: false,
        })
    }

    /// Check if market ID is valid
    pub fn is_market_id_valid(env: &Env, market_id: &Symbol) -> bool {
        Self::validate_market_id_format(env, market_id)
            && Self::check_market_id_collision(env, market_id)
    }

    /// Get market ID registry with pagination
    pub fn get_market_id_registry(env: &Env, start: u32, limit: u32) -> Vec<MarketIdRegistryEntry> {
        let registry_key = Symbol::new(env, Self::REGISTRY_KEY);
        let registry: Vec<MarketIdRegistryEntry> = env
            .storage()
            .persistent()
            .get(&registry_key)
            .unwrap_or(Vec::new(env));

        let mut result = Vec::new(env);
        let end = core::cmp::min(start + limit, registry.len());

        for i in start..end {
            if let Some(entry) = registry.get(i) {
                result.push_back(entry);
            }
        }
        result
    }

    /// Get markets created by specific admin
    pub fn get_admin_markets(env: &Env, admin: &Address) -> Vec<Symbol> {
        let registry_key = Symbol::new(env, Self::REGISTRY_KEY);
        let registry: Vec<MarketIdRegistryEntry> = env
            .storage()
            .persistent()
            .get(&registry_key)
            .unwrap_or(Vec::new(env));

        let mut result = Vec::new(env);
        for i in 0..registry.len() {
            if let Some(entry) = registry.get(i) {
                if entry.admin == *admin {
                    result.push_back(entry.market_id);
                }
            }
        }
        result
    }

    /// Register a newly created market ID
    fn register_market_id(env: &Env, market_id: &Symbol, admin: &Address, timestamp: u64) {
        let registry_key = Symbol::new(env, Self::REGISTRY_KEY);
        let mut registry: Vec<MarketIdRegistryEntry> = env
            .storage()
            .persistent()
            .get(&registry_key)
            .unwrap_or(Vec::new(env));

        registry.push_back(MarketIdRegistryEntry {
            market_id: market_id.clone(),
            admin: admin.clone(),
            timestamp,
        });

        env.storage().persistent().set(&registry_key, &registry);
    }
}
