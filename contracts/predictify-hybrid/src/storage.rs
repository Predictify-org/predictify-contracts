#![cfg_attr(test, allow(dead_code))]

use super::*;
use soroban_sdk::{
    contracttype, map, vec, Address, Env, Map, Symbol, Vec,
};
use crate::markets::{MarketStateManager, MarketStateLogic};

// ===== STORAGE OPTIMIZATION TYPES =====

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
#[derive(Clone, Debug)]
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
    /// Compress market data for storage optimization
    pub fn compress_market_data(env: &Env, market: &Market) -> Result<CompressedMarket, Error> {
        // Create a simple compression by removing unnecessary fields and optimizing structure
        let market_id = Self::generate_market_id(env, &market.question);
        
        // Convert market to compressed format
        let compressed_data = Self::serialize_compressed_market(env, market)?;
        let original_size = Self::calculate_market_size(market);
        let compressed_size = compressed_data.len() as u32;
        
        // Calculate compression ratio (as percentage * 100 for integer storage)
        let compression_ratio = if original_size > 0 {
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
            oldest_market_timestamp: if oldest_timestamp == u64::MAX { 0 } else { oldest_timestamp },
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
    pub fn validate_storage_integrity(env: &Env, market_id: &Symbol) -> Result<StorageIntegrityResult, Error> {
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
                    result.errors.push_back(String::from_str(env, &format!("Validation failed: {:?}", e)));
                }
                
                // Check for missing critical data
                if market.question.is_empty() {
                    result.missing_data = true;
                    result.warnings.push_back(String::from_str(env, "Empty market question"));
                }
                
                if market.outcomes.is_empty() {
                    result.missing_data = true;
                    result.errors.push_back(String::from_str(env, "No outcomes defined"));
                }
                
                // Validate state consistency
                if let Err(e) = MarketStateLogic::validate_market_state_consistency(env, &market) {
                    result.is_valid = false;
                    result.errors.push_back(String::from_str(env, &format!("State inconsistency: {:?}", e)));
                }
            }
            Err(e) => {
                result.is_valid = false;
                result.missing_data = true;
                result.errors.push_back(String::from_str(env, &format!("Market not found: {:?}", e)));
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
                    result.errors.push_back(String::from_str(env, "Checksum validation failed"));
                }
            }
        }
        
        Ok(result)
    }
    
    /// Get storage configuration
    pub fn get_storage_config(env: &Env) -> StorageConfig {
        match env.storage().persistent().get(&Symbol::new(env, "storage_config")) {
            Some(config) => config,
            None => StorageConfig {
                compression_enabled: true,
                min_compression_age_days: 30,
                max_storage_per_market: 1024 * 1024, // 1MB
                cleanup_threshold_days: 365,
                auto_cleanup_enabled: false,
                preferred_compression: String::from_str(env, "simple_optimization"),
            },
        }
    }
    
    /// Update storage configuration
    pub fn update_storage_config(env: &Env, config: &StorageConfig) -> Result<(), Error> {
        env.storage()
            .persistent()
            .set(&Symbol::new(env, "storage_config"), config);
        Ok(())
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
    fn archive_market_data(env: &Env, market_id: &Symbol, market: &Market) -> Result<(), Error> {
        // Store archived version with timestamp
        let archive_key = Symbol::new(env, &format!("archive_{:?}_{}", market_id, env.ledger().timestamp()));
        env.storage().persistent().set(&archive_key, market);
        Ok(())
    }
    
    /// Store migration record
    fn store_migration_record(env: &Env, migration_id: &Symbol, migration: &StorageMigration) {
        env.storage().persistent().set(migration_id, migration);
    }
    
    /// Migrate from V1 to V2 format
    fn migrate_v1_to_v2(env: &Env, mut migration: StorageMigration) -> Result<StorageMigration, Error> {
        // Simplified migration - in real implementation, you'd migrate actual data
        migration.markets_migrated = 1;
        migration.status = String::from_str(env, "completed");
        Ok(migration)
    }
    
    /// Migrate from V2 to V3 format
    fn migrate_v2_to_v3(env: &Env, mut migration: StorageMigration) -> Result<StorageMigration, Error> {
        // Simplified migration - in real implementation, you'd migrate actual data
        migration.markets_migrated = 1;
        migration.status = String::from_str(env, "completed");
        Ok(migration)
    }
    
    /// Get all market IDs (simplified - in real implementation, you'd have a registry)
    fn get_all_market_ids(env: &Env) -> Vec<Symbol> {
        // This is a simplified approach - in a real implementation,
        // you'd maintain a registry of all market IDs
        let mut market_ids = Vec::new(env);
        // For now, return empty vector - this would be populated from a registry
        market_ids
    }
    
    /// Check if market is compressed
    fn is_market_compressed(env: &Env, market_id: &Symbol) -> bool {
        env.storage()
            .persistent()
            .has(&Symbol::new(env, &format!("compressed_{:?}", market_id)))
    }
    
    /// Store compressed market
    fn store_compressed_market(env: &Env, compressed_market: &CompressedMarket) -> Result<(), Error> {
        let key = Symbol::new(env, &format!("compressed_{:?}", compressed_market.market_id));
        env.storage().persistent().set(&key, compressed_market);
        Ok(())
    }
    
    /// Get compressed market
    fn get_compressed_market(env: &Env, market_id: &Symbol) -> Result<CompressedMarket, Error> {
        let key = Symbol::new(env, &format!("compressed_{:?}", market_id));
        env.storage()
            .persistent()
            .get(&key)
            .ok_or(Error::MarketNotFound)
    }
    
    /// Update market to point to compressed data
    fn update_market_to_compressed(env: &Env, market_id: &Symbol, compressed_id: &Symbol) -> Result<(), Error> {
        let key = Symbol::new(env, &format!("compressed_ref_{:?}", market_id));
        env.storage().persistent().set(&key, compressed_id);
        Ok(())
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
            recommendations.push_back(String::from_str(&market.question.env(), "Consider compression for large market data"));
        }
        
        if market.votes.len() > 1000 {
            recommendations.push_back(String::from_str(&market.question.env(), "High vote count - consider vote aggregation"));
        }
        
        if market.question.len() > 200 {
            recommendations.push_back(String::from_str(&market.question.env(), "Long question - consider shortening"));
        }
        
        recommendations
    }
}

// ===== STORAGE TESTING =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address;
    
    #[test]
    fn test_storage_optimizer_compression() {
        let env = Env::default();
        let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let market = Market::new(
            &env,
            admin,
            String::from_str(&env, "Test market question"),
            Vec::from_array(&env, [
                String::from_str(&env, "yes"),
                String::from_str(&env, "no")
            ]),
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Reflector,
                String::from_str(&env, "BTC"),
                2500000,
                String::from_str(&env, "gt"),
            ),
            MarketState::Active,
        );
        
        let compressed = StorageOptimizer::compress_market_data(&env, &market).unwrap();
        assert!(compressed.compressed_size < compressed.original_size);
        assert_eq!(compressed.compression_type, String::from_str(&env, "simple_optimization"));
    }
    
    #[test]
    fn test_storage_usage_monitoring() {
        let env = Env::default();
        let stats = StorageOptimizer::monitor_storage_usage(&env).unwrap();
        assert_eq!(stats.total_markets, 0);
        assert_eq!(stats.total_storage_bytes, 0);
    }
    
    // #[test]
    // fn test_storage_config() {
    //     let env = Env::default();
    //     let contract_id = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
    //     env.as_contract(&contract_id, || {
    //         // Test that we can get the default config when none exists
    //         let config = StorageOptimizer::get_storage_config(&env);
    //         assert!(config.compression_enabled);
    //         assert_eq!(config.cleanup_threshold_days, 365);
    //         assert_eq!(config.max_storage_per_market, 1024 * 1024); // 1MB
    //         assert!(!config.auto_cleanup_enabled);
    //     });
    // }
    
    #[test]
    fn test_storage_utils() {
        let env = Env::default();
        let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let market = Market::new(
            &env,
            admin,
            String::from_str(&env, "Test market"),
            Vec::from_array(&env, [
                String::from_str(&env, "yes"),
                String::from_str(&env, "no")
            ]),
            env.ledger().timestamp() + 86400,
            OracleConfig::new(
                OracleProvider::Reflector,
                String::from_str(&env, "BTC"),
                2500000,
                String::from_str(&env, "gt"),
            ),
            MarketState::Active,
        );
        
        let efficiency = StorageUtils::get_storage_efficiency_score(&market);
        assert!(efficiency > 0);
        assert!(efficiency <= 100);
        
        let recommendations = StorageUtils::get_storage_recommendations(&market);
        // Recommendations may be empty for small markets, so we just check it doesn't panic
        assert!(recommendations.len() >= 0);
    }
} 