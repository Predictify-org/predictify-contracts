# Issue #1398: Prevent Duplicate Event-Creation Fees

## Executive Summary

This issue requires implementing **idempotency for event creation** to prevent charging duplicate event-creation fees on retry scenarios. When an admin creates an event and encounters a network timeout, a retry should not result in a second fee charge.

## Current State Analysis

### Event Creation Flow
- **Entry point**: `create_event` in `contracts/predictify-hybrid/src/lib.rs:773-873`
- **Current operations** (in order):
  1. Circuit breaker check
  2. Auth check (primary admin)
  3. Rate limiting
  4. Input validation (outcomes count, description, oracle config)
  5. Event ID generation (via MarketIdGenerator)
  6. Event storage
  7. Event emission
  8. Statistics recording
  9. Audit trail recording
  10. Gas tracking

### Fee Collection Issue
- **Current**: Fee collection is NOT implemented in `create_event`, despite tests expecting it
- **Reference implementation**: `process_creation_fee` exists in `fees.rs:779` and is used in `create_market` (markets.rs:132)
- **Test expectations**: Tests in `test.rs` expect fees to be charged:
  - `test_create_event_collects_configured_fee_and_emits_event` (line 2418)
  - `test_create_event_rejects_when_fee_insufficient` (line 2491)
  - `test_create_event_rejects_when_fee_asset_not_configured` (line 2532)
  - `test_create_event_uses_configured_fee_asset` (line 2568)

### Existing Idempotency Pattern: place_bets
Located in `place_bets_idempotency_tests.rs` and `bets.rs`:

**Storage Key Design**:
```rust
DataKey::PlaceBetsIdem(Address, BytesN<32>)
```

**Characteristics**:
- **Scope**: User-specific (keyed by caller Address + BytesN<32>)
- **Key Derivation**: Caller-supplied 32-byte key (required parameter)
- **TTL**: `PLACE_BETS_IDEM_TTL_LEDGERS` (~7 days at 5s/ledger = ~525k ledgers)
- **Behavior on Retry**: Returns `IdempotentBatchAlreadyApplied` error
- **Entry Point**: `place_bets` function in `bets.rs`

**Key Insights**:
- Keys are caller-supplied → requires client coordination
- Keys are address-scoped → same raw bytes can be reused by different users
- TTL-based expiry → keys are automatically cleaned up after TTL window
- Deterministic error on replay → clients receive clear signal

### Storage Key Infrastructure
Located in `storage.rs`:

**DataKey Enum**:
```rust
pub enum DataKey {
    PlaceBetsIdem(Address, soroban_sdk::BytesN<32>),
    EventNonce(Symbol),
    AdminOverrideNonce,
    // ... many others
}
```

**TTL Constants**:
```rust
pub const PLACE_BETS_IDEM_TTL_LEDGERS: u32 = 7 * LEDGERS_PER_DAY;  // ~7 days
pub const EVENT_TTL_LEDGERS: u32 = 90 * LEDGERS_PER_DAY;           // ~90 days
```

## Design Decision: Caller-Supplied Key (Recommended)

Following the pattern of `place_bets`, we will implement caller-supplied idempotency keys for `create_event`:

### Rationale
1. **Consistency**: Reuses proven pattern from `place_bets`
2. **Flexibility**: Allows clients to coordinate retries intelligently
3. **Safety**: Caller explicitly manages retry semantics
4. **Scope**: Admin-scoped keys prevent collision across different admins
5. **Minimally Invasive**: Adds optional parameter to existing function

### Key Characteristics
- **Storage Key**: `DataKey::CreateEventIdem(Address, BytesN<32>)`
- **Key Scope**: Admin-specific (keyed by admin Address + caller-supplied 32-byte key)
- **TTL**: `CREATE_EVENT_IDEM_TTL_LEDGERS` = `PLACE_BETS_IDEM_TTL_LEDGERS` (~7 days)
- **Behavior on Duplicate**: Return `IdempotentBatchAlreadyApplied` error before fee collection
- **Return on Success**: Return the cached event_id if already processed
- **Parameter**: Add optional `idempotency_key: Option<BytesN<32>>` to `create_event`

## Implementation Plan

### Phase 1: Storage Infrastructure
1. Add `CreateEventIdem(Address, BytesN<32>)` variant to `DataKey` enum in `storage.rs`
2. Add `CREATE_EVENT_IDEM_TTL_LEDGERS` constant (reuse `PLACE_BETS_IDEM_TTL_LEDGERS`)

### Phase 2: Core Implementation
1. Modify `create_event` function signature to accept optional idempotency key
2. Add idempotency check BEFORE any state mutation (fee collection, event storage, etc.)
3. On cache hit: return cached event_id with error if key already consumed
4. On cache miss: proceed with normal flow
5. On success: store idempotency key with event_id value and TTL

### Phase 3: Fee Integration
1. Call `process_creation_fee` AFTER idempotency check succeeds
2. Ensures fee is charged atomically with event creation
3. On duplicate: fees are NOT collected (idempotency check fires first)

### Phase 4: Testing
1. Happy path: Fresh key succeeds
2. Duplicate key: Returns `IdempotentBatchAlreadyApplied` without charging fee
3. Different key: Succeeds independently (same admin)
4. Key scope: Same key bytes, different admin → independent calls
5. TTL expiry: After TTL, key reusable
6. Integration: Fees charged exactly once per unique event

## Acceptance Criteria Mapping

| Criterion | Implementation | Verification |
|-----------|----------------|--------------|
| Deterministic behavior (valid/invalid/duplicate) | Idempotency key check + error codes | Tests for each scenario |
| Authorization/validation enforcement | Idempotency check AFTER auth/validation gates | Existing checks + integration tests |
| Retry/partial failure safety | Check BEFORE state mutation + atomic TTL store | Retry tests + fee verification |
| Concurrent execution safety | Soroban storage atomicity + key scoping | Concurrency tests if needed |
| Test coverage (success/rejection/boundary/regression) | Comprehensive test suite below | Test matrix |
| Backward compatibility | Optional parameter with sensible default | Works without key parameter |
| Observability | Clear error codes + audit trail entries | Error messages + audit logs |

## Error Handling

### New Error Code
Consider adding: `IdempotentEventAlreadyCreated` or reuse `IdempotentBatchAlreadyApplied`

**Rationale for Reuse**:
- Both represent "idempotent operation already applied"
- Reduces error enum bloat
- Consistent with place_bets pattern
- Clients can handle identically

### Error Sequence
1. Circuit breaker error → panic
2. Auth error → panic
3. Rate limit error → panic
4. Validation error (outcomes, description, oracle) → panic
5. **Idempotency check** → panic with error (NEW)
6. Fee collection error → panic
7. Storage error → panic

## State Invariants

### Before Implementation
- Event creation can be called multiple times with identical inputs → multiple fees charged

### After Implementation
- If idempotency key is provided:
  - First call with key K → event created, fee charged, key stored with event_id value
  - Second call with key K (within TTL) → `IdempotentBatchAlreadyApplied` error, no fee
  - After TTL expiry → key can be reused (new event creation)
  
- If no idempotency key provided:
  - Backward compatible: call proceeds (optional parameter)
  - Risk of duplicate fees remains for this call
  - Future migration can make key mandatory

## Failure Mode Analysis

### Partial Failure Scenarios
1. **Key stored, then fee fails**: Idempotency check fires on retry, no fee charged ✓
2. **Fee collected, then key storage fails**: Retry sees fee already charged (inconsistent) ✗
   - **Mitigation**: Store key FIRST (before fee), or use transaction-like semantics
   - **Soroban Consideration**: No transactions; use storage order carefully

3. **Network timeout during event storage**: Admin can retry safely with same key ✓

### Mitigation Strategy
- **Order**: Check idempotency → charge fee → store event → store idempotency key
- **Rationale**: If fee fails, admin can retry and will pay again (acceptable)
- **Alternative**: Store idempotency key first → charge fee → check didn't lose key
- **Chosen**: First approach (fee then key) matches `place_bets` pattern where key is checked first

## Storage Impact
- Per event creation: +1 persistent entry (idempotency key)
- Size: ~104 bytes (Address + BytesN<32> + event_id Symbol + metadata)
- TTL: ~7 days, then auto-expired
- Worst case: ~10k events/week × 104 bytes = ~1MB/week (manageable)

## Migration & Compatibility

### Non-Breaking
- `idempotency_key: Option<BytesN<32>>` parameter added to `create_event`
- Existing callers that don't provide key: still work (backward compatible)
- New callers: can provide key for deduplication safety

### Future Hardening
- Could make key mandatory in future version (new entrypoint or upgrade)
- Could auto-derive key from (admin, description, outcomes, end_time) for zero-touch idempotency
- Current approach gives flexibility for rollout

## Testing Strategy

### Test Cases
1. **Happy Path**
   - Fresh key succeeds, event created, fee charged ✓
   - Event id returned correctly

2. **Duplicate Detection**
   - Same key, same admin → `IdempotentBatchAlreadyApplied` (no fee)
   - Verify no double-charge via token balance checks

3. **Key Scope**
   - Admin A with key K succeeds
   - Admin B with same key K bytes → succeeds (different scope)
   - Same admin, different key → succeeds independently

4. **TTL Expiry**
   - Create event with key K → event created
   - Advance ledgers past TTL
   - Reuse key K → succeeds as fresh creation

5. **Boundary Cases**
   - Null/empty key → should reject or handle gracefully
   - Key collision (internal): near-impossible with 256-bit key

6. **Integration**
   - Fee collection happens exactly once
   - Audit trail records event creation correctly
   - Event appears in storage on first call only

7. **Regression**
   - All existing event creation tests still pass
   - No accidental changes to event schema or behavior
