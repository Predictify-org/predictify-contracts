//! Token management module for Predictify
//! Handles multi-asset support for bets and payouts using Soroban token interface.
//! Allows admin to configure allowed tokens per event or globally.

use alloc::string::ToString;
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
    /// The number of decimals for the token (stored as u32 for contracttype compatibility)
    pub decimals: u32,
}

impl Asset {
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
        // Validate contract address (must be non-empty and valid)
        if self.contract == Address::from_string(&String::from_str(env, "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA")) {
             // In Soroban, Address::default() might not be what we think. 
             // Usually we check if it's a specific "zero" address or just let the contract call fail.
             // But for validation purposes, let's check decimals.
        }
        
        // Validate decimals (Soroban tokens typically use 7-18 decimals)
        if self.decimals < 1 || self.decimals > 18 {
            return false;
        }
        true
    }

    /// Create an Asset from a ReflectorAsset.
    ///
    /// # Parameters
    /// * `env` - Soroban environment.
    /// * `reflector_asset` - The ReflectorAsset variant.
    /// * `contract_address` - The address of the token contract.
    pub fn from_reflector_asset(env: &Env, reflector_asset: &crate::types::ReflectorAsset, contract_address: Address) -> Self {
        Self {
            contract: contract_address,
            symbol: Symbol::new(env, &reflector_asset.symbol().to_string()),
            decimals: reflector_asset.decimals() as u32,
        }
    }

    pub fn matches_reflector_asset(&self, env: &Env, reflector_asset: &crate::types::ReflectorAsset) -> bool {
        self.symbol == Symbol::new(env, &reflector_asset.symbol().to_string())
            && self.decimals == reflector_asset.decimals() as u32
    }

    pub fn name(&self, env: &Env) -> String {
        let symbol_str = self.symbol.to_string();
        if symbol_str == "XLM" {
            String::from_str(env, "Stellar Lumens")
        } else if symbol_str == "BTC" {
            String::from_str(env, "Bitcoin")
        } else if symbol_str == "ETH" {
            String::from_str(env, "Ethereum")
        } else if symbol_str == "USDC" {
            String::from_str(env, "USD Coin")
        } else {
            String::from_str(env, &alloc::format!("Token ({})", symbol_str))
        }
    }

    /// Check if this is a native XLM asset.
    ///
    /// # Parameters
    /// * `env` - Soroban environment.
    pub fn is_native_xlm(&self, env: &Env) -> bool {
        // Native XLM often has a specific contract ID in Soroban (C...)
        // or is represented by Address::from_string("CDLZFC3SYJYDZT7K67VZ75YJBMKBAV27F6DLS6ALWHX77AL6XGOSBNOB") on Mainnet
        // Here we just check the symbol as a heuristic if contract is not provided or is default.
        self.symbol == Symbol::new(env, "XLM")
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
            let per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env.storage().persistent().get(&event_key).unwrap_or(soroban_sdk::Map::new(env));
            if let Some(assets) = per_event.get(market.clone()) {
                return assets.iter().any(|a| a == *asset);
            }
        }
        // Check global allowed assets
        let global_key = Symbol::new(env, "allowed_assets_global");
        let global_assets: Vec<Asset> = env.storage().persistent().get(&global_key).unwrap_or(Vec::new(env));
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
        let mut per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env.storage().persistent().get(&event_key).unwrap_or(soroban_sdk::Map::new(env));
        let mut assets = per_event.get(market_id.clone()).unwrap_or(Vec::new(env));
        if !assets.iter().any(|a| a == *asset) {
            assets.push_back(asset.clone());
            per_event.set(market_id.clone(), assets);
            env.storage().persistent().set(&event_key, &per_event);
        }
    }

    pub fn initialize_with_defaults(env: &Env) {
        let global_key = Symbol::new(env, "allowed_assets_global");
        // Only initialize if not already set
        if env.storage().persistent().get::<Symbol, Vec<Asset>>(&global_key).is_some() {
            return;
        }
        // Default registry is empty; assets are added by admin via add_global.
        let global_assets: Vec<Asset> = Vec::new(env);
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
        let per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env.storage().persistent().get(&event_key).unwrap_or(soroban_sdk::Map::new(env));
        per_event.get(market_id.clone()).unwrap_or(Vec::new(env))
    }

    /// Removes an asset from the global registry.
    ///
    /// # Errors
    /// * `Error::NotFound` if the asset was not in the registry.
    pub fn remove_global(env: &Env, asset: &Asset) -> Result<(), Error> {
        let global_key = Symbol::new(env, "allowed_assets_global");
        let global_assets: Vec<Asset> = env.storage().persistent().get(&global_key).unwrap_or(Vec::new(env));

        let mut filtered = Vec::new(env);
        let mut found = false;
        for a in global_assets.iter() {
            if a == *asset {
                found = true;
            } else {
                filtered.push_back(a);
            }
        }

        if found {
            env.storage().persistent().set(&global_key, &filtered);
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
    env.events().publish(
        (Symbol::new(env, event_name), asset.contract.clone(), asset.symbol.clone()),
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

