# Predictify Hybrid API Documentation

> **Version:** v1.0.0  
> **Platform:** Stellar Soroban  
> **Audience:** Developers integrating with Predictify Hybrid smart contracts

---

## 📋 Table of Contents

1. [API Overview](#api-overview)
2. [API Versioning](#api-versioning)
3. [Core API Reference](#core-api-reference)
4. [Data Structures](#data-structures)
5. [ReflectorAsset Coverage Matrix](#reflectorasset-coverage-matrix)
6. [Token and Asset Management](#token-and-asset-management)
7. [Error Codes](#error-codes)
8. [Integration Examples](#integration-examples)
9. [Troubleshooting Guide](#troubleshooting-guide)
10. [Support and Resources](#support-and-resources)

---

## 🚀 API Overview

The Predictify Hybrid smart contract provides a comprehensive API for building prediction market applications on the Stellar network. The API supports market creation, voting, dispute resolution, oracle integration, and administrative functions.

### Key Features

- **Market Management**: Create, extend, and resolve prediction markets
- **Voting System**: Stake-based voting with proportional payouts
- **Dispute Resolution**: Community-driven dispute and resolution system
- **Oracle Integration**: Support for Reflector, Pyth, and custom oracles
- **Asset Management**: Multi-asset support with comprehensive ReflectorAsset coverage
- **Fee Management**: Automated fee collection and distribution
- **Admin Governance**: Administrative functions for contract management

---

## 📚 API Versioning

### Current Version: v1.0.0

The Predictify Hybrid smart contract follows semantic versioning (SemVer) for API compatibility and contract upgrades. This section provides comprehensive information about API versions, compatibility, and migration strategies.

### 🏷️ Version Schema

We use **Semantic Versioning (SemVer)** with the format `MAJOR.MINOR.PATCH`:

- **MAJOR** (1.x.x): Breaking changes that require client updates
- **MINOR** (x.1.x): New features that are backward compatible
- **PATCH** (x.x.1): Bug fixes and optimizations

---

## 🔧 Core API Reference

### Rustdoc Coverage Contract

All exported contract entrypoints in `contracts/predictify-hybrid/src/lib.rs` are documented with
Rust doc comments (`///`) and include explicit `# Errors` and `# Events` sections.

This is intended to make API behavior auditable without reading all internals:

- **Errors**: Each entrypoint documents how `Error` values are surfaced.
  Functions returning `Result<_, Error>` propagate errors directly.
  Non-`Result` entrypoints surface contract failures via panic.
- **Events**: Each entrypoint documents event behavior.
  State-changing flows may emit events through internal managers (for example via `EventEmitter`),
  while read-only query flows emit no events.

For exact runtime behavior and error variants, also reference:
- `contracts/predictify-hybrid/src/err.rs`
- `contracts/predictify-hybrid/src/events.rs`

### Market Management Functions

#### `create_market()`
Creates a new prediction market with specified parameters.

**Signature:**
```rust
pub fn create_market(
    env: Env,
    admin: Address,
    question: String,
    outcomes: Vec<String>,
    duration_days: u32,
    oracle_config: OracleConfig,
    fallback_oracle_config: Option<OracleConfig>,
    resolution_timeout: u64,
    min_pool_size: Option<i128>,
    bet_deadline_mins_before_end: Option<u64>,
    dispute_window_seconds: Option<u64>,
) -> Symbol
```

**Parameters:**
- `admin`: Market administrator address
- `question`: Market question (max 500 characters)
- `outcomes`: Possible outcomes (2-10 options)
- `duration_days`: Market duration (1-365 days)
- `oracle_config`: Primary oracle configuration for resolution
- `fallback_oracle_config`: Optional fallback oracle configuration
- `resolution_timeout`: Timeout in seconds for oracle resolution
- `min_pool_size`: Optional minimum pool size required for resolution
- `bet_deadline_mins_before_end`: Optional early cutoff for bets (minutes before end)
- `dispute_window_seconds`: Optional dispute period duration (defaults to 24h)

**Returns:** Market ID (Symbol)

**Example:**
```typescript
const marketId = await contract.create_market(
    adminAddress,
    "Will Bitcoin reach $100,000 by end of 2025?",
    ["Yes", "No"],
    90, // 90 days
    oracleConfig,
    null, // no fallback
    3600, // 1h resolution timeout
    null, // no min pool size
    60,   // bet deadline 1h before end
    86400 // 24h dispute window
);
```

### Betting Functions

#### `place_bet()`
Places a bet on a prediction market outcome by locking user funds.

**Signature:**
```rust
pub fn place_bet(
    env: Env,
    user: Address,
    market_id: Symbol,
    outcome: String,
    amount: i128,
) -> Bet
```

**Parameters:**
- `user`: Address of the user placing the bet (requires authentication)
- `market_id`: Unique identifier of the market
- `outcome`: The outcome the user predicts will occur
- `amount`: Amount of tokens to lock for this bet

**Returns:** `Bet` structure with placement details

#### `cancel_bet()`
Cancels an active bet and refunds the user's locked funds. Can only be performed before the market deadline.

**Signature:**
```rust
pub fn cancel_bet(
    env: Env,
    user: Address,
    market_id: Symbol,
) -> Result<(), Error>
```

**Parameters:**
- `user`: Address of the user cancelling the bet (requires authentication)
- `market_id`: Unique identifier of the market

**Returns:** `Ok(())` on success, or an `Error` if cancellation fails.

**Example:**
```typescript
await contract.cancel_bet(
    userAddress,
    marketId
);
```

---

## 📊 Data Structures

### ReflectorAsset

Comprehensive asset enumeration for Reflector oracle integration with full testing matrix coverage.

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReflectorAsset {
    /// Stellar Lumens (XLM)
    Stellar,
    /// Bitcoin (BTC)
    BTC,
    /// Ethereum (ETH)
    ETH,
    /// Other asset identified by symbol
    Other(Symbol),
}
```

#### Asset Properties

| Asset | Symbol | Decimals | Feed ID | Supported |
|-------|---------|----------|----------|-----------|
| Stellar Lumens | XLM | 7 | XLM/USD | ✅ |
| Bitcoin | BTC | 8 | BTC/USD | ✅ |
| Ethereum | ETH | 18 | ETH/USD | ✅ |
| Custom Assets | * | 7 | CUSTOM/USD | ❌ |

#### ReflectorAsset Methods

```rust
impl ReflectorAsset {
    /// Check if this asset is Stellar Lumens (XLM)
    pub fn is_xlm(&self) -> bool;

    /// Returns symbol string for this asset
    pub fn symbol(&self) -> String;

    /// Returns human-readable name for this asset
    pub fn name(&self) -> String;

    /// Returns number of decimal places for this asset
    pub fn decimals(&self) -> u8;

    /// Returns Reflector feed ID for this asset (e.g., "BTC/USD")
    pub fn feed_id(&self) -> String;

    /// Checks if this asset is supported by Reflector oracle
    pub fn is_supported(&self) -> bool;

    /// Checks if this asset is a known asset (including custom ones)
    pub fn is_known(&self) -> bool;

    /// Validates asset for use in market creation
    pub fn validate_for_market(&self, env: &Env) -> Result<(), Error>;

    /// Creates a ReflectorAsset from a symbol string
    pub fn from_symbol(symbol: String) -> Self;

    /// Returns all supported assets for testing purposes
    pub fn all_supported() -> Vec<Self>;

    /// Returns all known assets (including unsupported) for testing purposes
    pub fn all_known() -> Vec<Self>;
}
```

#### Usage Examples

```rust
// Asset identification and properties
let btc = ReflectorAsset::BTC;
println!("Asset: {}", btc.symbol());
println!("Name: {}", btc.name());
println!("Decimals: {}", btc.decimals());

// Asset validation
let assets = vec![ReflectorAsset::BTC, ReflectorAsset::ETH, ReflectorAsset::XLM];
for asset in assets {
    if asset.is_supported() {
        println!("{} is supported by Reflector", asset.symbol());
    }
}

// Feed ID generation
let btc_feed = ReflectorAsset::BTC.feed_id();
println!("BTC feed ID: {}", btc_feed); // "BTC/USD"
```

### Asset

Represents a Stellar asset/token with contract address and metadata.

```rust
#[soroban_sdk::contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    pub contract: Address,
    pub symbol: Symbol,
    pub decimals: u8,
}
```

#### Asset Methods

```rust
impl Asset {
    /// Create a new Asset instance
    pub fn new(contract: Address, symbol: Symbol, decimals: u8) -> Self;

    /// Create an Asset from a ReflectorAsset
    pub fn from_reflector_asset(env: &Env, reflector_asset: &ReflectorAsset, contract_address: Address) -> Self;

    /// Check if this asset matches a ReflectorAsset
    pub fn matches_reflector_asset(&self, env: &Env, reflector_asset: &ReflectorAsset) -> bool;

    /// Get human-readable asset name
    pub fn name(&self, env: &Env) -> String;

    /// Check if this is a native XLM asset
    pub fn is_native_xlm(&self, env: &Env) -> bool;

    /// Validate asset for market creation
    pub fn validate_for_market(&self, env: &Env) -> Result<(), Error>;

    /// Validate token contract and decimals
    pub fn validate(&self, env: &Env) -> bool;
}
```

### TokenRegistry

Global registry for managing allowed assets with per-event and global support.

```rust
impl TokenRegistry {
    /// Checks if asset is allowed globally or for a specific event
    pub fn is_allowed(env: &Env, asset: &Asset, market_id: Option<&Symbol>) -> bool;

    /// Adds asset to global registry
    pub fn add_global(env: &Env, asset: &Asset);

    /// Adds asset to per-event registry
    pub fn add_event(env: &Env, market_id: &Symbol, asset: &Asset);

    /// Initialize registry with default supported assets
    pub fn initialize_with_defaults(env: &Env);

    /// Get all globally allowed assets
    pub fn get_global_assets(env: &Env) -> Vec<Asset>;

    /// Get assets allowed for a specific event
    pub fn get_event_assets(env: &Env, market_id: &Symbol) -> Vec<Asset>;

    /// Remove asset from global registry
    pub fn remove_global(env: &Env, asset: &Asset) -> Result<(), Error>;

    /// Validate asset against registry rules
    pub fn validate_asset(env: &Env, asset: &Asset, market_id: Option<&Symbol>) -> Result<(), Error>;
}
```

---

## 🎯 ReflectorAsset Coverage Matrix

### Comprehensive Testing Coverage

The ReflectorAsset system provides comprehensive end-to-end testing coverage for all representative assets used in production:

#### Supported Assets Matrix

| Asset | Symbol | Decimals | Feed ID | Validation | Market Creation | Oracle Integration |
|-------|---------|----------|----------|------------|------------------|-------------------|
| **Stellar Lumens** | XLM | 7 | XLM/USD | ✅ | ✅ | ✅ |
| **Bitcoin** | BTC | 8 | BTC/USD | ✅ | ✅ | ✅ |
| **Ethereum** | ETH | 18 | ETH/USD | ✅ | ✅ | ✅ |

#### Testing Coverage Areas

1. **Property Validation**: All asset properties (symbol, name, decimals, feed_id)
2. **Support Status**: is_supported() and is_known() method validation
3. **Market Validation**: validate_for_market() comprehensive testing
4. **Round-trip Conversion**: from_symbol() and back conversion testing
5. **Feed ID Format**: Consistent "/USD" suffix validation
6. **Asset-specific Properties**: XLM-specific behavior testing
7. **Integration Testing**: End-to-end market creation with all assets
8. **Registry Integration**: TokenRegistry compatibility testing

#### Test Coverage Statistics

- **Total Test Cases**: 25+ comprehensive tests
- **Coverage Target**: ≥95% line coverage on ReflectorAsset modules
- **Asset Variants**: All supported and custom asset variants
- **Error Paths**: All validation error scenarios tested
- **Edge Cases**: Boundary conditions and invalid inputs tested

### Production Asset Validation

#### Security Considerations

1. **Asset Validation**: All assets undergo rigorous validation before market creation
2. **Feed ID Verification**: Oracle feed IDs follow strict format requirements
3. **Decimal Precision**: Asset decimals are validated within acceptable ranges (1-18)
4. **Support Status**: Only supported assets can be used for live markets
5. **Registry Authorization**: Asset registry enforces authorization controls

#### Threat Model

| Threat | Mitigation | Coverage |
|---------|------------|-----------|
| Invalid Asset Symbols | Symbol validation and whitelisting | ✅ |
| Oracle Feed Manipulation | Feed ID format validation | ✅ |
| Decimal Precision Attacks | Decimal range validation (1-18) | ✅ |
| Unsupported Asset Usage | Support status validation | ✅ |
| Registry Unauthorized Access | Authorization controls | ✅ |

---

## 💰 Token and Asset Management

### Multi-Asset Support

Predictify Hybrid supports multiple asset types for betting and payouts:

1. **Native XLM**: Stellar's native asset with 7 decimal places
2. **Custom Tokens**: Soroban token contracts with configurable decimals
3. **Reflector Assets**: Pre-configured assets with oracle integration

### Asset Lifecycle

1. **Registration**: Assets registered in TokenRegistry (global or per-event)
2. **Validation**: Assets validated before use in markets
3. **Usage**: Assets used for betting, staking, and payouts
4. **Tracking**: Asset balances and transfers tracked securely

### Integration Examples

#### Creating Markets with Reflector Assets

```rust
// Create market with BTC price feed
let btc_asset = ReflectorAsset::BTC;
let oracle_config = OracleConfig::new(
    OracleProvider::Reflector,
    String::from_str(&env, &btc_asset.feed_id()),
    100_000_00, // $100,000 in cents
    String::from_str(&env, "gt")
);

let market_id = create_market(
    env,
    admin,
    String::from_str(&env, "Will BTC reach $100k?"),
    outcomes,
    30, // 30 days
    oracle_config
);
```

#### Asset Registry Management

```rust
// Initialize with default assets
TokenRegistry::initialize_with_defaults(&env);

// Add custom asset
let usdc_asset = Asset::new(
    token_contract_address,
    Symbol::new(&env, "USDC"),
    7
);
TokenRegistry::add_global(&env, &usdc_asset);

// Validate asset usage
TokenRegistry::validate_asset(&env, &usdc_asset, None)?;
```

### SAC Token Operations

Predictify Hybrid provides high-level operations for interacting with Stellar Asset Contracts (SAC) through the standard Soroban token interface.

#### `transfer_token()`
Transfers tokens from one address to another (requires sender's authorization).
```rust
pub fn transfer_token(env: &Env, asset: &Asset, from: &Address, to: &Address, amount: i128);
```

#### `approve_token()`
Approves a spender to use a specified amount of tokens from the owner.
```rust
pub fn approve_token(env: &Env, asset: &Asset, from: &Address, spender: &Address, amount: i128, expiration_ledger: u32);
```

#### `transfer_from_token()`
Transfers tokens using a previously granted allowance (requires spender's authorization).
```rust
pub fn transfer_from_token(env: &Env, asset: &Asset, spender: &Address, from: &Address, to: &Address, amount: i128);
```

#### `get_token_balance()`
Retrieves the token balance for a specified address.
```rust
pub fn get_token_balance(env: &Env, asset: &Asset, address: &Address) -> i128;
```

#### `get_token_allowance()`
Retrieves the allowance granted by an owner to a spender.
```rust
pub fn get_token_allowance(env: &Env, asset: &Asset, owner: &Address, spender: &Address) -> i128;
```

#### `validate_token_operation()`
Validates a token operation by checking asset validity and user balance.
```rust
pub fn validate_token_operation(env: &Env, asset: &Asset, user: &Address, amount: i128) -> Result<(), Error>;
```

---

## ⚠️ Error Codes

### Asset-Related Errors (500-599)

- **500**: `InvalidAsset` - Asset validation failed
- **501**: `UnsupportedAsset` - Asset not supported by Reflector
- **502**: `InvalidFeedId` - Malformed feed identifier
- **503**: `AssetNotRegistered` - Asset not found in registry
- **504**: `InvalidDecimals` - Asset decimals out of range
- **505**: `UnauthorizedAsset` - Asset not authorized for use

### User Operation Errors (100-199)
- **100**: `UserNotAuthorized` - User lacks required permissions
- **101**: `MarketNotFound` - Specified market doesn't exist
- **102**: `MarketClosed` - Market is closed for voting
- **103**: `InvalidOutcome` - Outcome not available for market
- **104**: `AlreadyVoted` - User has already voted on this market
- **105**: `NothingToClaim` - No winnings available to claim
- **106**: `MarketNotResolved` - Market resolution pending
- **107**: `InsufficientStake` - Stake below minimum requirement

### Oracle Errors (200-299)
- **200**: `OracleUnavailable` - Oracle service unavailable
- **201**: `InvalidOracleConfig` - Oracle configuration invalid
- **202**: `OracleTimeout` - Oracle response timeout
- **203**: `OracleDataInvalid` - Oracle data format invalid

---

## 💡 Integration Examples

### Basic Market Creation with Reflector Assets

```typescript
import { Contract, Keypair, Networks } from '@stellar/stellar-sdk';

// Initialize contract
const contract = new Contract(contractId);

// Create market with BTC price feed
const btcAsset = "BTC"; // ReflectorAsset::BTC
const marketId = await contract.create_market(
    adminKeypair.publicKey(),
    "Will Bitcoin reach $100,000 by Q2 2025?",
    ["Yes", "No"],
    90, // 90 days
    {
        provider: "Reflector",
        feed_id: "BTC/USD",
        threshold: 100000000, // $100,000 in cents
        timeout_seconds: 3600
    }
);

// Vote with XLM (native asset)
await contract.vote(
    userKeypair.publicKey(),
    marketId,
    "Yes",
    10000000 // 1 XLM in stroops
);
```

### Multi-Asset Market Operations

```typescript
// Create market with ETH price feed
const ethMarketId = await contract.create_market(
    adminKeypair.publicKey(),
    "Will Ethereum reach $5,000?",
    ["Yes", "No"],
    60,
    {
        provider: "Reflector",
        feed_id: "ETH/USD",
        threshold: 500000000, // $5,000 in cents
        timeout_seconds: 3600
    }
);

// Vote with custom token (USDC)
const usdcAsset = {
    contract: usdcTokenAddress,
    symbol: "USDC",
    decimals: 7
};

await contract.vote_with_asset(
    userKeypair.publicKey(),
    ethMarketId,
    "Yes",
    50000000, // 50 USDC
    usdcAsset
);
```

---

## 🆘 Troubleshooting Guide

### Asset-Related Issues

#### Problem: Asset validation fails
```rust
Error: InvalidAsset (500)
```
**Solution:**
1. Verify asset decimals are within range (1-18)
2. Check contract address is valid (not default for non-XLM)
3. Ensure asset symbol matches expected format
4. Confirm asset is registered in TokenRegistry

#### Problem: Reflector asset not supported
```rust
Error: UnsupportedAsset (501)
```
**Solution:**
1. Use supported assets: XLM, BTC, ETH
2. For custom assets, register them in TokenRegistry first
3. Check asset.is_supported() returns true before use

#### Problem: Oracle feed ID format invalid
```rust
Error: InvalidFeedId (502)
```
**Solution:**
1. Use standard feed ID format: "ASSET/USD"
2. Verify asset symbol is valid
3. Use ReflectorAsset.feed_id() for correct format

### Common Issues and Solutions

#### 🔧 Asset Registration Issues

**Problem: Asset not found in registry**
```bash
Error: AssetNotRegistered (503)
```
**Solution:**
```bash
# Check global assets
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_global_assets \
  --network mainnet

# Add asset if missing
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn add_global_asset \
  --arg asset_contract=$TOKEN_ADDRESS \
  --arg symbol=USDC \
  --arg decimals=7 \
  --network mainnet
```

#### 🔮 Oracle Integration Issues

**Problem: Reflector feed not working**
```rust
Error: OracleUnavailable (200)
```
**Solution:**
1. Verify feed ID format: "BTC/USD", "ETH/USD", "XLM/USD"
2. Check Reflector oracle service status
3. Ensure asset is supported by Reflector
4. Use fallback oracle if configured

#### 🗳️ Voting with Assets Issues

**Problem: Asset not authorized for voting**
```rust
Error: UnauthorizedAsset (505)
```
**Solution:**
1. Check asset is in global or event-specific registry
2. Verify asset validation passes
3. Ensure user has sufficient asset balance
4. Confirm asset is supported for market type

### 🔍 Debugging Tools

#### Asset Inspection
```bash
# Check asset properties
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_asset_info \
  --arg asset_contract=$TOKEN_ADDRESS \
  --network mainnet

# Validate asset
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn validate_asset \
  --arg asset_contract=$TOKEN_ADDRESS \
  --arg symbol=USDC \
  --arg decimals=7 \
  --network mainnet
```

#### Registry Inspection
```bash
# List all global assets
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_global_assets \
  --network mainnet

# Check event-specific assets
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_event_assets \
  --arg market_id="BTC_100K" \
  --network mainnet
```

---

## 📞 Support and Resources

### Error Code Reference
- **500-599**: Asset-related errors - Check asset validation and registration
- **100-199**: User operation errors - Check user permissions and market state
- **200-299**: Oracle errors - Verify oracle configuration and connectivity
- **300-399**: Validation errors - Check input parameters and formats
- **400-499**: System errors - Contact support for system-level issues

### Asset Integration Support
1. **GitHub Issues**: [Report asset integration bugs](https://github.com/predictify/contracts/issues)
2. **Discord Support**: [#asset-integration channel](https://discord.gg/predictify)
3. **Developer Forum**: [Asset integration discussions](https://forum.predictify.io)
4. **Email Support**: assets@predictify.io

### Before Contacting Support
1. Check this troubleshooting guide
2. Search existing GitHub issues for asset-related problems
3. Verify your asset configuration matches documentation
4. Collect relevant error messages and transaction hashes
5. Note your contract version and network

### Additional Resources
- [Stellar Soroban Documentation](https://soroban.stellar.org/)
- [Stellar SDK Documentation](https://stellar.github.io/js-stellar-sdk/)
- [Predictify GitHub Repository](https://github.com/predictify/contracts)
- [Reflector Oracle Documentation](https://reflector.org/)
- [Asset Integration Examples](https://github.com/predictify/examples/tree/main/assets)

---

## 🛡️ Security Considerations

### Asset Security

1. **Validation**: All assets undergo comprehensive validation before use
2. **Authorization**: Asset registry enforces strict authorization controls
3. **Precision**: Decimal precision is validated to prevent overflow attacks
4. **Feeds**: Oracle feed IDs follow strict format requirements
5. **Testing**: Comprehensive test coverage ensures asset reliability

### Threat Model

| Asset Category | Threat Level | Mitigation |
|----------------|---------------|------------|
| Native XLM | Low | Built-in Stellar security |
| Supported Tokens | Medium | Contract validation and registry |
| Custom Tokens | High | Full validation and authorization |
| Oracle Feeds | Medium | Feed ID validation and fallbacks |

### Best Practices

1. **Always validate assets** before use in markets
2. **Use supported Reflector assets** for reliable price feeds
3. **Register custom assets** in the TokenRegistry before use
4. **Test asset integration** thoroughly before production deployment
5. **Monitor asset balances** and transfers for security

---

**Last Updated:** 2026-03-27  
**API Version:** v1.0.0  
**Documentation Version:** 1.2  
**ReflectorAsset Coverage:** Production Ready with ≥95% Test Coverage
