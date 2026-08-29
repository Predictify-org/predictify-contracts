# Replay-Safe Claim Implementation - Invariants & State Transitions

## Overview

This document describes the key invariants and state transitions for the replay-safe prediction claim identifier system implemented in Predictify-contracts.

## Core Invariants

### 1. Nonce Monotonicity Invariant

**Statement**: For any user U on market M, if the claim nonce is incremented N times, then the stored nonce value is exactly N.

**Formal**: `get_nonce(U, M) = number_of_successful_claims(U, M)`

**Guarantee**: The nonce never decreases, never stays the same across successful claims, and never skips values.

**Implementation**: 
- `ClaimNonceManager::increment_nonce()` always returns `current + 1`
- Stored in persistent storage via `env.storage().persistent().set(&key, &new_nonce)`
- No external mechanism can reset or modify the nonce outside increment

### 2. Per-User-Per-Market Isolation Invariant

**Statement**: The claim nonce for user U1 on market M1 is completely independent from the nonce for user U2 or market M2.

**Formal**: 
- `get_nonce(U1, M1)` != `get_nonce(U2, M1)` (unless same claim count)
- `get_nonce(U1, M1)` != `get_nonce(U1, M2)` (unless same claim count)
- Changes to `get_nonce(U1, M1)` do not affect other (U, M) pairs

**Guarantee**: Each (user, market) pair maintains its own independent counter.

**Implementation**: Storage key is `DataKey::ClaimNonce(Address, Symbol)`, which uniquely identifies each (U, M) pair in the type system.

### 3. Claim Uniqueness Invariant

**Statement**: Each unique claim operation (invoke claim_winnings with specific nonce) can only succeed exactly once on the contract.

**Formal**: For a given (user, market, nonce) triple:
- If `validate_nonce(user, market, nonce)` returns Ok(), then the nonce is exactly equal to the stored nonce
- After a successful claim, `validate_nonce(user, market, old_nonce)` returns Err

**Guarantee**: Transaction replays are prevented because the nonce increments, invalidating old proofs.

**Implementation**: 
- Pre-claim validation: `validate_nonce()` checks `provided_nonce == stored_nonce`
- Post-claim mutation: `increment_nonce()` updates stored_nonce to provided_nonce + 1
- Replayed tx with old nonce will fail validation before any state change

### 4. Authorization Binding Invariant

**Statement**: Only the user U (verified via require_auth) can increment the claim nonce for U on any market.

**Formal**: If `require_auth(U)` is called and returns, then only U can invoke operations that affect `get_nonce(U, M)`

**Guarantee**: Users cannot claim on behalf of other users or manipulate other users' nonces.

**Implementation**: 
- `claim_winnings(user, market_id, claim_nonce)` calls `user.require_auth()` before nonce validation
- Nonce operations only run after authorization succeeds
- No other entry point modifies claim nonces

### 5. Idempotency Invariant

**Statement**: The sequence of observable state changes from claim_winnings is idempotent with respect to nonce validation failure.

**Formal**: If claim_winnings is called twice with the same (user, market_id, stale_nonce):
- First call succeeds and increments nonce to N
- Second call fails at nonce validation (InvalidNonce) with nonce still at N
- No additional state change occurs on the second call

**Guarantee**: If a user retransmits a claim with an old nonce, they get a clear error before any state mutation, making retry behavior safe and predictable.

**Implementation**:
- Nonce validation is the first check after authorization
- If validation fails, panic_with_error!(InvalidNonce) before any other operations
- State mutations (balance transfer, market update, etc.) only occur after validation

## State Transitions

### Claim Lifecycle: Happy Path

```
INITIAL STATE:
  T0: User U queries get_claim_nonce(U, M)
      Result: nonce = 0 (no prior claims)
      Storage: ClaimNonce(U, M) not set (defaults to 0)
      Market.claimed[U] not set

CLAIM ATTEMPT:
  T1: Client sends claim_winnings(user=U, market_id=M, claim_nonce=0)
      user.require_auth() validates signature → PASS
      validate_nonce(U, M, 0):
        - Get stored nonce (0, default)
        - Check 0 == 0 → TRUE
        - Return Ok()
      
CLAIM PROCESSING:
  T2: Market state validation passes
      Calculate payout P
      
CLAIM SUCCESS:
  T3: increment_nonce(U, M):
        - Get current nonce (0)
        - Calculate new = 0 + 1 = 1
        - Store ClaimNonce(U, M) = 1
        - Return 1
      
      Store ClaimInfo:
        - claimed = true
        - timestamp = current_ledger_timestamp
        - payout_amount = P
        - claim_nonce = 1
      
      Emit events
      Transfer payout to user balance

POST-CLAIM STATE:
  T4: get_claim_nonce(U, M) returns 1
      Market.claimed[U] = ClaimInfo{...claim_nonce: 1}
      User balance increased by P
```

### Replay Attack: Transaction Replay Flow

```
INITIAL STATE (from above):
  T0: First claim completed
      ClaimNonce(U, M) = 1
      Market.claimed[U].nonce = 1

REPLAY ATTEMPT:
  T1: Attacker/wallet resubmits old transaction:
      claim_winnings(user=U, market_id=M, claim_nonce=0)
      
      user.require_auth() validates signature → PASS
      (signature was valid before, still valid now)
      
      validate_nonce(U, M, 0):
        - Get stored nonce (1, from persistent storage)
        - Check 0 == 1 → FALSE
        - Return Err(InvalidNonce)
      
PANIC:
  T2: panic_with_error!(env, Error::InvalidNonce)
      Transaction reverts
      No state changes
      No balance transfer
      
POST-REPLAY STATE:
  T3: ClaimNonce(U, M) still = 1 (unchanged)
      Market.claimed[U].nonce still = 1 (unchanged)
      User balance unchanged
      No event emitted
```

### Legitimate Retry: After Perceived Failure

```
SCENARIO:
  User calls claim with nonce 0, tx seems to hang/fail
  User thinks: "Better retry"
  
RETRY ATTEMPT 1 (with old nonce):
  T1: claim_winnings(user=U, market_id=M, claim_nonce=0)
      validate_nonce(U, M, 0):
        - Stored nonce is now 1 (from first successful claim)
        - Check 0 == 1 → FALSE
        - Return Err(InvalidNonce)
      Result: ERROR (InvalidNonce)

RECOVERY PATH:
  T2: User/client queries get_claim_nonce(U, M)
      Result: nonce = 1 (current expected nonce)
      
RETRY ATTEMPT 2 (with correct nonce):
  T3: claim_winnings(user=U, market_id=M, claim_nonce=1)
      validate_nonce(U, M, 1):
        - Stored nonce is 1
        - Check 1 == 1 → TRUE
        - Return Ok()
      
      But Market.claimed[U].is_claimed() is true
      Result: ERROR (AlreadyClaimed)
      
      OR if already-claimed check didn't exist:
      Would increment nonce to 2 and attempt double payout
      But IDEMPOTENCY prevents this via:
      - Nonce tracks which specific invocation succeeded
      - Attempting with nonce=1 again would fail after first success

FINAL STATE:
  T4: Nonce = 1 (persists)
      Market.claimed[U].nonce = 1
      Only one payout occurred (from first successful claim)
      User gets clear error on retries (consistent UX)
```

## Authorization Integration

### Authorization Boundary

The authorization check (`user.require_auth()`) creates a cryptographic boundary that:

1. **Binds nonce operations to the user**: Only the user who signed can increment their own nonce
2. **Prevents impersonation**: Cross-user claim attempts fail at authorization before nonce checks
3. **Works independently**: Authorization failure happens regardless of nonce state

### Example: Cross-User Attack (Prevented)

```
ATTACK SCENARIO:
  User U1 successfully claimed (nonce = 1)
  Attacker wants to claim as U1 using correct nonce = 1

ATTEMPT:
  claim_winnings(user=U1, market_id=M, claim_nonce=1)
  
  Soroban's require_auth(U1):
    - Checks if current invocation was signed by U1
    - Attacker did NOT sign as U1
    - require_auth() panics with authorization error
    
  Result: FAIL at authorization boundary
  (Never reaches nonce validation)
```

## Error Classification

### InvalidNonce (Error::InvalidNonce = 113)

**Trigger**: `validate_nonce()` detects nonce mismatch

**Root Causes**:
1. **Transaction replay**: User resubmits old transaction with stale nonce
2. **Out-of-order execution**: Claims arrive out of order (unlikely in Soroban deterministic model)
3. **Client bug**: Client passes wrong nonce to contract

**Recovery**:
1. Query `get_claim_nonce(user, market_id)` to get current expected nonce
2. Resubmit with correct nonce
3. If already claimed, expect AlreadyClaimed error instead

### AlreadyClaimed (Error::AlreadyClaimed = 106)

**Trigger**: `market.claimed.get(user).is_claimed()` returns true

**Root Causes**:
1. **Legitimate duplicate attempt**: User already successfully claimed
2. **Nonce validation passed but claimed flag exists**: Should be rare (indicated idempotency success)

**Recovery**:
1. This is expected behavior, not an error
2. User already received their payout
3. No action needed (claim is already settled)

## Testing Strategy

### Unit Tests: ClaimNonceManager

Tests verify the nonce manager in isolation:
- Nonce starts at 0
- Nonce increments by exactly 1
- Validation matches stored nonce
- Independence across users and markets
- Persistence across calls

### Integration Tests: Claim Lifecycle

Tests verify full scenarios:
- Happy path: First claim with nonce 0
- Replay detection: Second claim with nonce 0 fails
- Nonce query: Clients can retrieve current nonce
- Multiple users: Independent nonce counters
- Zero payouts: Nonce increments even when no funds transfer

### Edge Cases

- Nonce overflow: u64 max (~18 billion) - acceptable per user per market
- Concurrent claims: Soroban single-threaded, no race conditions
- Failed transfers: Nonce only increments on success (after all validation)

## Backward Compatibility

### Migration Strategy

1. **Old Contracts**: Claims without nonce tracking; claimed flag only
2. **Upgraded Contracts**: All subsequent claims require nonce parameter
3. **Existing Claims**: Marked as claimed; no nonce field (defaults to 0)
4. **No Data Loss**: Old claims prevent double-payout naturally via claimed flag

### Breaking Change: Function Signature

```rust
// OLD (no longer accepted)
claim_winnings(env, user, market_id) -> i128

// NEW (required)
claim_winnings(env, user, market_id, claim_nonce: u64) -> i128
```

**Client Impact**:
1. Clients must query `get_claim_nonce()` before calling `claim_winnings()`
2. Clients must include nonce in transaction
3. Retry logic must query fresh nonce

**Mitigation**:
- Helper function `claim_with_auto_nonce()` automatically handles nonce retrieval
- Test client automatically manages nonce in integration tests
- Clear documentation in API changes

## Key Properties Summary

| Property | Value |
|----------|-------|
| Nonce range | 0 to u64::MAX (~18.4 billion) |
| Nonce per-user state | Persistent across ledger history |
| Nonce persistence TTL | Market TTL (365 days default) |
| Validation latency | O(1) lookup from persistent storage |
| Increment latency | O(1) write to persistent storage |
| Storage key format | `DataKey::ClaimNonce(Address, Symbol)` |
| Error on replay | `Error::InvalidNonce (113)` |
| Authorization check | `require_auth(user)` before nonce check |
| Validation sequence | Circuit breaker → auth → nonce → market state |
