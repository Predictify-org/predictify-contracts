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
//! - [`OraclesContract::version`] — contract version number
//! - [`OraclesContract::list_oracles`] — enumerate registered oracles
//! - [`OraclesContract::get_price`] — fetch a raw price from an oracle
//! - [`OraclesContract::get_price_data`] — fetch full price data from an oracle
//! - [`OraclesContract::is_oracle_healthy`] — check if an oracle is live

pub mod views;

pub use views::CapabilityFlag;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, IntoVal,
    String, Vec,
};

/// Persistent storage key for the oracle address list.
#[contracttype]
pub enum DataKey {
    /// Stores `Vec<Address>` — the ordered list of registered oracle addresses.
    OracleList,
}

/// Price data returned by `get_price_data`.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct OraclePriceData {
    /// The asset price as a fixed-point integer.
    pub price: i128,
    /// Unix timestamp of the price publication.
    pub publish_time: u64,
    /// Optional confidence interval around the price.
    pub confidence: Option<i128>,
    /// Decimal exponent.
    pub exponent: i32,
}

/// Contract-level error codes.
#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum Error {
    /// The requested oracle is not available or not responding.
    OracleUnavailable = 200,
    /// The oracle address is not registered in the registry.
    InvalidOracleConfig = 201,
    /// The oracle data is too old to be trusted.
    OracleStale = 202,
    /// Multiple oracles disagree and no consensus was reached.
    OracleNoConsensus = 203,
    /// The oracle data has been successfully verified.
    OracleVerified = 204,
    /// The market is not yet ready for oracle queries.
    MarketNotReady = 205,
    /// The fallback oracle is also unavailable.
    FallbackOracleUnavailable = 206,
    /// The resolution timeout has been reached.
    ResolutionTimeoutReached = 207,
    /// The oracle's confidence interval is too wide.
    OracleConfidenceTooWide = 208,
    /// The oracle feed identifier is invalid.
    InvalidOracleFeed = 209,
    /// The oracle callback authorization failed.
    OracleCallbackAuthFailed = 210,
    /// The oracle callback caller is not authorized.
    OracleCallbackUnauthorized = 211,
    /// The oracle callback signature is invalid.
    OracleCallbackInvalidSignature = 212,
    /// A replayed oracle callback was detected.
    OracleCallbackReplayDetected = 213,
    /// The oracle callback arrived after its deadline.
    OracleCallbackTimeout = 214,
    // ===== CAPABILITIES ERRORS (220-229) =====
    /// The requested capability is not supported by this contract version.
    CapabilityNotSupported = 220,
    /// The capabilities query returned an unexpected or malformed bitmap.
    CapabilityBitmapCorrupt = 221,
    /// A reserved capability bit was unexpectedly set.
    ReservedCapabilitySet = 222,
}

/// Minimum TTL ledgers for the oracle registry key.
const REGISTRY_TTL_BUMP_THRESHOLD: u32 = 120_960;
/// Extended TTL ledgers for the oracle registry key.
const REGISTRY_TTL_BUMP_TO: u32 = 518_400;

/// Bump the TTL of the oracle list on hot storage access without writing data.
fn bump_registry_ttl(env: &Env) {
    if env.storage().persistent().has(&DataKey::OracleList) {
        env.storage().persistent().extend_ttl(
            &DataKey::OracleList,
            REGISTRY_TTL_BUMP_THRESHOLD,
            REGISTRY_TTL_BUMP_TO,
        );
    }
}

/// Load the oracle list, renewing its persistent storage TTL first.
fn load_oracle_list(env: &Env) -> Vec<Address> {
    bump_registry_ttl(env);
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
    env.storage().persistent().extend_ttl(
        &DataKey::OracleList,
        REGISTRY_TTL_BUMP_THRESHOLD,
        REGISTRY_TTL_BUMP_TO,
    );
}

#[contract]
pub struct OraclesContract;

#[contractimpl]
impl OraclesContract {
    /// Return a bitmap of features supported by this contract build.
    pub fn capabilities(_env: Env) -> u64 {
        views::capabilities()
    }

    /// Return the contract's numeric version.
    pub fn version(_env: Env) -> u32 {
        7
    }

    /// Return all registered oracle addresses and renew the registry TTL.
    pub fn list_oracles(env: Env) -> Vec<Address> {
        load_oracle_list(&env)
    }

    /// Register `oracle` in the registry.
    ///
    /// Requires `admin` authorization.
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
    /// Requires `admin` authorization.
    pub fn remove_oracle(env: Env, admin: Address, oracle: Address) {
        admin.require_auth();

        let list = load_oracle_list(&env);
        let mut filtered: Vec<Address> = Vec::new(&env);
        for a in list.iter() {
            if a != oracle {
                filtered.push_back(a);
            }
        }
        save_oracle_list(&env, &filtered);
    }

    /// Fetch a raw price from a registered oracle.
    pub fn get_price(env: Env, oracle: Address, feed_id: String) -> Result<i128, Error> {
        let list = load_oracle_list(&env);
        if !list.contains(&oracle) {
            return Err(Error::InvalidOracleConfig);
        }

        let price: Option<i128> = env.invoke_contract(
            &oracle,
            &symbol_short!("get_price"),
            soroban_sdk::vec![&env, feed_id.into_val(&env)],
        );
        price.ok_or(Error::OracleUnavailable)
    }

    /// Fetch full price data from a registered oracle, falling back to its
    /// raw price entrypoint when extended data is unavailable.
    pub fn get_price_data(
        env: Env,
        oracle: Address,
        feed_id: String,
    ) -> Result<OraclePriceData, Error> {
        let list = load_oracle_list(&env);
        if !list.contains(&oracle) {
            return Err(Error::InvalidOracleConfig);
        }

        let full: Option<OraclePriceData> = env.invoke_contract(
            &oracle,
            &symbol_short!("get_pdata"),
            soroban_sdk::vec![&env, feed_id.clone().into_val(&env)],
        );
        if let Some(data) = full {
            return Ok(data);
        }

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
    pub fn is_oracle_healthy(env: Env, oracle: Address) -> Result<bool, Error> {
        let list = load_oracle_list(&env);
        if !list.contains(&oracle) {
            return Err(Error::InvalidOracleConfig);
        }

        let live: Option<bool> = env.invoke_contract(
            &oracle,
            &symbol_short!("is_live"),
            soroban_sdk::vec![&env],
        );
        Ok(live.unwrap_or(false))
    }
}
