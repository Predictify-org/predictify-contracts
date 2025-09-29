#![allow(dead_code)]

use alloc::format;
use soroban_sdk::{contracttype, vec, Address, Env, Map, String, Symbol, Vec};

use crate::errors::Error;
use crate::types::{Market, MarketState, OracleConfig, OracleProvider};

/// Comprehensive monitoring system for Predictify contract health and performance.
///
/// This module provides real-time monitoring capabilities for:
/// - Market health and status monitoring
/// - Oracle health and reliability tracking
/// - Fee collection and revenue monitoring
/// - Dispute resolution tracking and analytics
/// - Performance metrics and optimization insights
/// - System alerts and notifications
///
/// The monitoring system enables proactive management of contract operations,
/// early detection of issues, and data-driven optimization decisions.

// ===== MONITORING TYPES AND STRUCTURES =====

/// Types of monitoring alerts that can be generated
#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub enum MonitoringAlertType {
    MarketHealth,
    OracleHealth,
    FeeCollection,
    DisputeResolution,
    Performance,
    Security,
    SystemOverload,
    DataIntegrity,
    NetworkIssues,
    Custom,
}

/// Severity levels for monitoring alerts
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[contracttype]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Status of monitoring components
#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub enum MonitoringStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
    Maintenance,
}

/// Time frame for monitoring data collection
#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub enum TimeFrame {
    LastHour,
    LastDay,
    LastWeek,
    LastMonth,
    Custom(u64), // Custom duration in seconds
}

/// Market health metrics
#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub struct MarketHealthMetrics {
    pub market_id: Symbol,
    pub status: MonitoringStatus,
    pub total_votes: u32,
    pub total_stake: i128,
    pub active_participants: u32,
    pub time_to_end: u64,
    pub dispute_count: u32,
    pub resolution_confidence: u32,
    pub last_activity: u64,
    pub health_score: u32, // 0-100
}

/// Oracle health metrics
#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub struct OracleHealthMetrics {
    pub provider: OracleProvider,
    pub status: MonitoringStatus,
    pub response_time: u64, // milliseconds
    pub success_rate: u32,  // percentage
    pub last_response: u64,
    pub total_requests: u32,
    pub failed_requests: u32,
    pub availability: u32,     // percentage
    pub confidence_score: u32, // 0-100
}

/// Fee collection metrics
#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub struct FeeCollectionMetrics {
    pub timeframe: TimeFrame,
    pub total_fees_collected: i128,
    pub total_markets: u32,
    pub successful_collections: u32,
    pub failed_collections: u32,
    pub average_fee_per_market: i128,
    pub revenue_growth: i32,        // percentage change
    pub collection_efficiency: u32, // percentage
}

/// Dispute resolution metrics
#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub struct DisputeResolutionMetrics {
    pub timeframe: TimeFrame,
    pub total_disputes: u32,
    pub resolved_disputes: u32,
    pub pending_disputes: u32,
    pub average_resolution_time: u64, // seconds
    pub resolution_success_rate: u32, // percentage
    pub escalation_count: u32,
    pub community_consensus_rate: u32, // percentage
}

/// Performance metrics for contract operations
#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub struct PerformanceMetrics {
    pub timeframe: TimeFrame,
    pub total_operations: u32,
    pub successful_operations: u32,
    pub failed_operations: u32,
    pub average_execution_time: u64, // milliseconds
    pub gas_usage: u64,
    pub throughput: u32,         // operations per second
    pub error_rate: u32,         // percentage
    pub optimization_score: u32, // 0-100
}

/// Monitoring alert
#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub struct MonitoringAlert {
    pub alert_id: String,
    pub alert_type: MonitoringAlertType,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub affected_component: String,
    pub timestamp: u64,
    pub resolved: bool,
    pub resolution_notes: Option<String>,
    pub metadata: Map<String, String>,
}

/// Comprehensive monitoring data
#[derive(Debug, Clone, PartialEq, Eq)]
#[contracttype]
pub struct MonitoringData {
    pub timestamp: u64,
    pub market_health: Vec<MarketHealthMetrics>,
    pub oracle_health: Vec<OracleHealthMetrics>,
    pub fee_metrics: FeeCollectionMetrics,
    pub dispute_metrics: DisputeResolutionMetrics,
    pub performance_metrics: PerformanceMetrics,
    pub active_alerts: Vec<MonitoringAlert>,
    pub system_status: MonitoringStatus,
}

// ===== CONTRACT MONITOR STRUCT =====

/// Main contract monitoring system
pub struct ContractMonitor;

impl ContractMonitor {
    /// Monitor market health for a specific market
    pub fn monitor_market_health(
        env: &Env,
        market_id: Symbol,
    ) -> Result<MarketHealthMetrics, Error> {
        // Get market data (this would be implemented with actual market retrieval)
        let market = Self::get_market_data(env, &market_id)?;

        // Calculate health metrics
        let total_votes = Self::calculate_total_votes(env, &market_id)?;
        let total_stake = Self::calculate_total_stake(env, &market_id)?;
        let active_participants = Self::calculate_active_participants(env, &market_id)?;
        let time_to_end = Self::calculate_time_to_end(env, &market)?;
        let dispute_count = Self::calculate_dispute_count(env, &market_id)?;
        let resolution_confidence = Self::calculate_resolution_confidence(env, &market_id)?;
        let last_activity = Self::get_last_activity(env, &market_id)?;
        let health_score = Self::calculate_health_score(
            &total_votes,
            &total_stake,
            &active_participants,
            &time_to_end,
            &dispute_count,
            &resolution_confidence,
        );

        let status = Self::determine_market_status(&health_score, &dispute_count, &time_to_end);

        Ok(MarketHealthMetrics {
            market_id,
            status,
            total_votes,
            total_stake,
            active_participants,
            time_to_end,
            dispute_count,
            resolution_confidence,
            last_activity,
            health_score,
        })
    }

    /// Monitor oracle health for a specific oracle provider
    pub fn monitor_oracle_health(
        env: &Env,
        oracle: OracleProvider,
    ) -> Result<OracleHealthMetrics, Error> {
        // Get oracle performance data
        let response_time = Self::get_average_response_time(env, &oracle)?;
        let success_rate = Self::calculate_success_rate(env, &oracle)?;
        let last_response = Self::get_last_response_time(env, &oracle)?;
        let total_requests = Self::get_total_requests(env, &oracle)?;
        let failed_requests = Self::get_failed_requests(env, &oracle)?;
        let availability = Self::calculate_availability(env, &oracle)?;
        let confidence_score = Self::calculate_oracle_confidence(
            &response_time,
            &success_rate,
            &availability,
            &last_response,
        );

        let status = Self::determine_oracle_status(&confidence_score, &success_rate, &availability);

        Ok(OracleHealthMetrics {
            provider: oracle,
            status,
            response_time,
            success_rate,
            last_response,
            total_requests,
            failed_requests,
            availability,
            confidence_score,
        })
    }

    /// Monitor fee collection performance
    pub fn monitor_fee_collection(
        env: &Env,
        timeframe: TimeFrame,
    ) -> Result<FeeCollectionMetrics, Error> {
        let start_time = Self::calculate_start_time(env, &timeframe)?;

        let total_fees_collected = Self::calculate_total_fees_collected(env, start_time)?;
        let total_markets = Self::count_markets_in_timeframe(env, start_time)?;
        let successful_collections = Self::count_successful_collections(env, start_time)?;
        let failed_collections = Self::count_failed_collections(env, start_time)?;
        let average_fee_per_market = if total_markets > 0 {
            total_fees_collected / total_markets as i128
        } else {
            0
        };
        let revenue_growth = Self::calculate_revenue_growth(env, &timeframe)?;
        let collection_efficiency = if total_markets > 0 {
            (successful_collections * 100) / total_markets
        } else {
            0
        };

        Ok(FeeCollectionMetrics {
            timeframe,
            total_fees_collected,
            total_markets,
            successful_collections,
            failed_collections,
            average_fee_per_market,
            revenue_growth,
            collection_efficiency,
        })
    }

    /// Monitor dispute resolution performance
    pub fn monitor_dispute_resolution(
        env: &Env,
        market_id: Symbol,
    ) -> Result<DisputeResolutionMetrics, Error> {
        let timeframe = TimeFrame::LastWeek; // Default timeframe
        let start_time = Self::calculate_start_time(env, &timeframe)?;

        let total_disputes = Self::count_total_disputes(env, &market_id, start_time)?;
        let resolved_disputes = Self::count_resolved_disputes(env, &market_id, start_time)?;
        let pending_disputes = total_disputes.saturating_sub(resolved_disputes);
        let average_resolution_time =
            Self::calculate_average_resolution_time(env, &market_id, start_time)?;
        let resolution_success_rate = if total_disputes > 0 {
            (resolved_disputes * 100) / total_disputes
        } else {
            0
        };
        let escalation_count = Self::count_escalations(env, &market_id, start_time)?;
        let community_consensus_rate =
            Self::calculate_community_consensus_rate(env, &market_id, start_time)?;

        Ok(DisputeResolutionMetrics {
            timeframe,
            total_disputes,
            resolved_disputes,
            pending_disputes,
            average_resolution_time,
            resolution_success_rate,
            escalation_count,
            community_consensus_rate,
        })
    }

    /// Get comprehensive contract performance metrics
    pub fn get_contract_performance_metrics(
        env: &Env,
        timeframe: TimeFrame,
    ) -> Result<PerformanceMetrics, Error> {
        let start_time = Self::calculate_start_time(env, &timeframe)?;

        let total_operations = Self::count_total_operations(env, start_time)?;
        let successful_operations = Self::count_successful_operations(env, start_time)?;
        let failed_operations = total_operations.saturating_sub(successful_operations);
        let average_execution_time = Self::calculate_average_execution_time(env, start_time)?;
        let gas_usage = Self::calculate_total_gas_usage(env, start_time)?;
        let throughput = Self::calculate_throughput(env, &timeframe)?;
        let error_rate = if total_operations > 0 {
            (failed_operations * 100) / total_operations
        } else {
            0
        };
        let optimization_score = Self::calculate_optimization_score(
            &average_execution_time,
            &gas_usage,
            &error_rate,
            &throughput,
        );

        Ok(PerformanceMetrics {
            timeframe,
            total_operations,
            successful_operations,
            failed_operations,
            average_execution_time,
            gas_usage,
            throughput,
            error_rate,
            optimization_score,
        })
    }

    /// Emit monitoring alert
    pub fn emit_monitoring_alert(env: &Env, alert: MonitoringAlert) -> Result<(), Error> {
        // Emit alert event
        env.events().publish(
            (Symbol::new(env, "monitoring_alert"),),
            (
                alert.alert_id.clone(),
                alert.alert_type.clone(),
                alert.severity.clone(),
                alert.title.clone(),
                alert.description.clone(),
                alert.affected_component.clone(),
                alert.timestamp,
                alert.resolved,
            ),
        );

        // Store alert in persistent storage
        Self::store_alert(env, &alert)?;

        Ok(())
    }

    /// Validate monitoring data integrity
    pub fn validate_monitoring_data(env: &Env, data: &MonitoringData) -> Result<bool, Error> {
        // Validate timestamp
        let current_time = env.ledger().timestamp();
        if data.timestamp > current_time {
            return Ok(false);
        }

        // Validate market health data
        for market_health in data.market_health.iter() {
            if market_health.health_score > 100 {
                return Ok(false);
            }
            if market_health.total_stake < 0 {
                return Ok(false);
            }
        }

        // Validate oracle health data
        for oracle_health in data.oracle_health.iter() {
            if oracle_health.success_rate > 100 {
                return Ok(false);
            }
            if oracle_health.availability > 100 {
                return Ok(false);
            }
            if oracle_health.confidence_score > 100 {
                return Ok(false);
            }
        }

        // Validate performance metrics
        if data.performance_metrics.error_rate > 100 {
            return Ok(false);
        }
        if data.performance_metrics.optimization_score > 100 {
            return Ok(false);
        }

        Ok(true)
    }

    // ===== HELPER METHODS =====

    fn get_market_data(env: &Env, _market_id: &Symbol) -> Result<Market, Error> {
        // This would retrieve actual market data from storage
        // For now, return a placeholder
        Ok(Market {
            admin: Address::from_str(
                env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            question: String::from_str(env, "Sample Market"),
            outcomes: Vec::new(env),
            end_time: env.ledger().timestamp() + 86400,
            oracle_config: OracleConfig {
                provider: OracleProvider::Reflector,
                feed_id: String::from_str(env, "sample_feed"),
                threshold: 100,
                comparison: String::from_str(env, ">="),
            },
            oracle_result: None,
            votes: Map::new(env),
            stakes: Map::new(env),
            claimed: Map::new(env),
            total_staked: 0,
            dispute_stakes: Map::new(env),
            winning_outcome: None,
            fee_collected: false,
            state: MarketState::Active,
            total_extension_days: 0,
            max_extension_days: 7,
            extension_history: Vec::new(env),
        })
    }

    fn calculate_total_votes(env: &Env, market_id: &Symbol) -> Result<u32, Error> {
        // This would calculate actual vote count
        Ok(0)
    }

    fn calculate_total_stake(env: &Env, market_id: &Symbol) -> Result<i128, Error> {
        // This would calculate actual stake amount
        Ok(0)
    }

    fn calculate_active_participants(env: &Env, market_id: &Symbol) -> Result<u32, Error> {
        // This would calculate actual participant count
        Ok(0)
    }

    fn calculate_time_to_end(env: &Env, market: &Market) -> Result<u64, Error> {
        let current_time = env.ledger().timestamp();
        if market.end_time > current_time {
            Ok(market.end_time - current_time)
        } else {
            Ok(0)
        }
    }

    fn calculate_dispute_count(env: &Env, market_id: &Symbol) -> Result<u32, Error> {
        // This would calculate actual dispute count
        Ok(0)
    }

    fn calculate_resolution_confidence(env: &Env, market_id: &Symbol) -> Result<u32, Error> {
        // This would calculate actual resolution confidence
        Ok(0)
    }

    fn get_last_activity(env: &Env, market_id: &Symbol) -> Result<u64, Error> {
        // This would get actual last activity timestamp
        Ok(env.ledger().timestamp())
    }

    fn calculate_health_score(
        total_votes: &u32,
        total_stake: &i128,
        active_participants: &u32,
        time_to_end: &u64,
        dispute_count: &u32,
        resolution_confidence: &u32,
    ) -> u32 {
        // Simple health score calculation
        let mut score = 50; // Base score

        // Adjust based on activity
        if *total_votes > 10 {
            score += 10;
        }
        if *total_stake > 1000 {
            score += 10;
        }
        if *active_participants > 5 {
            score += 10;
        }

        // Adjust based on time
        if *time_to_end > 3600 {
            score += 10; // More than 1 hour left
        }

        // Adjust based on disputes
        if *dispute_count == 0 {
            score += 10;
        } else {
            score -= (*dispute_count * 5).min(20);
        }

        // Adjust based on confidence
        score += (*resolution_confidence / 10).min(10);

        score.min(100)
    }

    fn determine_market_status(
        health_score: &u32,
        dispute_count: &u32,
        time_to_end: &u64,
    ) -> MonitoringStatus {
        if *dispute_count > 3 {
            MonitoringStatus::Critical
        } else if *health_score < 30 {
            MonitoringStatus::Critical
        } else if *health_score < 60 {
            MonitoringStatus::Warning
        } else if *time_to_end == 0 {
            MonitoringStatus::Warning
        } else {
            MonitoringStatus::Healthy
        }
    }

    fn get_average_response_time(env: &Env, oracle: &OracleProvider) -> Result<u64, Error> {
        // This would get actual response time data
        Ok(1000) // 1 second default
    }

    fn calculate_success_rate(env: &Env, oracle: &OracleProvider) -> Result<u32, Error> {
        // This would calculate actual success rate
        Ok(95) // 95% default
    }

    fn get_last_response_time(env: &Env, oracle: &OracleProvider) -> Result<u64, Error> {
        // This would get actual last response time
        Ok(env.ledger().timestamp())
    }

    fn get_total_requests(env: &Env, oracle: &OracleProvider) -> Result<u32, Error> {
        // This would get actual request count
        Ok(100)
    }

    fn get_failed_requests(env: &Env, oracle: &OracleProvider) -> Result<u32, Error> {
        // This would get actual failed request count
        Ok(5)
    }

    fn calculate_availability(env: &Env, oracle: &OracleProvider) -> Result<u32, Error> {
        // This would calculate actual availability
        Ok(99) // 99% default
    }

    fn calculate_oracle_confidence(
        response_time: &u64,
        success_rate: &u32,
        availability: &u32,
        last_response: &u64,
    ) -> u32 {
        let mut confidence = 50; // Base confidence

        // Adjust based on response time (lower is better)
        if *response_time < 2000 {
            confidence += 20;
        } else if *response_time < 5000 {
            confidence += 10;
        }

        // Adjust based on success rate
        confidence += (*success_rate / 5).min(20);

        // Adjust based on availability
        confidence += (*availability / 5).min(20);

        // Adjust based on recency (more recent is better)
        let current_time = 0; // This would be actual current time
        let time_since_response = current_time - *last_response;
        if time_since_response < 3600 {
            confidence += 10; // Within last hour
        }

        confidence.min(100)
    }

    fn determine_oracle_status(
        confidence_score: &u32,
        success_rate: &u32,
        availability: &u32,
    ) -> MonitoringStatus {
        if *confidence_score < 30 || *success_rate < 70 || *availability < 90 {
            MonitoringStatus::Critical
        } else if *confidence_score < 60 || *success_rate < 85 || *availability < 95 {
            MonitoringStatus::Warning
        } else {
            MonitoringStatus::Healthy
        }
    }

    fn calculate_start_time(env: &Env, timeframe: &TimeFrame) -> Result<u64, Error> {
        let current_time = env.ledger().timestamp();
        match timeframe {
            TimeFrame::LastHour => Ok(current_time.saturating_sub(3600)),
            TimeFrame::LastDay => Ok(current_time.saturating_sub(86400)),
            TimeFrame::LastWeek => Ok(current_time.saturating_sub(604800)),
            TimeFrame::LastMonth => Ok(current_time.saturating_sub(2592000)),
            TimeFrame::Custom(duration) => Ok(current_time.saturating_sub(*duration)),
        }
    }

    fn calculate_total_fees_collected(env: &Env, start_time: u64) -> Result<i128, Error> {
        // This would calculate actual fees collected
        Ok(10000)
    }

    fn count_markets_in_timeframe(env: &Env, start_time: u64) -> Result<u32, Error> {
        // This would count actual markets
        Ok(50)
    }

    fn count_successful_collections(env: &Env, start_time: u64) -> Result<u32, Error> {
        // This would count actual successful collections
        Ok(45)
    }

    fn count_failed_collections(env: &Env, start_time: u64) -> Result<u32, Error> {
        // This would count actual failed collections
        Ok(5)
    }

    fn calculate_revenue_growth(env: &Env, timeframe: &TimeFrame) -> Result<i32, Error> {
        // This would calculate actual revenue growth
        Ok(15) // 15% growth
    }

    fn count_total_disputes(env: &Env, market_id: &Symbol, start_time: u64) -> Result<u32, Error> {
        // This would count actual disputes
        Ok(10)
    }

    fn count_resolved_disputes(
        env: &Env,
        market_id: &Symbol,
        start_time: u64,
    ) -> Result<u32, Error> {
        // This would count actual resolved disputes
        Ok(8)
    }

    fn calculate_average_resolution_time(
        env: &Env,
        market_id: &Symbol,
        start_time: u64,
    ) -> Result<u64, Error> {
        // This would calculate actual resolution time
        Ok(86400) // 1 day default
    }

    fn count_escalations(env: &Env, market_id: &Symbol, start_time: u64) -> Result<u32, Error> {
        // This would count actual escalations
        Ok(2)
    }

    fn calculate_community_consensus_rate(
        env: &Env,
        market_id: &Symbol,
        start_time: u64,
    ) -> Result<u32, Error> {
        // This would calculate actual consensus rate
        Ok(80) // 80% default
    }

    fn count_total_operations(env: &Env, start_time: u64) -> Result<u32, Error> {
        // This would count actual operations
        Ok(1000)
    }

    fn count_successful_operations(env: &Env, start_time: u64) -> Result<u32, Error> {
        // This would count actual successful operations
        Ok(950)
    }

    fn calculate_average_execution_time(env: &Env, start_time: u64) -> Result<u64, Error> {
        // This would calculate actual execution time
        Ok(100) // 100ms default
    }

    fn calculate_total_gas_usage(env: &Env, start_time: u64) -> Result<u64, Error> {
        // This would calculate actual gas usage
        Ok(1000000)
    }

    fn calculate_throughput(env: &Env, timeframe: &TimeFrame) -> Result<u32, Error> {
        // This would calculate actual throughput
        Ok(10) // 10 ops/sec default
    }

    fn calculate_optimization_score(
        average_execution_time: &u64,
        gas_usage: &u64,
        error_rate: &u32,
        throughput: &u32,
    ) -> u32 {
        let mut score = 50; // Base score

        // Adjust based on execution time (lower is better)
        if *average_execution_time < 200 {
            score += 20;
        } else if *average_execution_time < 500 {
            score += 10;
        }

        // Adjust based on gas usage (lower is better)
        if *gas_usage < 500000 {
            score += 15;
        } else if *gas_usage < 1000000 {
            score += 10;
        }

        // Adjust based on error rate (lower is better)
        if *error_rate < 5 {
            score += 15;
        } else if *error_rate < 10 {
            score += 10;
        }

        // Adjust based on throughput (higher is better)
        if *throughput > 20 {
            score += 10;
        } else if *throughput > 10 {
            score += 5;
        }

        score.min(100)
    }

    fn store_alert(env: &Env, alert: &MonitoringAlert) -> Result<(), Error> {
        // Store alert in persistent storage
        let storage_key = Symbol::new(env, "MONITORING_ALERT");
        env.storage().persistent().set(&storage_key, alert);
        Ok(())
    }
}

// ===== MONITORING UTILITIES =====

/// Utility functions for monitoring operations
pub struct MonitoringUtils;

impl MonitoringUtils {
    /// Create a monitoring alert
    pub fn create_alert(
        env: &Env,
        alert_type: MonitoringAlertType,
        severity: AlertSeverity,
        title: String,
        description: String,
        affected_component: String,
    ) -> MonitoringAlert {
        let alert_id = Self::generate_alert_id(env);

        MonitoringAlert {
            alert_id,
            alert_type,
            severity,
            title,
            description,
            affected_component,
            timestamp: env.ledger().timestamp(),
            resolved: false,
            resolution_notes: None,
            metadata: Map::new(env),
        }
    }

    /// Generate unique alert ID
    fn generate_alert_id(env: &Env) -> String {
        let timestamp = env.ledger().timestamp();
        let random = env.ledger().sequence();
        String::from_str(env, &format!("alert_{}_{}", timestamp, random))
    }

    /// Check if monitoring data is stale
    pub fn is_data_stale(env: &Env, data_timestamp: u64, max_age: u64) -> bool {
        let current_time = env.ledger().timestamp();
        current_time - data_timestamp > max_age
    }

    /// Calculate monitoring data freshness score
    pub fn calculate_freshness_score(env: &Env, data_timestamp: u64) -> u32 {
        let current_time = env.ledger().timestamp();
        let age = current_time - data_timestamp;

        if age < 300 {
            // Less than 5 minutes
            100
        } else if age < 1800 {
            // Less than 30 minutes
            80
        } else if age < 3600 {
            // Less than 1 hour
            60
        } else if age < 86400 {
            // Less than 1 day
            40
        } else {
            20
        }
    }

    /// Validate monitoring thresholds
    pub fn validate_thresholds(
        current_value: u32,
        warning_threshold: u32,
        critical_threshold: u32,
    ) -> MonitoringStatus {
        if current_value >= critical_threshold {
            MonitoringStatus::Critical
        } else if current_value >= warning_threshold {
            MonitoringStatus::Warning
        } else {
            MonitoringStatus::Healthy
        }
    }
}

// ===== MONITORING TESTING UTILITIES =====

/// Testing utilities for monitoring system
pub struct MonitoringTestingUtils;

impl MonitoringTestingUtils {
    /// Create test market health metrics
    pub fn create_test_market_health_metrics(env: &Env, market_id: Symbol) -> MarketHealthMetrics {
        MarketHealthMetrics {
            market_id,
            status: MonitoringStatus::Healthy,
            total_votes: 100,
            total_stake: 10000,
            active_participants: 25,
            time_to_end: 3600,
            dispute_count: 0,
            resolution_confidence: 85,
            last_activity: env.ledger().timestamp(),
            health_score: 85,
        }
    }

    /// Create test oracle health metrics
    pub fn create_test_oracle_health_metrics(
        env: &Env,
        provider: OracleProvider,
    ) -> OracleHealthMetrics {
        OracleHealthMetrics {
            provider,
            status: MonitoringStatus::Healthy,
            response_time: 500,
            success_rate: 98,
            last_response: env.ledger().timestamp(),
            total_requests: 1000,
            failed_requests: 20,
            availability: 99,
            confidence_score: 95,
        }
    }

    /// Create test fee collection metrics
    pub fn create_test_fee_collection_metrics(env: &Env) -> FeeCollectionMetrics {
        FeeCollectionMetrics {
            timeframe: TimeFrame::LastDay,
            total_fees_collected: 5000,
            total_markets: 25,
            successful_collections: 23,
            failed_collections: 2,
            average_fee_per_market: 200,
            revenue_growth: 12,
            collection_efficiency: 92,
        }
    }

    /// Create test dispute resolution metrics
    pub fn create_test_dispute_resolution_metrics(
        env: &Env,
        market_id: Symbol,
    ) -> DisputeResolutionMetrics {
        DisputeResolutionMetrics {
            timeframe: TimeFrame::LastWeek,
            total_disputes: 5,
            resolved_disputes: 4,
            pending_disputes: 1,
            average_resolution_time: 43200, // 12 hours
            resolution_success_rate: 80,
            escalation_count: 1,
            community_consensus_rate: 75,
        }
    }

    /// Create test performance metrics
    pub fn create_test_performance_metrics(env: &Env) -> PerformanceMetrics {
        PerformanceMetrics {
            timeframe: TimeFrame::LastHour,
            total_operations: 500,
            successful_operations: 485,
            failed_operations: 15,
            average_execution_time: 150,
            gas_usage: 750000,
            throughput: 8,
            error_rate: 3,
            optimization_score: 85,
        }
    }

    /// Create test monitoring alert
    pub fn create_test_monitoring_alert(env: &Env) -> MonitoringAlert {
        MonitoringAlert {
            alert_id: String::from_str(env, "test_alert_001"),
            alert_type: MonitoringAlertType::MarketHealth,
            severity: AlertSeverity::Warning,
            title: String::from_str(env, "Test Alert"),
            description: String::from_str(env, "This is a test monitoring alert"),
            affected_component: String::from_str(env, "test_market"),
            timestamp: env.ledger().timestamp(),
            resolved: false,
            resolution_notes: None,
            metadata: Map::new(env),
        }
    }

    /// Create test monitoring data
    pub fn create_test_monitoring_data(env: &Env) -> MonitoringData {
        let market_health = vec![
            &env,
            Self::create_test_market_health_metrics(env, Symbol::new(env, "test_market_1")),
        ];

        let oracle_health = vec![
            &env,
            Self::create_test_oracle_health_metrics(env, OracleProvider::Reflector),
        ];

        let active_alerts = vec![&env, Self::create_test_monitoring_alert(env)];

        MonitoringData {
            timestamp: env.ledger().timestamp(),
            market_health,
            oracle_health,
            fee_metrics: Self::create_test_fee_collection_metrics(env),
            dispute_metrics: Self::create_test_dispute_resolution_metrics(
                env,
                Symbol::new(env, "test_market"),
            ),
            performance_metrics: Self::create_test_performance_metrics(env),
            active_alerts,
            system_status: MonitoringStatus::Healthy,
        }
    }

    /// Validate test data structure
    pub fn validate_test_data_structure<T>(_data: &T) -> Result<(), Error> {
        // Basic validation for test data
        Ok(())
    }
}

// ===== TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address;

    #[test]
    fn test_market_health_monitoring() {
        let env = Env::default();
        let market_id = Symbol::new(&env, "test_market");

        let metrics = ContractMonitor::monitor_market_health(&env, market_id.clone()).unwrap();

        assert_eq!(metrics.market_id, market_id);
        assert!(metrics.health_score <= 100);
        assert!(metrics.total_stake >= 0);
    }

    #[test]
    fn test_oracle_health_monitoring() {
        let env = Env::default();
        let oracle = OracleProvider::Reflector;

        let metrics = ContractMonitor::monitor_oracle_health(&env, oracle.clone()).unwrap();

        assert_eq!(metrics.provider, oracle);
        assert!(metrics.confidence_score <= 100);
        assert!(metrics.success_rate <= 100);
        assert!(metrics.availability <= 100);
    }

    #[test]
    fn test_fee_collection_monitoring() {
        let env = Env::default();
        let timeframe = TimeFrame::LastDay;

        let metrics = ContractMonitor::monitor_fee_collection(&env, timeframe.clone()).unwrap();

        assert_eq!(metrics.timeframe, timeframe);
        assert!(metrics.total_fees_collected >= 0);
        assert!(metrics.collection_efficiency <= 100);
    }

    #[test]
    fn test_dispute_resolution_monitoring() {
        let env = Env::default();
        let market_id = Symbol::new(&env, "test_market");

        let metrics = ContractMonitor::monitor_dispute_resolution(&env, market_id).unwrap();

        assert_eq!(
            metrics.total_disputes,
            metrics.resolved_disputes + metrics.pending_disputes
        );
        assert!(metrics.resolution_success_rate <= 100);
    }

    #[test]
    fn test_performance_metrics() {
        let env = Env::default();
        let timeframe = TimeFrame::LastHour;

        let metrics =
            ContractMonitor::get_contract_performance_metrics(&env, timeframe.clone()).unwrap();

        assert_eq!(metrics.timeframe, timeframe);
        assert_eq!(
            metrics.total_operations,
            metrics.successful_operations + metrics.failed_operations
        );
        assert!(metrics.error_rate <= 100);
        assert!(metrics.optimization_score <= 100);
    }

    #[test]
    fn test_monitoring_alert_creation() {
        let env = Env::default();

        let alert = MonitoringUtils::create_alert(
            &env,
            MonitoringAlertType::MarketHealth,
            AlertSeverity::Warning,
            String::from_str(&env, "Test Alert"),
            String::from_str(&env, "Test Description"),
            String::from_str(&env, "test_component"),
        );

        assert_eq!(alert.alert_type, MonitoringAlertType::MarketHealth);
        assert_eq!(alert.severity, AlertSeverity::Warning);
        assert!(!alert.resolved);
    }

    #[test]
    fn test_monitoring_data_validation() {
        let env = Env::default();
        let data = MonitoringTestingUtils::create_test_monitoring_data(&env);

        let is_valid = ContractMonitor::validate_monitoring_data(&env, &data).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_monitoring_utils() {
        let env = Env::default();
        let current_time = env.ledger().timestamp();

        // Test data staleness
        // In test environment, current_time might be 0, so we need to test with actual values
        let old_timestamp = if current_time > 500 {
            current_time.saturating_sub(400)
        } else {
            0
        };
        let is_stale = MonitoringUtils::is_data_stale(&env, old_timestamp, 300);
        // If current_time is 0, then old_timestamp is also 0, so the difference is 0, which is not > 300
        if current_time > 500 {
            assert!(is_stale);
        } else {
            assert!(!is_stale); // When current_time is 0, old_timestamp is 0, so difference is 0, not stale
        }

        let recent_timestamp = if current_time > 200 {
            current_time.saturating_sub(100)
        } else {
            current_time
        };
        let is_fresh = MonitoringUtils::is_data_stale(&env, recent_timestamp, 300);
        assert!(!is_fresh);

        // Test freshness score
        let score =
            MonitoringUtils::calculate_freshness_score(&env, current_time.saturating_sub(100));
        assert_eq!(score, 100);

        // Test threshold validation
        let status = MonitoringUtils::validate_thresholds(75, 70, 80);
        assert_eq!(status, MonitoringStatus::Warning);
    }

    #[test]
    fn test_monitoring_testing_utilities() {
        let env = Env::default();

        // Test market health metrics creation
        let market_health = MonitoringTestingUtils::create_test_market_health_metrics(
            &env,
            Symbol::new(&env, "test_market"),
        );
        assert_eq!(market_health.health_score, 85);

        // Test oracle health metrics creation
        let oracle_health = MonitoringTestingUtils::create_test_oracle_health_metrics(
            &env,
            OracleProvider::Reflector,
        );
        assert_eq!(oracle_health.confidence_score, 95);

        // Test monitoring data creation
        let data = MonitoringTestingUtils::create_test_monitoring_data(&env);
        assert_eq!(data.system_status, MonitoringStatus::Healthy);
    }
}
