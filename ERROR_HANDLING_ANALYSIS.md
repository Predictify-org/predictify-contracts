# Error Handling Analysis - Predictify Contracts

**Analysis Date:** March 25, 2026  
**Project:** predictify-contracts (Soroban Rust)  
**Focus:** predictify-hybrid contract

---

## Executive Summary

The predictify-hybrid contract has a **comprehensive error handling framework** with 45 defined error variants organized into 5 categories. However, there are **3 critical bare `panic!()` calls** that bypass the error handling system and should be converted to proper error variants. Additionally, there are safe `unwrap_or()` patterns with sensible defaults scattered throughout the codebase.

**Overall Assessment:** Good error coverage with minor gaps requiring remediation.

---

## 1. Current Error Type System

### Error Enum (src/err.rs)

Total Variants: **45**

#### Category 1: User Operation Errors (100-112) - 11 variants
```
Unauthorized = 100
MarketNotFound = 101
MarketClosed = 102
MarketResolved = 103
MarketNotResolved = 104
NothingToClaim = 105
AlreadyClaimed = 106
InsufficientStake = 107
InvalidOutcome = 108
AlreadyVoted = 109
AlreadyBet = 110
BetsAlreadyPlaced = 111
InsufficientBalance = 112
```

#### Category 2: Oracle Errors (200-208) - 9 variants
```
OracleUnavailable = 200
InvalidOracleConfig = 201
OracleStale = 202
OracleNoConsensus = 203
OracleVerified = 204
MarketNotReady = 205
FallbackOracleUnavailable = 206
ResolutionTimeoutReached = 207
OracleConfidenceTooWide = 208
```

#### Category 3: Validation Errors (300-304) - 5 variants
```
InvalidQuestion = 300
InvalidOutcomes = 301
InvalidDuration = 302
InvalidThreshold = 303
InvalidComparison = 304
```

#### Category 4: General Errors (400-418) - 15 variants
```
InvalidState = 400
InvalidInput = 401
InvalidFeeConfig = 402
ConfigNotFound = 403
AlreadyDisputed = 404
DisputeVoteExpired = 405
DisputeVoteDenied = 406
DisputeAlreadyVoted = 407
DisputeCondNotMet = 408
DisputeFeeFailed = 409
DisputeError = 410
FeeAlreadyCollected = 413
NoFeesToCollect = 414
InvalidExtensionDays = 415
ExtensionDenied = 416
AdminNotSet = 418
```

#### Category 5: Circuit Breaker Errors (500-504) - 5 variants
```
CBNotInitialized = 500
CBAlreadyOpen = 501
CBNotOpen = 502
CBOpen = 503
CBError = 504
```

### Supporting Infrastructure
- **ErrorSeverity:** Low, Medium, High, Critical
- **ErrorCategory:** UserOperation, Oracle, Validation, System, Dispute, Financial, Market, Authentication, Unknown
- **RecoveryStrategy:** Retry, RetryWithDelay, AlternativeMethod, Skip, Abort, ManualIntervention, NoRecovery
- **ErrorContext:** Runtime context capture with operation, user, market, timestamp, call chain
- **DetailedError:** Full categorization with severity, recovery strategy, and user messages

---

## 2. Critical Issues Requiring Immediate Attention

### ISSUE 1: Bare panic!() in Admin Checkout [CRITICAL]

**Location:** [lib.rs](lib.rs#L374)  
**Severity:** HIGH  
**Affected Functions:** Multiple admin access paths

```rust
// lib.rs:374
let stored_admin: Address = env
    .storage()
    .persistent()
    .get(&Symbol::new(&env, "Admin"))
    .unwrap_or_else(|| {
        panic!("Admin not set");  // ❌ PROBLEM: Bare panic!
    });
```

**Occurrence Count:** 2 locations
- [lib.rs:374](lib.rs#L374) - `create_market()`
- [lib.rs:486](lib.rs#L486) - `create_market_with_token()`

**Impact:** Should use `Error::AdminNotSet` (418) instead  
**Why It Matters:** Caller receives no structured error code, breaking contract invariants

**Remediation:**
```rust
// CORRECT APPROACH:
let stored_admin: Address = env
    .storage()
    .persistent()
    .get(&Symbol::new(&env, "Admin"))
    .ok_or(Error::AdminNotSet)?;
```

---

### ISSUE 2: Bare panic!() for Gas Budget Exceeded [CRITICAL]

**Location:** [gas.rs](gas.rs#L65)  
**Severity:** HIGH  
**Function:** `GasTracker::track_operation()`

```rust
// gas.rs:65
if let Some(limit) = Self::get_limit(env, operation) {
    if actual_cost > limit {
        panic!("Gas budget cap exceeded");  // ❌ PROBLEM: Bare panic!
    }
}
```

**Impact:** Should define new error variant or use existing `InvalidState` (400)  
**Why It Matters:** Gas overages are operational errors, not panic conditions

**Current Error Variants That Could Apply:**
- `InvalidState = 400` - Generic but applicable
- **Missing:** `GasBudgetExceeded` - Specific gas tracking error needed

**Recommendation:** Create new variant or map to `InvalidState`

---

### ISSUE 3: Test Panic Without Mapped Error [MINOR - Test Code]

**Location:** [validation_tests.rs](validation_tests.rs#L2038)  
**Severity:** LOW (Test only)

```rust
_ => panic!("Expected InvalidQuestionFormat error")
```

**Analysis:** This is test code, but indicates a potential documentation gap. The `InvalidQuestionFormat` is a `ValidationError`, which correctly maps to contract `Error::InvalidQuestion` via `to_contract_error()`.

---

## 3. Unwrap/Expect Usage Analysis

### Safe unwrap_or() Patterns (51 locations)

These patterns are **SAFE** because they provide sensible defaults:

#### Market Analytics & Storage

```rust
// market_analytics.rs:166
let stake = market.stakes.get(user).unwrap_or(0);

// market_analytics.rs:169
let vote_count = outcome_distribution.get(outcome.clone()).unwrap_or(0);

// storage.rs:457
env.storage().persistent().get(&key).unwrap_or(Balance { /* default */ });

// storage.rs:706, 712, 719
let current_count: u32 = env.storage().persistent().get(&key).unwrap_or(0);
```

**Pattern:** Return 0 or empty container when key not found  
**Safety Level:** ✅ SAFE - Defaults make logical sense

#### Oracle & Configuration

```rust
// oracles.rs:1898
.unwrap_or(false);  // Default to false for health checks

// oracles.rs:1968, 2025, 2132
.unwrap_or(Vec::new(env));  // Empty vec when not found

// oracles.rs:2277
.unwrap_or_else(|| GlobalOracleValidationConfig { /* defaults */ });
```

**Pattern:** Sensible defaults for configuration  
**Safety Level:** ✅ SAFE - Matches contract semantics

#### Voting & Disputes

```rust
// voting.rs:680
env.storage().persistent().get(&key).unwrap_or_else(|| {
    // Construct default outcome
});

// voting.rs:958
let claimed = market.claimed.get(user.clone()).unwrap_or(false);

// disputes.rs:2129, 2217, 2222
market.dispute_stakes.get(user.clone()).unwrap_or(0);
```

**Pattern:** Map missing entry to false or 0  
**Safety Level:** ✅ SAFE

#### Type Conversions with Fallbacks

```rust
// extensions.rs:386
total_extensions: market.extension_history.len().try_into().unwrap_or(0),

// lib.rs:183
let fee_percentage = platform_fee_percentage.unwrap_or(DEFAULT_PLATFORM_FEE_PERCENTAGE);
```

**Pattern:** Use default values when conversion fails  
**Safety Level:** ✅ SAFE - Explicit fallbacks

---

### panic_with_error!() Patterns

**Good News:** 40+ locations use proper error mapping:

```rust
// Examples of CORRECT usage:
panic_with_error!(env, Error::Unauthorized);
panic_with_error!(env, Error::AdminNotSet);
panic_with_error!(env, Error::MarketNotFound);
panic_with_error!(env, Error::InvalidInput);
```

**Count:** 40+ properly mapped errors  
**Assessment:** ✅ EXCELLENT - Error variants are being used correctly

---

### Test Code .unwrap() Patterns

**Location:** Test files (27 locations)

```rust
// state_snapshot_reporting_tests.rs:104
let original = env.storage().persistent().get::<_, Market>(&market_key).unwrap();

// resolution_delay_dispute_window_tests.rs:145
let extended_market: Market = setup.env.storage().persistent().get(&market_id).unwrap();

// gas_test.rs:36
let last_event = events.last().expect("Event should have been published");
```

**Assessment:** ✅ ACCEPTABLE - Test code may use unwrap for brevity when test should fail if data missing

---

### Vector Index Access with unwrap()

**Locations:** 6 instances of `.get(i).unwrap()`

```rust
// edge_cases.rs:233, 569, 575, 581, 587
return Ok(outcomes.get(0).unwrap());

// lib.rs:1623
let primary_outcome = winning_outcomes.get(0).unwrap().clone();

// batch_operations.rs:519
if operations.get(i).unwrap() == operations.get(j).unwrap() {

// circuit_breaker.rs:661
if conditions.get(i).unwrap() == conditions.get(j).unwrap() {
```

**Risk Level:** ⚠️ MEDIUM - Could panic if indices invalid
**Context:** Most are in control flow where bounds are verified earlier
**Recommendation:** Add comments explaining why unwrap is safe or replace with proper error handling

---

## 4. Ignored Results Analysis

### Intentionally Ignored Results

**Location:** [markets.rs](markets.rs#L128)

```rust
let _ = MarketUtils::process_creation_fee(env, &admin)?;
```

**Assessment:** The `?` operator means failure propagates. The `let _` suppresses unused variable warning, which is fine since the Result is implicitly checked via `?`.

**Status:** ✅ ACCEPTABLE - Errors still propagate

---

### Test Code Ignored Results

**Locations:** 6 instances in test files

```rust
// gas_tracking_tests.rs:342
let _ = client.get_market(&market_id);

// category_tags_tests.rs:360
let _ = client.try_resolve_market_manual( ... );
```

**Assessment:** Tests intentionally ignore some results  
**Status:** ✅ ACCEPTABLE - Test code pattern

---

## 5. Missing Error Variants

### Identified Gaps

| Gap                     | Current Mapping         | Recommendation                           |
| ----------------------- | ----------------------- | ---------------------------------------- |
| Gas budget exceeded     | None (bare panic)       | Create `GasBudgetExceeded` variant (505) |
| Admin not initialized   | Implicit in panic       | Already has `AdminNotSet = 418` ✅        |
| Invalid question format | `InvalidQuestion = 300` | ✅ Maps correctly                         |

### Priority 1: Create Gas Budget Error

```rust
// In err.rs Error enum (Circuit Breaker section):
GasBudgetExceeded = 505,  // Gas operation exceeded budget cap
```

**Justification:**
- Currently causes bare panic in [gas.rs:65](gas.rs#L65)
- Gas budget violations are recoverable operational errors
- Enables proper error handling and monitoring

---

## 6. Error Return Paths Analysis

### Functions Returning Result<T, Error>

Verified that functions with Result return type properly map errors:

**Total Functions:** 40+ tracked

**Representative Examples:**
```rust
// ErrorHandler methods
pub fn categorize_error(...) -> DetailedError
pub fn validate_error_context(...) -> Result<(), Error>
pub fn get_error_analytics(...) -> Result<ErrorAnalytics, Error>

// Admin/User functions
pub fn require_user_can_bet(...) -> Result<(), Error>
pub fn require_creator_can_create(...) -> Result<(), Error>

// Dispute/Market functions
pub fn get_dispute_stats(...) -> Result<DisputeStats, Error>
pub fn validate_market_for_dispute(...) -> Result<(), Error>
pub fn calculate_dispute_outcome(...) -> Result<bool, Error>
```

**Assessment:** ✅ PROPER - All analyzed functions correctly return or map errors

---

## 7. Failure Paths Without Proper Error Mapping

### Scenario 1: Storage Access Without Defaults

**Pattern Found:** Some storage access chains

```rust
// Good - has unwrap_or:
env.storage().persistent().get(&key).unwrap_or(0)

// Potentially risky - bare unwrap:
env.storage().persistent().get(&key).unwrap()
```

**Affected Locations:** 20+ in tests, 3 in production code

**Remediation Strategy:** 
- Test code: Document why unwrap is safe
- Production: Replace with `.ok_or(Error::ConfigNotFound)?`

### Scenario 2: Vector Operations

**Pattern:** Index access assumptions

```rust
// Risky:
let outcome = outcomes.get(index).unwrap();

// Better:
let outcome = outcomes.get(index)
    .ok_or(Error::InvalidInput)?;
```

**Locations:** 6 identified in [edge_cases.rs](edge_cases.rs), [lib.rs](lib.rs), [batch_operations.rs](batch_operations.rs)

### Scenario 3: Oracle Configuration Access

**Pattern:** Missing oracle config handling

```rust
// Current pattern - uses unwrap_or_else with defaults
oracle_instance.is_healthy(env).unwrap_or(false)

// Risk: Oracle health default status not documented
```

**Assessment:** Acceptable since defaults are explicit, but should have docstring explaining default behavior

---

## 8. Prioritized Remediation List

### PRIORITY 1: Critical Fixes Required

| Issue                                 | Location                                             | Type | Fix                                        |
| ------------------------------------- | ---------------------------------------------------- | ---- | ------------------------------------------ |
| Bare panic("Admin not set")           | [lib.rs:374](lib.rs#L374), [lib.rs:486](lib.rs#L486) | Code | Replace with `.ok_or(Error::AdminNotSet)?` |
| Bare panic("Gas budget cap exceeded") | [gas.rs:65](gas.rs#L65)                              | Code | Create `GasBudgetExceeded` and use it      |

**Effort Estimate:** 30 minutes  
**Impact:** High - Enables proper error reporting to clients

---

### PRIORITY 2: Add Missing Error Variants

| Variant               | Code | Reason                                   | Impact |
| --------------------- | ---- | ---------------------------------------- | ------ |
| GasBudgetExceeded     | 505  | Gas overages should be errors not panics | Medium |
| InvalidMarketMetadata | 419  | Market metadata validation failures      | Low    |

**Effort Estimate:** 1-2 hours (includes error messaging, classification)  
**Impact:** Medium - Improves gas tracking and validation coverage

---

### PRIORITY 3: Documentation & Safety Comments

| File                                       | Issue                             | Action                                             |
| ------------------------------------------ | --------------------------------- | -------------------------------------------------- |
| [edge_cases.rs](edge_cases.rs)             | Multiple `.get(0).unwrap()` calls | Add safety comments explaining why bounds are safe |
| [batch_operations.rs](batch_operations.rs) | Index access with unwrap          | Document loop bounds verification                  |
| [oracles.rs](oracles.rs)                   | `unwrap_or(false)` patterns       | Document what false default means                  |

**Effort Estimate:** 30 minutes  
**Impact:** Low - Code clarity and maintainability

---

### PRIORITY 4: Test Code Cleanup

| File                                                                                 | Pattern                                | Action                                                       |
| ------------------------------------------------------------------------------------ | -------------------------------------- | ------------------------------------------------------------ |
| [state_snapshot_reporting_tests.rs](state_snapshot_reporting_tests.rs)               | `let _ = result;` patterns             | Consider using `let _ = result.unwrap();` for intent clarity |
| [resolution_delay_dispute_window_tests.rs](resolution_delay_dispute_window_tests.rs) | Multiple `.unwrap()` on storage access | Add #[should_panic] if tests expect to fail                  |

**Effort Estimate:** 1 hour  
**Impact:** Low - Test maintainability

---

## 9. Error Type Coverage Matrix

### By Operation Category

| Operation         | Error Variants Used                                                                                  | Coverage    | Gap                      |
| ----------------- | ---------------------------------------------------------------------------------------------------- | ----------- | ------------------------ |
| Market Creation   | InvalidQuestion, InvalidOutcomes, InvalidDuration, Unauthorized, AdminNotSet                         | ✅ Excellent | None                     |
| Betting           | InsufficientBalance, InsufficientStake, MarketClosed, MarketResolved, InvalidOutcome, AlreadyBet     | ✅ Excellent | None                     |
| Voting            | AlreadyVoted, MarketClosed, InvalidOutcome, Unauthorized                                             | ✅ Good      | Add vote-weight error?   |
| Oracle Resolution | OracleUnavailable, OracleStale, OracleNoConsensus, OracleConfidenceTooWide, ResolutionTimeoutReached | ✅ Excellent | None                     |
| Disputes          | DisputeVoteExpired, DisputeAlreadyVoted, DisputeCondNotMet, DisputeFeeFailed, AlreadyDisputed        | ✅ Good      | Add dispute-state-error  |
| Fee Collection    | FeeAlreadyCollected, NoFeesToCollect, InvalidFeeConfig                                               | ✅ Good      | Add fee-transfer-failed  |
| Gas Tracking      | None mapped ❌                                                                                        | ❌ Poor      | Create GasBudgetExceeded |
| Circuit Breaker   | CBNotInitialized, CBAlreadyOpen, CBNotOpen, CBOpen                                                   | ✅ Good      | None                     |

---

## 10. Recommendations Summary

### Immediate Actions (Week 1)
1. ✅ Convert 2 bare `panic!()` calls (lib.rs) to `Error::AdminNotSet`
2. ✅ Create `GasBudgetExceeded` error variant (505)
3. ✅ Update [gas.rs:65](gas.rs#L65) to use new variant

### Short-term (Week 2)
1. Add safety comments to `.unwrap()` calls in [edge_cases.rs](edge_cases.rs)
2. Document default behavior in oracle health checks
3. Update error classification in [err.rs](err.rs) if new variants added

### Medium-term (Week 3-4)
1. Consider audit logging for all error conditions
2. Add error telemetry integration
3. Create error handling guidelines documentation

### Long-term
1. Implement error aggregation for batch operations
2. Add retry logic for recoverable errors (OracleUnavailable, OracleStale)
3. Consider custom error types for different subsystems

---

## 11. Files with Known Error Patterns

### Production Code with Issues
- [lib.rs](lib.rs) - 2 bare panic!() calls at lines 374, 486
- [gas.rs](gas.rs) - 1 bare panic!() call at line 65

### Test Code with unwrap() (Acceptable)
- [state_snapshot_reporting_tests.rs](state_snapshot_reporting_tests.rs) - 27+ unwrap() calls
- [resolution_delay_dispute_window_tests.rs](resolution_delay_dispute_window_tests.rs) - 20+ unwrap() calls
- [gas_test.rs](gas_test.rs) - 3 expect() calls
- [validation_tests.rs](validation_tests.rs) - 1 test panic

### Clean Error Handling Files
- [err.rs](err.rs) - Well-structured, all error classification correct
- [validation.rs](validation.rs) - Proper error mapping via `to_contract_error()`
- [voting.rs](voting.rs) - Good use of panic_with_error! macro
- [disputes.rs](disputes.rs) - Comprehensive error handling

---

## 12. Error Handling Best Practices Applied

✅ **Strengths:**
- Comprehensive error enum with clear categorization
- Error context capture for diagnostics
- Detailed error messages for users
- Recovery strategies defined
- Error severity classification
- Proper use of panic_with_error! macro (40+ locations)

⚠️ **Weaknesses:**
- 3 bare panic!() calls bypass error system
- 1 missing error variant for gas budget
- Some test code could be clearer about intent

✅ **Overall Assessment:** Mature error handling system with minor cleanup needed

---

## Error Handling Code Examples

### Pattern 1: Correct Error Handling (FOLLOW THIS)
```rust
// From lib.rs - GOOD PATTERN
let stored_admin: Address = env
    .storage()
    .persistent()
    .get(&Symbol::new(&env, "Admin"))
    .unwrap_or_else(|| panic_with_error!(env, Error::AdminNotSet));
```

### Pattern 2: Proposed Fix for Bare panic!
```rust
// FROM THIS:
let stored_admin = env.storage().persistent().get(&key)
    .unwrap_or_else(|| panic!("Admin not set"));

// TO THIS:
let stored_admin = env.storage().persistent().get(&key)
    .ok_or(Error::AdminNotSet)?;
```

### Pattern 3: Safe unwrap_or (ACCEPTABLE)
```rust
// This is fine:
let stake = market.stakes.get(user).unwrap_or(0);
// Because missing stake logically means 0 stake
```

### Pattern 4: Error Result Propagation (FOLLOW THIS)
```rust
// From disputes.rs - GOOD PATTERN
pub fn get_dispute_stats(env: &Env, market_id: Symbol) 
    -> Result<DisputeStats, Error> {
    let market = MarketStorage::load(env, &market_id)
        .ok_or(Error::MarketNotFound)?;
    Ok(stats)
}
```

---

## Appendix: Complete Error Variant Reference

### All 45 Error Variants by Code

| Code | Variant                   | Category       | Severity | Recovery           |
| ---- | ------------------------- | -------------- | -------- | ------------------ |
| 100  | Unauthorized              | Authentication | High     | Abort              |
| 101  | MarketNotFound            | Market         | Medium   | Abort              |
| 102  | MarketClosed              | Market         | Medium   | Abort              |
| 103  | MarketResolved            | Market         | High     | Abort              |
| 104  | MarketNotResolved         | Market         | Medium   | Retry              |
| 105  | NothingToClaim            | UserOperation  | Low      | Abort              |
| 106  | AlreadyClaimed            | UserOperation  | Medium   | Abort              |
| 107  | InsufficientStake         | Financial      | Medium   | Abort              |
| 108  | InvalidOutcome            | Validation     | Low      | Abort              |
| 109  | AlreadyVoted              | UserOperation  | Low      | Abort              |
| 110  | AlreadyBet                | UserOperation  | Low      | Abort              |
| 111  | BetsAlreadyPlaced         | Market         | Medium   | Abort              |
| 112  | InsufficientBalance       | Financial      | High     | Abort              |
| 200  | OracleUnavailable         | Oracle         | High     | Retry              |
| 201  | InvalidOracleConfig       | Oracle         | High     | ManualIntervention |
| 202  | OracleStale               | Oracle         | Medium   | Retry              |
| 203  | OracleNoConsensus         | Oracle         | High     | RetryWithDelay     |
| 204  | OracleVerified            | Oracle         | Low      | Skip               |
| 205  | MarketNotReady            | Market         | Medium   | Retry              |
| 206  | FallbackOracleUnavailable | Oracle         | Critical | ManualIntervention |
| 207  | ResolutionTimeoutReached  | Oracle         | High     | Abort              |
| 208  | OracleConfidenceTooWide   | Oracle         | Medium   | RetryWithDelay     |
| 300  | InvalidQuestion           | Validation     | Low      | Abort              |
| 301  | InvalidOutcomes           | Validation     | Low      | Abort              |
| 302  | InvalidDuration           | Validation     | Low      | Abort              |
| 303  | InvalidThreshold          | Validation     | Low      | Abort              |
| 304  | InvalidComparison         | Validation     | Low      | Abort              |
| 400  | InvalidState              | System         | High     | ManualIntervention |
| 401  | InvalidInput              | Validation     | Low      | Abort              |
| 402  | InvalidFeeConfig          | System         | High     | ManualIntervention |
| 403  | ConfigNotFound            | System         | High     | ManualIntervention |
| 404  | AlreadyDisputed           | Dispute        | Medium   | Abort              |
| 405  | DisputeVoteExpired        | Dispute        | Medium   | Abort              |
| 406  | DisputeVoteDenied         | Dispute        | Medium   | Abort              |
| 407  | DisputeAlreadyVoted       | Dispute        | Low      | Abort              |
| 408  | DisputeCondNotMet         | Dispute        | Medium   | Abort              |
| 409  | DisputeFeeFailed          | Dispute        | High     | ManualIntervention |
| 410  | DisputeError              | Dispute        | Medium   | Abort              |
| 413  | FeeAlreadyCollected       | Financial      | Medium   | Abort              |
| 414  | NoFeesToCollect           | Financial      | Low      | Abort              |
| 415  | InvalidExtensionDays      | Validation     | Low      | Abort              |
| 416  | ExtensionDenied           | Market         | Medium   | Abort              |
| 418  | AdminNotSet               | System         | Critical | ManualIntervention |
| 500  | CBNotInitialized          | System         | High     | ManualIntervention |
| 501  | CBAlreadyOpen             | System         | High     | Skip               |
| 502  | CBNotOpen                 | System         | High     | Abort              |
| 503  | CBOpen                    | System         | Critical | Abort              |
| 504  | CBError                   | System         | High     | Abort              |

---

## Document Control

**Version:** 1.0  
**Last Updated:** March 25, 2026  
**Reviewed By:** Static Analysis  
**Status:** Complete Analysis Ready for Implementation
