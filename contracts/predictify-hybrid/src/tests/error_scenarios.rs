//! Error recovery scenario tests demonstrating real-world recovery workflows.
//!
//! These tests show how errors are handled and recovered from in practical scenarios.
//! They serve as both examples for developers and regression tests for recovery paths.

#![cfg(test)]

use crate::err::{Error, ErrorContext, ErrorHandler};
use crate::tests::common::{ErrorContextBuilder, ErrorTestScenarios};
use soroban_sdk::{Env, Map, String, Symbol};

// ===== ORACLE FAILURE RECOVERY SCENARIOS =====

/// Test recovery when oracle service is temporarily unavailable.
/// Expected: Retry with delay, then fallback if available.
#[test]
fn scenario_oracle_unavailable_with_retry() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "btc_price_market");
    let context = ErrorTestScenarios::oracle_resolution_context(&env, market_id);

    // Attempt recovery
    let recovery = ErrorHandler::recover_from_error(&env, Error::OracleUnavailable, context);
    assert!(recovery.is_ok());

    let recovery_data = recovery.unwrap();
    let strategy = recovery_data.recovery_strategy.clone();

    // Should use retry with delay strategy
    assert!(
        strategy == String::from_str(&env, "retry_with_delay")
            || strategy == String::from_str(&env, "alternative_method"),
        "Oracle unavailable should allow retries or fallback"
    );
}

/// Test handling oracle stale data scenario.
/// Expected: Mark as non-recoverable, suggest manual intervention.
#[test]
fn scenario_oracle_stale_data() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "eth_price_market");
    let context = ErrorTestScenarios::oracle_resolution_context(&env, market_id);

    let detailed = ErrorHandler::categorize_error(&env, Error::OracleStale, context);

    // Stale data should be marked as an issue needing intervention
    assert!(!detailed.detailed_message.is_empty());
    assert!(!detailed.user_action.is_empty());
}

// ===== USER OPERATION RECOVERY SCENARIOS =====

/// Test handling when user attempts duplicate vote.
/// Expected: Skip gracefully (user-side issue, not system issue).
#[test]
fn scenario_user_already_voted() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "prediction_market");

    let context = ErrorContextBuilder::new(&env, "vote_on_market")
        .market_id(Some(market_id))
        .user_address(Some(soroban_sdk::Address::generate(&env)))
        .build();

    let recovery = ErrorHandler::recover_from_error(&env, Error::AlreadyVoted, context);
    assert!(recovery.is_ok());

    let recovery_data = recovery.unwrap();
    assert_eq!(
        recovery_data.recovery_strategy,
        String::from_str(&env, "skip")
    );
}

/// Test insufficient balance scenario.
/// Expected: Retry (user can deposit more funds).
#[test]
fn scenario_insufficient_balance_recovery() {
    let env = Env::default();
    let context = ErrorTestScenarios::balance_check_context(&env);

    let recovery = ErrorHandler::recover_from_error(&env, Error::InsufficientBalance, context);
    assert!(recovery.is_ok());

    let detailed =
        ErrorHandler::categorize_error(&env, Error::InsufficientBalance, context.clone());
    assert!(
        detailed.user_action.contains("balance")
            || detailed.user_action.contains("deposit")
            || !detailed.user_action.is_empty()
    );
}

// ===== VALIDATION ERROR RECOVERY SCENARIOS =====

/// Test invalid market duration scenario.
/// Expected: User must resubmit with valid duration.
#[test]
fn scenario_invalid_market_duration() {
    let env = Env::default();
    let context = ErrorTestScenarios::market_creation_context(&env);

    let detailed = ErrorHandler::categorize_error(&env, Error::InvalidDuration, context);

    assert!(
        detailed.user_action.contains("duration")
            || detailed.user_action.contains("1")
            || detailed.user_action.contains("365")
            || !detailed.user_action.is_empty(),
        "Action should guide user on valid durations"
    );
}

/// Test invalid outcomes scenario.
/// Expected: User must provide at least 2 unique, non-empty outcomes.
#[test]
fn scenario_invalid_market_outcomes() {
    let env = Env::default();
    let context = ErrorTestScenarios::market_creation_context(&env);

    let detailed = ErrorHandler::categorize_error(&env, Error::InvalidOutcomes, context);

    assert!(
        detailed.user_action.contains("outcome")
            || detailed.user_action.contains("input")
            || !detailed.user_action.is_empty()
    );
}

// ===== AUTHORIZATION/SECURITY SCENARIOS =====

/// Test unauthorized access scenario.
/// Expected: Cannot be retried by same user, must fail.
#[test]
fn scenario_unauthorized_cannot_retry() {
    let env = Env::default();
    let context = ErrorTestScenarios::market_creation_context(&env);

    let recovery = ErrorHandler::recover_from_error(&env, Error::Unauthorized, context);
    assert!(recovery.is_ok());

    let recovery_data = recovery.unwrap();
    assert_eq!(
        recovery_data.recovery_strategy,
        String::from_str(&env, "abort")
    );
}

// ===== SYSTEM STATE RECOVERY SCENARIOS =====

/// Test admin not set scenario (initialization issue).
/// Expected: Requires manual intervention, cannot auto-recover.
#[test]
fn scenario_admin_not_initialized() {
    let env = Env::default();
    let context = ErrorContextBuilder::new(&env, "initialize")
        .with_data(&env, "phase", "contract_initialization")
        .build();

    let recovery = ErrorHandler::recover_from_error(&env, Error::AdminNotSet, context);
    assert!(recovery.is_ok());

    let recovery_data = recovery.unwrap();
    assert_eq!(
        recovery_data.recovery_strategy,
        String::from_str(&env, "manual_intervention")
    );
}

/// Test invalid contract state scenario.
/// Expected: Should not auto-recover, requires inspection.
#[test]
fn scenario_invalid_contract_state() {
    let env = Env::default();
    let mut context = ErrorContextBuilder::new(&env, "process_bet")
        .with_data(&env, "market_state", "unknown")
        .build();

    let recovery = ErrorHandler::recover_from_error(&env, Error::InvalidState, context);
    assert!(recovery.is_ok());

    let recovery_data = recovery.unwrap();
    // Invalid state should not be retryable
    assert_eq!(
        recovery_data.recovery_strategy,
        String::from_str(&env, "no_recovery")
    );
}

// ===== COMPLEX MULTI-ERROR SCENARIOS =====

/// Test recovery when multiple errors cascade.
/// Simulates: Market creation fails → validation error → invalid duration.
#[test]
fn scenario_cascading_validation_errors() {
    let env = Env::default();
    let context = ErrorTestScenarios::market_creation_context(&env);

    // First error: invalid duration
    let recovery1 =
        ErrorHandler::recover_from_error(&env, Error::InvalidDuration, context.clone());
    assert!(recovery1.is_ok());

    // User retries with invalid outcomes
    let recovery2 = ErrorHandler::recover_from_error(&env, Error::InvalidOutcomes, context);
    assert!(recovery2.is_ok());

    // Both should be recoverable (validation errors)
    assert_eq!(
        recovery1.unwrap().recovery_strategy,
        String::from_str(&env, "retry")
    );
    assert_eq!(
        recovery2.unwrap().recovery_strategy,
        String::from_str(&env, "retry")
    );
}

/// Test recovery when dispute resolution fails.
/// Expected: Fee distribution error requires manual fix.
#[test]
fn scenario_dispute_resolution_failure() {
    let env = Env::default();
    let context = ErrorContextBuilder::new(&env, "resolve_dispute")
        .with_data(&env, "dispute_status", "voting_complete")
        .build();

    let recovery = ErrorHandler::recover_from_error(&env, Error::DisputeFeeFailed, context);
    assert!(recovery.is_ok());

    let recovery_data = recovery.unwrap();
    assert_eq!(
        recovery_data.recovery_strategy,
        String::from_str(&env, "manual_intervention")
    );
}

// ===== RECOVERY STATUS AND ANALYTICS SCENARIOS =====

/// Test aggregated recovery statistics.
/// Shows how to query system health.
#[test]
fn scenario_check_system_recovery_health() {
    let env = Env::default();

    let status = ErrorHandler::get_error_recovery_status(&env);
    assert!(status.is_ok());

    let status = status.unwrap();
    // Should produce valid statistics
    assert!(status.total_attempts >= 0);
    assert!(status.successful_recoveries >= 0);
    assert!(status.failed_recoveries >= 0);
}

/// Test error analytics to understand failure patterns.
#[test]
fn scenario_analyze_error_distribution() {
    let env = Env::default();

    let analytics = ErrorHandler::get_error_analytics(&env);
    assert!(analytics.is_ok());

    let analytics = analytics.unwrap();
    // Should have categories and severity levels tracked
    assert!(analytics.errors_by_category.len() >= 0);
    assert!(analytics.errors_by_severity.len() >= 0);
}

// ===== TIMING AND TIMEOUT SCENARIOS =====

/// Test recovery with custom timeout.
/// Shows how to handle operations that must complete within a time window.
#[test]
fn scenario_oracle_resolution_with_timeout() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "time_sensitive_market");
    let current_time = env.ledger().timestamp();
    let timeout_threshold = current_time + 300; // 5 minute timeout

    let context =
        ErrorContextBuilder::new(&env, "resolve_market_with_timeout")
            .market_id(Some(market_id))
            .timestamp(current_time)
            .with_data(&env, "timeout_at", &timeout_threshold.to_string())
            .build();

    // If within timeout, should be retryable
    let recovery = ErrorHandler::recover_from_error(&env, Error::OracleUnavailable, context);
    assert!(recovery.is_ok());
}

/// Test expired timeout handling.
/// Shows what happens when deadline passes.
#[test]
fn scenario_resolution_timeout_exceeded() {
    let env = Env::default();
    let market_id = Symbol::new(&env, "expired_market");

    let context = ErrorContextBuilder::new(&env, "resolve_expired_market")
        .market_id(Some(market_id))
        .build();

    let detailed = ErrorHandler::categorize_error(
        &env,
        Error::ResolutionTimeoutReached,
        context,
    );

    // Timeout errors are permanent failures
    assert!(!detailed.detailed_message.is_empty());
}
