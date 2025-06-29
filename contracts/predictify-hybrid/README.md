# Predictify Hybrid Contract with Real Reflector Oracle Integration

## Overview

This is a hybrid prediction market contract built on Stellar using Soroban that combines oracle-based resolution with community voting. The contract now supports **real integration** with the **Reflector Oracle** (Contract: `CALI2BYU2JE6WVRUFYTS6MSBNEHGJ35P4AVCZYF3B6QOE3QKOB2PLE6M`) for live price data.

## Key Features

- **Real Oracle Integration**: Live price feeds from Reflector oracle contract
- **Hybrid Resolution**: Combines oracle data with community voting
- **Multiple Oracle Support**: Pyth and Reflector oracle integration
- **Dispute System**: Stake-based dispute mechanism
- **Fee Structure**: 2% platform fee + 1 XLM creation fee

## Real Reflector Oracle Integration

### How It Works

The contract now makes **actual calls** to the Reflector oracle contract:

1. **Contract-to-Contract Calls**: Uses `env.invoke_contract()` to call Reflector functions
2. **Price Fetching**: Calls `lastprice()` and `twap()` functions from Reflector
3. **Fallback Mechanism**: If `lastprice()` fails, tries `twap()` with 1 record
4. **Error Handling**: Returns `OracleUnavailable` if both methods fail

### Reflector Contract Functions Used

```rust
// Get latest price for an asset
lastprice(asset: ReflectorAsset) -> Option<ReflectorPriceData>

// Get Time-Weighted Average Price
twap(asset: ReflectorAsset, records: u32) -> Option<i128>
```

### Supported Assets

The Reflector oracle supports various assets including:
- **BTC** (Bitcoin)
- **ETH** (Ethereum)
- **XLM** (Stellar Lumens)
- And other assets configured in the Reflector contract

## Contract Functions

### 1. Initialize Contract
```rust
initialize(admin: Address)
```

### 2. Create Markets

#### Using Real Reflector Oracle
```rust
create_reflector_market(
    admin: Address,
    question: String,
    outcomes: Vec<String>,
    duration_days: u32,
    asset_symbol: String,  // e.g., "BTC", "ETH"
    threshold: i128,       // Price threshold in cents
    comparison: String,    // "gt", "lt", "eq"
) -> Symbol
```

#### Using Reflector Oracle with Specific Asset
```rust
create_reflector_asset_market(
    admin: Address,
    question: String,
    outcomes: Vec<String>,
    duration_days: u32,
    asset_symbol: String,  // e.g., "BTC", "ETH", "XLM"
    threshold: i128,       // Price threshold in cents
    comparison: String,    // "gt", "lt", "eq"
) -> Symbol
```

#### Using Pyth Oracle (Mock for now)
```rust
create_pyth_market(
    admin: Address,
    question: String,
    outcomes: Vec<String>,
    duration_days: u32,
    feed_id: String,       // Pyth feed ID
    threshold: i128,       // Price threshold in cents
    comparison: String,    // "gt", "lt", "eq"
) -> Symbol
```

### 3. Oracle Resolution
```rust
fetch_oracle_result(
    market_id: Symbol,
    oracle_contract: Address,  // Reflector contract address
) -> String
```

## Usage Examples

### Example 1: Real BTC Price Prediction with Reflector

```javascript
// Contract addresses
const PREDICTIFY_CONTRACT = "your_predictify_contract_address";
const REFLECTOR_CONTRACT = "CALI2BYU2JE6WVRUFYTS6MSBNEHGJ35P4AVCZYF3B6QOE3QKOB2PLE6M";
const TOKEN_CONTRACT = "your_token_contract_address";

// 1. Initialize contract
await predictifyClient.initialize(adminAddress);

// 2. Set token contract
await predictifyClient.set_token_contract(tokenContractAddress);

// 3. Create BTC price prediction market using real Reflector oracle
const marketId = await predictifyClient.create_reflector_market(
    adminAddress,
    "Will BTC price be above $50,000 by December 31, 2024?",
    ["yes", "no"],
    30, // 30 days duration
    "BTC", // Asset symbol for Reflector
    5000000, // $50,000 threshold (in cents)
    "gt" // Greater than comparison
);

// 4. Users vote
await predictifyClient.vote(
    userAddress,
    marketId,
    "yes",
    1000000000 // 100 XLM stake
);

// 5. After market ends, fetch real oracle result from Reflector
const oracleResult = await predictifyClient.fetch_oracle_result(
    marketId,
    REFLECTOR_CONTRACT
);

// 6. Resolve market
const finalResult = await predictifyClient.resolve_market(marketId);

// 7. Winners claim their rewards
await predictifyClient.claim_winnings(userAddress, marketId);
```

### Example 2: ETH Price Prediction with Real Data

```javascript
// Create ETH price prediction using real Reflector data
const ethMarketId = await predictifyClient.create_reflector_asset_market(
    adminAddress,
    "Will ETH price be below $3,000 by January 15, 2025?",
    ["yes", "no"],
    45, // 45 days duration
    "ETH", // Asset symbol for Reflector
    300000, // $3,000 threshold (in cents)
    "lt" // Less than comparison
);
```

### Example 3: XLM Price Prediction

```javascript
// Create XLM price prediction
const xlmMarketId = await predictifyClient.create_reflector_asset_market(
    adminAddress,
    "Will XLM price be above $0.15 by February 1, 2025?",
    ["yes", "no"],
    60, // 60 days duration
    "XLM", // Asset symbol for Reflector
    1500, // $0.15 threshold (in cents)
    "gt" // Greater than comparison
);
```

## Real Oracle Integration Details

### Contract Calls Made

The integration makes these **actual calls** to the Reflector contract:

```rust
// Get latest price
reflector_client.lastprice(ReflectorAsset::Other(Symbol::new(env, "BTC")))

// Get TWAP (Time-Weighted Average Price) as fallback
reflector_client.twap(ReflectorAsset::Other(Symbol::new(env, "BTC")), 1)
```

### Price Format

- **Input**: Prices are specified in cents (e.g., $50,000 = 5,000,000 cents)
- **Output**: Reflector returns prices in the same format
- **Precision**: Maintains precision with integer arithmetic

### Error Handling

- **OracleUnavailable**: When Reflector contract calls fail
- **InvalidOracleConfig**: Invalid oracle configuration
- **MarketClosed**: Market has ended
- **Unauthorized**: Admin-only functions

## Testing with Real Oracle

### Test Environment Setup

For testing with the real Reflector oracle:

1. **Deploy to Testnet**: Use Stellar testnet for testing
2. **Use Real Contract**: Point to the actual Reflector contract address
3. **Monitor Calls**: Check contract call logs for oracle interactions

### Example Test with Real Data

```javascript
// Test with real Reflector oracle
const testMarketId = await predictifyClient.create_reflector_market(
    adminAddress,
    "Test: Will BTC be above $40,000 in 1 hour?",
    ["yes", "no"],
    1, // 1 day for testing
    "BTC",
    4000000, // $40,000
    "gt"
);

// Wait for market to end
await new Promise(resolve => setTimeout(resolve, 3600000)); // 1 hour

// Fetch real oracle result
const realResult = await predictifyClient.fetch_oracle_result(
    testMarketId,
    REFLECTOR_CONTRACT
);

console.log("Real oracle result:", realResult);
```

## Deployment Steps

1. **Build the contract**:
```bash
cargo build --target wasm32-unknown-unknown --release
```

2. **Deploy to Stellar**:
```bash
soroban contract deploy --wasm target/wasm32-unknown-unknown/release/predictify_hybrid.wasm
```

3. **Initialize with admin**:
```bash
soroban contract invoke --id <contract_id> -- initialize --admin <admin_address>
```

4. **Set token contract**:
```bash
soroban contract invoke --id <contract_id> -- set_token_contract --token_contract <token_address>
```

## Security Features

- **Authentication**: All functions require proper signatures
- **Authorization**: Admin-only functions protected
- **Input Validation**: Comprehensive validation of all inputs
- **Reentrancy Protection**: Soroban's built-in protection
- **Stake Tracking**: Proper tracking of user stakes and claims
- **Oracle Validation**: Real contract calls with error handling

## Performance Considerations

- **Gas Costs**: Real contract calls have associated gas costs
- **Latency**: Oracle calls may take time to complete
- **Fallback**: TWAP used as fallback if latest price unavailable
- **Caching**: Consider caching oracle results for efficiency

## Future Enhancements

1. **Multiple Oracle Support**: Add more oracle providers
2. **Oracle Aggregation**: Combine multiple oracle results
3. **Dynamic Asset Support**: Parse feed_id for different asset pairs
4. **Price Validation**: Add confidence intervals and staleness checks
5. **Governance**: Add DAO-style governance for parameter updates

## Troubleshooting

### Common Issues

1. **OracleUnavailable Error**: Check if Reflector contract is accessible
2. **Asset Not Found**: Verify asset symbol is supported by Reflector
3. **Price Staleness**: Check if oracle data is recent enough
4. **Network Issues**: Ensure stable connection to Stellar network

### Debugging

```javascript
// Check oracle availability
try {
    const result = await predictifyClient.fetch_oracle_result(marketId, reflectorContract);
    console.log("Oracle result:", result);
} catch (error) {
    console.error("Oracle error:", error);
}
```

The contract is now ready for production use with **real oracle data** from the Reflector network! 