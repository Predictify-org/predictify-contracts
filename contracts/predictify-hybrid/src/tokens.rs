//! Token management module for Predictify
//! Handles multi-asset support for bets and payouts using Soroban token interface.
//! Allows admin to configure allowed tokens per event or globally.

use alloc::{format, string::ToString};
use soroban_sdk::{token, Address, Env, String, Symbol, Vec};
use crate::err::Error;

/// Represents a Stellar asset/token (contract address + symbol).
#[soroban_sdk::contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    /// The address of the token contract
    pub contract: Address,
    /// The symbol of the token (e.g., XLM, USDC)
    pub symbol: Symbol,
    /// The number of decimals for the token (stored as u32 for contract type compatibility)
    pub decimals: u32,
}

impl Asset {
    /// Create an Asset from a ReflectorAsset.
    ///
    /// # Parameters
    /// * `env` - Soroban environment.
    /// * `reflector_asset` - The ReflectorAsset variant.
    /// * `contract_address` - The address of the token contract.
    pub fn from_reflector_asset(env: &Env, reflector_asset: &crate::types::ReflectorAsset, contract_address: Address) -> Self {
        let symbol = match reflector_asset {
            crate::types::ReflectorAsset::Stellar => Symbol::new(env, "XLM"),
            crate::types::ReflectorAsset::BTC => Symbol::new(env, "BTC"),
            crate::types::ReflectorAsset::ETH => Symbol::new(env, "ETH"),
            crate::types::ReflectorAsset::Other(s) => s.clone(),
        };
        Self {
            contract: contract_address,
            symbol,
            decimals: reflector_asset.decimals() as u32,
        }
    }

    pub fn matches_reflector_asset(&self, env: &Env, reflector_asset: &crate::types::ReflectorAsset) -> bool {
        let expected_symbol = match reflector_asset {
            crate::types::ReflectorAsset::Stellar => Symbol::new(env, "XLM"),
            crate::types::ReflectorAsset::BTC => Symbol::new(env, "BTC"),
            crate::types::ReflectorAsset::ETH => Symbol::new(env, "ETH"),
            crate::types::ReflectorAsset::Other(s) => s.clone(),
        };
        self.symbol == expected_symbol && self.decimals == reflector_asset.decimals() as u32
    }

    pub fn name(&self, env: &Env) -> String {
        if self.symbol == Symbol::new(env, "XLM") {
            String::from_str(env, "Stellar Lumens")
        } else if self.symbol == Symbol::new(env, "BTC") {
            String::from_str(env, "Bitcoin")
        } else if self.symbol == Symbol::new(env, "ETH") {
            String::from_str(env, "Ethereum")
        } else if self.symbol == Symbol::new(env, "USDC") {
            String::from_str(env, "USD Coin")
        } else {
            String::from_str(env, "Token")
        }
    }

    /// Check if this is a native XLM asset.
    ///
    /// # Parameters
    /// * `env` - Soroban environment.
    pub fn is_native_xlm(&self, env: &Env) -> bool {
        self.symbol == Symbol::new(env, "XLM")
    }

    /// Create a new Asset instance.
    ///
    /// # Parameters
    /// * `contract` - The address of the token contract.
    /// * `symbol` - The token's symbol.
    /// * `decimals` - The number of decimals for the token.
    pub fn new(contract: Address, symbol: Symbol, decimals: u32) -> Self {
        Self {
            contract,
            symbol,
            decimals,
        }
    }

    /// Validate token contract and decimals.
    ///
    /// Ensures contract address is valid (not default) and decimals are within bounds (1-18).
    ///
    /// # Parameters
    /// * `env` - Soroban environment.
    ///
    /// # Returns
    /// * `true` if valid, `false` otherwise.
    pub fn validate(&self, env: &Env) -> bool {
        if self.decimals < 1 || self.decimals > 18 {
            return false;
        }
        true
    }

    /// Validate asset for market creation.
    ///
    /// # Errors
    /// * `Error::InvalidInput` if validation fails.
    pub fn validate_for_market(&self, env: &Env) -> Result<(), Error> {
        if !self.validate(env) {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

/// Token registry for managing allowed assets in the protocol.
pub struct TokenRegistry;

impl TokenRegistry {
    /// Checks if an asset is allowed globally or for a specific market.
    ///
    /// # Parameters
    /// * `env` - Soroban environment.
    /// * `asset` - The asset to check.
    /// * `market_id` - Optional market identifier for per-event overrides.
    pub fn is_allowed(env: &Env, asset: &Asset, market_id: Option<&Symbol>) -> bool {
        // Check per-event allowed assets
        if let Some(market) = market_id {
            let event_key = Symbol::new(env, "allowed_assets_evt");
            let per_event_empty: soroban_sdk::Map<Symbol, Vec<Asset>> = soroban_sdk::Map::new(env);
            let per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env.storage().persistent().get(&event_key).unwrap_or(per_event_empty);
            if let Some(assets) = per_event.get(market.clone()) {
                return assets.iter().any(|a| a == *asset);
            }
        }
        // Check global allowed assets
        let global_key = Symbol::new(env, "allowed_assets_global");
        let global_empty: Vec<Asset> = Vec::new(env);
        let global_assets: Vec<Asset> = env.storage().persistent().get(&global_key).unwrap_or(global_empty);
        global_assets.iter().any(|a| a == *asset)
    }

    /// Adds an asset to the global allowed registry.
    pub fn add_global(env: &Env, asset: &Asset) {
        let global_key = Symbol::new(env, "allowed_assets_global");
        let mut global_assets: Vec<Asset> = env.storage().persistent().get(&global_key).unwrap_or(Vec::new(env));
        if !global_assets.iter().any(|a| a == *asset) {
            global_assets.push_back(asset.clone());
            env.storage().persistent().set(&global_key, &global_assets);
        }
    }

    /// Adds an asset to a specific market's allowed registry.
    pub fn add_event(env: &Env, market_id: &Symbol, asset: &Asset) {
        let event_key = Symbol::new(env, "allowed_assets_evt");
        let per_event_empty: soroban_sdk::Map<Symbol, Vec<Asset>> = soroban_sdk::Map::new(env);
        let mut per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env.storage().persistent().get(&event_key).unwrap_or(per_event_empty);
        let empty_assets: Vec<Asset> = Vec::new(env);
        let mut assets: Vec<Asset> = per_event.get(market_id.clone()).unwrap_or(empty_assets);
        if !assets.iter().any(|a| a == *asset) {
            assets.push_back(asset.clone());
            per_event.set(market_id.clone(), assets);
            env.storage().persistent().set(&event_key, &per_event);
        }
    }

    pub fn initialize_with_defaults(env: &Env) {
        let global_key = Symbol::new(env, "allowed_assets_global");
        let mut global_assets: Vec<Asset> = Vec::new(env);

        // Add default supported assets from Reflector
        let reflector_assets = crate::types::ReflectorAsset::all_supported();
        for reflector_asset in reflector_assets.iter() {
            // Placeholder: in production these would be the actual SAC contract addresses
            // Using a placeholder address since actual addresses would come from deployment
            let contract_address = Address::from_string(&String::from_str(env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"));

            let asset = Asset::from_reflector_asset(env, &reflector_asset, contract_address);
            if !global_assets.iter().any(|a| a == asset) {
                global_assets.push_back(asset);
            }
        }

        env.storage().persistent().set(&global_key, &global_assets);
    }

    /// Returns a list of all globally allowed assets.
    pub fn get_global_assets(env: &Env) -> Vec<Asset> {
        let global_key = Symbol::new(env, "allowed_assets_global");
        env.storage().persistent().get(&global_key).unwrap_or(Vec::new(env))
    }

    /// Returns a list of assets allowed for a specific market.
    pub fn get_event_assets(env: &Env, market_id: &Symbol) -> Vec<Asset> {
        let event_key = Symbol::new(env, "allowed_assets_evt");
        let per_event_empty: soroban_sdk::Map<Symbol, Vec<Asset>> = soroban_sdk::Map::new(env);
        let per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env.storage().persistent().get(&event_key).unwrap_or(per_event_empty);
        let empty_assets: Vec<Asset> = Vec::new(env);
        per_event.get(market_id.clone()).unwrap_or(empty_assets)
    }

    /// Removes an asset from the global registry.
    ///
    /// # Errors
    /// * `Error::NotFound` if the asset was not in the registry.
    pub fn remove_global(env: &Env, asset: &Asset) -> Result<(), Error> {
        let global_key = Symbol::new(env, "allowed_assets_global");
        let mut global_assets: Vec<Asset> = env.storage().persistent().get(&global_key).unwrap_or(Vec::new(env));

        let initial_len = global_assets.len();
        let mut new_assets: Vec<Asset> = Vec::new(env);
        for a in global_assets.iter() {
            if a != *asset {
                new_assets.push_back(a);
            }
        }

        if new_assets.len() < initial_len {
            env.storage().persistent().set(&global_key, &new_assets);
            Ok(())
        } else {
            Err(Error::ConfigNotFound)
        }
    }

    /// Validates an asset against protocol rules and registry status.
    ///
    /// # Errors
    /// * `Error::InvalidInput` if asset properties are invalid.
    /// * `Error::Unauthorized` if asset is not in the registry.
    pub fn validate_asset(env: &Env, asset: &Asset, market_id: Option<&Symbol>) -> Result<(), Error> {
        // First validate basic asset properties
        asset.validate_for_market(env)?;
        
        // Then check if it's allowed in the relevant registry
        if !Self::is_allowed(env, asset, market_id) {
            return Err(Error::Unauthorized);
        }
        
        Ok(())
    }
}

// ===== TOKEN OPERATIONS =====

/// Transfers tokens using the Soroban token interface.
///
/// # Parameters
/// * `env` - Soroban environment.
/// * `asset` - The asset to transfer.
/// * `from` - Sender address.
/// * `to` - Recipient address.
/// * `amount` - Amount to transfer.
pub fn transfer_token(env: &Env, asset: &Asset, from: &Address, to: &Address, amount: i128) {
    from.require_auth();
    let client = token::Client::new(env, &asset.contract);
    client.transfer(from, to, &amount);
}

/// Approves an allowance for a spender.
///
/// # Parameters
/// * `env` - Soroban environment.
/// * `asset` - The asset to approve.
/// * `from` - Owner address.
/// * `spender` - Spender address.
/// * `amount` - Allowance amount.
/// * `expiration_ledger` - Ledger sequence when the allowance expires.
pub fn approve_token(env: &Env, asset: &Asset, from: &Address, spender: &Address, amount: i128, expiration_ledger: u32) {
    from.require_auth();
    let client = token::Client::new(env, &asset.contract);
    client.approve(from, spender, &amount, &expiration_ledger);
}

/// Transfers tokens using a previously granted allowance.
///
/// # Parameters
/// * `env` - Soroban environment.
/// * `asset` - The asset to transfer.
/// * `spender` - Spender address (caller).
/// * `from` - Owner address.
/// * `to` - Recipient address.
/// * `amount` - Amount to transfer.
pub fn transfer_from_token(env: &Env, asset: &Asset, spender: &Address, from: &Address, to: &Address, amount: i128) {
    spender.require_auth();
    let client = token::Client::new(env, &asset.contract);
    client.transfer_from(spender, from, to, &amount);
}

/// Retrieves the token balance for an address.
///
/// # Parameters
/// * `env` - Soroban environment.
/// * `asset` - The asset to check.
/// * `address` - The address to check balance for.
pub fn get_token_balance(env: &Env, asset: &Asset, address: &Address) -> i128 {
    let client = token::Client::new(env, &asset.contract);
    client.balance(address)
}

/// Checks if a user has sufficient balance for an operation.
///
/// # Errors
/// * `Error::InsufficientBalance` if balance is too low.
pub fn check_token_balance(env: &Env, asset: &Asset, user: &Address, amount: i128) -> Result<(), Error> {
    if get_token_balance(env, asset, user) < amount {
        return Err(Error::InsufficientBalance);
    }
    Ok(())
}

/// Checks the current allowance for a spender.
pub fn get_token_allowance(env: &Env, asset: &Asset, owner: &Address, spender: &Address) -> i128 {
    let client = token::Client::new(env, &asset.contract);
    client.allowance(owner, spender)
}

/// Emits an event with asset information for transparency.
///
/// # Parameters
/// * `env` - Soroban environment.
/// * `asset` - Asset info.
/// * `event_name` - Descriptive name of the event.
pub fn emit_asset_event(env: &Env, asset: &Asset, event_name: &str) {
    let event_symbol = Symbol::new(env, event_name);
    env.events().publish(
        (event_symbol, asset.contract.clone(), asset.symbol.clone()),
        "asset_event"
    );
}

// ===== ERROR HANDLING HELPERS =====

/// Validates token operations and provides detailed error feedback.
pub fn validate_token_operation(env: &Env, asset: &Asset, user: &Address, amount: i128) -> Result<(), Error> {
    if amount <= 0 {
        return Err(Error::InvalidInput);
    }
    asset.validate_for_market(env)?;
    check_token_balance(env, asset, user, amount)?;
    Ok(())
}

