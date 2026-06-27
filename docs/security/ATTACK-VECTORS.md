# Attack Vectors and Mitigations for Predictify Hybrid

This document analyzes security threats specific to the Predictify Hybrid prediction market smart contracts and details the implemented mitigations. Each attack vector is mapped to specific code-level protections and validation mechanisms.

## 🎯 Threat Model Overview

### Scope
- **Target**: Predictify Hybrid Soroban smart contracts
- **Assets**: User funds, market integrity, oracle data, admin controls
- **Actors**: Malicious users, compromised oracles, insider threats, external attackers

### Security Goals
- **Confidentiality**: Protect sensitive market and user data
- **Integrity**: Ensure market outcomes and transactions remain unaltered
- **Availability**: Maintain contract operation under adverse conditions
- **Accountability**: Track all administrative and critical actions

---

## 🔐 Critical Attack Vectors and Mitigations

### 1. Authorization and Access Control Attacks

#### 1.1 Unauthorized Admin Access
**Threat**: Malicious actors gain admin privileges to manipulate markets or steal funds.

**Attack Scenarios**:
- Direct admin function calls by non-admin users
- Privilege escalation through role manipulation
- Admin key compromise or theft

**Mitigations**:
```rust
// Multi-layered authorization checks
pub fn require_admin_role(env: &Env, caller: &Address, required_role: AdminRole) -> Result<(), Error> {
    let admin_manager = AdminManager::from_env(env);
    
    // Check if caller has required role
    if !admin_manager.has_role(caller, required_role)? {
        return Err(Error::Unauthorized);
    }
    
    // Additional permission validation
    if !admin_manager.has_permission(caller, required_permission)? {
        return Err(Error::Unauthorized);
    }
    
    Ok(())
}
```

**Code References**:
- `admin.rs`: Role-based access control system
- `validation.rs`: Admin permission validation
- Error code: `Error::Unauthorized (100)`

#### 1.2 Admin Key Compromise
**Threat**: Admin private keys are compromised, allowing complete contract control.

**Mitigations**:
- **Multi-Admin System**: Requires multiple admins for critical operations
- **Time-Based Controls**: Admin actions have time windows and rate limits
- **Audit Trail**: All admin actions are logged and auditable
- **Recovery Mechanisms**: Compromised admin keys can be revoked

```rust
// Multi-admin validation for critical operations
pub fn require_multi_admin_approval(env: &Env, action: AdminAction) -> Result<(), Error> {
    let required_approvals = get_required_approvals(action);
    let current_approvals = get_admin_approvals(env, action)?;
    
    if current_approvals.len() < required_approvals {
        return Err(Error::InsufficientApprovals);
    }
    
    Ok(())
}
```

**Code References**:
- `admin.rs`: Multi-admin approval system
- `audit_trail.rs`: Action logging and tracking
- Error codes: `Error::InsufficientApprovals (418)`

---

### 2. Market Manipulation Attacks

#### 2.1 Duplicate Outcome Creation
**Threat**: Attackers create confusing duplicate outcomes to manipulate betting behavior.

**Attack Scenarios**:
- "Yes" vs "yes " (with trailing space)
- "Yes" vs "YES" (case variation)
- "Yes" vs "yes!" (punctuation variation)

**Mitigations**:
```rust
// Outcome deduplication system
pub struct OutcomeDeduplicator;

impl OutcomeDeduplicator {
    /// Normalizes outcomes for consistent comparison
    pub fn normalize_outcome(outcome: &String) -> Result<String, ValidationError> {
        // 1. Trim whitespace
        let trimmed = outcome.trim();
        if trimmed.is_empty() {
            return Err(ValidationError::OutcomeNormalizationFailed);
        }
        
        // 2. Case normalization
        let lowercased = trimmed.to_lowercase();
        
        // 3. Whitespace compression
        let compressed = lowercased.split_whitespace().collect::<Vec<&str>>().join(" ");
        
        // 4. Punctuation removal
        let cleaned = compressed.chars()
            .filter(|c| !matches!(c, '!' | '?' | '.' | ',' | ';' | ':' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}'))
            .collect::<String>();
        
        if cleaned.is_empty() {
            return Err(ValidationError::OutcomeNormalizationFailed);
        }
        
        Ok(String::from_str(&outcome.env(), &cleaned))
    }
    
    /// Validates outcomes for duplicates and ambiguities
    pub fn validate_outcomes(outcomes: &Vec<String>) -> Result<(), ValidationError> {
        let mut normalized_outcomes = Vec::new();
        
        // Normalize and collect all outcomes
        for outcome in outcomes {
            let normalized = Self::normalize_outcome(outcome)?;
            
            // Check for exact duplicates
            if normalized_outcomes.contains(&normalized) {
                return Err(ValidationError::DuplicateOutcome);
            }
            
            normalized_outcomes.push(normalized);
        }
        
        // Check for ambiguous outcomes (high similarity)
        for i in 0..normalized_outcomes.len() {
            for j in (i + 1)..normalized_outcomes.len() {
                let similarity = Self::calculate_similarity(&normalized_outcomes[i], &normalized_outcomes[j]);
                if similarity > 80 { // 80% similarity threshold
                    return Err(ValidationError::AmbiguousOutcome);
                }
                
                // Check for semantic duplicates
                if Self::is_semantic_duplicate(&normalized_outcomes[i], &normalized_outcomes[j]) {
                    return Err(ValidationError::AmbiguousOutcome);
                }
            }
        }
        
        Ok(())
    }
}
```

**Code References**:
- `validation.rs`: OutcomeDeduplicator implementation
- Error codes: `ValidationError::DuplicateOutcome`, `ValidationError::AmbiguousOutcome`
- Test coverage: `metadata_validation_tests.rs`

#### 2.2 Market Resolution Manipulation
**Threat**: Manipulating oracle data or resolution process to favor specific outcomes.

**Mitigations**:
- **Oracle Validation**: Strict oracle provider validation and whitelisting
- **Signature Verification**: Cryptographic validation of oracle data
- **Replay Protection**: Timestamp and nonce validation
- **Multi-Oracle Consensus**: Require multiple oracle confirmations

```rust
// Oracle security validation
pub fn validate_oracle_signature(
    env: &Env,
    oracle_data: &OracleData,
    expected_oracle: &Address
) -> Result<(), Error> {
    // Verify oracle is whitelisted
    let oracle_whitelist = OracleWhitelist::from_env(env);
    if !oracle_whitelist.is_approved(expected_oracle)? {
        return Err(Error::UnauthorizedOracle);
    }
    
    // Verify cryptographic signature
    if !verify_signature(oracle_data, expected_oracle)? {
        return Err(Error::InvalidOracleSignature);
    }
    
    // Check for replay attacks
    if is_replay_attack(env, oracle_data)? {
        return Err(Error::ReplayAttack);
    }
    
    Ok(())
}
```

**Code References**:
- `oracles.rs`: Oracle validation and management
- `tests/security/oracle_security_tests.rs`: Oracle security tests
- Error codes: `Error::UnauthorizedOracle (201)`, `Error::InvalidOracleSignature (202)`

---

### 3. Input Validation and Injection Attacks

#### 3.1 Malicious Input Injection
**Threat**: Injecting malicious strings or data to cause unexpected behavior.

**Attack Scenarios**:
- Unicode manipulation attacks
- Zero-width character injection
- Buffer overflow attempts
- Format string attacks

**Mitigations**:
```rust
// Comprehensive input validation
pub struct InputValidator;

impl InputValidator {
    /// Validates string input with security checks
    pub fn validate_secure_string(input: &String, min_len: u32, max_len: u32) -> Result<(), ValidationError> {
        // Length validation
        if input.len() < min_len as usize || input.len() > max_len as usize {
            return Err(ValidationError::InvalidStringLength);
        }
        
        // Unicode safety validation
        if contains_malicious_unicode(input)? {
            return Err(ValidationError::InvalidUnicode);
        }
        
        // Control character validation
        if contains_control_characters(input)? {
            return Err(ValidationError::InvalidCharacters);
        }
        
        // Pattern validation
        if matches_dangerous_patterns(input)? {
            return Err(ValidationError::InvalidFormat);
        }
        
        Ok(())
    }
    
    /// Checks for malicious Unicode sequences
    fn contains_malicious_unicode(input: &String) -> Result<bool, ValidationError> {
        // Check for zero-width characters
        if input.chars().any(|c| c.is_control() && c != ' ' && c != '\t' && c != '\n' && c != '\r') {
            return Ok(true);
        }
        
        // Check for suspicious Unicode normalization
        let normalized = input.to_lowercase();
        if normalized != input.to_lowercase() {
            // Potential Unicode spoofing attempt
            return Ok(true);
        }
        
        Ok(false)
    }
}
```

**Code References**:
- `validation.rs`: InputValidator implementation
- Error codes: `ValidationError::InvalidString`, `ValidationError::InvalidFormat`

---

### 4. Denial of Service (DoS) Attacks

#### 4.1 Resource Exhaustion
**Threat**: Overwhelming the contract with expensive operations to cause denial of service.

**Attack Scenarios**:
- Creating excessive numbers of markets
- Large input strings causing gas exhaustion
- Recursive call attempts
- Storage exhaustion attacks

**Mitigations**:
```rust
// Circuit breaker for DoS protection
pub struct CircuitBreaker;

impl CircuitBreaker {
    /// Checks if operation should be allowed based on system state
    pub fn check_operation_allowed(env: &Env, operation: OperationType) -> Result<(), Error> {
        let breaker_state = Self::get_state(env);
        
        match breaker_state {
            BreakerState::Closed => {
                // Check operation-specific limits
                Self::check_operation_limits(env, operation)?;
            },
            BreakerState::Open => {
                return Err(Error::CircuitBreakerOpen);
            },
            BreakerState::HalfOpen => {
                // Limited operations allowed for testing
                Self::check_half_open_limits(env, operation)?;
            }
        }
        
        Ok(())
    }
    
    /// Rate limiting for expensive operations
    pub fn check_rate_limits(env: &Env, caller: &Address, operation: OperationType) -> Result<(), Error> {
        let rate_limiter = RateLimiter::from_env(env);
        
        if rate_limiter.is_rate_limited(caller, operation)? {
            return Err(Error::RateLimitExceeded);
        }
        
        Ok(())
    }
}
```

**Code References**:
- `circuit_breaker.rs`: DoS protection mechanisms
- `rate_limiter.rs`: Rate limiting implementation
- Error codes: `Error::CircuitBreakerOpen (500)`, `Error::RateLimitExceeded (501)`

#### 4.2 Gas Exhaustion Attacks
**Threat**: Forcing users to spend excessive gas on operations.

**Mitigations**:
- **Gas Limits**: Maximum gas limits for operations
- **Early Validation**: Fail fast on invalid inputs
- **Efficient Algorithms**: Optimized for gas usage
- **Batch Operation Limits**: Limits on batch sizes

```rust
// Gas-efficient validation
pub fn validate_gas_efficient(outcomes: &Vec<String>) -> Result<(), ValidationError> {
    // Early size check to prevent expensive operations
    if outcomes.len() > MAX_MARKET_OUTCOMES as usize {
        return Err(ValidationError::ArrayTooLarge);
    }
    
    // Batch validation for efficiency
    for (i, outcome) in outcomes.iter().enumerate() {
        // Quick length check first
        if outcome.len() > MAX_OUTCOME_LENGTH as usize {
            return Err(ValidationError::StringTooLong);
        }
        
        // Only then do more expensive validation
        if i < outcomes.len() - 1 {
            // Check for duplicates in remaining outcomes
            for other in &outcomes[i+1..] {
                if are_similar_fast(outcome, other)? {
                    return Err(ValidationError::AmbiguousOutcome);
                }
            }
        }
    }
    
    Ok(())
}
```

---

### 5. Economic and Financial Attacks

#### 5.1 Price Manipulation
**Threat**: Manipulating market prices or odds for financial gain.

**Mitigations**:
- **Price Validation**: Oracle price validation and sanity checks
- **Liquidity Requirements**: Minimum liquidity thresholds
- **Betting Limits**: Maximum bet sizes and exposure limits
- **Market Making**: Automated market making to prevent manipulation

```rust
// Price manipulation protection
pub fn validate_price_data(env: &Env, price_data: &PriceData) -> Result<(), Error> {
    // Sanity check on price values
    if price_data.price <= 0 || price_data.price > MAX_REASONABLE_PRICE {
        return Err(Error::InvalidPrice);
    }
    
    // Check for price deviation from previous
    if let Some(prev_price) = get_previous_price(env, &price_data.asset)? {
        let deviation = calculate_price_deviation(price_data.price, prev_price);
        if deviation > MAX_PRICE_DEVIATION {
            return Err(Error::ExcessivePriceDeviation);
        }
    }
    
    // Validate timestamp freshness
    if env.ledger().timestamp() - price_data.timestamp > MAX_PRICE_AGE {
        return Err(Error::StalePriceData);
    }
    
    Ok(())
}
```

**Code References**:
- `fees.rs`: Fee validation and limits
- `markets.rs`: Market integrity checks
- Error codes: `Error::InvalidPrice (203)`, `Error::ExcessivePriceDeviation (204)`

#### 5.2 Front-Running Attacks
**Threat**: Exploiting knowledge of pending transactions for profit.

**Mitigations**:
- **Transaction Ordering**: Fair transaction ordering mechanisms
- **Commit-Reveal Schemes**: Two-phase commitment for sensitive operations
- **Time-Based Windows**: Fixed time windows for operations
- **Randomized Delays**: Small randomized delays to prevent timing attacks

---

### 6. Oracle and Data Feed Attacks

#### 6.1 Oracle Data Corruption
**Threat**: Compromised oracles providing incorrect data.

**Mitigations**:
- **Multi-Oracle Consensus**: Require multiple oracle confirmations
- **Oracle Reputation**: Reputation-based oracle selection
- **Data Validation**: Cross-validation of oracle data
- **Fallback Mechanisms**: Alternative oracle sources

```rust
// Multi-oracle validation
pub fn validate_multi_oracle_consensus(
    env: &Env,
    oracle_data: Vec<OracleData>
) -> Result<ConsensusResult, Error> {
    // Require minimum number of oracle responses
    if oracle_data.len() < MIN_ORACLE_RESPONSES {
        return Err(Error::InsufficientOracleResponses);
    }
    
    // Check for consensus among oracles
    let consensus_data = find_consensus(&oracle_data)?;
    let agreement_percentage = calculate_agreement(&oracle_data, &consensus_data);
    
    if agreement_percentage < MIN_CONSENSUS_THRESHOLD {
        return Err(Error::NoOracleConsensus);
    }
    
    Ok(ConsensusResult {
        data: consensus_data,
        confidence: agreement_percentage,
        participating_oracles: oracle_data.len(),
    })
}
```

**Code References**:
- `oracles.rs`: Multi-oracle consensus system
- `tests/security/oracle_security_tests.rs`: Oracle security tests
- Error codes: `Error::InsufficientOracleResponses (205)`, `Error::NoOracleConsensus (206)`

---

### 7. Smart Contract Vulnerability Attacks

#### 7.1 Reentrancy Attacks
**Threat**: Recursive calls that can drain contract funds.

**Mitigations**:
- **Checks-Effects-Interactions Pattern**: Proper ordering of operations
- **Reentrancy Guards**: Explicit reentrancy protection
- **State Updates First**: Update contract state before external calls

```rust
// Reentrancy protection
pub struct ReentrancyGuard;

impl ReentrancyGuard {
    pub fn enter(env: &Env) -> Result<(), Error> {
        let guard_key = StorageKey::ReentrancyGuard;
        
        if env.storage().get(&guard_key).unwrap_or(false) {
            return Err(Error::ReentrancyDetected);
        }
        
        env.storage().set(&guard_key, &true);
        Ok(())
    }
    
    pub fn exit(env: &Env) {
        let guard_key = StorageKey::ReentrancyGuard;
        env.storage().set(&guard_key, &false);
    }
}

// Usage pattern
pub fn place_bet(env: &Env, caller: Address, market_id: u64, outcome: u32, amount: i128) -> Result<(), Error> {
    // Reentrancy protection
    ReentrancyGuard::enter(env)?;
    
    // Checks
    validate_bet_params(env, &caller, market_id, outcome, amount)?;
    
    // Effects (update state first)
    update_bet_state(env, &caller, market_id, outcome, amount)?;
    
    // Interactions (external calls last)
    transfer_tokens(env, &caller, amount)?;
    
    // Release guard
    ReentrancyGuard::exit(env);
    
    Ok(())
}
```

**Code References**:
- Error code: `Error::ReentrancyDetected (417)`

#### 7.2 Integer Overflow/Underflow
**Threat**: Arithmetic operations causing unexpected behavior.

**Mitigations**:
- **Safe Math Operations**: Use checked arithmetic
- **Range Validation**: Validate input ranges
- **Type Safety**: Use appropriate numeric types

```rust
// Safe arithmetic operations
pub fn safe_add(a: i128, b: i128) -> Result<i128, Error> {
    a.checked_add(b).ok_or(Error::ArithmeticOverflow)
}

pub fn safe_subtract(a: i128, b: i128) -> Result<i128, Error> {
    a.checked_sub(b).ok_or(Error::ArithmeticUnderflow)
}

pub fn safe_multiply(a: i128, b: i128) -> Result<i128, Error> {
    a.checked_mul(b).ok_or(Error::ArithmeticOverflow)
}

// Usage in betting calculations
pub fn calculate_winnings(bet_amount: i128, odds: u32) -> Result<i128, Error> {
    let odds_as_i128 = i128::from(odds);
    let numerator = safe_multiply(bet_amount, odds_as_i128)?;
    safe_divide(numerator, 10000i128) // Odds are in basis points
}
```

**Code References**:
- Error codes: `Error::ArithmeticOverflow (415)`, `Error::ArithmeticUnderflow (416)`

---

## 🔍 Security Testing Coverage

### Test Categories
1. **Unit Tests**: Individual function security testing
2. **Integration Tests**: Cross-module security validation
3. **Property-Based Tests**: Fuzzing and edge case testing
4. **Security-Specific Tests**: Attack simulation and mitigation verification

### Key Security Test Files
- `tests/security/oracle_security_tests.rs`: Oracle security testing
- `tests/metadata_validation_tests.rs`: Input validation and deduplication
- `tests/property_based_tests.rs`: Fuzzing and edge case testing
- `tests/circuit_breaker_tests.rs`: DoS protection testing

### Coverage Requirements
- **≥95% line coverage** on all security-critical modules
- **100% coverage** on authentication and authorization logic
- **Comprehensive edge case testing** for all validation functions

---

## 📊 Security Metrics and Monitoring

### Key Security Indicators
- **Failed Authentication Rate**: Monitor unauthorized access attempts
- **Validation Failure Rate**: Track input validation failures
- **Oracle Consensus Health**: Monitor oracle agreement rates
- **Circuit Breaker Triggers**: Track DoS protection activation

### Audit Trail
All security-relevant actions are logged:
```rust
pub struct AuditEvent {
    pub timestamp: u64,
    pub actor: Address,
    pub action: AuditAction,
    pub target: Option<Address>,
    pub metadata: String,
    pub result: AuditResult,
}
```

---

## 🚨 Incident Response

### Security Incident Categories
1. **Critical**: Fund loss, admin compromise, oracle manipulation
2. **High**: Market manipulation, DoS attacks, data corruption
3. **Medium**: Unauthorized access attempts, validation failures
4. **Low**: Suspicious activity, performance issues

### Response Procedures
1. **Detection**: Automated monitoring and alerting
2. **Assessment**: Impact analysis and threat classification
3. **Containment**: Circuit breaker activation, access revocation
4. **Recovery**: System restoration, fund recovery
5. **Post-Mortem**: Analysis and improvement implementation

---

## 🎯 Security Invariants

### Core Invariants
1. **No Unauthorized Admin Access**: All admin actions require proper authorization
2. **No Duplicate Market Outcomes**: All outcomes must be unique and unambiguous
3. **No Fund Drainage**: Contract funds cannot be drained by attackers
4. **No Oracle Manipulation**: Oracle data must be validated and consensus-based
5. **No DoS Vulnerabilities**: System remains available under attack conditions

### Validation Rules
- All inputs must pass comprehensive validation
- All state changes must be authorized and audited
- All external calls must be protected against reentrancy
- All arithmetic operations must be safe against overflow/underflow

---

## 📋 Security Checklist

### Pre-Deployment
- [ ] All security tests pass with ≥95% coverage
- [ ] Manual security review completed
- [ ] Third-party audit findings addressed
- [ ] Circuit breaker functionality verified
- [ ] Admin access controls tested
- [ ] Oracle security validated

### Post-Deployment
- [ ] Continuous monitoring enabled
- [ ] Alert systems configured
- [ ] Incident response procedures tested
- [ ] Regular security reviews scheduled
- [ ] Audit trail integrity verified

---

## 🔮 Future Security Enhancements

### Planned Improvements
1. **Zero-Knowledge Proofs**: Enhanced privacy for sensitive operations
2. **Multi-Signature Wallets**: Enhanced admin security
3. **Decentralized Oracle Networks**: Improved oracle security and reliability
4. **Formal Verification**: Mathematical proof of security properties
5. **Machine Learning Detection**: AI-powered anomaly detection

### Research Areas
- Quantum-resistant cryptography
- Advanced fraud detection algorithms
- Cross-chain security protocols
- Privacy-preserving market mechanisms

---

This document serves as the authoritative reference for Predictify Hybrid security architecture and should be updated whenever new threats are identified or mitigations are implemented.

---

## ⏱ Oracle Timeout vs Dispute Window Interaction

### Threat: Resolution Timeout Deadlocking a Disputed Market

**Description**: If `fetch_oracle_result` is called after `end_time + resolution_timeout` and a dispute is active, the naive implementation would cancel the market. This permanently locks dispute stakes and leaves the market unresolvable — a deadlock.

**Invariant**: `resolution_timeout` must never cancel a market that has an active dispute (`state == Disputed` or `total_dispute_stakes() > 0`). When a dispute is active, the dispute process is the authoritative resolution path.

**Mitigation** (`resolution.rs` — `OracleResolutionManager::fetch_oracle_result`):
```rust
if current_time > market.end_time + market.resolution_timeout {
    if market.state == MarketState::Disputed || market.total_dispute_stakes() > 0 {
        return Err(Error::ResolutionTimeoutReached); // dispute owns resolution
    }
    // No dispute: safe to cancel for refunds
    market.state = MarketState::Cancelled;
    ...
}
```

### Threat: Late Dispute Reopening a Settled Market

**Description**: Without enforcing `dispute_window_seconds`, a dispute could be filed long after the market ended, re-opening a market that participants already consider settled and payouts already expected.

**Invariant**: Disputes must be filed within `[end_time, end_time + dispute_window_seconds)`. After the window closes, payouts are unambiguously allowed.

**Mitigation** (`disputes.rs` — `DisputeValidator::validate_market_for_dispute`):
```rust
if market.dispute_window_seconds > 0
    && current_time >= market.end_time + market.dispute_window_seconds
{
    return Err(Error::MarketResolved);
}
```

**Note**: `dispute_window_seconds == 0` disables the window check (no restriction), preserving backward compatibility for markets created before this field was introduced.

### Non-Goals

- This does not prevent an oracle from returning stale data within the timeout window; that is handled separately by `OracleStale` / `OracleConfidenceTooWide`.
- Admin override (`finalize_market`) bypasses both checks by design — it is an emergency escape hatch requiring explicit admin authentication.
