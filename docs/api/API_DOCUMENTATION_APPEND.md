
---

## Module Catalog

### Admin Module

**Purpose**: Administrative roles, permissions, and authorization

**Key Types:**
- `AdminRole`: Role-based access control (Admin, Moderator, Viewer)
- `AdminPermission`: Granular permission definitions

**Key Functions:**
- `initialize_admin(env, admin_addr)`: Set contract admin
- `check_admin(env, addr)`: Verify if address is admin
- `add_admin(env, new_admin)`: Add additional admin
- `revoke_admin(env, admin_addr)`: Remove admin privileges

---

### Balances Module

**Purpose**: User balance tracking and asset management

**Key Types:**
- `Balance`: User balance for specific asset
- `ReflectorAsset`: Asset enumeration (BTC, ETH, XLM, etc.)

**Key Structs:**
```rust
#[contracttype]
pub struct Balance {
    pub user: Address,
    pub asset: ReflectorAsset,
    pub amount: i128,
    pub last_updated: u64,
}
```

**Operations:**
- Deposit/withdraw asset balances
- Query user balances
- Transfer between accounts
- Validate sufficient balance

---

### Bets Module

**Purpose**: Bet placement, management, and payout calculations

**Key Struct:**
```rust
#[contracttype]
pub struct Bet {
    pub user: Address,
    pub market_id: Symbol,
    pub outcome: String,
    pub amount: i128,
    pub placed_at: u64,
}
```

**Operations:**
- Place bets on outcomes
- Track active bets
- Calculate payouts
- Handle bet cancellation

**Constraints:**
- Minimum: 0.1 XLM
- Maximum: 10,000 XLM
- One bet per user per market

---

### Markets Module

**Purpose**: Core market state management

**Key Struct:**
```rust
#[contracttype]
pub struct Market {
    pub admin: Address,
    pub question: String,
    pub outcomes: Vec<String>,
    pub end_time: u64,
    pub oracle_config: OracleConfig,
    pub has_fallback: bool,
    pub fallback_oracle_config: OracleConfig,
    pub resolution_timeout: u64,
    pub oracle_result: Option<String>,
    pub votes: Map<Address, String>,
    pub total_staked: i128,
    pub dispute_stakes: Map<Address, i128>,
    pub stakes: Map<Address, i128>,
    pub claimed: Map<Address, bool>,
    pub winning_outcomes: Option<Vec<String>>,
    pub fee_collected: bool,
    pub state: MarketState,
    pub total_extension_days: u32,
    pub max_extension_days: u32,
}
```

**Operations:**
- Store and retrieve markets
- Update market state
- Track participants and stakes
- Manage disputes and extensions

---

### Disputes Module

**Purpose**: Dispute filing, voting, and resolution

**Key Struct:**
```rust
#[contracttype]
pub struct Dispute {
    pub market_id: Symbol,
    pub initiator: Address,
    pub reason: String,
    pub filed_at: u64,
    pub votes_for: u32,
    pub votes_against: u32,
    pub status: DisputeStatus,
}
```

**Operations:**
- File dispute on resolution
- Vote on disputes
- Resolve dispute via consensus
- Distribute dispute rewards

**Timeline:**
- 24-hour dispute filing window (post-resolution)
- 72-hour voting period
- Community consensus required (> 50% vote)

---

### Oracles Module

**Purpose**: Oracle integration and price feeds

**Supported Providers:**
- **Reflector**: Primary oracle (Stellar-native)
- **Pyth**: High-frequency oracle (placeholder)

**Key Struct:**
```rust
#[contracttype]
pub struct OracleConfig {
    pub oracle_type: OracleProvider,
    pub oracle_contract: Address,
    pub asset_code: Option<String>,
    pub threshold_value: Option<i128>,
}
```

**Operations:**
- Fetch latest prices
- Validate price freshness
- Handle oracle failures
- Fallback to secondary oracle

---

### Fees Module

**Purpose**: Fee calculation, collection, and distribution

**Fee Structure:**
- Platform fee: 2-10% of winnings (configurable)
- Applied during payout distribution
- Collected in designated account

**Functions:**
- `calculate_fee(amount)`: Compute fee amount
- `collect_platform_fee(env, market_id)`: Deduct fees
- `withdraw_collected_fees(env, admin)`: Admin withdrawal

---

### Voting Module

**Purpose**: User voting on market outcomes and disputes

**Features:**
- One vote per user per market
- One vote per user per dispute
- Vote weighting (optional by stake)
- Consensus calculation

**Functions:**
- `vote(env, user, market_id, outcome)`: Place vote
- `get_votes(env, market_id)`: Retrieve votes
- `calculate_consensus(env, market_id)`: Determine consensus

---

### Batch Operations Module

**Purpose**: Efficient multi-operation execution

**Functions:**
- `batch_place_bets(env, user, operations)`: Place multiple bets
- `batch_claim_winnings(env, user, market_ids)`: Claim from multiple markets
- `batch_vote(env, user, votes)`: Vote on multiple markets

**Benefits:**
- Single transaction for multiple operations
- Reduced gas costs
- Atomic execution (all-or-nothing)

---

### Circuit Breaker Module

**Purpose**: Emergency safety mechanism to pause operations

**States:**
- `Closed`: Normal operation
- `Open`: Paused, no operations allowed
- `HalfOpen`: Testing if conditions normalized

**Triggers:**
- High error rate threshold
- Oracle unavailability
- Unexpected system state

**Functions:**
- `trigger_circuit_breaker()`: Activate breaker
- `reset_circuit_breaker()`: Return to normal
- `query_breaker_status()`: Check current state

---

### Queries Module

**Purpose**: Read-only query operations

**Key Functions:**
- `query_market(env, market_id)`: Market details
- `query_user_bets(env, user)`: User's active bets
- `query_market_outcome_odds(env, market_id)`: Current odds
- `query_platform_fee(env)`: Current fee percentage

**Characteristics:**
- No state modification
- No authorization required
- Instant execution
- Gas efficient

---

### Validation Module

**Purpose**: Input validation and constraint checking

**Validations:**
- Market parameters (duration, outcomes, question)
- Bet amounts (min/max constraints)
- Oracle configurations
- User addresses and permissions
- Timestamps and deadlines

**Functions:**
- `validate_market_creation(env, params)`: Market validation
- `validate_bet_placement(env, user, amount)`: Bet validation
- `validate_oracle_config(env, config)`: Oracle validation

---

---

## Error Reference

All errors are defined in `err.rs` with codes 100-504.

### User Operation Errors (100-112)

| Code | Name | Meaning | Operations |
|------|------|---------|-----------|
| 100 | `Unauthorized` | Caller lacks required permissions | Any admin-only function |
| 101 | `MarketNotFound` | Market ID doesn't exist | All market operations |
| 102 | `MarketClosed` | Market deadline passed | `vote`, `place_bet` |
| 103 | `MarketResolved` | Market already resolved | `place_bet`, `vote` |
| 104 | `MarketNotResolved` | Market not yet resolved | `claim_winnings` |
| 105 | `NothingToClaim` | User has no winnings | `claim_winnings` |
| 106 | `AlreadyClaimed` | Already claimed from market | `claim_winnings` |
| 107 | `InsufficientStake` | Bet below minimum | `place_bet` |
| 108 | `InvalidOutcome` | Outcome not valid | `place_bet`, `vote`, resolve |
| 109 | `AlreadyVoted` | User already voted | `vote` |
| 110 | `AlreadyBet` | User already bet | `place_bet` |
| 111 | `BetsAlreadyPlaced` | Can't update market | `update_market` |
| 112 | `InsufficientBalance` | Insufficient funds | `place_bet`, `withdraw` |

### Oracle Errors (200-208)

| Code | Name | Meaning | Operations |
|------|------|---------|-----------|
| 200 | `OracleUnavailable` | Oracle service unreachable | `resolve_market` |
| 201 | `InvalidOracleConfig` | Oracle config invalid | `create_market` |
| 202 | `OracleStale` | Oracle data too old | `resolve_market` |
| 203 | `OracleNoConsensus` | Multiple oracles disagree | `resolve_market` |
| 204 | `OracleVerified` | Result already verified | `resolve_market` |
| 205 | `MarketNotReady` | Can't verify yet | `resolve_market` |
| 206 | `FallbackOracleUnavailable` | Fallback oracle down | `resolve_market` |
| 208 | `OracleConfidenceTooWide` | Confidence below threshold | `resolve_market` |

### Validation Errors (300-304)

| Code | Name | Meaning | Operations |
|------|------|---------|-----------|
| 300 | `InvalidQuestion` | Question empty/invalid | `create_market` |
| 301 | `InvalidOutcomes` | Outcomes < 2 or duplicates | `create_market` |
| 302 | `InvalidDuration` | Duration outside 1-365 days | `create_market` |
| 303 | `InvalidThreshold` | Threshold out of range | Configuration |
| 304 | `InvalidComparison` | Unsupported operator | Oracle config |

### General Errors (400-418)

| Code | Name | Meaning | Operations |
|------|------|---------|-----------|
| 400 | `InvalidState` | Unexpected state | Internal state mismatch |
| 401 | `InvalidInput` | Invalid parameters | All functions |
| 402 | `InvalidFeeConfig` | Fee outside 0-10% | `set_platform_fee` |
| 403 | `ConfigNotFound` | Config missing | Internal operations |
| 404 | `AlreadyDisputed` | Dispute already filed | `dispute_market` |
| 405 | `DisputeVoteExpired` | Dispute window closed | `vote_dispute` |
| 406 | `DisputeVoteDenied` | Can't vote now | `vote_dispute` |
| 407 | `DisputeAlreadyVoted` | User already voted | `vote_dispute` |
| 408 | `DisputeCondNotMet` | Requirements not met | `resolve_dispute` |
| 409 | `DisputeFeeFailed` | Fee distribution failed | Dispute resolution |
| 410 | `DisputeError` | Generic dispute error | Dispute operations |
| 413 | `FeeAlreadyCollected` | Fee already deducted | Payout operations |
| 414 | `NoFeesToCollect` | No fees available | `withdraw_fees` |
| 415 | `InvalidExtensionDays` | Extension invalid | `extend_market` |
| 416 | `ExtensionDenied` | Extension not allowed | `extend_market` |
| 417 | `GasBudgetExceeded` | Operation too expensive | Any operation |
| 418 | `AdminNotSet` | Admin not initialized | After fresh deployment |

### Circuit Breaker Errors (500-504)

| Code | Name | Meaning | Operations |
|------|------|---------|-----------|
| 500 | `CBNotInitialized` | Breaker not initialized | Breaker operations |
| 501 | `CBAlreadyOpen` | Breaker already open | `trigger_breaker` |
| 502 | `CBNotOpen` | Breaker not open | `reset_breaker` |
| 503 | `CBOpen` | Operations paused | All user operations |
| 504 | `CBError` | Generic breaker error | Breaker operations |

---

---

## Types Reference

### Market-Related Types

#### `Market` (Main Market State)

```rust
#[contracttype]
pub struct Market {
    pub admin: Address,                           // Market creator
    pub question: String,                          // Prediction question
    pub outcomes: Vec<String>,                     // Possible outcomes
    pub end_time: u64,                            // Unix timestamp end
    pub oracle_config: OracleConfig,              // Primary oracle
    pub has_fallback: bool,                       // Fallback available
    pub fallback_oracle_config: OracleConfig,     // Backup oracle
    pub resolution_timeout: u64,                   // Timeout for resolution
    pub oracle_result: Option<String>,            // Resolved outcome
    pub votes: Map<Address, String>,              // User votes
    pub total_staked: i128,                       // Total wagered
    pub dispute_stakes: Map<Address, i128>,       // Dispute stakes
    pub stakes: Map<Address, i128>,               // User stakes
    pub claimed: Map<Address, bool>,              // Claim flags
    pub winning_outcomes: Option<Vec<String>>,    // Winner(s)
    pub fee_collected: bool,                      // Fee deducted
    pub state: MarketState,                       // Current state
    pub total_extension_days: u32,                // Days extended
    pub max_extension_days: u32,                  // Max allowed
}
```

#### `MarketState` (State Enum)

```rust
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarketState {
    Active,      // Accepting votes/bets
    Ended,       // Deadline passed, awaiting resolution
    Disputed,    // Under dispute
    Resolved,    // Outcome determined, awaiting payouts
    Closed,      // All payouts distributed
    Cancelled,   // Market cancelled, stakes returned
}
```

**State Transitions:**
```
Active → Ended → Disputed → Resolved → Closed
          ↓
       (dispute)
          ↓
       Disputed → Resolved
Active (cancellation) → Cancelled
Active (override) → Resolved
```

#### `MarketStats` (Market Statistics)

```rust
#[contracttype]
pub struct MarketStats {
    pub market_id: Symbol,
    pub total_staked: i128,
    pub participant_count: u32,
    pub outcome_stakes: Map<String, i128>,
    pub outcome_vote_counts: Map<String, u32>,
    pub volume: i128,
    pub created_at: u64,
    pub ended_at: Option<u64>,
    pub resolved_at: Option<u64>,
}
```

---

### Oracle-Related Types

#### `OracleProvider` (Supported Oracles)

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleProvider {
    Reflector,     // Stellar-native oracle (primary)
    Pyth,          // High-frequency oracle (future)
    BandProtocol,  // Decentralized oracle (future)
    DIA,           // Multi-chain oracle (future)
}
```

**Current Status:**
- ✅ Reflector (production ready)
- ⏳ Pyth, BandProtocol, DIA (not yet on Stellar)

#### `OracleConfig` (Oracle Configuration)

```rust
#[contracttype]
pub struct OracleConfig {
    pub oracle_type: OracleProvider,     // Which oracle
    pub oracle_contract: Address,        // Oracle contract address
    pub asset_code: Option<String>,      // Asset code (BTC, ETH, etc.)
    pub threshold_value: Option<i128>,   // Price threshold for resolution
    pub freshness_threshold: Option<u64>, // Max age of price data
}
```

#### `OracleResult` (Oracle Response)

```rust
#[contracttype]
pub struct OracleResult {
    pub price: i128,
    pub timestamp: u64,
    pub asset: String,
    pub source: OracleProvider,
    pub confidence: Option<i128>,  // Percentage (0-10000)
}
```

---

### Balance & Asset Types

#### `Balance` (User Balance)

```rust
#[contracttype]
pub struct Balance {
    pub user: Address,
    pub asset: ReflectorAsset,
    pub amount: i128,
    pub last_updated: u64,
}
```

#### `ReflectorAsset` (Supported Assets)

```rust
#[contracttype]
pub enum ReflectorAsset {
    BTC,   // Bitcoin
    ETH,   // Ethereum
    XLM,   // Stellar Lumens
    USDC,  // USD Coin
    // ... additional assets
}
```

**Standard Precisions:**
- BTC/ETH: 7 decimals (e.g., 100_000_000 = 1.00000000)
- XLM: 7 decimals
- USDC: 6 decimals

---

### Voting & Dispute Types

#### `Vote` (User Vote Record)

```rust
#[contracttype]
pub struct Vote {
    pub user: Address,
    pub market_id: Symbol,
    pub outcome: String,
    pub timestamp: u64,
    pub weight: Option<i128>,  // Stake-weighted (optional)
}
```

#### `Dispute` (Dispute Record)

```rust
#[contracttype]
pub struct Dispute {
    pub market_id: Symbol,
    pub initiator: Address,
    pub reason: String,
    pub filed_at: u64,
    pub votes_for: u32,
    pub votes_against: u32,
    pub status: DisputeStatus,
    pub resolved_at: Option<u64>,
    pub resolution: Option<String>,
}
```

#### `DisputeStatus` (Dispute State)

```rust
#[contracttype]
pub enum DisputeStatus {
    Pending,    // Awaiting votes
    VoteClosed, // Voting ended
    Approved,   // Resolved in favor
    Rejected,   // Resolved against
    Withdrawn,  // Initiator withdrew
}
```

---

### Fee & Distribution Types

#### `FeeRecord` (Fee Collection)

```rust
#[contracttype]
pub struct FeeRecord {
    pub market_id: Symbol,
    pub fee_percentage: i128,
    pub fee_amount: i128,
    pub collected_at: u64,
    pub withdrawn: bool,
}
```

#### `Payout` (Winning Calculation)

```rust
#[contracttype]
pub struct Payout {
    pub user: Address,
    pub market_id: Symbol,
    pub gross_amount: i128,
    pub fee_amount: i128,
    pub net_amount: i128,
    pub distributed_at: Option<u64>,
}
```

---

### Utility Types

#### `ContractMetadata` (Version Info)

```rust
#[contracttype]
pub struct ContractMetadata {
    pub version: String,           // "1.2.3-beta1"
    pub deployment_time: u64,
    pub last_upgrade: Option<u64>,
    pub current_admin: Address,
    pub platform_fee: i128,
}
```

---

---

## Usage Examples

### Example 1: Complete Market Lifecycle (Create → Bet → Resolve → Claim)

```rust
use soroban_sdk::{Env, Address, String, Symbol, Vec};
use predictify_hybrid::{PredictifyHybrid, OracleConfig, OracleProvider, ReflectorAsset};

fn example_market_lifecycle() {
    let env = Env::default();
    
    // Step 1: Initialize contract
    let admin = Address::generate(&env);
    PredictifyHybrid::initialize(env.clone(), admin.clone(), Some(2)); // 2% fee
    
    // Step 2: Create a market
    let question = String::from_str(&env, "Will Bitcoin reach $100k by Dec 2024?");
    let mut outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "Yes"));
    outcomes.push_back(String::from_str(&env, "No"));
    
    let oracle_config = OracleConfig {
        oracle_type: OracleProvider::Reflector,
        oracle_contract: Address::generate(&env),
        asset_code: Some(String::from_str(&env, "BTC")),
        threshold_value: Some(100_000),
        freshness_threshold: None,
    };
    
    let market_id = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        question.clone(),
        outcomes.clone(),
        30,  // 30 days
        oracle_config.clone(),
        None,  // No fallback
        3600,  // 1 hour timeout
    );
    
    // Step 3: Deposit funds (user prepares to bet)
    let user = Address::generate(&env);
    user.require_auth();
    
    let balance = PredictifyHybrid::deposit(
        env.clone(),
        user.clone(),
        ReflectorAsset::XLM,
        1_000_000_000,  // 100 XLM (7 decimals)
    ).expect("Deposit failed");
    
    println!("User balance: {}", balance.amount);
    
    // Step 4: Place a bet
    PredictifyHybrid::place_bet(
        env.clone(),
        user.clone(),
        market_id.clone(),
        String::from_str(&env, "Yes"),
        500_000_000,  // 50 XLM
    ).expect("Bet placement failed");
    
    println!("Bet placed successfully on 'Yes'");
    
    // Step 5: Wait for market deadline...
    // (In real scenario: advance time via env)
    
    // Step 6: Resolve market via oracle
    PredictifyHybrid::resolve_market_oracle(
        env.clone(),
        market_id.clone(),
        String::from_str(&env, "Yes"),  // Outcome determined by oracle
    ).expect("Resolution failed");
    
    println!("Market resolved with 'Yes' outcome");
    
    // Step 7: Claim winnings
    let winnings = PredictifyHybrid::claim_winnings(
        env.clone(),
        user.clone(),
        market_id.clone(),
    ).expect("Claim failed");
    
    println!("Winnings claimed: {} XLM", winnings / 10_000_000);
    
    // Step 8: Withdraw funds
    let final_balance = PredictifyHybrid::withdraw(
        env.clone(),
        user.clone(),
        ReflectorAsset::XLM,
        winnings,
    ).expect("Withdrawal failed");
    
    println!("Final balance: {} XLM", final_balance.amount / 10_000_000);
}
```

---

### Example 2: Dispute Resolution Flow

```rust
use soroban_sdk::{Env, Address, String, Symbol};
use predictify_hybrid::PredictifyHybrid;

fn example_dispute_flow() {
    let env = Env::default();
    
    // Setup: Market created and resolved
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    
    // ... (create market, place bets, advance time, resolve market)
    // Assume market_id and resolved outcome exist
    
    let market_id = Symbol::new(&env, "mkt_abc_123");
    
    // Step 1: User files dispute
    user1.require_auth();
    PredictifyHybrid::dispute_market(
        env.clone(),
        user1.clone(),
        market_id.clone(),
        String::from_str(&env, "Oracle price was manipulated"),
    ).expect("Dispute filed successfully");
    
    println!("Dispute filed by user1");
    
    // Step 2: Community voting on dispute
    user2.require_auth();
    PredictifyHybrid::vote_dispute(
        env.clone(),
        user2.clone(),
        market_id.clone(),
        true,  // Vote in favor of dispute (reverse resolution)
    ).expect("Vote recorded");
    
    user3.require_auth();
    PredictifyHybrid::vote_dispute(
        env.clone(),
        user3.clone(),
        market_id.clone(),
        false,  // Vote against dispute (keep resolution)
    ).expect("Vote recorded");
    
    println!("Dispute votes recorded");
    
    // Step 3: Resolve dispute
    let dispute_result = PredictifyHybrid::resolve_dispute(
        env.clone(),
        market_id.clone(),
    ).expect("Dispute resolved");
    
    println!("Dispute resolved in favor: {}", dispute_result.approved);
    
    // Step 4: Distribute dispute rewards
    if dispute_result.approved {
        println!("Resolution reversed, new payouts calculated");
        // Winnings are recalculated and redistributed
    }
}
```

---

### Example 3: Multi-Outcome Market with Batch Operations

```rust
use soroban_sdk::{Env, Address, String, Symbol, Vec};
use predictify_hybrid::{PredictifyHybrid, OracleConfig, OracleProvider};

fn example_multi_outcome_batch() {
    let env = Env::default();
    
    // Step 1: Create a 3-outcome market (soccer match)
    let admin = Address::generate(&env);
    PredictifyHybrid::initialize(env.clone(), admin.clone(), Some(2));
    
    let question = String::from_str(&env, "Champions League Final - Match Winner?");
    let mut outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "Team A"));
    outcomes.push_back(String::from_str(&env, "Team B"));
    outcomes.push_back(String::from_str(&env, "Draw"));
    
    let oracle_config = OracleConfig {
        oracle_type: OracleProvider::Reflector,
        oracle_contract: Address::generate(&env),
        asset_code: Some(String::from_str(&env, "MATCH_RESULT")),
        threshold_value: None,
        freshness_threshold: None,
    };
    
    let market_id = PredictifyHybrid::create_market(
        env.clone(),
        admin.clone(),
        question,
        outcomes,
        14,  // 2 weeks
        oracle_config,
        None,
        7200,  // 2 hours
    );
    
    println!("3-outcome market created: {}", market_id.to_string());
    
    // Step 2: Batch place multiple bets
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    
    user1.require_auth();
    PredictifyHybrid::place_bet(
        env.clone(),
        user1.clone(),
        market_id.clone(),
        String::from_str(&env, "Team A"),
        100_000_000,  // 10 XLM
    ).expect("Bet 1 failed");
    
    user2.require_auth();
    PredictifyHybrid::place_bet(
        env.clone(),
        user2.clone(),
        market_id.clone(),
        String::from_str(&env, "Team B"),
        150_000_000,  // 15 XLM
    ).expect("Bet 2 failed");
    
    user3.require_auth();
    PredictifyHybrid::place_bet(
        env.clone(),
        user3.clone(),
        market_id.clone(),
        String::from_str(&env, "Draw"),
        50_000_000,  // 5 XLM
    ).expect("Bet 3 failed");
    
    println!("Batch bets placed for all outcomes");
    
    // Step 3: Market resolves with Draw outcome
    PredictifyHybrid::resolve_market_oracle(
        env.clone(),
        market_id.clone(),
        String::from_str(&env, "Draw"),
    ).expect("Resolution failed");
    
    println!("Market resolved with 'Draw' outcome");
    
    // Step 4: Winners claim winnings
    let user3_winnings = PredictifyHybrid::claim_winnings(
        env.clone(),
        user3.clone(),
        market_id.clone(),
    ).expect("Claim failed");
    
    println!("User 3 claims: {} XLM", user3_winnings / 10_000_000);
    
    // Users 1 and 2 would see "NothingToClaim" error since they didn't bet on Draw
}
```

---

## API Conventions

### Response Types

**Success Responses:**
- Void operations return nothing: `pub fn vote(...) -> Result<(), Error>`
- Value-returning operations return wrapped value: `pub fn claim_winnings(...) -> Result<i128, Error>`

### Parameter Conventions

- **Addresses**: Always use `Address` type, never raw bytes
- **Amounts**: Denominated in lowest unit (7 decimals for XLM/BTC/ETH)
- **Timestamps**: Unix epoch seconds (u64)
- **Symbols**: Market IDs and event IDs are `Symbol` type

### Authorization

- All admin functions require `admin.require_auth()`
- All user functions require the caller's authorization
- Authorization is verified via Soroban's built-in auth mechanism

### Gas Efficiency

- Batch operations significantly reduce gas costs
- Queries don't consume gas (read-only)
- Market creation has highest gas cost
- Bulk operations cheaper than individual operations

---

## Integration Checklist

- [ ] Import and initialize contract: `PredictifyHybrid::initialize(...)`
- [ ] Set up oracle configuration with valid provider
- [ ] Create test markets and verify state changes
- [ ] Test bet placement with minimum and maximum amounts
- [ ] Verify error handling for all error cases
- [ ] Implement event listeners for contract events
- [ ] Test dispute flow with multiple voters
- [ ] Validate multi-outcome market resolution
- [ ] Implement UI for balance management
- [ ] Add market monitoring/analytics integration

---

**Document Version:** 2.0  
**Last Updated:** March 25, 2026  
**Contract Version:** 1.2.3
