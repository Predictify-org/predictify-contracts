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
