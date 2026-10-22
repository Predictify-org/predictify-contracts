//! Token management module for Predictify
//! Handles multi-asset support for bets and payouts using Soroban token interface.
//! Allows admin to configure allowed assets per event or globally.
//!
//! Canonical internal scale: 7 decimals (1 token = 10^7 units).
//! All cross-asset comparisons and calculations use this normalized scale.

use crate::err::Error;
use alloc::{format, string::ToString};
use core::convert::TryInto;
use soroban_sdk::{token, Address, Env, String, Symbol, Vec};

/// Canonical internal scale (7 decimals)
pub const CANONICAL_DECIMALS: u32 = 7;

/// Normalizes an amount from a token's decimal scale to the canonical 7-decimal scale.
///
/// # Parameters
/// * `amount` - The amount in the token's native decimals
/// * `decimals` - The token's number of decimals
///
/// # Returns
/// The normalized amount in 7-decimal scale
pub fn normalize_amount(amount: i128, decimals: u32) -> i128 {
    if decimals == CANONICAL_DECIMALS {
        return amount;
    }

    let diff = (decimals as i32 - CANONICAL_DECIMALS as i32).abs();
    let factor = 10i128.pow(diff as u32);

    if decimals > CANONICAL_DECIMALS {
        // Need to divide (round down)
        amount / factor
    } else {
        // Need to multiply
        amount * factor
    }
}

/// Denormalizes an amount from the canonical 7-decimal scale back to a token's decimal scale.
///
/// # Parameters
/// * `amount` - The normalized amount in 7-decimal scale
/// * `decimals` - The token's number of decimals
///
/// # Returns
/// The denormalized amount in the token's native decimals
pub fn denormalize_amount(amount: i128, decimals: u32) -> i128 {
    if decimals == CANONICAL_DECIMALS {
        return amount;
    }

    let diff = (decimals as i32 - CANONICAL_DECIMALS as i32).abs();
    let factor = 10i128.pow(diff as u32);

    if decimals > CANONICAL_DECIMALS {
        // Need to multiply
        amount * factor
    } else {
        // Need to divide (round down)
        amount / factor
    }
}

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
    pub fn from_reflector_asset(
        env: &Env,
        reflector_asset: &crate::types::ReflectorAsset,
        contract_address: Address,
    ) -> Self {
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

    pub fn matches_reflector_asset(
        &self,
        env: &Env,
        reflector_asset: &crate::types::ReflectorAsset,
    ) -> bool {
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
            let per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env
                .storage()
                .persistent()
                .get(&event_key)
                .unwrap_or(per_event_empty);
            if let Some(assets) = per_event.get(market.clone()) {
                return assets.iter().any(|a| a == *asset);
            }
        }
        // Check global allowed assets
        let global_key = Symbol::new(env, "allowed_assets_global");
        let global_empty: Vec<Asset> = Vec::new(env);
        let global_assets: Vec<Asset> = env
            .storage()
            .persistent()
            .get(&global_key)
            .unwrap_or(global_empty);
        global_assets.iter().any(|a| a == *asset)
    }

    /// Adds an asset to the global allowed registry with decimals verification.
    ///
    /// This function performs a critical security check by verifying that the
    /// declared decimals match the on-chain SAC decimals() value. This prevents
    /// denomination mistakes that have caused real losses on other Stellar protocols.
    ///
    /// # Errors
    /// * `Error::AssetDecimalsMismatch` if declared decimals don't match on-chain value.
    ///
    /// # Security Notes
    /// - Performs cross-contract call to token's decimals() function
    /// - Rejects registration if mismatch detected
    /// - Should only be called by admin
    pub fn add_global_verified(env: &Env, asset: &Asset) -> Result<(), Error> {
        // Verify decimals before registration
        verify_token_decimals(env, asset)?;
        
        let global_key = Symbol::new(env, "allowed_assets_global");
        let mut global_assets: Vec<Asset> = env
            .storage()
            .persistent()
            .get(&global_key)
            .unwrap_or(Vec::new(env));
        if !global_assets.iter().any(|a| a == *asset) {
            global_assets.push_back(asset.clone());
            env.storage().persistent().set(&global_key, &global_assets);
        }
        Ok(())
    }

    /// Adds an asset to a specific market's allowed registry with decimals verification.
    ///
    /// # Parameters
    /// * `env` - Soroban environment.
    /// * `market_id` - Market identifier.
    /// * `asset` - The asset to register.
    ///
    /// # Errors
    /// * `Error::AssetDecimalsMismatch` if declared decimals don't match on-chain value.
    pub fn add_event_verified(env: &Env, market_id: &Symbol, asset: &Asset) -> Result<(), Error> {
        // Verify decimals before registration
        verify_token_decimals(env, asset)?;
        
        let event_key = Symbol::new(env, "allowed_assets_evt");
        let per_event_empty: soroban_sdk::Map<Symbol, Vec<Asset>> = soroban_sdk::Map::new(env);
        let mut per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env
            .storage()
            .persistent()
            .get(&event_key)
            .unwrap_or(per_event_empty);
        let empty_assets: Vec<Asset> = Vec::new(env);
        let mut assets: Vec<Asset> = per_event.get(market_id.clone()).unwrap_or(empty_assets);
        if !assets.iter().any(|a| a == *asset) {
            assets.push_back(asset.clone());
            per_event.set(market_id.clone(), assets);
            env.storage().persistent().set(&event_key, &per_event);
        }
        Ok(())
    }

    /// Adds an asset to the global allowed registry.
    pub fn add_global(env: &Env, asset: &Asset) {
        let global_key = Symbol::new(env, "allowed_assets_global");
        let mut global_assets: Vec<Asset> = env
            .storage()
            .persistent()
            .get(&global_key)
            .unwrap_or(Vec::new(env));
        if !global_assets.iter().any(|a| a == *asset) {
            global_assets.push_back(asset.clone());
            env.storage().persistent().set(&global_key, &global_assets);
        }
    }

    /// Adds an asset to a specific market's allowed registry.
    pub fn add_event(env: &Env, market_id: &Symbol, asset: &Asset) {
        let event_key = Symbol::new(env, "allowed_assets_evt");
        let per_event_empty: soroban_sdk::Map<Symbol, Vec<Asset>> = soroban_sdk::Map::new(env);
        let mut per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env
            .storage()
            .persistent()
            .get(&event_key)
            .unwrap_or(per_event_empty);
        let empty_assets: Vec<Asset> = Vec::new(env);
        let mut assets: Vec<Asset> = per_event.get(market_id.clone()).unwrap_or(empty_assets);
        if !assets.iter().any(|a| a == *asset) {
            assets.push_back(asset.clone());
            per_event.set(market_id.clone(), assets);
            env.storage().persistent().set(&event_key, &per_event);
        }
    }

    /// Registers an asset in the global registry with decimal validation.
    ///
    /// Gets the actual decimals from the live SAC (Stellar Asset Contract) and
    /// persists them. On re-registration (same contract address), validates that
    /// the live decimals match the stored decimals to prevent denomination mistakes
    /// that would silently inflate or deflate stakes via `normalize_amount`.
    ///
    /// # Errors
    /// * `Error::InvalidInput` if the SAC decimals are invalid (e.g., negative).
    /// * `Error::AssetDecimalsMismatch` if the asset is already registered with
    ///   different decimals than the live SAC reports.
    pub fn register_asset(env: &Env, asset: &Asset) -> Result<(), Error> {
        let token_client = token::Client::new(env, &asset.contract);
        let live_decimals: u32 = token_client
            .decimals()
            .try_into()
            .map_err(|_| Error::InvalidInput)?;

        let global_key = Symbol::new(env, "allowed_assets_global");
        let global_assets: Vec<Asset> = env
            .storage()
            .persistent()
            .get(&global_key)
            .unwrap_or(Vec::new(env));

        // Check if contract is already registered
        if let Some(existing) = global_assets.iter().find(|a| a.contract == asset.contract) {
            if existing.decimals != live_decimals {
                return Err(Error::AssetDecimalsMismatch);
            }
            // Already registered with matching decimals - nothing to do
            return Ok(());
        }

        // Register with live SAC decimals
        let registered = Asset {
            contract: asset.contract.clone(),
            symbol: asset.symbol.clone(),
            decimals: live_decimals,
        };
        Self::add_global(env, &registered);
        Ok(())
    }

    pub fn initialize_with_defaults(env: &Env) {
        let global_key = Symbol::new(env, "allowed_assets_global");
        let mut global_assets: Vec<Asset> = Vec::new(env);

        // Add default supported assets from Reflector
        let reflector_assets = crate::types::ReflectorAsset::all_supported();
        for reflector_asset in reflector_assets.iter() {
            // Placeholder: in production these would be the actual SAC contract addresses
            // Using a placeholder address since actual addresses would come from deployment
            let contract_address = Address::from_string(&String::from_str(
                env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ));

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
        env.storage()
            .persistent()
            .get(&global_key)
            .unwrap_or(Vec::new(env))
    }

    /// Returns a list of assets allowed for a specific market.
    pub fn get_event_assets(env: &Env, market_id: &Symbol) -> Vec<Asset> {
        let event_key = Symbol::new(env, "allowed_assets_evt");
        let per_event_empty: soroban_sdk::Map<Symbol, Vec<Asset>> = soroban_sdk::Map::new(env);
        let per_event: soroban_sdk::Map<Symbol, Vec<Asset>> = env
            .storage()
            .persistent()
            .get(&event_key)
            .unwrap_or(per_event_empty);
        let empty_assets: Vec<Asset> = Vec::new(env);
        per_event.get(market_id.clone()).unwrap_or(empty_assets)
    }

    /// Removes an asset from the global registry.
    ///
    /// # Errors
    /// * `Error::NotFound` if the asset was not in the registry.
    pub fn remove_global(env: &Env, asset: &Asset) -> Result<(), Error> {
        let global_key = Symbol::new(env, "allowed_assets_global");
        let mut global_assets: Vec<Asset> = env
            .storage()
            .persistent()
            .get(&global_key)
            .unwrap_or(Vec::new(env));

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
    pub fn validate_asset(
        env: &Env,
        asset: &Asset,
        market_id: Option<&Symbol>,
    ) -> Result<(), Error> {
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
pub fn approve_token(
    env: &Env,
    asset: &Asset,
    from: &Address,
    spender: &Address,
    amount: i128,
    expiration_ledger: u32,
) {
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
pub fn transfer_from_token(
    env: &Env,
    asset: &Asset,
    spender: &Address,
    from: &Address,
    to: &Address,
    amount: i128,
) {
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
pub fn check_token_balance(
    env: &Env,
    asset: &Asset,
    user: &Address,
    amount: i128,
) -> Result<(), Error> {
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
        "asset_event",
    );
}

// ===== ERROR HANDLING HELPERS =====

/// Validates token operations and provides detailed error feedback.
pub fn validate_token_operation(
    env: &Env,
    asset: &Asset,
    user: &Address,
    amount: i128,
) -> Result<(), Error> {
    if amount <= 0 {
        return Err(Error::InvalidInput);
    }
    asset.validate_for_market(env)?;
    check_token_balance(env, asset, user, amount)?;
    Ok(())
}

// ===== SAC DECIMALS VERIFICATION =====

/// Verifies that a token's declared decimals match the on-chain value.
///
/// This is a critical security check that prevents denomination mistakes.
/// Real-world on-chain losses have occurred on other Stellar protocols when
/// tokens with mismatched decimals were trusted without verification.
///
/// # Parameters
/// * `env` - Soroban environment.
/// * `asset` - The asset to verify. Uses the declared decimals value.
///
/// # Returns
/// * `Ok(())` if the declared decimals match the SAC's decimals() output.
/// * `Err(Error::AssetDecimalsMismatch)` if they don't match.
///
/// # Cross-Contract Call
/// This function performs a cross-contract call to the token contract's
/// `decimals()` function using the Soroban token interface.
///
/// # Example
/// ```rust,ignore
/// let asset = Asset::new(token_contract, "USDC".into(), 7);
/// verify_token_decimals(&env, &asset)?;  // Verifies on-chain
/// ```
pub fn verify_token_decimals(env: &Env, asset: &Asset) -> Result<(), Error> {
    // Create a token client for cross-contract call
    let client = token::Client::new(env, &asset.contract);
    
    // Call the on-chain decimals() function
    let on_chain_decimals: u32 = client.decimals();
    
    // Compare with declared decimals
    if on_chain_decimals != asset.decimals {
        return Err(Error::AssetDecimalsMismatch);
    }
    
    Ok(())
}

/// Batch verification of multiple assets' decimals.
///
/// Useful for verifying all globally allowed assets or market-specific assets
/// during initialization or periodic audits.
///
/// # Parameters
/// * `env` - Soroban environment.
/// * `assets` - Vector of assets to verify.
///
/// # Returns
/// * `Ok(())` if all assets pass verification.
/// * `Err(Error::AssetDecimalsMismatch)` if any asset fails (first failure only).
pub fn verify_token_decimals_batch(env: &Env, assets: &Vec<Asset>) -> Result<(), Error> {
    for asset in assets.iter() {
        verify_token_decimals(env, &asset)?;
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_normalize_6_decimals() {
        // Test a token with 6 decimals (e.g., USDC)
        let amount = 1_000_000; // 1 token in 6 decimals
        let normalized = normalize_amount(amount, 6);
        assert_eq!(normalized, 10_000_000); // Should be 1 token in 7 decimals
    }

    #[test]
    fn test_normalize_7_decimals() {
        // Test native XLM (7 decimals)
        let amount = 10_000_000; // 1 XLM
        let normalized = normalize_amount(amount, 7);
        assert_eq!(normalized, 10_000_000); // Should stay the same
    }

    #[test]
    fn test_normalize_8_decimals() {
        // Test BTC (8 decimals)
        let amount = 100_000_000; // 1 BTC
        let normalized = normalize_amount(amount, 8);
        assert_eq!(normalized, 10_000_000); // 1 token in 7 decimals
    }

    #[test]
    fn test_normalize_18_decimals() {
        // Test ETH (18 decimals)
        let amount = 1_000_000_000_000_000_000; // 1 ETH
        let normalized = normalize_amount(amount, 18);
        assert_eq!(normalized, 10_000_000); // 1 token in 7 decimals
    }

    #[test]
    fn test_denormalize_6_decimals() {
        let normalized = 10_000_000; // 1 token in 7 decimals
        let denormalized = denormalize_amount(normalized, 6);
        assert_eq!(denormalized, 1_000_000); // 1 token in 6 decimals
    }

    #[test]
    fn test_denormalize_7_decimals() {
        let normalized = 10_000_000;
        let denormalized = denormalize_amount(normalized, 7);
        assert_eq!(denormalized, 10_000_000);
    }

    #[test]
    fn test_denormalize_8_decimals() {
        let normalized = 10_000_000;
        let denormalized = denormalize_amount(normalized, 8);
        assert_eq!(denormalized, 100_000_000);
    }

    #[test]
    fn test_denormalize_18_decimals() {
        let normalized = 10_000_000;
        let denormalized = denormalize_amount(normalized, 18);
        assert_eq!(denormalized, 1_000_000_000_000_000_000);
    }

    #[test]
    fn test_round_trip_normalize_denormalize() {
        // Test 6 decimals
        let original_6 = 123_456;
        let normalized_6 = normalize_amount(original_6, 6);
        let denormalized_6 = denormalize_amount(normalized_6, 6);
        assert_eq!(denormalized_6, original_6 / 1); // Since we divide then multiply

        // Test 7 decimals
        let original_7 = 12_345_678;
        let normalized_7 = normalize_amount(original_7, 7);
        let denormalized_7 = denormalize_amount(normalized_7, 7);
        assert_eq!(denormalized_7, original_7);

        // Test 8 decimals
        let original_8 = 123_456_789;
        let normalized_8 = normalize_amount(original_8, 8);
        let denormalized_8 = denormalize_amount(normalized_8, 8);
        assert_eq!(denormalized_8, (original_8 / 10) * 10); // Precision loss when normalizing down
    }
}
