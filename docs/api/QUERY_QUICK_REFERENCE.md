# Query Functions Quick Reference

## Function Summary

### 1. Event/Market Queries

```rust
// Get complete market information
query_event_details(market_id) -> EventDetailsQuery

// Quick status check
query_event_status(market_id) -> (MarketStatus, u64)

// List all markets
get_all_markets() -> Vec<Symbol>
```

### 2. User Bet Queries

```rust
// Get user's bet on specific market
query_user_bet(user, market_id) -> UserBetQuery

// Get all user's bets
query_user_bets(user) -> MultipleBetsQuery
```

### 3. Balance Queries

```rust
// Get user account balance info
query_user_balance(user) -> UserBalanceQuery

// Get market pool distribution
query_market_pool(market_id) -> MarketPoolQuery

// Get total platform TVL
query_total_pool_size() -> i128
```

### 4. System Queries

```rust
// Get global contract state
query_contract_state() -> ContractStateQuery
```

---

## Quick Examples

### Display Market Information

```javascript
// Get market details
const market = await contract.query_event_details(marketId);

console.log(`Question: ${market.question}`);
console.log(`Status: ${market.status}`);
console.log(`Total Staked: ${market.total_staked / 10_000_000} XLM`);
console.log(`Outcomes: ${market.outcomes.join(", ")}`);

if (market.winning_outcome) {
    console.log(`Winner: ${market.winning_outcome}`);
}
```

### Show User Portfolio

```javascript
// Get all user bets
const portfolio = await contract.query_user_bets(userAddress);

console.log(`Active Bets: ${portfolio.bets.length}`);
console.log(`Total Staked: ${portfolio.total_stake / 10_000_000} XLM`);
console.log(`Potential Payout: ${portfolio.total_potential_payout / 10_000_000} XLM`);
console.log(`Winning Positions: ${portfolio.winning_bets}`);

portfolio.bets.forEach(bet => {
    console.log(`  - ${bet.market_id}: ${bet.stake_amount / 10_000_000} XLM on ${bet.outcome}`);
});
```

### Check Market Pool

```javascript
// Get market pool distribution
const pool = await contract.query_market_pool(marketId);

console.log(`Total Pool: ${pool.total_pool / 10_000_000} XLM`);
console.log(`Implied Probability (Yes): ${pool.implied_probability_yes}%`);
console.log(`Implied Probability (No): ${pool.implied_probability_no}%`);

// Show outcome distribution
for (const [outcome, amount] of Object.entries(pool.outcome_pools)) {
    const percentage = (amount * 100) / pool.total_pool;
    console.log(`  ${outcome}: ${amount / 10_000_000} XLM (${percentage}%)`);
}
```

### Display Account Balance

```javascript
// Get user balance
const balance = await contract.query_user_balance(userAddress);

console.log(`Available Balance: ${balance.available_balance / 10_000_000} XLM`);
console.log(`Total Staked: ${balance.total_staked / 10_000_000} XLM`);
console.log(`Total Winnings: ${balance.total_winnings / 10_000_000} XLM`);
console.log(`Unclaimed Balance: ${balance.unclaimed_balance / 10_000_000} XLM`);

if (balance.unclaimed_balance > 0) {
    console.log(`💰 You have ${balance.unclaimed_balance / 10_000_000} XLM to claim!`);
}
```

### Platform Dashboard

```javascript
// Get global stats
const state = await contract.query_contract_state();

console.log(`Platform Statistics:`);
console.log(`  Total Markets: ${state.total_markets}`);
console.log(`  Active Markets: ${state.active_markets}`);
console.log(`  Total Value Locked: ${state.total_value_locked / 10_000_000} XLM`);
console.log(`  Total Users: ${state.unique_users}`);
console.log(`  Total Fees: ${state.total_fees_collected / 10_000_000} XLM`);
```

---

## Response Types Reference

### EventDetailsQuery
```rust
{
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

### UserBetQuery
```rust
{
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

### UserBalanceQuery
```rust
{
    user: Address,
    available_balance: i128,
    total_staked: i128,
    total_winnings: i128,
    active_bet_count: u32,
    resolved_market_count: u32,
    unclaimed_balance: i128,
}
```

### MarketPoolQuery
```rust
{
    market_id: Symbol,
    total_pool: i128,
    outcome_pools: Map<String, i128>,
    platform_fees: i128,
    implied_probability_yes: u32,
    implied_probability_no: u32,
}
```

### ContractStateQuery
```rust
{
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

### MultipleBetsQuery
```rust
{
    bets: Vec<UserBetQuery>,
    total_stake: i128,
    total_potential_payout: i128,
    winning_bets: u32,
}
```

---

## Stroops to XLM Conversion

All monetary values are in stroops (1 XLM = 10,000,000 stroops):

```javascript
// Convert stroops to XLM
const stroops = 50000000;  // 5 XLM
const xlm = stroops / 10_000_000;  // 5.0
console.log(`${xlm} XLM`);

// Convert XLM to stroops
const xlm2 = 2.5;  // 2.5 XLM
const stroops2 = xlm2 * 10_000_000;  // 25000000
```

---

## Error Handling

```javascript
try {
    const bet = await contract.query_user_bet(userAddress, marketId);
} catch (error) {
    if (error.message.includes("MarketNotFound")) {
        console.log("Market does not exist");
    } else if (error.message.includes("UserNotFound")) {
        console.log("User has not participated in this market");
    } else {
        console.log(`Error: ${error.message}`);
    }
}
```

---

## Common Use Cases

### 1. Market Detail Page

```javascript
async function loadMarketPage(marketId) {
    // Get market info
    const details = await contract.query_event_details(marketId);
    
    // Get market pool
    const pool = await contract.query_market_pool(marketId);
    
    // Get user's bet (if logged in)
    const userBet = await contract.query_user_bet(currentUser, marketId);
    
    return {
        question: details.question,
        outcomes: details.outcomes,
        status: details.status,
        endTime: details.end_time,
        totalStaked: details.total_staked,
        poolDistribution: pool.outcome_pools,
        impliedProbabilities: {
            yes: pool.implied_probability_yes,
            no: pool.implied_probability_no
        },
        userBet: userBet ? {
            stake: userBet.stake_amount,
            outcome: userBet.outcome,
            isWinning: userBet.is_winning,
            potentialPayout: userBet.potential_payout
        } : null
    };
}
```

### 2. User Dashboard

```javascript
async function loadUserDashboard(userAddress) {
    // Get account overview
    const balance = await contract.query_user_balance(userAddress);
    
    // Get all active bets
    const portfolio = await contract.query_user_bets(userAddress);
    
    // Get platform stats for context
    const state = await contract.query_contract_state();
    
    return {
        account: {
            available: balance.available_balance,
            staked: balance.total_staked,
            winnings: balance.total_winnings,
            unclaimed: balance.unclaimed_balance
        },
        portfolio: {
            activeBets: portfolio.bets.length,
            totalStake: portfolio.total_stake,
            potentialPayout: portfolio.total_potential_payout,
            winningPositions: portfolio.winning_bets
        },
        platform: {
            totalMarkets: state.total_markets,
            activeSince: new Date(state.last_update * 1000),
            totalUsers: state.unique_users
        }
    };
}
```

### 3. Market Discovery

```javascript
async function discoverMarkets() {
    // Get all markets
    const allMarkets = await contract.get_all_markets();
    
    // Load details for each
    const marketList = await Promise.all(
        allMarkets.map(id => contract.query_event_details(id))
    );
    
    // Filter active markets
    const activeMarkets = marketList.filter(m => m.status === "Active");
    
    // Sort by liquidity
    const sortedByLiquidity = activeMarkets.sort((a, b) => 
        b.total_staked - a.total_staked
    );
    
    return sortedByLiquidity;
}
```

---

## Performance Tips

### 1. Caching

```javascript
// Cache market details (30 second TTL)
const marketCache = new Map();

async function getMarketDetails(marketId) {
    if (marketCache.has(marketId)) {
        return marketCache.get(marketId);
    }
    
    const details = await contract.query_event_details(marketId);
    marketCache.set(marketId, details);
    
    setTimeout(() => marketCache.delete(marketId), 30000);
    return details;
}
```

### 2. Batch Operations

```javascript
// Query multiple markets efficiently
async function getMultipleMarketDetails(marketIds) {
    return Promise.all(
        marketIds.map(id => contract.query_event_details(id))
    );
}
```

### 3. Pagination

```javascript
async function getMarketsPage(pageNum = 0, pageSize = 20) {
    const allMarkets = await contract.get_all_markets();
    const start = pageNum * pageSize;
    const end = Math.min(start + pageSize, allMarkets.length);
    
    const pageMarketIds = allMarkets.slice(start, end);
    const pageMarkets = await Promise.all(
        pageMarketIds.map(id => contract.query_event_details(id))
    );
    
    return {
        markets: pageMarkets,
        page: pageNum,
        pageSize,
        total: allMarkets.length,
        totalPages: Math.ceil(allMarkets.length / pageSize)
    };
}
```

---

## Status Values

```
Active    - Market is open for voting
Ended     - Voting has ended, waiting for resolution
Disputed  - Market outcome is under dispute
Resolved  - Market outcome has been determined
Closed    - Market is permanently closed
Cancelled - Market has been cancelled
```

---

## Key Constants

```
1 XLM = 10,000,000 stroops
Platform Fee = 2%
Minimum Vote = 0.1 XLM (1,000,000 stroops)
Minimum Dispute = 10 XLM (100,000,000 stroops)
```

---

## Troubleshooting

**Q: Query returns undefined/null**
- A: Check market_id or user address format
- A: Verify market/user exists on contract

**Q: implied_probability_yes + implied_probability_no ≠ 100%**
- A: Rounding can cause 1% variance
- A: For multi-outcome markets, only first two are used

**Q: Balance shows 0 for active user**
- A: User might not have participated in any market
- A: Check query_user_bets to see all participations

**Q: High gas costs**
- A: Avoid frequent full contract state queries
- A: Use specific query functions (cheaper than full state)
- A: Implement caching on client

---

## Links

- Full Documentation: [QUERY_FUNCTIONS.md](./QUERY_FUNCTIONS.md)
- Implementation Guide: [QUERY_IMPLEMENTATION_GUIDE.md](./QUERY_IMPLEMENTATION_GUIDE.md)
- Source Code: [queries.rs](../src/queries.rs)
- Tests: [query_tests.rs](../src/query_tests.rs)
