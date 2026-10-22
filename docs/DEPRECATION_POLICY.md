# Deprecation Policy



## Overview

This document describes the deprecation policy for the Predictify Hybrid contract. As the platform evolves, certain entrypoints become obsolete and need to be phased out. A structured deprecation process ensures that callers have adequate notice and can migrate smoothly.

## Lifecycle Stages

| Stage | Attribute | Behaviour |
|-------|-----------|-----------|
| **Active** | (none) | Full support, recommended for all callers |
| **Deprecated** | `#[deprecated]` + `DeprecatedCall` event | Function still works but emits a runtime deprecation event; slated for removal |
| **Removed** | n/a | Function deleted; callers must use the replacement |

## Deprecation Process

1. **Marking**: The entrypoint is annotated with `#[deprecated(note = "...")]` and emits a `DeprecatedCall` event on every invocation.
2. **Runtime Signal**: Callers receive a `DeprecatedCall(entrypoint: Symbol)` event in the Soroban ledger metadata. Indexers can use this event to monitor usage decay over time.
3. **Communication**: The deprecation note in the attribute points to the recommended replacement. A changelog entry is added at the time of marking.
4. **Removal**: After a minimum notice period (typically one major version cycle), the entrypoint may be removed.

## Emitting the Signal

Use the `emit_deprecated` helper from `events.rs`:

```rust
use crate::events::emit_deprecated;

#[deprecated(note = "Use new_function instead")]
pub fn legacy_function(env: Env, /* ... */) -> Result<(), Error> {
    emit_deprecated(&env, &Symbol::new(&env, "legacy_function"));
    // ... original logic unchanged ...
}
```

## Currently Deprecated Entrypoints

| Entrypoint | Replacement | Deprecated Since | Note |
|------------|-------------|------------------|------|
| `verify_result` | `fetch_oracle_result` | 2026-06-28 | Legacy oracle verification stub; always returns `OracleUnavailable` |
| `resolve_market` | `resolve_market_manual` | 2026-06-28 | Legacy resolution stub; only records statistics |

## Migration Guide

### `verify_result` → `fetch_oracle_result`

Old:
```rust
PredictifyHybrid::verify_result(env.clone(), caller, market_id);
```

New:
```rust
PredictifyHybrid::fetch_oracle_result(env.clone(), caller, market_id);
```

### `resolve_market` → `resolve_market_manual`

Old:
```rust
PredictifyHybrid::resolve_market(env.clone(), market_id);
```

New:
```rust
PredictifyHybrid::resolve_market_manual(env.clone(), admin, market_id, outcome);
```

## Testing

Deprecation behaviour is covered by tests in `events.rs`:

- `test_emit_deprecated_call` — verifies the event publishes without panic
- `test_emit_deprecated_call_stores_entrypoint` — verifies the entrypoint symbol is passed through

## Deprecated-Entrypoints Registry

As of the `task/deprecated-registry` feature branch, every deprecated entrypoint is also
recorded in a **persistent, on-chain registry** (`contracts/predictify-hybrid/src/deprecated.rs`).

### Registry API

All write operations require the contract admin; read operations are permissionless.

| Entrypoint                  | Caller      | Description                                                  |
|-----------------------------|-------------|--------------------------------------------------------------|
| `register_deprecated`       | admin-only  | Add an entry (idempotent)                                    |
| `remove_deprecated`         | admin-only  | Remove an entry (no-op if absent)                            |
| `get_deprecated_entry`      | anyone      | Look up one entry by name → `Option<DeprecatedEntry>`        |
| `list_deprecated_entries`   | anyone      | Return the full registry as `Vec<DeprecatedEntry>`           |
| `deprecated_entry_count`    | anyone      | Return the number of registered entries                      |
| `is_deprecated`             | anyone      | Boolean check for a single entrypoint                        |

### DeprecatedEntry fields

| Field         | Type              | Description                                           |
|---------------|-------------------|-------------------------------------------------------|
| `entrypoint`  | `Symbol`          | Name of the deprecated function                       |
| `replacement` | `Symbol`          | Name of the recommended replacement                   |
| `since`       | `u64`             | Ledger timestamp (seconds) when registered            |
| `note`        | `Option<String>`  | Optional migration hint (max 128 bytes UTF-8)         |

### Capacity

The registry is capped at `MAX_REGISTRY_ENTRIES` = **64** entries to bound gas usage.
Attempts to exceed this limit return `Error::RegistryFull` (528).

### Events

| Topic           | Payload              | When                                      |
|-----------------|----------------------|-------------------------------------------|
| `depr_reg`      | ledger timestamp     | Emitted on successful `register_deprecated` |
| `depr_rem`      | ledger timestamp     | Emitted on successful `remove_deprecated` when entry was present |
| `depr_call`     | `DeprecatedCall`     | Emitted by every deprecated entrypoint call (via `record_call`) |

### Usage in deprecated entrypoints

Replace direct `emit_deprecated` calls with `DeprecatedRegistry::record_call`:

```rust
use crate::deprecated::DeprecatedRegistry;

#[deprecated(note = "Use new_function instead")]
pub fn legacy_function(env: Env, /* ... */) -> Result<(), Error> {
    DeprecatedRegistry::record_call(
        &env,
        &Symbol::new(&env, "legacy_function"),
        &Symbol::new(&env, "new_function"),
    );
    // ... original logic unchanged ...
}
```

