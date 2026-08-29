# Replay-Safe Prediction Claim Identifiers - Design Specification

## Executive Summary

Implement deterministic, replay-safe identifiers for prediction market winnings claims to prevent transaction replay attacks in the Soroban environment.

## Problem Statement

The current `claim_winnings` implementation is vulnerable to transaction replay attacks:

1. **Current State**: Claims are tracked via a simple `claimed` boolean flag in the `Market.claimed` map
2. **Vulnerability**: While `require_auth()` verifies authorization, it does NOT prevent replaying old transactions
3. **Attack Vector**: An attacker can resubmit a previously-signed claim transaction, and if the claimed check is bypassed or removed, funds are double-paid
4. **Root Cause**: No per-claim unique identifier to distinguish fresh invocations from replayed transactions

## Design Goals

1. **Deterministic**: Work reliably in Soroban's deterministic execution model
2. **Minimal**: Smallest complete implementation without over-engineering
3. **Compatible**: Preserve existing public interfaces (no breaking changes to `claim_winnings` signature)
4. **Observable**: Track state transitions and authorization invariants in code comments
5. **Verifiable**: Comprehensive tests demonstrating replay protection

## Design Specification

### 1. Core Mechanism: Per-User Claim Nonces

Each user gets a monotonically-increasing claim nonce **per market**. Storage model:

```
DataKey::ClaimNonce(Address, Symbol) -> u64
```

- `Address`: User claiming the winnings
- `Symbol`: Market ID
- `u64`: Monotonic counter (starts at 0 for each user-market pair)

### 2. ClaimInfo Structure Enhancement

Extend `ClaimInfo` to include a nonce identifier:

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimInfo {
    pub claimed: bool,
    pub timestamp: u64,
    pub payout_amount: i128,
    pub claim_nonce: u64,  // NEW: Unique per (user, market) claim
}
```

**Invariants**:
- `claim_nonce` increments by exactly 1 on each successful claim (no resets)
- `claim_nonce` is stored in both `ClaimInfo` AND persistent storage key `DataKey::ClaimNonce`
- On replay: the provided nonce < stored nonce → error `InvalidNonce`

### 3. claim_winnings Signature Update

**Old**:
```rust
pub fn claim_winnings(env: Env, user: Address, market_id: Symbol) -> i128
```

**New**:
```rust
pub fn claim_winnings(
    env: Env, 
    user: Address, 
    market_id: Symbol,
    claim_nonce: u64,  // NEW: Caller must include the nonce
) -> i128
```

**Rationale**:
- Caller (user or third-party on behalf of user) must supply the nonce they expect
- This becomes part of the signed transaction
- Replay of the same transaction includes the same nonce, which no longer matches the incremented stored nonce

### 4. Validation Logic

In `claim_winnings`:

1. **Check nonce validity**:
   ```
   stored_nonce = DataKey::ClaimNonce(user, market_id) || 0 (first time)
   if claim_nonce != stored_nonce:
       panic(Error::InvalidNonce)
   ```

2. **Prevent double-claim**:
   ```
   if market.claimed.get(user).map(|info| info.is_claimed()).unwrap_or(false):
       panic(Error::AlreadyClaimed)
   ```
   (Redundant safety check; nonce check should prevent this)

3. **On success**:
   - Increment and store: `new_nonce = stored_nonce + 1`
   - Save to `DataKey::ClaimNonce(user, market_id)` → `new_nonce`
   - Save to `ClaimInfo.claim_nonce` → `new_nonce`
   - Store updated `Market.claimed` with new `ClaimInfo`

### 5. Error Codes

Add new error variant:

```rust
pub enum Error {
    // ... existing errors ...
    InvalidNonce,  // Provided claim_nonce does not match expected nonce
}
```

## State Transitions

### Happy Path (First Claim)

```
Initial State:
  - DataKey::ClaimNonce(user, market_id): NOT SET (0 in query)
  - market.claimed[user]: NOT SET

Invocation:
  - claim_winnings(user, market_id, claim_nonce=0)
  
Validation:
  ✓ stored_nonce = 0 (default)
  ✓ claim_nonce (0) == stored_nonce (0)
  ✓ market.claimed[user] is empty
  
Result:
  - Payout transferred to user
  - DataKey::ClaimNonce(user, market_id) = 1
  - market.claimed[user] = ClaimInfo {
      claimed: true,
      timestamp: now,
      payout_amount: X,
      claim_nonce: 1,
    }
```

### Replay Attack Scenario

```
After first claim:
  - DataKey::ClaimNonce(user, market_id) = 1
  - market.claimed[user].is_claimed() = true

Attacker replays:
  - claim_winnings(user, market_id, claim_nonce=0)  ← SAME nonce as original
  
Validation:
  ✗ stored_nonce = 1 (from persistent storage)
  ✗ claim_nonce (0) != stored_nonce (1)
  ✗ Panic: Error::InvalidNonce
  
Result:
  - Transaction reverts
  - No double-payout
  - User balance unchanged
```

### Legitimate Retry (User thinks tx failed)

```
User submits with nonce=0 (first time)
  → Success, stored_nonce becomes 1

Network/wallet thinks it failed, user retries with nonce=0 again
  → Validation: 0 != 1 → Error::InvalidNonce
  
User must know/query the current nonce and resubmit with nonce=1
  → Validation: 1 == 1 ✓ (but AlreadyClaimed fires before nonce check)
  → Consistent failure message (no silent double-pay)
```

## Storage Layout

### New Storage Key

Add to `DataKey` enum in `storage.rs`:

```rust
pub enum DataKey {
    // ... existing variants ...
    
    /// Per-user claim nonce for replay protection: (user, market_id) -> u64
    /// Incremented on each successful claim to ensure each claim is unique.
    ClaimNonce(Address, Symbol),
}
```

### Query Interface

For off-chain clients, add query function:

```rust
pub fn get_claim_nonce(env: Env, user: Address, market_id: Symbol) -> u64 {
    let key = DataKey::ClaimNonce(user.clone(), market_id);
    env.storage().persistent().get(&key).unwrap_or(0)
}
```

## Backward Compatibility

### Migration Strategy

1. **Default Behavior**: If `claim_nonce` field missing from `ClaimInfo`, treat as 0
2. **Upgrade Path**: 
   - Old contract: Claims without nonce tracking
   - New contract: Requires nonce in signature
   - After upgrade: All subsequent claims must include nonce
   - Existing claims: Marked as claimed (no replay possible via UI)

### Public API Impact

**Breaking Change**: `claim_winnings` signature changes from:
```rust
fn claim_winnings(env: Env, user: Address, market_id: Symbol) -> i128
```

to:
```rust
fn claim_winnings(env: Env, user: Address, market_id: Symbol, claim_nonce: u64) -> i128
```

**Mitigation**: 
- Document upgrade path in changelog
- Provide migration guide for clients
- No state loss (only add nonce tracking)

## Security Properties

### Invariants

1. **Nonce Monotonicity**: For each (user, market_id), `claim_nonce` strictly increases by 1 on each successful claim
2. **Claim Uniqueness**: Each unique claim has a distinct (user, market_id, nonce) tuple
3. **Replay Prevention**: An old transaction with (user, market_id, old_nonce) cannot succeed after first execution (stored_nonce > old_nonce)
4. **Authorization Binding**: `require_auth(user)` ensures only the user (or authorized agent with signature) can increment their nonce

### Boundary Cases

- **Zero Nonce**: First claim uses nonce=0; valid only once
- **Nonce Overflow**: u64 max (~18 billion) per user per market; acceptable limit
- **Concurrent Calls**: Not possible in single-threaded Soroban; no race conditions
- **Failed Transaction**: If tx fails before nonce increment, next attempt uses same nonce (safe)

## Testing Strategy

### Test Categories

1. **Basic Functionality**
   - First claim succeeds with nonce=0
   - Subsequent claims require incremented nonce

2. **Replay Protection**
   - Replayed tx with old nonce fails with `InvalidNonce`
   - Double-claim prevented by nonce validation

3. **State Invariants**
   - Nonce stored in both persistent storage and ClaimInfo
   - Nonce monotonically increases
   - No nonce resets between markets

4. **Authorization**
   - Claim with wrong user fails `require_auth()`
   - Claim with correct user but wrong nonce fails `InvalidNonce`

5. **Edge Cases**
   - Multiple users can claim independently
   - Each user has separate nonce per market
   - Nonce values do not collide across markets or users

## Implementation Checklist

- [ ] Add `claim_nonce: u64` field to `ClaimInfo` struct
- [ ] Add `ClaimNonce(Address, Symbol)` to `DataKey` enum
- [ ] Implement `get_claim_nonce` query function
- [ ] Update `claim_winnings` signature to include `claim_nonce` parameter
- [ ] Implement nonce validation logic in `claim_winnings`
- [ ] Implement nonce increment and storage logic
- [ ] Add `InvalidNonce` error code
- [ ] Update event emission (if applicable)
- [ ] Write comprehensive unit tests
- [ ] Write replay attack simulation tests
- [ ] Document state transitions in code comments
- [ ] Verify WASM size remains within budget
- [ ] Update API documentation

## References

- Existing EventNonce pattern: `src/storage.rs` (line ~160), `src/events.rs`
- ClaimInfo implementation: `src/types.rs` (line 1170)
- claim_winnings implementation: `src/lib.rs` (line 1692)
- Test suite: `src/claim_idempotency_tests.rs`
