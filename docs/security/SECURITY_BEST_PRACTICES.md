# Recommendations of Security and Best Practices

## Smart Contract Security

### Idempotency Patterns

Idempotency ensures operations can be performed multiple times without changing the result beyond the first application. This is critical for financial operations.

**Implementation Example**: Claim Idempotency
- **Problem**: Users may retry failed transactions, leading to double payouts
- **Solution**: Track claim status with immutable records (ClaimInfo struct)
- **Properties**:
  - One claim per user per market (enforced by claimed map)
  - Immutable record (append-only, no modifications)
  - Timestamp tracking (audit trail)
  - Payout verification (exact amount recorded)
- **Benefits**:
  - Safe retry mechanisms for users
  - Complete audit trail for compliance
  - Prevention of double-spend attacks
  - Front-running resistance

See [Claim Idempotency Guide](../claims/CLAIM_IDEMPOTENCY.md) for detailed implementation.

### Reentrancy Protection
- Use OWASP Application Security Verification Standard(ASVS) for the verification of security controls
- Implement servers and frameworks are running on latest versions.
- Encrypt highly sensitive information(authentication verification data)

## Infrastucture
- Monitor networks and update software and hardware regularly
- Use Web Application Firewall(WAF) that monitors HTTP traffic across Internet and blocks vulnerabilities.

## Updates
- Perform regular updates for libraries
- Use auto-scanning tools like Synk

## Access Control
- Principle of Least Priviledge(PoLP) ensures authorized users can execute jobs within the system.
- Roles based access towards some operations.

## Authentication
- Implementing strong password policies with rotation
- Implementing Multi-Factor Authentication(MFA)
- User tokens implemented during login form


---

## Idempotent Fee Collection

### Invariant

Platform fees for any given market are collected **at most once**. The
`fee_collected: bool` field on the `Market` struct is the single source of
truth.

### Implementation

`FeeManager::collect_fees` checks `market.fee_collected` before any state
mutation:

- If `true` → returns `Ok(0)` immediately (idempotent no-op).
- If `false` → validates, collects, sets `fee_collected = true`, persists.

`FeeValidator::validate_market_for_fee_collection` independently returns
`Error::FeeAlreadyCollected` for callers that use the validator directly.

### Threat model

| Threat | Mitigation |
|---|---|
| Retry on network failure double-charges users | `fee_collected` flag checked before any state write |
| Admin calls `collect_fees` twice maliciously | Second call returns `Ok(0)`, no tokens move |
| Flag reset attack | Flag is stored in persistent ledger state; only `mark_fees_collected` sets it and no pclears it post-collection |
| Reentrancy via token callback | Fee vault accumulates internally; transfer to admin only happens via time-locked `withdraw_fees` |

### Auditor checklist

- [ ] `market.fee_collected` is `false` in `Market::new` (see `types.rs`)
- [ ] `collect_fees` returns `Ok(0)` on retry, never `Err`
- [ ] `FeeValidator` returns `Error::FeeAlreadyCollected`, not `InvalidFeeConfig`
- [ ] `mark_fees_collected` is the only write path for the flag
