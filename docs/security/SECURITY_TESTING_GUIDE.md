# Security Testing Guide: Executable Checklist

This guide provides a comprehensive security testing checklist for the Predictify Hybrid smart contracts. Each item is mapped to specific automated tests to ensure continuous security validation.

## 1. Administrative & Access Control

Verify that sensitive administrative functions are properly protected and only accessible by authorized addresses.

- [ ] **Initialization Guard**: Contract can only be initialized once.
  - **Test**: `test_initialize_already_initialized` in `lib.rs`
- [ ] **Admin Authentication**: Market creation and management require admin authorization.
  - **Test**: `test_unauthorized_add_admin` in `multi_admin_multisig_tests.rs`
- [ ] **Multisig Enforcement**: Sensitive operations respect the configured M-of-N threshold.
  - **Test**: `test_2_of_3_multisig_workflow` in `multi_admin_multisig_tests.rs`

## 2. Oracle Security

Validate the integrity and reliability of price oracle integrations.

- [ ] **Provider Validation**: Only supported oracle providers (e.g., Reflector) can be configured.
  - **Test**: `test_oracle_provider_validation` in `oracle_security_tests.rs`
- [ ] **Signature Verification**: Rejection of malicious or invalid oracle signatures.
  - **Test**: `test_invalid_signature_rejection` in `oracle_security_tests.rs`
- [ ] **Whitelist Enforcement**: Only whitelisted oracle contracts can resolve markets.
  - **Test**: `test_oracle_whitelist_validation` in `oracle_security_tests.rs`
- [ ] **Freshness Checks**: Rejection of stale or outdated oracle data.
  - **Test**: `test_oracle_health_check_manipulation` in `oracle_security_tests.rs`

## 3. Financial Integrity & Invariants

Verify mathematical correctness and prevent common financial vulnerabilities.

- [ ] **Payout Accuracy**: Total distributed winnings must equal the total pool (minus fees).
  - **Test**: `prop_distribute_payouts_total_pool` in `property_based_tests.rs`
- [ ] **Double-Claim Protection**: Users cannot claim winnings multiple times for the same market.
  - **Test**: `test_double_claim_prevention` in `executable_checklist_tests.rs`
- [ ] **Fee Deduction**: Platform fees are calculated and deducted exactly according to configuration.
  - **Test**: `prop_fee_deduction_accuracy` in `property_based_tests.rs`
- [ ] **Zero-Winner Safety**: Proper handling of markets with no winners (funds remains in pool or handles as per spec).
  - **Test**: `test_zero_winner_scenario` in `executable_checklist_tests.rs`

## 4. Market Lifecycle & State Machine

Ensure markets transition correctly between states and enforce logical constraints at each stage.

- [ ] **State Transitions**: Valid transitions between Active -> Ended -> Resolving -> Resolved.
  - **Test**: `test_market_state_transitions` in `executable_checklist_tests.rs`
- [ ] **Dispute Window Enforcement**: Resolution is only finalized after the dispute period expires.
  - **Test**: `test_dispute_window_enforcement` in `resolution_delay_dispute_window_tests.rs`
- [ ] **Late Resolution Prevention**: Markers cannot be resolved after the resolution timeout.
  - **Test**: `test_resolution_timeout_enforcement` in `oracle_fallback_timeout_tests.rs`

## 5. Audit & Integrity

Ensure system transparency and tamper-evident logging.

- [ ] **Audit Trail Integrity**: All critical actions are recorded, and the trail is cryptographicly linked.
  - **Test**: `test_audit_trail_integrity` in `test_audit_trail.rs`

## Execution

Run all security-related tests using the following command:

```bash
cargo test -p predictify-hybrid --test security
```

For property-based tests:

```bash
cargo test -p predictify-hybrid --test property_based_tests
```
