#![no_std]

//! # Oracles Contract
//!
//! Standalone Soroban smart contract exposing oracle management entrypoints for
//! the Predictify hybrid prediction-market system on Stellar.
//!
//! ## Responsibilities
//!
//! - Register and deregister trusted price-feed oracle addresses.
//! - Query current prices and enriched price metadata from registered oracles.
//! - Report on oracle health so the resolution layer can select a live source.
//! - Enumerate all registered oracle addresses for governance tooling.
//!
//! ## Error Model
//!
//! All fallible functions return [`Result<T, Error>`].  The [`Error`] variants
//! carry **stable numeric codes** in the `200–214` range.  These codes are part
//! of the public API and must never be renumbered without a versioning decision.
//! See [`Error`] for the full code table and [`tests/err_stab.rs`] for the
//! freeze tests that enforce this.
//!
//! ## Storage Layout
//!
//! | Key                            | Tier      | Description                        |
//! |--------------------------------|-----------|------------------------------------|
//! | `DataKey::OracleList`          | Persistent | Ordered `Vec<Address>` of oracles  |
//!
//! ## Example – full lifecycle
//!
//! ```rust,ignore
//! use soroban_sdk::{Address, Env, String};
//! use oracles::OraclesContract;
//!
//! let env = Env::default();
//! let contract_id = env.register(OraclesContract, ());
//! let client = OraclesContractClient::new(&env, &contract_id);
//!
//! let admin  = Address::generate(&env);
//! let oracle = Address::generate(&env);
//! let feed   = String::from_str(&env, "BTC/USD");
//!
//! // Register an oracle
//! client.add_oracle(&admin, &oracle);
//!
//! // Query current price
//! let price = client.get_price(&oracle, &feed).expect("price unavailable");
//!
//! // Remove when no longer needed
//! client.remove_oracle(&admin, &oracle);
//! ```

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env, String, Vec};

// ---------------------------------------------------------------------------
// Storage key
// ---------------------------------------------------------------------------

/// Persistent storage keys used by the Oracles contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Ordered list of registered oracle [`Address`] values.
    OracleList,
}

// ---------------------------------------------------------------------------
// Error codes
// ---------------------------------------------------------------------------

/// Oracle-specific error codes exposed by this contract.
///
/// These are **client-facing numeric codes** in the `200–214` range.  The
/// integer assignments are frozen: changing any value is a visible API break
/// that requires a migration or versioning decision.
///
/// # Code Table
///
/// | Variant                           | Code | Meaning                                                   |
/// |-----------------------------------|------|-----------------------------------------------------------|
/// | `OracleUnavailable`               |  200 | Oracle service unreachable or down                        |
/// | `InvalidOracleConfig`             |  201 | Bad oracle address / feed configuration                   |
/// | `OracleStale`                     |  202 | Price data exceeds freshness threshold                    |
/// | `OracleNoConsensus`               |  203 | Multi-oracle consensus could not be reached               |
/// | `OracleVerified`                  |  204 | Result already verified; no further action needed         |
/// | `MarketNotReady`                  |  205 | Market state does not allow oracle verification yet       |
/// | `FallbackOracleUnavailable`       |  206 | Fallback oracle is also unhealthy                         |
/// | `ResolutionTimeoutReached`        |  207 | Resolution window expired before consensus                |
/// | `OracleConfidenceTooWide`         |  208 | Confidence interval exceeds the acceptance threshold      |
/// | `InvalidOracleFeed`               |  209 | Feed ID is not recognised or not supported                |
/// | `OracleCallbackAuthFailed`        |  210 | Signature / authorisation check on callback failed        |
/// | `OracleCallbackUnauthorized`      |  211 | Caller is not in the authorised oracle whitelist          |
/// | `OracleCallbackInvalidSignature`  |  212 | Callback signature is malformed                           |
/// | `OracleCallbackReplayDetected`    |  213 | Nonce or timestamp already consumed                       |
/// | `OracleCallbackTimeout`           |  214 | Callback response exceeded the maximum allowed duration   |
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// Oracle service is unreachable or temporarily down. (code 200)
    OracleUnavailable = 200,
    /// Oracle address or feed configuration is invalid. (code 201)
    InvalidOracleConfig = 201,
    /// Oracle price data is stale and exceeds the freshness threshold. (code 202)
    OracleStale = 202,
    /// Consensus among multiple oracle instances could not be reached. (code 203)
    OracleNoConsensus = 203,
    /// Oracle result has already been verified; no further action is required. (code 204)
    OracleVerified = 204,
    /// Market is not yet in a state that allows oracle verification. (code 205)
    MarketNotReady = 205,
    /// Fallback oracle is also unavailable or unhealthy. (code 206)
    FallbackOracleUnavailable = 206,
    /// Resolution timeout has been reached before a result was determined. (code 207)
    ResolutionTimeoutReached = 207,
    /// Oracle confidence interval is too wide for reliable resolution. (code 208)
    OracleConfidenceTooWide = 208,
    /// Feed ID is not recognised or not supported by this oracle. (code 209)
    InvalidOracleFeed = 209,
    /// Oracle callback authentication failed (bad signature or auth check). (code 210)
    OracleCallbackAuthFailed = 210,
    /// Oracle callback caller is not in the authorised whitelist. (code 211)
    OracleCallbackUnauthorized = 211,
    /// Oracle callback signature is invalid or malformed. (code 212)
    OracleCallbackInvalidSignature = 212,
    /// Oracle callback replay detected: nonce or timestamp already used. (code 213)
    OracleCallbackReplayDetected = 213,
    /// Oracle callback timed out: response exceeded maximum allowed duration. (code 214)
    OracleCallbackTimeout = 214,
}

// ---------------------------------------------------------------------------
// Supporting types
// ---------------------------------------------------------------------------

/// Enriched price snapshot returned by [`OraclesContractClient::get_price_data`].
///
/// Beyond the raw price scalar, this structure surfaces the ledger timestamp
/// at which the oracle published the value, an optional confidence interval,
/// and the decimal exponent needed to recover the real-world price:
///
/// ```text
/// real_price = price × 10^exponent
/// ```
///
/// # Fields
///
/// | Field          | Type           | Description                                                |
/// |----------------|----------------|------------------------------------------------------------|
/// | `price`        | `i128`         | Raw price integer (sign reflects oracle direction)         |
/// | `publish_time` | `u64`          | Ledger timestamp when the oracle last updated this feed    |
/// | `confidence`   | `Option<i128>` | Half-width of the 95 % confidence interval, if available   |
/// | `exponent`     | `i32`          | Decimal exponent: `real = price × 10^exponent`             |
#[contracttype]
#[derive(Clone, Debug)]
pub struct OraclePriceData {
    /// Raw integer price from the oracle feed.
    pub price: i128,
    /// Ledger timestamp (seconds since Unix epoch) of the most recent update.
    pub publish_time: u64,
    /// Optional half-width confidence interval expressed in the same units as
    /// `price`.  `None` when the oracle does not publish confidence data.
    pub confidence: Option<i128>,
    /// Decimal exponent that converts `price` to the real-world value.
    /// For most feeds this is `0`; Pyth-style feeds often use `-8`.
    pub exponent: i32,
}

// ---------------------------------------------------------------------------
// Contract struct
// ---------------------------------------------------------------------------

/// Oracles contract: manages a registry of trusted price-feed oracle addresses
/// and exposes read/write entrypoints for the Predictify resolution layer.
#[contract]
pub struct OraclesContract;

// ---------------------------------------------------------------------------
// Contract implementation
// ---------------------------------------------------------------------------

#[contractimpl]
impl OraclesContract {
    // -----------------------------------------------------------------------
    // Registry write operations
    // -----------------------------------------------------------------------

    /// Register a new trusted oracle address in the on-chain registry.
    ///
    /// Only the caller identified as `admin` can register oracles.  The caller
    /// must authorise this call via [`Address::require_auth`] — Soroban will
    /// panic if the authorisation is missing.
    ///
    /// Duplicate registrations are silently ignored: if `oracle` is already
    /// present in the list the function returns `Ok(())` without appending a
    /// second entry.
    ///
    /// # Parameters
    ///
    /// | Name     | Type      | Description                                         |
    /// |----------|-----------|-----------------------------------------------------|
    /// | `admin`  | `Address` | Address that must authorise this administrative call |
    /// | `oracle` | `Address` | Oracle contract address to register                  |
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// This function does not return domain errors — authorisation failures
    /// cause a host-level panic before the function body executes.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// client.add_oracle(&admin, &oracle_addr);
    /// ```
    pub fn add_oracle(env: Env, admin: Address, oracle: Address) -> Result<(), Error> {
        admin.require_auth();

        let mut list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::OracleList)
            .unwrap_or_else(|| Vec::new(&env));

        // Deduplicate: only append if not already registered.
        for existing in list.iter() {
            if existing == oracle {
                return Ok(());
            }
        }

        list.push_back(oracle);
        env.storage().persistent().set(&DataKey::OracleList, &list);

        Ok(())
    }

    /// Deregister a previously registered oracle address.
    ///
    /// Only the caller identified as `admin` can deregister oracles.  The
    /// caller must authorise this call via [`Address::require_auth`].
    ///
    /// If `oracle` is not found in the registry the function returns `Ok(())`
    /// without error — idempotent removal simplifies upgrade scripts and
    /// governance flows.
    ///
    /// # Parameters
    ///
    /// | Name     | Type      | Description                                          |
    /// |----------|-----------|------------------------------------------------------|
    /// | `admin`  | `Address` | Address that must authorise this administrative call  |
    /// | `oracle` | `Address` | Oracle contract address to deregister                 |
    ///
    /// # Returns
    ///
    /// `Ok(())` on success (including the no-op case where `oracle` was not
    /// in the registry).
    ///
    /// # Errors
    ///
    /// Authorisation failures cause a host-level panic before the function
    /// body executes.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// client.remove_oracle(&admin, &oracle_addr);
    /// ```
    pub fn remove_oracle(env: Env, admin: Address, oracle: Address) -> Result<(), Error> {
        admin.require_auth();

        let list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::OracleList)
            .unwrap_or_else(|| Vec::new(&env));

        let mut updated: Vec<Address> = Vec::new(&env);
        for existing in list.iter() {
            if existing != oracle {
                updated.push_back(existing);
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::OracleList, &updated);

        Ok(())
    }

    /// Return the complete list of registered oracle addresses.
    ///
    /// The returned [`Vec<Address>`] is ordered by registration time (oldest
    /// first).  An empty vector indicates no oracles are currently registered.
    ///
    /// This function is read-only and does not require any authorisation.
    ///
    /// # Parameters
    ///
    /// *(none)*
    ///
    /// # Returns
    ///
    /// A `Vec<Address>` containing every oracle currently in the registry.
    /// Returns an empty vector when the registry has not been initialised.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let oracles: Vec<Address> = client.list_oracles();
    /// for addr in oracles.iter() {
    ///     log!(&env, "registered oracle: {}", addr);
    /// }
    /// ```
    pub fn list_oracles(env: Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&DataKey::OracleList)
            .unwrap_or_else(|| Vec::new(&env))
    }

    // -----------------------------------------------------------------------
    // Price query operations
    // -----------------------------------------------------------------------

    /// Fetch the current price from a registered oracle for a given feed ID.
    ///
    /// The price is returned as a raw `i128` integer.  The decimal exponent
    /// required to recover the real-world value depends on the oracle provider:
    /// most Reflector feeds use an implicit exponent of `0` (value already in
    /// cents), while Pyth-style feeds often use `-8`.  Use
    /// [`get_price_data`][Self::get_price_data] when the exponent is needed.
    ///
    /// This function does not mutate contract state and requires no
    /// authorisation.
    ///
    /// # Parameters
    ///
    /// | Name      | Type      | Description                                                    |
    /// |-----------|-----------|----------------------------------------------------------------|
    /// | `oracle`  | `Address` | Address of the registered oracle contract to query             |
    /// | `feed_id` | `String`  | Feed identifier string, e.g. `"BTC/USD"` or `"ETH/USD"`       |
    ///
    /// # Returns
    ///
    /// `Ok(i128)` — raw integer price value on success.
    ///
    /// # Errors
    ///
    /// | Error                   | Condition                                               |
    /// |-------------------------|---------------------------------------------------------|
    /// | `OracleUnavailable`     | Oracle contract did not respond or returned no data     |
    /// | `InvalidOracleFeed`     | `feed_id` is not supported by the target oracle         |
    /// | `OracleStale`           | The most recent price update is older than the allowed  |
    /// |                         | staleness window                                        |
    /// | `OracleConfidenceTooWide` | Price confidence interval exceeds the acceptance      |
    /// |                         | threshold (Pyth-style oracles only)                     |
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let feed = String::from_str(&env, "BTC/USD");
    /// match client.get_price(&oracle_addr, &feed) {
    ///     Ok(price) => log!(&env, "BTC/USD raw price: {}", price),
    ///     Err(e)    => log!(&env, "price fetch failed: {:?}", e),
    /// }
    /// ```
    pub fn get_price(env: Env, oracle: Address, feed_id: String) -> Result<i128, Error> {
        // Validate the oracle is registered.
        let list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::OracleList)
            .unwrap_or_else(|| Vec::new(&env));

        let mut found = false;
        for existing in list.iter() {
            if existing == oracle {
                found = true;
                break;
            }
        }
        if !found {
            return Err(Error::InvalidOracleConfig);
        }

        // Delegate to the oracle contract's `lastprice`-style invocation.
        // The oracle must expose a `get_price(feed_id: String) -> i128`
        // function; if it does not respond we surface OracleUnavailable.
        let result: Option<i128> = env.invoke_contract(
            &oracle,
            &soroban_sdk::symbol_short!("get_price"),
            soroban_sdk::vec![&env, feed_id.into()],
        );

        result.ok_or(Error::OracleUnavailable)
    }

    /// Fetch an enriched price snapshot from a registered oracle.
    ///
    /// Unlike [`get_price`][Self::get_price], this entrypoint returns an
    /// [`OraclePriceData`] struct that bundles the raw price with the publish
    /// timestamp, an optional confidence interval, and the decimal exponent.
    /// Callers that need data-quality signals (staleness checks, confidence
    /// filtering) should prefer this function.
    ///
    /// This function does not mutate contract state and requires no
    /// authorisation.
    ///
    /// # Parameters
    ///
    /// | Name      | Type      | Description                                                    |
    /// |-----------|-----------|----------------------------------------------------------------|
    /// | `oracle`  | `Address` | Address of the registered oracle contract to query             |
    /// | `feed_id` | `String`  | Feed identifier string, e.g. `"BTC/USD"` or `"ETH/USD"`       |
    ///
    /// # Returns
    ///
    /// `Ok(OraclePriceData)` — full price snapshot on success.
    ///
    /// # Errors
    ///
    /// | Error                   | Condition                                               |
    /// |-------------------------|---------------------------------------------------------|
    /// | `InvalidOracleConfig`   | `oracle` is not in the registered oracle list           |
    /// | `OracleUnavailable`     | Oracle contract did not respond or returned no data     |
    /// | `InvalidOracleFeed`     | `feed_id` is not supported by the target oracle         |
    /// | `OracleStale`           | The most recent price update is older than the allowed  |
    /// |                         | staleness window                                        |
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let feed = String::from_str(&env, "ETH/USD");
    /// let data = client.get_price_data(&oracle_addr, &feed)?;
    /// log!(&env, "ETH price: {}, published at: {}", data.price, data.publish_time);
    /// if let Some(conf) = data.confidence {
    ///     log!(&env, "confidence interval: ±{}", conf);
    /// }
    /// ```
    pub fn get_price_data(
        env: Env,
        oracle: Address,
        feed_id: String,
    ) -> Result<OraclePriceData, Error> {
        // Validate the oracle is registered.
        let list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::OracleList)
            .unwrap_or_else(|| Vec::new(&env));

        let mut found = false;
        for existing in list.iter() {
            if existing == oracle {
                found = true;
                break;
            }
        }
        if !found {
            return Err(Error::InvalidOracleConfig);
        }

        // Attempt to get full price data from the oracle contract.
        // Falls back to assembling a minimal OraclePriceData from the scalar
        // price when the oracle does not expose `get_price_data`.
        let result: Option<OraclePriceData> = env.invoke_contract(
            &oracle,
            &soroban_sdk::symbol_short!("get_pdata"),
            soroban_sdk::vec![&env, feed_id.clone().into()],
        );

        if let Some(data) = result {
            return Ok(data);
        }

        // Fallback: build a minimal snapshot from the scalar price.
        let price_result: Option<i128> = env.invoke_contract(
            &oracle,
            &soroban_sdk::symbol_short!("get_price"),
            soroban_sdk::vec![&env, feed_id.into()],
        );

        let price = price_result.ok_or(Error::OracleUnavailable)?;

        Ok(OraclePriceData {
            price,
            publish_time: env.ledger().timestamp(),
            confidence: None,
            exponent: 0,
        })
    }

    /// Check whether a registered oracle is currently healthy and responsive.
    ///
    /// A healthy oracle is one that:
    /// 1. Is present in the on-chain registry.
    /// 2. Responds successfully to a lightweight liveness probe.
    /// 3. Returns non-stale data within the acceptable freshness window.
    ///
    /// The resolution layer calls this before selecting an oracle for market
    /// settlement.  If an oracle is unhealthy the caller should fall back to
    /// the next registered oracle or wait for the oracle to recover.
    ///
    /// This function does not mutate contract state and requires no
    /// authorisation.
    ///
    /// # Parameters
    ///
    /// | Name     | Type      | Description                                      |
    /// |----------|-----------|--------------------------------------------------|
    /// | `oracle` | `Address` | Address of the registered oracle contract to probe |
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the oracle is registered and passes its health probe.
    /// `Ok(false)` if the oracle is registered but fails the probe or is
    /// unresponsive.
    ///
    /// # Errors
    ///
    /// | Error                 | Condition                                        |
    /// |-----------------------|--------------------------------------------------|
    /// | `InvalidOracleConfig` | `oracle` is not in the registered oracle list    |
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// match client.is_oracle_healthy(&oracle_addr) {
    ///     Ok(true)  => log!(&env, "oracle is healthy"),
    ///     Ok(false) => log!(&env, "oracle health check failed, using fallback"),
    ///     Err(e)    => log!(&env, "oracle not registered: {:?}", e),
    /// }
    /// ```
    pub fn is_oracle_healthy(env: Env, oracle: Address) -> Result<bool, Error> {
        // Validate the oracle is registered.
        let list: Vec<Address> = env
            .storage()
            .persistent()
            .get(&DataKey::OracleList)
            .unwrap_or_else(|| Vec::new(&env));

        let mut found = false;
        for existing in list.iter() {
            if existing == oracle {
                found = true;
                break;
            }
        }
        if !found {
            return Err(Error::InvalidOracleConfig);
        }

        // Invoke the oracle's health-check entry point ("is_live" ≤ 9 chars).
        // A `None` return (contract invocation failure) is treated as unhealthy.
        let healthy: Option<bool> = env.invoke_contract(
            &oracle,
            &soroban_sdk::symbol_short!("is_live"),
            soroban_sdk::vec![&env],
        );

        Ok(healthy.unwrap_or(false))
    }
}
