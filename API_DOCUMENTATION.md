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
5. [Error Codes](#error-codes)
6. [Integration Examples](#integration-examples)
7. [Troubleshooting Guide](#troubleshooting-guide)
8. [Support and Resources](#support-and-resources)

---

## 🚀 API Overview

The Predictify Hybrid smart contract provides a comprehensive API for building prediction market applications on the Stellar network. The API supports market creation, voting, dispute resolution, oracle integration, and administrative functions.

### Key Features

- **Market Management**: Create, extend, and resolve prediction markets
- **Voting System**: Stake-based voting with proportional payouts
- **Dispute Resolution**: Community-driven dispute and resolution system
- **Oracle Integration**: Support for Reflector, Pyth, and custom oracles
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

### 📋 Version History

#### v1.0.0 (Current) - Production Release
**Release Date:** 2025-01-15  
**Status:** ✅ Active

**Core Features:**
- Complete prediction market functionality
- Oracle integration (Reflector, Pyth)
- Voting and dispute resolution system
- Fee collection and distribution
- Admin governance functions
- Comprehensive validation system

**API Endpoints:**
- `initialize(admin: Address)` - Contract initialization
- `create_market(...)` - Market creation
- `vote(...)` - User voting
- `dispute_market(...)` - Dispute submission
- `claim_winnings(...)` - Claim payouts
- `collect_fees(...)` - Admin fee collection
- `resolve_market(...)` - Market resolution

**Breaking Changes from v0.x.x:**
- Renamed `submit_vote()` to `vote()`
- Updated oracle configuration structure
- Modified dispute threshold calculation
- Enhanced validation error codes

### 🔄 Compatibility Matrix

| Client Version | Contract v1.0.x | Contract v0.9.x | Contract v0.8.x |
|----------------|-----------------|-----------------|------------------|
| Client v1.0.x  | ✅ Full         | ⚠️ Limited      | ❌ Incompatible  |
| Client v0.9.x  | ⚠️ Limited      | ✅ Full         | ✅ Full          |
| Client v0.8.x  | ❌ Incompatible | ⚠️ Limited      | ✅ Full          |

**Legend:**
- ✅ **Full**: Complete compatibility, all features supported
- ⚠️ **Limited**: Basic functionality works, some features unavailable
- ❌ **Incompatible**: Not supported, upgrade required

### 🚀 Upgrade Strategies

#### For Contract Upgrades

**1. Backward Compatible Updates (MINOR/PATCH)**
```bash
# Deploy new version alongside existing
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/predictify_hybrid_v1_1_0.wasm \
  --network mainnet

# Update contract references gradually
# Old version continues to work
```

**2. Breaking Changes (MAJOR)**
```bash
# 1. Deploy new contract version
# 2. Migrate critical state (if supported)
# 3. Update all client applications
# 4. Deprecate old contract

# Migration example
soroban contract invoke \
  --id $NEW_CONTRACT_ID \
  --fn migrate_from_v0 \
  --arg old_contract=$OLD_CONTRACT_ID
```

#### For Client Applications

**JavaScript/TypeScript Example:**
```typescript
// Version-aware client initialization
const contractVersion = await getContractVersion(contractId);

if (contractVersion.startsWith('1.0')) {
    // Use v1.0 API
    await contract.vote(marketId, outcome, stake);
} else if (contractVersion.startsWith('0.9')) {
    // Use legacy API
    await contract.submit_vote(marketId, outcome, stake);
} else {
    throw new Error(`Unsupported contract version: ${contractVersion}`);
}
```

### 📖 API Documentation by Version

#### Current API (v1.0.x)

**Core Functions:**
- **Market Management**: `create_market()`, `extend_market()`, `resolve_market()`
- **Voting Operations**: `vote()`, `claim_winnings()`
- **Dispute System**: `dispute_market()`, `vote_on_dispute()`
- **Oracle Integration**: `submit_oracle_result()`, `update_oracle_config()`
- **Admin Functions**: `collect_fees()`, `update_config()`, `pause_contract()`

**Data Structures:**
- `Market`: Core market data structure
- `Vote`: User vote representation
- `OracleConfig`: Oracle configuration
- `DisputeThreshold`: Dynamic dispute thresholds

**Error Codes:**
- 100-199: User operation errors
- 200-299: Oracle errors
- 300-399: Validation errors
- 400-499: System errors

#### Legacy API (v0.9.x)

**Deprecated Functions:**
- `submit_vote()` → Use `vote()` in v1.0+
- `create_prediction_market()` → Use `create_market()` in v1.0+
- `get_market_stats()` → Use `get_market_analytics()` in v1.0+

### 🔍 Version Detection

**Check Contract Version:**
```bash
# Using Soroban CLI
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_version \
  --network mainnet
```

**JavaScript/TypeScript:**
```typescript
import { Contract } from '@stellar/stellar-sdk';

const getContractVersion = async (contractId: string): Promise<string> => {
    try {
        const result = await contract.call('get_version');
        return result.toString();
    } catch (error) {
        // Fallback for older contracts without version endpoint
        return '0.9.0';
    }
};
```

### 🛡️ Deprecation Policy

**Timeline:**
- **Announcement**: 90 days before deprecation
- **Warning Period**: 60 days with deprecation warnings
- **End of Support**: 30 days notice before complete removal

**Current Deprecations:**
- `submit_vote()`: Deprecated in v1.0.0, removal planned for v2.0.0
- `create_prediction_market()`: Deprecated in v1.0.0, removal planned for v2.0.0

### 📅 Release Schedule

**Planned Releases:**
- **v1.1.0** (Q2 2025): Enhanced analytics, batch operations
- **v1.2.0** (Q3 2025): Multi-token support, advanced oracles
- **v2.0.0** (Q4 2025): Complete API redesign, performance improvements

### 🔗 Version-Specific Resources

**Documentation:**
- [v1.0.x API Reference](./docs/api/v1.0/)
- [v0.9.x Legacy Docs](./docs/api/v0.9/)
- [Migration Guide v0.9 → v1.0](./docs/migration/v0.9-to-v1.0.md)

**Contract Addresses:**
- **v1.0.x Mainnet**: `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQAHHAGK3HGU`
- **v0.9.x Mainnet**: `CBLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQAHHAGK3ABC`

**Support Channels:**
- [GitHub Issues](https://github.com/predictify/contracts/issues) - Bug reports and feature requests
- [Discord #api-support](https://discord.gg/predictify) - Community support
- [Developer Forum](https://forum.predictify.io) - Technical discussions

---

## 🔧 Core API Reference

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
) -> Result<Symbol, Error>
```

**Parameters:**
- `admin`: Market administrator address
- `question`: Market question (max 200 characters)
- `outcomes`: Possible outcomes (2-10 options)
- `duration_days`: Market duration (1-365 days)
- `oracle_config`: Oracle configuration for resolution

**Returns:** Market ID (Symbol)

**Example:**
```typescript
const marketId = await contract.create_market(
    adminAddress,
    "Will Bitcoin reach $100,000 by end of 2025?",
    ["Yes", "No"],
    90, // 90 days
    oracleConfig
);
```

#### `vote()`
Submit a vote on a market outcome with stake.

**Signature:**
```rust
pub fn vote(
    env: Env,
    voter: Address,
    market_id: Symbol,
    outcome: String,
    stake: i128,
) -> Result<(), Error>
```

**Parameters:**
- `voter`: Voter's address
- `market_id`: Target market ID
- `outcome`: Chosen outcome
- `stake`: Stake amount (minimum 0.1 XLM)

**Example:**
```typescript
await contract.vote(
    voterAddress,
    "BTC_100K",
    "Yes",
    5000000 // 0.5 XLM in stroops
);
```

#### `claim_winnings()`
Claim winnings from resolved markets.

**Signature:**
```rust
pub fn claim_winnings(
    env: Env,
    user: Address,
    market_id: Symbol,
) -> Result<i128, Error>
```

**Returns:** Amount claimed in stroops

---

## 📊 Data Structures

### Market
Core market data structure containing all market information.

```rust
pub struct Market {
    pub id: Symbol,
    pub question: String,
    pub outcomes: Vec<String>,
    pub creator: Address,
    pub created_at: u64,
    pub deadline: u64,
    pub resolved: bool,
    pub winning_outcome: Option<String>,
    pub total_stake: i128,
    pub oracle_config: OracleConfig,
}
```

### Vote
Represents a user's vote on a market.

```rust
pub struct Vote {
    pub voter: Address,
    pub market_id: Symbol,
    pub outcome: String,
    pub stake: i128,
    pub timestamp: u64,
    pub claimed: bool,
}
```

### OracleConfig
Configuration for oracle integration.

```rust
pub struct OracleConfig {
    pub provider: OracleProvider,
    pub feed_id: String,
    pub threshold: i128,
    pub timeout_seconds: u64,
}
```

---

## ⚠️ Error Codes

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

### Validation Errors (300-399)
- **300**: `InvalidInput` - General input validation failure
- **301**: `InvalidMarket` - Market parameters invalid
- **302**: `InvalidVote` - Vote parameters invalid
- **303**: `InvalidDispute` - Dispute parameters invalid

### System Errors (400-499)
- **400**: `ContractNotInitialized` - Contract requires initialization
- **401**: `AdminRequired` - Admin privileges required
- **402**: `ContractPaused` - Contract is paused
- **403**: `InsufficientBalance` - Account balance too low

---

## 💡 Integration Examples

### Basic Market Creation and Voting

```typescript
import { Contract, Keypair, Networks } from '@stellar/stellar-sdk';

// Initialize contract
const contract = new Contract(contractId);

// Create market
const marketId = await contract.create_market(
    adminKeypair.publicKey(),
    "Will Ethereum reach $5,000 by Q2 2025?",
    ["Yes", "No"],
    120, // 120 days
    {
        provider: "Reflector",
        feed_id: "ETH/USD",
        threshold: 5000000000, // $5,000 in stroops
        timeout_seconds: 3600
    }
);

// Vote on market
await contract.vote(
    userKeypair.publicKey(),
    marketId,
    "Yes",
    10000000 // 1 XLM stake
);

// Check market status
const market = await contract.get_market(marketId);
console.log(`Market: ${market.question}`);
console.log(`Total stake: ${market.total_stake} stroops`);

// Claim winnings (after resolution)
const winnings = await contract.claim_winnings(
    userKeypair.publicKey(),
    marketId
);
console.log(`Claimed: ${winnings} stroops`);
```

### Batch Operations

```typescript
// Create multiple markets
const markets = await Promise.all([
    contract.create_market(admin, "BTC > $100K?", ["Yes", "No"], 90, btcConfig),
    contract.create_market(admin, "ETH > $5K?", ["Yes", "No"], 90, ethConfig),
    contract.create_market(admin, "SOL > $200?", ["Yes", "No"], 90, solConfig)
]);

// Vote on multiple markets
await Promise.all(
    markets.map(marketId => 
        contract.vote(user, marketId, "Yes", 5000000)
    )
);
```

---

## 🆘 Troubleshooting Guide

### Common Issues and Solutions

#### 🔧 Deployment Issues

**Problem: Contract deployment fails with "Insufficient Balance"**
```bash
Error: Account has insufficient balance for transaction
```
**Solution:**
```bash
# Check account balance
soroban config identity address
soroban balance --id <your-address> --network mainnet

# Fund account if needed (minimum 100 XLM recommended)
# Use Stellar Laboratory or send from funded account
```

**Problem: WASM file not found during deployment**
```bash
Error: No such file or directory: target/wasm32-unknown-unknown/release/predictify_hybrid.wasm
```
**Solution:**
```bash
# Ensure contract is built first
cd contracts/predictify-hybrid
make build

# Verify WASM file exists
ls -la target/wasm32-unknown-unknown/release/
```

#### 🔮 Oracle Integration Issues

**Problem: Oracle results not being accepted**
```rust
Error: InvalidOracleConfig (201)
```
**Solution:**
```bash
# Verify oracle configuration
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_oracle_config \
  --network mainnet

# Update oracle configuration if needed
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn update_oracle_config \
  --arg provider=Reflector \
  --arg feed_id="BTC/USD" \
  --network mainnet
```

**Problem: Oracle price feeds timing out**
```rust
Error: OracleUnavailable (200)
```
**Solution:**
1. Check oracle service status
2. Verify network connectivity
3. Implement fallback oracle providers
4. Add retry logic with exponential backoff

#### 🗳️ Voting and Market Issues

**Problem: User unable to vote**
```rust
Error: MarketClosed (102)
```
**Solution:**
```bash
# Check market status and deadline
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_market \
  --arg market_id="BTC_100K" \
  --network mainnet

# Extend market if authorized and appropriate
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn extend_market \
  --arg market_id="BTC_100K" \
  --arg additional_days=7 \
  --network mainnet
```

**Problem: Insufficient stake error**
```rust
Error: InsufficientStake (107)
```
**Solution:**
```bash
# Check minimum stake requirements
echo "Minimum vote stake: 1,000,000 stroops (0.1 XLM)"
echo "Minimum dispute stake: 100,000,000 stroops (10 XLM)"

# Verify user balance
soroban balance --id <user-address> --network mainnet
```

#### 🏛️ Dispute Resolution Issues

**Problem: Dispute submission rejected**
```rust
Error: DisputeVotingNotAllowed (406)
```
**Solution:**
1. Verify market is in resolved state
2. Check dispute window timing (24-48 hours after resolution)
3. Ensure sufficient dispute stake
4. Verify user hasn't already disputed

**Problem: Dispute threshold too high**
```rust
Error: ThresholdExceedsMaximum (412)
```
**Solution:**
```bash
# Check current dispute threshold
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_dispute_threshold \
  --arg market_id="BTC_100K" \
  --network mainnet

# Admin can adjust if necessary
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn update_dispute_threshold \
  --arg market_id="BTC_100K" \
  --arg new_threshold=50000000 \
  --network mainnet
```

#### 💰 Fee and Payout Issues

**Problem: Fee collection fails**
```rust
Error: NoFeesToCollect (415)
```
**Solution:**
```bash
# Check if fees are available
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_collectable_fees \
  --arg market_id="BTC_100K" \
  --network mainnet

# Ensure market is resolved and fees haven't been collected
```

**Problem: User cannot claim winnings**
```rust
Error: NothingToClaim (105)
```
**Solution:**
1. Verify user voted on winning outcome
2. Check market resolution status
3. Ensure user hasn't already claimed
4. Verify market dispute period has ended

### 🔍 Debugging Tools

#### Contract State Inspection
```bash
# Get complete market information
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_market_analytics \
  --arg market_id="BTC_100K" \
  --network mainnet

# Check user voting history
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_user_votes \
  --arg user=<address> \
  --network mainnet

# Inspect contract configuration
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn get_config \
  --network mainnet
```

#### Transaction Analysis
```bash
# View transaction details
soroban events --id $CONTRACT_ID --network mainnet

# Check specific transaction
soroban transaction --hash <tx_hash> --network mainnet
```

#### Log Analysis
```bash
# Enable verbose logging
export RUST_LOG=debug

# Run with detailed output
soroban contract invoke \
  --id $CONTRACT_ID \
  --fn vote \
  --arg market_id="BTC_100K" \
  --arg outcome="yes" \
  --arg stake=5000000 \
  --network mainnet \
  --verbose
```

---

## 📞 Support and Resources

### Error Code Reference
- **100-199**: User operation errors - Check user permissions and market state
- **200-299**: Oracle errors - Verify oracle configuration and connectivity
- **300-399**: Validation errors - Check input parameters and formats
- **400-499**: System errors - Contact support for system-level issues

### Support Channels
1. **GitHub Issues**: [Report bugs and request features](https://github.com/predictify/contracts/issues)
2. **Discord Support**: [#technical-support channel](https://discord.gg/predictify)
3. **Developer Forum**: [Technical discussions](https://forum.predictify.io)
4. **Email Support**: technical-support@predictify.io

### Before Contacting Support
1. Check this troubleshooting guide
2. Search existing GitHub issues
3. Verify your environment and configuration
4. Collect relevant error messages and transaction hashes
5. Note your contract version and network

### Additional Resources
- [Stellar Soroban Documentation](https://soroban.stellar.org/)
- [Stellar SDK Documentation](https://stellar.github.io/js-stellar-sdk/)
- [Predictify GitHub Repository](https://github.com/predictify/contracts)
- [Community Examples](https://github.com/predictify/examples)

---

**Last Updated:** 2025-01-15  
**API Version:** v1.0.0  
**Documentation Version:** 1.0
