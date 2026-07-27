# Resolution State Property Tests (GrantFox FWC26 Campaign)

## Overview

This document describes the property-based test suite for resolution state invariants implemented for the GrantFox FWC26 campaign. The tests validate the correctness of the `ResolutionState` enum and `ResolutionUtils::get_resolution_state` function across all possible market configurations.

## Purpose

Property-based testing complements traditional unit tests by:
- **Exhaustive validation**: Testing across the full input space rather than hand-picked cases
- **Edge case discovery**: Finding unexpected behaviors through randomized generation
- **Invariant enforcement**: Ensuring business rules hold for all valid inputs
- **Regression prevention**: Detecting future changes that violate invariants

## ResolutionState Logic

The `ResolutionState` enum represents the resolution status of a prediction market with the following priority order:

```rust
pub enum ResolutionState {
    Active,           // No special fields set
    OracleResolved,   // oracle_result is set, but no winning_outcomes
    MarketResolved,   // winning_outcomes is set (highest priority)
    Disputed,         // dispute_stakes > 0, no oracle_result or winning_outcomes
    Finalized,        // Resolution is final and immutable
}
```

### State Determination Algorithm

The state is determined by `ResolutionUtils::get_resolution_state` using this priority:

```text
if winning_outcomes.is_some() → MarketResolved
else if oracle_result.is_some() → OracleResolved
else if total_dispute_stakes() > 0 → Disputed
else → Active
```

**Key insight**: Higher-priority fields override lower-priority ones, regardless of other field values.

## Invariants Tested

### 1. State Priority Invariants

- **Invariant 1**: `winning_outcomes` takes precedence over `oracle_result`
  - If `winning_outcomes.is_some()`, state must be `MarketResolved`
  - Even if `oracle_result` and `dispute_stakes` are also set

- **Invariant 2**: `oracle_result` takes precedence over `dispute_stakes`
  - If `oracle_result.is_some()` and no `winning_outcomes`, state must be `OracleResolved`
  - Even if `dispute_stakes > 0`

- **Invariant 3**: `dispute_stakes` takes precedence over `Active`
  - If `total_dispute_stakes() > 0` and no higher-priority fields, state must be `Disputed`

- **Invariant 4**: No special fields → `Active` state
  - If all special fields are absent/empty, state must be `Active`

### 2. State Consistency Invariants

- **Invariant 5**: Deterministic state determination
  - Same market configuration always produces the same state
  - No randomness or external factors should affect state

- **Invariant 6**: `MarketResolved` implies `winning_outcomes` is set
  - If state is `MarketResolved`, `winning_outcomes` must be `Some(...)`

- **Invariant 7**: `OracleResolved` implies `oracle_result` is set
  - If state is `OracleResolved`, `oracle_result` must be `Some(...)`
  - And `winning_outcomes` must be `None`

- **Invariant 8**: `Disputed` implies `dispute_stakes > 0`
  - If state is `Disputed`, `total_dispute_stakes()` must be `> 0`
  - And both `oracle_result` and `winning_outcomes` must be `None`

- **Invariant 9**: `Active` implies no special fields
  - If state is `Active`, all special fields must be absent/empty

### 3. Edge Case Invariants

- **Invariant 10**: Multiple dispute stakeholders still yield `Disputed`
  - Multiple users with dispute stakes should still produce `Disputed` state
  - Total stake should be sum of all individual stakes

- **Invariant 11**: Zero dispute stakes does not yield `Disputed`
  - If `total_dispute_stakes() == 0`, state should not be `Disputed`

- **Invariant 12**: State is invariant to `MarketState` enum
  - `ResolutionState` should not depend on the `MarketState` enum value
  - These are separate concerns (lifecycle vs resolution status)

## Test Configuration

### Proptest Settings

```rust
ProptestConfig::with_cases(100)  // 100 test cases per property
```

### Test Generators

- **`arb_oracle_result`**: Generates `Option<String>` (None, Some("yes"), Some("no"))
- **`arb_winning_outcomes`**: Generates `Option<Vec<String>>` (None, single outcome, multiple outcomes)
- **`arb_dispute_stake`**: Generates dispute stake amounts (0 to 1,000 XLM)
- **`arb_dispute_stakes`**: Generates multiple dispute stakeholders (0-5 users)

## Running the Tests

### Run All Resolution State Property Tests

```bash
cargo test -p predictify-hybrid resolution_state_property_tests
```

### Run Specific Property Test

```bash
cargo test -p predictify-hybrid test_resolution_state_property_name
```

### Run with Verbose Output

```bash
cargo test -p predictify-hybrid resolution_state_property_tests -- --nocapture
```

### Run on Failing Test (for Shrinking)

If a property test fails, proptest will automatically shrink the input to find the minimal failing case:

```bash
cargo test -p predictify-hybrid failing_test_name -- --nocapture
```

## Test Coverage

### Property Tests (12 tests)

1. `winning_outcomes_takes_precedence_over_oracle_result`
2. `oracle_result_takes_precedence_over_dispute_stakes`
3. `dispute_stakes_takes_precedence_over_active`
4. `no_special_fields_yields_active_state`
5. `state_determination_is_deterministic`
6. `market_resolved_implies_winning_outcomes_set`
7. `oracle_resolved_implies_oracle_result_set`
8. `disputed_implies_dispute_stakes_positive`
9. `active_implies_no_special_fields`
10. `multiple_dispute_stakeholders_yields_disputed`
11. `zero_dispute_stakes_does_not_yield_disputed`
12. `resolution_state_invariant_to_market_state`

### Unit Tests (6 tests)

1. `test_resolution_state_priority_order` - Verifies full priority chain
2. `test_resolution_state_with_empty_winning_outcomes` - Edge case: empty vector
3. `test_resolution_state_with_zero_dispute_stake_entry` - Edge case: zero stake entry
4. `test_resolution_state_consistency_across_multiple_calls` - Idempotency
5. `test_resolution_state_with_large_dispute_stakes` - Overflow protection
6. `test_resolution_state_transition_simulation` - Lifecycle simulation

## Expected Output

```
running 18 tests
test resolution_state_property_tests::winning_outcomes_takes_precedence_over_oracle_result ... ok
test resolution_state_property_tests::oracle_result_takes_precedence_over_dispute_stakes ... ok
test resolution_state_property_tests::dispute_stakes_takes_precedence_over_active ... ok
test resolution_state_property_tests::no_special_fields_yields_active_state ... ok
test resolution_state_property_tests::state_determination_is_deterministic ... ok
test resolution_state_property_tests::market_resolved_implies_winning_outcomes_set ... ok
test resolution_state_property_tests::oracle_resolved_implies_oracle_result_set ... ok
test resolution_state_property_tests::disputed_implies_dispute_stakes_positive ... ok
test resolution_state_property_tests::active_implies_no_special_fields ... ok
test resolution_state_property_tests::multiple_dispute_stakeholders_yields_disputed ... ok
test resolution_state_property_tests::zero_dispute_stakes_does_not_yield_disputed ... ok
test resolution_state_property_tests::resolution_state_invariant_to_market_state ... ok
test resolution_state_property_tests::test_resolution_state_priority_order ... ok
test resolution_state_property_tests::test_resolution_state_with_empty_winning_outcomes ... ok
test resolution_state_property_tests::test_resolution_state_with_zero_dispute_stake_entry ... ok
test resolution_state_property_tests::test_resolution_state_consistency_across_multiple_calls ... ok
test resolution_state_property_tests::test_resolution_state_with_large_dispute_stakes ... ok
test resolution_state_property_tests::test_resolution_state_transition_simulation ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured
```

## Integration with CI

These tests are automatically included in:
- `cargo test -p predictify-hybrid`
- CI pipeline test runs
- Pre-deployment validation

## Troubleshooting

### Test Fails with "Input Shrunk to Minimal Case"

If a test fails, proptest will display the minimal failing input. Example:

```
thread 'resolution_state_property_tests::winning_outcomes_takes_precedence_over_oracle_result' panicked at '...
Failing input: (Some("yes"), 50000000)
```

This indicates the specific input that violates the invariant. Use this to:
1. Understand the bug
2. Create a focused unit test
3. Fix the underlying issue

### Test Times Out

If tests are slow, reduce the case count:

```rust
#![proptest_config(ProptestConfig::with_cases(50))]  // Reduce from 100
```

### Compilation Errors

Ensure `proptest` is in `dev-dependencies` in `Cargo.toml`:

```toml
[dev-dependencies]
proptest = "1.0"
```

## Future Enhancements

Potential extensions to the test suite:

1. **State transition validation**: Test legal state transitions
2. **Finalized state testing**: Add tests for `Finalized` state when implemented
3. **Cross-field validation**: Test interactions between resolution state and market state
4. **Performance testing**: Benchmark state determination for large markets
5. **Fuzzing integration**: Integrate with stateful fuzzing for lifecycle testing

## References

- [Proptest Book](https://altsysrq.github.io/proptest-book/intro.html)
- [Stellar Fuzzing Guide](https://developers.stellar.org/docs/build/guides/testing/fuzzing)
- [Property-Based Testing Patterns](https://fsharpforfunandprofit.com/posts/property-based-testing/)
- [Resolution Module Documentation](../src/resolution.rs)

## Acceptance Criteria (GrantFox FWC26)

- ✅ Implementation matches description: Property tests for resolution state invariants
- ✅ Tests added and passing: 18 tests (12 property + 6 unit)
- ✅ Code review ready: Well-documented, follows existing patterns
- ✅ Docs updated: This documentation file added

## Security Considerations

These tests help ensure:
- **No state corruption**: Invariants prevent invalid state combinations
- **Deterministic behavior**: State determination is predictable
- **Edge case handling**: Boundary conditions are correctly handled
- **Overflow protection**: Large stake values don't cause issues
