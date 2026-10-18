# Migration Guide: Lifecycle-Bound Archive and Restore Transitions

## Overview

This guide documents the implementation of lifecycle-bound archive and restore transitions for GitHub issue #1403. The changes enforce strict state machine rules for market lifecycle transitions, ensuring data consistency and preventing invalid operations.

## Changes Summary

### New States
- **`Archived`**: Market is archived (immutable, read-only state)
- **`Restored`**: Market is restored from archive

### New Error Codes
- **`CannotArchiveFromState (442)`**: Archive only allowed from `Resolved` or `Cancelled`
- **`CannotRestoreFromState (444)`**: Restore only allowed from `Archived`
- **`MarketAlreadyArchived (445)`**: Market is already archived
- **`MarketAlreadyRestored (446)`**: Market is already restored

### New Functions
- **`archive_event(admin, market_id)`**: Transition market from Resolved/Cancelled to Archived
- **`restore_event(admin, market_id, reason)`**: Transition market from Archived to Restored
- **`is_archived(market_id)`**: Check if market is archived
- **`is_restored(market_id)`**: Check if market is restored
- **`validate_archive_consistency(market_id)`**: Validate archive state consistency
- **`validate_restore_consistency(market_id)`**: Validate restore state consistency
- **`validate_market_lifecycle(market_id)`**: Comprehensive lifecycle validation
- **`validate_state_transition(from, to)`**: Validate state transition legality

### New Events
- **`ArchiveTransitionEvent`**: Emitted when market transitions to Archived
- **`RestoreTransitionEvent`**: Emitted when market transitions to Restored

## Compatibility Assessment

### ✅ BACKWARD COMPATIBLE: No Breaking Changes for Existing Callers

**The implementation is fully backward compatible.** Existing contract callers can continue using all existing functions without modifications:

1. **Market Creation**: `create_market()` works unchanged
2. **Voting**: `vote()` works unchanged
3. **Betting**: `place_bet()`, `place_bets()` work unchanged
4. **Claims**: `claim_winnings()` works unchanged
5. **Resolution**: `resolve_market_manual()`, `force_resolve_market()` work unchanged
6. **Queries**: `get_market()`, `query_events_history()` work unchanged

**Why compatible?**
- Archive and restore are new optional features that do not affect existing workflows
- Existing market lifecycle (Active → Ended → Resolved → Closed) continues unchanged
- Archive/restore only apply to markets that explicitly call `archive_event()` and `restore_event()`
- Archived markets are still queryable via existing functions

### State Transition Rules

#### Legal Transitions

```
Active → Ended, Disputed, Closed, Cancelled
Ended → Disputed, Resolved, Closed
Disputed → Resolved, Closed
Resolved → Archived, Closed
Cancelled → Archived, Closed
Archived → Restored
Restored → Closed (or reactivation path if implemented)
Closed → (terminal, no transitions)
```

#### Invalid Transitions (Will Return `IllegalMarketStateTransition`)

- **Resolved → Active** (cannot reopen a resolved market)
- **Closed → Ended** (terminal state, no transitions allowed)
- **Active → Active** (self-loops are not valid transitions)
- **Any → Archived** except from Resolved or Cancelled
- **Any → Restored** except from Archived
- **Any → Closed** except from terminal or non-terminal states

### Authorization

Both archive and restore operations are **admin-only**:

```rust
// Only the stored admin address can archive/restore
archive_event(&admin, &market_id) → requires admin.require_auth()
restore_event(&admin, &market_id, &reason) → requires admin.require_auth()
```

Non-admin callers will receive `Error::Unauthorized` (100).

## Migration Path for New Features

### If You Want to Use Archive/Restore

#### Step 1: After Market Resolution
Once a market is `Resolved` or `Cancelled`, you can archive it:

```rust
// After market is resolved
resolve_market_manual(&admin, &market_id, &winning_outcome);

// Now archive it
archive_event(&admin, &market_id);
```

#### Step 2: Query Archive Status
```rust
// Check if archived
if is_archived(&market_id) {
    println!("Market is archived");
}

// Get archive details
if let Some(entry) = get_archive_entry(&market_id) {
    println!("Archived at: {}", entry.archived_at);
}
```

#### Step 3: Optional Restore (if needed for corrections)
```rust
// If correction needed, restore from archive
restore_event(&admin, &market_id, &String::from_str(&env, "Dispute resolution"))?;
```

### Archived Market Behavior

Archived markets are:
- **Queryable**: `get_market()` still returns the market
- **Immutable**: No voting, betting, or resolution changes allowed
- **Retention**: Kept in archive until manually pruned via `prune_archive()`
- **Metadata**: Archive timestamp and admin details are recorded

### Pruning Archived Markets

When archive capacity is reached (1,000 entries), prune oldest entries:

```rust
// Prune oldest 10 archived markets
let (pruned_count, next_cursor) = prune_archive(&admin, 10, None)?;

// Resume pruning with cursor
loop {
    let (count, cursor) = prune_archive(&admin, 30, next_cursor)?;
    if cursor.done {
        break; // No more entries to prune
    }
    next_cursor = Some(cursor);
}
```

## Testing Your Integration

### Required Tests

1. **Test Archive Flow**
   ```rust
   #[test]
   fn test_archive_resolved_market() {
       // Create and resolve market
       // Archive it
       // Verify is_archived() returns true
   }
   ```

2. **Test Restore Flow**
   ```rust
   #[test]
   fn test_restore_archived_market() {
       // Create, resolve, archive market
       // Restore it
       // Verify is_restored() returns true
   }
   ```

3. **Test Authorization**
   ```rust
   #[test]
   fn test_archive_requires_admin() {
       // Verify non-admin gets Unauthorized error
   }
   ```

4. **Test Invalid Transitions**
   ```rust
   #[test]
   fn test_archive_fails_from_active() {
       // Verify CannotArchiveFromState error for Active market
   }
   ```

### Existing Test Compatibility

All existing tests should continue to pass without modification. The implementation does not change:
- Market creation flow
- Voting mechanics
- Betting mechanics
- Resolution logic
- Claim logic

## API Reference

### Archive Operations

#### `archive_event(admin: Address, market_id: Symbol) → Result<(), Error>`

Archives a market. Market must be in `Resolved` or `Cancelled` state.

**Errors:**
- `Unauthorized`: Caller is not admin
- `MarketNotFound`: Market does not exist
- `CannotArchiveFromState`: Market state is not Resolved or Cancelled
- `MarketAlreadyArchived`: Market already archived
- `ArchiveFull`: Archive capacity (1,000) reached

**Events Emitted:**
- `ArchiveTransitionEvent` (topic `arch_trn`)

### Restore Operations

#### `restore_event(admin: Address, market_id: Symbol, reason: String) → Result<(), Error>`

Restores a market from archive. Market must be in `Archived` state.

**Errors:**
- `Unauthorized`: Caller is not admin
- `MarketNotFound`: Market does not exist
- `CannotRestoreFromState`: Market state is not Archived
- `MarketAlreadyRestored`: Market already restored

**Events Emitted:**
- `RestoreTransitionEvent` (topic `rest_trn`)

### Query Operations

#### `is_archived(market_id: Symbol) → bool`

Returns `true` if market is in `Archived` state with valid archive metadata.

#### `is_restored(market_id: Symbol) → bool`

Returns `true` if market is in `Restored` state with valid restore metadata.

### Validation Operations

#### `validate_market_lifecycle(market_id: Symbol) → Result<LifecycleValidationResult, Error>`

Comprehensive consistency validation for a market's lifecycle state.

**Returns:**
- `is_valid`: Boolean indicating validation success
- `error`: Error code if validation failed
- `message`: Diagnostic message
- `checked_at`: Validation timestamp

#### `validate_state_transition(from: MarketState, to: MarketState) → Result<(), Error>`

Validates if a state transition is legal.

**Returns:**
- `Ok(())`: Transition is legal
- `Err(IllegalMarketStateTransition)`: Transition is illegal

## Observability

### Events

All archive and restore operations emit events for audit trails:

```rust
// Archive event
pub struct ArchiveTransitionEvent {
    pub market_id: Symbol,
    pub admin: Address,
    pub from_state: String,
    pub archived_at: u64,
    pub nonce: u64,              // Replay protection
    pub timestamp: u64,
}

// Restore event
pub struct RestoreTransitionEvent {
    pub market_id: Symbol,
    pub admin: Address,
    pub reason: String,
    pub restored_at: u64,
    pub nonce: u64,              // Replay protection
    pub timestamp: u64,
}
```

### Event Topics

- **Archive**: `arch_trn` (with market_id as second topic)
- **Restore**: `rest_trn` (with market_id as second topic)

Use these topics to filter events in your indexer:

```rust
// Listen for archive events
filter.topic(0) == "arch_trn"

// Listen for restore events
filter.topic(0) == "rest_trn"
```

## Concurrency Safety

### Guaranteed Properties

1. **Atomic Transitions**: Archive and restore are atomic operations
2. **Idempotency**: Duplicate requests are rejected deterministically
3. **State Consistency**: Archive/restore metadata is always synchronized with market state
4. **Deterministic Pruning**: Archive pruning is deterministic and resumable

### Thread Safety

Soroban's storage model guarantees:
- No partial updates (transactions are atomic)
- Consistent state across concurrent calls
- Deterministic key derivation prevents collisions

## Known Limitations

1. **Archive is One-Way by Default**: Markets can be archived from Resolved/Cancelled, but once archived, they can only be restored or pruned (no automatic cleanup)

2. **Archive Capacity**: Maximum 1,000 archived entries. Older entries must be pruned to make room for new ones.

3. **Restore is Optional**: Restore functionality is included but not required for basic archive/prune workflows.

4. **No Auto-Expiry**: Archived entries do not automatically expire. Admins must explicitly prune old entries.

## Troubleshooting

### Issue: "CannotArchiveFromState" Error

**Cause**: Trying to archive a market that is not in Resolved or Cancelled state

**Solution**: 
1. Verify market state with `get_market(&market_id)`
2. Only archive after market is resolved: `resolve_market_manual()` → `archive_event()`

### Issue: "MarketAlreadyArchived" Error

**Cause**: Attempting to archive a market that is already archived

**Solution**: This is expected behavior and indicates idempotency protection is working. Check if market is already archived with `is_archived()`.

### Issue: "ArchiveFull" Error

**Cause**: Archive has reached capacity (1,000 entries)

**Solution**: Call `prune_archive()` to remove oldest entries before archiving new ones.

### Issue: "MarketAlreadyRestored" Error

**Cause**: Attempting to restore a market that is already restored

**Solution**: This is expected behavior. Check with `is_restored()` before restoring.

## Rollback Plan

If issues arise with archive/restore functionality:

1. **Disable Archive/Restore**: Do not call `archive_event()` or `restore_event()` on new markets
2. **Existing Archived Markets**: Can still be queried and pruned
3. **No Data Loss**: Archive/restore do not modify market resolution or payouts

Archive/restore are completely independent from the core market lifecycle and can be disabled without affecting existing operations.

## Questions?

Refer to:
- `LIFECYCLE_VALIDATION.md` - State validation rules and consistency checks
- `tests/lifecycle.rs` - Complete test suite with examples
- `src/lifecycle_validation.rs` - Validation implementation
- `src/event_archive.rs` - Archive implementation
- `src/restore_archive.rs` - Restore implementation

## Support

For issues or questions, create a GitHub issue with:
1. Error code received
2. Market ID and state
3. Steps to reproduce
4. Contract version
