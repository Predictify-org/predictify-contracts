# PR: Add Invariant Proptest for Disputes (v7)

## Issue

Closes #940

## Summary

This PR adds comprehensive property-based (proptest) invariant tests for the dispute module (v7) in the Predictify Hybrid contract. The tests verify that core dispute invariants hold across a wide range of randomly generated inputs, catching edge cases and regression bugs that traditional unit tests might miss.

## Changes

### New File: `contracts/predictify-hybrid/src/tests/dispute_proptest.rs`

Added 18 property-based tests covering the following dispute invariants:

#### 1. Voting Outcome Determinism
- For any `(support, against)` stake pair, `calculate_stake_weighted_outcome` returns the same result on repeated calls.

#### 2. Tie Resolution
- Equal support and against stakes always resolve to `false` (oracle result stands; admin escalation per docs).

#### 3. Monotonicity in Support Stake
- Increasing support stake (holding against constant) can only change the outcome from `false` to `true`, never the reverse.

#### 4. Empty Vote Set Safety
- Zero stakes on both sides never panic and resolve to `false`.

#### 5. Stake Validation — Below Minimum Rejected
- Dispute stakes below `MIN_DISPUTE_STAKE` are rejected by `DisputeValidator::validate_dispute_parameters`.

#### 6. Stake Validation — At/Above Minimum Passes
- Dispute stakes at or above `MIN_DISPUTE_STAKE` pass validation when all other conditions are met.

#### 7. Fee Distribution Bounds
- The winner's payout is always >= their original stake and <= total staked.
- Total distributed fees never exceed total staked.

#### 8. Dispute Window Validation
- Markets that have ended and are past the dispute window are rejected for new disputes.

#### 9. Resolved Markets Reject New Disputes
- Markets that already have `winning_outcomes` set are rejected for new disputes.

#### 10. Cooldown Enforcement
- Admin state-changing actions within the cooldown period are rejected; actions after the cooldown expire succeed.

#### 11. Dispute Timeout Parameter Validation
- Timeout hours within valid range (1..=720) pass validation.
- Zero timeout hours are rejected.
- Excessive timeout hours (>720) are rejected.

#### 12. Dispute Escalation Requires Participation
- Only users who have voted on a dispute can escalate it.

#### 13. Dispute Voting Active Period
- Voting before the start time is rejected.

#### 14. Stake Decay Calculation Safety
- `tally_votes` never panics and always returns a value <= raw stake and >= 0.

#### 15. Dispute History Cap Enforcement
- After eviction, history length never exceeds the configured cap.

#### 16. Market Dispute Stake Floor Enforcement
- Stakes below the market-specific dispute stake floor are rejected.

#### 17. Market Dispute Stake Floor Passes At/Above
- Stakes at or above the market-specific floor pass validation.

#### 18. Decay Config Roundtrip
- Setting a decay config and retrieving it yields consistent values.

#### 19. Timeout Lifecycle
- Setting, retrieving, extending, and removing a dispute timeout is consistent.

#### 20. Dispute Outcome Consistency
- When support > against, outcome is true; when support < against, outcome is false; when support == against (tie), outcome is false.

#### 21. Dispute Voting Roundtrip
- Storing and retrieving dispute voting data yields consistent results.

## Test Setup

The tests follow the existing patterns in the codebase:
- `Env::default()` for creating a Soroban test environment
- `env.mock_all_auths()` for authorizing admin actions
- `env.register(PredictifyHybrid, ())` for deploying the contract
- `env.as_contract(&contract_id, ...)` for executing in contract context
- `proptest!` macro for property-based test definitions

## Coverage

The proptest module provides broad coverage of dispute state invariants:
- Voting outcome logic (determinism, monotonicity, tie-breaking)
- Stake validation (minimum, floor, caps)
- Fee distribution (bounds, reconciliation)
- State transitions (window validation, resolved market rejection)
- Admin controls (cooldown, history cap)
- Timeout lifecycle (parameters, extension, removal)
- Decay configuration (roundtrip consistency)

## Verification

The implementation follows the repo's coding standards:
- NatSpec-style `///` rustdoc on all public items
- No `unwrap()` in production paths (only in test setup where appropriate)
- Overflow-safe math using `checked_add`, `saturating_sub`, etc.
- `require_auth` on every state-changing entrypoint (enforced by the contract's existing design)
- Follows existing proptest patterns from `fee_calculator_proptest.rs`

## Acceptance Criteria

- [x] Implementation matches the description
- [x] Proptest invariants added for dispute state
- [x] Code follows repo's lint and code style
- [x] NatSpec-style rustdoc documentation
- [x] Overflow-safe math used throughout
- [x] `require_auth` on state-changing entrypoints (inherited from contract design)
closes #940
