use soroban_sdk::contracterror;

/// Essential error enum for the Predictify Hybrid contract
#[contracterror]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    // Core errors
    Unauthorized = 1,
    MarketClosed = 2,
    MarketNotFound = 3,
    InsufficientStake = 4,
    InvalidOutcome = 5,
    AlreadyClaimed = 6,
    
    // Essential market errors
    MarketAlreadyResolved = 7,
    NothingToClaim = 8,
    AlreadyVoted = 9,
    AlreadyDisputed = 10,
    OracleUnavailable = 11,
    
    // Reentrancy errors  
    ReentrancyAttack = 101,
    InvalidReentrancyState = 102,
    InconsistentReentrancyState = 103,
    InvalidCallState = 104,
    CallStackOverflow = 105,
    
    // Essential validation errors
    InvalidInput = 201,
    InvalidConfig = 202,
    MarketNotResolved = 203,
    InvalidThreshold = 204,
    InvalidState = 205,
    
    // Essential Oracle errors
    OracleDataStale = 301,
    OraclePriceOutOfRange = 302,
    InvalidOracleFeed = 303,
    
    // Essential system errors  
    ConfigurationNotFound = 401,
    NoFeesToCollect = 402,
    FeeAlreadyCollected = 403,
    InternalError = 500,
}

/// Error helper functions for common scenarios
pub mod helpers {
    use super::Error;
    use soroban_sdk::{panic_with_error, String, Env, Vec, Address};

    /// Validate that the caller is the admin
    pub fn require_admin(
        env: &Env,
        caller: &Address,
        admin: &Address,
    ) -> Result<(), Error> {
        if caller != admin {
            panic_with_error!(env, Error::Unauthorized);
        }
        Ok(())
    }

    /// Validate that the market exists and is not closed
    pub fn require_market_open(env: &Env, market: &Option<crate::Market>) -> Result<(), Error> {
        match market {
            Some(market) => {
                if env.ledger().timestamp() >= market.end_time {
                    panic_with_error!(env, Error::MarketClosed);
                }
                Ok(())
            }
            None => {
                panic_with_error!(env, Error::MarketNotFound);
            }
        }
    }

    /// Validate that the market is resolved
    pub fn require_market_resolved(env: &Env, market: &Option<crate::Market>) -> Result<(), Error> {
        match market {
            Some(market) => {
                if market.winning_outcome.is_none() {
                    panic_with_error!(env, Error::MarketNotResolved);
                }
                Ok(())
            }
            None => {
                panic_with_error!(env, Error::MarketNotFound);
            }
        }
    }

    /// Validate that the outcome is valid for the market
    pub fn require_valid_outcome(
        env: &Env,
        outcome: &String,
        outcomes: &Vec<String>,
    ) -> Result<(), Error> {
        if !outcomes.contains(outcome) {
            panic_with_error!(env, Error::InvalidOutcome);
        }
        Ok(())
    }

    /// Validate that the stake amount is sufficient
    pub fn require_sufficient_stake(env: &Env, stake: i128, min_stake: i128) -> Result<(), Error> {
        if stake < min_stake {
            panic_with_error!(env, Error::InsufficientStake);
        }
        Ok(())
    }

    /// Validate that the user hasn't already claimed
    pub fn require_not_claimed(env: &Env, claimed: bool) -> Result<(), Error> {
        if claimed {
            panic_with_error!(env, Error::AlreadyClaimed);
        }
        Ok(())
    }

    /// Validate oracle configuration
    pub fn require_valid_oracle_config(
        env: &Env,
        config: &crate::OracleConfig,
    ) -> Result<(), Error> {
        if config.threshold <= 0 {
            panic_with_error!(env, Error::InvalidConfig);
        }

        if config.comparison != String::from_str(env, "gt")
            && config.comparison != String::from_str(env, "lt")
            && config.comparison != String::from_str(env, "eq")
        {
            panic_with_error!(env, Error::InvalidConfig);
        }

        Ok(())
    }

    /// Validate market creation parameters
    pub fn require_valid_market_params(
        env: &Env,
        question: &String,
        outcomes: &Vec<String>,
        duration_days: u32,
    ) -> Result<(), Error> {
        if question.is_empty() {
            panic_with_error!(env, Error::InvalidInput);
        }

        if outcomes.len() < 2 {
            panic_with_error!(env, Error::InvalidInput);
        }

        if duration_days == 0 || duration_days > 365 {
            panic_with_error!(env, Error::InvalidInput);
        }

        Ok(())
    }
}
