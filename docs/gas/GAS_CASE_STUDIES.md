## Gas Optimization Case Studies (Predictify Hybrid)

### 1) Voting: Avoid per-iteration storage access

Issue: Repeated `.get()`/`.set()` inside loops increases read/write entries.

Fix: Read market once, update in-memory, write once.

See the `vote` implementation which already batches to a single final write:

```302:308:contracts/predictify-hybrid/src/lib.rs
market.votes.set(user.clone(), outcome);
market.stakes.set(user.clone(), stake);
market.total_staked += stake;
env.storage().persistent().set(&market_id, &market);
```

Further improvement: Pre-validate `outcome` using an in-memory set if outcomes are large to avoid repeated scans.

### 2) Claiming: Scale with participants carefully

Current approach iterates all votes to compute `winning_total`:

```395:404:contracts/predictify-hybrid/src/lib.rs
let mut winning_total = 0;
for (voter, outcome) in market.votes.iter() {
    if &outcome == winning_outcome {
        winning_total += market.stakes.get(voter.clone()).unwrap_or(0);
    }
}
```

Optimizations:

- Maintain `stakes_per_outcome` totals during `vote` to avoid O(n) scan at claim time.
- Consider a compact bitmap/flag for `claimed` to reduce map overhead.

### 3) Market Creation: Bound string sizes

Cost driver: `question` and `outcomes` lengths inflate write-bytes.

Guideline: Enforce caps (e.g., 140/32 chars). Reject overlong inputs to protect fees.

### 4) Oracle Resolution: Validate before calling

Call the cheapest checks first (staleness, feed format) before cross-contract calls. Skip persistence until a valid result is known.

### 5) Events over Storage

Emit events for analytics (e.g., vote tally changes) and only persist aggregates needed for on-chain reads.

