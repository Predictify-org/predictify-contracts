//! Market ID Generator
//!
//! Generates collision-resistant market IDs for the Predictify Hybrid contract.
//!
//! # Entropy sources
//!
//! Each ID is derived from a SHA-256 digest of two independent inputs:
//!
//! | Source | Bytes | Notes |
//! |--------|-------|-------|
//! | Ledger sequence | 4 | Monotonically increasing; unique per ledger |
//! | Global nonce | 4 | Monotonically increasing across all markets |
//!
//! The global nonce increments on every call regardless of admin, so two
//! admins calling `generate_market_id` in the same ledger still produce
//! different IDs.  The per-admin counter is tracked separately for
//! auditability (it appears in the ID suffix) but is not the sole source
//! of uniqueness.
//!
//! # Collision risk
//!
//! The ID space is the first 8 hex characters of the SHA-256 digest (32 bits).
//! With two independent monotonic inputs the effective pre-image space is
//! `2^32 × N_ledgers`, making accidental collisions negligible in practice.
//! The generator also performs an explicit collision check against persistent
//! storage and retries up to [`MarketIdGenerator::MAX_RETRIES`] times before
//! panicking, providing a hard safety net.
//!
//! # Format
//!
//! ```text
//! mkt_{8 hex chars}_{admin_counter}
//! ```
//!
//! Example: `mkt_3f9a1b2c_0`
//!
//! The counter suffix is the per-admin counter at creation time, making IDs
//! human-readable and auditable without decoding the hash.

use crate::errors::Error;
use crate::types::Market;
use alloc::format;
use alloc::string::ToString;
use soroban_sdk::{contracttype, panic_with_error, Address, Bytes, Env, Symbol, Vec};

// ── Public types ─────────────────────────────────────────────────────────────

/// Parsed components of a market ID.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketIdComponents {
    /// The per-admin counter embedded in the ID suffix.
    pub counter: u32,
    /// `true` for IDs that pre-date the current format (no `mkt_` prefix).
    pub is_legacy: bool,
}

/// One entry in the market ID registry.
#[contracttype]
#[derive(Clone, Debug)]
pub struct MarketIdRegistryEntry {
    /// The market ID symbol.
    pub market_id: Symbol,
    /// Admin who created the market.
    pub admin: Address,
    /// Ledger timestamp at creation time.
    pub timestamp: u64,
}

// ── Generator ────────────────────────────────────────────────────────────────

/// Stateless helper that generates and validates market IDs.
pub struct MarketIdGenerator;

impl MarketIdGenerator {
    const ADMIN_COUNTERS_KEY: &'static str = "admin_counters";
    pub(crate) const GLOBAL_NONCE_KEY: &'static str = "mid_nonce";
    const REGISTRY_KEY: &'static str = "mid_registry";
    /// Hard upper bound on the per-admin counter.
    pub const MAX_COUNTER: u32 = 999_999;
    /// Maximum collision-retry attempts before giving up.
    pub const MAX_RETRIES: u32 = 10;

    // ── Public API ───────────────────────────────────────────────────────────

    /// Generate a unique, collision-resistant market ID for `admin`.
    ///
    /// The ID is derived from SHA-256(ledger_sequence ‖ global_nonce) and
    /// formatted as `mkt_{8 hex chars}_{admin_counter}`.
    ///
    /// # Panics
    ///
    /// - [`Error::InvalidInput`] if the admin's counter has reached [`MAX_COUNTER`].
    /// - [`Error::InvalidState`] if [`MAX_RETRIES`] consecutive collision checks
    ///   all find an existing market (should never happen in normal operation).
    pub fn generate_market_id(env: &Env, admin: &Address) -> Symbol {
        let timestamp = env.ledger().timestamp();
        let admin_counter = Self::get_admin_counter(env, admin);

        if admin_counter > Self::MAX_COUNTER {
            panic_with_error!(env, Error::InvalidInput);
        }

        for attempt in 0..Self::MAX_RETRIES {
            let current_admin_counter = admin_counter + attempt;
            if current_admin_counter > Self::MAX_COUNTER {
                panic_with_error!(env, Error::InvalidInput);
            }

            let nonce = Self::get_and_bump_global_nonce(env);
            let market_id = Self::build_market_id(env, nonce, current_admin_counter);

            if !Self::check_market_id_collision(env, &market_id) {
                Self::set_admin_counter(env, admin, current_admin_counter + 1);
                Self::register_market_id(env, &market_id, admin, timestamp);
                return market_id;
            }
        }

        panic_with_error!(env, Error::InvalidState);
    }

    /// Returns `true` if `market_id` already exists in persistent storage.
    pub fn check_market_id_collision(env: &Env, market_id: &Symbol) -> bool {
        env.storage()
            .persistent()
            .get::<Symbol, Market>(market_id)
            .is_some()
    }

    /// Returns `true` if `market_id` passes format validation *and* exists in
    /// persistent storage (i.e. it is a live market).
    pub fn is_market_id_valid(env: &Env, market_id: &Symbol) -> bool {
        Self::validate_market_id_format(env, market_id)
            && Self::check_market_id_collision(env, market_id)
    }

    /// Returns `true` if `market_id` starts with the `mkt_` prefix.
    ///
    /// Legacy IDs (created before this module existed) do not carry the prefix
    /// and will return `false` here; callers should treat them as valid but
    /// unstructured.
    pub fn validate_market_id_format(_env: &Env, market_id: &Symbol) -> bool {
        market_id.to_string().starts_with("mkt_")
    }

    /// Parse the counter and legacy flag out of a market ID symbol.
    ///
    /// Returns [`Error::InvalidInput`] if the ID cannot be parsed.
    pub fn parse_market_id_components(
        _env: &Env,
        market_id: &Symbol,
    ) -> Result<MarketIdComponents, Error> {
        let s = market_id.to_string();
        // Expected format: mkt_{hex}_{counter}
        if !s.starts_with("mkt_") {
            return Ok(MarketIdComponents {
                counter: 0,
                is_legacy: true,
            });
        }
        // Split on '_': ["mkt", "{hex}", "{counter}"]
        let parts: alloc::vec::Vec<&str> = s.splitn(3, '_').collect();
        if parts.len() != 3 {
            return Err(Error::InvalidInput);
        }
        let counter = parts[2].parse::<u32>().map_err(|_| Error::InvalidInput)?;
        Ok(MarketIdComponents {
            counter,
            is_legacy: false,
        })
    }

    /// Return a paginated slice of the market ID registry.
    pub fn get_market_id_registry(env: &Env, start: u32, limit: u32) -> Vec<MarketIdRegistryEntry> {
        let key = Symbol::new(env, Self::REGISTRY_KEY);
        let registry: Vec<MarketIdRegistryEntry> = env
            .storage()
            .persistent()
            .get(&key)
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

    /// Return all market IDs created by `admin`.
    pub fn get_admin_markets(env: &Env, admin: &Address) -> Vec<Symbol> {
        let key = Symbol::new(env, Self::REGISTRY_KEY);
        let registry: Vec<MarketIdRegistryEntry> = env
            .storage()
            .persistent()
            .get(&key)
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

    // ── Private helpers ──────────────────────────────────────────────────────

    /// Build a market ID symbol.
    ///
    /// Hash input layout (big-endian):
    /// ```text
    /// [ ledger_sequence (4 B) | global_nonce (4 B) ]
    /// ```
    fn build_market_id(env: &Env, nonce: u32, admin_counter: u32) -> Symbol {
        let sequence = env.ledger().sequence();

        let seq_bytes = Bytes::from_array(env, &sequence.to_be_bytes());
        let nonce_bytes = Bytes::from_array(env, &nonce.to_be_bytes());

        let mut input = seq_bytes;
        input.append(&nonce_bytes);

        let hash = env.crypto().sha256(&input);
        let hash_bytes = hash.to_bytes();

        // First 4 bytes → 8 hex chars.
        let hex: alloc::string::String = (0..4)
            .map(|i| format!("{:02x}", hash_bytes.get(i).unwrap_or(0)))
            .collect();

        let id_str = format!("mkt_{}_{}", hex, admin_counter);
        Symbol::new(env, &id_str)
    }

    /// Read the global nonce and increment it atomically.
    fn get_and_bump_global_nonce(env: &Env) -> u32 {
        let key = Symbol::new(env, Self::GLOBAL_NONCE_KEY);
        let nonce: u32 = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(0u32);
        env.storage().persistent().set(&key, &(nonce + 1));
        nonce
    }

    pub(crate) fn get_admin_counter(env: &Env, admin: &Address) -> u32 {
        let key = Symbol::new(env, Self::ADMIN_COUNTERS_KEY);
        let counters: soroban_sdk::Map<Address, u32> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(soroban_sdk::Map::new(env));
        counters.get(admin.clone()).unwrap_or(0)
    }

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

    fn register_market_id(env: &Env, market_id: &Symbol, admin: &Address, timestamp: u64) {
        let key = Symbol::new(env, Self::REGISTRY_KEY);
        let mut registry: Vec<MarketIdRegistryEntry> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(Vec::new(env));
        registry.push_back(MarketIdRegistryEntry {
            market_id: market_id.clone(),
            admin: admin.clone(),
            timestamp,
        });
        env.storage().persistent().set(&key, &registry);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn setup() -> (Env, Address, Address) {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);
        (env, contract_id, admin)
    }

    fn with_contract<T>(env: &Env, contract_id: &Address, f: impl FnOnce() -> T) -> T {
        env.as_contract(contract_id, f)
    }

    // ── Format & parsing ─────────────────────────────────────────────────────

    #[test]
    fn test_generated_id_has_mkt_prefix() {
        let (env, contract_id, admin) = setup();
        let id = with_contract(&env, &contract_id, || {
            MarketIdGenerator::generate_market_id(&env, &admin)
        });
        assert!(id.to_string().starts_with("mkt_"), "ID must start with mkt_");
    }

    #[test]
    fn test_generated_id_format_three_parts() {
        let (env, contract_id, admin) = setup();
        let id = with_contract(&env, &contract_id, || {
            MarketIdGenerator::generate_market_id(&env, &admin)
        });
        let s = id.to_string();
        let parts: alloc::vec::Vec<&str> = s.splitn(3, '_').collect();
        assert_eq!(parts.len(), 3, "ID must have three '_'-separated parts");
        assert_eq!(parts[0], "mkt");
        assert_eq!(parts[1].len(), 8, "hex segment must be 8 chars");
        assert!(parts[2].parse::<u32>().is_ok(), "counter must be numeric");
    }

    #[test]
    fn test_validate_format_accepts_well_formed_id() {
        let (env, _, _) = setup();
        let id = Symbol::new(&env, "mkt_3f9a1b2c_0");
        assert!(MarketIdGenerator::validate_market_id_format(&env, &id));
    }

    #[test]
    fn test_validate_format_rejects_legacy_id() {
        let (env, _, _) = setup();
        let id = Symbol::new(&env, "legacy_market_id");
        assert!(!MarketIdGenerator::validate_market_id_format(&env, &id));
    }

    #[test]
    fn test_parse_components_extracts_counter() {
        let (env, _, _) = setup();
        let id = Symbol::new(&env, "mkt_abcdef12_42");
        let components = MarketIdGenerator::parse_market_id_components(&env, &id).unwrap();
        assert_eq!(components.counter, 42);
        assert!(!components.is_legacy);
    }

    #[test]
    fn test_parse_components_marks_legacy() {
        let (env, _, _) = setup();
        let id = Symbol::new(&env, "old_format_id");
        let components = MarketIdGenerator::parse_market_id_components(&env, &id).unwrap();
        assert!(components.is_legacy);
    }

    #[test]
    fn test_parse_components_counter_zero() {
        let (env, _, _) = setup();
        let id = Symbol::new(&env, "mkt_00000000_0");
        let components = MarketIdGenerator::parse_market_id_components(&env, &id).unwrap();
        assert_eq!(components.counter, 0);
    }

    // ── Uniqueness ───────────────────────────────────────────────────────────

    #[test]
    fn test_sequential_ids_for_same_admin_are_unique() {
        let (env, contract_id, admin) = setup();
        let (id1, id2) = with_contract(&env, &contract_id, || {
            let a = MarketIdGenerator::generate_market_id(&env, &admin);
            let b = MarketIdGenerator::generate_market_id(&env, &admin);
            (a, b)
        });
        assert_ne!(id1.to_string(), id2.to_string());
    }

    #[test]
    fn test_same_counter_different_admins_produce_different_ids() {
        let (env, contract_id, _) = setup();
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        // Both admins start at counter 0; the global nonce increments between
        // calls, so their IDs must differ.
        let (id1, id2) = with_contract(&env, &contract_id, || {
            let a = MarketIdGenerator::generate_market_id(&env, &admin1);
            let b = MarketIdGenerator::generate_market_id(&env, &admin2);
            (a, b)
        });
        assert_ne!(id1.to_string(), id2.to_string());
    }

    #[test]
    fn test_same_admin_different_ledger_sequence_produces_different_ids() {
        let (env, contract_id, admin) = setup();

        let id1 = with_contract(&env, &contract_id, || {
            MarketIdGenerator::generate_market_id(&env, &admin)
        });

        // Advance the ledger sequence.
        env.ledger().set(LedgerInfo {
            sequence_number: env.ledger().sequence() + 1,
            timestamp: env.ledger().timestamp() + 5,
            protocol_version: 22,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 100,
        });

        let id2 = with_contract(&env, &contract_id, || {
            MarketIdGenerator::generate_market_id(&env, &admin)
        });

        assert_ne!(id1.to_string(), id2.to_string());
    }

    // ── Counter mechanics ────────────────────────────────────────────────────

    #[test]
    fn test_admin_counter_increments_after_generation() {
        let (env, contract_id, admin) = setup();
        with_contract(&env, &contract_id, || {
            MarketIdGenerator::generate_market_id(&env, &admin);
            assert_eq!(MarketIdGenerator::get_admin_counter(&env, &admin), 1);
        });
    }

    #[test]
    fn test_admin_counter_is_independent_per_admin() {
        let (env, contract_id, _) = setup();
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        with_contract(&env, &contract_id, || {
            MarketIdGenerator::generate_market_id(&env, &admin1);
            MarketIdGenerator::generate_market_id(&env, &admin1);
            MarketIdGenerator::generate_market_id(&env, &admin2);

            assert_eq!(MarketIdGenerator::get_admin_counter(&env, &admin1), 2);
            assert_eq!(MarketIdGenerator::get_admin_counter(&env, &admin2), 1);
        });
    }

    #[test]
    fn test_global_nonce_increments_across_admins() {
        let (env, contract_id, _) = setup();
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        with_contract(&env, &contract_id, || {
            MarketIdGenerator::generate_market_id(&env, &admin1);
            MarketIdGenerator::generate_market_id(&env, &admin2);
            // Nonce should be 2 after two generations.
            let nonce_key = Symbol::new(&env, MarketIdGenerator::GLOBAL_NONCE_KEY);
            let nonce: u32 = env
                .storage()
                .persistent()
                .get(&nonce_key)
                .unwrap_or(0);
            assert_eq!(nonce, 2);
        });
    }

    // ── Collision detection ──────────────────────────────────────────────────

    #[test]
    fn test_no_collision_for_fresh_id() {
        let (env, contract_id, _) = setup();
        with_contract(&env, &contract_id, || {
            let id = Symbol::new(&env, "mkt_fresh_0");
            assert!(!MarketIdGenerator::check_market_id_collision(&env, &id));
        });
    }

    #[test]
    fn test_is_market_id_valid_returns_false_for_nonexistent() {
        let (env, contract_id, _) = setup();
        let valid = with_contract(&env, &contract_id, || {
            let id = Symbol::new(&env, "mkt_00000000_0");
            MarketIdGenerator::is_market_id_valid(&env, &id)
        });
        assert!(!valid);
    }

    // ── Registry ─────────────────────────────────────────────────────────────

    #[test]
    fn test_registry_empty_initially() {
        let (env, contract_id, _) = setup();
        let entries = with_contract(&env, &contract_id, || {
            MarketIdGenerator::get_market_id_registry(&env, 0, 10)
        });
        assert_eq!(entries.len(), 0);
    }

    #[test]
    fn test_registry_records_generated_ids() {
        let (env, contract_id, admin) = setup();
        with_contract(&env, &contract_id, || {
            MarketIdGenerator::generate_market_id(&env, &admin);
            MarketIdGenerator::generate_market_id(&env, &admin);
            let entries = MarketIdGenerator::get_market_id_registry(&env, 0, 10);
            assert_eq!(entries.len(), 2);
        });
    }

    #[test]
    fn test_registry_pagination_start_beyond_end() {
        let (env, contract_id, admin) = setup();
        with_contract(&env, &contract_id, || {
            MarketIdGenerator::generate_market_id(&env, &admin);
            let entries = MarketIdGenerator::get_market_id_registry(&env, 100, 10);
            assert_eq!(entries.len(), 0);
        });
    }

    #[test]
    fn test_registry_pagination_limit() {
        let (env, contract_id, admin) = setup();
        with_contract(&env, &contract_id, || {
            for _ in 0..5 {
                MarketIdGenerator::generate_market_id(&env, &admin);
            }
            let page = MarketIdGenerator::get_market_id_registry(&env, 0, 3);
            assert_eq!(page.len(), 3);
        });
    }

    #[test]
    fn test_get_admin_markets_empty_for_new_admin() {
        let (env, contract_id, admin) = setup();
        let markets = with_contract(&env, &contract_id, || {
            MarketIdGenerator::get_admin_markets(&env, &admin)
        });
        assert_eq!(markets.len(), 0);
    }

    #[test]
    fn test_get_admin_markets_returns_only_own_markets() {
        let (env, contract_id, _) = setup();
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        with_contract(&env, &contract_id, || {
            MarketIdGenerator::generate_market_id(&env, &admin1);
            MarketIdGenerator::generate_market_id(&env, &admin1);
            MarketIdGenerator::generate_market_id(&env, &admin2);

            let m1 = MarketIdGenerator::get_admin_markets(&env, &admin1);
            let m2 = MarketIdGenerator::get_admin_markets(&env, &admin2);
            assert_eq!(m1.len(), 2);
            assert_eq!(m2.len(), 1);
        });
    }

    // ── Stress: many IDs per ledger context ──────────────────────────────────

    /// Generate 50 IDs for a single admin within the same ledger and verify
    /// all are unique.  This exercises the global-nonce path and confirms no
    /// accidental hash collisions at small nonce values.
    #[test]
    fn test_stress_50_ids_same_admin_same_ledger() {
        let (env, contract_id, admin) = setup();
        with_contract(&env, &contract_id, || {
            let mut ids = alloc::vec::Vec::new();
            for _ in 0..50 {
                let id = MarketIdGenerator::generate_market_id(&env, &admin);
                let s = id.to_string();
                assert!(!ids.contains(&s), "duplicate ID: {}", s);
                ids.push(s);
            }
            assert_eq!(ids.len(), 50);
        });
    }

    /// Generate IDs for 20 different admins in the same ledger and verify
    /// all are unique across admins.
    #[test]
    fn test_stress_20_admins_same_ledger_all_unique() {
        let (env, contract_id, _) = setup();
        let admins: alloc::vec::Vec<Address> =
            (0..20).map(|_| Address::generate(&env)).collect();

        with_contract(&env, &contract_id, || {
            let mut ids = alloc::vec::Vec::new();
            for admin in &admins {
                let id = MarketIdGenerator::generate_market_id(&env, admin);
                let s = id.to_string();
                assert!(!ids.contains(&s), "cross-admin duplicate: {}", s);
                ids.push(s);
            }
            assert_eq!(ids.len(), 20);
        });
    }

    /// Generate IDs across 5 different ledger sequences and verify uniqueness.
    #[test]
    fn test_stress_ids_across_multiple_ledgers() {
        let (env, contract_id, admin) = setup();
        let mut all_ids = alloc::vec::Vec::new();

        for ledger_bump in 0u32..5 {
            env.ledger().set(LedgerInfo {
                sequence_number: 100 + ledger_bump,
                timestamp: 1_000_000 + (ledger_bump as u64) * 5,
                protocol_version: 22,
                network_id: Default::default(),
                base_reserve: 10,
                min_temp_entry_ttl: 1,
                min_persistent_entry_ttl: 1,
                max_entry_ttl: 100,
            });

            with_contract(&env, &contract_id, || {
                for _ in 0..5 {
                    let id = MarketIdGenerator::generate_market_id(&env, &admin);
                    let s = id.to_string();
                    assert!(!all_ids.contains(&s), "cross-ledger duplicate: {}", s);
                    all_ids.push(s);
                }
            });
        }
        assert_eq!(all_ids.len(), 25);
    }
}
