//! Common test utilities and helpers for error testing.
//!
//! This module provides shared test fixtures, context builders, and assertion helpers
//! to reduce duplication across error-related tests.

#![cfg(test)]

use crate::err::{Error, ErrorContext};
use soroban_sdk::{Address, Env, Map, String, Symbol};

/// Test environment factory with commonly used defaults.
pub struct TestEnv;

impl TestEnv {
    /// Creates a new test environment.
    pub fn new() -> Env {
        Env::default()
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        TestEnv
    }
}

/// Builder for constructing `ErrorContext` with flexible options.
///
/// # Example
///
/// ```rust,ignore
/// let context = ErrorContextBuilder::new(&env)
///     .operation("place_bet")
///     .user_address(Some(user_addr))
///     .market_id(Some(market_id))
///     .build();
/// ```
pub struct ErrorContextBuilder {
    operation: String,
    user_address: Option<Address>,
    market_id: Option<Symbol>,
    context_data: Map<String, String>,
    timestamp: u64,
    call_chain: Option<Vec<String>>,
}

impl ErrorContextBuilder {
    /// Creates a new builder with a required operation name.
    pub fn new(env: &Env, operation: &str) -> Self {
        ErrorContextBuilder {
            operation: String::from_str(env, operation),
            user_address: None,
            market_id: None,
            context_data: Map::new(env),
            timestamp: env.ledger().timestamp(),
            call_chain: None,
        }
    }

    /// Sets the user address in the context.
    pub fn user_address(mut self, addr: Option<Address>) -> Self {
        self.user_address = addr;
        self
    }

    /// Sets the market ID in the context.
    pub fn market_id(mut self, id: Option<Symbol>) -> Self {
        self.market_id = id;
        self
    }

    /// Sets a custom timestamp (useful for testing timeout logic).
    pub fn timestamp(mut self, ts: u64) -> Self {
        self.timestamp = ts;
        self
    }

    /// Adds a key-value pair to context data.
    pub fn with_data(mut self, env: &Env, key: &str, value: &str) -> Self {
        self.context_data.set(
            String::from_str(env, key),
            String::from_str(env, value),
        );
        self
    }

    /// Sets the call chain (useful for debugging nested calls).
    pub fn call_chain(mut self, chain: Option<Vec<String>>) -> Self {
        self.call_chain = chain;
        self
    }

    /// Builds the final `ErrorContext`.
    pub fn build(self) -> ErrorContext {
        ErrorContext {
            operation: self.operation,
            user_address: self.user_address,
            market_id: self.market_id,
            context_data: self.context_data,
            timestamp: self.timestamp,
            call_chain: self.call_chain,
        }
    }
}

/// Helper for creating common test scenarios.
pub struct ErrorTestScenarios;

impl ErrorTestScenarios {
    /// Creates a context for a market creation failure.
    pub fn market_creation_context(env: &Env) -> ErrorContext {
        ErrorContextBuilder::new(env, "create_market")
            .user_address(Some(Address::generate(env)))
            .build()
    }

    /// Creates a context for a bet placement failure.
    pub fn bet_placement_context(env: &Env, market_id: Symbol) -> ErrorContext {
        ErrorContextBuilder::new(env, "place_bet")
            .user_address(Some(Address::generate(env)))
            .market_id(Some(market_id))
            .build()
    }

    /// Creates a context for an oracle resolution failure.
    pub fn oracle_resolution_context(env: &Env, market_id: Symbol) -> ErrorContext {
        ErrorContextBuilder::new(env, "resolve_market")
            .user_address(Some(Address::generate(env)))
            .market_id(Some(market_id))
            .with_data(env, "oracle_contract", "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABSC4")
            .build()
    }

    /// Creates a context for a balance check failure.
    pub fn balance_check_context(env: &Env) -> ErrorContext {
        ErrorContextBuilder::new(env, "check_balance")
            .user_address(Some(Address::generate(env)))
            .build()
    }

    /// Creates a context for a withdrawal failure.
    pub fn withdrawal_context(env: &Env) -> ErrorContext {
        ErrorContextBuilder::new(env, "withdraw_funds")
            .user_address(Some(Address::generate(env)))
            .build()
    }
}

/// Test assertions for error properties.
pub struct ErrorAssertions;

impl ErrorAssertions {
    /// Asserts that an error code is within a specific range.
    ///
    /// # Panics
    ///
    /// Panics if the error code is outside the specified range.
    pub fn assert_error_in_range(error: Error, min: u32, max: u32) {
        let code = error as u32;
        assert!(
            code >= min && code <= max,
            "Error code {} is outside range [{}, {}]",
            code,
            min,
            max
        );
    }

    /// Asserts that an error code string is valid (uppercase, underscores allowed).
    ///
    /// # Panics
    ///
    /// Panics if the code contains invalid characters.
    pub fn assert_error_code_format(code: &str) {
        for c in code.chars() {
            assert!(
                c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_',
                "Invalid character '{}' in error code '{}'",
                c,
                code
            );
        }
        assert!(
            !code.starts_with('_') && !code.ends_with('_'),
            "Error code '{}' has invalid leading/trailing underscores",
            code
        );
    }

    /// Asserts that all error descriptions are non-empty.
    pub fn assert_all_descriptions_non_empty(errors: &[Error]) {
        for error in errors {
            assert!(
                !error.description().is_empty(),
                "Error {:?} has empty description",
                error
            );
        }
    }

    /// Asserts that all error code strings are unique within a set.
    pub fn assert_error_codes_unique(errors: &[Error]) {
        let mut codes = std::collections::HashSet::new();
        for error in errors {
            let code = error.code();
            assert!(
                codes.insert(code),
                "Duplicate error code: {}",
                code
            );
        }
    }
}

/// Test fixture providing a standard set of errors for batch testing.
pub struct ErrorTestSuite;

impl ErrorTestSuite {
    /// Returns all user operation errors (100-112 range).
    pub fn user_operation_errors() -> Vec<Error> {
        vec![
            Error::Unauthorized,
            Error::MarketNotFound,
            Error::MarketClosed,
            Error::MarketResolved,
            Error::MarketNotResolved,
            Error::NothingToClaim,
            Error::AlreadyClaimed,
            Error::InsufficientStake,
            Error::InvalidOutcome,
            Error::AlreadyVoted,
            Error::AlreadyBet,
            Error::BetsAlreadyPlaced,
            Error::InsufficientBalance,
        ]
    }

    /// Returns all oracle errors (200-208 range).
    pub fn oracle_errors() -> Vec<Error> {
        vec![
            Error::OracleUnavailable,
            Error::InvalidOracleConfig,
            Error::OracleStale,
            Error::OracleNoConsensus,
            Error::OracleVerified,
            Error::MarketNotReady,
            Error::FallbackOracleUnavailable,
            Error::ResolutionTimeoutReached,
            Error::OracleConfidenceTooWide,
        ]
    }

    /// Returns all validation errors (300-304 range).
    pub fn validation_errors() -> Vec<Error> {
        vec![
            Error::InvalidQuestion,
            Error::InvalidOutcomes,
            Error::InvalidDuration,
            Error::InvalidThreshold,
            Error::InvalidComparison,
        ]
    }

    /// Returns all general/system errors (400-418 range).
    pub fn system_errors() -> Vec<Error> {
        vec![
            Error::InvalidState,
            Error::InvalidInput,
            Error::InvalidFeeConfig,
            Error::ConfigNotFound,
            Error::AlreadyDisputed,
            Error::DisputeVoteExpired,
            Error::DisputeVoteDenied,
            Error::DisputeAlreadyVoted,
            Error::DisputeCondNotMet,
            Error::DisputeFeeFailed,
            Error::DisputeError,
            Error::FeeAlreadyCollected,
            Error::NoFeesToCollect,
            Error::InvalidExtensionDays,
            Error::ExtensionDenied,
            Error::AdminNotSet,
        ]
    }

    /// Returns all circuit breaker errors (500-504 range).
    pub fn circuit_breaker_errors() -> Vec<Error> {
        vec![
            Error::CBNotInitialized,
            Error::CBAlreadyOpen,
            Error::CBNotOpen,
            Error::CBOpen,
            Error::CBError,
        ]
    }

    /// Returns all error variants (for exhaustive testing).
    pub fn all_errors() -> Vec<Error> {
        let mut all = Self::user_operation_errors();
        all.extend(Self::oracle_errors());
        all.extend(Self::validation_errors());
        all.extend(Self::system_errors());
        all.extend(Self::circuit_breaker_errors());
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_builder_simple() {
        let env = Env::default();
        let context = ErrorContextBuilder::new(&env, "test_op").build();
        assert!(!context.operation.is_empty());
    }

    #[test]
    fn test_error_context_builder_full() {
        let env = Env::default();
        let user = Address::generate(&env);
        let market = Symbol::new(&env, "test_market");

        let context = ErrorContextBuilder::new(&env, "test_op")
            .user_address(Some(user.clone()))
            .market_id(Some(market.clone()))
            .with_data(&env, "key1", "value1")
            .build();

        assert_eq!(context.user_address, Some(user));
        assert_eq!(context.market_id, Some(market));
    }

    #[test]
    fn test_error_test_suite_all_errors() {
        let all = ErrorTestSuite::all_errors();
        assert!(!all.is_empty());
        // Should have at least representatives from each category
        assert!(ErrorTestSuite::user_operation_errors().len() > 0);
        assert!(ErrorTestSuite::oracle_errors().len() > 0);
        assert!(ErrorTestSuite::validation_errors().len() > 0);
        assert!(ErrorTestSuite::system_errors().len() > 0);
        assert!(ErrorTestSuite::circuit_breaker_errors().len() > 0);
    }
}
