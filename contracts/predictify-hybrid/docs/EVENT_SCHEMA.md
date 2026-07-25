# Event Schema Reference

This document is the structured, canonical reference for every event emitted by the
`predictify-hybrid` contract. It covers the on-chain topic (Soroban `Symbol` key), the
payload schema, and the **stability guarantee** for each event, so that indexers,
frontends, and downstream consumers know what they can safely depend on.

Source of truth: `src/events.rs` (event types + `EventEmitter`) and
`src/event_archive.rs` (historical/archive query surface). If this document and the
source ever disagree, the source wins — please open an issue.

## How events are emitted

Two patterns exist in the codebase today:

1. **Typed struct events (majority, stable).** A `#[contracttype]` struct is built,
   optionally persisted via `EventEmitter::store_event` (for simple on-chain
   "last value" querying), and published with
   `env.events().publish((topic_symbol, ...indexed_topics), event_struct)`.
2. **Untyped tuple events (minority, unstable).** A handful of emitters skip the
   struct and publish a raw tuple of primitives as the event body. These have no
   named schema, are not persisted, and are more likely to change shape without
   notice. They are called out explicitly in [Stability Legend](#stability-legend)
   and in their own table.

## Delivery guarantees under cross-contract failure

Several emitters sit next to a cross-contract call — `GovernanceManager::execute_proposal`
invokes an arbitrary proposal target before emitting its execution event, and the
oracle resolution path calls out to `ReflectorOracleClient` before emitting its
result events. Soroban gives each invocation its own rollback frame, which fixes
what an indexer can observe when the callee fails:

| Scenario | What the indexer observes |
|---|---|
| Callee reverts (`panic_with_error!`) and the caller lets it propagate | **Nothing.** The caller's frame failed too, so events it published *before* the call are rolled back with it. |
| Callee aborts (untyped `panic!`) and the caller lets it propagate | **Nothing.** Identical rollback to a revert; only the error code surfaced to the caller differs. |
| Failure at any depth (caller → middle → callee) | **Nothing** from any frame in the failed chain. |
| Caller recovers via `env.try_invoke_contract` | The **caller's** events (both those published before the call and any failure-path event such as `fbk_used`) are observable. The **callee's** events, and its storage writes, are discarded. |

Two consequences worth designing around:

- **Events are atomic with the state change they describe.** There is no window in
  which an event is visible but its transaction's writes are not, so a consumer
  never has to compensate for a "phantom" emission.
- **The per-topic replay nonce is restored, not burned.** `EventEmitter` keeps a
  monotonic nonce per topic in persistent storage (`DataKey::EventNonce(topic)`).
  A failed call rolls that write back to its previous value, so failed attempts
  leave no gaps in the nonce sequence: consumers can still treat a gap as
  genuine data loss rather than as a reverted transaction.

These properties are pinned by `tests/xcontract.rs`. Note that a single logical
emission produces **two** published events for emitters that call `store_event`:
one from the archive helper (topics `(topic,)`) and one from the typed emission
(topics `(topic, id)`). Both carry the same event-name symbol as their first topic.

## Stability legend

| Badge | Meaning |
|---|---|
| 🟢 Stable | `#[contracttype]` struct payload, field-named, safe to depend on. Fields will only be added (not removed/retyped) without a major version bump. |
| 🟡 Stable, duplicated emitter | Struct is stable, but note there are two code paths that emit under the same conceptual name — see notes. |
| 🔴 Unstable | Raw tuple payload, no named schema, ordering-dependent. Treat as internal/debug; do not build critical indexing logic on these without confirming with maintainers first. |

Every stable event topic is a `Symbol` created with `symbol_short!("...")` (≤ 9
chars) or `Symbol::new(env, "...")` (longer names). The topic tuple's first element
is always this event-name symbol; most also index a second topic (typically
`market_id`, `admin`, or another `Address`/`Symbol`) so consumers can filter by
contract-events RPC topic filters without decoding the payload.

---

## 1. Market Lifecycle Events

| Event | Topic | Stability |
|---|---|---|
| `MarketCreatedEvent` | `("mkt_crt", market_id)` | 🟢 |
| `EventCreatedEvent` | `("evt_crt", event_id)` | 🟢 |
| `MarketClosedEvent` | `("mkt_close", market_id)` | 🟢 |
| `MarketFinalizedEvent` | `("mkt_final", market_id)` | 🟢 |
| `StateChangeEvent` | `("st_chng", market_id)` | 🟢 |
| `MarketDeadlineExtendedEvent` | `("mkt_ext", market_id)` | 🟢 |
| `MarketDescriptionUpdatedEvent` | `("mkt_dsc", market_id)` | 🟢 |
| `MarketOutcomesUpdatedEvent` | `("mkt_out", market_id)` | 🟢 |
| `CategoryUpdatedEvent` | `("mkt_cat", market_id)` | 🟢 |
| `TagsUpdatedEvent` | `("mkt_tag", market_id)` | 🟢 |
| `ExtensionRequestedEvent` | `("ext_req", market_id)` | 🟢 |
| `MinPoolSizeNotMetEvent` | `("pool_lo", market_id)` | 🟢 |
| `RefundOnOracleFailureEvent` | `("ref_oracl", market_id)` | 🟢 |

### `MarketCreatedEvent`
Emitted immediately after a new prediction market is created.

| Field | Type | Notes |
|---|---|---|
| `market_id` | `Symbol` | Unique market identifier |
| `question` | `String` | Market question text |
| `outcomes` | `Vec<String>` | Available outcomes (≥ 2) |
| `admin` | `Address` | Market admin |
| `end_time` | `u64` | Unix timestamp, market close |
| `timestamp` | `u64` | Emission timestamp |

### `EventCreatedEvent`
Same idea as `MarketCreatedEvent` but for the fee-charging "prediction event" flow.

| Field | Type | Notes |
|---|---|---|
| `event_id` | `Symbol` | Unique event identifier |
| `description` | `String` | Event description |
| `outcomes` | `Vec<String>` | Available outcomes |
| `end_time` | `u64` | Unix timestamp |
| `creation_fee_amount` | `i128` | Fee charged, in stroops |
| `admin` | `Address` | Event admin |
| `timestamp` | `u64` | Emission timestamp |

### `MarketClosedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `admin` | `Address` |
| `timestamp` | `u64` |

### `MarketFinalizedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `admin` | `Address` |
| `outcome` | `String` |
| `timestamp` | `u64` |

### `StateChangeEvent`
Fired on every market state transition (`Active → Ended → Disputed/Resolved →
Closed`, or `Any → Cancelled`).

| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `old_state` | `crate::types::MarketState` |
| `new_state` | `crate::types::MarketState` |
| `reason` | `String` |
| `timestamp` | `u64` |

### `MarketDeadlineExtendedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `old_end_time` | `u64` |
| `new_end_time` | `u64` |
| `additional_days` | `u32` |
| `admin` | `Address` |
| `reason` | `String` |
| `fee` | `i128` |
| `timestamp` | `u64` |

### `MarketDescriptionUpdatedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `old_description` | `String` |
| `new_description` | `String` |
| `admin` | `Address` |
| `timestamp` | `u64` |

### `MarketOutcomesUpdatedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `old_outcomes` | `Vec<String>` |
| `new_outcomes` | `Vec<String>` |
| `admin` | `Address` |
| `timestamp` | `u64` |

### `CategoryUpdatedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `old_category` | `Option<String>` |
| `new_category` | `Option<String>` |
| `admin` | `Address` |
| `timestamp` | `u64` |

### `TagsUpdatedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `old_tags` | `Vec<String>` |
| `new_tags` | `Vec<String>` |
| `admin` | `Address` |
| `timestamp` | `u64` |

### `ExtensionRequestedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `admin` | `Address` |
| `additional_days` | `u32` |
| `reason` | `String` |
| `fee` | `i128` |
| `timestamp` | `u64` |

### `MinPoolSizeNotMetEvent`
Emitted when a market fails to meet its configured minimum pool size at
resolution time.

| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `current_pool` | `i128` |
| `required_min` | `i128` |
| `timestamp` | `u64` |

### `RefundOnOracleFailureEvent`
Emitted after all bets on a market are refunded in full (no fee deducted)
following an oracle resolution failure or timeout. Market is marked
`Cancelled`; no further resolution is possible.

| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `total_refunded` | `i128` |
| `timestamp` | `u64` |

---

## 2. Voting & Betting Events

| Event | Topic | Stability |
|---|---|---|
| `VoteCastEvent` | `("vote", market_id)` | 🟢 |
| `BetPlacedEvent` | `("bet_plc", market_id)` | 🟢 |
| `BetStatusUpdatedEvent` | `("bet_upd", market_id)` | 🟢 |
| `BetLimitsUpdatedEvent` | `("bet_lim", scope)` | 🟢 |
| `WinningsClaimedEvent` | `("win_clm", market_id)` | 🟢 |
| `WinningsClaimedBatchEvent` | `("win_btc", user)` | 🟢 |
| `ClaimPeriodUpdatedEvent` | `("clm_prd", admin)` | 🟢 |
| `MarketClaimPeriodUpdatedEvent` | `("m_clm_pd", market_id)` | 🟢 |
| `UnclaimedWinningsSweptEvent` | `("unc_swip", market_id)` | 🟢 |

### `VoteCastEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `voter` | `Address` |
| `outcome` | `String` |
| `stake` | `i128` |
| `timestamp` | `u64` |

### `BetPlacedEvent`
A bet is a financial wager with locked funds for payout, distinct from a vote
(community-resolution input). See rustdoc in `events.rs` for the full
distinction.

| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `bettor` | `Address` |
| `outcome` | `String` |
| `amount` | `i128` |
| `timestamp` | `u64` |

### `BetStatusUpdatedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `bettor` | `Address` |
| `old_status` | `String` |
| `new_status` | `String` |
| `payout_amount` | `Option<i128>` |
| `timestamp` | `u64` |

### `BetLimitsUpdatedEvent`
`scope` is `Symbol::new(env, "global")` for global limits, or a market ID for
per-market limits.

| Field | Type |
|---|---|
| `admin` | `Address` |
| `scope` | `Symbol` |
| `min_bet` | `i128` |
| `max_bet` | `i128` |
| `timestamp` | `u64` |

### `WinningsClaimedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `user` | `Address` |
| `amount` | `i128` |
| `timestamp` | `u64` |

### `WinningsClaimedBatchEvent`
| Field | Type |
|---|---|
| `user` | `Address` |
| `market_claims` | `Vec<(Symbol, i128)>` — per-market `(market_id, amount)` |
| `total_amount` | `i128` |
| `claim_count` | `u32` |
| `timestamp` | `u64` |

### `ClaimPeriodUpdatedEvent` / `MarketClaimPeriodUpdatedEvent`
Global vs. per-market claim window updates.

| Field | Type | Applies to |
|---|---|---|
| `admin` | `Address` | both |
| `market_id` | `Symbol` | market-scoped only |
| `claim_period_seconds` | `u64` | both |
| `timestamp` | `u64` | both |

### `UnclaimedWinningsSweptEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `caller` | `Address` |
| `recipient` | `Option<Address>` — `None` when funds are burned |
| `amount` | `i128` |
| `burned` | `bool` |
| `timestamp` | `u64` |

---

## 3. Oracle Events

| Event | Topic | Stability |
|---|---|---|
| `OracleResultEvent` | `("oracle_rs", market_id)` | 🟢 |
| `OracleVerifInitiatedEvent` | `("orc_init", market_id)` | 🟢 |
| `OracleResultVerifiedEvent` | `("orc_ver", market_id)` | 🟢 |
| `OracleVerificationFailedEvent` | `("orc_fail", market_id)` | 🟢 |
| `OracleValidationFailedEvent` | `("orc_val", market_id)` | 🟢 |
| `OracleConsensusReachedEvent` | `("orc_cons", market_id)` | 🟢 |
| `OracleHealthStatusEvent` | `("orc_hlth", oracle_address)` | 🟢 |
| `FallbackUsedEvent` | `("fbk_used", market_id)` | 🟢 |
| `ResolutionTimeoutEvent` | `("res_tmo", market_id)` | 🟢 |
| `AdminOverrideEvent` | `("adm_ovrd", market_id)` | 🟢 |
| `OracleDegradationEvent` (struct) | `("ora_deg",)` | 🟡 duplicated — see note |
| `OracleRecoveryEvent` | `("ora_rec",)` | 🟢 |
| `ManualResolutionRequiredEvent` (struct) | `("man_res", market_id)` | 🟡 duplicated — see note |
| free-fn `emit_oracle_callback` | `("oracle_callback", oracle_address, feed_id, price, timestamp)` | 🔴 tuple |
| free-fn `emit_oracle_degradation` | `("oracle_degradation", oracle, reason, timestamp)` | 🔴 tuple |
| free-fn `emit_manual_resolution_required` | `("manual_resolution_required", market_id, reason, timestamp)` | 🔴 tuple |

> **Duplicated emitters:** `OracleDegradationEvent`/`OracleRecoveryEvent`/
> `ManualResolutionRequiredEvent` each have a typed `EventEmitter::emit_*` method
> **and** a free function of the same conceptual purpose (`emit_oracle_callback`,
> `emit_oracle_degradation`, `emit_manual_resolution_required` at module scope)
> that instead publishes an untyped tuple under a *different* topic name
> (`"oracle_degradation"` vs. `"ora_deg"`, etc.). Callers should use the
> `EventEmitter::emit_*` (struct) form; the free-function tuple form should be
> treated as legacy/internal and is a good candidate for deprecation in a future
> cleanup — consumers should not rely on both existing indefinitely.

### `OracleResultEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `result` | `String` |
| `provider` | `String` |
| `feed_id` | `String` |
| `price` | `i128` |
| `threshold` | `i128` |
| `comparison` | `String` (`"gt"`, `"gte"`, `"lt"`, `"lte"`, `"eq"`, etc.) |
| `timestamp` | `u64` |

### `OracleVerifInitiatedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `initiator` | `Address` |
| `feed_id` | `String` |
| `oracle_count` | `u32` |
| `timestamp` | `u64` |

### `OracleResultVerifiedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `outcome` | `String` |
| `price` | `i128` |
| `threshold` | `i128` |
| `comparison` | `String` |
| `provider` | `String` |
| `feed_id` | `String` |
| `confidence_score` | `u32` (0–100) |
| `sources_consulted` | `u32` |
| `verification_status` | `String` |
| `is_final` | `bool` |
| `timestamp` | `u64` |
| `block_number` | `u32` |

### `OracleVerificationFailedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `error_code` | `u32` |
| `error_message` | `String` |
| `attempted_providers` | `u32` |
| `fallback_available` | `bool` |
| `timestamp` | `u64` |

### `OracleValidationFailedEvent`
Fired when oracle data fails staleness or confidence-interval validation.

| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `provider` | `String` |
| `feed_id` | `String` |
| `reason` | `String` (`"stale_data"` \| `"confidence_too_wide"`) |
| `observed_age_secs` | `u64` |
| `max_age_secs` | `u64` |
| `observed_confidence_bps` | `Option<u32>` |
| `max_confidence_bps` | `u32` |
| `timestamp` | `u64` |

### `OracleConsensusReachedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `consensus_outcome` | `String` |
| `agreeing_sources` | `u32` |
| `total_sources` | `u32` |
| `agreement_percentage` | `u32` — derived as `agreeing_sources * 100 / total_sources` |
| `average_price` | `i128` |
| `price_variance` | `i128` |
| `timestamp` | `u64` |

### `OracleHealthStatusEvent`
| Field | Type |
|---|---|
| `oracle_address` | `Address` |
| `provider` | `String` |
| `previous_status` | `bool` |
| `current_status` | `bool` |
| `consecutive_failures` | `u32` |
| `timestamp` | `u64` |

### `FallbackUsedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `primary_oracle` | `Address` |
| `fallback_oracle` | `Address` |
| `timestamp` | `u64` |

### `ResolutionTimeoutEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `timeout_timestamp` | `u64` |

Note: unlike almost every other event in this contract, this struct has **no**
`timestamp: env.ledger().timestamp()` emission-time field — `timeout_timestamp`
doubles as the only time reference.

### `AdminOverrideEvent`
Emitted when an admin manually overrides an oracle-verified result.

| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `admin` | `Address` |
| `old_result` | `String` |
| `new_result` | `String` |
| `reason` | `String` |
| `timestamp` | `u64` |

### `OracleDegradationEvent` / `OracleRecoveryEvent`
| Field | Type |
|---|---|
| `oracle` | `crate::types::OracleProvider` |
| `reason` / `recovery_message` | `String` |
| `timestamp` | `u64` |

### `ManualResolutionRequiredEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `reason` | `String` |
| `timestamp` | `u64` |

---

## 4. Dispute & Governance Events

| Event | Topic | Stability |
|---|---|---|
| `DisputeCreatedEvent` | `("dispt_crt", market_id)` | 🟢 |
| `DisputeResolvedEvent` | `("dispt_res", market_id)` | 🟢 |
| `DisputeTimeoutSetEvent` | `("tout_set", dispute_id)` | 🟢 |
| `DisputeTimeoutExpiredEvent` | `("tout_exp", dispute_id)` | 🟢 |
| `DisputeTimeoutExtendedEvent` | `("tout_ext", dispute_id)` | 🟢 |
| `DisputeAutoResolvedEvent` | `("auto_res", dispute_id)` | 🟢 |
| `GovernanceProposalCreatedEvent` | `("gov_prop", proposal_id)` | 🟢 |
| `GovernanceVoteCastEvent` | `("gov_vote", proposal_id)` | 🟢 |
| `GovernanceProposalExecutedEvent` | `("gov_exec", proposal_id)` | 🟢 |

### `DisputeCreatedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `disputer` | `Address` |
| `stake` | `i128` |
| `reason` | `Option<String>` |
| `timestamp` | `u64` |

### `DisputeResolvedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `outcome` | `String` (e.g. `Dispute_Upheld`, `Dispute_Rejected`, `Dispute_Inconclusive`, `Dispute_Invalid`) |
| `winners` | `Vec<Address>` |
| `losers` | `Vec<Address>` |
| `fee_distribution` | `i128` |
| `timestamp` | `u64` |

### `DisputeTimeoutSetEvent`
| Field | Type |
|---|---|
| `dispute_id` | `Symbol` |
| `market_id` | `Symbol` |
| `timeout_hours` | `u32` |
| `set_by` | `Address` |
| `timestamp` | `u64` |

### `DisputeTimeoutExpiredEvent`
| Field | Type |
|---|---|
| `dispute_id` | `Symbol` |
| `market_id` | `Symbol` |
| `expiration_timestamp` | `u64` |
| `outcome` | `String` |
| `resolution_method` | `String` |

### `DisputeTimeoutExtendedEvent`
| Field | Type |
|---|---|
| `dispute_id` | `Symbol` |
| `market_id` | `Symbol` |
| `additional_hours` | `u32` |
| `extended_by` | `Address` |
| `timestamp` | `u64` |

### `DisputeAutoResolvedEvent`
| Field | Type |
|---|---|
| `dispute_id` | `Symbol` |
| `market_id` | `Symbol` |
| `outcome` | `String` |
| `reason` | `String` |
| `timestamp` | `u64` |

### `GovernanceProposalCreatedEvent`
| Field | Type |
|---|---|
| `proposal_id` | `Symbol` |
| `proposer` | `Address` |
| `title` | `String` |
| `description` | `String` |
| `timestamp` | `u64` |

### `GovernanceVoteCastEvent`
| Field | Type |
|---|---|
| `proposal_id` | `Symbol` |
| `voter` | `Address` |
| `support` | `bool` |
| `timestamp` | `u64` |

### `GovernanceProposalExecutedEvent`
| Field | Type |
|---|---|
| `proposal_id` | `Symbol` |
| `executor` | `Address` |
| `timestamp` | `u64` |

---

## 5. Fees & Treasury Events

| Event | Topic | Stability |
|---|---|---|
| `FeeCollectedEvent` | `("fee_col", market_id)` | 🟢 |
| `FeeWithdrawalAttemptEvent` | `("fwd_att", admin)` | 🟢 |
| `FeeWithdrawnEvent` | `("fwd_ok", admin)` | 🟢 |
| `PlatformFeeSetEvent` | `("platform_fee_set",)` | 🟢 |
| `TreasuryUpdatedEvent` | `("treas_up", admin)` | 🟢 |

### `FeeCollectedEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `collector` | `Address` |
| `amount` | `i128` |
| `fee_type` | `String` |
| `timestamp` | `u64` |

### `FeeWithdrawalAttemptEvent`
Emitted on **every** admin call to withdraw fees, including blocked attempts
(timelock not satisfied, no fees available, etc.) — this is an audit trail, not
just a success log.

| Field | Type |
|---|---|
| `admin` | `Address` |
| `requested_amount` | `i128` — `0` means "withdraw max" |
| `available_fees` | `i128` |
| `withdrawal_amount` | `i128` — `0` if blocked |
| `status` | `crate::fees::FeeWithdrawalStatus` |
| `last_withdrawal_ts` | `u64` — `0` if never |
| `next_allowed_ts` | `u64` — `0` if never withdrawn yet |
| `timelock_seconds` | `u64` |
| `max_withdrawal_bps` | `u32` |
| `timestamp` | `u64` |

### `FeeWithdrawnEvent`
| Field | Type |
|---|---|
| `admin` | `Address` |
| `amount` | `i128` |
| `remaining_fees` | `i128` |
| `timestamp` | `u64` |

### `PlatformFeeSetEvent`
| Field | Type |
|---|---|
| `fee_percentage` | `i128` |
| `set_by` | `Address` |
| `timestamp` | `u64` |

### `TreasuryUpdatedEvent`
| Field | Type |
|---|---|
| `admin` | `Address` |
| `treasury` | `Address` |
| `timestamp` | `u64` |

---

## 6. Admin, Config & Access Control Events

| Event | Topic | Stability |
|---|---|---|
| `ContractInitializedEvent` | `("contract_initialized",)` | 🟢 |
| `ConfigInitializedEvent` | `("cfg_init", admin)` | 🟢 |
| `ConfigUpdatedEvent` | `("cfg_upd", updated_by)` | 🟢 |
| `AdminInitializedEvent` | `("adm_init", admin)` | 🟢 |
| `AdminTransferredEvent` | `("adm_xfer", new_admin)` | 🟢 |
| `AdminRoleEvent` (assigned) | `("adm_role", admin)` | 🟢 |
| `AdminRoleEvent` (deactivated) | `("adm_deact", admin)` | 🟡 same struct, different topic |
| `AdminActionEvent` | `("adm_act", admin)` | 🟢 |
| `AdminPermissionEvent` | *(not published — see note)* | 🔴 dead code |
| `ContractPausedEvent` | `("ctr_pause", admin)` | 🟢 |
| `ContractUnpausedEvent` | `("ctr_unp", admin)` | 🟢 |

> `AdminPermissionEvent` is defined as a `#[contracttype]` struct but has **no**
> corresponding `EventEmitter::emit_*` method — it is currently unused/unreachable
> from the emission layer. Flagging for maintainers: either wire it up or remove
> it as dead code.

> `AdminRoleEvent` is reused for two distinct actions (`emit_admin_role_assigned`
> and `emit_admin_role_deactivated`) that publish under different topics
> (`adm_role` vs `adm_deact`) but share the same payload shape, where the `role`
> field is overloaded to also carry the literal string `"deactivated"` on the
> deactivation path. Consumers should key off the **topic**, not the `role`
> field, to distinguish the two.

### `ContractInitializedEvent`
| Field | Type |
|---|---|
| `admin` | `Address` |
| `platform_fee_percentage` | `i128` |
| `timestamp` | `u64` |

### `ConfigInitializedEvent`
| Field | Type |
|---|---|
| `admin` | `Address` |
| `environment` | `String` (`Development` \| `Testnet` \| `Mainnet` \| `Custom`) |
| `timestamp` | `u64` |

### `ConfigUpdatedEvent`
| Field | Type |
|---|---|
| `updated_by` | `Address` |
| `config_type` | `String` |
| `old_value` | `String` |
| `new_value` | `String` |
| `timestamp` | `u64` |

### `AdminInitializedEvent`
| Field | Type |
|---|---|
| `admin` | `Address` |
| `timestamp` | `u64` |

### `AdminTransferredEvent`
| Field | Type |
|---|---|
| `previous_admin` | `Address` |
| `new_admin` | `Address` |
| `timestamp` | `u64` |

### `AdminRoleEvent`
| Field | Type |
|---|---|
| `admin` | `Address` |
| `role` | `String` (`Owner` \| `Admin` \| `Moderator` \| `deactivated`) |
| `assigned_by` | `Address` |
| `timestamp` | `u64` |

### `AdminActionEvent`
| Field | Type |
|---|---|
| `admin` | `Address` |
| `action` | `String` |
| `target` | `Option<String>` |
| `timestamp` | `u64` |
| `success` | `bool` |

### `AdminPermissionEvent` (dead code — see note above)
| Field | Type |
|---|---|
| `admin` | `Address` |
| `permission` | `String` |
| `granted` | `bool` |
| `timestamp` | `u64` |

### `ContractPausedEvent` / `ContractUnpausedEvent`
| Field | Type |
|---|---|
| `admin` | `Address` |
| `timestamp` | `u64` |

---

## 7. Upgrade & Storage Events

| Event | Topic | Stability |
|---|---|---|
| `ContractUpgradedEvent` | `("up_grade", upgrade_id)` | 🟢 |
| `ContractRollbackEvent` | `("rollback",)` | 🟢 |
| `UpgradeProposalCreatedEvent` | `("up_prop", proposal_id)` | 🟢 |
| `StorageCleanupEvent` | `("stor_cln", market_id)` | 🟢 |
| `StorageOptimizationEvent` | `("stor_opt", market_id)` | 🟢 |
| `StorageMigrationEvent` | `("stor_mig", migration_id)` | 🟢 |

### `ContractUpgradedEvent`
| Field | Type |
|---|---|
| `old_wasm_hash` | `BytesN<32>` |
| `new_wasm_hash` | `BytesN<32>` |
| `upgrade_id` | `Symbol` |
| `timestamp` | `u64` |

### `ContractRollbackEvent`
| Field | Type |
|---|---|
| `current_wasm_hash` | `BytesN<32>` |
| `rollback_wasm_hash` | `BytesN<32>` |
| `timestamp` | `u64` |

### `UpgradeProposalCreatedEvent`
| Field | Type |
|---|---|
| `proposal_id` | `Symbol` |
| `proposer` | `Address` |
| `target_version` | `String` |
| `timestamp` | `u64` |

### `StorageCleanupEvent` / `StorageOptimizationEvent`
| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `cleanup_type` / `optimization_type` | `String` |
| `timestamp` | `u64` |

### `StorageMigrationEvent`
| Field | Type |
|---|---|
| `migration_id` | `Symbol` |
| `from_format` | `String` |
| `to_format` | `String` |
| `markets_migrated` | `u32` |
| `timestamp` | `u64` |

---

## 8. Monitoring, Errors & Diagnostics

| Event | Topic | Stability |
|---|---|---|
| `ErrorLoggedEvent` | `("err_log",)` | 🟢 |
| `ErrorRecoveryEvent` | `("err_rec",)` | 🟢 |
| `PerformanceMetricEvent` | `("perf_met",)` | 🟢 |
| `StatisticsUpdatedEvent` | `("stats_upd",)` | 🟢 |
| `CircuitBreakerEvent` | *(persisted only — see note)* | 🟡 not published |
| `emit_diagnostic_event` (uses `ErrorLoggedEvent` shape) | `("err_evt",)` | 🟢 |
| free-fn `emit_security_event` | `("security_event", actor, message, timestamp)` | 🔴 tuple |

> **`CircuitBreakerEvent` is never published to the ledger event stream.**
> `EventEmitter::emit_circuit_breaker_event` only calls `Self::store_event`
> (persistent storage), which itself *also* calls `env.events().publish` — but
> only under the *generic* topic `("cb_event",)`, publishing the struct with no
> market/admin-scoped topic. This means circuit-breaker events **are** technically
> on the event stream (via `store_event`'s internal publish), just always under
> the flat `cb_event` topic rather than a scoped one like every other event in
> this contract. Consumers filtering by a second topic element will miss these.

### `ErrorLoggedEvent`
| Field | Type |
|---|---|
| `error_code` | `u32` |
| `message` | `String` |
| `context` | `String` |
| `user` | `Option<Address>` |
| `market_id` | `Option<Symbol>` |
| `timestamp` | `u64` |

### `ErrorRecoveryEvent`
| Field | Type |
|---|---|
| `error_code` | `u32` |
| `recovery_strategy` | `String` |
| `recovery_status` | `String` |
| `recovery_attempts` | `u32` |
| `user` | `Option<Address>` |
| `market_id` | `Option<Symbol>` |
| `timestamp` | `u64` |

### `PerformanceMetricEvent`
| Field | Type |
|---|---|
| `metric_name` | `String` |
| `value` | `i128` |
| `unit` | `String` |
| `context` | `String` |
| `timestamp` | `u64` |

### `StatisticsUpdatedEvent`
| Field | Type |
|---|---|
| `total_volume` | `i128` |
| `total_bets` | `u64` |
| `active_markets` | `u32` |
| `timestamp` | `u64` |

### `CircuitBreakerEvent`
| Field | Type |
|---|---|
| `action` | `crate::circuit_breaker::BreakerAction` |
| `condition` | `crate::circuit_breaker::BreakerCondition` |
| `reason` | `String` |
| `timestamp` | `u64` |
| `admin` | `Option<Address>` — `None` if automatic |

---

## 9. Unstable / Tuple-Payload Events (🔴)

These publish raw tuples rather than named `#[contracttype]` structs. Field
order matters and there is no schema to validate against; treat as
internal/debug signals only.

| Emitter | Topic | Payload (positional) |
|---|---|---|
| `emit_balance_changed` | `("bal_chg", user, asset)` | `(operation: String, amount: i128, new_balance: i128, timestamp: u64)` |
| `emit_event_visibility_set` | `("evt_vis", event_id)` | `(visibility: crate::types::EventVisibility, admin: Address, timestamp: u64)` |
| `emit_allowlist_updated` | `("allowlst", event_id)` | `(addresses: Vec<Address>, admin: Address, timestamp: u64)` |
| `emit_oracle_callback` (free fn + duplicate method) | `("oracle_callback", oracle_address, feed_id, price, timestamp)` | `()` — all data is in the topic tuple itself |
| `emit_security_event` (free fn + duplicate method) | `("security_event", actor, message, timestamp)` | `()` |
| `emit_oracle_degradation` (free fn) | `("oracle_degradation", oracle, reason, timestamp)` | `()` |
| `emit_manual_resolution_required` (free fn) | `("manual_resolution_required", market_id, reason, timestamp)` | `()` |

These are strong candidates for migration to typed `#[contracttype]` structs in
a follow-up — tracked as a suggested improvement, not part of this issue's scope.

---

## 10. Historical / Archive Query Surface (`event_archive.rs`)

Not a ledger event, but the primary way past events (resolved/cancelled markets)
are queried after the fact. Documented here for completeness since it's the
counterpart to the live event stream above.

- **`EventArchive::archive_event(env, admin, market_id) -> Result<(), Error>`**
  Admin-only. Marks a `Resolved`/`Cancelled` market as archived. Fails with
  `Error::ArchiveFull` once `MAX_ARCHIVE_SIZE` (1,000 entries) is reached; call
  `prune_archive` first.
- **`EventArchive::prune_archive(env, admin, count) -> Result<u32, Error>`**
  Admin-only. Removes the oldest `count` archived entries (capped at
  `MAX_QUERY_LIMIT` = 30 per call) to free capacity.
- **`EventArchive::is_archived(env, market_id) -> bool`**
- **`EventArchive::archive_size(env) -> u32`**
- **`EventArchive::query_events_history(env, from_ts, to_ts, cursor, limit) -> (Vec<EventHistoryEntry>, u32)`**
  Paginated, filtered by inclusive creation-time range.
- **`EventArchive::query_events_by_resolution_status(env, status, cursor, limit) -> (Vec<EventHistoryEntry>, u32)`**
- **`EventArchive::query_events_by_category(env, category, cursor, limit) -> (Vec<EventHistoryEntry>, u32)`**
  Matches the dedicated `category` field if set, else falls back to
  `oracle_config.feed_id`.
- **`EventArchive::query_events_by_tags(env, tags, cursor, limit) -> (Vec<EventHistoryEntry>, u32)`**
  OR logic across tags; empty tag list returns an empty result immediately.

All query functions cap `limit` at `MAX_QUERY_LIMIT = 30` and return
`(entries, next_cursor)` — callers should paginate until `next_cursor` stops
advancing.

`EventHistoryEntry` exposes **public metadata and outcome only** — no votes,
stakes, or per-user addresses — by design (see module doc comment in
`event_archive.rs`).

| Field | Type |
|---|---|
| `market_id` | `Symbol` |
| `question` | `String` |
| `outcomes` | `Vec<String>` |
| `end_time` | `u64` |
| `created_at` | `u64` |
| `state` | `MarketState` |
| `winning_outcome` | derived from `Market::get_winning_outcome()` |
| `total_staked` | `i128` |
| `archived_at` | `Option<u64>` |
| `category` | `String` |
| `tags` | `Vec<String>` |

---

## Stability Policy

- 🟢 **Stable** event structs follow additive-only evolution: new fields may be
  appended, but existing fields will not be renamed, retyped, reordered, or
  removed without a documented breaking-change notice and version bump.
- 🔴 **Unstable** tuple-payload events carry no such guarantee and may change
  shape at any time. Do not build critical infrastructure on them.
- Any new event added to this contract **must** be added to this document in
  the same PR (enforced informally via review; see `event_creation_tests.rs`
  for programmatic coverage of emission behavior).
