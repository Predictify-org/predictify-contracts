# Market Lifecycle Stateful Fuzzing

## Overview

This document describes the stateful property-based testing infrastructure for the Predictify Hybrid prediction market contract using `proptest`. The stateful fuzzing tests validate the complete market lifecycle, invariants, and edge cases through randomized operation sequences.

## Purpose

The stateful fuzzing tests serve to:

1. **Validate Invariants**: Ensure that critical business rules hold across all possible operation sequences
2. **Discover Edge Cases**: Identify unexpected behaviors and edge cases through randomized testing
3. **Verify State Transitions**: Confirm that market state transitions follow the defined lifecycle
4. **Test Authorization**: Validate that unauthorized operations are properly rejected
5. **Ensure Consistency**: Check that balances, payouts, and stakes remain consistent

## Test Structure

### Test File

- **Location**: `tests/stateful.rs`
- **Framework**: `proptest` for property-based testing
- **Coverage**: Complete market lifecycle from creation to closure

### Key Components

#### 1. TestState

Tracks the entire test environment including:
- Soroban environment and contract
- Token contract for staking
- Admin and user addresses
- Market models (expected state)
- User balances

#### 2. MarketModel

Represents the expected state of a market:
- Market ID and state
- Outcomes and creator
- End time
- Total stakes per outcome
- Votes and bets
- Resolved outcome
- Claimed winnings

#### 3. Operations

Six types of operations that can be applied:

1. **CreateMarket**: Create a new prediction market
   - Parameters: creator index, duration days, number of outcomes
   - Validates: Market creation rules

2. **PlaceVote**: User places a vote with stake
   - Parameters: user index, market index, outcome index, stake amount
   - Validates: Vote authorization, market state, outcome validity

3. **PlaceBet**: User places a bet
   - Parameters: user index, market index, outcome index, bet amount
   - Validates: Bet authorization, market state, balance

4. **AdvanceTime**: Move ledger time forward
   - Parameters: number of days
   - Effect: Triggers state transitions (Active → Ended)

5. **ResolveMarket**: Resolve a market with winning outcome
   - Parameters: market index, winning outcome index
   - Validates: Market must be Ended

6. **ClaimWinnings**: User claims winnings
   - Parameters: user index, market index
   - Validates: Market resolved, user eligible, no double claims

## Invariants Validated

The tests enforce the following invariants after every operation:

### 1. State Transition Validity
- Markets follow valid state transitions
- No illegal state changes occur

### 2. Outcome Consistency
- Resolved markets always have a valid outcome
- Outcomes match one of the defined options

### 3. Stake Non-Negativity
- Total stakes for each outcome remain ≥ 0
- No negative balances are created

### 4. Vote/Bet Exclusivity
- Users cannot both vote and bet on the same market (if enforced by contract)

### 5. Claim Ordering
- No claims occur before resolution
- Duplicate claims are prevented

## Property Tests

### 1. Lifecycle Invariants Test

```rust
#[test]
fn test_market_lifecycle_invariants(operations in prop::collection::vec(Operation::strategy(), 1..MAX_OPERATIONS))
```

- **Purpose**: Validates that arbitrary operation sequences maintain all invariants
- **Strategy**: Generate 1-20 random operations
- **Cases**: 100 test cases per run
- **Validation**: Checks all invariants after each operation

### 2. State Transitions Test

```rust
#[test]
fn test_state_transitions(duration_days in 1u32..=30, time_advance_days in 1u32..=60)
```

- **Purpose**: Verifies correct market state transitions based on time
- **Strategy**: Create market with random duration, advance time randomly
- **Validation**: Market state matches expected based on time elapsed

### 3. Idempotency Test

```rust
#[test]
fn test_idempotency(user_idx in 0..MAX_USERS, stake in 1i128..=MAX_STAKE)
```

- **Purpose**: Ensures duplicate operations are handled correctly
- **Strategy**: Attempt to vote twice on same market
- **Validation**: Second vote fails with AlreadyVoted error

## Unit Tests

Additional unit tests validate specific scenarios:

### 1. Basic Market Creation
- Tests successful market creation
- Verifies market is in Active state

### 2. Vote on Active Market
- Tests voting on an active market
- Validates vote is accepted

### 3. No Vote After Market Ends
- Tests that votes are rejected after market end time
- Validates temporal business rules

## Configuration

### Constants

```rust
const MAX_OPERATIONS: usize = 20;       // Max operations per test
const MAX_USERS: usize = 5;             // Max users to simulate
const MAX_STAKE: i128 = 1_000_000_000;  // 1,000 XLM max stake
const INITIAL_BALANCE: i128 = 10_000_000_000; // 10,000 XLM per user
```

### Proptest Config

```rust
ProptestConfig {
    cases: 100,                    // 100 test cases per property
    max_shrink_iters: 1000,       // Shrink failures up to 1000 iterations
    ..ProptestConfig::default()
}
```

## Running the Tests

### Run Stateful Tests Only

```bash
cargo test -p predictify-hybrid --test stateful
```

### Run with Verbose Output

```bash
cargo test -p predictify-hybrid --test stateful -- --nocapture
```

### Run All Tests

```bash
cargo test -p predictify-hybrid
```

## Test Coverage

The stateful fuzzing tests provide coverage for:

- ✅ Market creation with various parameters
- ✅ Voting on active markets
- ✅ Betting on active markets
- ✅ Time-based state transitions
- ✅ Market resolution
- ✅ Winnings claims
- ✅ Authorization checks
- ✅ Idempotency guarantees
- ✅ Balance consistency
- ✅ Invariant preservation

## Interpreting Results

### Success

```
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured
```

All property tests passed across all generated cases.

### Failure

When a property test fails, proptest will:
1. Show the failing operation sequence
2. Attempt to shrink the failure to a minimal case
3. Display the specific invariant violation

Example:
```
thread 'test_market_lifecycle_invariants' panicked at 'Invariant violation after operation 5 (PlaceVote { ... }): Market mk_0 outcome A has negative stake: -100'
```

### Regression Files

Failed cases are saved in `proptest-regressions/stateful.txt` for replay.

## Best Practices

1. **Add New Operations**: When adding contract features, add corresponding operations to the fuzzer
2. **Update Invariants**: Keep invariants in sync with business rules
3. **Check Coverage**: Ensure all critical paths are exercised
4. **Review Failures**: Investigate all failures - they indicate bugs or missing invariants
5. **Maintain Models**: Keep MarketModel in sync with actual Market type

## Integration with CI/CD

The stateful tests are included in the standard test suite and run on:
- Pre-commit hooks (if configured)
- CI pipeline for all pull requests
- Pre-deployment validation

## Future Enhancements

Potential improvements to the fuzzing infrastructure:

1. **Add Dispute Operations**: Test dispute lifecycle
2. **Multi-Market Scenarios**: Test interactions between multiple markets
3. **Fee Validation**: Add fee calculation invariants
4. **Oracle Integration**: Test oracle callback fuzzing
5. **Extended Lifecycle**: Add market cancellation and closure operations
6. **Performance Metrics**: Track gas usage across operations
7. **Adversarial Testing**: Add malicious operation sequences

## References

- [Proptest Documentation](https://altsysrq.github.io/proptest-book/intro.html)
- [Soroban Testing Guide](https://developers.stellar.org/docs/build/guides/testing)
- [Stellar Fuzzing Guide](https://developers.stellar.org/docs/build/guides/testing/fuzzing)
- [Property-Based Testing Patterns](https://fsharpforfunandprofit.com/posts/property-based-testing/)

## Support

For issues or questions about the stateful fuzzing tests:
- Check test output for specific failure details
- Review invariant definitions in `stateful.rs`
- Consult Soroban testing documentation
- Open an issue on the repository
