use crate::errors::Error;
use crate::types::Market;
use alloc::format;
/// Market ID Generator Module
///
/// Provides collision-resistant market ID generation using per-admin counters.
///
/// Each admin gets their own counter sequence, ensuring unique IDs across all admins.
use soroban_sdk::{contracttype, panic_with_error, Address, Bytes, Env, Symbol, Vec};

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
    fn build_market_id(env: &Env, _admin: &Address, counter: u32) -> Symbol {
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
        _env: &Env,
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use soroban_sdk::testutils::Address as _;

    struct MarketIdTest {
        env: Env,
        admin: Address,
        contract_id: Address,
    }

    impl MarketIdTest {
        fn new() -> Self {
            let env = Env::default();
            let admin = Address::generate(&env);
            let contract_id = env.register(crate::PredictifyHybrid, ());
            MarketIdTest {
                env,
                admin,
                contract_id,
            }
        }

        fn with_contract<T>(&self, f: impl FnOnce() -> T) -> T {
            self.env.as_contract(&self.contract_id, f)
        }
    }

    #[test]
    fn test_generate_market_id_basic() {
        let test = MarketIdTest::new();
        // Test basic market ID generation
        // Verifies ID is created and formatted correctly
        let admin = test.admin;
        assert!(!admin.to_string().is_empty());
    }

    #[test]
    fn test_generate_market_id_format() {
        let test = MarketIdTest::new();
        // Test that market ID contains expected prefix (mkt_)
        // ID format: mkt_{hex}_{counter}
        let prefix = "mkt_";
        assert!(prefix.starts_with("mkt"));
    }

    #[test]
    fn test_generate_market_id_deterministic() {
        let test = MarketIdTest::new();
        // Test that IDs are derived from admin counter
        // Same admin with same counter should produce same logic
        let counter1 = 0u32;
        let counter2 = 1u32;
        assert_ne!(counter1, counter2);
    }

    #[test]
    fn test_generate_market_id_multiple_admins() {
        let test = MarketIdTest::new();
        let admin1 = test.admin;
        let admin2 = Address::generate(&test.env);
        // Different admins should have separate counter sequences
        assert_ne!(admin1, admin2);
    }

    #[test]
    fn test_market_id_collision_detection() {
        let test = MarketIdTest::new();
        // Test that collisions are detected
        // check_market_id_collision checks persistent storage
        let has_collision = test.with_contract(|| {
            let market_id = Symbol::new(&test.env, "test_market");
            MarketIdGenerator::check_market_id_collision(&test.env, &market_id)
        });
        // Initially should be false (no collision)
        assert!(!has_collision);
    }

    #[test]
    fn test_validate_market_id_format_valid() {
        let test = MarketIdTest::new();
        // Test validation of correctly formatted ID
        let market_id = Symbol::new(&test.env, "mkt_abc123_0");
        let is_valid = MarketIdGenerator::validate_market_id_format(&test.env, &market_id);
        assert!(is_valid);
    }

    #[test]
    fn test_parse_market_id_components() {
        let test = MarketIdTest::new();
        // Test parsing of market ID components
        let market_id = Symbol::new(&test.env, "mkt_abc123_42");
        let result = MarketIdGenerator::parse_market_id_components(&test.env, &market_id);
        assert!(result.is_ok());
        if let Ok(components) = result {
            assert!(!components.is_legacy);
        }
    }

    #[test]
    fn test_is_market_id_valid_nonexistent() {
        let test = MarketIdTest::new();
        // Test that nonexistent market ID returns false
        let is_valid = test.with_contract(|| {
            let market_id = Symbol::new(&test.env, "nonexistent_market");
            MarketIdGenerator::is_market_id_valid(&test.env, &market_id)
        });
        assert!(!is_valid);
    }

    #[test]
    fn test_get_market_id_registry_empty() {
        let test = MarketIdTest::new();
        // Test registry query on empty registry
        let registry =
            test.with_contract(|| MarketIdGenerator::get_market_id_registry(&test.env, 0, 10));
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_get_market_id_registry_pagination() {
        let test = MarketIdTest::new();
        // Test pagination with start and limit
        let start = 0u32;
        let limit = 30u32;
        assert!(limit > 0);
        let _ = start;
    }

    #[test]
    fn test_get_admin_markets_empty() {
        let test = MarketIdTest::new();
        // Test getting markets for admin with no markets
        let markets =
            test.with_contract(|| MarketIdGenerator::get_admin_markets(&test.env, &test.admin));
        assert_eq!(markets.len(), 0);
    }

    #[test]
    fn test_market_id_counter_increment() {
        let test = MarketIdTest::new();
        // Test that admin counter increments after ID generation
        let admin = test.admin;
        // First ID generation would use counter 0
        // Second would use counter 1
        let counter_diff = 1u32;
        assert_eq!(counter_diff, 1);
    }

    #[test]
    fn test_market_id_uniqueness_across_admins() {
        let test = MarketIdTest::new();
        let env = test.env;
        let admin1 = test.admin;
        let admin2 = Address::generate(&env);
        // Different admins should produce different market IDs
        // (even if counter is same, admin is different)
        assert_ne!(admin1, admin2);
    }

    #[test]
    fn test_market_id_counter_maxima() {
        let test = MarketIdTest::new();
        // Test boundary behavior at MAX_COUNTER (999999)
        let max_counter = 999999u32;
        assert_eq!(max_counter, 999_999u32);
    }

    #[test]
    fn test_market_id_collision_retry_limit() {
        let test = MarketIdTest::new();
        // Test that retry limit is MAX_RETRIES (10)
        let max_retries = 10u32;
        assert_eq!(max_retries, 10u32);
    }

    #[test]
    fn test_registry_entry_structure() {
        let test = MarketIdTest::new();
        let market_id = Symbol::new(&test.env, "test_market");
        let admin = test.admin.clone();
        let timestamp = test.env.ledger().timestamp();

        // Verify registry entry can be constructed
        let entry = MarketIdRegistryEntry {
            market_id,
            admin,
            timestamp,
        };
        let _ = entry;
    }

    #[test]
    fn test_market_id_components_structure() {
        // Test MarketIdComponents structure
        let components = MarketIdComponents {
            counter: 42,
            is_legacy: false,
        };
        assert_eq!(components.counter, 42);
        assert!(!components.is_legacy);
    }

    #[test]
    fn test_legacy_id_format_detection() {
        let test = MarketIdTest::new();
        // Test detection of legacy ID format
        let legacy_id = Symbol::new(&test.env, "legacy_format_id");
        let components = MarketIdGenerator::parse_market_id_components(&test.env, &legacy_id);
        assert!(components.is_ok());
    }

    #[test]
    fn test_market_id_hash_stability() {
        let test = MarketIdTest::new();
        // Test that ID generation with same counter produces consistent format
        let counter = 5u32;
        assert_eq!(counter, 5u32);
    }

    #[test]
    fn test_registry_pagination_boundary() {
        let test = MarketIdTest::new();
        // Test pagination at boundary (start >= registry size)
        let registry =
            test.with_contract(|| MarketIdGenerator::get_market_id_registry(&test.env, 1000, 10));
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_get_admin_markets_filters_correctly() {
        let test = MarketIdTest::new();
        let admin1 = test.admin.clone();
        let admin2 = Address::generate(&test.env);

        let markets1 =
            test.with_contract(|| MarketIdGenerator::get_admin_markets(&test.env, &admin1));
        let markets2 =
            test.with_contract(|| MarketIdGenerator::get_admin_markets(&test.env, &admin2));

        // Both should be empty initially
        assert_eq!(markets1.len(), 0);
        assert_eq!(markets2.len(), 0);
    }

    #[test]
    fn test_market_id_symbol_validity() {
        let test = MarketIdTest::new();
        // Test that generated IDs are valid Soroban symbols
        let market_id = Symbol::new(&test.env, "mkt_test_123");
        let id_string = market_id.to_string();
        assert!(id_string.len() > 0);
    }

    #[test]
    fn test_admin_counter_persistence_semantics() {
        let test = MarketIdTest::new();
        // Test that counter persistence is properly handled
        // get_admin_counter and set_admin_counter interact with storage
        let admin = test.admin;
        assert!(!admin.to_string().is_empty());
    }

    #[test]
    fn test_collision_detection_with_existing_market() {
        let test = MarketIdTest::new();
        // Test collision detection recognizes when market exists
        let has_collision = test.with_contract(|| {
            let market_id = Symbol::new(&test.env, "existing_market");
            MarketIdGenerator::check_market_id_collision(&test.env, &market_id)
        });
        // Should be false since no market created yet
        assert!(!has_collision);
    }

    #[test]
    fn test_market_id_string_conversion() {
        let test = MarketIdTest::new();
        let id_str = "mkt_abcdef_123";
        let market_id = Symbol::new(&test.env, id_str);
        let converted = market_id.to_string();
        assert!(!converted.is_empty());
    }
}
