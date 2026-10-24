# Predictify Hybrid Smart Contract - Formal Payout Specification

## Overview

This document provides a comprehensive formal specification for the payout calculation and distribution mechanism in the Predictify Hybrid prediction market smart contract.

## 0. Resolution and Payout Timeline

Resolution and payouts are deliberately separated so that no winner can be paid
before the result has been challengeable for a full dispute window.

```text
          market ends            result proposed                 payouts open
              |                         |                              |
              |<--- resolution may ---->|                              |
              |     happen at any       |<--- dispute window --------->|
              |     time after end      |    (dispute_window_seconds)  |
              |                         |                              |
         end_time                   resolved_at              resolved_at + window
```

Rules:

1. **Resolution is one-way.** `resolve_market_manual`, `resolve_market_with_ties`
   and `force_resolve_market` reject a market whose `winning_outcomes` is set or
   whose state is `Resolved` with `Error::MarketResolved`. A `Cancelled` market
   is rejected with `Error::InvalidState`. Force-resolve may bypass the market's
   end time, but it cannot re-settle a resolved market or revive a cancelled one.
2. **Resolution time is recorded.** Every successful resolution stores the
   ledger timestamp as the market's claim-window start (the existing
   `UnclaimedWinningsPolicy::set_claim_window_start_if_missing` key). This value
   is the `resolved_at` used by the dispute and payout gates. Resolved markets
   created before this field existed fall back to `end_time`.
3. **The dispute window runs from `resolved_at`, not `end_time`.** A result
   proposed long after `end_time` still gets a full `dispute_window_seconds`
   during which it can be disputed. Once that window closes the result is final.
4. **Payouts are a separate step.** Resolution never calls `distribute_payouts`.
   `claim_winnings`, `distribute_payouts` and `sweep_unclaimed_winnings` all use
   the shared `require_payout_allowed` gate, which requires:
   - a final outcome (`winning_outcomes` is set; otherwise
     `Error::MarketNotResolved`),
   - `now >= resolved_at + dispute_window_seconds` (otherwise
     `Error::InvalidState`),
   - no open (`Active`) dispute (otherwise `Error::InvalidState`).
5. **Claims stay idempotent.** Once a winner is credited through either
   `claim_winnings` or `distribute_payouts`, the market's `claimed` map prevents
   any other payout path from crediting the same winner again.

Error precedence for payout paths is `MarketNotResolved`, then
`InvalidState` (window still open), then `InvalidState` (open dispute), then
`AlreadyClaimed`.

## 1. Fee Source and Payout Calculation

The `ConfigManager` platform fee is the canonical source. The fee applies at
market resolution time, so a fee update affects every still-open market in the
same way regardless of whether winners claim individually or receive a
distributed payout. The legacy `platform_fee` storage value is only a
read-compatibility fallback when the canonical config is absent and is kept in
sync by `set_platform_fee`.

All fee values are basis points in the range 0–1000; 10,000 bps is 100%.

### 1.1 Basic Payout Formula

For a market with total pool `P`, total stake on all winning outcomes `W`, a
winning user's stake `S`, and platform fee `F` bps:

```
user_share = floor(S * (10_000 - F) / 10_000)
payout = floor(user_share * P / W)
platform_fee = floor(P * F / 10_000)
payout_remainder = P - platform_fee - sum(all winner payouts)
```

Every division rounds down (floor) toward the user payout. The unallocated
remainder is credited to the fee vault. Consequently, for every resolved
market, `sum(all winner payouts) + platform_fee + payout_remainder == P`
exactly in integer units. All intermediate multiplication and addition uses
checked arithmetic and reports an overflow error.

## 2. Fee Structure

### 2.1 Platform Fee Configuration

```rust
pub struct FeeConfig {
    pub platform_fee_percentage: i128,  // In basis points (1/100th of percent)
    pub creation_fee: i128,             // Fixed fee to create markets
    pub min_fee: i128,                 // Minimum platform fee
    pub max_fee: i128,                 // Maximum platform fee
    pub fees_enabled: bool,              // Whether fees are active
}
```

### 2.2 Default Fee Structures

#### Testnet Configuration
- Platform Fee: 2% (200 basis points)
- Creation Fee: 1 XLM
- Minimum Fee: 0.1 XLM
- Maximum Fee: 100 XLM

#### Mainnet Configuration
- Platform Fee: 3% (300 basis points)
- Creation Fee: 1.5 XLM
- Minimum Fee: 0.2 XLM
- Maximum Fee: 200 XLM

### 2.3 Fee Application

Fees are only deducted from winning payouts, not from:
- Market creation fees (paid separately)
- Losing bets (user loses entire stake)
- Refunds (full stake returned)

## 3. Multi-Outcome and Tie Handling

### 3.1 Winning Outcomes Array

The system supports multiple winning outcomes for tie scenarios:

```rust
market.winning_outcomes: Option<Vec<String>>
```

### 3.2 Tie Distribution

When multiple outcomes are declared winners:
1. Calculate `total_winning_bets` as sum of bets on all winning outcomes
2. Each winner receives proportionate share based on their contribution to winning pool
3. Formula applies identically to all winning outcomes

### 3.3 Example Tie Scenario

```
Market: "Who will win the match?"
Outcomes: ["Team A", "Team B", "Draw"]
Bets: Team A (5000), Team B (3000), Draw (2000)
Winning Outcomes: ["Team A", "Draw"]  // Tie declared
Total Winning Bets: 5000 + 2000 = 7000
Total Pool: 10000

Team A Winner Payout: (5000/7000) * 10000 * 0.98 = 7000 tokens
Draw Winner Payout: (2000/7000) * 10000 * 0.98 = 2800 tokens
```

## 4. Payout Calculation Implementation

### 4.1 Core Function

All settlement operations call `effective_fee_bps(env, market)`. It reads the
canonical configuration, validates the fee bounds, and consults the legacy key
only when the canonical config is absent. If neither source exists, it returns
`ConfigNotFound`; settlement has no implicit fee default.

### 4.2 Utility Calculation

```rust
pub fn calculate_payout(
    user_stake: i128,
    winning_total: i128,
    total_pool: i128,
    fee_percentage: i128,
) -> Result<i128, Error> {
    if winning_total == 0 {
        return Err(Error::NothingToClaim);
    }

    // Apply overflow protection
    let user_share = (user_stake
        .checked_mul(10_000 - fee_percentage)
        .ok_or(Error::Overflow)?)
        / 10_000;
    
    let payout = (user_share
        .checked_mul(total_pool)
        .ok_or(Error::Overflow)?)
        / winning_total;

    Ok(payout)
}
```

## 5. Payout Distribution Process

### 5.1 Claim Requirements

Users can claim winnings when:
1. Market is in `Resolved` state
2. User has a winning bet
3. Payout has not been claimed previously
4. Market dispute window has expired (if applicable)

### 5.2 Claim Function

```rust
pub fn claim_winnings(
    env: Env,
    user: Address,
    market_id: Symbol,
) -> Result<i128, Error> {
    // 1. Authenticate user
    user.require_auth();

    // 2. Validate market state
    let market = MarketStateManager::get_market(&env, &market_id)?;
    if market.state != MarketState::Resolved {
        return Err(Error::MarketNotResolved);
    }

    // 3. Calculate payout amount
    let payout_amount = Self::calculate_bet_payout(&env, &market_id, &user)?;

    // 4. Check if already claimed
    let claim_key = ClaimKey { market_id, user: user.clone() };
    if ClaimStorage::has_claimed(&env, &claim_key) {
        return Err(Error::AlreadyClaimed);
    }

    // 5. Transfer tokens
    TokenClient::transfer(&env, &market.token_id, &user, payout_amount)?;

    // 6. Mark as claimed
    ClaimStorage::record_claim(&env, &claim_key, payout_amount);

    // 7. Emit event
    EventEmitter::emit_winnings_claimed(&env, &market_id, &user, payout_amount);

    Ok(payout_amount)
}
```

### 5.3 Claim Tracking

The system tracks claimed payouts to prevent double-claiming:

```rust
pub struct ClaimRecord {
    pub market_id: Symbol,
    pub user: Address,
    pub amount: i128,
    pub timestamp: u64,
}
```

## 6. Edge Cases and Error Handling

### 6.1 Zero Winning Total

If `total_winning_bets` is 0 (should never happen in resolved market):
- Returns `Error::NothingToClaim`
- Prevents division by zero

### 6.2 Overflow Protection

All calculations use `checked_mul()` and `checked_div()`:
- Prevents integer overflow in large payout scenarios
- Returns `Error::InvalidInput` on overflow

### 6.3 Invalid Market States

Payout claims are rejected for markets in:
- `Active` state: `Error::MarketNotResolved`
- `Ended` state: `Error::MarketNotResolved`
- `Cancelled` state: `Error::MarketCancelled`

### 6.4 Already Claimed

Users cannot claim winnings multiple times:
- Returns `Error::AlreadyClaimed`
- Persistent claim tracking prevents double-claiming

## 7. Payout Multipliers

### 7.1 Calculation

For user interface display, the system calculates payout multipliers:

```rust
pub fn calculate_payout_multiplier(env: &Env, market_id: &Symbol, outcome: &String) -> i128 {
    let stats = BetStorage::get_market_bet_stats(env, market_id);
    let outcome_amount = stats.outcome_totals.get(outcome.clone()).unwrap_or(0);
    
    if stats.total_amount_locked == 0 {
        return 0;
    }

    // Return multiplier scaled by 100 for precision
    (outcome_amount * 100) / stats.total_amount_locked
}
```

### 7.2 Multiplier Interpretation

- Multiplier of 100: User would double their money (break-even)
- Multiplier > 100: User makes profit
- Multiplier < 100: User loses money (should not happen for winning bets)

## 8. Integration with Oracle Resolution

### 8.1 Hybrid Resolution

The payout system integrates with the hybrid oracle-community resolution:
1. Oracle provides primary outcome
2. Community voting provides consensus
3. Final outcome determined by weighted algorithm
4. Payouts calculated based on final outcome

### 8.2 Dispute Window

Markets may have a dispute window after resolution:
- Payouts are locked during dispute period
- Users can challenge outcomes
- After dispute window expires, payouts become claimable

## 9. Security Considerations

### 9.1 Reentrancy Protection

- All state changes happen before external calls
- Claim status updated before token transfer
- Events emitted after successful operations

### 9.2 Access Control

- Only bet owners can claim their winnings
- Market resolution requires proper authorization
- Fee configuration restricted to admin

### 9.3 Overflow Protection

- All arithmetic operations use checked variants
- Explicit bounds validation
- Graceful error handling for edge cases

## 10. Testing and Validation

### 10.1 Test Coverage

The payout system includes comprehensive tests:
- Single winner scenarios
- Multi-winner (tie) scenarios
- Edge cases (zero amounts, overflow conditions)
- Fee calculation accuracy
- Claim prevention mechanisms

### 10.2 Example Test Cases

```rust
#[test]
fn test_payout_calculation() {
    // Single winner: 1000 stake, 5000 winning total, 10000 pool, 2% fee
    let payout = calculate_payout(1000, 5000, 10000, 200).unwrap();
    assert_eq!(payout, 1960); // (1000 * 98 / 100) * 10000 / 5000
    
    // Tie scenario: 2000 stake, 7000 winning total, 10000 pool, 2% fee
    let payout = calculate_payout(2000, 7000, 10000, 200).unwrap();
    assert_eq!(payout, 2800); // (2000 * 98 / 100) * 10000 / 7000
}
```

## 11. Gas and Performance Considerations

### 11.1 Optimization

- Minimal storage reads for payout calculations
- Efficient batch operations for multiple claims
- Optimized fee structure lookups

### 11.2 Limits

- Maximum payout amount limited by available contract balance
- Maximum number of claims per transaction to prevent gas exhaustion
- Reasonable bounds on market sizes and bet amounts

## 12. Future Enhancements

### 12.1 Potential Improvements

1. **Batch Claims**: Allow users to claim multiple market winnings in one transaction
2. **Dynamic Fees**: Time-based or volume-based fee adjustments
3. **Insurance Fund**: Protection against oracle failures or contract insolvency
4. **Liquidity Pools**: Automated market maker incentives

### 12.2 Extensibility

The payout system is designed to be:
- Modular for easy fee structure modifications
- Compatible with different token types
- Extensible for new resolution mechanisms
- Upgradable without breaking existing claims

---

**Specification Version**: 1.0  
**Last Updated**: 2026-03-27  
**Contract**: Predictify Hybrid v0.0.0

This specification serves as the authoritative reference for payout calculation and distribution logic in the Predictify Hybrid smart contract.
