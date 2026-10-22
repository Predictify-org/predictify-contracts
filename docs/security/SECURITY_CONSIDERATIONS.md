# Security Considerations

This document outlines the security considerations and protection strategies for the Predictify Hybrid prediction market smart contracts.

## 🎯 Overview

The Predictify Hybrid contract implements comprehensive security measures to protect user funds, ensure market integrity, and maintain system availability. This document details the security architecture, threat models, and mitigation strategies.

## 🔐 Core Security Principles

### 1. **Defense in Depth**
Multiple layers of security controls ensure that failure of one control doesn't compromise the system:
- **Input Validation**: Comprehensive validation of all user inputs
- **Access Control**: Role-based authorization for administrative functions
- **Oracle Authentication**: Cryptographic verification of oracle data
- **Rate Limiting**: Protection against flooding attacks
- **Audit Logging**: Comprehensive tracking of all security-relevant events

### 2. **Least Privilege**
Users and contracts only have the minimum permissions necessary:
- **User Operations**: Limited to betting, voting, and claiming winnings
- **Admin Operations**: Segregated by role with specific permissions
- **Oracle Operations**: Only authorized oracle contracts can update market outcomes

### 3. **Fail Secure**
System defaults to secure state when errors occur:
- **Circuit Breaker**: Automatic pause on suspicious activity
- **Graceful Degradation**: Fallback mechanisms for oracle failures
- **Error Handling**: Secure error responses that don't leak sensitive information

---

## 🔒 Oracle Callback Authentication

### Overview
The Predictify Hybrid contract implements a comprehensive oracle callback authentication system to ensure that only authorized oracle contracts can update oracle-driven state. This prevents malicious actors from manipulating market outcomes through unauthorized oracle callbacks.

### Authentication Flow

#### 1. **Caller Authorization**
```rust
// Only whitelisted oracle contracts can invoke callbacks
pub fn verify_caller_authorization(&self, caller: &Address) -> Result<(), Error> {
    let whitelist = OracleWhitelist::from_env(&self.env);
    
    if !whitelist.is_oracle_authorized(caller)? {
        return Err(Error::OracleCallbackUnauthorized);
    }
    Ok(())
}
```

#### 2. **Signature Verification**
```rust
// Cryptographic verification of oracle data authenticity
pub fn verify_signature(&self, caller: &Address, callback_data: &OracleCallbackData) -> Result<(), Error> {
    let message = self.create_signature_message(callback_data);
    
    if !self.verify_ed25519_signature(caller, &message, &callback_data.signature)? {
        return Err(Error::OracleCallbackInvalidSignature);
    }
    Ok(())
}
```

#### 3. **Replay Protection**
```rust
// Prevent replay attacks using nonce tracking
pub fn prevent_replay_attack(&self, caller: &Address, callback_data: &OracleCallbackData) -> Result<(), Error> {
    let nonce_key = StorageKey::OracleNonce(caller.clone(), callback_data.nonce);
    
    if self.env.storage().get(&nonce_key).unwrap_or(false) {
        return Err(Error::OracleCallbackReplayDetected);
    }
    
    self.env.storage().set(&nonce_key, &true);
    Ok(())
}
```

#### 4. **Rate Limiting**
```rust
// Prevent oracle flooding attacks
pub fn enforce_rate_limiting(&self, caller: &Address) -> Result<(), Error> {
    let rate_limit_key = StorageKey::OracleRateLimit(caller.clone());
    let current_time = self.env.ledger().timestamp();
    let last_callback_time: u64 = self.env.storage().get(&rate_limit_key).unwrap_or(0);
    
    if current_time < last_callback_time + MIN_CALLBACK_INTERVAL {
        return Err(Error::OracleCallbackTimeout);
    }
    
    self.env.storage().set(&rate_limit_key, &current_time);
    Ok(())
}
```

### Security Controls

#### **Data Validation**
- **Feed ID Validation**: Ensures feed identifiers are non-empty and properly formatted
- **Price Bounds**: Validates prices are within reasonable ranges (prevents extreme values)
- **Timestamp Freshness**: Ensures data is not stale or from the future
- **Nonce Validation**: Prevents replay attacks with unique nonces

#### **Cryptographic Protection**
- **Ed25519 Signatures**: Industry-standard cryptographic signatures
- **Message Construction**: Deterministic message creation for signature verification
- **Key Management**: Secure oracle contract key management

#### **Access Control**
- **Oracle Whitelist**: Only pre-approved oracle contracts can make callbacks
- **Active Status Check**: Inactive oracles cannot update market outcomes
- **Admin Authorization**: Oracle whitelist management requires admin permissions

### Error Types

| Error | Description | Security Impact |
|-------|-------------|------------------|
| `OracleCallbackAuthFailed` | General authentication failure | High |
| `OracleCallbackUnauthorized` | Caller not in whitelist | High |
| `OracleCallbackInvalidSignature` | Signature verification failed | High |
| `OracleCallbackReplayDetected` | Replay attack detected | High |
| `OracleCallbackTimeout` | Rate limit exceeded | Medium |

### Integration Points

#### **Market Resolution**
```rust
// Oracle callbacks are integrated with market resolution
pub fn process_authenticated_callback(
    env: &Env,
    caller: &Address,
    callback_data: &OracleCallbackData,
    market_id: &Symbol,
) -> Result<(), Error> {
    // Authenticate callback
    let auth = OracleCallbackAuth::new(env);
    auth.authenticate_and_process(caller, callback_data)?;
    
    // Update market resolution
    Self::update_market_resolution(env, callback_data, market_id)?;
    Ok(())
}
```

#### **Event Emission**
```rust
// Comprehensive audit trail
pub fn emit_oracle_callback(
    env: &Env,
    oracle_address: &Address,
    feed_id: &String,
    price: i128,
    timestamp: u64,
) {
    env.events().publish(
        (Symbol::new(env, "oracle_callback"), oracle_address, feed_id, price, timestamp),
        (),
    );
}
```

---

## 🔁 Reentrancy and Cross-Call State Consistency

Implemented in [`contracts/predictify-hybrid/src/reentrancy_guard.rs`](../../contracts/predictify-hybrid/src/reentrancy_guard.rs).

### Why Soroban needs a reentrancy guard at all

Soroban's execution model differs from EVM in ways that make the **classic** EVM reentrancy exploit (the recipient running its fallback during `call.value(...)`) inapplicable in many places:

- There is **no fallback function** that runs implicitly on every value transfer.
- A **Stellar Asset Contract (SAC) token's `transfer`** cannot execute caller-supplied code on the recipient.
- Cross-contract calls are **explicit** (`env.invoke_contract(...)`), and the Soroban host accounts for storage modifications atomically per top-level call.

The guard is still required because:

1. **Custom token contracts** — Predictify Hybrid accepts any contract implementing the `token::Client` interface (see [`tokens.rs`](../../contracts/predictify-hybrid/src/tokens.rs)). Any such contract is third-party code and may re-enter the protocol during `transfer`, `mint`, or `burn`.
2. **Oracle contracts** — every `oracles.rs` call crosses into upgradable third-party code.
3. **Cross-function reentrancy** — even when the inbound call hits a different public entrypoint, shared persistent state (fee vault, market totals) can be observed at an inconsistent intermediate value.
4. **Panic safety** — a Soroban host function may panic. A panic between a state mutation and its matching external call leaves on-ledger state inconsistent unless the writes are sequenced after the call.

### Threat model

| Attacker action | Without guard | With guard |
|---|---|---|
| Malicious custom token re-enters `claim_winnings` during `transfer` | Could double-claim before the `claimed` flag is written | `before_external_call` rejects the inner invocation with `ReentrancyGuardActive` |
| Malicious oracle re-enters `resolve_market` during a price callback | Could observe a partially-resolved market and bias subsequent reads | Inner `check_reentrancy_state` returns `ReentrancyGuardActive` |
| Failed external call leaves the lock set | n/a | `with_external_call` releases the lock on every return path; `after_external_call` is idempotent |
| Cross-transaction race (two top-level invocations) | n/a | Out of scope — handled by higher-level state machines (`MarketState`, `ClaimInfo`) |

### Invariants

Auditors should verify these invariants hold for every change to a protected entrypoint:

- **I-1 (mutual exclusion)**: while the lock is held, no public entrypoint of this contract may make further state changes.
- **I-2 (release on every path)**: every `before_external_call` is paired with `after_external_call` on **all** return paths (including error paths). `with_external_call` enforces this by construction.
- **I-3 (CEI ordering)**: protected sections write internal state **before** the external call (Checks-Effects-Interactions). The guard is an additional defensive layer, **not** a substitute for ordering.
- **I-4 (idempotent release)**: calling `after_external_call` on an already-released lock is a no-op, never an error. This makes nested cleanup safe.
- **I-5 (panic safety)**: a panic inside a protected section aborts the host invocation and rolls back the persistent-storage write that acquired the lock; the next top-level invocation sees the lock cleared.
- **I-6 (single key)**: one global persistent flag (`reent_lk`). No per-market or per-user variants. Finer locks would not prevent cross-function reentrancy and would cost extra ledger writes.

### Public API summary

| Function | Purpose | When to use |
|---|---|---|
| `ReentrancyGuard::with_external_call(env, f)` | Acquire lock, run `f`, release on every return path | **Default choice for any new external-call site.** |
| `ReentrancyGuard::before_external_call(env)` | Acquire the lock | Only when you cannot use `with_external_call` (e.g. batch flows that span helper functions) — caller is responsible for I-2 |
| `ReentrancyGuard::after_external_call(env)` | Release the lock | Pair with the manual `before_external_call` above |
| `ReentrancyGuard::check_reentrancy_state(env)` | Assert no external call is in flight | Sensitive read/state-only entrypoints that themselves do not make outbound calls |
| `ReentrancyGuard::is_locked(env)` | Read the lock without mutating | Diagnostics and tests |
| `ReentrancyGuard::validate_external_call_success(env, ok)` | Standardise the failure code returned for an external-call boolean | All external-call result checks |
| `ReentrancyGuard::restore_state_on_failure(env, f)` | Run rollback closure when CEI cannot be followed | Rare cases where provisional state must be written before the call |

### Error taxonomy

| Error | Meaning | Caller mapping |
|---|---|---|
| `GuardError::ReentrancyGuardActive` | Reentry attempted while a call is in flight | Map to `Error::ReentrancyDetected` (417) |
| `GuardError::ExternalCallFailed` | Caller-supplied success flag was `false` | Surface as the matching protocol-level error (e.g. `Error::TransferFailed`) |

### Recommended call pattern

```rust
use crate::reentrancy_guard::ReentrancyGuard;

ReentrancyGuard::with_external_call(env, || {
    // Effects: update internal state first (CEI).
    vault::debit(env, amount)?;

    // Interactions: external call last.
    token_client.transfer(&env.current_contract_address(), &user, &amount);
    Ok::<_, crate::Error>(())
})?;
```

### Where it is currently applied

| Module | External call | Protection strategy |
|---|---|---|
| [`fees.rs`](../../contracts/predictify-hybrid/src/fees.rs) — fee withdrawal | `token_client.transfer` to admin | Vault accounting written first; transfer wrapped by reentrancy guard. Aggregation inside the contract means a malicious token cannot re-enter to double-withdraw. |
| [`bets.rs`](../../contracts/predictify-hybrid/src/bets.rs) — `BetUtils::lock_funds` / `unlock_funds` | `token_client.transfer` between user and contract | Caller (`cancel_event` etc.) holds the reentrancy lock for the entire batch; helpers do not re-acquire so batch refunds remain atomic under a single guard scope. See the doc comment on `BetUtils::unlock_funds`. |
| `oracles.rs` resolution callback | `env.invoke_contract` to oracle | Authenticated via [Oracle Callback Authentication](#-oracle-callback-authentication); state writes follow oracle response (CEI). |

### Non-goals

- **Per-market or per-user locking** — intentionally a single global flag.
- **Cross-transaction protection** — out of scope; higher-level state machines own that.
- **Replacing CEI** — the guard is defence-in-depth, not a substitute for correct ordering.

### Auditor checklist

- [ ] Every new external call site uses `ReentrancyGuard::with_external_call` *or* documents why it manually pairs `before_external_call` / `after_external_call`.
- [ ] State writes occur **before** the external call inside the protected section (CEI).
- [ ] The closure returned to `with_external_call` does not silently swallow errors — failures must propagate so the lock release is observable in logs.
- [ ] Tests cover both success and error return paths and assert `is_locked == false` afterward.
- [ ] No protected entrypoint introduces a new persistent storage key matching `reent_lk`.

### Integrator notes

External integrators that call Predictify Hybrid from another contract should:

- Treat `Error::ReentrancyDetected` as a **transient** error in the same transaction; retry in a new top-level transaction.
- Avoid invoking Predictify Hybrid from inside a callback invoked by Predictify itself (e.g. a custom token's `transfer` hook). Such calls will be rejected.
- Not rely on observing the lock flag externally; it is an implementation detail and may move to a different storage class in future versions.

---

## 🛡️ Access Control and Authorization

### Role-Based Access Control (RBAC)

#### **Admin Roles**
- **SuperAdmin**: Full system control
- **MarketAdmin**: Market management permissions
- **ConfigAdmin**: Configuration management
- **FeeAdmin**: Fee management
- **ReadOnlyAdmin**: View-only permissions

#### **Permission Matrix**
| Role | Market Creation | Oracle Management | Fee Management | User Management |
|------|----------------|------------------|----------------|------------------|
| SuperAdmin | ✅ | ✅ | ✅ | ✅ |
| MarketAdmin | ✅ | ❌ | ❌ | ❌ |
| ConfigAdmin | ❌ | ✅ | ✅ | ❌ |
| FeeAdmin | ❌ | ❌ | ✅ | ❌ |
| ReadOnlyAdmin | ❌ | ❌ | ❌ | ❌ |

### Multi-Admin Approval

Critical operations require multiple admin approvals:
- **Oracle Whitelist Changes**: Require 2+ admin approvals
- **Configuration Updates**: Require 2+ admin approvals
- **Emergency Pauses**: Require superadmin + 1 other admin

### Primary Admin Storage and Entrypoint Authentication

Predictify Hybrid treats persistent storage key `Admin` as the contract's root of trust for administrative authority.

- `initialize()` stores the primary admin once in persistent storage.
- Primary-admin-only contract entrypoints now require both Soroban `require_auth()` and an exact match against the stored `Admin` address.
- If the `Admin` key is missing, admin-gated entrypoints fail with `AdminNotSet` rather than silently falling back to another storage source.
- Upgrade and rollback flows use the same persistent `Admin` check as the rest of the contract; they do not trust legacy instance-storage admin keys.

### Delegated Multi-Admin Flows

The contract also supports delegated admin roles after migration to the multi-admin storage layout.

- `migrate_to_multi_admin()` may only be triggered by the stored primary admin.
- Delegated admin entrypoints still require Soroban `require_auth()`.
- Delegated authorization is only evaluated after confirming the contract has an initialized primary admin root in persistent storage.
- The stored `Admin` address remains the primary authority record even after migration; delegated admins do not replace it.

### Admin Rotation and Transfer

Primary-admin rotation is implemented by `ContractPauseManager::transfer_admin()`.

- The current primary admin must satisfy Soroban `require_auth()`.
- The caller must match the stored `Admin` address.
- On success, the contract atomically rewrites the persistent `Admin` key to the new address.
- After rotation, the old primary admin immediately loses access to primary-admin-only entrypoints, and the new primary admin gains it.

Current public contract entrypoints do not expose a standalone `transfer_admin()` method. Integrators should treat the stored `Admin` value as the canonical authority source and wire any rotation workflow through an audited governance or admin-management wrapper if they need on-chain rotation at the application level.

---

## 🔄 Input Validation and Data Integrity

### Outcome Deduplication

The contract implements comprehensive outcome deduplication to prevent market manipulation:

#### **Normalization Process**
1. **Whitespace Normalization**: Trim and compress whitespace
2. **Case Normalization**: Convert to lowercase
3. **Punctuation Removal**: Remove common punctuation
4. **Unicode Safety**: Handle Unicode characters securely

#### **Similarity Detection**
- **Levenshtein Distance**: Detect outcomes >80% similar
- **Semantic Groups**: Identify common synonyms (yes/yeah, no/nope)
- **Exact Duplicates**: Case-insensitive exact match detection

### Validation Rules

| Validation | Rule | Error Type |
|-------------|------|-----------|
| String Length | 1-255 characters | `StringTooLong`/`StringTooShort` |
| Array Size | 2-10 outcomes | `ArrayTooLarge`/`ArrayTooSmall` |
| Price Range | 0 to $10M (8 decimals) | `InvalidPrice` |
| Timestamp | ±5 minutes from current | `OracleStale` |
| Nonce | 1 to 2^64-1 | `InvalidOracleFeed` |

---

## ⚡ Denial of Service Protection

### Circuit Breaker System

#### **Trigger Conditions**
- **High Error Rate**: >50% error rate over 5 minutes
- **Oracle Failures**: Multiple oracle failures
- **Network Congestion**: High transaction failure rate
- **Security Threats**: Detected attack patterns

#### **Protection Mechanisms**
- **Rate Limiting**: Per-user and per-operation limits
- **Gas Optimization**: Early validation to prevent gas exhaustion
- **Batch Limits**: Maximum batch sizes for operations
- **Timeout Protection**: Maximum execution time limits

### Rate Limiting Configuration

| Operation | Rate Limit | Window |
|-----------|------------|--------|
| Oracle Callbacks | 1 per 10 seconds | Per oracle |
| Market Creation | 5 per hour | Per user |
| Betting | 100 per hour | Per user |
| Voting | 50 per hour | Per user |

---

## 🔍 Audit and Monitoring

### Event Logging

#### **Security Events**
- **Oracle Callbacks**: All authenticated oracle callbacks
- **Authorization Failures**: Failed access attempts
- **Admin Actions**: All administrative operations
- **Circuit Breaker**: System protection activations

#### **Event Structure**
```rust
pub struct SecurityEvent {
    pub timestamp: u64,
    pub actor: Address,
    pub action: SecurityAction,
    pub target: Option<Address>,
    pub metadata: String,
    pub result: SecurityResult,
}
```

### Monitoring Metrics

#### **Key Indicators**
- **Failed Authentication Rate**: Monitor unauthorized access attempts
- **Oracle Consensus Health**: Track oracle agreement rates
- **Circuit Breaker Triggers**: Track protection system activations
- **Error Rates**: Monitor system health and performance

#### **Alert Thresholds**
- **Authentication Failures**: >10 per hour
- **Oracle Disagreements**: >3 consecutive disagreements
- **Error Rate**: >5% over 1 hour
- **Gas Usage**: Abnormal gas consumption patterns

---

## 🚨 Incident Response

### Incident Classification

#### **Critical**
- Fund loss or theft
- Admin key compromise
- Oracle manipulation
- System-wide failure

#### **High**
- Market manipulation
- DoS attacks
- Data corruption
- Security breach

#### **Medium**
- Unauthorized access attempts
- Validation failures
- Performance degradation
- Configuration errors

#### **Low**
- Suspicious activity
- Performance issues
- Minor errors
| Recovery Strategy | Error Types |
|------------------|-------------|
| RetryWithDelay | `OracleUnavailable`, `InvalidInput` |
| AlternativeMethod | `MarketNotFound`, `ConfigNotFound` |
| Skip | `AlreadyVoted`, `AlreadyBet`, `AlreadyClaimed` |
| Abort | `Unauthorized`, `MarketClosed`, `MarketResolved` |
| ManualIntervention | `AdminNotSet`, `DisputeFeeFailed` |
| NoRecovery | `InvalidState`, `InvalidOracleConfig`, `OracleConfidenceTooWide` |

### Response Procedures

#### **Detection**
1. **Automated Monitoring**: Continuous system monitoring
2. **Alert Systems**: Real-time alerting for critical events
3. **Log Analysis**: Regular review of security logs
4. **Health Checks**: Periodic system health verification

#### **Assessment**
1. **Impact Analysis**: Determine scope and impact
2. **Threat Classification**: Categorize threat level
3. **Root Cause Analysis**: Identify underlying cause
4. **Risk Assessment**: Evaluate ongoing risks

#### **Containment**
1. **Circuit Breaker**: Activate protection systems
2. **Access Revocation**: Revoke compromised access
3. **System Isolation**: Isolate affected components
4. **Data Preservation**: Preserve evidence for analysis

#### **Recovery**
1. **System Restoration**: Restore normal operations
2. **Data Recovery**: Recover from backups if needed
3. **Security Updates**: Apply security patches
4. **Access Restoration**: Restore legitimate access

#### **Post-Mortem**
1. **Incident Report**: Document incident details
2. **Lessons Learned**: Identify improvement opportunities
3. **Security Updates**: Implement security improvements
4. **Training**: Update team knowledge and procedures

---

## 🔮 Future Security Enhancements

### Planned Improvements

#### **Advanced Cryptography**
- **Zero-Knowledge Proofs**: Enhanced privacy for sensitive operations
- **Multi-Signature Wallets**: Enhanced admin security
- **Threshold Signatures**: Distributed key management

#### **Advanced Monitoring**
- **Machine Learning Detection**: AI-powered anomaly detection
- **Behavioral Analysis**: User behavior pattern analysis
- **Predictive Analytics**: Predictive threat detection

#### **Enhanced Protection**
- **Formal Verification**: Mathematical proof of security properties
- **Decentralized Oracle Networks**: Improved oracle security
- **Cross-Chain Security**: Multi-chain security coordination

### Research Areas

#### **Quantum-Resistant Cryptography**
- **Post-Quantum Algorithms**: Quantum-resistant cryptographic primitives
- **Key Migration**: Secure migration to quantum-resistant systems
- **Hybrid Schemes**: Combined classical and quantum security

#### **Advanced Fraud Detection**
- **Graph Analysis**: Network-based fraud detection
- **Temporal Patterns**: Time-based anomaly detection
- **Cross-Platform Analysis**: Multi-platform correlation

---

## 📋 Security Checklist

### Pre-Deployment
- [ ] All security tests pass with ≥95% coverage
- [ ] Manual security review completed
- [ ] Third-party audit findings addressed
- [ ] Circuit breaker functionality verified
- [ ] Oracle authentication tested
- [ ] Access controls validated
- [ ] Rate limiting configured
- [ ] Event logging verified
- [ ] Error handling tested
- [ ] Performance benchmarks met

### Post-Deployment
- [ ] Continuous monitoring enabled
- [ ] Alert systems configured
- [ ] Incident response procedures tested
- [ ] Regular security reviews scheduled
- [ ] Audit trail integrity verified
- [ ] System health checks automated
- [ ] Backup procedures validated
- [ ] Security metrics dashboard active
- [ ] Team training completed
- [ ] Documentation updated

---

## 🎯 Security Goals and Metrics

### Security Objectives

#### **Confidentiality**
- Protect sensitive market and user data
- Ensure privacy of user transactions
- Maintain confidentiality of oracle data sources

#### **Integrity**
- Ensure market outcomes remain unaltered
- Prevent manipulation of betting odds
- Maintain accuracy of oracle data

#### **Availability**
- Ensure system remains operational under attack
- Provide graceful degradation during failures
- Maintain service availability for legitimate users

#### **Accountability**
- Track all administrative actions
- Provide comprehensive audit trails
- Enable forensic analysis of security events

### Success Metrics

#### **Security Metrics**
- **Mean Time to Detect (MTTD)**: < 5 minutes for critical events
- **Mean Time to Respond (MTTR)**: < 30 minutes for critical incidents
- **False Positive Rate**: < 5% for security alerts
- **System Uptime**: > 99.9% availability
- **Authentication Success Rate**: > 99.95% for legitimate users

#### **Quality Metrics**
- **Test Coverage**: ≥95% line coverage on security modules
- **Code Review**: 100% security code review coverage
- **Security Score**: A+ grade on security assessments
- **Vulnerability Density**: < 1 vulnerability per 10,000 lines of code

---

This document serves as the authoritative reference for Predictify Hybrid security architecture and should be updated whenever new security features are implemented or threats are identified.

- GDPR: General Data Protection Regulation permits individuals the right to ask organisations to delete their personal data.
