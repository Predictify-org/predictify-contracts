# Lifecycle State Invariants

## Formal Specification

This document specifies the formal invariants that govern market lifecycle state transitions for archive and restore functionality (GitHub issue #1403).

## State Definitions

### Market States
- **Active**: Market accepting votes and bets
- **Ended**: Market deadline passed, resolution pending
- **Disputed**: Market under dispute
- **Resolved**: Market has winning outcome determined
- **Closed**: Market closed (terminal state)
- **Cancelled**: Market cancelled (terminal state)
- **Archived**: Market archived (immutable read-only state)
- **Restored**: Market restored from archive

### Archive Metadata States
- **Not Archived**: No archive record exists
- **Archived**: Archive record exists (timestamp, admin, metadata)
- **Restored**: Archive record exists + restore record exists

## Core Invariants

### Invariant 1: Archive Precondition
```
archive_allowed(market) ⟺ market.state ∈ {Resolved, Cancelled}
```

A market can only be archived if its current state is `Resolved` or `Cancelled`.

**Enforcement**: `EventArchive::archive_event()` returns `CannotArchiveFromState` if violated

**Rationale**: 
- Ensures only final/terminal market outcomes are archived
- Prevents archiving of active or disputed markets (data loss risk)

### Invariant 2: Restore Precondition
```
restore_allowed(market) ⟺ market.state == Archived
```

A market can only be restored if its current state is exactly `Archived`.

**Enforcement**: `RestoreArchive::restore_event()` returns `CannotRestoreFromState` if violated

**Rationale**:
- Ensures restore only applies to archived markets
- Prevents incorrect state transitions

### Invariant 3: Archive Idempotency
```
∀ market ∀ t₁ t₂: (t₁ < t₂) ∧ archive_called(market, t₁) ∧ archive_called(market, t₂)
  ⟹ Error::MarketAlreadyArchived at t₂
```

Calling archive twice on the same market is rejected the second time.

**Enforcement**: `EventArchive::archive_event()` checks `archived.get(market_id).is_some()` before archiving

**Rationale**:
- Prevents accidental duplicate operations
- Ensures deterministic state (same inputs always produce same output)

### Invariant 4: Restore Idempotency
```
∀ market ∀ t₁ t₂: (t₁ < t₂) ∧ restore_called(market, t₁) ∧ restore_called(market, t₂)
  ⟹ Error::MarketAlreadyRestored at t₂
```

Calling restore twice on the same market is rejected the second time.

**Enforcement**: `RestoreArchive::restore_event()` checks `restored_map.get(market_id).is_some()` before restoring

**Rationale**:
- Prevents accidental duplicate restores
- Maintains consistency of restore records

### Invariant 5: State Consistency
```
market.state == Archived ⟺ ∃ archive_record(market_id) ∧ ¬∃ restore_record(market_id)
market.state == Restored ⟺ ∃ restore_record(market_id) ∧ ∃ archive_record(market_id)
market.state ∉ {Archived, Restored} ⟹ ¬∃ archive_record(market_id) ∧ ¬∃ restore_record(market_id)
```

Market state and archive/restore metadata must be synchronized.

**Enforcement**: 
- `EventArchive::validate_archive_consistency()`
- `RestoreArchive::validate_restore_consistency()`
- `LifecycleValidator::validate_market_lifecycle()`

**Rationale**:
- Ensures all archive/restore operations maintain consistency
- Detects and reports state corruption
- Enables recovery from partial failures

### Invariant 6: Archive Capacity
```
|archived_markets| ≤ MAX_ARCHIVE_SIZE (1000)
```

Total number of archived markets never exceeds the capacity limit.

**Enforcement**: 
- `EventArchive::archive_event()` checks `archived.len() >= MAX_ARCHIVE_SIZE`
- Returns `Error::ArchiveFull` when exceeded

**Rationale**:
- Prevents unbounded on-chain storage growth
- Allows admins to manage capacity via pruning

### Invariant 7: Mutual Exclusivity
```
¬(market.state == Archived ∧ market.state == Restored)
```

A market cannot be both archived and restored simultaneously.

**Enforcement**: `LifecycleValidator::validate_market_lifecycle()` ensures mutually exclusive states

**Rationale**:
- Archived → Restored is a state transition, not a bitwise combination
- Once restored, market is no longer in Archived state

### Invariant 8: Authorization
```
archive_allowed(admin) ⟺ admin == stored_admin
restore_allowed(admin) ⟺ admin == stored_admin
```

Only the contract admin can perform archive and restore operations.

**Enforcement**: Both `archive_event()` and `restore_event()` call `admin.require_auth()`

**Rationale**:
- Restricts powerful operations to trusted admin only
- Prevents unauthorized market transitions

### Invariant 9: Deterministic Ordering
```
∀ (t₁, id₁) (t₂, id₂): archive_index sorted by (t₁, id₁) < (t₂, id₂) lexicographically
```

Archive index is maintained in ascending order by (timestamp, market_id) for deterministic pruning.

**Enforcement**: `EventArchive::insert_into_sorted_index()` maintains sorted order

**Rationale**:
- Enables deterministic pruning (oldest first)
- Prevents unpredictable archive behavior

### Invariant 10: Versioning Compatibility
```
∀ restore_entry: restore_entry.version ∈ {1}  (current version)
```

All restore entries must have recognized version numbers.

**Enforcement**: `RestoreArchive::validate_restore_consistency()` checks `entry.version == 1`

**Rationale**:
- Allows safe schema evolution in future versions
- Detects incompatible restore records

## Transition Rules

### Legal Transitions (State Machine Edges)

```
Active     → {Ended, Disputed, Closed, Cancelled}
Ended      → {Disputed, Resolved, Closed}
Disputed   → {Resolved, Closed}
Resolved   → {Archived, Closed}
Cancelled  → {Archived, Closed}
Archived   → {Restored}
Restored   → {Closed}
Closed     → {}  (terminal state)
```

**Enforcement**: `LifecycleValidator::validate_state_transition(from, to)`

**Invalid Transitions** (will return `Error::IllegalMarketStateTransition`):
```
Active     → {Active, Resolved, Archived, Restored, Cancelled*}  (*via Cancelled allowed)
Ended      → {Active, Ended, Archived, Restored, Cancelled}
Disputed   → {Active, Disputed, Ended, Cancelled, Archived, Restored}
Resolved   → {Active, Resolved, Ended, Disputed, Cancelled, Restored}
Cancelled  → {Active, Cancelled, Ended, Disputed, Resolved, Restored}
Archived   → {Active, Archived, Ended, Disputed, Resolved, Cancelled}
Restored   → {Active, Resolved, Archived, Restored}
Closed     → {*}  (no transitions from terminal state)
```

## Property Preservation

### Preservation of Previous Invariants

The lifecycle implementation **does not affect** existing invariants:
- Market creation invariants (question, outcomes, timing)
- Voting invariants (one vote per user, stake validation)
- Betting invariants (sufficient balance, valid outcome)
- Resolution invariants (winning outcome determination, payout calculation)
- Dispute invariants (dispute window, dispute stakes)

Archive and restore are independent overlay features that do not modify:
- Market metadata (question, outcomes, timing)
- User votes and bets
- Payout calculations
- Dispute resolution

### New Safety Properties

1. **Liveness**: Every archived market can eventually be restored or pruned
2. **Consistency**: No partial or corrupted states are possible
3. **Auditability**: All archive/restore operations are recorded via events
4. **Recoverability**: State corruption can be detected and reported

## Corruption Detection

### Detected Corruption Patterns

1. **Missing Archive Metadata**: `market.state == Archived ∧ ¬∃ archive_record`
2. **Missing Restore Metadata**: `market.state == Restored ∧ ¬∃ restore_record`
3. **Orphaned Archive Metadata**: `market.state ≠ Archived ∧ ∃ archive_record`
4. **Orphaned Restore Metadata**: `market.state ≠ Restored ∧ ∃ restore_record`
5. **Capacity Overflow**: `|archived_markets| > MAX_ARCHIVE_SIZE`
6. **Invalid Version**: `restore_entry.version ≠ 1`
7. **Dual Archived-Restored**: `market.state == Archived ∧ market.state == Restored`

### Detection Methods

- **Real-time**: Checked during `archive_event()` and `restore_event()`
- **Validation**: Checked via `validate_market_lifecycle()` (read-only)
- **Query-time**: Checked before operations in `is_archived()` and `is_restored()`

### Error Reporting

All corruption is reported via `Error::InvalidState` (400) with diagnostic details:

```rust
pub fn validate_market_lifecycle(env: &Env, market_id: &Symbol) 
  → Result<LifecycleValidationResult, Error>

pub struct LifecycleValidationResult {
    pub is_valid: bool,
    pub error: Option<Error>,
    pub message: String,
    pub checked_at: u64,
}
```

## Performance Characteristics

### Time Complexity

| Operation | Complexity | Notes |
|-----------|-----------|-------|
| `archive_event()` | O(log n) | n = archive size (sorted insertion) |
| `restore_event()` | O(1) | Direct metadata update |
| `is_archived()` | O(1) | Hash map lookup |
| `is_restored()` | O(1) | Hash map lookup |
| `validate_market_lifecycle()` | O(1) | Constant-time checks |
| `prune_archive()` | O(m) | m = count to prune (capped at 30) |

### Space Complexity

| Storage | Size | Notes |
|---------|------|-------|
| Archive Map | O(n) | n ≤ 1000 entries |
| Restore Map | O(m) | m ≤ n archived markets |
| Index | O(n) | Sorted (timestamp, id) pairs |

## Concurrency Model

### Guarantees (Soroban Storage Model)

1. **Atomicity**: Each transaction is all-or-nothing
2. **Consistency**: No intermediate states visible to other transactions
3. **Isolation**: Concurrent transactions are serialized
4. **Determinism**: Same inputs always produce same outputs

### Race Condition Prevention

All race conditions are prevented by:
- **Idempotency checks**: Duplicate operations rejected
- **Atomic transitions**: State and metadata updated together
- **Deterministic keys**: No key collision issues
- **Soroban storage guarantees**: Implicit serialization

## Testing Strategy

### Unit Tests
- Invariant 1-10 enforcement
- All legal transitions
- All illegal transitions rejected
- Corruption detection

### Integration Tests
- Multi-market archive sequences
- Mixed archive/restore operations
- Archive capacity limits
- Pruning workflows

### Property Tests
- Idempotency verification
- State consistency after each operation
- Deterministic behavior confirmation

### Edge Cases
- Boundary market IDs
- Empty archive/restore maps
- Capacity limits
- Concurrent operation simulation

## References

- [Event Archive Implementation](src/event_archive.rs)
- [Restore Archive Implementation](src/restore_archive.rs)
- [Lifecycle Validation](src/lifecycle_validation.rs)
- [Type Definitions](src/types.rs)
- [Error Codes](src/err.rs)
- [Test Suite](tests/lifecycle.rs)
