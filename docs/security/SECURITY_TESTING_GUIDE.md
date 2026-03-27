# Security Test Guide

## 1. Dependency Scanning
- Regularly check for source-code files with changes
- Check for compatibility and resolve performance issues

## 2. Penetration Testing
- Use Kali Linux and Burp Suite to identify vulnerabilities
- Use Wireshark to check network traffic

## 3. Dynamic Application Security Testing(DASP)
- DASP tools are used for identifying security misconfiguration, broken authentication and input/output validation
- ZED Attack Proxy is an open source tool for security testing provided by OWASP

## 4. Static Application Security Testing(SAST)
- Tools help in detecting SQL injections,and other vulnerabilities
- SonarQube, Fortify are commonly used tools
- Integrate with IDEs and CI/CD pipelines

## 5. Property-Based Testing (Proptest)
- Smart contract invariants (especially around financial logic like stake distributions, payouts, and fee deductions) are verified using property-based fuzzing.
- **Threat Model Covered**: Payout calculation overflow/underflow, rounding errors giving away more funds than total pooled, double-claim attacks, zero-winner scenarios, fee evasion.
- **Invariants Proven**:
  - `distribute_payouts`: Total distributed to all users is `total_pool` (minus fees/truncation) and mathematically proportional.
  - Payout is strictly zero when there are no winners.
  - Fees are deducted exactly according to the percentage configuration.
  - Double distributions and double claims result in zero extra payouts.
- **Explicit Non-Goals**: Property testing of off-chain components or exact sub-stroop distribution (small 1 stroop differences due to integer div truncation are securely kept in contract).
- **Execution**: Run with `cargo test -p predictify-hybrid --test property_based_tests`.

---

## 6. Event Emission Audit (Issue #426)

### Overview

All financially material state changes in the Predictify Hybrid contract **must** emit
indexer-friendly events via `env.events().publish(…)` so that off-chain services
(Horizon, custom indexers, DeFi analytics) can reconstruct position histories, fund
flows, and dispute timelines without querying persistent storage.

### Architectural Fix

Prior to this audit, `EventEmitter::store_event` wrote events only to
`env.storage().persistent()` — which is **invisible** to the Soroban event stream.
The fix upgrades `store_event` to call **both** `env.events().publish(topic, data)`
and persist the latest-event cache slot, making all past and future `emit_*` helpers
indexer-visible with no caller-site changes required.

### Threat Model

| Threat | Mitigated By |
|---|---|
| Silent fund transfer — a bet payout or refund occurs with no on-chain evidence | `emit_winnings_claimed` / `emit_bet_cancelled` required on every payout path |
| Undetectable resolution — market transitions to Resolved with no public record | `emit_market_resolved` + `emit_state_change_event` required on every resolution path |
| Dispute opacity — a dispute is created/resolved without notifying indexers | `emit_dispute_created` and `emit_dispute_resolved` required |
| Bet settlement gap — bets are marked Won/Lost internally but not externally | `emit_bet_resolved` called for every bet in `resolve_market_bets` |
| Indexer desync — off-chain tools must poll storage instead of listening to events | Fixed by making `store_event` call `env.events().publish` for every event |

### Invariants Proven by Audit

1. **Every `place_bet` call publishes a `BetPlacedEvent`** (topic: `bet_plc`).
2. **Every `resolve_market_manual` / oracle resolution call publishes a `MarketResolvedEvent`** (topic: `mkt_res`) and a `StateChangeEvent` (topic: `st_chng`).
3. **Every bet settled in `resolve_market_bets` publishes a `BetStatusUpdatedEvent`** with `new_status = "Won"` or `new_status = "Lost"` (topic: `bet_upd`).
4. **Every `claim_winnings` call publishes a `WinningsClaimedEvent`** (topic: `win_clm`).
5. **Every `dispute_market` call publishes a `DisputeCreatedEvent`** (topic: `dispt_crt`).
6. **Every `resolve_dispute` call publishes a `DisputeResolvedEvent`** (topic: `dispt_res`).
7. **Every refund in `refund_market_bets` publishes a `BetStatusUpdatedEvent`** with `new_status = "Cancelled"` (topic: `bet_upd`).
8. **Every `create_market` call publishes a `MarketCreatedEvent`** (topic: `mkt_crt`).
9. **Every `vote` call publishes a `VoteCastEvent`** (topic: `vote`).

### Financially-Material Event Table

| Event Struct | Topic Symbol | When Emitted | Module |
|---|---|---|---|
| `MarketCreatedEvent` | `mkt_crt` | Market creation | `lib.rs::create_market` |
| `VoteCastEvent` | `vote` | Stake-weighted vote | `lib.rs::vote` |
| `BetPlacedEvent` | `bet_plc` | Bet placement | `bets.rs::place_bet` |
| `BetStatusUpdatedEvent` | `bet_upd` | Bet resolved (Won/Lost) or cancelled | `bets.rs::resolve_market_bets`, `bets.rs::refund_market_bets` |
| `MarketResolvedEvent` | `mkt_res` | Oracle or manual resolution | `lib.rs`, `resolution.rs` |
| `StateChangeEvent` | `st_chng` | Any market state transition | `markets.rs`, `lib.rs`, `resolution.rs` |
| `DisputeCreatedEvent` | `dispt_crt` | Dispute raised by community member | `disputes.rs::process_dispute` |
| `DisputeResolvedEvent` | `dispt_res` | Dispute finalized | `disputes.rs::resolve_dispute` |
| `WinningsClaimedEvent` | `win_clm` | Winning payout claimed | `lib.rs::claim_winnings` |
| `FeeCollectedEvent` | `fee_col` | Platform fee collected | `lib.rs` |
| `FeeWithdrawnEvent` | `fwd_ok` | Admin withdraws accumulated fees | `lib.rs` |
| `ContractInitializedEvent` | `contract_initialized` | Contract first deployed | `lib.rs::initialize` |

### Verifying Events in Tests

Soroban's test environment captures all events published with `env.events().publish`.
After clearing the event buffer with `env.events().all()`, call the operation under
test, then assert the buffer is non-empty:

```rust
setup.env.events().all(); // drain/clear previously buffered events
client.place_bet(&user, &market_id, &outcome, &amount);
let emitted = setup.env.events().all();
assert!(!emitted.is_empty(), "place_bet must emit an event");
```

To inspect the specific topic symbol, destructure the event tuple:

```rust
let emitted = env.events().all();
// Each entry is (contract_id, topics: Vec<Val>, data: Val)
// Topics[0] is typically the symbol_short!("bet_plc") for BetPlacedEvent
```

### Explicit Non-Goals

- This audit does **not** guarantee event ordering — Soroban events within a single
  transaction are ordered by publication sequence but cross-transaction ordering is
  determined by ledger sequence.
- This audit does **not** validate event schema backwards-compatibility — struct field
  additions are breaking changes for indexers and require a migration plan.
- Off-chain indexer implementations (Horizon event consumers, subgraphs) are out of scope.
- Events in read-only query paths are explicitly not required and should not be added.

### Execution

```powershell
# Run all event emission tests
cargo test -p predictify-hybrid event_management 2>&1

# Run the full test suite to check for regressions
cargo test -p predictify-hybrid 2>&1
```

**Target coverage**: ≥ 95 % line coverage for `events.rs`, `bets.rs` (resolution path),
and `disputes.rs` (resolution path) as measured by `cargo-tarpaulin` or equivalent.
