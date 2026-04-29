# Storage Layout and Key Collision Review

## Overview

This document provides a comprehensive audit of all persistent storage keys used in the Predictify Hybrid Soroban smart contract. It enumerates all storage keys, documents their purposes, validates that no symbol collisions exist across modules, and provides constraints for safely adding new fields to core data structures.

**Last Updated**: 2026-04-27  
**Contract Version**: Predictify Hybrid v1.0  
**Audit Status**: ✅ Complete

## Table of Contents

1. [Storage Key Enumeration](#storage-key-enumeration)
2. [Key Collision Analysis](#key-collision-analysis)
3. [Storage Key Namespacing Strategy](#storage-key-namespacing-strategy)
4. [Data Structure Constraints](#data-structure-constraints)
5. [Migration Safety Guidelines](#migration-safety-guidelines)
6. [Adding New Storage Keys](#adding-new-storage-keys)

---

## Storage Key Enumeration

### 1. Core Contract Storage Keys

#### 1.1 Admin and Authorization

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `"Admin"` | `Symbol` | `admin.rs` | Primary admin address | ⚠️ HIGH - Core key |
| `"admin_role"` | `Symbol` | `admin.rs` | Admin role assignment | LOW |
| `"AdminCount"` | `Symbol` | `admin.rs` | Count of active admins | LOW |
| `"AdminList"` | `Symbol` | `admin.rs` | List of admin addresses | LOW |
| `"MultisigConfig"` | `Symbol` | `admin.rs` | Multisig configuration | LOW |
| `"NextActionId"` | `Symbol` | `admin.rs` | Counter for pending actions | LOW |
| `"ContractPaused"` | `Symbol` | `admin.rs` | Contract pause state | ⚠️ MEDIUM |

**Storage Pattern**: Simple `Symbol` keys for singleton values.

**Key Format**: `env.storage().persistent().set(&Symbol::new(env, "Admin"), &admin)`

#### 1.2 Market Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `market_id` | `Symbol` | `markets.rs` | Individual market data | LOW - Unique per market |
| `"MarketCounter"` | `Symbol` | `markets.rs` | Market ID generation counter | LOW |
| `pause_info_key` | `Symbol` | `markets.rs` | Market pause information | LOW |

**Storage Pattern**: Market ID as direct key for market data.

**Key Format**: `env.storage().persistent().set(&market_id, &market)`

**Collision Prevention**: Market IDs are generated with unique counters and include timestamp-based uniqueness.

#### 1.3 Event Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `("Event", event_id)` | `(Symbol, Symbol)` | `storage.rs` | Event data | LOW - Tuple namespace |

**Storage Pattern**: Tuple key with namespace prefix.

**Key Format**: `(Symbol::new(env, "Event"), event_id.clone())`

**Collision Prevention**: Tuple keys provide namespace isolation.

#### 1.4 Balance Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `[Symbol("Balance"), Address, ReflectorAsset]` | `Vec<Val>` | `storage.rs` | User asset balances | LOW - Composite key |

**Storage Pattern**: Vector-based composite key with three components.

**Key Format**:
```rust
let mut key = Vec::new(env);
key.push_back(Symbol::new(env, "Balance").into_val(env));
key.push_back(user.to_val());
key.push_back(asset.into_val(env));
```

**Collision Prevention**: Three-part composite key ensures uniqueness per user-asset pair.

#### 1.5 Audit Trail Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `"AUDIT_HEAD"` | `Symbol` | `audit_trail.rs` | Audit trail head pointer | LOW |
| `("AUDIT_REC", index)` | `(Symbol, u64)` | `audit_trail.rs` | Individual audit records | LOW - Tuple with index |

**Storage Pattern**: Tuple key with namespace and sequential index.

**Key Format**: `(Symbol::new(env, "AUDIT_REC"), index)`

**Collision Prevention**: Sequential indexing with namespace prefix.

#### 1.6 Circuit Breaker Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `"CB_CONFIG"` | `Symbol` (const) | `circuit_breaker.rs` | Circuit breaker configuration | LOW |
| `"CB_STATE"` | `Symbol` (const) | `circuit_breaker.rs` | Circuit breaker state | LOW |
| `"CB_EVENTS"` | `Symbol` (const) | `circuit_breaker.rs` | Circuit breaker events | LOW |
| `"CB_CONDITIONS"` | `Symbol` (const) | `circuit_breaker.rs` | Circuit breaker conditions | LOW |

**Storage Pattern**: Instance storage with constant symbol keys.

**Key Format**: `env.storage().instance().set(&Symbol::new(env, Self::CONFIG_KEY), &config)`

**Collision Prevention**: Prefixed with "CB_" namespace.

#### 1.7 Configuration Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `"storage_config"` | `Symbol` | `storage.rs` | Storage optimization config | LOW |
| `"Config"` | `Symbol` | `config.rs` | Contract configuration | ⚠️ MEDIUM |

**Storage Pattern**: Simple symbol keys for singleton configuration.

**Collision Prevention**: Descriptive names reduce collision risk.

#### 1.8 Recovery Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `"RecoveryRecords"` | `Symbol` | `recovery.rs` | Recovery record map | LOW |
| `"RecoveryStatus"` | `Symbol` | `recovery.rs` | Recovery status map | LOW |

**Storage Pattern**: Symbol keys for recovery data structures.

#### 1.9 Extension Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `"ext_event"` | `symbol_short!` | `extensions.rs` | Extension events | LOW |

**Storage Pattern**: Short symbol for extension data.

#### 1.10 Reentrancy Guard Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `"reent_lk"` | `symbol_short!` | `reentrancy_guard.rs` | Reentrancy lock flag | LOW |

**Storage Pattern**: Short symbol for boolean flag.

**Key Format**: `env.storage().persistent().set(&Self::key(), &true)`

#### 1.11 Creator Limits Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `("ActiveEvents", creator)` | `(Symbol, Address)` | `storage.rs` | Active event count per creator | LOW - Tuple key |

**Storage Pattern**: Tuple key with creator address.

**Key Format**: `(Symbol::new(env, "ActiveEvents"), creator.clone())`

#### 1.12 Compressed Market Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `"compressed_{market_id}"` | `Symbol` (formatted) | `storage.rs` | Compressed market data | LOW - Prefixed |
| `"compressed_ref_{market_id}"` | `Symbol` (formatted) | `storage.rs` | Compressed market reference | LOW - Prefixed |

**Storage Pattern**: Formatted symbol with market ID.

**Key Format**: `Symbol::new(env, &format!("compressed_{:?}", market_id))`

#### 1.13 Migration Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `migration_id` | `Symbol` | `storage.rs` | Migration records | LOW - Unique ID |

**Storage Pattern**: Migration ID as direct key.

#### 1.14 Archive Storage

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `"archive_{market_id}_{timestamp}"` | `Symbol` (formatted) | `storage.rs` | Archived market data | LOW - Timestamped |

**Storage Pattern**: Formatted symbol with market ID and timestamp.

#### 1.15 Pending Admin Actions

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `"PendingAction_{action_id}"` | `Symbol` (formatted) | `admin.rs` | Pending multisig actions | LOW - Unique ID |

**Storage Pattern**: Formatted symbol with action ID.

**Key Format**: `Symbol::new(env, &format!("PendingAction_{}", action_id))`

#### 1.16 Token Storage (Test/Custom)

| Key | Type | Module | Purpose | Collision Risk |
|-----|------|--------|---------|----------------|
| `"TokenID"` | `Symbol` | Various test files | Test token addresses | LOW - Test only |

**Storage Pattern**: Test-only storage key.

---

## Key Collision Analysis

### Collision Risk Assessment

#### ✅ NO COLLISIONS DETECTED

After comprehensive analysis of all storage keys across all modules, **no symbol collisions were found**. The codebase employs several effective collision prevention strategies:

1. **Namespace Prefixing**: Keys use descriptive prefixes (e.g., "CB_", "AUDIT_", "compressed_")
2. **Tuple Keys**: Multi-component keys provide natural namespacing
3. **Composite Keys**: Vector-based keys with multiple components
4. **Unique Identifiers**: Market IDs, event IDs, and action IDs are unique
5. **Formatted Keys**: Dynamic keys include unique identifiers in format strings

### High-Risk Keys (Require Extra Caution)

These keys are critical and should never be modified without careful migration:

1. **`"Admin"`** - Primary admin address (core authorization)
2. **`"ContractPaused"`** - Contract pause state (safety mechanism)
3. **`"Config"`** - Contract configuration (system-wide settings)
4. **Market IDs** - Direct keys for market data (economic data)

### Collision Prevention Mechanisms

#### 1. Tuple Key Pattern
```rust
// Good: Namespace isolation
let key = (Symbol::new(env, "Event"), event_id);
env.storage().persistent().set(&key, &event);
```

#### 2. Composite Vector Key Pattern
```rust
// Good: Multi-component uniqueness
let mut key = Vec::new(env);
key.push_back(Symbol::new(env, "Balance").into_val(env));
key.push_back(user.to_val());
key.push_back(asset.into_val(env));
```

#### 3. Formatted Key Pattern
```rust
// Good: Dynamic unique keys
let key = Symbol::new(env, &format!("compressed_{:?}", market_id));
```

#### 4. Constant Key Pattern
```rust
// Good: Compile-time constants
const CONFIG_KEY: &str = "CB_CONFIG";
```

---

## Storage Key Namespacing Strategy

### Current Namespacing Conventions

1. **Admin Keys**: No prefix, descriptive names (e.g., "Admin", "AdminCount")
2. **Circuit Breaker**: "CB_" prefix (e.g., "CB_CONFIG", "CB_STATE")
3. **Audit Trail**: "AUDIT_" prefix (e.g., "AUDIT_HEAD", "AUDIT_REC")
4. **Compressed Data**: "compressed_" prefix
5. **Archive Data**: "archive_" prefix
6. **Recovery**: "Recovery" prefix (e.g., "RecoveryRecords")

### Recommended Namespace Additions

For future development, consider these namespace prefixes:

- **Governance**: "GOV_" prefix
- **Oracle**: "ORACLE_" prefix
- **Statistics**: "STATS_" prefix
- **Cache**: "CACHE_" prefix
- **Temporary**: "TEMP_" prefix

---

## Data Structure Constraints

### Market Structure

**File**: `contracts/predictify-hybrid/src/types.rs`

**Current Fields** (as of audit):
```rust
pub struct Market {
    pub admin: Address,
    pub question: String,
    pub outcomes: Vec<String>,
    pub end_time: u64,
    pub oracle_config: OracleConfig,
    pub has_fallback: bool,
    pub fallback_oracle_config: OracleConfig,
    pub resolution_timeout: u64,
    pub oracle_result: Option<String>,
    pub votes: Map<Address, String>,
    pub stakes: Map<Address, i128>,
    pub claimed: Map<Address, ClaimInfo>,
    pub total_staked: i128,
    pub dispute_stakes: Map<Address, i128>,
    pub winning_outcomes: Option<Vec<String>>,
    pub fee_collected: bool,
    pub state: MarketState,
    pub total_extension_days: u32,
    pub max_extension_days: u32,
    pub extension_history: Vec<MarketExtension>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub min_pool_size: Option<i128>,
    pub bet_deadline: u64,
    pub dispute_window_seconds: u64,
}
```

#### ✅ Safe to Add (Append-Only)

New fields can be safely added to the **end** of the struct:

```rust
// ✅ SAFE: Appending new fields
pub struct Market {
    // ... existing fields ...
    pub dispute_window_seconds: u64,
    
    // NEW FIELDS (safe to add here)
    pub creation_timestamp: u64,
    pub last_updated: u64,
    pub metadata_hash: Option<String>,
}
```

#### ❌ UNSAFE Operations

1. **Reordering fields** - Breaks serialization compatibility
2. **Removing fields** - Causes deserialization failures
3. **Changing field types** - Incompatible with existing data
4. **Inserting fields in middle** - Breaks field order

#### Migration Required For

1. **Field type changes** - Requires data migration
2. **Field removal** - Requires migration to remove data
3. **Struct reorganization** - Requires full migration

### Event Structure

**File**: `contracts/predictify-hybrid/src/types.rs`

**Current Fields**:
```rust
pub struct Event {
    pub id: Symbol,
    pub description: String,
    pub outcomes: Vec<String>,
    pub end_time: u64,
    pub oracle_config: OracleConfig,
    pub has_fallback: bool,
    pub fallback_oracle_config: OracleConfig,
    pub resolution_timeout: u64,
    pub admin: Address,
    pub created_at: u64,
    pub status: MarketState,
    pub visibility: EventVisibility,
    pub allowlist: Vec<Address>,
}
```

#### ✅ Safe to Add (Append-Only)

```rust
// ✅ SAFE: Appending new fields
pub struct Event {
    // ... existing fields ...
    pub allowlist: Vec<Address>,
    
    // NEW FIELDS (safe to add here)
    pub participant_count: u32,
    pub total_volume: i128,
}
```

### ClaimInfo Structure

**Current Fields**:
```rust
pub struct ClaimInfo {
    pub claimed: bool,
    pub timestamp: u64,
    pub payout_amount: i128,
}
```

#### ✅ Safe to Add

```rust
// ✅ SAFE: Appending new fields
pub struct ClaimInfo {
    pub claimed: bool,
    pub timestamp: u64,
    pub payout_amount: i128,
    
    // NEW FIELDS (safe to add here)
    pub claim_transaction_id: Option<String>,
}
```

### OracleConfig Structure

**Current Fields**:
```rust
pub struct OracleConfig {
    pub provider: OracleProvider,
    pub oracle_address: Address,
    pub feed_id: String,
    pub threshold: i128,
    pub comparison: String,
}
```

#### ⚠️ CAUTION: High-Risk Structure

This structure is embedded in Market and Event. Changes require careful migration.

#### ✅ Safe to Add

```rust
// ✅ SAFE: Appending new fields
pub struct OracleConfig {
    pub provider: OracleProvider,
    pub oracle_address: Address,
    pub feed_id: String,
    pub threshold: i128,
    pub comparison: String,
    
    // NEW FIELDS (safe to add here)
    pub staleness_threshold: Option<u64>,
    pub confidence_threshold: Option<u32>,
}
```

---

## Migration Safety Guidelines

### Pre-Migration Checklist

Before modifying any storage structure:

- [ ] Identify all storage keys affected
- [ ] Document current data format
- [ ] Create migration plan
- [ ] Write migration tests
- [ ] Test on testnet
- [ ] Create rollback plan
- [ ] Document migration in CHANGELOG

### Migration Patterns

#### Pattern 1: Append-Only (No Migration Needed)

```rust
// OLD
pub struct Market {
    pub admin: Address,
    pub question: String,
}

// NEW (✅ Safe)
pub struct Market {
    pub admin: Address,
    pub question: String,
    pub created_at: u64,  // New field with default
}
```

**Safety**: Soroban's serialization handles missing fields with defaults.

#### Pattern 2: Field Type Change (Migration Required)

```rust
// OLD
pub struct Market {
    pub end_time: u64,
}

// NEW (❌ Requires migration)
pub struct Market {
    pub end_time: i128,  // Changed type
}
```

**Migration Steps**:
1. Create `MarketV2` struct with new type
2. Implement migration function
3. Migrate all existing markets
4. Update all references
5. Remove old struct

#### Pattern 3: Field Removal (Migration Required)

```rust
// OLD
pub struct Market {
    pub admin: Address,
    pub deprecated_field: String,
}

// NEW (❌ Requires migration)
pub struct Market {
    pub admin: Address,
    // deprecated_field removed
}
```

**Migration Steps**:
1. Mark field as deprecated
2. Stop writing to field
3. Create migration to remove field
4. Test thoroughly
5. Deploy migration

### Storage Format Versioning

Implement version tracking for complex migrations:

```rust
#[contracttype]
pub enum StorageVersion {
    V1,
    V2,
    V3,
}

pub struct VersionedMarket {
    pub version: StorageVersion,
    pub data: Market,
}
```

---

## Adding New Storage Keys

### Guidelines for New Keys

1. **Use Descriptive Names**: Keys should clearly indicate their purpose
2. **Apply Namespace Prefixes**: Use consistent prefixes for related keys
3. **Avoid Generic Names**: Don't use "data", "info", "temp" alone
4. **Document Immediately**: Add to this document when creating new keys
5. **Check for Collisions**: Search codebase for existing usage
6. **Use Appropriate Key Type**: Choose between Symbol, tuple, or composite keys

### Key Type Selection Guide

| Use Case | Recommended Key Type | Example |
|----------|---------------------|---------|
| Singleton config | `Symbol` | `Symbol::new(env, "Config")` |
| Per-user data | `(Symbol, Address)` | `(Symbol::new(env, "Balance"), user)` |
| Per-market data | `Symbol` (market_id) | `market_id` |
| Namespaced data | `(Symbol, Symbol)` | `(Symbol::new(env, "Event"), event_id)` |
| Multi-key data | `Vec<Val>` | `[Symbol, Address, Asset]` |
| Temporary data | `symbol_short!` | `symbol_short!("tmp")` |

### Example: Adding a New Storage Key

```rust
// ✅ GOOD: Descriptive, namespaced, documented
pub struct UserReputationManager;

impl UserReputationManager {
    /// Storage key for user reputation scores
    /// Format: ("REPUTATION", user_address)
    fn reputation_key(env: &Env, user: &Address) -> (Symbol, Address) {
        (Symbol::new(env, "REPUTATION"), user.clone())
    }
    
    pub fn get_reputation(env: &Env, user: &Address) -> u32 {
        let key = Self::reputation_key(env, user);
        env.storage().persistent().get(&key).unwrap_or(0)
    }
    
    pub fn set_reputation(env: &Env, user: &Address, score: u32) {
        let key = Self::reputation_key(env, user);
        env.storage().persistent().set(&key, &score);
    }
}
```

### Anti-Patterns to Avoid

```rust
// ❌ BAD: Generic name, no namespace
let key = Symbol::new(env, "data");

// ❌ BAD: Collision risk with existing keys
let key = Symbol::new(env, "Admin");  // Already used!

// ❌ BAD: No documentation
fn get_key(env: &Env) -> Symbol {
    Symbol::new(env, "xyz")  // What is this?
}

// ❌ BAD: Inconsistent naming
let key1 = Symbol::new(env, "user_balance");
let key2 = Symbol::new(env, "UserStake");  // Inconsistent case
```

---

## Storage Key Registry

### Quick Reference Table

| Key Pattern | Module | Count | Risk Level |
|-------------|--------|-------|------------|
| Simple Symbol | Multiple | ~20 | MEDIUM |
| Tuple (Symbol, Symbol) | storage.rs, audit_trail.rs | ~3 | LOW |
| Tuple (Symbol, Address) | storage.rs, admin.rs | ~2 | LOW |
| Tuple (Symbol, u64) | audit_trail.rs | ~1 | LOW |
| Vec<Val> composite | storage.rs | ~1 | LOW |
| Formatted Symbol | storage.rs, admin.rs | ~5 | LOW |
| Market ID direct | markets.rs | ~N | LOW |

### Total Storage Keys: ~35 unique patterns

---

## Audit Trail

| Date | Auditor | Changes | Status |
|------|---------|---------|--------|
| 2026-04-27 | Storage Audit Team | Initial comprehensive audit | ✅ Complete |
| TBD | - | Next review | Pending |

---

## Recommendations

### Immediate Actions

1. ✅ **No collisions found** - Current implementation is safe
2. ✅ **Namespace strategy is effective** - Continue current patterns
3. ✅ **Key documentation is now complete** - Maintain this document

### Future Improvements

1. **Implement Storage Version Tracking**: Add version field to major structures
2. **Create Migration Framework**: Build reusable migration utilities
3. **Add Storage Metrics**: Track storage usage and costs
4. **Automated Collision Detection**: Add CI checks for new keys
5. **Storage Key Constants**: Move all keys to centralized constants file

### Monitoring

1. **Track Storage Growth**: Monitor total storage usage
2. **Key Usage Analytics**: Track which keys are accessed most
3. **Migration History**: Maintain log of all storage migrations
4. **Collision Alerts**: Alert on potential new collisions

---

## Conclusion

The Predictify Hybrid contract storage layout has been comprehensively audited. **No storage key collisions were detected**. The current namespacing and key structure strategies are effective and should be maintained. This document provides clear guidelines for safely extending data structures and adding new storage keys.

**Audit Status**: ✅ **PASSED** - No issues found

**Next Review**: Recommended after any major feature additions or before mainnet deployment.

---

## References

- [Soroban Storage Documentation](https://soroban.stellar.org/docs/learn/storage)
- [Contract Source Code](../../contracts/predictify-hybrid/src/)
- [Types System Documentation](./TYPES_SYSTEM.md)
- [Migration Best Practices](https://soroban.stellar.org/docs/learn/storage#storage-migrations)

---

**Document Version**: 1.0  
**Last Updated**: 2026-04-27  
**Maintained By**: Predictify Development Team
