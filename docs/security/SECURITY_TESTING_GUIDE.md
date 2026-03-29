# Security Test Guide

## Scope
- Contract-only security verification for `predictify-hybrid`.
- Focus on state transitions that move value or change claimability: market creation, bet placement, bet resolution, refund, fee collection, and cancellation.

## Threat Model
- A transition updates storage but does not publish a ledger event, leaving indexers blind.
- A transition emits an event before state changes are committed, creating a false audit trail.
- A transition emits the wrong status or amount, causing downstream payout or analytics errors.

## Event Invariants
- Every financially material transition must publish a ledger event and persist the same payload for internal queries.
- `mkt_crt` must reflect the created market ID, admin, outcomes, and timestamp.
- `bet_plc` must reflect the bettor, market ID, outcome, amount, and timestamp.
- `bet_upd` must reflect every change from `Active` to `Won`, `Lost`, `Refunded`, or `Cancelled`.
- `mkt_res` must be published when a market is resolved.
- Event publication must be atomic with the contract state change.

## Verification Commands
- `cargo test -p predictify-hybrid`
- `cargo test -p predictify-hybrid event_management_tests`
- `cargo test -p predictify-hybrid test_market_resolution_publishes_status_events`

## Review Checklist
- Confirm the ledger event topic matches the transition name.
- Confirm event data decodes to the expected contract type.
- Confirm the stored state matches the event payload.
- Confirm unauthorized or invalid transitions emit no success event.

## Non-Goals
- This guide does not cover frontend indexing pipelines.
- This guide does not define retention or archival policy for old events.
- This guide does not replace contract-level authorization or balance checks.

## Notes For Integrators
- Prefer consuming ledger events first.
- Use persistent storage only for reconciliation or backfill.
- Treat missing events as a failed contract release until the regression tests pass.
