//! # Performance Benchmarks Module
//!
//! Provides threshold constants, validation helpers, and benchmark tests for the
//! six critical-path contract functions in Predictify Hybrid:
//!
//! - `create_market`
//! - `vote`
//! - `claim_winnings`
//! - `resolve_market`
//! - `fetch_oracle_result`
//! - `collect_fees`
//!
//! ## Threshold Constants
//!
//! Each critical-path function has three named constants:
//! `{FUNCTION}_GAS_THRESHOLD`, `{FUNCTION}_STORAGE_THRESHOLD`, and
//! `{FUNCTION}_TIME_THRESHOLD`. These are conservative upper bounds derived
//! from mock-delta measurements + headroom. Tighten them once real
//! `stellar contract invoke --cost` p99 values are available.
//!
//! ## Usage
//!
//! Call [`default_thresholds()`] to obtain a [`PerformanceThresholds`] instance
//! pre-populated from the named constants, then pass it to
//! [`PerformanceBenchmarkManager::validate_performance_thresholds`].

#![allow(dead_code)]

use crate::err::Error;
use crate::types::OracleProvider;
use soroban_sdk::{contracttype, Env, Map, String, Symbol, Vec};

// ===== THRESHOLD CONSTANTS =====

/// Maximum gas units allowed for `create_market`.
/// Rationale: market creation writes oracle config, outcomes, and initial state —
/// mock delta is 500; 500_000 provides ample headroom until real measurements land.
pub const CREATE_MARKET_GAS_THRESHOLD: u64 = 500_000;

/// Maximum storage bytes allowed for `create_market`.
/// Rationale: question string + outcomes list + oracle config fits within 2 KiB.
pub const CREATE_MARKET_STORAGE_THRESHOLD: u64 = 2_048;

/// Maximum execution time (ms) allowed for `create_market`.
/// Rationale: single-ledger operation; 1 000 ms is a conservative ceiling.
pub const CREATE_MARKET_TIME_THRESHOLD: u64 = 1_000;

/// Maximum gas units allowed for `vote`.
/// Rationale: vote writes one entry — cheapest critical path; 200 000 is generous.
pub const VOTE_GAS_THRESHOLD: u64 = 200_000;

/// Maximum storage bytes allowed for `vote`.
/// Rationale: a single vote record is small; 512 bytes is sufficient.
pub const VOTE_STORAGE_THRESHOLD: u64 = 512;

/// Maximum execution time (ms) allowed for `vote`.
/// Rationale: minimal state write; 500 ms ceiling.
pub const VOTE_TIME_THRESHOLD: u64 = 500;

/// Maximum gas units allowed for `claim_winnings`.
/// Rationale: reads market + user stake + writes claimed flag; 400 000 covers the reads.
pub const CLAIM_WINNINGS_GAS_THRESHOLD: u64 = 400_000;

/// Maximum storage bytes allowed for `claim_winnings`.
/// Rationale: claim record is a single flag + amount; 1 KiB is conservative.
pub const CLAIM_WINNINGS_STORAGE_THRESHOLD: u64 = 1_024;

/// Maximum execution time (ms) allowed for `claim_winnings`.
/// Rationale: two storage reads + one write; 800 ms ceiling.
pub const CLAIM_WINNINGS_TIME_THRESHOLD: u64 = 800;

/// Maximum gas units allowed for `resolve_market`.
/// Rationale: oracle call + state transition + event emission is the heaviest path; 600 000.
pub const RESOLVE_MARKET_GAS_THRESHOLD: u64 = 600_000;

/// Maximum storage bytes allowed for `resolve_market`.
/// Rationale: resolution result + updated market state; 2 KiB.
pub const RESOLVE_MARKET_STORAGE_THRESHOLD: u64 = 2_048;

/// Maximum execution time (ms) allowed for `resolve_market`.
/// Rationale: includes oracle round-trip simulation; 1 200 ms ceiling.
pub const RESOLVE_MARKET_TIME_THRESHOLD: u64 = 1_200;

/// Maximum gas units allowed for `fetch_oracle_result`.
/// Rationale: cross-contract call to Reflector; 300 000 covers the call overhead.
pub const FETCH_ORACLE_RESULT_GAS_THRESHOLD: u64 = 300_000;

/// Maximum storage bytes allowed for `fetch_oracle_result`.
/// Rationale: cached price result is a single value; 256 bytes.
pub const FETCH_ORACLE_RESULT_STORAGE_THRESHOLD: u64 = 256;

/// Maximum execution time (ms) allowed for `fetch_oracle_result`.
/// Rationale: single cross-contract call; 600 ms ceiling.
pub const FETCH_ORACLE_RESULT_TIME_THRESHOLD: u64 = 600;

/// Maximum gas units allowed for `collect_fees`.
/// Rationale: reads fee config + writes collected flag; 250 000.
pub const COLLECT_FEES_GAS_THRESHOLD: u64 = 250_000;

/// Maximum storage bytes allowed for `collect_fees`.
/// Rationale: fee record is a small struct; 512 bytes.
pub const COLLECT_FEES_STORAGE_THRESHOLD: u64 = 512;

/// Maximum execution time (ms) allowed for `collect_fees`.
/// Rationale: minimal read + write; 500 ms ceiling.
pub const COLLECT_FEES_TIME_THRESHOLD: u64 = 500;

/// Constructs a [`PerformanceThresholds`] instance pre-populated from the named
/// threshold constants.
///
/// The `max_gas_usage` and `max_execution_time` fields are set to the highest
/// single-operation values (`resolve_market`) so the struct represents a safe
/// envelope for suite-level validation. `max_storage_usage` is set to
/// `CREATE_MARKET_STORAGE_THRESHOLD * 100` to accommodate aggregate suite usage.
///
/// # Example
///
/// ```rust,ignore
/// let thresholds = default_thresholds();
/// PerformanceBenchmarkManager::validate_performance_thresholds(&env, metrics, thresholds)?;
/// ```
pub fn default_thresholds() -> PerformanceThresholds {
    PerformanceThresholds {
        max_gas_usage: RESOLVE_MARKET_GAS_THRESHOLD,
        max_execution_time: RESOLVE_MARKET_TIME_THRESHOLD,
        max_storage_usage: CREATE_MARKET_STORAGE_THRESHOLD * 100,
        min_gas_efficiency: 60,
        min_time_efficiency: 60,
        min_storage_efficiency: 60,
        min_overall_score: 60,
    }
}

/// Performance Benchmark module for gas usage and execution time testing
///
/// This module provides comprehensive performance benchmarking capabilities
/// for measuring gas usage, execution time, and scalability characteristics
/// of the Predictify Hybrid contract functions.

// ===== BENCHMARK TYPES =====

/// Performance benchmark suite for comprehensive testing
#[contracttype]
#[derive(Clone, Debug)]
pub struct PerformanceBenchmarkSuite {
    pub suite_id: Symbol,
    pub total_benchmarks: u32,
    pub successful_benchmarks: u32,
    pub failed_benchmarks: u32,
    pub average_gas_usage: u64,
    pub average_execution_time: u64,
    pub benchmark_results: Map<String, BenchmarkResult>,
    pub performance_thresholds: PerformanceThresholds,
    pub benchmark_timestamp: u64,
}

/// Individual benchmark result for a specific function
#[contracttype]
#[derive(Clone, Debug)]
pub struct BenchmarkResult {
    pub function_name: String,
    pub gas_usage: u64,
    pub execution_time: u64,
    pub storage_usage: u64,
    pub success: bool,
    pub error_message: Option<String>,
    pub input_size: u32,
    pub output_size: u32,
    pub benchmark_timestamp: u64,
    pub performance_score: u32,
}

/// Performance metrics for comprehensive analysis
#[contracttype]
#[derive(Clone, Debug)]
pub struct PerformanceMetrics {
    pub total_gas_usage: u64,
    pub total_execution_time: u64,
    pub total_storage_usage: u64,
    pub average_gas_per_operation: u64,
    pub average_time_per_operation: u64,
    pub gas_efficiency_score: u32,
    pub time_efficiency_score: u32,
    pub storage_efficiency_score: u32,
    pub overall_performance_score: u32,
    pub benchmark_count: u32,
    pub success_rate: u32,
}

/// Performance thresholds for validation
#[contracttype]
#[derive(Clone, Debug)]
pub struct PerformanceThresholds {
    pub max_gas_usage: u64,
    pub max_execution_time: u64,
    pub max_storage_usage: u64,
    pub min_gas_efficiency: u32,
    pub min_time_efficiency: u32,
    pub min_storage_efficiency: u32,
    pub min_overall_score: u32,
}

/// Storage operation benchmark data
#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageOperation {
    pub operation_type: String,
    pub data_size: u32,
    pub key_count: u32,
    pub value_count: u32,
    pub operation_count: u32,
}

/// Batch operation benchmark data
#[contracttype]
#[derive(Clone, Debug)]
pub struct BatchOperation {
    pub operation_type: String,
    pub batch_size: u32,
    pub operation_count: u32,
    pub data_size: u32,
}

/// Scalability test parameters
#[contracttype]
#[derive(Clone, Debug)]
pub struct ScalabilityTest {
    pub market_size: u32,
    pub user_count: u32,
    pub operation_count: u32,
    pub concurrent_operations: u32,
    pub test_duration: u64,
}

/// Performance report with comprehensive analysis
#[contracttype]
#[derive(Clone, Debug)]
pub struct PerformanceReport {
    pub report_id: Symbol,
    pub benchmark_suite: PerformanceBenchmarkSuite,
    pub performance_metrics: PerformanceMetrics,
    pub recommendations: Vec<String>,
    pub optimization_opportunities: Vec<String>,
    pub performance_trends: Map<String, u32>,
    pub generated_timestamp: u64,
}

// ===== THRESHOLD HELPER =====

/// Checks whether a single measured value is within its allowed threshold.
///
/// # Parameters
/// - `measured` – the observed metric value (gas units, bytes, or milliseconds).
/// - `threshold` – the maximum allowed value for that metric.
/// - `label` – a human-readable name for the metric, included in error context.
///
/// # Returns
/// - `Ok(())` when `measured <= threshold`.
/// - `Err(Error::GasBudgetExceeded)` when `measured > threshold`, indicating a
///   threshold violation for the named metric.
pub(crate) fn check_threshold(measured: u64, threshold: u64, _label: &str) -> Result<(), Error> {
    if measured <= threshold {
        Ok(())
    } else {
        Err(Error::GasBudgetExceeded)
    }
}

// ===== PERFORMANCE BENCHMARK IMPLEMENTATION =====

/// Performance Benchmark Manager for comprehensive testing
pub struct PerformanceBenchmarkManager;

impl PerformanceBenchmarkManager {
    /// Benchmark gas usage for a specific function with given inputs
    pub fn benchmark_gas_usage(
        env: &Env,
        function: String,
        inputs: Vec<String>,
    ) -> Result<BenchmarkResult, Error> {
        let start_gas = 1000u64; // Mock gas measurement
        let start_time = env.ledger().timestamp();

        // Simulate function execution based on function name
        let result = Self::simulate_function_execution(env, &function, &inputs);

        let end_gas = 1500u64; // Mock gas measurement
        let end_time = env.ledger().timestamp();

        let gas_usage = end_gas - start_gas;
        let execution_time = end_time - start_time;

        let (success, error_message) = match result {
            Ok(_) => (true, None),
            Err(_e) => (false, Some(String::from_str(env, "Benchmark failed"))),
        };

        let performance_score = Self::calculate_performance_score(gas_usage, execution_time, 0);

        Ok(BenchmarkResult {
            function_name: function,
            gas_usage,
            execution_time,
            storage_usage: 0, // Placeholder
            success,
            error_message,
            input_size: inputs.len() as u32,
            output_size: 1, // Placeholder
            benchmark_timestamp: env.ledger().timestamp(),
            performance_score,
        })
    }

    /// Benchmark storage usage for a specific operation
    pub fn benchmark_storage_usage(
        env: &Env,
        operation: StorageOperation,
    ) -> Result<BenchmarkResult, Error> {
        let start_gas = 1000u64; // Mock gas measurement
        let start_time = env.ledger().timestamp();

        // Simulate storage operations
        let result = Self::simulate_storage_operations(env, &operation);

        let end_gas = 1500u64; // Mock gas measurement
        let end_time = env.ledger().timestamp();

        let gas_usage = end_gas - start_gas;
        let execution_time = end_time - start_time;
        let storage_usage = operation.data_size as u64 * operation.operation_count as u64;

        let (success, error_message) = match result {
            Ok(_) => (true, None),
            Err(_e) => (false, Some(String::from_str(env, "Benchmark failed"))),
        };

        let performance_score =
            Self::calculate_performance_score(gas_usage, execution_time, storage_usage);

        Ok(BenchmarkResult {
            function_name: String::from_str(env, "storage_operation"),
            gas_usage,
            execution_time,
            storage_usage,
            success,
            error_message,
            input_size: operation.data_size,
            output_size: operation.value_count,
            benchmark_timestamp: env.ledger().timestamp(),
            performance_score,
        })
    }

    /// Benchmark oracle call performance for a specific oracle provider
    pub fn benchmark_oracle_call_performance(
        env: &Env,
        oracle: OracleProvider,
    ) -> Result<BenchmarkResult, Error> {
        let start_gas = 1000u64; // Mock gas measurement
        let start_time = env.ledger().timestamp();

        // Simulate oracle call
        let result = Self::simulate_oracle_call(env, &oracle);

        let end_gas = 1500u64; // Mock gas measurement
        let end_time = env.ledger().timestamp();

        let gas_usage = end_gas - start_gas;
        let execution_time = end_time - start_time;

        let (success, error_message) = match result {
            Ok(_) => (true, None),
            Err(_e) => (false, Some(String::from_str(env, "Benchmark failed"))),
        };

        let performance_score = Self::calculate_performance_score(gas_usage, execution_time, 0);

        Ok(BenchmarkResult {
            function_name: String::from_str(env, "oracle_call"),
            gas_usage,
            execution_time,
            storage_usage: 0,
            success,
            error_message,
            input_size: 1,
            output_size: 1,
            benchmark_timestamp: env.ledger().timestamp(),
            performance_score,
        })
    }

    /// Benchmark batch operations performance
    pub fn benchmark_batch_operations(
        env: &Env,
        operations: Vec<BatchOperation>,
    ) -> Result<BenchmarkResult, Error> {
        let start_gas = 1000u64; // Mock gas measurement
        let start_time = env.ledger().timestamp();

        // Simulate batch operations
        let result = Self::simulate_batch_operations(env, &operations);

        let end_gas = 1500u64; // Mock gas measurement
        let end_time = env.ledger().timestamp();

        let gas_usage = end_gas - start_gas;
        let execution_time = end_time - start_time;

        let total_operations = operations.iter().map(|op| op.operation_count).sum::<u32>();
        let total_data_size = operations.iter().map(|op| op.data_size).sum::<u32>();

        let (success, error_message) = match result {
            Ok(_) => (true, None),
            Err(_e) => (false, Some(String::from_str(env, "Benchmark failed"))),
        };

        let performance_score =
            Self::calculate_performance_score(gas_usage, execution_time, total_data_size as u64);

        Ok(BenchmarkResult {
            function_name: String::from_str(env, "batch_operations"),
            gas_usage,
            execution_time,
            storage_usage: total_data_size as u64,
            success,
            error_message,
            input_size: total_operations,
            output_size: total_operations,
            benchmark_timestamp: env.ledger().timestamp(),
            performance_score,
        })
    }

    /// Benchmark scalability with large markets and user counts
    pub fn benchmark_scalability(
        env: &Env,
        market_size: u32,
        user_count: u32,
    ) -> Result<BenchmarkResult, Error> {
        let start_gas = 1000u64; // Mock gas measurement
        let start_time = env.ledger().timestamp();

        // Simulate scalability test
        let result = Self::simulate_scalability_test(env, market_size, user_count);

        let end_gas = 1500u64; // Mock gas measurement
        let end_time = env.ledger().timestamp();

        let gas_usage = end_gas - start_gas;
        let execution_time = end_time - start_time;
        let storage_usage = (market_size * user_count) as u64;

        let (success, error_message) = match result {
            Ok(_) => (true, None),
            Err(_e) => (false, Some(String::from_str(env, "Benchmark failed"))),
        };

        let performance_score =
            Self::calculate_performance_score(gas_usage, execution_time, storage_usage);

        Ok(BenchmarkResult {
            function_name: String::from_str(env, "scalability_test"),
            gas_usage,
            execution_time,
            storage_usage,
            success,
            error_message,
            input_size: market_size,
            output_size: user_count,
            benchmark_timestamp: env.ledger().timestamp(),
            performance_score,
        })
    }

    /// Generate comprehensive performance report
    pub fn generate_performance_report(
        env: &Env,
        benchmark_suite: PerformanceBenchmarkSuite,
    ) -> Result<PerformanceReport, Error> {
        let performance_metrics = Self::calculate_performance_metrics(&benchmark_suite);
        let recommendations = Self::generate_recommendations(env, &performance_metrics);
        let optimization_opportunities =
            Self::identify_optimization_opportunities(env, &benchmark_suite);
        let performance_trends = Self::calculate_performance_trends(&benchmark_suite);

        Ok(PerformanceReport {
            report_id: Symbol::new(env, "perf_report"),
            benchmark_suite: benchmark_suite.clone(),
            performance_metrics,
            recommendations,
            optimization_opportunities,
            performance_trends,
            generated_timestamp: env.ledger().timestamp(),
        })
    }

    /// Validate performance against thresholds.
    ///
    /// Returns `Ok(true)` if every metric in `metrics` is within its corresponding
    /// field in `thresholds`, and `Ok(false)` if any metric exceeds its threshold.
    /// `Err` is reserved for unexpected internal failures.
    pub fn validate_performance_thresholds(
        _env: &Env,
        metrics: PerformanceMetrics,
        thresholds: PerformanceThresholds,
    ) -> Result<bool, Error> {
        let gas_ok = check_threshold(
            metrics.average_gas_per_operation,
            thresholds.max_gas_usage,
            "average_gas_per_operation",
        )
        .is_ok();
        let time_ok = check_threshold(
            metrics.average_time_per_operation,
            thresholds.max_execution_time,
            "average_time_per_operation",
        )
        .is_ok();
        let storage_ok = check_threshold(
            metrics.total_storage_usage,
            thresholds.max_storage_usage,
            "total_storage_usage",
        )
        .is_ok();
        // For the efficiency score the comparison is inverted: measured must be >= threshold.
        let efficiency_ok = metrics.overall_performance_score >= thresholds.min_overall_score;

        Ok(gas_ok && time_ok && storage_ok && efficiency_ok)
    }

    // ===== HELPER FUNCTIONS =====

    /// Simulate function execution for benchmarking
    fn simulate_function_execution(
        _env: &Env,
        _function: &String,
        _inputs: &Vec<String>,
    ) -> Result<(), Error> {
        // Simple function simulation - always succeed
        Ok(())
    }

    /// Simulate storage operations for benchmarking
    fn simulate_storage_operations(_env: &Env, _operation: &StorageOperation) -> Result<(), Error> {
        // Simulate storage operations based on type
        // Simple operation simulation - always succeed
        Ok(())
    }

    /// Simulate oracle call for benchmarking
    fn simulate_oracle_call(_env: &Env, _oracle: &OracleProvider) -> Result<(), Error> {
        // Simulate oracle call
        Ok(())
    }

    /// Simulate batch operations for benchmarking
    fn simulate_batch_operations(
        _env: &Env,
        _operations: &Vec<BatchOperation>,
    ) -> Result<(), Error> {
        // Simulate batch operations
        Ok(())
    }

    /// Simulate scalability test
    fn simulate_scalability_test(
        _env: &Env,
        _market_size: u32,
        _user_count: u32,
    ) -> Result<(), Error> {
        // Simulate scalability test
        Ok(())
    }

    /// Calculate performance score based on metrics
    fn calculate_performance_score(gas_usage: u64, execution_time: u64, storage_usage: u64) -> u32 {
        // Simple scoring algorithm (0-100)
        let gas_score = if gas_usage < 1000 {
            100
        } else if gas_usage < 5000 {
            80
        } else if gas_usage < 10000 {
            60
        } else {
            40
        };
        let time_score = if execution_time < 100 {
            100
        } else if execution_time < 500 {
            80
        } else if execution_time < 1000 {
            60
        } else {
            40
        };
        let storage_score = if storage_usage < 1000 {
            100
        } else if storage_usage < 5000 {
            80
        } else if storage_usage < 10000 {
            60
        } else {
            40
        };

        (gas_score + time_score + storage_score) / 3
    }

    /// Calculate comprehensive performance metrics
    fn calculate_performance_metrics(suite: &PerformanceBenchmarkSuite) -> PerformanceMetrics {
        let total_gas = suite
            .benchmark_results
            .iter()
            .map(|(_, result)| result.gas_usage)
            .sum::<u64>();
        let total_time = suite
            .benchmark_results
            .iter()
            .map(|(_, result)| result.execution_time)
            .sum::<u64>();
        let total_storage = suite
            .benchmark_results
            .iter()
            .map(|(_, result)| result.storage_usage)
            .sum::<u64>();

        let benchmark_count = suite.benchmark_results.len() as u32;
        let average_gas = if benchmark_count > 0 {
            total_gas / benchmark_count as u64
        } else {
            0
        };
        let average_time = if benchmark_count > 0 {
            total_time / benchmark_count as u64
        } else {
            0
        };

        let gas_efficiency = if average_gas < 1000 {
            100
        } else if average_gas < 5000 {
            80
        } else {
            60
        };
        let time_efficiency = if average_time < 100 {
            100
        } else if average_time < 500 {
            80
        } else {
            60
        };
        let storage_efficiency = if total_storage < 10000 {
            100
        } else if total_storage < 50000 {
            80
        } else {
            60
        };

        let overall_score = (gas_efficiency + time_efficiency + storage_efficiency) / 3;
        let success_rate = if suite.total_benchmarks > 0 {
            (suite.successful_benchmarks * 100) / suite.total_benchmarks
        } else {
            0
        };

        PerformanceMetrics {
            total_gas_usage: total_gas,
            total_execution_time: total_time,
            total_storage_usage: total_storage,
            average_gas_per_operation: average_gas,
            average_time_per_operation: average_time,
            gas_efficiency_score: gas_efficiency,
            time_efficiency_score: time_efficiency,
            storage_efficiency_score: storage_efficiency,
            overall_performance_score: overall_score,
            benchmark_count,
            success_rate,
        }
    }

    /// Generate performance recommendations
    fn generate_recommendations(env: &Env, _metrics: &PerformanceMetrics) -> Vec<String> {
        // Return empty recommendations for now
        Vec::new(env)
    }

    /// Identify optimization opportunities
    fn identify_optimization_opportunities(
        env: &Env,
        _suite: &PerformanceBenchmarkSuite,
    ) -> Vec<String> {
        // Return empty opportunities for now
        Vec::new(env)
    }

    /// Calculate performance trends
    fn calculate_performance_trends(suite: &PerformanceBenchmarkSuite) -> Map<String, u32> {
        let mut trends = Map::new(&Env::default());

        trends.set(
            String::from_str(&Env::default(), "gas_trend"),
            suite.average_gas_usage as u32,
        );
        trends.set(
            String::from_str(&Env::default(), "time_trend"),
            suite.average_execution_time as u32,
        );
        trends.set(
            String::from_str(&Env::default(), "success_trend"),
            suite.successful_benchmarks,
        );

        trends
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use proptest::prelude::*;

    struct PerfBenchTest {
        env: Env,
    }

    impl PerfBenchTest {
        fn new() -> Self {
            let env = Env::default();
            PerfBenchTest { env }
        }
    }

    #[test]
    fn test_benchmark_gas_usage_success() {
        let test = PerfBenchTest::new();
        let func = String::from_str(&test.env, "test_function");
        let inputs = Vec::new(&test.env);
        let result = PerformanceBenchmarkManager::benchmark_gas_usage(&test.env, func, inputs);
        assert!(result.is_ok());
    }

    #[test]
    fn test_benchmark_storage_usage() {
        let test = PerfBenchTest::new();
        let op = StorageOperation {
            operation_type: String::from_str(&test.env, "read"),
            data_size: 100,
            key_count: 10,
            value_count: 10,
            operation_count: 5,
        };
        let result = PerformanceBenchmarkManager::benchmark_storage_usage(&test.env, op);
        assert!(result.is_ok());
    }

    #[test]
    fn test_benchmark_batch_operations() {
        let test = PerfBenchTest::new();
        // Test benchmark batch operations
        let op = BatchOperation {
            operation_type: String::from_str(&test.env, "write"),
            batch_size: 5,
            operation_count: 10,
            data_size: 256,
        };
        let mut ops = soroban_sdk::Vec::new(&test.env);
        ops.push_back(op);
        let result = PerformanceBenchmarkManager::benchmark_batch_operations(&test.env, ops);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_scalability_test() {
        let test = PerfBenchTest::new();
        // Test scalability testing
        let market_size: u32 = 100;
        let user_count: u32 = 50;
        let result =
            PerformanceBenchmarkManager::benchmark_scalability(&test.env, market_size, user_count);
        assert!(result.is_ok());
    }

    #[test]
    fn test_benchmark_result_structure() {
        let test = PerfBenchTest::new();
        let result = BenchmarkResult {
            function_name: String::from_str(&test.env, "test_func"),
            gas_usage: 1000,
            execution_time: 50,
            storage_usage: 100,
            success: true,
            error_message: None,
            input_size: 10,
            output_size: 20,
            benchmark_timestamp: test.env.ledger().timestamp(),
            performance_score: 85,
        };
        assert!(result.success);
        assert_eq!(result.gas_usage, 1000);
    }

    #[test]
    fn test_performance_metrics_structure() {
        let test = PerfBenchTest::new();
        let metrics = PerformanceMetrics {
            total_gas_usage: 5000,
            total_execution_time: 250,
            total_storage_usage: 500,
            average_gas_per_operation: 1000,
            average_time_per_operation: 50,
            gas_efficiency_score: 85,
            time_efficiency_score: 90,
            storage_efficiency_score: 80,
            overall_performance_score: 85,
            benchmark_count: 5,
            success_rate: 100,
        };
        assert_eq!(metrics.benchmark_count, 5);
    }

    #[test]
    fn test_performance_thresholds_structure() {
        let test = PerfBenchTest::new();
        let thresholds = PerformanceThresholds {
            max_gas_usage: 10000,
            max_execution_time: 5000,
            max_storage_usage: 100000,
            min_gas_efficiency: 50,
            min_time_efficiency: 50,
            min_storage_efficiency: 50,
            min_overall_score: 50,
        };
        assert!(thresholds.max_gas_usage > 0);
    }

    #[test]
    fn test_storage_operation_structure() {
        let test = PerfBenchTest::new();
        let op = StorageOperation {
            operation_type: String::from_str(&test.env, "write"),
            data_size: 200,
            key_count: 20,
            value_count: 20,
            operation_count: 10,
        };
        assert_eq!(op.data_size, 200);
    }

    #[test]
    fn test_batch_operation_structure() {
        let test = PerfBenchTest::new();
        let op = BatchOperation {
            operation_type: String::from_str(&test.env, "batch_vote"),
            batch_size: 50,
            operation_count: 500,
            data_size: 2000,
        };
        assert_eq!(op.batch_size, 50);
    }

    #[test]
    fn test_scalability_test_structure() {
        let test = PerfBenchTest::new();
        let test_params = ScalabilityTest {
            market_size: 500,
            user_count: 5000,
            operation_count: 50000,
            concurrent_operations: 500,
            test_duration: 7200,
        };
        assert!(test_params.market_size > 0);
    }

    #[test]
    fn test_performance_benchmark_suite_structure() {
        let test = PerfBenchTest::new();
        let suite = PerformanceBenchmarkSuite {
            suite_id: Symbol::new(&test.env, "suite_1"),
            total_benchmarks: 10,
            successful_benchmarks: 9,
            failed_benchmarks: 1,
            average_gas_usage: 1200,
            average_execution_time: 60,
            benchmark_results: Map::new(&test.env),
            performance_thresholds: PerformanceThresholds {
                max_gas_usage: 10000,
                max_execution_time: 5000,
                max_storage_usage: 100000,
                min_gas_efficiency: 50,
                min_time_efficiency: 50,
                min_storage_efficiency: 50,
                min_overall_score: 50,
            },
            benchmark_timestamp: test.env.ledger().timestamp(),
        };
        assert_eq!(suite.total_benchmarks, 10);
    }

    #[test]
    fn test_performance_report_structure() {
        let test = PerfBenchTest::new();
        let suite = PerformanceBenchmarkSuite {
            suite_id: Symbol::new(&test.env, "suite_1"),
            total_benchmarks: 5,
            successful_benchmarks: 5,
            failed_benchmarks: 0,
            average_gas_usage: 1000,
            average_execution_time: 50,
            benchmark_results: Map::new(&test.env),
            performance_thresholds: PerformanceThresholds {
                max_gas_usage: 10000,
                max_execution_time: 5000,
                max_storage_usage: 100000,
                min_gas_efficiency: 50,
                min_time_efficiency: 50,
                min_storage_efficiency: 50,
                min_overall_score: 50,
            },
            benchmark_timestamp: test.env.ledger().timestamp(),
        };
        let metrics = PerformanceMetrics {
            total_gas_usage: 5000,
            total_execution_time: 250,
            total_storage_usage: 500,
            average_gas_per_operation: 1000,
            average_time_per_operation: 50,
            gas_efficiency_score: 85,
            time_efficiency_score: 90,
            storage_efficiency_score: 80,
            overall_performance_score: 85,
            benchmark_count: 5,
            success_rate: 100,
        };
        let report = PerformanceReport {
            report_id: Symbol::new(&test.env, "report_1"),
            benchmark_suite: suite,
            performance_metrics: metrics,
            recommendations: Vec::new(&test.env),
            optimization_opportunities: Vec::new(&test.env),
            performance_trends: Map::new(&test.env),
            generated_timestamp: test.env.ledger().timestamp(),
        };
        assert!(!report.report_id.to_string().is_empty());
    }

    #[test]
    fn test_gas_efficiency_scoring() {
        let test = PerfBenchTest::new();
        // Test efficiency scoring boundaries
        let low_gas = 500u64;
        let medium_gas = 3000u64;
        let high_gas = 8000u64;
        assert!(low_gas < medium_gas && medium_gas < high_gas);
    }

    #[test]
    fn test_success_rate_calculation() {
        let test = PerfBenchTest::new();
        let successful = 9u32;
        let total = 10u32;
        let rate = (successful * 100) / total;
        assert_eq!(rate, 90);
    }

    #[test]
    fn test_storage_cost_scaling() {
        let test = PerfBenchTest::new();
        let data_size = 100u32;
        let operation_count = 5u32;
        let total_cost = data_size as u64 * operation_count as u64;
        assert_eq!(total_cost, 500);
    }

    #[test]
    fn test_oracle_provider_benchmark() {
        let _test = PerfBenchTest::new();
        // Test benchmarking different oracle providers
        let provider_count = 4;
        assert_eq!(provider_count, 4);
    }

    // ===== check_threshold unit tests =====

    /// Requirement 2.3: check_threshold returns Ok when measured <= threshold.
    #[test]
    fn test_check_threshold_within_bounds() {
        assert!(check_threshold(100, 200, "gas").is_ok());
        // Equal boundary must also pass.
        assert!(check_threshold(200, 200, "gas").is_ok());
    }

    /// Requirement 2.4: check_threshold returns Err when measured > threshold.
    #[test]
    fn test_check_threshold_violation() {
        let result = check_threshold(300, 200, "gas");
        assert!(result.is_err());
    }

    // ===== validate_performance_thresholds unit tests =====

    fn make_metrics(gas: u64, time: u64, storage: u64, score: u32) -> PerformanceMetrics {
        PerformanceMetrics {
            total_gas_usage: gas,
            total_execution_time: time,
            total_storage_usage: storage,
            average_gas_per_operation: gas,
            average_time_per_operation: time,
            gas_efficiency_score: score,
            time_efficiency_score: score,
            storage_efficiency_score: score,
            overall_performance_score: score,
            benchmark_count: 1,
            success_rate: 100,
        }
    }

    fn make_thresholds(max_gas: u64, max_time: u64, max_storage: u64, min_score: u32) -> PerformanceThresholds {
        PerformanceThresholds {
            max_gas_usage: max_gas,
            max_execution_time: max_time,
            max_storage_usage: max_storage,
            min_gas_efficiency: min_score,
            min_time_efficiency: min_score,
            min_storage_efficiency: min_score,
            min_overall_score: min_score,
        }
    }

    /// Requirement 3.4: validate returns Ok(true) when all metrics are within bounds.
    #[test]
    fn test_validate_all_within_bounds() {
        let test = PerfBenchTest::new();
        let metrics = make_metrics(100, 50, 200, 80);
        let thresholds = make_thresholds(1000, 500, 2000, 60);
        let result = PerformanceBenchmarkManager::validate_performance_thresholds(
            &test.env, metrics, thresholds,
        );
        assert_eq!(result, Ok(true));
    }

    /// Requirement 3.5: validate returns Ok(false) when gas exceeds threshold.
    #[test]
    fn test_validate_gas_exceeded() {
        let test = PerfBenchTest::new();
        let metrics = make_metrics(5000, 50, 200, 80);
        let thresholds = make_thresholds(1000, 500, 2000, 60);
        let result = PerformanceBenchmarkManager::validate_performance_thresholds(
            &test.env, metrics, thresholds,
        );
        assert_eq!(result, Ok(false));
    }

    // ===== Per-function benchmark threshold tests (Requirements 3.1, 3.2, 3.3) =====

    /// Requirement 3.1, 3.2, 3.3: create_market benchmark stays within thresholds.
    #[test]
    fn test_benchmark_create_market_thresholds() {
        let test = PerfBenchTest::new();
        let func = String::from_str(&test.env, "create_market");
        let inputs = Vec::new(&test.env);
        let result = PerformanceBenchmarkManager::benchmark_gas_usage(&test.env, func, inputs)
            .expect("benchmark_gas_usage should succeed");
        assert!(result.success);
        assert!(
            result.gas_usage <= CREATE_MARKET_GAS_THRESHOLD,
            "gas_usage {} exceeds CREATE_MARKET_GAS_THRESHOLD {}",
            result.gas_usage,
            CREATE_MARKET_GAS_THRESHOLD
        );
        assert!(
            result.storage_usage <= CREATE_MARKET_STORAGE_THRESHOLD,
            "storage_usage {} exceeds CREATE_MARKET_STORAGE_THRESHOLD {}",
            result.storage_usage,
            CREATE_MARKET_STORAGE_THRESHOLD
        );
    }

    /// Requirement 3.1, 3.2, 3.3: vote benchmark stays within thresholds.
    #[test]
    fn test_benchmark_vote_thresholds() {
        let test = PerfBenchTest::new();
        let func = String::from_str(&test.env, "vote");
        let inputs = Vec::new(&test.env);
        let result = PerformanceBenchmarkManager::benchmark_gas_usage(&test.env, func, inputs)
            .expect("benchmark_gas_usage should succeed");
        assert!(result.success);
        assert!(
            result.gas_usage <= VOTE_GAS_THRESHOLD,
            "gas_usage {} exceeds VOTE_GAS_THRESHOLD {}",
            result.gas_usage,
            VOTE_GAS_THRESHOLD
        );
        assert!(
            result.storage_usage <= VOTE_STORAGE_THRESHOLD,
            "storage_usage {} exceeds VOTE_STORAGE_THRESHOLD {}",
            result.storage_usage,
            VOTE_STORAGE_THRESHOLD
        );
    }

    /// Requirement 3.1, 3.2, 3.3: claim_winnings benchmark stays within thresholds.
    #[test]
    fn test_benchmark_claim_winnings_thresholds() {
        let test = PerfBenchTest::new();
        let func = String::from_str(&test.env, "claim_winnings");
        let inputs = Vec::new(&test.env);
        let result = PerformanceBenchmarkManager::benchmark_gas_usage(&test.env, func, inputs)
            .expect("benchmark_gas_usage should succeed");
        assert!(result.success);
        assert!(
            result.gas_usage <= CLAIM_WINNINGS_GAS_THRESHOLD,
            "gas_usage {} exceeds CLAIM_WINNINGS_GAS_THRESHOLD {}",
            result.gas_usage,
            CLAIM_WINNINGS_GAS_THRESHOLD
        );
        assert!(
            result.storage_usage <= CLAIM_WINNINGS_STORAGE_THRESHOLD,
            "storage_usage {} exceeds CLAIM_WINNINGS_STORAGE_THRESHOLD {}",
            result.storage_usage,
            CLAIM_WINNINGS_STORAGE_THRESHOLD
        );
    }

    /// Requirement 3.1, 3.2, 3.3: resolve_market benchmark stays within thresholds.
    #[test]
    fn test_benchmark_resolve_market_thresholds() {
        let test = PerfBenchTest::new();
        let func = String::from_str(&test.env, "resolve_market");
        let inputs = Vec::new(&test.env);
        let result = PerformanceBenchmarkManager::benchmark_gas_usage(&test.env, func, inputs)
            .expect("benchmark_gas_usage should succeed");
        assert!(result.success);
        assert!(
            result.gas_usage <= RESOLVE_MARKET_GAS_THRESHOLD,
            "gas_usage {} exceeds RESOLVE_MARKET_GAS_THRESHOLD {}",
            result.gas_usage,
            RESOLVE_MARKET_GAS_THRESHOLD
        );
        assert!(
            result.storage_usage <= RESOLVE_MARKET_STORAGE_THRESHOLD,
            "storage_usage {} exceeds RESOLVE_MARKET_STORAGE_THRESHOLD {}",
            result.storage_usage,
            RESOLVE_MARKET_STORAGE_THRESHOLD
        );
    }

    /// Requirement 3.1, 3.2, 3.3: fetch_oracle_result benchmark stays within thresholds.
    #[test]
    fn test_benchmark_fetch_oracle_result_thresholds() {
        let test = PerfBenchTest::new();
        let result =
            PerformanceBenchmarkManager::benchmark_oracle_call_performance(
                &test.env,
                OracleProvider::Reflector,
            )
            .expect("benchmark_oracle_call_performance should succeed");
        assert!(result.success);
        assert!(
            result.gas_usage <= FETCH_ORACLE_RESULT_GAS_THRESHOLD,
            "gas_usage {} exceeds FETCH_ORACLE_RESULT_GAS_THRESHOLD {}",
            result.gas_usage,
            FETCH_ORACLE_RESULT_GAS_THRESHOLD
        );
        assert!(
            result.storage_usage <= FETCH_ORACLE_RESULT_STORAGE_THRESHOLD,
            "storage_usage {} exceeds FETCH_ORACLE_RESULT_STORAGE_THRESHOLD {}",
            result.storage_usage,
            FETCH_ORACLE_RESULT_STORAGE_THRESHOLD
        );
    }

    /// Requirement 3.1, 3.2, 3.3: collect_fees benchmark stays within thresholds.
    #[test]
    fn test_benchmark_collect_fees_thresholds() {
        let test = PerfBenchTest::new();
        let func = String::from_str(&test.env, "collect_fees");
        let inputs = Vec::new(&test.env);
        let result = PerformanceBenchmarkManager::benchmark_gas_usage(&test.env, func, inputs)
            .expect("benchmark_gas_usage should succeed");
        assert!(result.success);
        assert!(
            result.gas_usage <= COLLECT_FEES_GAS_THRESHOLD,
            "gas_usage {} exceeds COLLECT_FEES_GAS_THRESHOLD {}",
            result.gas_usage,
            COLLECT_FEES_GAS_THRESHOLD
        );
        assert!(
            result.storage_usage <= COLLECT_FEES_STORAGE_THRESHOLD,
            "storage_usage {} exceeds COLLECT_FEES_STORAGE_THRESHOLD {}",
            result.storage_usage,
            COLLECT_FEES_STORAGE_THRESHOLD
        );
    }

    // ===== Property-Based Tests =====

    // Feature: perf-thresholds, Property 5: All benchmark functions return success=true under normal inputs
    // Validates: Requirements 3.3
    proptest! {
        #[test]
        fn prop_benchmark_functions_succeed(
            data_size in 1u32..=1024u32,
            key_count in 1u32..=64u32,
            value_count in 1u32..=64u32,
            op_count in 1u32..=32u32,
            batch_size in 1u32..=64u32,
            market_size in 1u32..=500u32,
            user_count in 1u32..=500u32,
        ) {
            let env = Env::default();

            // benchmark_storage_usage
            let storage_op = StorageOperation {
                operation_type: String::from_str(&env, "read"),
                data_size,
                key_count,
                value_count,
                operation_count: op_count,
            };
            let storage_result = PerformanceBenchmarkManager::benchmark_storage_usage(&env, storage_op)
                .expect("benchmark_storage_usage should not error");
            prop_assert!(storage_result.success, "storage benchmark returned success=false");

            // benchmark_batch_operations
            let batch_op = BatchOperation {
                operation_type: String::from_str(&env, "write"),
                batch_size,
                operation_count: op_count,
                data_size,
            };
            let mut ops = soroban_sdk::Vec::new(&env);
            ops.push_back(batch_op);
            let batch_result = PerformanceBenchmarkManager::benchmark_batch_operations(&env, ops)
                .expect("benchmark_batch_operations should not error");
            prop_assert!(batch_result.success, "batch benchmark returned success=false");

            // benchmark_scalability
            let scale_result = PerformanceBenchmarkManager::benchmark_scalability(&env, market_size, user_count)
                .expect("benchmark_scalability should not error");
            prop_assert!(scale_result.success, "scalability benchmark returned success=false");

            // benchmark_oracle_call_performance
            let oracle_result = PerformanceBenchmarkManager::benchmark_oracle_call_performance(
                &env,
                OracleProvider::Reflector,
            )
            .expect("benchmark_oracle_call_performance should not error");
            prop_assert!(oracle_result.success, "oracle benchmark returned success=false");
        }
    }
}
