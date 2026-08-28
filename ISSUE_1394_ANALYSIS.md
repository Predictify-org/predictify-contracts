# Issue #1394: Bound Oracle Deviation and Fallback Semantics

## Executive Summary

This issue requires implementing **deterministic oracle deviation bounds** and **graceful fallback semantics** to ensure prediction market resolutions are protected against anomalous price movements and oracle failures. The implementation must preserve existing interfaces, enforce invariants, and be fully tested across success, boundary, and failure scenarios.

## Current State Analysis

### Existing Components

**Oracle Resolution (resolution.rs)**
- `fetch_oracle_result()`: Attempts primary oracle, then fallback oracle if primary fails
- Fallback logic: triggered only on primary oracle *failure*, not on deviation
- No deviation bound checking between primary and fallback
- Single oracle call per config (no retries on deviation)

**Oracle Types (types.rs)**
- `OracleConfig`: Defines provider, feed_id, threshold, comparison operator
- Supports Reflector, Pyth, Band Protocol providers
- Sentinel pattern: `none_sentinel()` for "no fallback" encoding
- No deviation bound fields in config

**Validation (validation.rs)**
- `OracleValidator::validate_oracle_config()`: Validates provider/feed/threshold/comparison
- No deviation bound validation
- No comparison logic between price sources

**Fallback Mechanism (resolution.rs, integration_test.rs)**
- Current fallback: primary → fallback (on primary *error*)
- Outcome reconciliation: `OracleUtils::resolve_outcome_with_fallback()`
- Events: `FallbackUsedEvent`, `ManualResolutionRequiredEvent`
- No deviation-based triggering

**Error Handling (err.rs)**
- `OracleUnavailable` (code 200)
- `FallbackOracleUnavailable` (code 206)
- No `OracleDeviationExceeded` error currently

### Gap Analysis

1. **No Deviation Bound Concept**: Oracle configs don't define maximum allowed deviation between primary and fallback
2. **No Deviation Checking**: Resolution logic doesn't compare prices from primary vs fallback
3. **Limited Fallback Triggering**: Fallback only used on primary oracle *error*, not on anomalies
4. **No Deterministic Price Agreement**: No logic to ensure price agreement between oracles
5. **No Failure Mode Separation**: Errors don't distinguish "oracle down" from "oracle anomaly"

## Proposed Solution

### 1. Core Data Structures

**New: `DeviationBounds` in types.rs**
```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviationBounds {
    /// Maximum allowed deviation as percentage (basis points: 0-10000 = 0-100%)
    /// Example: 500 = 5% maximum deviation
    pub max_deviation_bps: u32,
    
    /// When true: deviation triggers mandatory fallback
    /// When false: deviation is logged but primary result is used
    pub enforce_fallback_on_deviation: bool,
}
```

**Update: `OracleConfig` in types.rs**
```rust
pub struct OracleConfig {
    pub provider: OracleProvider,
    pub oracle_address: Address,
    pub feed_id: String,
    pub threshold: i128,
    pub comparison: String,
    // NEW: Deviation bounds between primary and fallback
    pub deviation_bounds: Option<DeviationBounds>,
}
```

### 2. Validation & Invariants

**New: `DeviationValidator` in validation.rs**
```rust
impl DeviationValidator {
    /// Validate deviation bounds structure
    pub fn validate_bounds(bounds: &DeviationBounds) -> Result<(), Error> {
        // max_deviation_bps must be 0-10000 (0-100%)
        if bounds.max_deviation_bps > 10000 {
            return Err(Error::InvalidDeviationBounds);
        }
        Ok(())
    }
    
    /// Check if price deviation exceeds bounds
    pub fn check_deviation(
        primary_price: i128,
        fallback_price: i128,
        bounds: &DeviationBounds,
    ) -> Result<bool, Error> {
        if primary_price <= 0 || fallback_price <= 0 {
            return Err(Error::InvalidOraclePrice);
        }
        
        // Calculate deviation as percentage (in basis points)
        let deviation_bps = Self::calculate_deviation_bps(primary_price, fallback_price);
        Ok(deviation_bps > bounds.max_deviation_bps)
    }
    
    /// Calculate deviation in basis points (0-10000)
    fn calculate_deviation_bps(price1: i128, price2: i128) -> u32 {
        let (larger, smaller) = if price1 > price2 {
            (price1, price2)
        } else {
            (price2, price1)
        };
        
        // Avoid division by zero
        if smaller == 0 {
            return 10000; // 100% deviation
        }
        
        // deviation = (larger - smaller) / smaller * 10000
        let diff = (larger - smaller).abs();
        let percentage = ((diff as u128 * 10000) / smaller as u128) as u32;
        
        // Cap at 10000 (100%)
        percentage.min(10000)
    }
}
```

### 3. Resolution Logic Changes

**Updated: `fetch_oracle_result()` in resolution.rs**

The new flow:
1. Attempt primary oracle
2. On primary success with fallback configured:
   - Attempt fallback oracle
   - Calculate deviation between prices
   - If deviation exceeds bounds AND `enforce_fallback_on_deviation` is true:
     - Use fallback result + emit `DeviationDetectedEvent`
   - Otherwise:
     - Use primary result
3. On primary failure with fallback configured:
   - Attempt fallback oracle (existing behavior)
4. Emit appropriate events for diagnostics

### 4. New Error Types (err.rs)

```rust
pub enum Error {
    // Existing...
    OracleUnavailable = 200,
    FallbackOracleUnavailable = 206,
    
    // NEW:
    OracleDeviationExceeded = 207,      // Deviation bounds exceeded
    InvalidDeviationBounds = 208,       // Invalid deviation configuration
    InvalidOraclePrice = 209,           // Price validation failed
}
```

### 5. Events (events.rs)

```rust
#[derive(Clone, Debug)]
pub struct DeviationDetectedEvent {
    pub market_id: Symbol,
    pub primary_oracle: Address,
    pub fallback_oracle: Address,
    pub primary_price: i128,
    pub fallback_price: i128,
    pub max_deviation_bps: u32,
    pub actual_deviation_bps: u32,
    pub resolution_outcome: String,  // Which oracle result was used
}
```

### 6. State Invariants

1. **Price Validity**: Both primary and fallback prices must be positive (>0)
2. **Deviation Bounds**: `max_deviation_bps` must be 0-10000
3. **Deterministic Comparison**: Given identical inputs, price comparison is deterministic
4. **Fallback Ordering**: Fallback is only consulted after primary is resolved
5. **No Retry on Deviation**: Single attempt per oracle; deviations don't trigger retries
6. **Outcome Consistency**: Outcome determination uses same logic regardless of deviation
7. **Error Separation**: Errors distinguish: oracle-down vs. deviation-exceeded vs. validation-failed

### 7. Test Strategy

#### Success Path Tests
- Primary oracle succeeds, no fallback configured → use primary result
- Primary oracle succeeds, fallback configured, deviation within bounds → use primary
- Primary oracle succeeds, fallback configured, deviation exceeds bounds, enforce=true → use fallback
- Primary oracle succeeds, fallback configured, deviation exceeds bounds, enforce=false → use primary
- Primary oracle fails, fallback succeeds → use fallback (existing behavior)

#### Boundary Tests
- Deviation exactly at bound: `actual_deviation_bps == max_deviation_bps` → within bounds
- Deviation 1 BPS above bound → exceeds bounds
- Price = 1 (minimum positive) vs. price = max_i128
- Zero price handling (invalid)
- Negative price handling (invalid)

#### Invalid Input Tests
- Invalid deviation bounds (>10000)
- Negative prices
- Zero prices
- Invalid comparison operators
- Empty feed IDs

#### Retry & Concurrency Tests
- Single attempt per oracle (no retries on deviation)
- Fallback only called once after primary fails
- Concurrent markets don't interfere (market state isolated)

#### Failure Recovery Tests
- Primary fails, fallback succeeds → correct fallback result
- Both fail → appropriate error code
- Deviation bounds exceeded → event emitted with metrics
- Timeout reached → ResolutionTimeoutReached (not overridden)

## Compatibility & Migration

### Public Interface Changes
- **OracleConfig**: Adds optional `deviation_bounds` field
  - Existing configs without bounds → backward compatible (None)
  - New configs with bounds → enforced when set
- **fetch_oracle_result()**: Return type unchanged
  - New error: `OracleDeviationExceeded` (added to Error enum)
  - Behavior: seamless fallback on deviation (when configured)

### Storage Layout
- New field in OracleConfig adds minimal storage overhead
- Existing markets remain fully functional (None bounds = no deviation checking)
- No migration required for existing data

### Event Changes
- New event: `DeviationDetectedEvent`
- Existing events unchanged
- Callers unaffected (additive change)

## Implementation Roadmap

1. **Add types** → types.rs: DeviationBounds, update OracleConfig
2. **Add validation** → validation.rs: DeviationValidator
3. **Add errors** → err.rs: New error codes
4. **Add events** → events.rs: DeviationDetectedEvent
5. **Update resolution** → resolution.rs: Implement deviation checking in fetch_oracle_result()
6. **Add tests** → tests/: Comprehensive test suite (>20 test cases)
7. **Verify CI** → Run existing tests, check WASM size, ensure backward compat

## Success Criteria

✅ Deterministic behavior for valid, invalid, duplicate, and boundary-case inputs
✅ Authorization, validation, and state-transition invariants enforced
✅ Retries, partial failure, and concurrent execution safe (no corruption)
✅ Focused tests cover success, rejection, boundary, and regression scenarios
✅ Existing callers compatible (no breaking changes required)
✅ Logs/metrics make failures diagnosable without exposing secrets
✅ CI passes, WASM size stays within budget
✅ Code documented with invariants and failure modes clearly explained

## Non-Goals

- Typo/formatting/documentation-only changes (this is implementation)
- Unrelated refactors or dependency upgrades
- Weakening validation to make tests pass
- Removing safeguards for edge cases
