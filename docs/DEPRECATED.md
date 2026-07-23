# Deprecated Entrypoints Reference

> **Source of truth**: [`contracts/predictify-hybrid/src/deprecated.rs`](../contracts/predictify-hybrid/src/deprecated.rs)

This document lists every deprecated entrypoint tracked by the `DeprecatedRegistry`.
For the overall deprecation lifecycle and policy, see [DEPRECATION_POLICY.md](DEPRECATION_POLICY.md).

---

## Registry Overview

The `DeprecatedRegistry` is a compile-time, storage-free registry that mirrors
the pattern established by `EventSchemaRegistry` in `events.rs`.  Every
deprecated function is registered with:

| Field             | Type     | Description                                      |
|-------------------|----------|--------------------------------------------------|
| `name`            | `Symbol` | Exact function name                              |
| `replacement`     | `Symbol` | Recommended replacement (shortened for Symbol)   |
| `since`           | `Symbol` | ISO-8601 date the deprecation was introduced     |
| `removal_version` | `Symbol` | Contract version for planned removal (`"TBD"` if not yet scheduled) |

---

## Currently Deprecated Entrypoints

### Contract-Level Entrypoints (`#[contractimpl]`)

| Function | Replacement | Deprecated Since | Removal Version | Notes |
|----------|-------------|------------------|-----------------|-------|
| `verify_result` | [`fetch_oracle_result`](../contracts/predictify-hybrid/src/lib.rs) | 2026-06-28 | TBD | Legacy oracle verification stub; always returns `OracleUnavailable`. |
| `resolve_market` | [`resolve_market_manual`](../contracts/predictify-hybrid/src/lib.rs) | 2026-06-28 | TBD | Legacy resolution stub; only records statistics and invalidates cache. |

### Module-Internal Deprecated Wrappers

These functions are not exposed as `#[contractimpl]` entrypoints but are
tracked in the registry because their doc comments mark them as deprecated.

| Function | Module | Replacement (fully-qualified) | Deprecated Since | Removal Version |
|----------|--------|-------------------------------|------------------|-----------------|
| `collect_fees` | `voting.rs` — `VotingManager` | `crate::fees::FeeManager::collect_fees` | 2026-06-28 | TBD |
| `transfer_fees` | `voting.rs` — `VotingUtils` | `crate::fees::FeeUtils::transfer_fees_to_admin` | 2026-06-28 | TBD |
| `calculate_fee_amount` | `voting.rs` — `VotingUtils` | `crate::fees::FeeCalculator::calculate_platform_fee` | 2026-06-28 | TBD |
| `process_creation_fee` | `markets.rs` — `MarketUtils` | `crate::fees::FeeManager::process_creation_fee` | 2026-06-28 | TBD |

---

## Querying the Registry

### On-Chain (Contract Call)

```rust
// Read-only entrypoint — no auth required
let entries = PredictifyHybrid::list_deprecated(env.clone());
for entry in entries.iter() {
    // entry.name, entry.replacement, entry.since, entry.removal_version
}
```

### In Rust Code

```rust
use crate::deprecated::DeprecatedRegistry;

// Look up a single function
if let Some(entry) = DeprecatedRegistry::lookup(&env, "verify_result") {
    log!("Use {} instead", entry.replacement);
}

// Check if a function is deprecated
if DeprecatedRegistry::is_deprecated(&env, "resolve_market") {
    // ...
}

// Emit a deprecation event (safe no-op for non-deprecated names)
DeprecatedRegistry::emit_if_deprecated(&env, "verify_result");
```

---

## Adding a New Entry

1. **Register**: Add a new `match` arm in `DeprecatedRegistry::lookup()` and
   a corresponding `push_back` in `DeprecatedRegistry::all()` inside
   [`deprecated.rs`](../contracts/predictify-hybrid/src/deprecated.rs).

2. **Bump count**: Update `DeprecatedRegistry::ENTRY_COUNT` to match.

3. **Annotate**: Add `#[deprecated(note = "...")]` to the function and call
   `DeprecatedRegistry::emit_if_deprecated(&env, "function_name")` in the
   function body.

4. **Document**: Add a row to the table in this file.

5. **Test**: Add the name to the `names` array in
   `test_all_entries_match_lookup` inside `deprecated.rs`.

---

## Migration Guides

### `verify_result` → `fetch_oracle_result`

**Before:**
```rust
PredictifyHybrid::verify_result(env.clone(), caller, market_id);
```

**After:**
```rust
PredictifyHybrid::fetch_oracle_result(env.clone(), caller, market_id);
```

### `resolve_market` → `resolve_market_manual`

**Before:**
```rust
PredictifyHybrid::resolve_market(env.clone(), market_id);
```

**After:**
```rust
PredictifyHybrid::resolve_market_manual(env.clone(), admin, market_id, outcome);
```

### `VotingManager::collect_fees` → `FeeManager::collect_fees`

**Before:**
```rust
VotingManager::collect_fees(&env, admin, market_id);
```

**After:**
```rust
crate::fees::FeeManager::collect_fees(&env, admin, market_id);
```

---

## Testing

Deprecation registry tests live in `deprecated.rs` (`#[cfg(test)] mod tests`):

```bash
cargo test -p predictify-hybrid -- deprecated
```

Key test cases:

| Test | What it verifies |
|------|-----------------|
| `test_lookup_known_entry` | Correct metadata for a known deprecated function |
| `test_lookup_unknown_returns_none` | Unknown names return `None` |
| `test_lookup_empty_string_returns_none` | Edge case: empty string |
| `test_is_deprecated_true` | Predicate returns `true` for all registered entries |
| `test_is_deprecated_false` | Predicate returns `false` for active functions |
| `test_all_returns_expected_count` | `all()` count matches `ENTRY_COUNT` |
| `test_all_entries_have_nonempty_replacement` | No entry has the same replacement as its name |
| `test_registry_entries_unique_names` | No duplicate names |
| `test_all_entries_match_lookup` | `all()` and `lookup()` agree on every entry |
| `test_emit_if_deprecated_known_does_not_panic` | Event emission for known entries succeeds |
| `test_emit_if_deprecated_unknown_is_noop` | Unknown names are a safe no-op |
