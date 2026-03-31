## Gas Usage Analysis (Function Catalog)

This document catalogs public entrypoints in `predictify-hybrid` and provides a structure to record gas usage characteristics and measured costs. Use the benchmarking guide to populate the "Measured Cost" columns.

### Method Inventory

- initialize(env, admin)
- create_market(env, admin, question, outcomes, duration_days, oracle_config) -> Symbol
- vote(env, user, market_id, outcome, stake)
- claim_winnings(env, user, market_id)
- get_market(env, market_id) -> Option<Market>
- fetch_oracle_result(env, market_id, oracle_contract) -> Result<String, Error>
- resolve_market(env, market_id) -> Result<(), Error>
- get_resolution_analytics(env) -> Result<ResolutionAnalytics, Error>
- get_market_analytics(env, market_id) -> Result<MarketStats, Error>
- dispute_market(env, user, market_id, stake, reason) -> Result<(), Error>
- vote_on_dispute(env, user, market_id, dispute_id, vote, stake, reason) -> Result<(), Error>
- resolve_dispute(env, admin, market_id) -> Result<DisputeResolution, Error>
- collect_fees(env, admin, market_id) -> Result<i128, Error>
- extend_market(env, admin, market_id, additional_days, reason, fee_amount) -> Result<(), Error>
- Storage optimization helpers (compress/cleanup/migrate/monitor/optimize/...)

### Storage Touch Patterns (selected excerpts)

Vote path writes a vote and stake, updates totals, and persists market:

```275:308:contracts/predictify-hybrid/src/lib.rs
// vote(...)
// ...
// Store the vote and stake
market.votes.set(user.clone(), outcome);
market.stakes.set(user.clone(), stake);
market.total_staked += stake;

env.storage().persistent().set(&market_id, &market);
```

Market creation allocates a new `Market` with several empty maps and persists once:

```183:221:contracts/predictify-hybrid/src/lib.rs
// create_market(...)
// Generate ID, compute end_time, then
let market = Market {
    // ...
    oracle_result: None,
    votes: Map::new(&env),
    total_staked: 0,
    dispute_stakes: Map::new(&env),
    stakes: Map::new(&env),
    claimed: Map::new(&env),
    winning_outcome: None,
    fee_collected: false,
    state: MarketState::Active,
    total_extension_days: 0,
    max_extension_days: 30,
    extension_history: Vec::new(&env),
};
env.storage().persistent().set(&market_id, &market);
```

Claim path iterates to compute `winning_total` and marks `claimed`:

```395:419:contracts/predictify-hybrid/src/lib.rs
// claim_winnings(...)
// Calculate total winning stakes
let mut winning_total = 0;
for (voter, outcome) in market.votes.iter() {
    if &outcome == winning_outcome {
        winning_total += market.stakes.get(voter.clone()).unwrap_or(0);
    }
}
// Mark as claimed
market.claimed.set(user.clone(), true);
env.storage().persistent().set(&market_id, &market);
```

### Analysis Template

Fill per method after running benchmarks (see ../gas/GAS_BENCHMARKING.md):

- initialize
  - Reads: 0-1 (admin guard if re-init)
  - Writes: 1 (Admin key)
  - Bytes written (est.): small
  - Measured: instructions=…, r-entries=…, w-entries=…, rKB=…, wKB=…

- create_market
  - Reads: 1 (admin)
  - Writes: 2 (counter, market)
  - Bytes drivers: `question`, `outcomes` length
  - Risks: long strings blow write-bytes; validate lengths
  - Measured: …

- vote
  - Reads: 1 (market)
  - Writes: 1 (market)
  - Map ops: votes.set, stakes.set
  - Loop: none
  - Measured: …

- claim_winnings
  - Reads: 1 (market)
  - Writes: 1 (market)
  - Loop: iterates `votes` (cost scales with voters)
  - Optimization: accumulate and cache totals off-chain; filter losers early
  - Measured: …

- fetch_oracle_result
  - Reads: 1 (market)
  - Cross-contract: yes (oracle)
  - Writes: 0 (this method returns result only)
  - Measured: …

- resolve_market
  - Likely reads+writes market; hybrid algorithm cost scales with votes
  - Measured: …

- collect_fees / extend_market / dispute*
  - Admin read, market write patterns
  - Measured: …

### Length Limits to Enforce (to control write-bytes)

- `question`: recommend <= 140 chars
- `outcomes[i]`: recommend <= 32 chars
- `reason` fields: recommend <= 160 chars

### Recording Results

Record CLI `--cost` outputs and RPC simulation breakdowns in a CSV under `benchmarks/results/` for each function and typical scenarios (small/medium/large markets).

