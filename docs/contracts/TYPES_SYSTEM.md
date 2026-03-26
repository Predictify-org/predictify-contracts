# Predictify Hybrid Types System

## Overview

The Predictify Hybrid contract now features a comprehensive, organized type system that centralizes all data structures and provides better organization, validation, and maintainability. This document outlines the architecture, usage patterns, and best practices for working with the types system.

## Architecture

### Type Categories

Types are organized into logical categories for better understanding and maintenance:

1. **Oracle Types** - Oracle providers, configurations, and data structures
2. **Market Types** - Market data structures and state management
3. **Price Types** - Price data and validation structures
4. **Validation Types** - Input validation and business logic types
5. **Utility Types** - Helper types and conversion utilities

### Core Components

#### 1. Oracle Types

**OracleProvider Struct (Forward-Compatible)**
```rust
pub struct OracleProvider {
    provider_id: String,
}
```

**Key Features:**
- **Forward Compatibility**: String-based representation allows new providers without breaking existing markets
- **Backward Compatibility**: Existing markets continue to work across contract upgrades
- **Graceful Degradation**: Unknown providers are handled safely with fallback behavior

**Standard Provider Identifiers:**
- `"reflector"` - Reflector oracle (primary for Stellar Network)
- `"pyth"` - Pyth Network oracle (placeholder for future Stellar support)
- `"band_protocol"` - Band Protocol oracle (not available on Stellar)
- `"dia"` - DIA oracle (not available on Stellar)

**Constructor Methods:**
```rust
let provider = OracleProvider::reflector();
let provider = OracleProvider::pyth();
let provider = OracleProvider::from_str(String::from_str(&env, "new_provider"));
```

**Validation Methods:**
```rust
provider.is_known()      // Recognized by current contract version
provider.is_supported()  // Available on current network (Stellar)
provider.validate_for_market(&env)?  // Strict validation for new markets
```

**Forward Compatibility Strategy:**
- New providers use descriptive lowercase names with underscores
- Unknown providers return "Unknown Provider (name)" for display
- Validation fails safely for unsupported providers
- No storage migration required for new provider additions

**OracleConfig Struct**
```rust
pub struct OracleConfig {
    pub provider: OracleProvider,
    pub oracle_address: Address,
    pub feed_id: String,
    pub threshold: i128,
    pub comparison: String,
}
```

**Fallback Sentinel Encoding**

Optional fallback oracles are stored as a `(has_fallback, fallback_oracle_config)` pair rather
than `Option<OracleConfig>`. When `has_fallback` is `false`, the contracts write
`OracleConfig::none_sentinel()` into `fallback_oracle_config`.

The reserved sentinel semantics are:

- `provider_id == "reflector"`
- `feed_id == ""`
- `threshold == 0`
- `comparison == ""`

That tuple is intentionally outside the valid oracle-config domain:

- valid configs require a non-empty `feed_id`
- valid configs require `threshold > 0`
- valid configs require a supported comparison operator such as `gt`, `lt`, or `eq`

Because of those invariants, the sentinel cannot collide with any configuration that is valid for
live oracle creation or resolution, even if the placeholder `oracle_address` is reused elsewhere.

#### 2. Market Types

**Market Struct**
```rust
pub struct Market {
    pub admin: Address,
    pub question: String,
    pub outcomes: Vec<String>,
    pub end_time: u64,
    pub oracle_config: OracleConfig,
    // ... other fields
}
```

#### 3. Price Types

**PythPrice Struct**
```rust
pub struct PythPrice {
    pub price: i128,
    pub conf: u64,
    pub expo: i32,
    pub publish_time: u64,
}
```

**ReflectorPriceData Struct**
```rust
pub struct ReflectorPriceData {
    pub price: i128,
    pub timestamp: u64,
}
```

## Usage Patterns

### 1. Creating Oracle Configurations

```rust
use soroban_sdk::{Address, String};
use types::{OracleConfig, OracleProvider};

let oracle_address = Address::generate(&env);

let oracle_config = OracleConfig::new(
    OracleProvider::reflector(), // Use constructor method
    oracle_address,
    String::from_str(&env, "BTC/USD"),
    2500000, // $25,000 threshold
    String::from_str(&env, "gt"), // greater than
);

// Validate the configuration
oracle_config.validate(&env)?;
```

### Forward Compatibility Examples

```rust
// Creating markets with future oracle providers
let future_provider = OracleProvider::from_str(String::from_str(&env, "chainlink"));
let config = OracleConfig::new(
    future_provider,
    oracle_address,
    String::from_str(&env, "ETH/USD"),
    2000000,
    String::from_str(&env, "gt"),
);

// Older contract versions can read this safely
assert!(!config.provider.is_known()); // Not recognized in this version
assert!(!config.provider.is_supported()); // Not supported on Stellar
assert_eq!(config.provider.as_str(), "chainlink"); // But can read the identifier

// Display name provides graceful fallback
let display_name = config.provider.name();
// Returns: "Unknown Provider (chainlink)"
```

### 2. Creating Markets

```rust
use types::{Market, OracleConfig, OracleProvider};

let market = Market::new(
    &env,
    admin,
    question,
    outcomes,
    end_time,
    oracle_config,
);

// Validate market parameters
market.validate(&env)?;
```

### 3. Market State Management

```rust
use types::MarketState;

let state = MarketState::from_market(&market, &env);

if state.is_active() {
    // Market is accepting votes
} else if state.has_ended() {
    // Market has ended
} else if state.is_resolved() {
    // Market is resolved
}
```

### 4. Oracle Result Handling

```rust
use types::OracleResult;

let result = OracleResult::price(2500000);

if result.is_available() {
    if let Some(price) = result.get_price() {
        // Use the price
    }
}
```

## Type Validation

### Built-in Validation

All types include built-in validation methods:

```rust
// Oracle configuration validation
oracle_config.validate(&env)?;

// Market validation
market.validate(&env)?;

// Price validation
pyth_price.validate()?;
```

### Validation Helpers

The types module provides validation helper functions:

```rust
use types::validation;

// Validate oracle provider
validation::validate_oracle_provider(&OracleProvider::Pyth)?;

// Validate price
validation::validate_price(2500000)?;

// Validate stake
validation::validate_stake(stake, min_stake)?;

// Validate duration
validation::validate_duration(30)?;
```

## Type Conversion

### Conversion Helpers

```rust
use types::conversion;

// Convert string to oracle provider
let provider = conversion::string_to_oracle_provider("pyth")
    .ok_or(Error::InvalidOracleConfig)?;

// Convert oracle provider to string
let provider_name = conversion::oracle_provider_to_string(&provider);

// Validate comparison operator
conversion::validate_comparison(&comparison, &env)?;
```

## Market Operations

### Market State Queries

```rust
// Check if market is active
if market.is_active(&env) {
    // Accept votes
}

// Check if market has ended
if market.has_ended(&env) {
    // Resolve market
}

// Check if market is resolved
if market.is_resolved() {
    // Allow claims
}
```

### User Operations

```rust
// Get user's vote
let user_vote = market.get_user_vote(&user);

// Get user's stake
let user_stake = market.get_user_stake(&user);

// Check if user has claimed
let has_claimed = market.has_user_claimed(&user);

// Get user's dispute stake
let dispute_stake = market.get_user_dispute_stake(&user);
```

### Market Modifications

```rust
// Add vote and stake
market.add_vote(user, outcome, stake);

// Add dispute stake
market.add_dispute_stake(user, stake);

// Mark user as claimed
market.mark_claimed(user);

// Set oracle result
market.set_oracle_result(result);

// Set winning outcome
market.set_winning_outcome(outcome);

// Mark fees as collected
market.mark_fees_collected();
```

### Market Calculations

```rust
// Get total dispute stakes
let total_disputes = market.total_dispute_stakes();

// Get winning stake total
let winning_total = market.winning_stake_total();
```

### Oracle Integration

### Oracle Provider Support

```rust
// Check if provider is supported
if oracle_provider.is_supported() {
    // Use the provider
} else {
    // Handle unsupported provider gracefully
}

// Get provider identifier
let provider_id = oracle_provider.as_str();

// Get human-readable name
let name = oracle_provider.name();

// Validate for market creation (strict)
oracle_provider.validate_for_market(&env)?;

// Check if provider is known (less strict)
if oracle_provider.is_known() {
    // Provider is recognized by this contract version
} else {
    // Provider from future contract version - handle gracefully
}
```

### Forward Compatibility Patterns

```rust
// Safe handling of unknown providers
match oracle_provider.as_str() {
    "reflector" => handle_reflector_oracle(),
    "pyth" => handle_pyth_oracle(),
    unknown => {
        println!("Unknown provider: {}", oracle_provider.name());
        use_fallback_oracle();
    }
}

// Validation for different contexts
fn validate_for_read(provider: &OracleProvider) -> Result<(), Error> {
    // Less strict validation - just check if provider exists
    if provider.as_str().is_empty() {
        return Err(Error::InvalidOracleConfig);
    }
    Ok(())
}

fn validate_for_creation(provider: &OracleProvider, env: &Env) -> Result<(), Error> {
    // Strict validation for new markets
    provider.validate_for_market(env)
}
```

### Oracle Configuration

```rust
// Check comparison operators
if oracle_config.is_greater_than(&env) {
    // Handle greater than comparison
} else if oracle_config.is_less_than(&env) {
    // Handle less than comparison
} else if oracle_config.is_equal_to(&env) {
    // Handle equal to comparison
}
```

## Price Data Handling

### Pyth Price Data

```rust
let pyth_price = PythPrice::new(2500000, 1000, -2, timestamp);

// Get price in cents
let price_cents = pyth_price.price_in_cents();

// Check if price is stale (manual check)
if env.ledger().timestamp() > pyth_price.publish_time + max_age {
    // Handle stale price
}

// Validate price data
pyth_price.validate()?;
```

### Reflector Price Data

```rust
let reflector_price = ReflectorPriceData::new(2500000, timestamp);

// Get price in cents
let price_cents = reflector_price.price_in_cents();

// Check if price is stale (manual check)
if env.ledger().timestamp() > reflector_price.timestamp + max_age {
    // Handle stale price
}

// Validate price data
reflector_price.validate()?;
```

## Validation Types

### Market Creation Parameters

```rust
let params = MarketCreationParams::new(
    admin,
    question,
    outcomes,
    duration_days,
    oracle_config,
);

// Validate all parameters
params.validate(&env)?;

// Calculate end time
let end_time = params.calculate_end_time(&env);
```

### Vote Parameters

```rust
let vote_params = VoteParams::new(user, outcome, stake);

// Validate vote parameters
vote_params.validate(&env, &market)?;
```

## Best Practices

### 1. Always Validate Types

```rust
// ❌ Don't skip validation
let market = Market::new(&env, admin, question, outcomes, end_time, oracle_config);

// ✅ Always validate
let market = Market::new(&env, admin, question, outcomes, end_time, oracle_config);
market.validate(&env)?;
```

### 2. Use Type-Safe Operations

```rust
// ❌ Manual state checking
if current_time < market.end_time && market.winning_outcome.is_none() {
    // Market is active
}

// ✅ Use type-safe methods
if market.is_active(current_time) {
    // Market is active
}
```

### 3. Leverage Built-in Methods

```rust
// ❌ Manual calculations
let mut total = 0;
for (user, outcome) in market.votes.iter() {
    if &outcome == winning_outcome {
        total += market.stakes.get(user.clone()).unwrap_or(0);
    }
}

// ✅ Use built-in methods
let total = market.winning_stake_total();
```

### 4. Use Validation Helpers

```rust
// ❌ Manual validation
if stake < min_stake {
    return Err(Error::InsufficientStake);
}

// ✅ Use validation helpers
validation::validate_stake(stake, min_stake)?;
```

### 5. Handle Oracle Results Safely

```rust
// ❌ Direct access
let price = oracle_result.price;

// ✅ Safe access
if let Some(price) = oracle_result.get_price() {
    // Use the price
}
```

## Testing

### Type Testing

The types module includes comprehensive tests:

```rust
#[test]
fn test_oracle_provider() {
    let provider = OracleProvider::Pyth;
    assert_eq!(provider.name(), "Pyth Network");
    assert!(provider.is_supported());
}

#[test]
fn test_market_creation() {
    let market = Market::new(&env, admin, question, outcomes, end_time, oracle_config);
    assert!(market.is_active(&env));
    assert!(!market.is_resolved());
}

#[test]
fn test_validation_helpers() {
    assert!(validation::validate_oracle_provider(&OracleProvider::Pyth).is_ok());
    assert!(validation::validate_price(2500000).is_ok());
}
```

## Migration Guide

### From Direct Type Usage

1. **Replace direct struct creation**:
   ```rust
   // Old
   let market = Market { /* fields */ };
   
   // New
   let market = Market::new(&env, admin, question, outcomes, end_time, oracle_config);
   ```

2. **Use validation methods**:
   ```rust
   // Old
   if threshold <= 0 { return Err(Error::InvalidThreshold); }
   
   // New
   oracle_config.validate(&env)?;
   ```

3. **Use type-safe operations**:
   ```rust
   // Old
   if current_time < market.end_time { /* active */ }
   
   // New
   if market.is_active(&env) { /* active */ }
   ```

## Type Reference

### Oracle Types

| Type | Purpose | Key Methods |
|------|---------|-------------|
| `OracleProvider` | Forward-compatible oracle provider | `reflector()`, `pyth()`, `from_str()`, `as_str()`, `name()`, `is_known()`, `is_supported()`, `validate_for_market()` |
| `OracleConfig` | Oracle configuration | `new()`, `validate()`, `none_sentinel()`, `is_none_sentinel()` |
| `PythPrice` | Pyth price data | `new()`, `price_in_cents()`, `is_stale()`, `validate()` |
| `ReflectorPriceData` | Reflector price data | `new()`, `price_in_cents()`, `is_stale()`, `validate()` |

### Market Types

| Type | Purpose | Key Methods |
|------|---------|-------------|
| `Market` | Market data structure | `new()`, `validate()`, `is_active()`, `add_vote()` |
| `MarketState` | Market state enumeration | `from_market()`, `is_active()`, `has_ended()` |
| `MarketCreationParams` | Market creation parameters | `new()`, `validate()`, `calculate_end_time()` |
| `VoteParams` | Vote parameters | `new()`, `validate()` |

### Utility Types

| Type | Purpose | Key Methods |
|------|---------|-------------|
| `OracleResult` | Oracle result wrapper | `price()`, `unavailable()`, `is_available()`, `get_price()` |
| `ReflectorAsset` | Reflector asset types | `stellar()`, `other()`, `is_stellar()`, `is_other()` |

### Validation Functions

| Function | Purpose | Parameters |
|----------|---------|------------|
| `validate_oracle_provider()` | Validate oracle provider | `provider: &OracleProvider` |
| `validate_price()` | Validate price value | `price: i128` |
| `validate_stake()` | Validate stake amount | `stake: i128, min_stake: i128` |
| `validate_duration()` | Validate duration | `duration_days: u32` |

### Conversion Functions

| Function | Purpose | Parameters |
|----------|---------|------------|
| `string_to_oracle_provider()` | Convert string to provider | `s: &str` |
| `oracle_provider_to_string()` | Convert provider to string | `provider: &OracleProvider` |
| `validate_comparison()` | Validate comparison operator | `comparison: &String, env: &Env` |

## Forward Compatibility Guide

### Adding New Oracle Providers

When adding new oracle providers to future contract versions:

1. **Choose Provider ID**: Use descriptive lowercase names with underscores
   ```rust
   // Good examples
   "chainlink"
   "uniswap_oracle"
   "custom_provider_v2"
   ```

2. **Update Constructor Methods**: Add new constructor methods in future versions
   ```rust
   // Future contract version
   impl OracleProvider {
       pub fn chainlink() -> Self {
           Self { provider_id: String::from_str(&env, "chainlink") }
       }
   }
   ```

3. **Update Validation Logic**: Add new providers to `is_known()` and `is_supported()` methods
   ```rust
   // Future contract version
   pub fn is_known(&self) -> bool {
       matches!(self.as_str(), "reflector" | "pyth" | "band_protocol" | "dia" | "chainlink")
   }
   ```

4. **No Storage Migration Required**: Existing markets continue to work without migration

### Handling Unknown Providers

When reading markets created by newer contract versions:

```rust
fn handle_oracle_provider(provider: &OracleProvider) -> Result<(), Error> {
    match provider.as_str() {
        "reflector" => use_reflector_oracle(),
        "pyth" => use_pyth_oracle(),
        "band_protocol" => use_band_protocol_oracle(),
        "dia" => use_dia_oracle(),
        unknown => {
            // Provider from future contract version
            println!("Unknown oracle provider: {}", provider.name());
            
            // Option 1: Fail gracefully
            return Err(Error::UnsupportedOracleProvider);
            
            // Option 2: Use fallback oracle
            use_fallback_oracle();
            
            // Option 3: Read-only mode (no new operations)
            enter_read_only_mode();
        }
    }
}
```

### Migration from Enum-Based System

For contracts migrating from the old enum system:

```rust
// Old enum (deprecated)
pub enum OldOracleProvider {
    Reflector,
    Pyth,
    BandProtocol,
    DIA,
}

// Migration function
fn migrate_oracle_provider(old: OldOracleProvider, env: &Env) -> OracleProvider {
    match old {
        OldOracleProvider::Reflector => OracleProvider::reflector(),
        OldOracleProvider::Pyth => OracleProvider::pyth(),
        OldOracleProvider::BandProtocol => OracleProvider::band_protocol(),
        OldOracleProvider::DIA => OracleProvider::dia(),
    }
}
```

## Future Enhancements

1. **Type Serialization**: Proper serialization/deserialization support
2. **Type Metrics**: Collection and reporting of type usage statistics
3. **Type Validation**: Enhanced validation with custom rules
4. **Type Events**: Event emission for type state changes
5. **Type Localization**: Support for multiple languages in type messages

## Conclusion

The new types system provides a robust foundation for managing data structures in the Predictify Hybrid contract. By following the patterns and best practices outlined in this document, developers can create more maintainable, type-safe, and well-organized code. 
