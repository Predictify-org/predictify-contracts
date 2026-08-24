# Security Test Guide

## Scope
- Contract-only security verification for `predictify-hybrid`.
- Focus on state transitions that move value or change claimability: market creation, bet placement, bet resolution, refund, fee collection, and cancellation.

## Threat Model
- A transition updates storage but does not publish a ledger event, leaving indexers blind.
- A transition emits an event before state changes are committed, creating a false audit trail.
- A transition emits the wrong status or amount, causing downstream payout or analytics errors.
- A paused contract still accepts write paths, allowing partial state changes during an incident.

## Event Invariants
- Every financially material transition must publish a ledger event and persist the same payload for internal queries.
- `mkt_crt` must reflect the created market ID, admin, outcomes, and timestamp.
- `bet_plc` must reflect the bettor, market ID, outcome, amount, and timestamp.
- `bet_upd` must reflect every change from `Active` to `Won`, `Lost`, `Refunded`, or `Cancelled`.
- `mkt_res` must be published when a market is resolved.
- Event publication must be atomic with the contract state change.

## Circuit Breaker Semantics
- Read-only queries remain available while the breaker is open or half-open.
- Mutating operations must check the breaker before they touch storage or move funds.
- `PauseScope::Full` blocks all writes.
- `PauseScope::BettingOnly` blocks betting and other value-locking paths, but leaves reads and non-betting writes explicit in code.
- The expected failure mode for blocked writes is `Error::CBOpen`.

## Verification Commands
- `cargo test -p predictify-hybrid`
- `cargo test -p predictify-hybrid event_management_tests`
- `cargo test -p predictify-hybrid test_market_resolution_publishes_status_events`

## Review Checklist
- Confirm the ledger event topic matches the transition name.
- Confirm event data decodes to the expected contract type.
- Confirm the stored state matches the event payload.
- Confirm unauthorized or invalid transitions emit no success event.
- Confirm every write path touched by the breaker returns `CBOpen` while reads still succeed.

## Non-Goals
- This guide does not cover frontend indexing pipelines.
- This guide does not define retention or archival policy for old events.
- This guide does not replace contract-level authorization or balance checks.

## Notes For Integrators
- Prefer consuming ledger events first.
- Use persistent storage only for reconciliation or backfill.
- Treat missing events as a failed contract release until the regression tests pass.

## 3. Dynamic Application Security Testing (DAST)
- DAST tools are used for identifying security misconfiguration, broken authentication and input/output validation.
- ZED Attack Proxy is an open source tool for security testing provided by OWASP.

## 4. Static Application Security Testing (SAST)
- Tools help in detecting SQL injections and other vulnerabilities.
- SonarQube, Fortify are commonly used tools.
- Integrate with IDEs and CI/CD pipelines.

## 5. Property-Based Testing (Proptest)
- Smart contract invariants, especially around financial logic like stake distributions, payouts, and fee deductions, are verified using property-based fuzzing.
- Threat model covered: payout calculation overflow/underflow, rounding errors giving away more funds than total pooled, double-claim attacks, zero-winner scenarios, fee evasion.
- Invariants proven:
  - `distribute_payouts`: total distributed to all users is `total_pool` minus fees/truncation and mathematically proportional.
  - Payout is strictly zero when there are no winners.
  - Fees are deducted exactly according to the percentage configuration.
  - Double distributions and double claims result in zero extra payouts.
- Execution: run `cargo test -p predictify-hybrid --test property_based_tests`.

## 6. Event Emission Security (Audit Focus)

Events are critical for off-chain transparency and indexer reliability. Every financially material transition must be published to the Soroban event stream.

### 6.1 Threat Model
- Invisible payouts: winnings claimed without event emission, making it impossible for trackers to verify total supply and distributions.
- Silent malfeasance: admin role transfers or market parameter changes occurring without public audit logs.
- Indexer desynchronization: missing state change events leading to off-chain UIs showing stale or incorrect market statuses.

### 6.2 Security Invariants
- Consistency: every `store_event()` call in the contract must be accompanied by a corresponding `env.events().publish()` call if the data is required for external auditing.
- Efficiency: events use specific, searchable topics `(Symbol, ScVal)` to allow indexers to filter by market ID or user without full chain scans.
- Atomicity: events are published within the same transaction as the state change they describe.

### 6.3 Event Topic Reference

| Event Key | Topic Identifier | Search Data | Description |
|---|---|---|---|
| `mkt_crt` | `symbol_short!("mkt_crt")` | `market_id` | New market creation |
| `vote` | `symbol_short!("vote")` | `market_id` | Stake-weighted vote cast |
| `bet_plc` | `symbol_short!("bet_plc")` | `market_id` | Bet placement (funds locked) |
| `mkt_res` | `symbol_short!("mkt_res")` | `market_id` | Market resolution (payouts determined) |
| `win_clm` | `symbol_short!("win_clm")` | `market_id` | Payout claim executed |
| `dispt_crt` | `symbol_short!("dispt_crt")` | `market_id` | Dispute initiation (funds locked) |
| `st_chng` | `symbol_short!("st_chng")` | `market_id` | Market life-cycle state change |
| `adm_xfer` | `symbol_short!("adm_xfer")` | `new_admin` | Administrative authority transfer |

### 6.4 Verification Coverage
All material events are verified in `contracts/predictify-hybrid/src/event_management_tests.rs`. Use `env.events().all()` in tests to ensure both persistent storage and the public event stream are correctly updated.
