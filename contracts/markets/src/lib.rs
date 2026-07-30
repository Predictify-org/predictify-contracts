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

pub mod errors;

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

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
#[derive(Clone, Debug)]
pub struct BetData {
    /// Index of the selected outcome.
    pub outcome_index: u32,
    /// Amount staked in the platform's base unit.
    pub amount: i128,
}

/// Tracks how much liquidity a user has added to a market.
#[contracttype]
#[derive(Clone, Debug)]
pub struct LiquidityData {
    /// Total amount of liquidity provided.
    pub total_amount: i128,
}

pub mod admin;

#[contract]
pub struct MarketsContract;

#[contractimpl]
impl MarketsContract {
    // -----------------------------------------------------------------------
    //  Read-only
    // -----------------------------------------------------------------------

    /// @notice Returns the contract version.
    /// @dev This is a read-only introspection entrypoint that does **not** require authentication.
    /// @param _env The contract environment.
    /// @return The version number as u32.
    pub fn version(_env: Env) -> u32 {
        7
    }

    /// @notice Read a market from persistent storage and bump its TTL.
    /// @param env The contract environment.
    /// @param market_id The unique symbol identifying the market.
    /// @return An optional value containing the market data if it exists.
    pub fn get_market(env: Env, market_id: soroban_sdk::Symbol) -> Option<soroban_sdk::Val> {
        let market: Option<soroban_sdk::Val> = env.storage().persistent().get(&market_id);
        if market.is_some() {
            // Bump TTL: 365 days * 17280 ledgers per day = 6307200
            env.storage().persistent().extend_ttl(&market_id, 6307200, 6307200);
        }
        market
    }

    /// @notice Whether the markets subsystem is currently paused.
    /// @dev Read-only; defaults to `false` (not paused) before `pause_markets` is ever called.
    pub fn is_paused(env: Env) -> bool {
        env.storage().instance().get(&DataKey::Paused).unwrap_or(false)
    }

    // -----------------------------------------------------------------------
    //  Admin
    // -----------------------------------------------------------------------

    /// @notice Set the admin address once. Required before `pause_markets`/`unpause_markets`.
    /// @dev Fails with `AlreadyInitialized`-equivalent (`InvalidState`) if the admin is already set.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(ContractError::InvalidState);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        Ok(())
    }

    /// @notice Pause the markets subsystem. Admin-only.
    /// @dev Once paused, other entrypoints are expected to check [`Self::is_paused`] and
    /// reject state-changing calls with `MarketClosed` while active.
    pub fn pause_markets(env: Env, admin: Address) -> Result<(), ContractError> {
        Self::require_admin(&env, &admin)?;
        env.storage().instance().set(&DataKey::Paused, &true);
        Ok(())
    }

    /// @notice Resume the markets subsystem after a pause. Admin-only.
    pub fn unpause_markets(env: Env, admin: Address) -> Result<(), ContractError> {
        Self::require_admin(&env, &admin)?;
        env.storage().instance().set(&DataKey::Paused, &false);
        Ok(())
    }

    fn require_admin(env: &Env, caller: &Address) -> Result<(), ContractError> {
        caller.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(ContractError::InvalidState)?;
        if *caller != stored_admin {
            return Err(ContractError::Unauthorized);
        }
        Ok(())
    }
}
