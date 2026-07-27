# Requirements Document

## Introduction

The `predictify-hybrid` contract resolves prediction markets by consulting a primary oracle and,
when configured, a fallback oracle. Currently, when the two oracle sources return data with
conflicting staleness characteristics — for example, the primary data is fresh while the fallback
data is stale, or both sources are stale — the contract does not emit a dedicated on-chain event
to signal this cross-oracle staleness condition. Downstream consumers (indexers, monitoring
dashboards, dispute tools) therefore have no structured, filterable signal indicating that a
multi-source staleness anomaly occurred during a resolution attempt.

This feature adds a new `CrossOracleStalenessEvent` Soroban contract event that is emitted
precisely when a cross-oracle staleness mismatch is detected in `OracleResolutionManager::fetch_oracle_result`
and in the median-resolution path (`resolve_with_median`). It also adds a corresponding emitter
method to `EventEmitter`, registers the event in `EventSchemaRegistry`, documents the new event in
`docs/EVENT_SCHEMA.md`, and provides focused tests covering the happy-path and all staleness
mismatch scenarios.

---

## Glossary

- **The Contract**: The `predictify-hybrid` Soroban smart contract.
- **Primary_Oracle**: The oracle source identified by `market.oracle_config`.
- **Fallback_Oracle**: The optional secondary oracle source identified by
  `market.fallback_oracle_config`, present when `market.has_fallback == true`.
- **Cross-Oracle Staleness**: The condition where at least one oracle source's price data exceeds
  the effective `max_staleness_secs` threshold _and_ a second oracle source is present, so the
  staleness state differs across the two sources (one fresh, one stale; or both stale).
- **OraclePriceData**: The struct returned by `OracleInterface::get_price_data`, containing
  `price`, `confidence`, and `publish_time` fields.
- **Staleness Age**: `env.ledger().timestamp().saturating_sub(publish_time)` for a given
  `OraclePriceData` value.
- **Effective Config**: The per-event `EventOracleValidationConfig` if set; otherwise the global
  `GlobalOracleValidationConfig`. Resolved by `OracleValidationConfigManager::get_effective_config`.
- **EventEmitter**: The `EventEmitter` struct in `src/events.rs` that centralises all on-chain
  event emission.
- **EventSchemaRegistry**: The `EventSchemaRegistry` struct in `src/events.rs` that maps event
  names to topic symbols and schema versions.
- **topic symbol**: The ≤ 9-character `Symbol` used as the first element of the Soroban event
  topic tuple, created via `symbol_short!`.
- **nonce**: The per-topic monotonically-increasing replay-protection counter managed by
  `EventEmitter::get_and_increment_nonce`.
- **OracleResultEvent**: The existing `#[contracttype]` struct emitted after a successful oracle
  fetch, under topic `"oracle_rs"`.
- **OracleValidationFailedEvent**: The existing `#[contracttype]` struct emitted when a single
  oracle source fails staleness or confidence validation, under topic `"orc_val"`.
- **CrossOracleStalenessEvent**: The new `#[contracttype]` struct introduced by this feature,
  emitted under topic `"orc_xstl"` when a cross-oracle staleness condition is detected.

---

## Requirements

### Requirement 1: CrossOracleStalenessEvent struct

**User Story:** As a market indexer operator, I want a named, versioned on-chain event struct
whenever two oracle sources have mismatched staleness, so that I can filter and alert on this
condition without parsing untyped raw data.

#### Acceptance Criteria

1. THE Contract SHALL define a `CrossOracleStalenessEvent` struct annotated with `#[contracttype]`
   and `#[derive(Clone, Debug, Eq, PartialEq)]` in `src/events.rs`.

2. THE `CrossOracleStalenessEvent` SHALL contain the following fields exactly:
   - `market_id: Symbol` — identifier of the market being resolved
   - `primary_provider: String` — display name of the Primary_Oracle provider
   - `primary_feed_id: String` — feed ID used by the Primary_Oracle
   - `primary_age_secs: u64` — Staleness Age of the Primary_Oracle's data at detection time
   - `fallback_provider: String` — display name of the Fallback_Oracle provider
   - `fallback_feed_id: String` — feed ID used by the Fallback_Oracle
   - `fallback_age_secs: u64` — Staleness Age of the Fallback_Oracle's data at detection time
   - `max_age_secs: u64` — the `max_staleness_secs` from the Effective Config at detection time
   - `nonce: u64` — replay-protection nonce
   - `timestamp: u64` — `env.ledger().timestamp()` at emission time

3. IF any field listed in criterion 2 is absent or has a different type, THEN THE Contract SHALL
   fail to compile, preventing deployment of a malformed event schema.

---

### Requirement 2: EventEmitter emission method

**User Story:** As a contract developer integrating new oracle resolution paths, I want a single
`EventEmitter::emit_cross_oracle_staleness` function, so that all call sites emit the event
consistently without duplicating struct construction logic.

#### Acceptance Criteria

1. THE `EventEmitter` SHALL expose a public method with the signature:
   ```
   pub fn emit_cross_oracle_staleness(
       env: &Env,
       market_id: &Symbol,
       primary_provider: &String,
       primary_feed_id: &String,
       primary_age_secs: u64,
       fallback_provider: &String,
       fallback_feed_id: &String,
       fallback_age_secs: u64,
       max_age_secs: u64,
   )
   ```

2. WHEN `emit_cross_oracle_staleness` is called, THE Contract SHALL build a
   `CrossOracleStalenessEvent` struct populated with all provided parameters, with `nonce` set
   by `EventEmitter::get_and_increment_nonce` using the topic symbol for `"orc_xstl"`, and
   `timestamp` set to `env.ledger().timestamp()`.

3. WHEN `emit_cross_oracle_staleness` is called, THE Contract SHALL persist the event via
   `Self::store_event(env, &symbol_short!("orc_xstl"), &event)`.

4. WHEN `emit_cross_oracle_staleness` is called, THE Contract SHALL publish the event to the
   Soroban ledger stream via
   `env.events().publish((symbol_short!("orc_xstl"), market_id.clone()), event)`.

5. THE topic symbol for `CrossOracleStalenessEvent` SHALL be `symbol_short!("orc_xstl")`.

---

### Requirement 3: Detection in the dual-oracle resolution path

**User Story:** As a market participant, I want the contract to emit `CrossOracleStalenessEvent`
whenever the dual-oracle path detects a staleness mismatch across sources, so that I can be
alerted that resolution data quality may be degraded.

#### Acceptance Criteria

1. WHEN `OracleResolutionManager::fetch_oracle_result` fetches price data from both the
   Primary_Oracle and the Fallback_Oracle and the Staleness Age of either source exceeds
   `max_staleness_secs` from the Effective Config, THEN THE Contract SHALL call
   `EventEmitter::emit_cross_oracle_staleness` before returning from the function.

2. WHEN only one oracle source has a Staleness Age that exceeds `max_staleness_secs` (i.e., a
   partial staleness mismatch), THEN THE Contract SHALL still emit `CrossOracleStalenessEvent`.

3. WHEN both oracle sources have a Staleness Age that exceeds `max_staleness_secs`, THEN THE
   Contract SHALL emit `CrossOracleStalenessEvent` exactly once for the resolution call.

4. WHEN only the primary oracle is available (i.e., `market.has_fallback == false`), THEN THE
   Contract SHALL NOT emit `CrossOracleStalenessEvent`, because no cross-source comparison is
   possible.

5. WHEN `fetch_oracle_result` is called and the Primary_Oracle fetch itself returns an error
   before price data is available, THEN THE Contract SHALL NOT emit `CrossOracleStalenessEvent`
   for the primary source.

6. THE emission of `CrossOracleStalenessEvent` SHALL NOT alter the existing return value or
   error behaviour of `fetch_oracle_result`; cross-oracle staleness is an observability signal,
   not a resolution blocker.

---

### Requirement 4: Detection in the median-resolution path

**User Story:** As an on-chain analytics consumer, I want `CrossOracleStalenessEvent` to also be
emitted when the multi-source median resolution path detects cross-oracle staleness, so that
monitoring is consistent across both resolution strategies.

#### Acceptance Criteria

1. WHEN `OracleResolutionManager::resolve_with_median` collects price quotes from multiple oracle
   sources and the Staleness Age of any included quote exceeds `max_staleness_secs` from the
   Effective Config while at least one other quote is within the staleness limit, THEN THE
   Contract SHALL call `EventEmitter::emit_cross_oracle_staleness` once per resolution call.

2. WHEN all included quotes in `resolve_with_median` exceed `max_staleness_secs`, THEN THE
   Contract SHALL emit `CrossOracleStalenessEvent` exactly once, using the first two quotes as
   representative primary and fallback sources.

3. THE emission of `CrossOracleStalenessEvent` in `resolve_with_median` SHALL NOT alter the
   existing return value or error behaviour of that function.

---

### Requirement 5: EventSchemaRegistry registration

**User Story:** As a contract integrator using `EventSchemaRegistry::get_schema`, I want the new
event to be registered in the registry, so that I can discover its canonical topic symbol and
schema version programmatically.

#### Acceptance Criteria

1. THE `EventSchemaRegistry::get_schema` function SHALL return a valid `EventSchemaEntry` when
   called with the name `"cross_oracle_staleness"`.

2. THE returned `EventSchemaEntry` SHALL have `topic` equal to `symbol_short!("orc_xstl")` and
   `schema_version` equal to `1`.

3. IF `"cross_oracle_staleness"` is not registered and `get_schema` falls back to the default
   branch, THEN THE Contract SHALL still return an `EventSchemaEntry` consistent with
   `topic = symbol_short!("orc_xstl")` and `schema_version = 1`.

---

### Requirement 6: Documentation update

**User Story:** As a downstream consumer building an indexer, I want `docs/EVENT_SCHEMA.md` to
document `CrossOracleStalenessEvent`, so that I know its topic, all fields, and stability
guarantee without reading source code.

#### Acceptance Criteria

1. THE file `contracts/predictify-hybrid/docs/EVENT_SCHEMA.md` SHALL include a row for
   `CrossOracleStalenessEvent` in the Oracle Events table with topic `"orc_xstl"` and stability
   badge 🟢 Stable.

2. THE file SHALL include a `### CrossOracleStalenessEvent` subsection that lists every field
   from the struct defined in Requirement 1 criterion 2, including field name, type, and a
   one-line description.

3. THE documentation SHALL state that the event is emitted in both `fetch_oracle_result` and
   `resolve_with_median` when a cross-oracle staleness mismatch is detected.

4. THE documentation SHALL state that emitting this event does not block or alter market
   resolution.

---

### Requirement 7: Tests

**User Story:** As a code reviewer, I want focused tests that prove each staleness scenario
triggers (or suppresses) the event correctly, so that I can approve the PR with confidence.

#### Acceptance Criteria

1. THE codebase SHALL include a test named `test_cross_oracle_staleness_event_emitted_when_primary_stale`
   that verifies `CrossOracleStalenessEvent` is emitted when the Primary_Oracle data is stale
   and the Fallback_Oracle data is fresh.

2. THE codebase SHALL include a test named `test_cross_oracle_staleness_event_emitted_when_fallback_stale`
   that verifies `CrossOracleStalenessEvent` is emitted when the Fallback_Oracle data is stale
   and the Primary_Oracle data is fresh.

3. THE codebase SHALL include a test named `test_cross_oracle_staleness_event_emitted_when_both_stale`
   that verifies `CrossOracleStalenessEvent` is emitted exactly once when both oracle sources
   exceed `max_staleness_secs`.

4. THE codebase SHALL include a test named `test_cross_oracle_staleness_event_not_emitted_single_oracle`
   that verifies `CrossOracleStalenessEvent` is NOT emitted when only a single oracle source is
   present (`has_fallback == false`).

5. THE codebase SHALL include a test named `test_cross_oracle_staleness_event_not_emitted_both_fresh`
   that verifies `CrossOracleStalenessEvent` is NOT emitted when both oracle sources return data
   within `max_staleness_secs`.

6. WHEN any test listed in criteria 1–5 asserts that the event IS emitted, THE test SHALL verify
   the following fields carry correct values: `market_id`, `primary_age_secs`,
   `fallback_age_secs`, `max_age_secs`, and `timestamp`.

7. THE tests SHALL use the Soroban SDK's `env.events().all()` or equivalent introspection API to
   assert event emission without relying on side effects in persistent storage.

8. WHEN adding the new tests, THE codebase SHALL continue to compile and all existing tests SHALL
   pass without modification.

---

### Requirement 8: Code style and lint compliance

**User Story:** As a code reviewer, I want the new code to be indistinguishable in style from the
existing `events.rs` and `resolution.rs` modules, so that the PR diff is easy to read and
review.

#### Acceptance Criteria

1. THE new `CrossOracleStalenessEvent` struct SHALL follow the same rustdoc comment style used
   by neighbouring event structs in `src/events.rs` (doc comment on the struct, per-field doc
   comments).

2. THE `emit_cross_oracle_staleness` method SHALL be placed in the `EventEmitter` impl block
   in alphabetical or logical order relative to neighbouring oracle-related emit methods.

3. WHEN `cargo clippy` is run on the workspace, THE new code SHALL produce zero new warnings
   or errors.

4. WHEN `cargo fmt --check` is run on the workspace, THE new code SHALL produce no formatting
   violations.
