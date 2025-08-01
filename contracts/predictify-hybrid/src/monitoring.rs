extern crate alloc;

use soroban_sdk::{vec, Address, Env, Map, String, Symbol, Vec};
use crate::types::{MonitoringAlert, MonitoringData, TimeFrame, OracleProvider};
use crate::errors::Error;
use crate::markets::MarketStateManager;
use crate::fees::FeeManager;
use crate::disputes::DisputeManager;

pub struct ContractMonitor;

impl ContractMonitor {
    /// Analyse market health (e.g. liquidity, open interest, status).
    pub fn monitor_market_health(env: &Env, market_id: Symbol) -> MonitoringData {
        // Placeholder calculations
        let liquidity: i128 = 0; // TODO: compute real liquidity
        let open_interest: i128 = 0; // TODO: compute real open interest

        MonitoringData::MarketHealth { market_id, liquidity, open_interest }
    }

    /// Assess oracle provider status (online/offline, response time, etc.).
    pub fn monitor_oracle_health(env: &Env, provider: OracleProvider) -> MonitoringData {
        // TODO: query oracle manager for status
        let is_online = true;
        MonitoringData::OracleHealth { provider: String::from_str(env, provider.name()), is_online }
    }

    /// Aggregate fee collection within a given timeframe.
    pub fn monitor_fee_collection(env: &Env, timeframe: TimeFrame) -> MonitoringData {
        // TODO: derive revenue numbers using FeeManager analytics
        let revenue: i128 = 0;
        MonitoringData::FeeRevenue { timeframe, amount: revenue }
    }

    /// Track dispute status for a market.
    pub fn monitor_dispute_resolution(env: &Env, market_id: Symbol) -> MonitoringData {
        // TODO: inspect DisputeManager to count open disputes
        let open_disputes: u32 = 0;
        MonitoringData::DisputeStatus { market_id, open_disputes }
    }

    /// Retrieve high-level contract performance metrics.
    pub fn get_contract_performance_metrics(env: &Env) -> MonitoringData {
        // TODO: gather actual metrics once performance tracker exists
        let tx_count: u32 = 0;
        let avg_gas: i128 = 0;
        MonitoringData::PerformanceMetrics { tx_count, avg_gas }
    }

    /// Emit monitoring alert using Soroban event log + internal logger.
    pub fn emit_monitoring_alert(env: &Env, alert: MonitoringAlert) {
        // Log via EventLogger if available, fall back to env events.
        let topic = (Symbol::new(env, "MonitoringAlert"), alert.alert_type.clone());
        env.events().publish(topic, alert.clone());


    }

    /// Validate monitoring data (basic example).
    /// Returns Ok(()) if valid, Err otherwise.
    pub fn validate_monitoring_data(_env: &Env, data: &MonitoringData) -> Result<(), Error> {
        match data {
            MonitoringData::MarketHealth { liquidity, open_interest, .. } => {
                if *liquidity < 0 || *open_interest < 0 {
                    Err(Error::InvalidInput)
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        }
    }
}

/// Convenience helpers (thin wrappers around [`ContractMonitor`] impl).
pub mod helpers {
    use super::*;

    pub fn monitor_and_alert_market(env: &Env, market_id: Symbol) {
        let data = ContractMonitor::monitor_market_health(env, market_id);
        if let Err(_e) = ContractMonitor::validate_monitoring_data(env, &data) {
            let alert = MonitoringAlert {
                alert_type: String::from_str(env, "InvalidMarketData"),
                message: String::from_str(env, "Market health data failed validation"),
                severity: 1,
                timestamp: env.ledger().timestamp(),
            };
            ContractMonitor::emit_monitoring_alert(env, alert);
        }
    }
}
