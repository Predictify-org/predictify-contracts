# Predictify Hybrid Error Management System

## Overview

The Predictify Hybrid contract now features a comprehensive, centralized error management system designed to provide better organization, debugging capabilities, and maintainability. This document outlines the architecture, usage patterns, and best practices for working with the error system.

## Architecture

### Error Categories

Errors are organized into logical categories for better understanding and handling:

1. **Security Errors (1-10)**
   - Authentication and authorization failures
   - Unauthorized access attempts

2. **Market Errors (11-30)**
   - Market state and operation issues
   - Market lifecycle management problems

3. **Oracle Errors (31-50)**
   - Oracle integration failures
   - Data availability and validation issues

4. **Validation Errors (51-70)**
   - Input validation failures
   - Business logic validation errors

5. **State Errors (71-90)**
   - Contract state inconsistencies
   - User action conflicts

6. **System Errors (91-100)**
   - Internal contract errors
   - Storage and arithmetic failures

### Core Components

#### 1. Error Enum (`Error`)
The main error type with comprehensive categorization and documentation.

```rust
#[contracterror]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    // Security errors
    Unauthorized = 1,
    
    // Market errors
    MarketClosed = 2,
    MarketAlreadyResolved = 5,
    // ... more errors
}
```

#### 2. Error Categories (`ErrorCategory`)
Logical grouping of errors for better organization and handling.

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrorCategory {
    Security,
    Market,
    Oracle,
    Validation,
    State,
    System,
}
```

#### 3. Error Context (`ErrorContext`)
Additional debugging information for errors.

```rust
#[derive(Clone, Debug)]
pub struct ErrorContext {
    pub operation: String,
    pub details: String,
    pub timestamp: u64,
}
```

## Usage Patterns

### 1. Basic Error Handling

```rust
use errors::Error;

// Direct error usage
if condition {
    panic_with_error!(env, Error::Unauthorized);
}
```

### 2. Error Helper Functions

The system provides helper functions for common validation scenarios:

```rust
use errors::helpers;

// Admin validation
helpers::require_admin(&env, &caller, &admin);

// Market state validation
helpers::require_market_open(&env, &market);

// Input validation
helpers::require_valid_outcome(&env, &outcome, &outcomes);
helpers::require_sufficient_stake(&env, stake, min_stake);
```

### 3. Error Context and Debugging

```rust
use errors::{ErrorContext, debug};

// Create error context
let context = ErrorContext::new(&env, "vote", "User attempted to vote on closed market");

// Log error with context
debug::log_error(&env, Error::MarketClosed, &context);

// Create detailed error report
let report = debug::create_error_report(&env, Error::MarketClosed, &context);
```

### 4. Error Conversion

```rust
use errors::conversions::IntoPredictifyError;

// Convert external results to Predictify errors
let result: Result<T, ExternalError> = external_call();
result.into_predictify_error(&env, Error::InternalError)?;
```

## Error Helper Functions

### Security Helpers

- `require_admin(env, caller, admin)` - Validates admin permissions
- `require_authorized(env, caller, authorized_users)` - Validates user authorization

### Market Helpers

- `require_market_open(env, market)` - Ensures market is active
- `require_market_resolved(env, market)` - Ensures market is resolved
- `require_market_exists(env, market)` - Ensures market exists

### Validation Helpers

- `require_valid_outcome(env, outcome, outcomes)` - Validates outcome choice
- `require_sufficient_stake(env, stake, min_stake)` - Validates stake amount
- `require_valid_market_params(env, question, outcomes, duration)` - Validates market parameters
- `require_valid_oracle_config(env, config)` - Validates oracle configuration

### State Helpers

- `require_not_claimed(env, claimed)` - Ensures user hasn't claimed
- `require_not_voted(env, voted)` - Ensures user hasn't voted
- `require_not_staked(env, staked)` - Ensures user hasn't staked

## Error Properties and Methods

### Category Information

```rust
let error = Error::Unauthorized;
assert_eq!(error.category(), ErrorCategory::Security);
```

### Human-Readable Messages

```rust
let error = Error::MarketClosed;
assert_eq!(error.message(), "Market is closed and no longer accepting votes or stakes");
```

### Error Codes

```rust
let error = Error::OracleUnavailable;
assert_eq!(error.code(), "ORACLE_UNAVAILABLE");
```

### Recoverability

```rust
let error = Error::InvalidOutcome;
assert!(error.is_recoverable()); // Validation errors are recoverable

let error = Error::Unauthorized;
assert!(!error.is_recoverable()); // Security errors are not recoverable
```

### Criticality

```rust
let error = Error::InternalError;
assert!(error.is_critical()); // System errors are critical

let error = Error::InvalidOutcome;
assert!(!error.is_critical()); // Validation errors are not critical
```

## Best Practices

### 1. Use Helper Functions

Instead of manual validation, use the provided helper functions:

```rust
// ❌ Manual validation
if caller != admin {
    panic_with_error!(env, Error::Unauthorized);
}

// ✅ Using helper function
helpers::require_admin(&env, &caller, &admin);
```

### 2. Provide Context for Debugging

Always provide meaningful context when logging errors:

```rust
let context = ErrorContext::new(
    &env,
    "create_market",
    &format!("Admin: {}, Question: {}", admin, question)
);
debug::log_error(&env, error, &context);
```

### 3. Handle Errors Appropriately

- **Security Errors**: Always critical, should halt execution
- **Validation Errors**: Usually recoverable, provide clear feedback
- **State Errors**: May be recoverable depending on context
- **System Errors**: Critical, should be logged and handled gracefully

### 4. Use Error Categories for Logic

```rust
match error.category() {
    ErrorCategory::Security => {
        // Handle security errors
        log_security_violation(error);
    }
    ErrorCategory::Validation => {
        // Handle validation errors
        provide_user_feedback(error);
    }
    ErrorCategory::System => {
        // Handle system errors
        log_system_error(error);
    }
    _ => {
        // Handle other errors
        handle_generic_error(error);
    }
}
```

## Testing

The error system includes comprehensive tests:

```rust
#[test]
fn test_error_categories() {
    assert_eq!(Error::Unauthorized.category(), ErrorCategory::Security);
    assert_eq!(Error::MarketClosed.category(), ErrorCategory::Market);
}

#[test]
fn test_error_messages() {
    assert_eq!(Error::Unauthorized.message(), "Unauthorized access - caller lacks required permissions");
}

#[test]
fn test_error_recoverability() {
    assert!(!Error::Unauthorized.is_recoverable());
    assert!(Error::InvalidOutcome.is_recoverable());
}
```

## Migration Guide

### From Old Error System

1. **Replace direct error usage**:
   ```rust
   // Old
   panic_with_error!(env, Error::Unauthorized);
   
   // New (same, but with better organization)
   panic_with_error!(env, Error::Unauthorized);
   ```

2. **Use helper functions**:
   ```rust
   // Old
   if admin != stored_admin {
       panic_with_error!(env, Error::Unauthorized);
   }
   
   // New
   helpers::require_admin(&env, &admin, &stored_admin);
   ```

3. **Add error context for debugging**:
   ```rust
   // New
   let context = ErrorContext::new(&env, "operation", "details");
   debug::log_error(&env, error, &context);
   ```

## Error Codes Reference

| Error | Code | Category | Recoverable | Critical |
|-------|------|----------|-------------|----------|
| Unauthorized | UNAUTHORIZED | Security | No | Yes |
| MarketClosed | MARKET_CLOSED | Market | No | No |
| OracleUnavailable | ORACLE_UNAVAILABLE | Oracle | Yes | No |
| InsufficientStake | INSUFFICIENT_STAKE | Validation | Yes | No |
| MarketAlreadyResolved | MARKET_ALREADY_RESOLVED | Market | No | No |
| InvalidOracleConfig | INVALID_ORACLE_CONFIG | Oracle | Yes | No |
| AlreadyClaimed | ALREADY_CLAIMED | State | No | No |
| NothingToClaim | NOTHING_TO_CLAIM | State | No | No |
| MarketNotResolved | MARKET_NOT_RESOLVED | Market | No | No |
| InvalidOutcome | INVALID_OUTCOME | Validation | Yes | No |

## Future Enhancements

1. **Error Chaining**: Support for error chains and causal relationships
2. **Error Metrics**: Collection and reporting of error statistics
3. **Error Recovery**: Automatic recovery mechanisms for certain error types
4. **Error Notifications**: Event emission for critical errors
5. **Error Localization**: Support for multiple languages in error messages

## Conclusion

The new error management system provides a robust foundation for handling errors in the Predictify Hybrid contract. By following the patterns and best practices outlined in this document, developers can create more maintainable, debuggable, and user-friendly error handling code. 