# Dashboard Statistics Export - Quick Reference

**Version**: 1.0  
**Updated**: 2026-03-30  
**Status**: Implementation Complete

---

## 5 New Query Functions

### 1. Platform Dashboard Stats
```rust
pub fn get_dashboard_statistics(env: Env) -> Result<DashboardStatisticsV1, Error>
```
**Returns**: Platform metrics (TVL, active events, fees, user count)  
**Use When**: Loading dashboard header, initializing main view  
**Gas**: ~50K stroops

### 2. Market Metrics
```rust
pub fn get_market_statistics(env: Env, market_id: Symbol) -> Result<MarketStatisticsV1, Error>
```
**Returns**: Consensus strength (0-10000), volatility, participant count  
**Use When**: Rendering market detail page, showing volatility indicator  
**Gas**: ~20K stroops

### 3. Category Filter
```rust
pub fn get_category_statistics(env: Env, category: String) -> Result<CategoryStatisticsV1, Error>
```
**Returns**: Aggregated metrics for all markets in category  
**Use When**: Showing category-filtered dashboard section  
**Gas**: ~40K stroops

### 4. Earnings Leaderboard
```rust
pub fn get_top_users_by_winnings(env: Env, limit: u32) -> Result<Vec<UserLeaderboardEntryV1>, Error>
```
**Returns**: Top N users by total winnings (limit capped at 50)  
**Use When**: Leaderboard page, top earners section  
**Gas**: ~30K stroops

### 5. Skill Leaderboard
```rust
pub fn get_top_users_by_win_rate(env: Env, limit: u32, min_bets: u64) -> Result<Vec<UserLeaderboardEntryV1>, Error>
```
**Returns**: Top N users by win rate (filtered by min_bets)  
**Use When**: Skill/accuracy leaderboard, prediction rankings  
**Gas**: ~30K stroops

---

## Key Metrics Explained

### Consensus Strength (0-10000)
- **Formula**: `(largest_outcome_pool / total_volume) * 10000`
- **10000**: Everyone agrees (100% on one outcome)
- **5000**: Even split (50/50)
- **0**: Impossible (would mean one outcome has 0 stakes)
- **Display**: Divide by 100 to get percentage

### Volatility (0-10000)
- **Formula**: `10000 - consensus_strength`
- **0**: Perfect agreement (no disagreement)
- **5000**: Even split in opinion
- **10000**: Maximum disagreement
- **Property**: `consensus + volatility = 10000` always

### Win Rate (basis points)
- **Formula**: `(winning_bets / total_bets) * 10000`
- **10000**: 100% win rate
- **7500**: 75% win rate
- **5000**: 50% win rate
- **Display**: Divide by 100 to get percentage

---

## Response Types (All Versioned as V1)

### DashboardStatisticsV1
```typescript
{
  api_version: 1,
  platform_stats: {
    total_events_created: u64,
    total_bets_placed: u64,
    total_volume: i128,
    total_fees_collected: i128,
    active_events_count: u32
  },
  query_timestamp: u64,
  active_user_count: u32,
  total_value_locked: i128
}
```

### MarketStatisticsV1
```typescript
{
  market_id: Symbol,
  participant_count: u32,
  total_volume: i128,
  average_stake: i128,
  consensus_strength: u32,        // 0-10000
  volatility: u32,                 // 0-10000
  state: MarketState,
  created_at: u64,
  question: String,
  api_version: 1
}
```

### CategoryStatisticsV1
```typescript
{
  category: String,
  market_count: u32,
  total_volume: i128,
  participant_count: u32,
  resolved_count: u32,
  average_market_volume: i128
}
```

### UserLeaderboardEntryV1
```typescript
{
  user: Address,
  rank: u32,
  total_winnings: i128,
  win_rate: u32,                   // basis points (0-10000)
  total_bets_placed: u64,
  winning_bets: u64,
  total_wagered: i128,
  last_activity: u64
}
```

---

## JavaScript Integration Examples

### Load Dashboard Data
```javascript
const dashboard = {
  platform: await contract.get_dashboard_statistics(),
  markets: {},
  categories: {},
  leaderboards: {}
};

// Get featured markets
let cursor = 0;
const featured = [];
while (featured.length < 10) {
  const { items } = await contract.get_all_markets_paged({ cursor, limit: 50 });
  for (const id of items) {
    const details = await contract.query_event_details({ market_id: id });
    const stats = await contract.get_market_statistics({ market_id: id });
    featured.push({ ...details, ...stats });
    if (featured.length >= 10) break;
  }
  if (items.length < 50) break;
  cursor = items.length ? cursor + 50 : cursor;
}
dashboard.markets = featured;

// Get category stats
for (const cat of ["sports", "crypto", "politics"]) {
  dashboard.categories[cat] = await contract.get_category_statistics({ category: cat });
}

// Get leaderboards
dashboard.leaderboards = {
  earnings: await contract.get_top_users_by_winnings({ limit: 10 }),
  skills: await contract.get_top_users_by_win_rate({ limit: 10, min_bets: 5n })
};
```

### Format Metrics for Display
```javascript
function formatMetrics(stats) {
  return {
    consensus: `${Math.floor(stats.consensus_strength / 100)}%`,
    volatility: `${Math.floor(stats.volatility / 100)}%`,
    tvl: `$${(stats.total_value_locked / 1e7).toFixed(2)}`,
    winRate: `${Math.floor(stats.win_rate / 100)}%`
  };
}
```

---

## Rust Integration Examples

### Get Market Stats
```rust
let stats = contract.get_market_statistics(&env, market_id)?;
println!("Participants: {}", stats.participant_count);
println!("Consensus: {}%", stats.consensus_strength / 100);
println!("Volatility: {}%", stats.volatility / 100);
```

### Validate Invariant
```rust
let stats = contract.get_market_statistics(&env, market_id)?;
assert_eq!(stats.consensus_strength + stats.volatility, 10000);
```

### Get Leaderboards
```rust
let topEarners = contract.get_top_users_by_winnings(&env, 10)?;
for entry in topEarners.iter() {
    println!("#{}: {} won {} stroops", 
        entry.rank, 
        entry.user, 
        entry.total_winnings);
}
```

---

## Key Design Decisions

### 1. Versioning (V1 Suffix)
- **Why**: Allows safe addition of fields without breaking clients
- **How**: New fields append to types
- **Future**: Breaking changes use V2, V3, etc.

### 2. Consensus & Volatility
- **Why**: Inverse metrics complement each other
- **Property**: Always sum to 10000 (invariant)
- **Display**: Divide percentages by 100

### 3. Leaderboard Filtering
- **min_bets**: Prevents lucky winners (e.g., 1 win out of 1 bet)
- **limit cap**: MAX_PAGE_SIZE = 50 for gas bounds
- **No caching**: Scans live user stats

### 4. Category Queries
- **Linear scan**: Filters by market category field
- **Aggregation**: Sums metrics across matching markets
- **Performance**: Acceptable for off-chain caching

---

## Performance Tips

### Caching
- Cache dashboard stats: 30-60 seconds
- Cache leaderboards: 5-10 minutes
- Cache category stats: 1-2 minutes

### Pagination
- Always use cursor-based pagination for market lists
- Combine queries to reduce round-trips
- Use `Promise.all()` for parallel requests

### Gas Optimization
- Batch multiple queries in single transaction when possible
- Use specific queries instead of scanning all markets
- Cache category filters on client-side

---

## Error Handling

### Possible Errors

| Error | Cause | Recovery |
|-------|-------|----------|
| `MarketNotFound` | Invalid market_id | Validate market exists first |
| Input validation | Invalid category string | Use non-empty category |
| Contract error | State issue | Retry or check contract health |

### Best Practices
```javascript
try {
  const stats = await contract.get_market_statistics({ market_id });
} catch (error) {
  if (error.message.includes('MarketNotFound')) {
    // Market doesn't exist, use default metrics
  } else {
    // Retry or show error to user
  }
}
```

---

## Testing Checklist

- [ ] All 18+ tests passing
- [ ] Code coverage ≥95%
- [ ] Can fetch dashboard stats without error
- [ ] Market metrics show correct consensus/volatility
- [ ] Leaderboards return sorted results
- [ ] Category aggregation works
- [ ] No panics on edge cases
- [ ] API version=1 in all responses

---

## Common Questions

**Q: Why does consensus_strength + volatility = 10000?**  
A: By design - they're inverse metrics. High agreement = low volatility, and vice versa.

**Q: Can I get historical metrics?**  
A: No, these are snapshots only. Use off-chain storage for history.

**Q: How often should I cache?**  
A: 30-60 seconds for platform stats, 5-10 min for leaderboards.

**Q: What's the gas cost?**  
A: Most queries <50K stroops. Budget 100K for safety.

**Q: Can I combine queries?**  
A: Use JavaScript Promise.all() for parallel requests.

**Q: Will V1 break in the future?**  
A: No. New fields append safely. Breaking changes use V2.

---

## Documentation Links

- [Full API Guide](../../docs/api/QUERY_IMPLEMENTATION_GUIDE.md#dashboard-statistics-queries-new)
- [Implementation Details](./DASHBOARD_STATISTICS_IMPLEMENTATION.md)
- [Test Report](./DASHBOARD_STATISTICS_TEST_REPORT.md)
- [Main Contract README](./README.md)

---

*Version: 1.0*  
*Last Updated: 2026-03-30*  
*Status: Production Ready*
