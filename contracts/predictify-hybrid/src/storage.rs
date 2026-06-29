#![cfg_attr(test, allow(dead_code))]

use super::*;
use crate::markets::{MarketStateLogic, MarketStateManager};
use crate::types::{Balance, ReflectorAsset, Market, MarketState, OracleConfig};
use soroban_sdk::{contracttype, Address, Env, IntoVal, Map, Symbol, Val, Vec};

const STORAGE_CONFIG_KEY: &str = "storage_config";
const LEDGERS_PER_DAY: u32 = 17_280;
const BALANCE_TTL_LEDGERS: u32 = 31 * LEDGERS_PER_DAY;
const MARKET_TTL_LEDGERS: u32 = 365 * LEDGERS_PER_DAY;
const EVENT_TTL_LEDGERS: u32 = 90 * LEDGERS_PER_DAY;
const ARCHIVE_TTL_LEDGERS: u32 = 365 * LEDGERS_PER_DAY;

/// TTL for instance storage cache entries, in ledgers.
/// At ~5 seconds per ledger on Soroban mainnet, 100 ledgers ≈ 8 minutes.
/// Instance TTL is shared - bumping extends all instance storage keys.
/// Increase for longer-lived deployments; decrease to reduce ledger rent costs.
pub const MARKET_CACHE_TTL_LEDGERS: u32 = 100;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StorageTtlTier {
    Balance,
    Market,
    Event,
    Archive,
}

// ===== STORAGE OPTIMIZATION TYPES =====

/// Storage key variants for contracts/predictify-hybrid
///
/// These variants are used as persistent storage keys. Each variant must have a unique
/// XDR encoding to avoid collisions in the storage layer. A collision detection test
/// exists in `tests/datakey_collision.rs` that verifies all variants produce unique
/// XDR byte representations. If this test fails, it indicates a critical issue with
/// storage key uniqueness that must be resolved before deploying to production.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Whitelisted(Address),
    Blacklisted(Address),
    AdminOverrideNonce(Address),
    ArchivedMarket(Symbol, u64),
    /// Cumulative days extended for a given market (u32).
    MarketExtensionTotal(Symbol),
    MarketMetadata(Symbol),
    MarketScratch(Symbol),
    DisputeHistoryCap,
    DisputeHistory(Symbol),
    DisputeStakeCap(Symbol, Address),
    /// Instance storage cache key for Market structs, keyed by market_id.
    /// Used by MarketReadCache in markets.rs.
    MarketCache(Symbol),
}

/// Storage format version for migration tracking
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageFormat {
    /// Original format (v1)
    V1,
    /// Optimized format with compression (v2)
    V2,
    /// Latest format with advanced compression (v3)
    V3,
}

/// Compressed market data structure for storage optimization
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompressedMarket {
    /// Market ID
    pub market_id: Symbol,
    /// Compressed market data (using i128 instead of u8 for Soroban compatibility)
    pub compressed_data: Vec<i128>,
    /// Compression algorithm used
    pub compression_type: String,
    /// Original data size
    pub original_size: u32,
    /// Compressed data size
    pub compressed_size: u32,
    /// Compression timestamp
    pub compressed_at: u64,
    /// Checksum for data integrity
    pub checksum: String,
}

/// Storage usage statistics
#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageUsageStats {
    /// Total number of markets stored
    pub total_markets: u32,
    /// Total storage used (in bytes)
    pub total_storage_bytes: u64,
    /// Average storage per market (in bytes)
    pub avg_storage_per_market: u64,
    /// Number of compressed markets
    pub compressed_markets: u32,
    /// Storage savings from compression (in bytes)
    pub storage_savings: u64,
    /// Compression ratio (percentage as i128 * 100)
    pub compression_ratio: i128,
    /// Oldest market timestamp
    pub oldest_market_timestamp: u64,
    /// Newest market timestamp
    pub newest_market_timestamp: u64,
}

/// Storage optimization configuration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageConfig {
    /// Whether compression is enabled
    pub compression_enabled: bool,
    /// Minimum market age for compression (in days)
    pub min_compression_age_days: u32,
    /// Maximum storage per market (in bytes)
    pub max_storage_per_market: u64,
    /// Storage cleanup threshold (in days)
    pub cleanup_threshold_days: u32,
    /// Whether to enable automatic cleanup
    pub auto_cleanup_enabled: bool,
    /// Compression algorithm preference
    pub preferred_compression: String,
    /// TTL tier for balance records in ledgers (~31 days at 5s/ledger).
    pub balance_ttl_ledgers: u32,
    /// TTL tier for live market and market-adjacent records in ledgers (~365 days at 5s/ledger).
    pub market_ttl_ledgers: u32,
    /// TTL tier for event records in ledgers (~90 days at 5s/ledger).
    pub event_ttl_ledgers: u32,
    /// TTL tier for archive and migration records in ledgers (~365 days at 5s/ledger).
    pub archive_ttl_ledgers: u32,
}

/// Storage migration record
#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageMigration {
    /// Migration ID
    pub migration_id: Symbol,
    /// Source format
    pub from_format: StorageFormat,
    /// Target format
    pub to_format: StorageFormat,
    /// Number of markets migrated
    pub markets_migrated: u32,
    /// Migration start timestamp
    pub started_at: u64,
    /// Migration completion timestamp
    pub completed_at: Option<u64>,
    /// Migration status
    pub status: String,
    /// Error message if failed
    pub error_message: Option<String>,
}

impl StorageMigration {
    pub fn promote_market_to_persistent(env: &Env, market_id: &Symbol) -> Result<(), Error> {
        let temp_key = DataKey::MarketMetadata(market_id.clone());
        let persistent_key = DataKey::MarketMetadata(market_id.clone());

        let metadata_opt = env.storage().temporary().get::<_, Market>(&temp_key);

        if let Some(metadata) = metadata_opt {
            metadata.admin.require_auth();

            let config = StorageOptimizer::get_storage_config(env);
            
            StorageOptimizer::set_persistent_with_ttl(
                env,
                &persistent_key,
                &metadata,
                config.market_ttl_ledgers,
            );

            env.storage().temporary().remove(&temp_key);

            crate::events::EventEmitter::emit_market_archived(
                env,
                market_id,
                &String::from_str(env, "Temporary"),
                &String::from_str(env, "Persistent"),
            );

            Ok(())
        } else {
            let persistent_opt = env.storage().persistent().get::<_, Market>(&persistent_key);
            if let Some(metadata) = persistent_opt {
                metadata.admin.require_auth();

                let config = StorageOptimizer::get_storage_config(env);
                StorageOptimizer::extend_persistent_ttl(
                    env,
                    &persistent_key,
                    config.market_ttl_ledgers,
                );
                Ok(())
            } else {
                Err(Error::MarketNotFound)
            }
        }
    }

    pub fn demote_scratch_keys(env: &Env, market_id: &Symbol) -> Result<(), Error> {
        let temp_key = DataKey::MarketScratch(market_id.clone());
        let persistent_key = DataKey::MarketScratch(market_id.clone());
        let metadata_key = DataKey::MarketMetadata(market_id.clone());

        let market = if let Some(m) = env.storage().persistent().get::<_, Market>(&metadata_key) {
            m
        } else if let Some(m) = env.storage().temporary().get::<_, Market>(&metadata_key) {
            m
        } else {
            MarketStateManager::get_market(env, market_id)?
        };

        market.admin.require_auth();

        let scratch_opt = env.storage().persistent().get::<_, Vec<i128>>(&persistent_key);

        if let Some(scratch_data) = scratch_opt {
            let config = StorageOptimizer::get_storage_config(env);

            env.storage().temporary().set(&temp_key, &scratch_data);
            env.storage().temporary().extend_ttl(
                &temp_key,
                config.balance_ttl_ledgers,
                config.balance_ttl_ledgers,
            );

            env.storage().persistent().remove(&persistent_key);

            crate::events::EventEmitter::emit_market_archived(
                env,
                market_id,
                &String::from_str(env, "Persistent"),
                &String::from_str(env, "Temporary"),
            );

            Ok(())
        } else {
            if env.storage().temporary().has(&temp_key) {
                let config = StorageOptimizer::get_storage_config(env);
                env.storage().temporary().extend_ttl(
                    &temp_key,
                    config.balance_ttl_ledgers,
                    config.balance_ttl_ledgers,
                );
                Ok(())
            } else {
                Ok(())
            }
        }
    }
}

/// Storage integrity check result
#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageIntegrityResult {
    /// Market ID
    pub market_id: Symbol,
    /// Whether integrity check passed
    pub is_valid: bool,
    /// Data corruption detected
    pub corruption_detected: bool,
    /// Missing data detected
    pub missing_data: bool,
    /// Checksum validation result
    pub checksum_valid: bool,
    /// Error messages
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

// ===== STORAGE OPTIMIZER =====

/// Main storage optimization manager
pub struct StorageOptimizer;

impl StorageOptimizer {
    fn default_storage_config(env: &Env) -> StorageConfig {
        StorageConfig {
            compression_enabled: true,
            min_compression_age_days: 30,
            max_storage_per_market: 1024 * 1024, // 1MB
            cleanup_threshold_days: 365,
            auto_cleanup_enabled: false,
            preferred_compression: String::from_str(env, "simple_optimization"),
            balance_ttl_ledgers: BALANCE_TTL_LEDGERS,
            market_ttl_ledgers: MARKET_TTL_LEDGERS,
            event_ttl_ledgers: EVENT_TTL_LEDGERS,
            archive_ttl_ledgers: ARCHIVE_TTL_LEDGERS,
        }
    }

    fn ttl_for_tier(config: &StorageConfig, tier: StorageTtlTier) -> u32 {
        match tier {
            StorageTtlTier::Balance => config.balance_ttl_ledgers,
            StorageTtlTier::Market => config.market_ttl_ledgers,
            StorageTtlTier::Event => config.event_ttl_ledgers,
            StorageTtlTier::Archive => config.archive_ttl_ledgers,
        }
    }

    fn clamp_persistent_ttl(env: &Env, desired_ttl_ledgers: u32) -> u32 {
        desired_ttl_ledgers.min(env.storage().max_ttl())
    }

    fn persistent_ttl_for_tier(env: &Env, tier: StorageTtlTier) -> u32 {
        let config = Self::get_storage_config(env);
        Self::clamp_persistent_ttl(env, Self::ttl_for_tier(&config, tier))
    }

    pub fn extend_persistent_ttl<K>(env: &Env, key: &K, desired_ttl_ledgers: u32)
    where
        K: IntoVal<Env, Val>,
    {
        let effective_ttl = Self::clamp_persistent_ttl(env, desired_ttl_ledgers);
        env.storage()
            .persistent()
            .extend_ttl(key, effective_ttl, effective_ttl);
    }

    fn set_persistent_with_ttl<K, V>(
        env: &Env,
        key: &K,
        value: &V,
        desired_ttl_ledgers: u32,
    ) where
        K: IntoVal<Env, Val>,
        V: IntoVal<Env, Val>,
    {
        env.storage().persistent().set(key, value);
        Self::extend_persistent_ttl(env, key, desired_ttl_ledgers);
    }

    /// Compress market data for storage optimization
    pub fn compress_market_data(env: &Env, market: &Market) -> Result<CompressedMarket, Error> {
        // Create a simple compression by removing unnecessary fields and optimizing structure
        let market_id = Self::generate_market_id(env, &market.question);

        // Convert market to compressed format
        let compressed_data = Self::serialize_compressed_market(env, market)?;
        let original_size = Self::calculate_market_size(market);
        let compressed_size = compressed_data.len() as u32;

        // Calculate compression ratio (as percentage * 100 for integer storage)
        let _compression_ratio = if original_size > 0 {
            (compressed_size as i128 * 10000) / original_size as i128
        } else {
            0
        };

        // Generate checksum for data integrity
        let checksum = Self::generate_checksum(&compressed_data);

        Ok(CompressedMarket {
            market_id,
            compressed_data,
            compression_type: String::from_str(env, "simple_optimization"),
            original_size,
            compressed_size,
            compressed_at: env.ledger().timestamp(),
            checksum,
        })
    }

    /// Clean up old market data based on age and state
    pub fn cleanup_old_market_data(env: &Env, market_id: &Symbol) -> Result<bool, Error> {
        let market = MarketStateManager::get_market(env, market_id)?;
        let current_time = env.ledger().timestamp();

        // Check if market is old enough for cleanup
        let market_age_days = (current_time - market.end_time) / (24 * 60 * 60);
        let config = Self::get_storage_config(env);

        if market_age_days > config.cleanup_threshold_days.into() {
            // Only cleanup closed or cancelled markets
            if market.state == MarketState::Closed || market.state == MarketState::Cancelled {
                // Archive market data before deletion
                Self::archive_market_data(env, market_id, &market)?;

                // Remove from storage
                MarketStateManager::remove_market(env, market_id);

                // Emit cleanup event
                events::EventEmitter::emit_storage_cleanup_event(
                    env,
                    market_id,
                    &String::from_str(env, "old_market_cleanup"),
                );

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Migrate storage format from old to new format
    pub fn migrate_storage_format(
        env: &Env,
        from_format: StorageFormat,
        to_format: StorageFormat,
    ) -> Result<StorageMigration, Error> {
        let migration_id = Symbol::new(env, &format!("migration_{}", env.ledger().timestamp()));

        let mut migration = StorageMigration {
            migration_id: migration_id.clone(),
            from_format: from_format.clone(),
            to_format: to_format.clone(),
            markets_migrated: 0,
            started_at: env.ledger().timestamp(),
            completed_at: None,
            status: String::from_str(env, "in_progress"),
            error_message: None,
        };

        // Store migration record
        Self::store_migration_record(env, &migration_id, &migration);

        match (from_format, to_format) {
            (StorageFormat::V1, StorageFormat::V2) => {
                migration = Self::migrate_v1_to_v2(env, migration)?;
            }
            (StorageFormat::V2, StorageFormat::V3) => {
                migration = Self::migrate_v2_to_v3(env, migration)?;
            }
            _ => {
                migration.status = String::from_str(env, "unsupported_migration");
                migration.error_message = Some(String::from_str(env, "Unsupported migration path"));
            }
        }

        // Update migration record
        migration.completed_at = Some(env.ledger().timestamp());
        Self::store_migration_record(env, &migration_id, &migration);

        Ok(migration)
    }

    /// Monitor storage usage and return statistics
    pub fn monitor_storage_usage(env: &Env) -> Result<StorageUsageStats, Error> {
        let mut total_markets = 0;
        let mut total_storage_bytes = 0u64;
        let mut compressed_markets = 0;
        let mut storage_savings = 0u64;
        let mut oldest_timestamp = u64::MAX;
        let mut newest_timestamp = 0u64;

        // Iterate through all markets (this is a simplified approach)
        // In a real implementation, you'd have a market registry
        let market_ids = Self::get_all_market_ids(env);

        for market_id in market_ids.iter() {
            if let Ok(market) = MarketStateManager::get_market(env, &market_id) {
                total_markets += 1;
                let market_size = Self::calculate_market_size(&market);
                total_storage_bytes += market_size as u64;

                // Track timestamps
                if market.end_time < oldest_timestamp {
                    oldest_timestamp = market.end_time;
                }
                if market.end_time > newest_timestamp {
                    newest_timestamp = market.end_time;
                }

                // Check if market is compressed
                if Self::is_market_compressed(env, &market_id) {
                    compressed_markets += 1;
                    // Calculate savings (simplified)
                    storage_savings += market_size as u64 / 2; // Assume 50% compression
                }
            }
        }

        let avg_storage_per_market = if total_markets > 0 {
            total_storage_bytes / total_markets as u64
        } else {
            0
        };

        let compression_ratio = if total_storage_bytes > 0 {
            (storage_savings as i128 * 10000) / total_storage_bytes as i128
        } else {
            0
        };

        Ok(StorageUsageStats {
            total_markets,
            total_storage_bytes,
            avg_storage_per_market,
            compressed_markets,
            storage_savings,
            compression_ratio,
            oldest_market_timestamp: if oldest_timestamp == u64::MAX {
                0
            } else {
                oldest_timestamp
            },
            newest_market_timestamp: newest_timestamp,
        })
    }

    /// Optimize storage layout for a specific market
    pub fn optimize_storage_layout(env: &Env, market_id: &Symbol) -> Result<bool, Error> {
        let market = MarketStateManager::get_market(env, market_id)?;

        // Check if optimization is needed
        let current_size = Self::calculate_market_size(&market);
        let config = Self::get_storage_config(env);

        if current_size as u64 > config.max_storage_per_market {
            // Apply compression
            let compressed_market = Self::compress_market_data(env, &market)?;

            // Store compressed version
            Self::store_compressed_market(env, &compressed_market)?;

            // Update market reference to point to compressed data
            Self::update_market_to_compressed(env, market_id, &compressed_market.market_id)?;

            // Emit optimization event
            events::EventEmitter::emit_storage_optimization_event(
                env,
                market_id,
                &String::from_str(env, "compression_applied"),
            );

            return Ok(true);
        }

        Ok(false)
    }

    /// Get storage usage statistics
    pub fn get_storage_usage_statistics(env: &Env) -> Result<StorageUsageStats, Error> {
        Self::monitor_storage_usage(env)
    }

    /// Validate storage integrity for a specific market
    pub fn validate_storage_integrity(
        env: &Env,
        market_id: &Symbol,
    ) -> Result<StorageIntegrityResult, Error> {
        let mut result = StorageIntegrityResult {
            market_id: market_id.clone(),
            is_valid: true,
            corruption_detected: false,
            missing_data: false,
            checksum_valid: true,
            errors: Vec::new(env),
            warnings: Vec::new(env),
        };

        // Try to get market data
        match MarketStateManager::get_market(env, market_id) {
            Ok(market) => {
                // Validate market structure
                if let Err(e) = market.validate(env) {
                    result.is_valid = false;
                    result.corruption_detected = true;
                    result.errors.push_back(String::from_str(
                        env,
                        &format!("Validation failed: {:?}", e),
                    ));
                }

                // Check for missing critical data
                if market.question.is_empty() {
                    result.missing_data = true;
                    result
                        .warnings
                        .push_back(String::from_str(env, "Empty market question"));
                }

                if market.outcomes.is_empty() {
                    result.missing_data = true;
                    result
                        .errors
                        .push_back(String::from_str(env, "No outcomes defined"));
                }

                // Validate state consistency
                if let Err(e) = MarketStateLogic::validate_market_state_consistency(env, &market) {
                    result.is_valid = false;
                    result.errors.push_back(String::from_str(
                        env,
                        &format!("State inconsistency: {:?}", e),
                    ));
                }
            }
            Err(e) => {
                result.is_valid = false;
                result.missing_data = true;
                result
                    .errors
                    .push_back(String::from_str(env, &format!("Market not found: {:?}", e)));
            }
        }

        // Check compressed data if exists
        if Self::is_market_compressed(env, market_id) {
            if let Ok(compressed) = Self::get_compressed_market(env, market_id) {
                // Validate checksum
                let calculated_checksum = Self::generate_checksum(&compressed.compressed_data);
                if calculated_checksum != compressed.checksum {
                    result.checksum_valid = false;
                    result.corruption_detected = true;
                    result
                        .errors
                        .push_back(String::from_str(env, "Checksum validation failed"));
                }
            }
        }

        Ok(result)
    }

    /// Get storage configuration
    pub fn get_storage_config(env: &Env) -> StorageConfig {
        match env
            .storage()
            .persistent()
            .get(&Symbol::new(env, STORAGE_CONFIG_KEY))
        {
            Some(config) => config,
            None => Self::default_storage_config(env),
        }
    }

    /// Update storage configuration
    pub fn update_storage_config(env: &Env, config: &StorageConfig) -> Result<(), Error> {
        let key = Symbol::new(env, STORAGE_CONFIG_KEY);
        Self::set_persistent_with_ttl(env, &key, config, config.archive_ttl_ledgers);
        Ok(())
    }
}

// ===== BALANCE STORAGE =====

/// Persistent storage manager for user balances.
pub struct BalanceStorage;

impl BalanceStorage {
    fn validate_balance_delta(amount: i128) -> Result<(), Error> {
        if amount <= 0 {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Generates the storage key for a user's asset balance.
    fn get_key(env: &Env, user: &Address, asset: &ReflectorAsset) -> Vec<Val> {
        let mut key = Vec::new(env);
        key.push_back(Symbol::new(env, "Balance").into_val(env));
        key.push_back(user.to_val());
        key.push_back(asset.into_val(env));
        key
    }

    /// Retrieves the current balance for a user and asset from persistent storage.
    ///
    /// Returns a default Balance with amount 0 if no record exists.
    pub fn get_balance(env: &Env, user: &Address, asset: &ReflectorAsset) -> Balance {
        let key = Self::get_key(env, user, asset);
        env.storage().persistent().get(&key).unwrap_or(Balance {
            user: user.clone(),
            asset: asset.clone(),
            amount: 0,
        })
    }

    /// Stores the balance record in persistent storage and extends its TTL.
    pub fn set_balance(env: &Env, balance: &Balance) -> Result<(), Error> {
        if balance.amount < 0 {
            return Err(Error::InvalidState);
        }

        let key = Self::get_key(env, &balance.user, &balance.asset);
        env.storage().persistent().set(&key, balance);
        // Extend TTL to ensure balance persists (approx 30 days)
        env.storage().persistent().extend_ttl(&key, 535680, 535680);
        Ok(())
    }

    /// Computes the resulting balance after a credit without mutating storage.
    pub fn checked_add_balance(
        env: &Env,
        user: &Address,
        asset: &ReflectorAsset,
        amount: i128,
    ) -> Result<Balance, Error> {
        Self::validate_balance_delta(amount)?;

        let mut balance = Self::get_balance(env, user, asset);
        balance.amount = balance
            .amount
            .checked_add(amount)
            .ok_or(Error::InvalidInput)?;
        Ok(balance)
    }

    /// Computes the resulting balance after a debit without mutating storage.
    pub fn checked_sub_balance(
        env: &Env,
        user: &Address,
        asset: &ReflectorAsset,
        amount: i128,
    ) -> Result<Balance, Error> {
        Self::validate_balance_delta(amount)?;

        let mut balance = Self::get_balance(env, user, asset);
        if amount > balance.amount {
            return Err(Error::InsufficientBalance);
        }

        balance.amount = balance
            .amount
            .checked_sub(amount)
            .ok_or(Error::InvalidState)?;
        Ok(balance)
    }

    /// Increments a user's balance by the specified amount.
    ///
    /// # Errors
    /// - `Error::InvalidInput` if the resulting amount overflows i128.
    pub fn add_balance(
        env: &Env,
        user: &Address,
        asset: &ReflectorAsset,
        amount: i128,
    ) -> Result<Balance, Error> {
        let balance = Self::checked_add_balance(env, user, asset, amount)?;
        Self::set_balance(env, &balance)?;
        Ok(balance)
    }

    /// Decrements a user's balance by the specified amount.
    ///
    /// # Errors
    /// - `Error::InsufficientBalance` if the user has less than the requested amount.
    pub fn sub_balance(
        env: &Env,
        user: &Address,
        asset: &ReflectorAsset,
        amount: i128,
    ) -> Result<Balance, Error> {
        let balance = Self::checked_sub_balance(env, user, asset, amount)?;
        Self::set_balance(env, &balance)?;
        Ok(balance)
    }
}

// ===== PRIVATE HELPER METHODS =====

impl StorageOptimizer {
    /// Serialize market to compressed format
    fn serialize_compressed_market(env: &Env, market: &Market) -> Result<Vec<i128>, Error> {
        // Simple serialization - in a real implementation, you'd use a proper serialization library
        let mut data = Vec::new(env);

        // Add essential fields only
        data.push_back(0); // Simplified - in real implementation, you'd properly serialize the address
        data.push_back(market.question.len() as i128);
        data.push_back(market.outcomes.len() as i128);
        data.push_back((market.end_time >> 56) as i128);
        data.push_back((market.end_time >> 48) as i128);
        data.push_back((market.end_time >> 40) as i128);
        data.push_back((market.end_time >> 32) as i128);
        data.push_back((market.end_time >> 24) as i128);
        data.push_back((market.end_time >> 16) as i128);
        data.push_back((market.end_time >> 8) as i128);
        data.push_back(market.end_time as i128);
        data.push_back(market.total_staked);
        data.push_back(market.state as i128);

        Ok(data)
    }

    /// Calculate approximate size of market data
    fn calculate_market_size(market: &Market) -> u32 {
        // Simplified size calculation
        let base_size = 100; // Base overhead
        let question_size = market.question.len() as u32;
        let outcomes_size = market.outcomes.len() as u32 * 50; // Average outcome size
        let votes_size = market.votes.len() as u32 * 100; // Average vote entry size
        let stakes_size = market.stakes.len() as u32 * 50; // Average stake entry size

        base_size + question_size + outcomes_size + votes_size + stakes_size
    }

    /// Generate checksum for data integrity
    fn generate_checksum(data: &Vec<i128>) -> String {
        // Simple checksum - in production, use a proper hash function
        let mut checksum = 0i128;
        for value in data.iter() {
            checksum = checksum.wrapping_add(value);
        }
        String::from_str(&data.env(), &format!("{:016x}", checksum))
    }

    /// Generate market ID from question
    fn generate_market_id(env: &Env, question: &String) -> Symbol {
        // Simple hash-based ID generation
        let mut hash = 0i128;
        // Simplified hash generation - in real implementation, you'd properly hash the string
        hash = hash.wrapping_add(question.len() as i128);
        Symbol::new(env, &format!("market_{:016x}", hash))
    }

    /// Archive market data before deletion
    pub(crate) fn archive_market_data(env: &Env, market_id: &Symbol, market: &Market) -> Result<(), Error> {
        // Store archived version with timestamp
        let archive_key = DataKey::ArchivedMarket(market_id.clone(), env.ledger().timestamp());
        Self::set_persistent_with_ttl(
            env,
            &archive_key,
            market,
            Self::persistent_ttl_for_tier(env, StorageTtlTier::Archive),
        );
        Ok(())
    }

    /// Store migration record
    fn store_migration_record(env: &Env, migration_id: &Symbol, migration: &StorageMigration) {
        Self::set_persistent_with_ttl(
            env,
            migration_id,
            migration,
            Self::persistent_ttl_for_tier(env, StorageTtlTier::Archive),
        );
    }

    /// Migrate from V1 to V2 format
    fn migrate_v1_to_v2(
        env: &Env,
        mut migration: StorageMigration,
    ) -> Result<StorageMigration, Error> {
        // Simplified migration - in real implementation, you'd migrate actual data
        migration.markets_migrated = 1;
        migration.status = String::from_str(env, "completed");
        Ok(migration)
    }

    /// Migrate from V2 to V3 format
    fn migrate_v2_to_v3(
        env: &Env,
        mut migration: StorageMigration,
    ) -> Result<StorageMigration, Error> {
        // Simplified migration - in real implementation, you'd migrate actual data
        migration.markets_migrated = 1;
        migration.status = String::from_str(env, "completed");
        Ok(migration)
    }

    /// Get all market IDs (simplified - in real implementation, you'd have a registry)
    fn get_all_market_ids(env: &Env) -> Vec<Symbol> {
        // This is a simplified approach - in a real implementation,
        // you'd maintain a registry of all market IDs
        let market_ids = Vec::new(env);
        // For now, return empty vector - this would be populated from a registry
        market_ids
    }

    /// Check if market is compressed
    fn is_market_compressed(env: &Env, market_id: &Symbol) -> bool {
        let key = crate::event_archive::derive_archive_key(env, market_id, "compressed");
        env.storage().persistent().has(&key)
    }

    /// Store compressed market
    fn store_compressed_market(
        env: &Env,
        compressed_market: &CompressedMarket,
    ) -> Result<(), Error> {
        let key = crate::event_archive::derive_archive_key(env, &compressed_market.market_id, "compressed");
        Self::set_persistent_with_ttl(
            env,
            &key,
            compressed_market,
            Self::persistent_ttl_for_tier(env, StorageTtlTier::Market),
        );
        Ok(())
    }

    /// Get compressed market
    fn get_compressed_market(env: &Env, market_id: &Symbol) -> Result<CompressedMarket, Error> {
        let key = crate::event_archive::derive_archive_key(env, market_id, "compressed");
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::MarketNotFound)
    }

    /// Update market to point to compressed data
    fn update_market_to_compressed(
        env: &Env,
        market_id: &Symbol,
        compressed_id: &Symbol,
    ) -> Result<(), Error> {
        let key = crate::event_archive::derive_archive_key(env, market_id, "compressed_ref");
        Self::set_persistent_with_ttl(
            env,
            &key,
            compressed_id,
            Self::persistent_ttl_for_tier(env, StorageTtlTier::Market),
        );
        Ok(())
    }
}

// ===== EVENT STORAGE =====

/// Manager for event storage operations
pub struct EventManager;

impl EventManager {
    fn event_storage_key(env: &Env, event_id: &Symbol) -> (Symbol, Symbol) {
        (Symbol::new(env, "Event"), event_id.clone())
    }

    /// Store a new event in persistent storage
    pub fn store_event(env: &Env, event: &Event) {
        let key = Self::event_storage_key(env, &event.id);
        StorageOptimizer::set_persistent_with_ttl(
            env,
            &key,
            event,
            StorageOptimizer::persistent_ttl_for_tier(env, StorageTtlTier::Event),
        );
    }

    /// Retrieve an event from persistent storage
    pub fn get_event(env: &Env, event_id: &Symbol) -> Result<Event, Error> {
        let key = Self::event_storage_key(env, event_id);
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::MarketNotFound)
    }

    /// Check if an event exists
    pub fn has_event(env: &Env, event_id: &Symbol) -> bool {
        let key = Self::event_storage_key(env, event_id);
        env.storage().persistent().has(&key)
    }

    /// Update an existing event
    pub fn update_event(env: &Env, event: &Event) -> Result<(), Error> {
        if !Self::has_event(env, &event.id) {
            return Err(Error::MarketNotFound);
        }
        Self::store_event(env, event);
        Ok(())
    }
}

// ===== CREATOR LIMITS STORAGE =====

/// Manager for creator-related limit and tracking operations
pub struct CreatorLimitsManager;

impl CreatorLimitsManager {
    /// Get the storage key for a specific creator's active events count
    fn get_active_events_key(env: &Env, creator: &Address) -> Symbol {
        let mut key_bytes = soroban_sdk::Bytes::new(env);
        key_bytes.append(&soroban_sdk::Bytes::from_slice(env, b"ActiveEvents_"));
        // Simply use a composite struct to represent the key to avoid complex byte manipulation.
        // A common pattern in Soroban is a tuple `(Symbol, Address)`.
        Symbol::new(env, "ActiveEvt") // we will construct a tuple key instead in the actual methods
    }

    /// Retrieve the number of active events for a given creator
    pub fn get_active_events(env: &Env, creator: &Address) -> u32 {
        let key = (Symbol::new(env, "ActiveEvents"), creator.clone());
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    /// Increment a creator's active events count by 1
    pub fn increment_active_events(env: &Env, creator: &Address) {
        let key = (Symbol::new(env, "ActiveEvents"), creator.clone());
        let current_count: u32 = env.storage().persistent().get(&key).unwrap_or(0);
        StorageOptimizer::set_persistent_with_ttl(
            env,
            &key,
            &(current_count + 1),
            StorageOptimizer::persistent_ttl_for_tier(env, StorageTtlTier::Market),
        );
    }

    /// Decrement a creator's active events count by 1
    pub fn decrement_active_events(env: &Env, creator: &Address) {
        let key = (Symbol::new(env, "ActiveEvents"), creator.clone());
        let current_count: u32 = env.storage().persistent().get(&key).unwrap_or(0);

        // Prevent underflow if count is already 0
        if current_count > 0 {
            StorageOptimizer::set_persistent_with_ttl(
                env,
                &key,
                &(current_count - 1),
                StorageOptimizer::persistent_ttl_for_tier(env, StorageTtlTier::Market),
            );
        }
    }
}

// ===== STORAGE UTILITIES =====

/// Storage utility functions
pub struct StorageUtils;

impl StorageUtils {
    /// Calculate storage cost for a market
    pub fn calculate_storage_cost(market: &Market) -> u64 {
        let size = StorageOptimizer::calculate_market_size(market);
        // Simplified cost calculation - in real implementation, use actual blockchain costs
        size as u64 * 100 // 100 stroops per byte
    }

    /// Get storage efficiency score (0-100)
    pub fn get_storage_efficiency_score(market: &Market) -> u32 {
        let size = StorageOptimizer::calculate_market_size(market);
        let efficiency = match size {
            0..=1000 => 100,
            1001..=5000 => 80,
            5001..=10000 => 60,
            10001..=50000 => 40,
            _ => 20,
        };
        efficiency
    }

    /// Check if market needs optimization
    pub fn needs_optimization(market: &Market, config: &StorageConfig) -> bool {
        let size = StorageOptimizer::calculate_market_size(market);
        size as u64 > config.max_storage_per_market
    }

    /// Get storage recommendations for a market
    pub fn get_storage_recommendations(market: &Market) -> Vec<String> {
        let mut recommendations = Vec::new(&market.question.env());

        let size = StorageOptimizer::calculate_market_size(market);
        if size > 10000 {
            recommendations.push_back(String::from_str(
                &market.question.env(),
                "Consider compression for large market data",
            ));
        }

        if market.votes.len() > 1000 {
            recommendations.push_back(String::from_str(
                &market.question.env(),
                "High vote count - consider vote aggregation",
            ));
        }

        if market.question.len() > 200 {
            recommendations.push_back(String::from_str(
                &market.question.env(),
                "Long question - consider shortening",
            ));
        }

        recommendations
    }
}

// ===== STORAGE TESTING =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::storage::Persistent;
    use soroban_sdk::testutils::{Address as _, EnvTestConfig, Ledger};

    #[test]
    fn test_sub_balance_rejects_overdraw_without_mutation() {
        let mut env = Env::default();
        env.set_config(EnvTestConfig {
            capture_snapshot_at_drop: false,
        });
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let user = soroban_sdk::Address::generate(&env);
        let asset = ReflectorAsset::Stellar;

        env.as_contract(&contract_id, || {
            BalanceStorage::add_balance(&env, &user, &asset, 250).unwrap();

            let result = BalanceStorage::sub_balance(&env, &user, &asset, 251);

            assert_eq!(result, Err(Error::InsufficientBalance));
            assert_eq!(BalanceStorage::get_balance(&env, &user, &asset).amount, 250);
        });
    }

    #[test]
    fn test_balance_mutators_reject_non_positive_amounts() {
        let mut env = Env::default();
        env.set_config(EnvTestConfig {
            capture_snapshot_at_drop: false,
        });
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let user = soroban_sdk::Address::generate(&env);
        let asset = ReflectorAsset::Stellar;

        env.as_contract(&contract_id, || {
            assert_eq!(
                BalanceStorage::add_balance(&env, &user, &asset, 0),
                Err(Error::InvalidInput)
            );
            assert_eq!(
                BalanceStorage::sub_balance(&env, &user, &asset, -1),
                Err(Error::InvalidInput)
            );
            assert_eq!(BalanceStorage::get_balance(&env, &user, &asset).amount, 0);
        });
    }

    fn create_test_market(env: &Env) -> Market {
        let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(env);
        Market::new(
            env,
            admin,
            String::from_str(env, "Test market question"),
            Vec::from_array(
                env,
                [String::from_str(env, "yes"), String::from_str(env, "no")],
            ),
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::reflector(),
                soroban_sdk::Address::from_str(
                    env,
                    "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
                ),
                String::from_str(env, "BTC"),
                2500000,
                String::from_str(env, "gt"),
            ),
            None,
            86400,
            MarketState::Active,
        )
    }

    fn create_test_event(env: &Env) -> Event {
        let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(env);
        Event {
            id: Symbol::new(env, "event_ttl"),
            description: String::from_str(env, "Event ttl test"),
            outcomes: soroban_sdk::vec![env, String::from_str(env, "Yes")],
            end_time: env.ledger().timestamp() + 86400,
            oracle_config: OracleConfig::none_sentinel(env),
            has_fallback: false,
            fallback_oracle_config: OracleConfig::none_sentinel(env),
            resolution_timeout: 86400,
            admin,
            created_at: env.ledger().timestamp(),
            status: MarketState::Active,
            visibility: EventVisibility::Public,
            allowlist: Vec::new(env),
        }
    }

    fn create_contract_env() -> (Env, soroban_sdk::Address) {
        let env = Env::default();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        (env, contract_id)
    }

    #[test]
    fn test_storage_optimizer_compression() {
        let env = Env::default();
        let market = create_test_market(&env);

        let compressed = StorageOptimizer::compress_market_data(&env, &market).unwrap();
        assert!(compressed.compressed_size < compressed.original_size);
        assert_eq!(
            compressed.compression_type,
            String::from_str(&env, "simple_optimization")
        );
    }

    #[test]
    fn test_storage_usage_monitoring() {
        let env = Env::default();
        let stats = StorageOptimizer::monitor_storage_usage(&env).unwrap();
        assert_eq!(stats.total_markets, 0);
        assert_eq!(stats.total_storage_bytes, 0);
    }

    #[test]
    fn test_storage_config_exposes_ttl_tiers() {
        let (env, contract_id) = create_contract_env();
        env.as_contract(&contract_id, || {
            let config = StorageOptimizer::get_storage_config(&env);
            assert!(config.compression_enabled);
            assert_eq!(config.cleanup_threshold_days, 365);
            assert_eq!(config.max_storage_per_market, 1024 * 1024);
            assert_eq!(config.balance_ttl_ledgers, BALANCE_TTL_LEDGERS);
            assert_eq!(config.market_ttl_ledgers, MARKET_TTL_LEDGERS);
            assert_eq!(config.event_ttl_ledgers, EVENT_TTL_LEDGERS);
            assert_eq!(config.archive_ttl_ledgers, ARCHIVE_TTL_LEDGERS);
        });
    }

    #[test]
    fn test_balance_storage_extends_ttl_on_each_write() {
        let (env, contract_id) = create_contract_env();
        let user = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let asset = ReflectorAsset::BTC;

        env.as_contract(&contract_id, || {
            let balance = Balance {
                user: user.clone(),
                asset: asset.clone(),
                amount: 10,
            };
            BalanceStorage::set_balance(&env, &balance);

            let key = BalanceStorage::get_key(&env, &user, &asset);
            let expected_ttl = StorageOptimizer::persistent_ttl_for_tier(&env, StorageTtlTier::Balance);
            assert_eq!(env.storage().persistent().get_ttl(&key), expected_ttl);

            env.ledger().with_mut(|li| {
                li.sequence_number += 500;
            });
            assert!(env.storage().persistent().get_ttl(&key) < expected_ttl);

            let updated_balance = Balance {
                amount: 20,
                ..balance
            };
            BalanceStorage::set_balance(&env, &updated_balance);
            assert_eq!(env.storage().persistent().get_ttl(&key), expected_ttl);
        });
    }

    #[test]
    fn test_event_storage_uses_event_ttl_tier() {
        let (env, contract_id) = create_contract_env();
        let event = create_test_event(&env);

        env.as_contract(&contract_id, || {
            EventManager::store_event(&env, &event);
            let key = EventManager::event_storage_key(&env, &event.id);
            let expected_ttl = StorageOptimizer::persistent_ttl_for_tier(&env, StorageTtlTier::Event);
            assert_eq!(env.storage().persistent().get_ttl(&key), expected_ttl);
        });
    }

    #[test]
    fn test_archive_storage_extends_when_rewritten_near_expiry() {
        let (env, contract_id) = create_contract_env();

        env.as_contract(&contract_id, || {
            let mut config = StorageOptimizer::get_storage_config(&env);
            config.archive_ttl_ledgers = ARCHIVE_TTL_LEDGERS;
            StorageOptimizer::update_storage_config(&env, &config).unwrap();

            let archive_key = Symbol::new(&env, STORAGE_CONFIG_KEY);
            let initial_ttl = env.storage().persistent().get_ttl(&archive_key);

            env.ledger().with_mut(|li| {
                li.sequence_number += 500;
            });
            let near_expiry_ttl = env.storage().persistent().get_ttl(&archive_key);
            assert!(near_expiry_ttl < initial_ttl);

            StorageOptimizer::update_storage_config(&env, &config).unwrap();
            let refreshed_ttl = env.storage().persistent().get_ttl(&archive_key);

            assert!(refreshed_ttl > near_expiry_ttl);
            assert_eq!(refreshed_ttl, initial_ttl);
        });
    }

    #[test]
    fn test_storage_utils() {
        let env = Env::default();
        let market = create_test_market(&env);

        let efficiency = StorageUtils::get_storage_efficiency_score(&market);
        assert!(efficiency > 0);
        assert!(efficiency <= 100);

        let recommendations = StorageUtils::get_storage_recommendations(&market);
        // Recommendations may be empty for small markets, so we just check it doesn't panic
        // len() is always >= 0 for Vec
    }
}
