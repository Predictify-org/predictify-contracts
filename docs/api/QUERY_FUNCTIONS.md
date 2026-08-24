# Query Functions Documentation

## Overview

The Predictify Hybrid contract provides a comprehensive set of **read-only query functions** for retrieving event information, bet details, and contract state. All query functions are:

- **Gas-Efficient**: Pure read operations with no state modifications
- **Secure**: Full input validation on all parameters
- **Well-Documented**: Comprehensive examples and usage patterns
- **Tested**: Full unit and integration test coverage
- **Structured**: Return strongly-typed responses for easy parsing

## Query Categories

### 1. Event/Market Information Queries

Functions to retrieve detailed information about prediction markets.

#### `query_event_details(market_id: Symbol) -> EventDetailsQuery`

Returns comprehensive market information including question, outcomes, status, and statistics.

**Returns:**
```rust
struct EventDetailsQuery {
    market_id: Symbol,
    question: String,
    outcomes: Vec<String>,
    created_at: u64,
    end_time: u64,
    status: MarketStatus,
    oracle_provider: String,
    feed_id: String,
    total_staked: i128,
    winning_outcome: Option<String>,
    oracle_result: Option<String>,
    participant_count: u32,
    vote_count: u32,
    admin: Address,
}
```

**Example Usage:**
```rust
// Query market details
let details = contract.query_event_details(env, market_id)?;

// Access market information
println!("Question: {}", details.question);
println!("Status: {:?}", details.status);
println!("Total Staked: {} XLM", details.total_staked / 10_000_000);
println!("Participants: {}", details.participant_count);

// Check if market is resolved
if let Some(outcome) = details.winning_outcome {
    println!("Winning outcome: {}", outcome);
}
```

**Use Cases:**
- Display market information on user interface
- Show market status to participants
- Get oracle configuration details
- Monitor market participation

---

#### `query_event_status(market_id: Symbol) -> (MarketStatus, u64)`

Lightweight query for quick status checks without full market details.

**Returns:**
- `MarketStatus`: Current market status (Active, Ended, Resolved, Disputed, Closed, Cancelled)
- `u64`: Market end time (Unix timestamp)

**Example Usage:**
```rust
// Quick status check
let (status, end_time) = contract.query_event_status(env, market_id)?;

match status {
    MarketStatus::Active => println!("Market is active"),
    MarketStatus::Ended => println!("Voting period ended"),
    MarketStatus::Resolved => println!("Market resolved"),
    MarketStatus::Disputed => println!("Market under dispute"),
    MarketStatus::Closed => println!("Market closed"),
    MarketStatus::Cancelled => println!("Market cancelled"),
}

let time_remaining = end_time - current_time;
println!("Time remaining: {} seconds", time_remaining);
```

**Use Cases:**
- Quick polling for market status updates
- Decision logic based on market state
- Determine if market is still accepting votes
- Check time remaining until market ends

---

#### `get_all_markets() -> Vec<Symbol>`

Returns list of all market IDs created on the contract.

**Returns:**
- `Vec<Symbol>`: Vector of all market identifiers

**Example Usage:**
```rust
// Get all markets
let all_markets = contract.get_all_markets(env)?;

println!("Total markets: {}", all_markets.len());

// Iterate through markets
for market_id in all_markets {
    let details = contract.query_event_details(env, market_id)?;
    println!("Market: {}", details.question);
}

// Implement pagination
let page_size = 10;
let page_num = 0;
let start = page_num * page_size;
let end = (start + page_size).min(all_markets.len());
let page_markets = &all_markets[start..end];
```

**Use Cases:**
- Implement market discovery UI
- Pagination through markets
- Generate market lists and dashboards
- Find markets by iteration

---

### 2. User Bet Queries

Functions to retrieve user-specific betting and voting information.

#### `query_user_bet(user: Address, market_id: Symbol) -> UserBetQuery`

Returns comprehensive information about a user's participation in a specific market.

**Returns:**
```rust
struct UserBetQuery {
    user: Address,
    market_id: Symbol,
    outcome: String,
    stake_amount: i128,
    voted_at: u64,
    is_winning: bool,
    has_claimed: bool,
    potential_payout: i128,
    dispute_stake: i128,
}
```

**Example Usage:**
```rust
// Query user's bet
let bet = contract.query_user_bet(env, user, market_id)?;

// Display bet information
println!("Voted for: {}", bet.outcome);
println!("Staked: {} XLM", bet.stake_amount / 10_000_000);
println!("Voted at: {}", bet.voted_at);

// Check payout eligibility
if bet.is_winning && !bet.has_claimed {
    println!("Potential payout: {} XLM", bet.potential_payout / 10_000_000);
    // Prompt user to claim winnings
}

// Check dispute status
if bet.dispute_stake > 0 {
    println!("Dispute stake: {} XLM", bet.dispute_stake / 10_000_000);
}
```

**Use Cases:**
- Show user's specific bets on a market
- Display vote and stake information
- Calculate potential payouts
- Track dispute participation
- Monitor claim status

---

#### `query_user_bets(user: Address) -> MultipleBetsQuery`

Returns all of a user's bets across all markets with aggregated statistics.

**Returns:**
```rust
struct MultipleBetsQuery {
    bets: Vec<UserBetQuery>,
    total_stake: i128,
    total_potential_payout: i128,
    winning_bets: u32,
}
```

**Example Usage:**
```rust
// Get all user bets
let all_bets = contract.query_user_bets(env, user)?;

// Display portfolio
println!("Portfolio Overview:");
println!("Total bets: {}", all_bets.bets.len());
println!("Total staked: {} XLM", all_bets.total_stake / 10_000_000);
println!("Potential payout: {} XLM", all_bets.total_potential_payout / 10_000_000);
println!("Winning bets: {}", all_bets.winning_bets);

// Calculate ROI
let roi = (all_bets.total_potential_payout - all_bets.total_stake) / all_bets.total_stake;
println!("Potential ROI: {}%", roi);

// Display individual bets
for bet in all_bets.bets {
    println!("Market {}: {} on {}", 
        bet.market_id, bet.stake_amount / 10_000_000, bet.outcome);
}
```

**Use Cases:**
- User portfolio/dashboard display
- Calculate aggregate statistics
- Portfolio performance analysis
- Show all active and resolved positions
- Identify claimable payouts

---

### 3. Balance and Pool Queries

Functions to retrieve account balance and market pool information.

#### `query_user_balance(user: Address) -> UserBalanceQuery`

Returns comprehensive user account information including balances and participation metrics.

**Returns:**
```rust
struct UserBalanceQuery {
    user: Address,
    available_balance: i128,
    total_staked: i128,
    total_winnings: i128,
    active_bet_count: u32,
    resolved_market_count: u32,
    unclaimed_balance: i128,
}
```

**Example Usage:**
```rust
// Get user balance
let balance = contract.query_user_balance(env, user)?;

// Display account overview
println!("Account Summary:");
println!("Available balance: {} XLM", balance.available_balance / 10_000_000);
println!("Total staked: {} XLM", balance.total_staked / 10_000_000);
println!("Total winnings: {} XLM", balance.total_winnings / 10_000_000);
println!("Unclaimed balance: {} XLM", balance.unclaimed_balance / 10_000_000);

// Activity metrics
println!("Active bets: {}", balance.active_bet_count);
println!("Resolved markets: {}", balance.resolved_market_count);

// Prompt actions
if balance.unclaimed_balance > 0 {
    println!("You have claimable winnings!");
}

// Portfolio analysis
let total_invested = balance.total_staked;
let total_available = balance.available_balance + balance.total_winnings;
let profit = total_available - total_invested;
println!("Net profit/loss: {} XLM", profit / 10_000_000);
```

**Use Cases:**
- Display account balance information
- Show available funds
- Track total staked amount
- Monitor winnings
- Identify unclaimed payouts
- Calculate portfolio statistics

---

#### `query_market_pool(market_id: Symbol) -> MarketPoolQuery`

Returns market pool distribution and calculated implied probabilities.

**Returns:**
```rust
struct MarketPoolQuery {
    market_id: Symbol,
    total_pool: i128,
    outcome_pools: Map<String, i128>,
    platform_fees: i128,
    implied_probability_yes: u32,
    implied_probability_no: u32,
}
```

**Example Usage:**
```rust
// Get market pool information
let pool = contract.query_market_pool(env, market_id)?;

// Display liquidity information
println!("Market Pool Analysis:");
println!("Total pool: {} XLM", pool.total_pool / 10_000_000);
println!("Platform fees: {} XLM", pool.platform_fees / 10_000_000);

// Show outcome distributions
println!("\nOutcome Distribution:");
for (outcome, amount) in pool.outcome_pools.iter() {
    let percentage = (amount * 100) / pool.total_pool;
    println!("{}: {} XLM ({}%)", outcome, amount / 10_000_000, percentage);
}

// Implied probabilities
println!("\nImplied Probabilities:");
println!("Yes: {}%", pool.implied_probability_yes);
println!("No: {}%", pool.implied_probability_no);

// Price discovery
let yes_pool = pool.outcome_pools.get(String::from_str(&env, "yes")).unwrap_or(0);
let no_pool = pool.outcome_pools.get(String::from_str(&env, "no")).unwrap_or(0);
let yes_probability = (no_pool * 100) / (yes_pool + no_pool);
println!("Market probability of YES: {}%", yes_probability);
```

**Use Cases:**
- Analyze market liquidity
- Discover implied probabilities
- Monitor stake distribution
- Price discovery and odds calculation
- Liquidity assessment before betting
- Understand market consensus

---

#### `query_total_pool_size() -> i128`

Returns total value locked across all markets on the contract.

**Returns:**
- `i128`: Total stakes across all markets (in stroops/XLM cents)

**Example Usage:**
```rust
// Get total TVL
let total_pool = contract.query_total_pool_size(env)?;

println!("Total Value Locked: {} XLM", total_pool / 10_000_000);

// Platform metrics
let total_markets = contract.get_all_markets(env)?.len() as i128;
let avg_market_size = total_pool / total_markets;
println!("Average market size: {} XLM", avg_market_size / 10_000_000);

// Track platform growth
println!("Platform liquidity: {} XLM", total_pool / 10_000_000);
```

**Use Cases:**
- Monitor total platform liquidity
- Dashboard metrics
- Platform growth tracking
- Incentive calculations
- Liquidity provision decisions

---

### 4. Contract State Queries

Functions to retrieve global contract state and system statistics.

#### `query_contract_state() -> ContractStateQuery`

Returns comprehensive system-level metrics and contract state.

**Returns:**
```rust
struct ContractStateQuery {
    total_markets: u32,
    active_markets: u32,
    resolved_markets: u32,
    total_value_locked: i128,
    total_fees_collected: i128,
    unique_users: u32,
    contract_version: String,
    last_update: u64,
}
```

**Example Usage:**
```rust
// Get contract state
let state = contract.query_contract_state(env)?;

// Display platform metrics
println!("Platform Overview:");
println!("Total markets: {}", state.total_markets);
println!("Active markets: {}", state.active_markets);
println!("Resolved markets: {}", state.resolved_markets);
println!("Unique users: {}", state.unique_users);

// Financial metrics
println!("\nFinancial Metrics:");
println!("Total value locked: {} XLM", state.total_value_locked / 10_000_000);
println!("Total fees collected: {} XLM", state.total_fees_collected / 10_000_000);

// System information
println!("\nSystem Information:");
println!("Contract version: {}", state.contract_version);
println!("Last update: {}", state.last_update);

// Market health
let market_health = (state.active_markets as f64) / (state.total_markets as f64);
println!("Market health: {}%", (market_health * 100.0) as u32);

// Platform growth tracking
if state.total_markets > 0 {
    let avg_users_per_market = state.unique_users / state.total_markets;
    println!("Avg users per market: {}", avg_users_per_market);
}
```

**Use Cases:**
- Display platform statistics
- Monitor system health
- Track growth metrics
- Dashboard displays
- System health checks
- Version tracking

---

## Query Response Types

### MarketStatus

Enumeration of market states optimized for queries:

```rust
enum MarketStatus {
    Active,      // Market is open for voting
    Ended,       // Voting has ended
    Disputed,    // Outcome is disputed
    Resolved,    // Outcome determined
    Closed,      // Market closed
    Cancelled,   // Market cancelled
}
```

---

## Gas Efficiency

All query functions are optimized for gas efficiency:

- **Read-Only Operations**: No state modifications
- **Minimal Storage Reads**: Direct lookups without iteration where possible
- **Batch Query Support**: Multiple queries can be combined
- **Caching-Friendly**: Immutable responses suitable for client-side caching

### Estimated Gas Costs

| Query | Estimated Gas |
|-------|---------------|
| `query_event_details` | ~2,000 stroops |
| `query_event_status` | ~1,000 stroops |
| `query_user_bet` | ~1,500 stroops |
| `query_market_pool` | ~2,500 stroops |
| `query_contract_state` | ~3,000 stroops |
| `get_all_markets` | ~500 stroops + 50 per market |

---

## Security Considerations

### Input Validation

All query functions validate inputs:
- **Market IDs**: Verified to exist in storage
- **User Addresses**: Format validation
- **Market State**: Consistency checks

### Error Handling

Queries return structured error results:
- `MarketNotFound`: Requested market doesn't exist
- `UserNotFound`: User hasn't participated in market
- `InvalidMarket`: Market data is corrupted
- `ContractStateError`: System state is invalid

### Data Consistency

- All queries return point-in-time snapshots
- No race conditions (read-only operations)
- State consistency guaranteed by contract design

---

## Integration Examples

### React/JavaScript Client

```javascript
// Query event details
const details = await contract.query_event_details(marketId);
console.log(`Market: ${details.question}`);

// Check user bet
try {
    const bet = await contract.query_user_bet(userAddress, marketId);
    console.log(`User staked: ${bet.stake_amount}`);
} catch (e) {
    console.log("User hasn't bet on this market");
}

// Get user balance
const balance = await contract.query_user_balance(userAddress);
console.log(`Available: ${balance.available_balance}`);
console.log(`Unclaimed: ${balance.unclaimed_balance}`);
```

### Python Client

```python
# Query market pool
pool = contract.query_market_pool(market_id)
total_staked = pool['total_pool']
implied_prob = pool['implied_probability_yes']

# Get all markets
markets = contract.get_all_markets()
for market_id in markets:
    details = contract.query_event_details(market_id)
    print(f"Market: {details['question']}")
```

---

## Performance Optimization Tips

### Client-Side Caching

Cache query results with appropriate TTL:
- **Event details**: 30 second TTL
- **User bets**: 10 second TTL
- **Market pools**: 5 second TTL
- **Contract state**: 60 second TTL

### Batch Queries

Combine multiple queries to reduce round trips:

```rust
// Instead of separate calls
let details = contract.query_event_details(env, market_id)?;
let pool = contract.query_market_pool(env, market_id)?;
let user_bet = contract.query_user_bet(env, user, market_id)?;

// Combine results for efficient client display
let market_view = MarketView {
    details,
    pool,
    user_bet,
};
```

### Pagination

For large market lists:

```rust
let all_markets = contract.get_all_markets(env)?;
let page_size = 20;
let total_pages = (all_markets.len() + page_size - 1) / page_size;

for page in 0..total_pages {
    let start = page * page_size;
    let end = std::cmp::min(start + page_size, all_markets.len());
    let page_markets = &all_markets[start..end];
    // Display page_markets
}
```

---

## Testing

All query functions include comprehensive tests:

- **Unit Tests**: Individual function testing
- **Integration Tests**: Cross-function interactions
- **Property-Based Tests**: Edge case coverage
- **Gas Benchmarks**: Performance validation

Run tests with:
```bash
make test
```

---

## Support and Troubleshooting

### Common Issues

**Q: Query returns MarketNotFound but I know the market exists**
- A: Check that the market_id symbol matches exactly (case-sensitive)
- A: Verify market was created on the same contract instance

**Q: User balance shows 0 when user has bets**
- A: Ensure user address is spelled correctly
- A: Some balances may only update after market resolution

**Q: Implied probabilities don't add up to 100%**
- A: Probabilities are rounded to integers; rounding may cause 1% variance
- A: Probabilities assume binary outcomes; multi-outcome markets use only first two

### Getting Help

For issues or questions:
1. Check documentation examples
2. Review test cases
3. Examine error messages carefully
4. Verify input formats match expected types

