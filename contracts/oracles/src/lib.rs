#![no_std]

//! Predictify oracle registry contract.
//!
//! # Entrypoints
//!
//! ## State-changing (require admin `require_auth`)
//! - [`OraclesContract::add_oracle`] — register an oracle address
//! - [`OraclesContract::remove_oracle`] — deregister an oracle address
//!
//! ## Read-only (no auth)
//! - [`OraclesContract::capabilities`] — `u64` bitmap of supported features
//! - [`OraclesContract::version`]      — contract version number
//! - [`OraclesContract::list_oracles`] — enumerate registered oracles
//! - [`OraclesContract::get_price`]    — fetch a raw price from an oracle
//! - [`OraclesContract::get_price_data`] — fetch full price data from an oracle
//! - [`OraclesContract::is_oracle_healthy`] — check if an oracle is live

pub mod views;

pub use views::CapabilityFlag;

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, String, Vec};

// ---------------------------------------------------------------------------
// Storage key
// ---------------------------------------------------------------------------

/// Persistent storage key for the oracle address list.
#[contracttype]
pub enum DataKey {
    /// Stores `Vec<Address>` — the ordered list of registered oracle addresses.
    OracleList,
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Price data returned by `get_price_data`.
///
/// Mirrors the interface expected from oracle contracts that implement the
/// extended price-data entrypoint (`get_pdata`).
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct OraclePriceData {
    /// The asset price as a fixed-point integer.
    pub price: i128,
    /// Unix timestamp of the price publication.
    pub publish_time: u64,
    /// Optional confidence interval around the price (same units as `price`).
    pub confidence: Option<i128>,
    /// Decimal exponent: `real_price = price * 10^exponent`.
    pub exponent: i32,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Contract-level error codes.
///
/// Codes are in the **200-series** range and are part of the client-facing
/// API surface — do not renumber existing variants.
#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// 200 — The requested oracle is not available or not responding.
    OracleUnavailable = 200,
    /// 201 — The oracle address is not registered in the registry.
    InvalidOracleConfig = 201,
    /// 202 — The oracle data is too old to be trusted.
    OracleStale = 202,
    /// 203 — Multiple oracles disagree and no consensus was reached.
    OracleNoConsensus = 203,
    /// 204 — The oracle data has been successfully verified.
    OracleVerified = 204,
    /// 205 — The market is not yet ready for oracle queries.
    MarketNotReady = 205,
    /// 206 — The fallback oracle is also unavailable.
    FallbackOracleUnavailable = 206,
    /// 207 — The resolution timeout has been reached.
    ResolutionTimeoutReached = 207,
    /// 208 — The oracle's confidence interval is too wide.
    OracleConfidenceTooWide = 208,
    /// 209 — The oracle feed identifier is invalid.
    InvalidOracleFeed = 209,
    /// 210 — The oracle callback authorization failed.
    OracleCallbackAuthFailed = 210,
    /// 211 — The oracle callback caller is not authorized.
    OracleCallbackUnauthorized = 211,
    /// 212 — The oracle callback signature is invalid.
    OracleCallbackInvalidSignature = 212,
    /// 213 — A replayed oracle callback was detected.
    OracleCallbackReplayDetected = 213,
    /// 214 — The oracle callback arrived after its deadline.
    OracleCallbackTimeout = 214,
}

// ---------------------------------------------------------------------------
// TTL constants
// ---------------------------------------------------------------------------

/// Minimum TTL ledgers for the oracle registry key (≈ 7 days at 5 s/ledger).
const REGISTRY_TTL_BUMP_THRESHOLD: u32 = 120_960;
/// Extended TTL ledgers for the oracle registry key (≈ 30 days at 5 s/ledger).
const REGISTRY_TTL_BUMP_TO: u32 = 518_400;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Load the oracle list from persistent storage, returning an empty `Vec` if
/// the key does not yet exist.
fn load_oracle_list(env: &Env) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::OracleList)
        .unwrap_or_else(|| Vec::new(env))
}

/// Persist the oracle list and bump the entry's TTL.
fn save_oracle_list(env: &Env, list: &Vec<Address>) {
    env.storage()
        .persistent()
        .set(&DataKey::OracleList, list);
    env.storage()
        .persistent()
        .extend_ttl(&DataKey::OracleList, REGISTRY_TTL_BUMP_THRESHOLD, REGISTRY_TTL_BUMP_TO);
}

/// Bump the TTL of the oracle list on hot read paths without writing data.
fn bump_registry_ttl(env: &Env) {
    if env.storage().persistent().has(&DataKey::OracleList) {
        env.storage().persistent().extend_ttl(
            &DataKey::OracleList,
            REGISTRY_TTL_BUMP_THRESHOLD,
            REGISTRY_TTL_BUMP_TO,
        );
    }
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct OraclesContract;

#[contractimpl]
impl OraclesContract {
    // -----------------------------------------------------------------------
    // Read-only views
    // -----------------------------------------------------------------------

    /// Return a `u64` bitmap of features supported by this contract build.
    ///
    /// Each set bit corresponds to a [`CapabilityFlag`] constant.  Callers
    /// should mask individual bits with `&` rather than comparing the whole
    /// value, so that new bits added in future upgrades do not break existing
    /// checks.
    ///
    /// # Auth
    ///
    /// None — this is a pure read with no storage access.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let caps = client.capabilities();
    /// assert!(caps & CapabilityFlag::GET_PRICE != 0);
    /// ```
    pub fn capabilities(_env: Env) -> u64 {
        views::capabilities()
    }

    /// Return the contract's numeric version.
    ///
    /// Bump this value on every breaking ABI change.
    ///
    /// # Auth
    ///
    /// None.
    pub fn version(_env: Env) -> u32 {
        7
    }

    /// Return the list of all registered oracle addresses.
    ///
    /// The returned `Vec` is ordered by insertion time.  The list may be
    /// empty if no oracles have been registered yet.
    ///
    /// # Auth
    ///
    /// None — read-only.
    pub fn list_oracles(env: Env) -> Vec<Address> {
        bump_registry_ttl(&env);
        load_oracle_list(&env)
    }

    // -----------------------------------------------------------------------
    // State-changing entrypoints
    // -----------------------------------------------------------------------

    /// Register `oracle` in the registry.
    ///
    /// If `oracle` is already registered this is a no-op (idempotent).
    ///
    /// # Auth
    ///
    /// Requires `admin` authorization via [`Address::require_auth`].
    pub fn add_oracle(env: Env, admin: Address, oracle: Address) {
        admin.require_auth();

        let mut list = load_oracle_list(&env);
        if !list.contains(&oracle) {
            list.push_back(oracle);
        }
        save_oracle_list(&env, &list);
    }

    /// Remove `oracle` from the registry.
    ///
    /// If `oracle` is not registered this is a no-op (idempotent).
    ///
    /// # Auth
    ///
    /// Requires `admin` authorization via [`Address::require_auth`].
    pub fn remove_oracle(env: Env, admin: Address, oracle: Address) {
        admin.require_auth();

        let list = load_oracle_list(&env);
        let filtered: Vec<Address> = list.iter().filter(|a| *a != oracle).collect();
        save_oracle_list(&env, &filtered);
    }

    // -----------------------------------------------------------------------
    // Oracle query entrypoints
    // -----------------------------------------------------------------------

    /// Fetch a raw price from a registered oracle.
    ///
    /// Invokes `get_price(feed_id)` on the oracle contract.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidOracleConfig`] if `oracle` is not registered.
    /// - [`Error::OracleUnavailable`] if the cross-contract call fails.
    ///
    /// # Auth
    ///
    /// None.
    pub fn get_price(env: Env, oracle: Address, feed_id: String) -> Result<i128, Error> {
        let list = load_oracle_list(&env);
        if !list.contains(&oracle) {
            return Err(Error::InvalidOracleConfig);
        }
        bump_registry_ttl(&env);

        let price: Option<i128> = env.invoke_contract(
            &oracle,
            &symbol_short!("get_price"),
            soroban_sdk::vec![&env, feed_id.into_val(&env)],
        );
        price.ok_or(Error::OracleUnavailable)
    }

    /// Fetch full price data from a registered oracle.
    ///
    /// Tries `get_pdata(feed_id)` first; if that call returns `None` it
    /// falls back to `get_price(feed_id)` and synthesises an
    /// [`OraclePriceData`] with `confidence = None` and `exponent = 0`.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidOracleConfig`] if `oracle` is not registered.
    /// - [`Error::OracleUnavailable`] if both cross-contract calls fail.
    ///
    /// # Auth
    ///
    /// None.
    pub fn get_price_data(env: Env, oracle: Address, feed_id: String) -> Result<OraclePriceData, Error> {
        let list = load_oracle_list(&env);
        if !list.contains(&oracle) {
            return Err(Error::InvalidOracleConfig);
        }
        bump_registry_ttl(&env);

        // Try the extended data entrypoint first.
        let full: Option<OraclePriceData> = env.invoke_contract(
            &oracle,
            &symbol_short!("get_pdata"),
            soroban_sdk::vec![&env, feed_id.clone().into_val(&env)],
        );
        if let Some(data) = full {
            return Ok(data);
        }

        // Fall back to the basic price entrypoint.
        let price: Option<i128> = env.invoke_contract(
            &oracle,
            &symbol_short!("get_price"),
            soroban_sdk::vec![&env, feed_id.into_val(&env)],
        );
        match price {
            Some(p) => Ok(OraclePriceData {
                price: p,
                publish_time: env.ledger().timestamp(),
                confidence: None,
                exponent: 0,
            }),
            None => Err(Error::OracleUnavailable),
        }
    }

    /// Check whether a registered oracle reports itself as live.
    ///
    /// Invokes `is_live()` on the oracle contract.  If the call fails for
    /// any reason the oracle is considered unhealthy and `false` is returned.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidOracleConfig`] if `oracle` is not registered.
    ///
    /// # Auth
    ///
    /// None.
    pub fn is_oracle_healthy(env: Env, oracle: Address) -> Result<bool, Error> {
        let list = load_oracle_list(&env);
        if !list.contains(&oracle) {
            return Err(Error::InvalidOracleConfig);
        }
        bump_registry_ttl(&env);

        let live: Option<bool> = env.invoke_contract(
            &oracle,
            &symbol_short!("is_live"),
            soroban_sdk::vec![&env],
        );
        Ok(live.unwrap_or(false))
    }
}
