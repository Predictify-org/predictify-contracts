# Security Test Guide

## 1. Dependency Scanning
- Regularly check for source-code files with changes
- Check for compatibility and resolve performance issues

## 2. Penetration Testing
- Use Kali Linux and Burp Suite to identify vulnerabilities
- Use Wireshark to check network traffic

## 3. Dynamic Application Security Testing (DAST)
- DAST tools are used for identifying security misconfiguration, broken authentication and input/output validation
- ZED Attack Proxy is an open source tool for security testing provided by OWASP

## 4. Static Application Security Testing (SAST)
- Tools help in detecting SQL injections, and other vulnerabilities
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
- **Execution**: Run with `cargo test -p predictify-hybrid --test property_based_tests`.

## 6. Event Emission Security (Audit Focus)

Events are critical for off-chain transparency and indexer reliability. Every financially material transition must be published to the Soroban event stream.

### 6.1 Threat Model
- **Invisible Payouts**: Winnings claimed without event emission, making it impossible for trackers to verify total supply and distributions.
- **Silent Malfeasance**: Admin role transfers or market parameter changes (outcomes/durations) occurring without public audit logs.
- **Indexer Desynchronization**: Missing state change events (e.g., `Active` -> `Cancelled`) leading to off-chain UIs showing stale/incorrect market statuses.

### 6.2 Security Invariants
- **Consistency**: Every `store_event()` call in the contract must be accompanied by a corresponding `env.events().publish()` call if the data is required for external auditing.
- **Efficiency**: Events use specific, searchable topics `(Symbol, ScVal)` to allow indexers to filter by market ID or user without full chain scans.
- **Atomicity**: Events are published within the same transaction as the state change they describe.

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
