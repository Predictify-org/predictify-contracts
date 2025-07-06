
#![allow(dead_code)]

use soroban_sdk::{contracttype, Address, String, Map, Vec, Symbol, Env};

// ===== ORACLE TYPES =====

/// Oracle provider enumeration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleProvider {
    /// Reflector oracle (primary oracle for Stellar Network)
    Reflector,
    /// Pyth Network oracle (placeholder for Stellar)
    Pyth,
}

impl OracleProvider {
    /// Get provider name
    pub fn name(&self) -> &'static str {
        match self {
            OracleProvider::Reflector => "Reflector",
            OracleProvider::Pyth => "Pyth",
        }
    }

    /// Check if provider is supported on Stellar
    pub fn is_supported(&self) -> bool {
        matches!(self, OracleProvider::Reflector)
    }
}

/// Oracle configuration for markets
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    /// The oracle provider to use
    pub provider: OracleProvider,
    /// Oracle-specific identifier (e.g., "BTC/USD" for Pyth, "BTC" for Reflector)
    pub feed_id: String,
    /// Price threshold in cents (e.g., 10_000_00 = $10k)
    pub threshold: i128,
    /// Comparison operator: "gt", "lt", "eq"
    pub comparison: String,
}

impl OracleConfig {
    /// Create a new oracle configuration
    pub fn new(
        provider: OracleProvider,
        feed_id: String,
        threshold: i128,
        comparison: String,
    ) -> Self {
        Self {
            provider,
            feed_id,
            threshold,
            comparison,
        }
    }

    /// Validate the oracle configuration
    pub fn validate(&self, env: &Env) -> Result<(), crate::Error> {
        // Validate threshold
        if self.threshold <= 0 {
            return Err(crate::Error::InvalidThreshold);
        }

        // Validate comparison operator
        if self.comparison != String::from_str(env, "gt")
            && self.comparison != String::from_str(env, "lt")
            && self.comparison != String::from_str(env, "eq")
        {
            return Err(crate::Error::InvalidComparison);
        }

        // Validate provider is supported
        if !self.provider.is_supported() {
            return Err(crate::Error::InvalidOracleConfig);
        }

        Ok(())
    }
}

// ===== MARKET TYPES =====

/// Market state and data structure
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Market {
    /// Market administrator address
    pub admin: Address,
    /// Market question/prediction
    pub question: String,
    /// Available outcomes for the market
    pub outcomes: Vec<String>,
    /// Market end time (Unix timestamp)
    pub end_time: u64,
    /// Oracle configuration for this market
    pub oracle_config: OracleConfig,
    /// Oracle result (set after market ends)
    pub oracle_result: Option<String>,
    /// User votes mapping (address -> outcome)
    pub votes: Map<Address, String>,
    /// User stakes mapping (address -> stake amount)
    pub stakes: Map<Address, i128>,
    /// Claimed status mapping (address -> claimed)
    pub claimed: Map<Address, bool>,
    /// Total amount staked in the market
    pub total_staked: i128,
    /// Dispute stakes mapping (address -> dispute stake)
    pub dispute_stakes: Map<Address, i128>,
    /// Winning outcome (set after resolution)
    pub winning_outcome: Option<String>,
    /// Whether fees have been collected
    pub fee_collected: bool,
}

impl Market {
    /// Create a new market
    pub fn new(
        env: &Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        end_time: u64,
        oracle_config: OracleConfig,
    ) -> Self {
        Self {
            admin,
            question,
            outcomes,
            end_time,
            oracle_config,
            oracle_result: None,
            votes: Map::new(env),
            stakes: Map::new(env),
            claimed: Map::new(env),
            total_staked: 0,
            dispute_stakes: Map::new(env),
            winning_outcome: None,
            fee_collected: false,
        }
    }

    /// Check if the market is active (not ended)
    pub fn is_active(&self, current_time: u64) -> bool {
        current_time < self.end_time
    }

    /// Check if the market has ended
    pub fn has_ended(&self, current_time: u64) -> bool {
        current_time >= self.end_time
    }

    /// Check if the market is resolved
    pub fn is_resolved(&self) -> bool {
        self.winning_outcome.is_some()
    }

    /// Validate market parameters
    pub fn validate(&self, env: &Env) -> Result<(), crate::Error> {
        // Validate question
        if self.question.is_empty() {
            return Err(crate::Error::InvalidQuestion);
        }

        // Validate outcomes
        if self.outcomes.len() < 2 {
            return Err(crate::Error::InvalidOutcomes);
        }

        // Validate oracle config
        self.oracle_config.validate(env)?;

        // Validate end time
        if self.end_time <= env.ledger().timestamp() {
            return Err(crate::Error::InvalidDuration);
        }

        Ok(())
    }
}
