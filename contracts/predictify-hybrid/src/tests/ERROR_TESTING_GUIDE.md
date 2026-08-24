# Error Testing Extensions

This document describes the comprehensive extensions made to the error testing infrastructure for the Predictify Hybrid contract.

## Overview

The error testing suite has been significantly expanded with:
- **476 new test lines** in `error_code_tests.rs`
- **346 line test utility module** (`tests/common.rs`)
- **313 line scenario test module** (`tests/error_scenarios.rs`)
- **Module organization** with `tests/mod.rs`

## New Test Categories

### 1. Comprehensive Error Classification Tests
Located in `error_code_tests.rs` (lines 1310+)

Tests that verify error classification for severity, category, and recovery strategy:
- `test_error_classification_user_operation_errors()` - Validates user operation error properties
- `test_error_classification_oracle_errors()` - Validates oracle error properties
- `test_error_classification_validation_errors()` - Validates validation error properties
- `test_error_classification_severity_levels()` - Tests severity level assignments

**Purpose**: Ensures each error is correctly classified for proper handling and recovery.

### 2. Error Recovery Lifecycle Tests
Located in `error_code_tests.rs` (lines 1400+)

Tests covering the complete recovery process:
- `test_error_recovery_full_lifecycle()` - Full recovery flow from error to resolution
- `test_error_recovery_attempts_tracking()` - Validates retry attempt counting
- `test_error_recovery_context_validation()` - Tests context validation during recovery
- `test_error_recovery_status_aggregation()` - Tests recovery statistics collection

**Purpose**: Validates that error recovery mechanisms work end-to-end.

### 3. Error Message Generation Tests
Located in `error_code_tests.rs` (lines 1460+)

Tests for user-facing error messages:
- `test_error_message_generation_all_errors()` - All errors produce messages
- `test_error_message_context_aware()` - Messages are context-sensitive

**Purpose**: Ensures users receive helpful, actionable error messages.

### 4. Error Analytics Tests
Located in `error_code_tests.rs` (lines 1490+)

Tests for error tracking and analytics:
- `test_error_analytics_structure()` - Analytics data structure is valid
- `test_error_recovery_procedures_documented()` - Recovery procedures are documented

**Purpose**: Validates error monitoring and diagnostics infrastructure.

### 5. Error Recovery Strategy Mapping Tests
Located in `error_code_tests.rs` (lines 1515+)

Tests that validate recovery strategy selection:
- `test_recovery_strategy_mapping_retryable_errors()` - Retryable errors map correctly
- `test_recovery_strategy_mapping_skip_errors()` - Skippable errors map correctly
- `test_recovery_strategy_mapping_abort_errors()` - Abort errors map correctly

**Purpose**: Ensures each error type is assigned the appropriate recovery strategy.

### 6. Error Code Uniqueness Tests
Located in `error_code_tests.rs` (lines 1543+)

Tests for error code consistency:
- `test_all_error_codes_are_unique()` - All error code strings and numerics are unique
- Tests include 47+ error variants for exhaustive coverage

**Purpose**: Prevents duplicate error codes which would break client error handling.

### 7. Error Description Consistency Tests
Located in `error_code_tests.rs` (lines 1636+)

Tests verifying error descriptions:
- `test_all_error_descriptions_consistent()` - All descriptions are non-empty and clear

**Purpose**: Ensures users always get helpful descriptions.

### 8. Error Context Edge Cases
Located in `error_code_tests.rs` (lines 1656+)

Tests for boundary conditions:
- `test_error_context_with_future_timestamp()` - Rejects future timestamps
- `test_error_recovery_exceeding_max_attempts()` - Rejects excessive retry attempts

**Purpose**: Ensures robustness against edge cases and malformed inputs.

## Test Utilities Module (`tests/common.rs`)

Provides reusable test helpers to reduce duplication:

### ErrorContextBuilder
Fluent builder for constructing `ErrorContext` objects:
```rust
let context = ErrorContextBuilder::new(&env, "place_bet")
    .user_address(Some(user_addr))
    .market_id(Some(market_id))
    .with_data(&env, "key", "value")
    .build();
```

### ErrorTestScenarios
Pre-built contexts for common error scenarios:
- `market_creation_context()` - Market creation failure context
- `bet_placement_context()` - Bet placement failure context
- `oracle_resolution_context()` - Oracle resolution failure context
- `balance_check_context()` - Balance check failure context
- `withdrawal_context()` - Withdrawal failure context

### ErrorAssertions
Common assertions for error testing:
- `assert_error_in_range()` - Validates error code is in expected range
- `assert_error_code_format()` - Validates error code format (UPPER_SNAKE_CASE)
- `assert_all_descriptions_non_empty()` - Batch validation of descriptions
- `assert_error_codes_unique()` - Checks for duplicate codes

### ErrorTestSuite
Provides error groupings for batch testing:
- `user_operation_errors()` - 100-112 range
- `oracle_errors()` - 200-208 range
- `validation_errors()` - 300-304 range
- `system_errors()` - 400-418 range
- `circuit_breaker_errors()` - 500-504 range
- `all_errors()` - Complete set for exhaustive testing

## Error Recovery Scenarios (`tests/error_scenarios.rs`)

Demonstrates real-world error handling patterns:

### Oracle Failure Scenarios
- `scenario_oracle_unavailable_with_retry()` - Retry logic for unavailable oracles
- `scenario_oracle_stale_data()` - Handling stale oracle data

### User Operation Scenarios
- `scenario_user_already_voted()` - Duplicate vote handling
- `scenario_insufficient_balance_recovery()` - Balance shortage recovery
- `scenario_invalid_market_duration()` - Invalid duration guidance
- `scenario_invalid_market_outcomes()` - Invalid outcome guidance

### Authorization Scenarios
- `scenario_unauthorized_cannot_retry()` - Unauthorized access is final

### System State Scenarios
- `scenario_admin_not_initialized()` - Initialization failures need manual intervention
- `scenario_invalid_contract_state()` - Invalid state cannot auto-recover

### Complex Scenarios
- `scenario_cascading_validation_errors()` - Multiple validation errors in sequence
- `scenario_dispute_resolution_failure()` - Dispute fee distribution failures
- `scenario_check_system_recovery_health()` - Query recovery statistics
- `scenario_analyze_error_distribution()` - Analyze error patterns
- `scenario_oracle_resolution_with_timeout()` - Time-limited resolution
- `scenario_resolution_timeout_exceeded()` - Expired deadline handling

## Test Coverage Statistics

### Error Code Tests
- **Total lines**: 1,781 (increased from 1,305)
- **New test functions**: 15+
- **Error variants tested**: 47+
- **Coverage areas**:
  - Error numeric codes
  - Error string codes
  - Error descriptions
  - Error classifications (severity, category, recovery)
  - Recovery strategies
  - Recovery lifecycle
  - Analytics and status
  - Edge cases and bounds

### Test Utilities
- **Classes/Structs**: 4 major (ErrorContextBuilder, ErrorTestScenarios, ErrorAssertions, ErrorTestSuite)
- **Methods**: 20+
- **Self-contained tests**: 5+

### Error Scenarios
- **Scenario tests**: 20+
- **Real-world patterns**: Oracle failures, user operations, authorization, system state
- **Timeout/timing**: Deadline and timeout handling
- **Complex cases**: Cascading errors, dispute resolution, analytics

## Usage Examples

### Running All Error Tests
```bash
cd /home/skorggg/predictify-contracts
cargo test error_code_tests --lib
cargo test error_scenarios --lib
```

### Running Specific Test Categories
```bash
# Classification tests
cargo test error_classification --lib

# Recovery tests
cargo test error_recovery --lib

# Scenario tests
cargo test scenario_ --lib
```

### Using Test Utilities in New Tests
```rust
#[test]
fn my_error_test() {
    use crate::tests::common::{ErrorContextBuilder, ErrorTestScenarios, ErrorAssertions};
    
    let env = Env::default();
    let context = ErrorContextBuilder::new(&env, "my_operation")
        .user_address(Some(Address::generate(&env)))
        .build();
    
    let recovery = ErrorHandler::recover_from_error(&env, Error::SomeError, context);
    assert!(recovery.is_ok());
}
```

## Error Code Ranges

The error codes are organized by category:
- **100-112**: User Operation Errors
- **200-208**: Oracle Errors (+ 208 for confidence)
- **300-304**: Validation Errors
- **400-418**: System/General Errors (including dispute errors)
- **500-504**: Circuit Breaker Errors

Each range is tested exhaustively for numeric uniqueness, string consistency, and semantic appropriateness.

## Best Practices for Error Testing

1. **Use Test Utilities**: Leverage `ErrorContextBuilder` and `ErrorTestScenarios` to reduce boilerplate
2. **Test Recovery**: Ensure errors have appropriate recovery strategies
3. **Validate Messages**: Error messages should guide users to resolution
4. **Check Categories**: Ensure error classification matches intended severity
5. **Test Edge Cases**: Include boundary conditions and malformed inputs
6. **Document Intent**: Scenario tests show developers how errors are handled

## Future Enhancements

Potential areas for further extension:
- Integration tests combining multiple error conditions
- Performance benchmarks for error handling
- Fuzz testing for invalid error construction
- Analytics visualization for error patterns
- Client library error mapping tests
