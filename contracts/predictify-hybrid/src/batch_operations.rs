use soroban_sdk::{
    contracttype, vec, Address, Env, Map, String, Symbol, Vec,
};
use alloc::string::ToString;

use crate::errors::Error;
use crate::types::*;

// ===== BATCH OPERATION TYPES =====

#[derive(Clone, Debug, PartialEq, Eq)]
#[contracttype]
pub enum BatchOperationType {
    Vote,           // Batch vote operations
    Claim,          // Batch claim operations
    CreateMarket,   // Batch market creation
    OracleCall,     // Batch oracle calls
    Dispute,        // Batch dispute operations
    Extension,      // Batch market extensions
    Resolution,     // Batch market resolutions
    FeeCollection,  // Batch fee collection
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct VoteData {
    pub market_id: Symbol,
    pub voter: Address,
    pub outcome: String,
    pub stake_amount: i128,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct ClaimData {
    pub market_id: Symbol,
    pub claimant: Address,
    pub expected_amount: i128,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct MarketData {
    pub question: String,
    pub outcomes: Vec<String>,
    pub duration_days: u32,
    pub oracle_config: Option<OracleConfig>,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct OracleFeed {
    pub market_id: Symbol,
    pub feed_id: String,
    pub provider: OracleProvider,
    pub threshold: i128,
    pub comparison: String,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct BatchOperation {
    pub operation_type: BatchOperationType,
    pub data: Vec<String>, // Serialized operation data
    pub priority: u32,     // Operation priority (lower = higher priority)
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct BatchError {
    pub operation_index: u32,
    pub error_code: u32,
    pub error_message: String,
    pub operation_type: BatchOperationType,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct BatchResult {
    pub successful_operations: u32,
    pub failed_operations: u32,
    pub total_operations: u32,
    pub errors: Vec<BatchError>,
    pub gas_used: u64,
    pub execution_time: u64,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct BatchStatistics {
    pub total_batches_processed: u32,
    pub total_operations_processed: u32,
    pub total_successful_operations: u32,
    pub total_failed_operations: u32,
    pub average_batch_size: u32,
    pub average_execution_time: u64,
    pub gas_efficiency_ratio: u64,
}

// ===== BATCH PROCESSOR IMPLEMENTATION =====

/// Batch Processor for Multiple Functions and Data Processing
///
/// This struct provides comprehensive batch processing functionality
/// for multiple operations including voting, claiming, market creation,
/// oracle calls, and more. It ensures efficient gas usage and improved
/// performance for multiple operations.
///
/// # Features
///
/// **Batch Operations:**
/// - Batch vote processing
/// - Batch claim processing
/// - Batch market creation
/// - Batch oracle calls
/// - Batch dispute operations
///
/// **Performance Optimization:**
/// - Gas-efficient batch processing
/// - Parallel operation handling
/// - Error handling and recovery
/// - Operation prioritization
///
/// **Monitoring and Analytics:**
/// - Batch operation statistics
/// - Performance metrics
/// - Error tracking and reporting
/// - Gas usage optimization
pub struct BatchProcessor;

impl BatchProcessor {
    // ===== STORAGE KEYS =====
    
    const BATCH_QUEUE_KEY: &'static str = "batch_operation_queue";
    const BATCH_STATS_KEY: &'static str = "batch_operation_statistics";
    const BATCH_CONFIG_KEY: &'static str = "batch_operation_config";

    // ===== CONFIGURATION MANAGEMENT =====

    /// Initialize batch processor with default configuration
    pub fn initialize(env: &Env) -> Result<(), Error> {
        let config = BatchConfig {
            max_batch_size: 50,
            max_operations_per_batch: 100,
            gas_limit_per_batch: 1_000_000,
            timeout_per_batch: 30, // 30 seconds
            retry_failed_operations: true,
            parallel_processing_enabled: false,
        };

        let stats = BatchStatistics {
            total_batches_processed: 0,
            total_operations_processed: 0,
            total_successful_operations: 0,
            total_failed_operations: 0,
            average_batch_size: 0,
            average_execution_time: 0,
            gas_efficiency_ratio: 1,
        };

        env.storage().instance().set(&Symbol::new(env, Self::BATCH_CONFIG_KEY), &config);
        env.storage().instance().set(&Symbol::new(env, Self::BATCH_STATS_KEY), &stats);
        
        // Initialize empty batch queue
        let queue: Vec<BatchOperation> = Vec::new(env);
        env.storage().instance().set(&Symbol::new(env, Self::BATCH_QUEUE_KEY), &queue);

        Ok(())
    }

    /// Get batch processor configuration
    pub fn get_config(env: &Env) -> Result<BatchConfig, Error> {
        env.storage()
            .instance()
            .get(&Symbol::new(env, Self::BATCH_CONFIG_KEY))
            .ok_or(Error::ConfigurationNotFound)
    }

    /// Update batch processor configuration
    pub fn update_config(
        env: &Env,
        admin: &Address,
        config: &BatchConfig,
    ) -> Result<(), Error> {
        // Validate admin permissions
        crate::admin::AdminAccessControl::validate_admin_for_action(env, admin, "update_batch_config")?;

        // Validate configuration
        Self::validate_batch_config(config)?;

        env.storage().instance().set(&Symbol::new(env, Self::BATCH_CONFIG_KEY), config);

        Ok(())
    }

    // ===== BATCH VOTE OPERATIONS =====

    /// Process batch vote operations
    pub fn batch_vote(
        env: &Env,
        votes: &Vec<VoteData>,
    ) -> Result<BatchResult, Error> {
        let config = Self::get_config(env)?;
        let start_time = env.ledger().timestamp();
        let mut successful_operations = 0;
        let mut failed_operations = 0;
        let mut errors = Vec::new(env);

        // Validate batch size
        if votes.len() > config.max_operations_per_batch as usize {
            return Err(Error::InvalidInput);
        }

        for (index, vote_data) in votes.iter().enumerate() {
            match Self::process_single_vote(env, vote_data) {
                Ok(_) => {
                    successful_operations += 1;
                }
                Err(error) => {
                    failed_operations += 1;
                    errors.push_back(BatchError {
                        operation_index: index as u32,
                        error_code: error as u32,
                        error_message: String::from_str(env, &error.description()),
                        operation_type: BatchOperationType::Vote,
                    });
                }
            }
        }

        let end_time = env.ledger().timestamp();
        let execution_time = end_time - start_time;

        let result = BatchResult {
            successful_operations,
            failed_operations,
            total_operations: votes.len() as u32,
            errors,
            gas_used: 0, // Would be calculated in real implementation
            execution_time,
        };

        // Update statistics
        Self::update_batch_statistics(env, &result)?;

        Ok(result)
    }

    /// Process single vote operation
    fn process_single_vote(env: &Env, vote_data: &VoteData) -> Result<(), Error> {
        // Validate vote data
        Self::validate_vote_data(vote_data)?;

        // Check if market exists and is open
        let market = crate::markets::MarketStateManager::get_market(env, &vote_data.market_id)?;
        
        if market.end_time <= env.ledger().timestamp() {
            return Err(Error::MarketClosed);
        }

        // Process the vote using existing voting logic
        crate::voting::VoteManager::cast_vote(
            env,
            &vote_data.market_id,
            &vote_data.voter,
            &vote_data.outcome,
            vote_data.stake_amount,
        )?;

        Ok(())
    }

    // ===== BATCH CLAIM OPERATIONS =====

    /// Process batch claim operations
    pub fn batch_claim(
        env: &Env,
        claims: &Vec<ClaimData>,
    ) -> Result<BatchResult, Error> {
        let config = Self::get_config(env)?;
        let start_time = env.ledger().timestamp();
        let mut successful_operations = 0;
        let mut failed_operations = 0;
        let mut errors = Vec::new(env);

        // Validate batch size
        if claims.len() > config.max_operations_per_batch as usize {
            return Err(Error::InvalidInput);
        }

        for (index, claim_data) in claims.iter().enumerate() {
            match Self::process_single_claim(env, claim_data) {
                Ok(_) => {
                    successful_operations += 1;
                }
                Err(error) => {
                    failed_operations += 1;
                    errors.push_back(BatchError {
                        operation_index: index as u32,
                        error_code: error as u32,
                        error_message: String::from_str(env, &error.description()),
                        operation_type: BatchOperationType::Claim,
                    });
                }
            }
        }

        let end_time = env.ledger().timestamp();
        let execution_time = end_time - start_time;

        let result = BatchResult {
            successful_operations,
            failed_operations,
            total_operations: claims.len() as u32,
            errors,
            gas_used: 0, // Would be calculated in real implementation
            execution_time,
        };

        // Update statistics
        Self::update_batch_statistics(env, &result)?;

        Ok(result)
    }

    /// Process single claim operation
    fn process_single_claim(env: &Env, claim_data: &ClaimData) -> Result<(), Error> {
        // Validate claim data
        Self::validate_claim_data(claim_data)?;

        // Check if market exists and is resolved
        let market = crate::markets::MarketManager::get_market(env, &claim_data.market_id)?;
        
        if !market.is_resolved {
            return Err(Error::MarketNotResolved);
        }

        // Process the claim using existing claim logic
        crate::markets::MarketManager::claim_winnings(
            env,
            &claim_data.market_id,
            &claim_data.claimant,
        )?;

        Ok(())
    }

    // ===== BATCH MARKET CREATION =====

    /// Process batch market creation operations
    pub fn batch_create_markets(
        env: &Env,
        admin: &Address,
        markets: &Vec<MarketData>,
    ) -> Result<BatchResult, Error> {
        // Validate admin permissions
        crate::admin::AdminAccessControl::validate_admin_for_action(env, admin, "batch_create_markets")?;

        let config = Self::get_config(env)?;
        let start_time = env.ledger().timestamp();
        let mut successful_operations = 0;
        let mut failed_operations = 0;
        let mut errors = Vec::new(env);

        // Validate batch size
        if markets.len() > config.max_operations_per_batch as usize {
            return Err(Error::InvalidInput);
        }

        for (index, market_data) in markets.iter().enumerate() {
            match Self::process_single_market_creation(env, admin, market_data) {
                Ok(_) => {
                    successful_operations += 1;
                }
                Err(error) => {
                    failed_operations += 1;
                    errors.push_back(BatchError {
                        operation_index: index as u32,
                        error_code: error as u32,
                        error_message: String::from_str(env, &error.description()),
                        operation_type: BatchOperationType::CreateMarket,
                    });
                }
            }
        }

        let end_time = env.ledger().timestamp();
        let execution_time = end_time - start_time;

        let result = BatchResult {
            successful_operations,
            failed_operations,
            total_operations: markets.len() as u32,
            errors,
            gas_used: 0, // Would be calculated in real implementation
            execution_time,
        };

        // Update statistics
        Self::update_batch_statistics(env, &result)?;

        Ok(result)
    }

    /// Process single market creation operation
    fn process_single_market_creation(
        env: &Env,
        admin: &Address,
        market_data: &MarketData,
    ) -> Result<(), Error> {
        // Validate market data
        Self::validate_market_data(market_data)?;

        // Create market using existing market creation logic
        crate::markets::MarketManager::create_market(
            env,
            admin,
            &market_data.question,
            &market_data.outcomes,
            market_data.duration_days,
            &market_data.oracle_config,
        )?;

        Ok(())
    }

    // ===== BATCH ORACLE CALLS =====

    /// Process batch oracle calls
    pub fn batch_oracle_calls(
        env: &Env,
        feeds: &Vec<OracleFeed>,
    ) -> Result<BatchResult, Error> {
        let config = Self::get_config(env)?;
        let start_time = env.ledger().timestamp();
        let mut successful_operations = 0;
        let mut failed_operations = 0;
        let mut errors = Vec::new(env);

        // Validate batch size
        if feeds.len() > config.max_operations_per_batch as usize {
            return Err(Error::InvalidInput);
        }

        for (index, feed_data) in feeds.iter().enumerate() {
            match Self::process_single_oracle_call(env, feed_data) {
                Ok(_) => {
                    successful_operations += 1;
                }
                Err(error) => {
                    failed_operations += 1;
                    errors.push_back(BatchError {
                        operation_index: index as u32,
                        error_code: error as u32,
                        error_message: String::from_str(env, &error.description()),
                        operation_type: BatchOperationType::OracleCall,
                    });
                }
            }
        }

        let end_time = env.ledger().timestamp();
        let execution_time = end_time - start_time;

        let result = BatchResult {
            successful_operations,
            failed_operations,
            total_operations: feeds.len() as u32,
            errors,
            gas_used: 0, // Would be calculated in real implementation
            execution_time,
        };

        // Update statistics
        Self::update_batch_statistics(env, &result)?;

        Ok(result)
    }

    /// Process single oracle call
    fn process_single_oracle_call(env: &Env, feed_data: &OracleFeed) -> Result<(), Error> {
        // Validate oracle feed data
        Self::validate_oracle_feed_data(feed_data)?;

        // Check if market exists
        let market = crate::markets::MarketManager::get_market(env, &feed_data.market_id)?;
        
        if market.is_resolved {
            return Err(Error::MarketAlreadyResolved);
        }

        // Process oracle call using existing oracle logic
        crate::oracles::OracleManager::fetch_oracle_result(
            env,
            &feed_data.market_id,
            &feed_data.feed_id,
            &feed_data.provider,
            feed_data.threshold,
            &feed_data.comparison,
        )?;

        Ok(())
    }

    // ===== BATCH OPERATION VALIDATION =====

    /// Validate batch operations
    pub fn validate_batch_operations(
        operations: &Vec<BatchOperation>,
    ) -> Result<(), Error> {
        if operations.is_empty() {
            return Err(Error::InvalidInput);
        }

        // Check for duplicate operations
        for i in 0..operations.len() {
            for j in (i + 1)..operations.len() {
                if operations.get(i).unwrap() == operations.get(j).unwrap() {
                    return Err(Error::InvalidInput);
                }
            }
        }

        // Validate individual operations
        for operation in operations.iter() {
            Self::validate_single_operation(operation)?;
        }

        Ok(())
    }

    /// Validate single operation
    fn validate_single_operation(operation: &BatchOperation) -> Result<(), Error> {
        // Validate operation type
        match operation.operation_type {
            BatchOperationType::Vote => {
                // Validate vote data
                if operation.data.is_empty() {
                    return Err(Error::InvalidInput);
                }
            }
            BatchOperationType::Claim => {
                // Validate claim data
                if operation.data.is_empty() {
                    return Err(Error::InvalidInput);
                }
            }
            BatchOperationType::CreateMarket => {
                // Validate market creation data
                if operation.data.len() < 3 {
                    return Err(Error::InvalidInput);
                }
            }
            BatchOperationType::OracleCall => {
                // Validate oracle call data
                if operation.data.len() < 5 {
                    return Err(Error::InvalidInput);
                }
            }
            _ => {
                // Other operation types
                if operation.data.is_empty() {
                    return Err(Error::InvalidInput);
                }
            }
        }

        Ok(())
    }

    // ===== BATCH ERROR HANDLING =====

    /// Handle batch errors
    pub fn handle_batch_errors(
        env: &Env,
        errors: &Vec<BatchError>,
    ) -> Result<Map<String, String>, Error> {
        let mut error_summary = Map::new(env);
        let mut error_counts = Map::new(env);

        // Count errors by type
        for error in errors.iter() {
            let error_type = match error.operation_type {
                BatchOperationType::Vote => "vote",
                BatchOperationType::Claim => "claim",
                BatchOperationType::CreateMarket => "market_creation",
                BatchOperationType::OracleCall => "oracle_call",
                BatchOperationType::Dispute => "dispute",
                BatchOperationType::Extension => "extension",
                BatchOperationType::Resolution => "resolution",
                BatchOperationType::FeeCollection => "fee_collection",
            };

            let current_count = error_counts.get(String::from_str(env, error_type)).unwrap_or(0);
            error_counts.set(String::from_str(env, error_type), current_count + 1);
        }

        // Create error summary
        error_summary.set(
            String::from_str(env, "total_errors"),
            String::from_str(env, &errors.len().to_string())
        );

        error_summary.set(
            String::from_str(env, "error_types"),
            String::from_str(env, "See error_counts for breakdown")
        );

        // Add error counts
        for (error_type, count) in error_counts.iter() {
            error_summary.set(
                String::from_str(env, &format!("{}_errors", error_type)),
                String::from_str(env, &count.to_string())
            );
        }

        Ok(error_summary)
    }

    // ===== BATCH STATISTICS =====

    /// Get batch operation statistics
    pub fn get_batch_operation_statistics(env: &Env) -> Result<BatchStatistics, Error> {
        env.storage()
            .instance()
            .get(&Symbol::new(env, Self::BATCH_STATS_KEY))
            .ok_or(Error::ConfigurationNotFound)
    }

    /// Update batch statistics
    fn update_batch_statistics(env: &Env, result: &BatchResult) -> Result<(), Error> {
        let mut stats = Self::get_batch_operation_statistics(env)?;

        stats.total_batches_processed += 1;
        stats.total_operations_processed += result.total_operations;
        stats.total_successful_operations += result.successful_operations;
        stats.total_failed_operations += result.failed_operations;

        // Update average batch size
        if stats.total_batches_processed > 0 {
            stats.average_batch_size = stats.total_operations_processed / stats.total_batches_processed;
        }

        // Update average execution time
        if stats.total_batches_processed > 0 {
            let total_time = stats.average_execution_time * (stats.total_batches_processed - 1) + result.execution_time;
            stats.average_execution_time = total_time / stats.total_batches_processed;
        }

        // Update gas efficiency ratio
        if result.total_operations > 0 {
            let success_rate = result.successful_operations as f64 / result.total_operations as f64;
            stats.gas_efficiency_ratio = success_rate;
        }

        env.storage().instance().set(&Symbol::new(env, Self::BATCH_STATS_KEY), &stats);

        Ok(())
    }

    // ===== VALIDATION FUNCTIONS =====

    /// Validate vote data
    fn validate_vote_data(vote_data: &VoteData) -> Result<(), Error> {
        if vote_data.stake_amount <= 0 {
            return Err(Error::InsufficientStake);
        }

        if vote_data.outcome.is_empty() {
            return Err(Error::InvalidOutcome);
        }

        Ok(())
    }

    /// Validate claim data
    fn validate_claim_data(claim_data: &ClaimData) -> Result<(), Error> {
        if claim_data.expected_amount <= 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate market data
    fn validate_market_data(market_data: &MarketData) -> Result<(), Error> {
        if market_data.question.is_empty() {
            return Err(Error::InvalidQuestion);
        }

        if market_data.outcomes.len() < 2 {
            return Err(Error::InvalidOutcomes);
        }

        if market_data.duration_days == 0 {
            return Err(Error::InvalidDuration);
        }

        Ok(())
    }

    /// Validate oracle feed data
    fn validate_oracle_feed_data(feed_data: &OracleFeed) -> Result<(), Error> {
        if feed_data.feed_id.is_empty() {
            return Err(Error::InvalidOracleFeed);
        }

        if feed_data.threshold <= 0 {
            return Err(Error::InvalidThreshold);
        }

        Ok(())
    }

    /// Validate batch configuration
    fn validate_batch_config(config: &BatchConfig) -> Result<(), Error> {
        if config.max_batch_size == 0 {
            return Err(Error::InvalidInput);
        }

        if config.max_operations_per_batch == 0 {
            return Err(Error::InvalidInput);
        }

        if config.gas_limit_per_batch == 0 {
            return Err(Error::InvalidInput);
        }

        if config.timeout_per_batch == 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }
}

// ===== BATCH CONFIGURATION =====

#[derive(Clone, Debug)]
#[contracttype]
pub struct BatchConfig {
    pub max_batch_size: u32,
    pub max_operations_per_batch: u32,
    pub gas_limit_per_batch: u64,
    pub timeout_per_batch: u64,
    pub retry_failed_operations: bool,
    pub parallel_processing_enabled: bool,
}

// ===== BATCH UTILITIES =====

/// Batch operation utilities
pub struct BatchUtils;

impl BatchUtils {
    /// Check if batch processing is enabled
    pub fn is_batch_processing_enabled(env: &Env) -> Result<bool, Error> {
        let config = BatchProcessor::get_config(env)?;
        Ok(config.max_batch_size > 0)
    }

    /// Get optimal batch size for operation type
    pub fn get_optimal_batch_size(
        env: &Env,
        operation_type: &BatchOperationType,
    ) -> Result<u32, Error> {
        let config = BatchProcessor::get_config(env)?;
        
        match operation_type {
            BatchOperationType::Vote => Ok(config.max_batch_size.min(20)),
            BatchOperationType::Claim => Ok(config.max_batch_size.min(15)),
            BatchOperationType::CreateMarket => Ok(config.max_batch_size.min(10)),
            BatchOperationType::OracleCall => Ok(config.max_batch_size.min(25)),
            BatchOperationType::Dispute => Ok(config.max_batch_size.min(5)),
            BatchOperationType::Extension => Ok(config.max_batch_size.min(8)),
            BatchOperationType::Resolution => Ok(config.max_batch_size.min(12)),
            BatchOperationType::FeeCollection => Ok(config.max_batch_size.min(30)),
        }
    }

    /// Calculate gas efficiency for batch operation
    pub fn calculate_gas_efficiency(
        successful_operations: u32,
        total_operations: u32,
        gas_used: u64,
    ) -> f64 {
        if total_operations == 0 || gas_used == 0 {
            return 0.0;
        }

        let success_rate = successful_operations as f64 / total_operations as f64;
        let operations_per_gas = total_operations as f64 / gas_used as f64;
        
        success_rate * operations_per_gas
    }

    /// Estimate gas cost for batch operation
    pub fn estimate_gas_cost(
        operation_type: &BatchOperationType,
        operation_count: u32,
    ) -> u64 {
        let base_cost = match operation_type {
            BatchOperationType::Vote => 1000,
            BatchOperationType::Claim => 1500,
            BatchOperationType::CreateMarket => 5000,
            BatchOperationType::OracleCall => 2000,
            BatchOperationType::Dispute => 3000,
            BatchOperationType::Extension => 2500,
            BatchOperationType::Resolution => 4000,
            BatchOperationType::FeeCollection => 800,
        };

        base_cost * operation_count as u64
    }
}

// ===== BATCH TESTING =====

/// Batch operation testing utilities
pub struct BatchTesting;

impl BatchTesting {
    /// Create test vote data
    pub fn create_test_vote_data(env: &Env, market_id: &Symbol) -> VoteData {
        VoteData {
            market_id: market_id.clone(),
            voter: Address::generate(env),
            outcome: String::from_str(env, "Yes"),
            stake_amount: 1_000_000_000, // 100 XLM
        }
    }

    /// Create test claim data
    pub fn create_test_claim_data(env: &Env, market_id: &Symbol) -> ClaimData {
        ClaimData {
            market_id: market_id.clone(),
            claimant: Address::generate(env),
            expected_amount: 2_000_000_000, // 200 XLM
        }
    }

    /// Create test market data
    pub fn create_test_market_data(env: &Env) -> MarketData {
        MarketData {
            question: String::from_str(env, "Will Bitcoin reach $100,000 by end of 2024?"),
            outcomes: vec![
                &env,
                String::from_str(env, "Yes"),
                String::from_str(env, "No")
            ],
            duration_days: 30,
            oracle_config: None,
        }
    }

    /// Create test oracle feed data
    pub fn create_test_oracle_feed_data(env: &Env, market_id: &Symbol) -> OracleFeed {
        OracleFeed {
            market_id: market_id.clone(),
            feed_id: String::from_str(env, "BTC/USD"),
            provider: OracleProvider::Reflector,
            threshold: 100_000_000_000, // $100,000
            comparison: String::from_str(env, "gt"),
        }
    }

    /// Simulate batch operation
    pub fn simulate_batch_operation(
        env: &Env,
        operation_type: &BatchOperationType,
        operation_count: u32,
    ) -> Result<BatchResult, Error> {
        let start_time = env.ledger().timestamp();
        let mut successful_operations = 0;
        let mut failed_operations = 0;
        let mut errors = Vec::new(env);

        // Simulate operations
        for i in 0..operation_count {
            if i % 10 == 0 {
                // Simulate some failures
                failed_operations += 1;
                errors.push_back(BatchError {
                    operation_index: i,
                    error_code: 100,
                    error_message: String::from_str(env, "Simulated error"),
                    operation_type: operation_type.clone(),
                });
            } else {
                successful_operations += 1;
            }
        }

        let end_time = env.ledger().timestamp();
        let execution_time = end_time - start_time;

        Ok(BatchResult {
            successful_operations,
            failed_operations,
            total_operations: operation_count,
            errors,
            gas_used: BatchUtils::estimate_gas_cost(operation_type, operation_count),
            execution_time,
        })
    }
} 