# Oracle-Resolution and Dispute-Attack Threat Model

This document is the single, code-grounded threat model for the oracle-resolution and dispute subsystems of the Predictify Hybrid contract. It enumerates each threat, maps it to the concrete defense in the source, and cites the relevant `Error` variant from `contracts/predictify-hybrid/src/err.rs`.

For broader attack-surface context (reentrancy, access control, flash-loan), see [ATTACK-VECTORS.md](./ATTACK-VECTORS.md).  
For system-wide security considerations see [SECURITY_CONSIDERATIONS.md](./SECURITY_CONSIDERATIONS.md).  
For the Security Features and Oracle/Dispute Management API surface see [API_DOCUMENTATION.md](../api/API_DOCUMENTATION.md).

---

## Scope and Assets

| Asset | Description |
|---|---|
| Market outcome | The string stored as the canonical resolution of a prediction market |
| User funds | Bet stakes and dispute stakes locked in the contract |
| Oracle data | Price/confidence data consumed from Reflector or Pyth feeds |
| Dispute votes | Stake-weighted community votes that can overturn an oracle result |

**Threat actors**: malicious oracle operators, bot-driven dispute spammers, economic attackers with large stake, colluding voters.

---

## 1. Oracle Subsystem Threats

### 1.1 Oracle Manipulation (Feed Poisoning)

**Threat**: An attacker controls or bribes one oracle source and submits a fabricated price to force a false market outcome.

**Defense — Oracle whitelist**  
Only addresses registered in `OracleWhitelist` (`oracles.rs`, `OracleWhitelistKey`) are accepted. Unregistered callers are rejected by `OracleWhitelist::validate_oracle_contract`.  
Relevant errors: `Error::OracleCallbackUnauthorized = 211`, `Error::OracleCallbackAuthFailed = 210`.

**Defense — Multi-source consensus**  
`OracleIntegrationManager` (`oracles.rs`, line 2594) fetches from all active sources and requires `DEFAULT_CONSENSUS_THRESHOLD = 66` (66 %, i.e. ≥ 2/3 majority) before accepting any outcome. A single compromised source cannot reach the threshold alone.  
Relevant error: `Error::OracleNoConsensus = 203`.

**Defense — Callback replay prevention**  
Each oracle callback carries a nonce/timestamp checked against stored state; replays are rejected immediately.  
Relevant error: `Error::OracleCallbackReplayDetected = 213`.

**Residual risk**: An attacker who controls ≥ 2/3 of whitelisted oracle sources could still manipulate a result. Mitigation is operational: the whitelist should hold independently-operated, geographically-distributed providers.

---

### 1.2 Stale-Price Exploitation

**Threat**: An attacker triggers resolution using cached oracle data that is outdated, selecting a favorable historical price.

**Defense — Staleness validation**  
`OracleValidationConfigManager::validate_oracle_data` (`oracles.rs`, line 2428) computes `observed_age = now - data.publish_time` and rejects data exceeding the configured threshold.  
Default: `DEFAULT_MAX_STALENESS_SECS = 60` seconds (global config, `oracles.rs`, line 2346).  
Relevant error: `Error::OracleStale = 202`.

**Defense — Per-market staleness override**  
Admins may set tighter or looser windows per market via `EventOracleValidationConfig` (`types.rs`, `EventOracleValidationConfig::max_staleness_secs`), stored and resolved through `OracleValidationConfigManager::set_event_config` / `get_effective_config`. Per-market config takes precedence over the global default.

**Residual risk**: If network latency is high and the staleness window is not tightened for fast-moving markets, there is a brief window in which slightly stale data may be accepted. Operators should reduce `max_staleness_secs` for volatile assets.

---

### 1.3 Low-Confidence / Wide-Interval Manipulation

**Threat**: An attacker submits or induces an oracle reading with a very wide confidence interval, making the price meaningless while still passing staleness checks.

**Defense — Confidence-bound enforcement**  
`validate_oracle_data` computes the confidence ratio in basis points and rejects readings where `conf_bps > max_confidence_bps`.  
Default: `DEFAULT_MAX_CONFIDENCE_BPS = 500` bps (5 %).  
Relevant error: `Error::OracleConfidenceTooWide = 208`.

Per-market overrides are available through `EventOracleValidationConfig::max_confidence_bps`.

**Residual risk**: Confidence intervals are only enforced for providers that supply them (e.g. Pyth). Providers without a confidence field bypass this check; they rely on whitelist and staleness controls alone.

---

### 1.4 Oracle Unavailability / DoS

**Threat**: An attacker takes down oracle infrastructure to prevent markets from resolving, holding funds hostage.

**Defense — Fallback oracle**  
When the primary oracle is unavailable the contract falls back to a secondary source.  
Relevant error: `Error::FallbackOracleUnavailable = 206`.

**Defense — Resolution timeout**  
If neither primary nor fallback responds within the allowed window, `Error::ResolutionTimeoutReached = 207` is returned, enabling administrative recovery paths.

---

## 2. Dispute Subsystem Threats

### 2.1 Dispute Griefing (Spam)

**Threat**: An attacker floods the contract with baseless disputes against many markets to raise gas costs, lock user funds, or delay payouts.

**Defense — Minimum stake requirement**  
`DisputeUtils` and `VotingUtils` enforce `MIN_DISPUTE_STAKE = 10_000_000` stroops (1 XLM) per dispute (`config.rs`, line 293; re-exported in `voting.rs`, line 20; enforced in `disputes.rs`, line 2175).  
Relevant error: `Error::InsufficientStake = 107`.

**Defense — One dispute per market**  
`Error::AlreadyDisputed = 404` is returned if a dispute already exists for a market, preventing iterative griefing against the same market.

**Known gap — No dispute rate-limiting across markets**  
There is currently no cap on how many *different* markets a single address can dispute in a given period. A well-funded actor could still spam across many markets simultaneously. This is a tracked gap; see [issue #594](https://github.com/your-org/predictify-contracts/issues/594) for the rate-limiting work item.

---

### 2.2 Dispute Stake-Manipulation / Sybil Attack

**Threat**: An attacker creates many wallets, each staking just above `MIN_DISPUTE_STAKE`, to numerically dominate the voting tally while committing little total capital.

**Defense — Stake-weighted tally**  
Dispute outcomes are determined by `DisputeUtils::calculate_stake_weighted_outcome` (`disputes.rs`, line 1517). Raw vote *count* does not matter; each vote is weighted by its stake. A Sybil attacker spreading 10 XLM across 10 wallets has the same voting power as a single 10 XLM vote.

**Residual risk**: A well-capitalised attacker can acquire a majority stake-weight. The economic cost of doing so scales linearly with honest-voter participation; market design (high TVL, broad community) is the primary mitigation.

---

### 2.3 Tie Manipulation

**Threat**: An attacker engineers an exact stake-weighted tie to cause an indeterminate outcome and exploit the resolution path.

**Defense — Tie → oracle stands**  
`DisputeUtils` implements the rule: exact tie ⇒ the original oracle result is upheld (`disputes.rs`, line 491, `OracleIntegrationManager` result is preserved). The attacker gains nothing from a tie.  
Relevant error context: `Error::DisputeCondNotMet = 408` if resolution conditions are not satisfied.

---

### 2.4 Double-Dispute / Duplicate Vote

**Threat**: An attacker submits multiple dispute votes from the same address to inflate their stake weight.

**Defense — Single-vote enforcement**  
`VotingUtils::cast_vote` returns `Error::AlreadyVoted = 109` on a second vote from the same address; `Error::DisputeAlreadyVoted = 407` specifically guards the dispute-vote path.

**Defense — Single-dispute-per-market**  
The outer dispute creation check (`Error::AlreadyDisputed = 404`) prevents the same market being disputed twice, closing the "create a fresh dispute to revote" vector.

---

### 2.5 Voting-Window Expiry Attack

**Threat**: An attacker delays casting their vote until the window closes on the honest side, then votes just before expiry to prevent counter-votes.

**Defense — Voting window enforcement**  
`VotingUtils` enforces `DISPUTE_EXTENSION_HOURS = 24` hours (`config.rs`, line 308) as the dispute-vote deadline. Votes submitted after expiry are rejected.  
Relevant error: `Error::DisputeVoteExpired = 405`.

**Residual risk**: A 24-hour window may be insufficient for global coordination on high-value markets. Admins can extend via `dispute_extension_hours` in the voting config, but there is no automatic adaptive window.

---

## 3. Error Code Quick Reference

| Error | Code | Subsystem | Threat Mitigated |
|---|---|---|---|
| `OracleUnavailable` | 200 | Oracle | Unavailability / DoS |
| `InvalidOracleConfig` | 201 | Oracle | Misconfiguration |
| `OracleStale` | 202 | Oracle | Stale-price exploitation |
| `OracleNoConsensus` | 203 | Oracle | Feed poisoning (multi-source) |
| `MarketNotReady` | 205 | Oracle | Premature resolution |
| `FallbackOracleUnavailable` | 206 | Oracle | Unavailability / DoS |
| `ResolutionTimeoutReached` | 207 | Oracle | Unavailability / DoS |
| `OracleConfidenceTooWide` | 208 | Oracle | Low-confidence manipulation |
| `OracleCallbackAuthFailed` | 210 | Oracle | Feed poisoning (auth) |
| `OracleCallbackUnauthorized` | 211 | Oracle | Feed poisoning (whitelist) |
| `OracleCallbackInvalidSignature` | 212 | Oracle | Feed poisoning (signature) |
| `OracleCallbackReplayDetected` | 213 | Oracle | Feed poisoning (replay) |
| `OracleCallbackTimeout` | 214 | Oracle | Unavailability / DoS |
| `InsufficientStake` | 107 | Dispute | Griefing / spam |
| `AlreadyVoted` | 109 | Dispute | Double-vote |
| `AlreadyDisputed` | 404 | Dispute | Double-dispute / spam |
| `DisputeVoteExpired` | 405 | Dispute | Window expiry attack |
| `DisputeAlreadyVoted` | 407 | Dispute | Double-vote (dispute path) |
| `DisputeCondNotMet` | 408 | Dispute | Tie / condition manipulation |

---

## 4. Known Gaps and Tracking

| Gap | Description | Status |
|---|---|---|
| Dispute rate-limiting across markets | A single address can dispute arbitrarily many different markets simultaneously. No per-address, per-period dispute count cap exists. | Open — tracked in issue #594 |
| Confidence validation for non-Pyth providers | Confidence-bound checks apply only when the provider supplies a confidence field. Providers without one rely solely on whitelist and staleness. | Accepted risk — documented above |
| Adaptive voting window | The dispute window is fixed at 24 h (configurable by admin). There is no automatic extension triggered by late voting activity. | Open — future governance consideration |

---

## 5. Key Module and Constant Index

| Symbol | File | Purpose |
|---|---|---|
| `OracleIntegrationManager` | `oracles.rs:2592` | Multi-source fetch, consensus, result storage |
| `DEFAULT_CONSENSUS_THRESHOLD = 66` | `oracles.rs:2602` | 66 % majority required across sources |
| `OracleValidationConfigManager::validate_oracle_data` | `oracles.rs:2428` | Staleness and confidence validation |
| `DEFAULT_MAX_STALENESS_SECS = 60` | `oracles.rs:2346` | Global staleness window (seconds) |
| `DEFAULT_MAX_CONFIDENCE_BPS = 500` | `oracles.rs:2348` | Global confidence ceiling (basis points) |
| `EventOracleValidationConfig` | `types.rs:1864` | Per-market staleness/confidence override |
| `OracleWhitelist` | `oracles.rs:1797` | Permitted oracle contract registry |
| `MIN_DISPUTE_STAKE = 10_000_000` | `config.rs:293` | Minimum dispute stake (1 XLM in stroops) |
| `DISPUTE_EXTENSION_HOURS = 24` | `config.rs:308` | Dispute voting window duration |
| `DisputeUtils::calculate_stake_weighted_outcome` | `disputes.rs:1517` | Stake-weighted tally and tie-break rule |
| `Error` variants (all) | `err.rs` | Canonical error codes |
