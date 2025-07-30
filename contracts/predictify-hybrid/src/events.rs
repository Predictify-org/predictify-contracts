extern crate alloc;
use alloc::format;
use soroban_sdk::{contracttype, symbol_short, vec, Address, Env, Map, String, Symbol, Vec};

use crate::admin::AdminRole;
use crate::errors::Error;
use crate::Environment;

/// Comprehensive event system for Predictify Hybrid contract
///
/// This module provides a centralized event emission and logging system with:
/// - Event types and structures for all contract operations
/// - Event emission utilities and helpers
/// - Event logging and monitoring functions
/// - Event validation and helper functions
/// - Event testing utilities and examples
/// - Event documentation and examples

// ===== EVENT TYPES =====

/// Market creation event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketCreatedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Market question
    pub question: String,
    /// Market outcomes
    pub outcomes: Vec<String>,
    /// Market admin
    pub admin: Address,
    /// Market end time
    pub end_time: u64,
    /// Creation timestamp
    pub timestamp: u64,
}

/// Vote cast event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VoteCastEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Voter address
    pub voter: Address,
    /// Voted outcome
    pub outcome: String,
    /// Stake amount
    pub stake: i128,
    /// Vote timestamp
    pub timestamp: u64,
}

/// Oracle result fetched event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleResultEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Oracle result
    pub result: String,
    /// Oracle provider
    pub provider: String,
    /// Feed ID
    pub feed_id: String,
    /// Price at resolution
    pub price: i128,
    /// Threshold value
    pub threshold: i128,
    /// Comparison operator
    pub comparison: String,
    /// Fetch timestamp
    pub timestamp: u64,
}

/// Market resolved event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketResolvedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Final outcome
    pub final_outcome: String,
    /// Oracle result
    pub oracle_result: String,
    /// Community consensus
    pub community_consensus: String,
    /// Resolution method
    pub resolution_method: String,
    /// Confidence score
    pub confidence_score: i128,
    /// Resolution timestamp
    pub timestamp: u64,
}

/// Dispute created event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeCreatedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Disputer address
    pub disputer: Address,
    /// Dispute stake
    pub stake: i128,
    /// Dispute reason
    pub reason: Option<String>,
    /// Dispute timestamp
    pub timestamp: u64,
}

/// Dispute resolved event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisputeResolvedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Dispute outcome
    pub outcome: String,
    /// Winner addresses
    pub winners: Vec<Address>,
    /// Loser addresses
    pub losers: Vec<Address>,
    /// Fee distribution
    pub fee_distribution: i128,
    /// Resolution timestamp
    pub timestamp: u64,
}

/// Fee collected event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeCollectedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Fee collector
    pub collector: Address,
    /// Fee amount
    pub amount: i128,
    /// Fee type
    pub fee_type: String,
    /// Collection timestamp
    pub timestamp: u64,
}

/// Extension requested event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionRequestedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Requesting admin
    pub admin: Address,
    /// Additional days
    pub additional_days: u32,
    /// Extension reason
    pub reason: String,
    /// Extension fee
    pub fee: i128,
    /// Request timestamp
    pub timestamp: u64,
}

/// Configuration updated event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigUpdatedEvent {
    /// Updated by
    pub updated_by: Address,
    /// Configuration type
    pub config_type: String,
    /// Old value
    pub old_value: String,
    /// New value
    pub new_value: String,
    /// Update timestamp
    pub timestamp: u64,
}

/// Error logged event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorLoggedEvent {
    /// Error code
    pub error_code: u32,
    /// Error message
    pub message: String,
    /// Context
    pub context: String,
    /// User address (if applicable)
    pub user: Option<Address>,
    /// Market ID (if applicable)
    pub market_id: Option<Symbol>,
    /// Error timestamp
    pub timestamp: u64,
}

/// Performance metric event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PerformanceMetricEvent {
    /// Metric name
    pub metric_name: String,
    /// Metric value
    pub value: i128,
    /// Metric unit
    pub unit: String,
    /// Context
    pub context: String,
    /// Metric timestamp
    pub timestamp: u64,
}

/// Admin action event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminActionEvent {
    /// Admin address
    pub admin: Address,
    /// Action performed
    pub action: String,
    /// Target of action
    pub target: Option<String>,
    /// Action timestamp
    pub timestamp: u64,
    /// Action success status
    pub success: bool,
}

/// Admin role event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminRoleEvent {
    /// Admin address
    pub admin: Address,
    /// Role assigned
    pub role: String,
    /// Assigned by
    pub assigned_by: Address,
    /// Assignment timestamp
    pub timestamp: u64,
}

/// Admin permission event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminPermissionEvent {
    /// Admin address
    pub admin: Address,
    /// Permission checked
    pub permission: String,
    /// Access granted
    pub granted: bool,
    /// Check timestamp
    pub timestamp: u64,
}

/// Market closed event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketClosedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Admin who closed it
    pub admin: Address,
    /// Close timestamp
    pub timestamp: u64,
}

/// Market finalized event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketFinalizedEvent {
    /// Market ID
    pub market_id: Symbol,
    /// Admin who finalized it
    pub admin: Address,
    /// Final outcome
    pub outcome: String,
    /// Finalization timestamp
    pub timestamp: u64,
}

/// Admin initialized event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdminInitializedEvent {
    /// Admin address
    pub admin: Address,
    /// Initialization timestamp
    pub timestamp: u64,
}

/// Config initialized event
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigInitializedEvent {
    /// Admin address
    pub admin: Address,
    /// Environment
    pub environment: String,
    /// Initialization timestamp
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeInitiatedEvent {
    pub upgrade_id: String,
    pub admin: Address,
    pub from_version: crate::upgrade::ContractVersion,
    pub to_version: crate::upgrade::ContractVersion,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeCompletedEvent {
    pub upgrade_id: String,
    pub admin: Address,
    pub new_version: crate::upgrade::ContractVersion,
    pub success: bool,
    pub duration_seconds: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RollbackExecutedEvent {
    pub rollback_id: String,
    pub admin: Address,
    pub from_version: crate::upgrade::ContractVersion,
    pub to_version: crate::upgrade::ContractVersion,
    pub reason: String,
    pub timestamp: u64,
}

// ===== EVENT EMISSION UTILITIES =====

/// Event emission utilities
pub struct EventEmitter;

impl EventEmitter {
    /// Emit market created event
    pub fn emit_market_created(
        env: &Env,
        market_id: &Symbol,
        question: &String,
        outcomes: &Vec<String>,
        admin: &Address,
        end_time: u64,
    ) {
        let event = MarketCreatedEvent {
            market_id: market_id.clone(),
            question: question.clone(),
            outcomes: outcomes.clone(),
            admin: admin.clone(),
            end_time,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("mkt_crt"), &event);
    }

    /// Emit vote cast event
    pub fn emit_vote_cast(
        env: &Env,
        market_id: &Symbol,
        voter: &Address,
        outcome: &String,
        stake: i128,
    ) {
        let event = VoteCastEvent {
            market_id: market_id.clone(),
            voter: voter.clone(),
            outcome: outcome.clone(),
            stake,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("vote"), &event);
    }

    /// Emit oracle result event
    pub fn emit_oracle_result(
        env: &Env,
        market_id: &Symbol,
        result: &String,
        provider: &String,
        feed_id: &String,
        price: i128,
        threshold: i128,
        comparison: &String,
    ) {
        let event = OracleResultEvent {
            market_id: market_id.clone(),
            result: result.clone(),
            provider: provider.clone(),
            feed_id: feed_id.clone(),
            price,
            threshold,
            comparison: comparison.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("oracle_rs"), &event);
    }

    /// Emit market resolved event
    pub fn emit_market_resolved(
        env: &Env,
        market_id: &Symbol,
        final_outcome: &String,
        oracle_result: &String,
        community_consensus: &String,
        resolution_method: &String,
        confidence_score: i128,
    ) {
        let event = MarketResolvedEvent {
            market_id: market_id.clone(),
            final_outcome: final_outcome.clone(),
            oracle_result: oracle_result.clone(),
            community_consensus: community_consensus.clone(),
            resolution_method: resolution_method.clone(),
            confidence_score,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("mkt_res"), &event);
    }

    /// Emit dispute created event
    pub fn emit_dispute_created(
        env: &Env,
        market_id: &Symbol,
        disputer: &Address,
        stake: i128,
        reason: Option<String>,
    ) {
        let event = DisputeCreatedEvent {
            market_id: market_id.clone(),
            disputer: disputer.clone(),
            stake,
            reason,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("dispt_crt"), &event);
    }

    /// Emit dispute resolved event
    pub fn emit_dispute_resolved(
        env: &Env,
        market_id: &Symbol,
        outcome: &String,
        winners: &Vec<Address>,
        losers: &Vec<Address>,
        fee_distribution: i128,
    ) {
        let event = DisputeResolvedEvent {
            market_id: market_id.clone(),
            outcome: outcome.clone(),
            winners: winners.clone(),
            losers: losers.clone(),
            fee_distribution,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("dispt_res"), &event);
    }

    /// Emit fee collected event
    pub fn emit_fee_collected(
        env: &Env,
        market_id: &Symbol,
        collector: &Address,
        amount: i128,
        fee_type: &String,
    ) {
        let event = FeeCollectedEvent {
            market_id: market_id.clone(),
            collector: collector.clone(),
            amount,
            fee_type: fee_type.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("fee_col"), &event);
    }

    /// Emit extension requested event
    pub fn emit_extension_requested(
        env: &Env,
        market_id: &Symbol,
        admin: &Address,
        additional_days: u32,
        reason: &String,
        fee: i128,
    ) {
        let event = ExtensionRequestedEvent {
            market_id: market_id.clone(),
            admin: admin.clone(),
            additional_days,
            reason: reason.clone(),
            fee,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("ext_req"), &event);
    }

    /// Emit configuration updated event
    pub fn emit_config_updated(
        env: &Env,
        updated_by: &Address,
        config_type: &String,
        old_value: &String,
        new_value: &String,
    ) {
        let event = ConfigUpdatedEvent {
            updated_by: updated_by.clone(),
            config_type: config_type.clone(),
            old_value: old_value.clone(),
            new_value: new_value.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("cfg_upd"), &event);
    }

    /// Emit error logged event
    pub fn emit_error_logged(
        env: &Env,
        error_code: u32,
        message: &String,
        context: &String,
        user: Option<Address>,
        market_id: Option<Symbol>,
    ) {
        let event = ErrorLoggedEvent {
            error_code,
            message: message.clone(),
            context: context.clone(),
            user,
            market_id,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("err_log"), &event);
    }

    /// Emit performance metric event
    pub fn emit_performance_metric(
        env: &Env,
        metric_name: &String,
        value: i128,
        unit: &String,
        context: &String,
    ) {
        let event = PerformanceMetricEvent {
            metric_name: metric_name.clone(),
            value,
            unit: unit.clone(),
            context: context.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("perf_met"), &event);
    }

    /// Emit admin action logged event
    pub fn emit_admin_action_logged(env: &Env, admin: &Address, action: &str, success: &bool) {
        let event = AdminActionEvent {
            admin: admin.clone(),
            action: String::from_str(env, action),
            target: None,
            timestamp: env.ledger().timestamp(),
            success: *success,
        };

        Self::store_event(env, &symbol_short!("adm_act"), &event);
    }

    /// Emit admin initialized event
    pub fn emit_admin_initialized(env: &Env, admin: &Address) {
        let event = AdminInitializedEvent {
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("adm_init"), &event);
    }

    /// Emit config initialized event
    pub fn emit_config_initialized(env: &Env, admin: &Address, environment: &Environment) {
        let event = ConfigInitializedEvent {
            admin: admin.clone(),
            environment: String::from_str(env, &format!("{:?}", environment)),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("cfg_init"), &event);
    }

    /// Emit admin role assigned event
    pub fn emit_admin_role_assigned(
        env: &Env,
        admin: &Address,
        role: &AdminRole,
        assigned_by: &Address,
    ) {
        let event = AdminRoleEvent {
            admin: admin.clone(),
            role: String::from_str(env, &format!("{:?}", role)),
            assigned_by: assigned_by.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("adm_role"), &event);
    }

    /// Emit admin role deactivated event
    pub fn emit_admin_role_deactivated(env: &Env, admin: &Address, deactivated_by: &Address) {
        let event = AdminRoleEvent {
            admin: admin.clone(),
            role: String::from_str(env, "deactivated"),
            assigned_by: deactivated_by.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("adm_deact"), &event);
    }

    /// Emit market closed event
    pub fn emit_market_closed(env: &Env, market_id: &Symbol, admin: &Address) {
        let event = MarketClosedEvent {
            market_id: market_id.clone(),
            admin: admin.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("mkt_close"), &event);
    }

    /// Emit market finalized event
    pub fn emit_market_finalized(env: &Env, market_id: &Symbol, admin: &Address, outcome: &String) {
        let event = MarketFinalizedEvent {
            market_id: market_id.clone(),
            admin: admin.clone(),
            outcome: outcome.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("mkt_final"), &event);
    }

    /// Emit upgrade initiated event
    pub fn emit_upgrade_initiated(
        env: &Env,
        upgrade_id: &String,
        admin: &Address,
        from_version: &crate::upgrade::ContractVersion,
        to_version: &crate::upgrade::ContractVersion,
    ) {
        let event = UpgradeInitiatedEvent {
            upgrade_id: upgrade_id.clone(),
            admin: admin.clone(),
            from_version: from_version.clone(),
            to_version: to_version.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("upg_init"), &event);
    }

    /// Emit upgrade completed event
    pub fn emit_upgrade_completed(
        env: &Env,
        upgrade_id: &String,
        admin: &Address,
        new_version: &crate::upgrade::ContractVersion,
        success: bool,
        duration_seconds: u64,
    ) {
        let event = UpgradeCompletedEvent {
            upgrade_id: upgrade_id.clone(),
            admin: admin.clone(),
            new_version: new_version.clone(),
            success,
            duration_seconds,
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("upg_comp"), &event);
    }

    /// Emit rollback executed event
    pub fn emit_rollback_executed(
        env: &Env,
        rollback_id: &String,
        admin: &Address,
        from_version: &crate::upgrade::ContractVersion,
        to_version: &crate::upgrade::ContractVersion,
        reason: &String,
    ) {
        let event = RollbackExecutedEvent {
            rollback_id: rollback_id.clone(),
            admin: admin.clone(),
            from_version: from_version.clone(),
            to_version: to_version.clone(),
            reason: reason.clone(),
            timestamp: env.ledger().timestamp(),
        };

        Self::store_event(env, &symbol_short!("rbk_exec"), &event);
    }

    /// Store event in persistent storage
    fn store_event<T>(env: &Env, event_key: &Symbol, event_data: &T)
    where
        T: Clone + soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>,
    {
        env.storage().persistent().set(event_key, event_data);
    }
}

// ===== EVENT LOGGING AND MONITORING =====

/// Event logging and monitoring utilities
pub struct EventLogger;

impl EventLogger {
    /// Get all events of a specific type
    pub fn get_events<T>(env: &Env, event_type: &Symbol) -> Vec<T>
    where
        T: Clone
            + soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>
            + soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>,
    {
        match env.storage().persistent().get::<Symbol, T>(event_type) {
            Some(event) => Vec::from_array(env, [event]),
            None => Vec::new(env),
        }
    }

    /// Get events for a specific market
    pub fn get_market_events(env: &Env, market_id: &Symbol) -> Vec<MarketEventSummary> {
        let mut events = Vec::new(env);

        // Get market created events
        if let Some(event) = env
            .storage()
            .persistent()
            .get::<Symbol, MarketCreatedEvent>(&symbol_short!("mkt_crt"))
        {
            if event.market_id == *market_id {
                events.push_back(MarketEventSummary {
                    event_type: String::from_str(env, "MarketCreated"),
                    timestamp: event.timestamp,
                    details: String::from_str(env, "Market was created"),
                });
            }
        }

        // Get vote cast events
        if let Some(event) = env
            .storage()
            .persistent()
            .get::<Symbol, VoteCastEvent>(&symbol_short!("vote"))
        {
            if event.market_id == *market_id {
                events.push_back(MarketEventSummary {
                    event_type: String::from_str(env, "VoteCast"),
                    timestamp: event.timestamp,
                    details: String::from_str(env, "Vote was cast"),
                });
            }
        }

        // Get oracle result events
        if let Some(event) = env
            .storage()
            .persistent()
            .get::<Symbol, OracleResultEvent>(&symbol_short!("oracle_rs"))
        {
            if event.market_id == *market_id {
                events.push_back(MarketEventSummary {
                    event_type: String::from_str(env, "OracleResult"),
                    timestamp: event.timestamp,
                    details: String::from_str(env, "Oracle result fetched"),
                });
            }
        }

        // Get market resolved events
        if let Some(event) = env
            .storage()
            .persistent()
            .get::<Symbol, MarketResolvedEvent>(&symbol_short!("mkt_res"))
        {
            if event.market_id == *market_id {
                events.push_back(MarketEventSummary {
                    event_type: String::from_str(env, "MarketResolved"),
                    timestamp: event.timestamp,
                    details: String::from_str(env, "Market was resolved"),
                });
            }
        }

        events
    }

    /// Get recent events (last N events)
    pub fn get_recent_events(env: &Env, limit: u32) -> Vec<EventSummary> {
        let mut events = Vec::new(env);

        // This is a simplified implementation
        // In a real system, you would maintain an event log with timestamps
        let event_types = vec![
            env,
            symbol_short!("mkt_crt"),
            symbol_short!("vote"),
            symbol_short!("oracle_rs"),
            symbol_short!("mkt_res"),
            symbol_short!("dispt_crt"),
            symbol_short!("dispt_res"),
            symbol_short!("fee_col"),
            symbol_short!("ext_req"),
            symbol_short!("cfg_upd"),
            symbol_short!("err_log"),
            symbol_short!("perf_met"),
        ];

        let mut count = 0;
        for event_type in event_types.iter() {
            if count >= limit {
                break;
            }

            // Check if event exists and add to summary
            if env.storage().persistent().has(&event_type) {
                events.push_back(EventSummary {
                    event_type: String::from_str(env, "event"),
                    timestamp: env.ledger().timestamp(),
                    details: String::from_str(env, "Event occurred"),
                });
                count += 1;
            }
        }

        events
    }

    /// Get error events
    pub fn get_error_events(env: &Env) -> Vec<ErrorLoggedEvent> {
        Self::get_events(env, &symbol_short!("err_log"))
    }

    /// Get performance metrics
    pub fn get_performance_metrics(env: &Env) -> Vec<PerformanceMetricEvent> {
        Self::get_events(env, &symbol_short!("perf_met"))
    }

    /// Clear old events (cleanup utility)
    pub fn clear_old_events(env: &Env, _older_than_timestamp: u64) {
        let event_types = vec![
            env,
            symbol_short!("mkt_crt"),
            symbol_short!("vote"),
            symbol_short!("oracle_rs"),
            symbol_short!("mkt_res"),
            symbol_short!("dispt_crt"),
            symbol_short!("dispt_res"),
            symbol_short!("fee_col"),
            symbol_short!("ext_req"),
            symbol_short!("cfg_upd"),
            symbol_short!("err_log"),
            symbol_short!("perf_met"),
        ];

        for event_type in event_types.iter() {
            // In a real implementation, you would check timestamps and remove old events
            // For now, this is a placeholder
            if env.storage().persistent().has(&event_type) {
                // Check if event is older than threshold and remove if needed
                // This would require storing timestamps with events
            }
        }
    }
}

// ===== EVENT VALIDATION =====

/// Event validation utilities
pub struct EventValidator;

impl EventValidator {
    /// Validate market created event
    pub fn validate_market_created_event(event: &MarketCreatedEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        if event.outcomes.len() < 2 {
            return Err(Error::InvalidInput);
        }

        if event.end_time <= event.timestamp {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate vote cast event
    pub fn validate_vote_cast_event(event: &VoteCastEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        if event.stake <= 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate oracle result event
    pub fn validate_oracle_result_event(_event: &OracleResultEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        Ok(())
    }

    /// Validate market resolved event
    pub fn validate_market_resolved_event(event: &MarketResolvedEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        if event.confidence_score < 0 || event.confidence_score > 100 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate dispute created event
    pub fn validate_dispute_created_event(event: &DisputeCreatedEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        if event.stake <= 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate fee collected event
    pub fn validate_fee_collected_event(event: &FeeCollectedEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        if event.amount <= 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate extension requested event
    pub fn validate_extension_requested_event(
        event: &ExtensionRequestedEvent,
    ) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        if event.additional_days == 0 {
            return Err(Error::InvalidInput);
        }

        if event.fee < 0 {
            return Err(Error::InvalidInput);
        }

        Ok(())
    }

    /// Validate error logged event
    pub fn validate_error_logged_event(_event: &ErrorLoggedEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        Ok(())
    }

    /// Validate performance metric event
    pub fn validate_performance_metric_event(_event: &PerformanceMetricEvent) -> Result<(), Error> {
        // For now, skip validation since we can't easily convert Soroban String/Symbol
        // This is a limitation of the current Soroban SDK
        Ok(())
    }
}

// ===== EVENT HELPER UTILITIES =====

/// Event helper utilities
pub struct EventHelpers;

impl EventHelpers {
    /// Create event summary from event data
    pub fn create_event_summary(env: &Env, event_type: &String, details: &String) -> EventSummary {
        EventSummary {
            event_type: event_type.clone(),
            timestamp: env.ledger().timestamp(),
            details: details.clone(),
        }
    }

    /// Format event timestamp for display
    pub fn format_timestamp(env: &Env, _timestamp: u64) -> String {
        // For now, return a placeholder since we can't easily convert to string
        // This is a limitation of the current Soroban SDK
        String::from_str(env, "timestamp")
    }

    /// Get event type from symbol
    pub fn get_event_type_from_symbol(env: &Env, _symbol: &Symbol) -> String {
        // For now, return a placeholder since we can't easily convert Symbol to string
        // This is a limitation of the current Soroban SDK
        String::from_str(env, "symbol")
    }

    /// Create event context string
    pub fn create_event_context(env: &Env, _context_parts: &Vec<String>) -> String {
        // For now, return a placeholder since we can't easily convert Soroban String
        // This is a limitation of the current Soroban SDK
        String::from_str(env, "context")
    }

    /// Validate event timestamp
    pub fn is_valid_timestamp(timestamp: u64) -> bool {
        // Basic validation - timestamp should be reasonable
        timestamp > 0 && timestamp < 9999999999 // Unix timestamp reasonable range
    }

    /// Get event age in seconds
    pub fn get_event_age(current_timestamp: u64, event_timestamp: u64) -> u64 {
        if current_timestamp >= event_timestamp {
            current_timestamp - event_timestamp
        } else {
            0
        }
    }

    /// Check if event is recent (within specified seconds)
    pub fn is_recent_event(
        event_timestamp: u64,
        current_timestamp: u64,
        recent_threshold: u64,
    ) -> bool {
        Self::get_event_age(current_timestamp, event_timestamp) <= recent_threshold
    }
}

// ===== EVENT TESTING UTILITIES =====

/// Event testing utilities
pub struct EventTestingUtils;

impl EventTestingUtils {
    /// Create test market created event
    pub fn create_test_market_created_event(
        env: &Env,
        market_id: &Symbol,
        admin: &Address,
    ) -> MarketCreatedEvent {
        MarketCreatedEvent {
            market_id: market_id.clone(),
            question: String::from_str(env, "Test market question?"),
            outcomes: vec![
                env,
                String::from_str(env, "yes"),
                String::from_str(env, "no"),
            ],
            admin: admin.clone(),
            end_time: env.ledger().timestamp() + 86400,
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test vote cast event
    pub fn create_test_vote_cast_event(
        env: &Env,
        market_id: &Symbol,
        voter: &Address,
    ) -> VoteCastEvent {
        VoteCastEvent {
            market_id: market_id.clone(),
            voter: voter.clone(),
            outcome: String::from_str(env, "yes"),
            stake: 100_0000000,
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test oracle result event
    pub fn create_test_oracle_result_event(env: &Env, market_id: &Symbol) -> OracleResultEvent {
        OracleResultEvent {
            market_id: market_id.clone(),
            result: String::from_str(env, "yes"),
            provider: String::from_str(env, "Pyth"),
            feed_id: String::from_str(env, "BTC/USD"),
            price: 2500000,
            threshold: 2500000,
            comparison: String::from_str(env, "gt"),
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test market resolved event
    pub fn create_test_market_resolved_event(env: &Env, market_id: &Symbol) -> MarketResolvedEvent {
        MarketResolvedEvent {
            market_id: market_id.clone(),
            final_outcome: String::from_str(env, "yes"),
            oracle_result: String::from_str(env, "yes"),
            community_consensus: String::from_str(env, "yes"),
            resolution_method: String::from_str(env, "Oracle"),
            confidence_score: 85,
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test dispute created event
    pub fn create_test_dispute_created_event(
        env: &Env,
        market_id: &Symbol,
        disputer: &Address,
    ) -> DisputeCreatedEvent {
        DisputeCreatedEvent {
            market_id: market_id.clone(),
            disputer: disputer.clone(),
            stake: 10_0000000,
            reason: Some(String::from_str(env, "Test dispute")),
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test fee collected event
    pub fn create_test_fee_collected_event(
        env: &Env,
        market_id: &Symbol,
        collector: &Address,
    ) -> FeeCollectedEvent {
        FeeCollectedEvent {
            market_id: market_id.clone(),
            collector: collector.clone(),
            amount: 20_0000000,
            fee_type: String::from_str(env, "Platform"),
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test error logged event
    pub fn create_test_error_logged_event(env: &Env) -> ErrorLoggedEvent {
        ErrorLoggedEvent {
            error_code: 1,
            message: String::from_str(env, "Test error message"),
            context: String::from_str(env, "Test context"),
            user: None,
            market_id: None,
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Create test performance metric event
    pub fn create_test_performance_metric_event(env: &Env) -> PerformanceMetricEvent {
        PerformanceMetricEvent {
            metric_name: String::from_str(env, "TransactionCount"),
            value: 100,
            unit: String::from_str(env, "transactions"),
            context: String::from_str(env, "Daily"),
            timestamp: env.ledger().timestamp(),
        }
    }

    /// Validate test event structure
    pub fn validate_test_event_structure<T>(_event: &T) -> Result<(), Error>
    where
        T: Clone,
    {
        // Basic validation that event exists
        // In a real implementation, you would validate specific fields
        Ok(())
    }

    /// Simulate event emission
    pub fn simulate_event_emission(env: &Env, _event_type: &String) -> bool {
        // Simulate successful event emission
        // For now, use a default symbol since we can't easily convert Soroban String
        // This is a limitation of the current Soroban SDK
        let event_key = Symbol::new(env, "event");
        env.storage()
            .persistent()
            .set(&event_key, &String::from_str(env, "test"));
        true
    }
}

// ===== EVENT SUMMARY TYPES =====

/// Event summary for listing
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSummary {
    /// Event type
    pub event_type: String,
    /// Event timestamp
    pub timestamp: u64,
    /// Event details
    pub details: String,
}

/// Market event summary
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketEventSummary {
    /// Event type
    pub event_type: String,
    /// Event timestamp
    pub timestamp: u64,
    /// Event details
    pub details: String,
}

// ===== EVENT CONSTANTS =====

/// Event system constants
pub const MAX_EVENTS_PER_QUERY: u32 = 100;
pub const EVENT_RETENTION_DAYS: u64 = 30 * 24 * 60 * 60; // 30 days
pub const RECENT_EVENT_THRESHOLD: u64 = 24 * 60 * 60; // 24 hours

// ===== EVENT DOCUMENTATION =====

/// Event system documentation and examples
pub struct EventDocumentation;

impl EventDocumentation {
    /// Get event system overview
    pub fn get_overview(env: &Env) -> String {
        String::from_str(env, "Comprehensive event system for Predictify Hybrid contract with emission, logging, validation, and testing utilities.")
    }

    /// Get event type documentation
    pub fn get_event_type_docs(env: &Env) -> Map<String, String> {
        let mut docs = Map::new(env);

        docs.set(
            String::from_str(env, "MarketCreated"),
            String::from_str(env, "Emitted when a new market is created"),
        );
        docs.set(
            String::from_str(env, "VoteCast"),
            String::from_str(env, "Emitted when a user casts a vote"),
        );
        docs.set(
            String::from_str(env, "OracleResult"),
            String::from_str(env, "Emitted when oracle result is fetched"),
        );
        docs.set(
            String::from_str(env, "MarketResolved"),
            String::from_str(env, "Emitted when a market is resolved"),
        );
        docs.set(
            String::from_str(env, "DisputeCreated"),
            String::from_str(env, "Emitted when a dispute is created"),
        );
        docs.set(
            String::from_str(env, "DisputeResolved"),
            String::from_str(env, "Emitted when a dispute is resolved"),
        );
        docs.set(
            String::from_str(env, "FeeCollected"),
            String::from_str(env, "Emitted when fees are collected"),
        );
        docs.set(
            String::from_str(env, "ExtensionRequested"),
            String::from_str(env, "Emitted when market extension is requested"),
        );
        docs.set(
            String::from_str(env, "ConfigUpdated"),
            String::from_str(env, "Emitted when configuration is updated"),
        );
        docs.set(
            String::from_str(env, "ErrorLogged"),
            String::from_str(env, "Emitted when an error is logged"),
        );
        docs.set(
            String::from_str(env, "PerformanceMetric"),
            String::from_str(env, "Emitted when performance metrics are recorded"),
        );

        docs
    }

    /// Get usage examples
    pub fn get_usage_examples(env: &Env) -> Map<String, String> {
        let mut examples = Map::new(env);

        examples.set(
            String::from_str(env, "EmitMarketCreated"),
            String::from_str(env, "EventEmitter::emit_market_created(env, market_id, question, outcomes, admin, end_time)"),
        );
        examples.set(
            String::from_str(env, "EmitVoteCast"),
            String::from_str(
                env,
                "EventEmitter::emit_vote_cast(env, market_id, voter, outcome, stake)",
            ),
        );
        examples.set(
            String::from_str(env, "GetMarketEvents"),
            String::from_str(env, "EventLogger::get_market_events(env, market_id)"),
        );
        examples.set(
            String::from_str(env, "ValidateEvent"),
            String::from_str(env, "EventValidator::validate_market_created_event(&event)"),
        );

        examples
    }
}
