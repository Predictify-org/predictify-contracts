//! Token management module for Predictify
// Handles multi-asset support for bets and payouts using Soroban token interface.
// Allows admin to configure allowed tokens per event or globally.

use soroban_sdk::{Address, Env, Symbol};

/// Represents a Stellar asset/token (contract address + symbol).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
        /**
         * @notice Validate token contract and decimals for custom Stellar asset.
         * @dev Ensures contract address is valid and decimals are within bounds (1-18).
         * @param env Soroban environment
         * @return True if valid, false otherwise
         */
    pub contract: Address,
    pub symbol: Symbol,
    pub decimals: u8,
}

impl Asset {
    /// Validate token contract and decimals
    /// Returns true if contract address is valid and decimals are within reasonable bounds (1-18)
    pub fn validate(&self, env: &Env) -> bool {
        // Validate contract address (must be non-empty and valid)
        if self.contract.is_default(env) {
            return false;
        }
        // Validate decimals (Soroban tokens typically use 7-18 decimals)
        if self.decimals < 1 || self.decimals > 18 {
            return false;
        }
        true
    }
}

/// Token registry for allowed assets
pub struct TokenRegistry;
    /**
     * @notice Check if asset is allowed globally or for a specific event.
     * @dev Supports per-event and global asset registry.
     * @param env Soroban environment
     * @param asset Asset info
     * @param market_id Optional market identifier
     * @return True if allowed, false otherwise
     */

impl TokenRegistry {
    /// Checks if asset is allowed globally or for a specific event
    pub fn is_allowed(env: &Env, asset: &Asset, market_id: Option<&Symbol>) -> bool {
        // Check per-event allowed assets
        if let Some(market) = market_id {
            let event_key = Symbol::new(env, "allowed_assets_evt");
            let per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env.storage().persistent().get(&event_key).unwrap_or(soroban_sdk::Map::new(env));
            if let Some(assets) = per_event.get(market.clone()) {
                return assets.iter().any(|a| a == asset);
            }
        }
        // Check global allowed assets
        let global_key = Symbol::new(env, "allowed_assets_global");
        let global_assets: Vec<Asset> = env.storage().persistent().get(&global_key).unwrap_or(Vec::new(env));
        global_assets.iter().any(|a| a == asset)
    }

    /// Adds asset to global registry
    pub fn add_global(env: &Env, asset: &Asset) {
        let global_key = Symbol::new(env, "allowed_assets_global");
        let mut global_assets: Vec<Asset> = env.storage().persistent().get(&global_key).unwrap_or(Vec::new(env));
        if !global_assets.iter().any(|a| a == asset) {
            global_assets.push_back(asset.clone());
            env.storage().persistent().set(&global_key, &global_assets);
        }
    }

    /// Adds asset to per-event registry
    pub fn add_event(env: &Env, market_id: &Symbol, asset: &Asset) {
        let event_key = Symbol::new(env, "allowed_assets_evt");
        let mut per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env.storage().persistent().get(&event_key).unwrap_or(soroban_sdk::Map::new(env));
        let mut assets = per_event.get(market_id.clone()).unwrap_or(Vec::new(env));
        if !assets.iter().any(|a| a == asset) {
            assets.push_back(asset.clone());
            per_event.set(market_id.clone(), assets);
            env.storage().persistent().set(&event_key, &per_event);
        }
    }
}

/// Handles token transfer for bets and payouts
pub fn transfer_token(env: &Env, asset: &Asset, from: &Address, to: &Address, amount: i128) {
        /**
         * @notice Transfer custom Stellar token/asset using Soroban token interface.
         * @dev Calls token contract's transfer method.
         * @param env Soroban environment
         * @param asset Asset info
         * @param from Sender address
         * @param to Recipient address
         * @param amount Amount to transfer
         */
    // Use Soroban token interface for transfer
    let contract = &asset.contract;
    // Validate decimals
    if !asset.validate(env) {
        panic_with_error!(env, crate::errors::Error::InvalidInput);
    }
    // Call Soroban token contract's transfer method
    // Actual Soroban token interface: contract.call("transfer", from, to, amount)
    contract.call(env, "transfer", (from.clone(), to.clone(), amount));
}

/// Emits event with asset info
pub fn emit_asset_event(env: &Env, asset: &Asset, event: &str) {
        /**
         * @notice Emit event with asset info for transparency.
         * @dev Publishes asset details in contract events.
         * @param env Soroban environment
         * @param asset Asset info
         * @param event Event name
         */
    // Emit event with asset details
    env.events().publish(
        (event, asset.contract.clone(), asset.symbol.clone(), asset.decimals),
        "asset_event"
    );
}
