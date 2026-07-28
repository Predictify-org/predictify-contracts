#![no_std]

//! Markets contract with auth-gated entrypoints.
//!
//! Provides a prediction-market subsystem where every state-changing
//! entrypoint enforces `require_auth` on the acting [`Address`].
//!
//! # Auth Matrix
//!
//! | Function               | Required Role              |
//! |------------------------|----------------------------|
//! | `create_market`        | Creator (any address)      |
//! | `place_bet`            | Bettor (any address)       |
//! | `resolve_market`       | Market creator             |
//! | `claim_winnings`       | Winner (bettor)            |
//! | `cancel_market`        | Market creator             |
//! | `withdraw_funds`       | Market creator             |
//! | `update_market_params` | Market creator             |
//! | `add_liquidity`        | Liquidity provider         |
//! | `remove_liquidity`     | Liquidity provider         |
//! | `pause_markets`        | Admin                      |
//! | `unpause_markets`      | Admin                      |
//! | `transfer_ownership`   | Admin                      |
//! | `version`              | Anyone (read-only)         |

mod errors;

use soroban_sdk::{contract, contractimpl, contracttype, panic_with_error, Address, Env, String, Vec};

pub use errors::ContractError;

/// Type alias used by generated client code and test harnesses.
pub type Error = ContractError;

/// Persistent-storage keys used by the Markets contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Sequential counter for generating unique market IDs.
    MarketCounter,
    /// Market data keyed by the numeric market ID.
    Market(u32),
    /// Bet placed by a user on a specific market.
    Bet(u32, Address),
    /// Whether the markets subsystem is globally paused (`true` = paused).
    Paused,
    /// The admin / owner address.
    Admin,
    /// Liquidity provided by a user to a specific market.
    Liquidity(u32, Address),
}

/// On-chain representation of a prediction market.
#[contracttype]
#[derive(Clone)]
pub struct MarketData {
    /// The address that created the market.
    pub creator: Address,
    /// The prediction question.
    pub question: String,
    /// A human-readable description of the market.
    pub description: String,
    /// Unix timestamp (seconds) when betting closes.
    pub end_time: u64,
    /// Identifies the data source used at resolution time.
    pub resolution_source: String,
    /// Ordered list of possible outcomes.
    pub outcome_tags: Vec<String>,
    /// Whether a winning outcome has been recorded.
    pub resolved: bool,
    /// Index (0-based) of the winning outcome.
    pub winning_outcome: u32,
}

/// On-chain record of a user's bet on a market.
#[contracttype]
#[derive(Clone)]
pub struct BetData {
    /// Index of the selected outcome.
    pub outcome_index: u32,
    /// Amount staked in the platform's base unit.
    pub amount: i128,
}

/// Tracks how much liquidity a user has added to a market.
#[contracttype]
#[derive(Clone)]
pub struct LiquidityData {
    /// Total amount of liquidity provided.
    pub total_amount: i128,
}

pub mod errors;
pub mod admin;

#[contract]
pub struct MarketsContract;

#[contractimpl]
impl MarketsContract {
    // -----------------------------------------------------------------------
    //  Read-only
    // -----------------------------------------------------------------------

    /// Returns the contract version.
    ///
    /// This is a read-only introspection entrypoint that does **not** require
    /// authentication.
    pub fn version(_env: Env) -> u32 {
        7
    }

    /// Read a market from persistent storage and bump its TTL.
    pub fn get_market(env: Env, market_id: soroban_sdk::Symbol) -> Option<soroban_sdk::Val> {
        let market: Option<soroban_sdk::Val> = env.storage().persistent().get(&market_id);
        if market.is_some() {
            // Bump TTL: 365 days * 17280 ledgers per day = 6307200
            env.storage().persistent().extend_ttl(&market_id, 6307200, 6307200);
        }
        market
    }

}
