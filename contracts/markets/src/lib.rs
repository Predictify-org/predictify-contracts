#![no_std]

//! # Markets contract (gas-snapshot harness, v7)
//!
//! Focused Soroban contract exposing the core **market lifecycle** entrypoints
//! used for per-call CPU / memory regression baselines.
//!
//! | Entrypoint         | Auth  | Mutates state |
//! |--------------------|-------|---------------|
//! | [`initialize`]     | admin | yes           |
//! | [`create_market`]  | admin | yes           |
//! | [`vote`]           | user  | yes           |
//! | [`resolve_market`] | admin | yes           |
//! | [`claim_winnings`] | user  | yes           |
//! | [`get_market`]     | none  | no            |
//! | [`get_stake`]      | none  | no            |
//! | [`gas_snap_version`] | none | no          |
//!
//! Integration gas baselines: `tests/gas_snap.rs`.
//!
//! ## Security
//!
//! - Every state-changing entrypoint calls [`Address::require_auth`].
//! - Arithmetic uses `checked_*`; production paths do not call `unwrap()`.
//! - Snapshot schema version is [`GAS_SNAP_VERSION`].

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, Env, String, Symbol, Vec,
};

/// Schema version for the markets gas snapshot suite (v7).
pub const GAS_SNAP_VERSION: u32 = 7;

/// Maximum number of outcomes a single market may declare.
pub const MAX_OUTCOMES: u32 = 8;

/// Minimum stake accepted by [`MarketsContract::vote`].
pub const MIN_STAKE: i128 = 1;

// ---------------------------------------------------------------------------
// Storage
// ---------------------------------------------------------------------------

#[contracttype]
#[derive(Clone)]
enum DataKey {
    Admin,
    Initialized,
    MarketCount,
    Market(Symbol),
    Stake(Symbol, Address),
    VoteOutcome(Symbol, Address),
}

/// On-chain market record.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Market {
    /// Human-readable question.
    pub question: String,
    /// Allowed outcomes (length 2..=[`MAX_OUTCOMES`]).
    pub outcomes: Vec<String>,
    /// Ledger timestamp after which voting is closed.
    pub end_time: u64,
    /// Aggregate stake across all voters.
    pub total_staked: i128,
    /// Winning outcome once resolved; empty while open.
    pub winning_outcome: String,
    /// Whether the market has been resolved.
    pub resolved: bool,
}

/// Client-facing markets error codes (stable numeric assignments).
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Contract has already been initialized.
    AlreadyInitialized = 1,
    /// Contract has not been initialized.
    NotInitialized = 2,
    /// Caller is not the stored admin.
    Unauthorized = 3,
    /// Market id is unknown.
    MarketNotFound = 4,
    /// Outcomes list is empty, too short, or too long.
    InvalidOutcomes = 5,
    /// Outcome string is not in the market's outcome set.
    InvalidOutcome = 6,
    /// Stake amount is non-positive.
    InvalidStake = 7,
    /// Voting window has closed.
    MarketClosed = 8,
    /// Market is already resolved.
    AlreadyResolved = 9,
    /// Market is not yet resolved.
    NotResolved = 10,
    /// Duration must be strictly positive.
    InvalidDuration = 11,
    /// Caller has no stake to claim.
    NothingToClaim = 12,
    /// Arithmetic overflow/underflow.
    Overflow = 13,
    /// Market end time has not been reached.
    MarketStillOpen = 14,
}

#[contract]
pub struct MarketsContract;

#[contractimpl]
impl MarketsContract {
    /// Initialize the markets contract and set the primary admin.
    ///
    /// # Auth
    /// Requires `admin.require_auth()`.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        admin.require_auth();
        if env.storage().instance().has(&DataKey::Initialized) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Initialized, &true);
        env.storage().instance().set(&DataKey::MarketCount, &0u32);
        Ok(())
    }

    /// Create a new prediction market.
    ///
    /// # Auth
    /// Requires `admin.require_auth()` and equality with the stored admin.
    ///
    /// # Returns
    /// A unique [`Symbol`] market id (`m0`..`m999999` style label).
    pub fn create_market(
        env: Env,
        admin: Address,
        question: String,
        outcomes: Vec<String>,
        duration_days: u32,
    ) -> Result<Symbol, Error> {
        admin.require_auth();
        Self::require_admin(&env, &admin)?;
        if duration_days == 0 {
            return Err(Error::InvalidDuration);
        }
        let n = outcomes.len();
        if n < 2 || n > MAX_OUTCOMES {
            return Err(Error::InvalidOutcomes);
        }

        let count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::MarketCount)
            .unwrap_or(0);
        let next = count.checked_add(1).ok_or(Error::Overflow)?;
        let market_id = Self::market_id(&env, next)?;

        let seconds = u64::from(duration_days)
            .checked_mul(86_400)
            .ok_or(Error::Overflow)?;
        let end_time = env
            .ledger()
            .timestamp()
            .checked_add(seconds)
            .ok_or(Error::Overflow)?;

        let market = Market {
            question,
            outcomes,
            end_time,
            total_staked: 0,
            winning_outcome: String::from_str(&env, ""),
            resolved: false,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Market(market_id.clone()), &market);
        env.storage().instance().set(&DataKey::MarketCount, &next);
        Ok(market_id)
    }

    /// Cast a stake-weighted vote on an open market.
    ///
    /// Re-voting adds to the existing stake (overflow-safe) and overwrites the
    /// recorded outcome preference.
    ///
    /// # Auth
    /// Requires `user.require_auth()`.
    pub fn vote(
        env: Env,
        user: Address,
        market_id: Symbol,
        outcome: String,
        stake: i128,
    ) -> Result<(), Error> {
        user.require_auth();
        Self::require_initialized(&env)?;
        if stake < MIN_STAKE {
            return Err(Error::InvalidStake);
        }

        let mut market = Self::load_market(&env, &market_id)?;
        if market.resolved {
            return Err(Error::AlreadyResolved);
        }
        if env.ledger().timestamp() >= market.end_time {
            return Err(Error::MarketClosed);
        }
        if !Self::outcome_allowed(&market, &outcome) {
            return Err(Error::InvalidOutcome);
        }

        let stake_key = DataKey::Stake(market_id.clone(), user.clone());
        let prior: i128 = env.storage().persistent().get(&stake_key).unwrap_or(0);
        let new_user_stake = prior.checked_add(stake).ok_or(Error::Overflow)?;
        market.total_staked = market
            .total_staked
            .checked_add(stake)
            .ok_or(Error::Overflow)?;

        env.storage().persistent().set(&stake_key, &new_user_stake);
        env.storage().persistent().set(
            &DataKey::VoteOutcome(market_id.clone(), user),
            &outcome,
        );
        env.storage()
            .persistent()
            .set(&DataKey::Market(market_id), &market);
        Ok(())
    }

    /// Resolve a closed market by selecting the winning outcome.
    ///
    /// # Auth
    /// Requires `admin.require_auth()` and admin equality.
    pub fn resolve_market(
        env: Env,
        admin: Address,
        market_id: Symbol,
        winning_outcome: String,
    ) -> Result<(), Error> {
        admin.require_auth();
        Self::require_admin(&env, &admin)?;

        let mut market = Self::load_market(&env, &market_id)?;
        if market.resolved {
            return Err(Error::AlreadyResolved);
        }
        if env.ledger().timestamp() < market.end_time {
            return Err(Error::MarketStillOpen);
        }
        if !Self::outcome_allowed(&market, &winning_outcome) {
            return Err(Error::InvalidOutcome);
        }

        market.winning_outcome = winning_outcome;
        market.resolved = true;
        env.storage()
            .persistent()
            .set(&DataKey::Market(market_id), &market);
        Ok(())
    }

    /// Claim after resolution. Clears the caller's stake to prevent double claims.
    ///
    /// # Auth
    /// Requires `user.require_auth()`.
    ///
    /// # Returns
    /// The caller's stake if they selected the winning outcome, otherwise `0`
    /// (losing stake is still cleared).
    pub fn claim_winnings(env: Env, user: Address, market_id: Symbol) -> Result<i128, Error> {
        user.require_auth();
        Self::require_initialized(&env)?;

        let market = Self::load_market(&env, &market_id)?;
        if !market.resolved {
            return Err(Error::NotResolved);
        }

        let stake_key = DataKey::Stake(market_id.clone(), user.clone());
        let vote_key = DataKey::VoteOutcome(market_id, user);
        let stake: i128 = env
            .storage()
            .persistent()
            .get(&stake_key)
            .ok_or(Error::NothingToClaim)?;
        if stake <= 0 {
            return Err(Error::NothingToClaim);
        }
        let voted: String = env
            .storage()
            .persistent()
            .get(&vote_key)
            .ok_or(Error::NothingToClaim)?;

        env.storage().persistent().remove(&stake_key);
        env.storage().persistent().remove(&vote_key);

        if voted == market.winning_outcome {
            Ok(stake)
        } else {
            Ok(0)
        }
    }

    /// Read a market record (no auth).
    pub fn get_market(env: Env, market_id: Symbol) -> Result<Market, Error> {
        Self::load_market(&env, &market_id)
    }

    /// Read a user's current stake on a market (no auth).
    pub fn get_stake(env: Env, market_id: Symbol, user: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::Stake(market_id, user))
            .unwrap_or(0)
    }

    /// Return the gas-snapshot schema version embedded in this contract.
    pub fn gas_snap_version(_env: Env) -> u32 {
        GAS_SNAP_VERSION
    }
}

impl MarketsContract {
    fn require_initialized(env: &Env) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Initialized) {
            Ok(())
        } else {
            Err(Error::NotInitialized)
        }
    }

    fn require_admin(env: &Env, admin: &Address) -> Result<(), Error> {
        Self::require_initialized(env)?;
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        if &stored != admin {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    fn load_market(env: &Env, market_id: &Symbol) -> Result<Market, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Market(market_id.clone()))
            .ok_or(Error::MarketNotFound)
    }

    fn outcome_allowed(market: &Market, outcome: &String) -> bool {
        for i in 0..market.outcomes.len() {
            if let Some(o) = market.outcomes.get(i) {
                if &o == outcome {
                    return true;
                }
            }
        }
        false
    }

    /// Build a short ASCII market id label without heap formatting helpers.
    fn market_id(env: &Env, n: u32) -> Result<Symbol, Error> {
        // Labels: m0 .. m999999 (fits Symbol::new length limits used in tests).
        if n > 999_999 {
            return Err(Error::Overflow);
        }
        let mut buf = [b'm', b'0', b'0', b'0', b'0', b'0', b'0'];
        let mut x = n;
        for i in (1..7).rev() {
            buf[i] = b'0' + (x % 10) as u8;
            x /= 10;
        }
        // Trim leading zeros after 'm' except keep at least one digit.
        let mut start = 1;
        while start < 6 && buf[start] == b'0' {
            start += 1;
        }
        let mut label = [0u8; 7];
        label[0] = b'm';
        let mut len = 1;
        for i in start..7 {
            label[len] = buf[i];
            len += 1;
        }
        let s = core::str::from_utf8(&label[..len]).map_err(|_| Error::Overflow)?;
        Ok(Symbol::new(env, s))
    }
}
