# Governance Contract

A minimal, self-contained on-chain governance module for Predictify with
**structured lifecycle events** for off-chain indexers.

Accounts create proposals, cast weighted votes during a bounded voting window,
and proposals are finalized (executed or rejected) once voting closes. Every
lifecycle transition emits a typed event with a **stable topic symbol** so that
indexers can filter, decode, and replay governance state deterministically.

## Lifecycle

```text
                      cast_vote (many)
                     ┌──────────────┐
                     ▼              │
create_proposal → Active ──────────┘
                     │
       ┌─────────────┼──────────────┐
       ▼ execute     ▼ execute       ▼ cancel
   Executed       Rejected        Canceled
```

A proposal starts `Active` and moves to exactly one terminal state:
`Executed`, `Rejected`, or `Canceled`.

## Structured lifecycle events

The first topic of every event is a stable `Symbol`. Identifying fields (actor
address, proposal id) are placed in **topics** so indexers can subscribe by key
without decoding the payload; quantitative fields go in the **data** payload.
In addition to each action-specific event, **every** status transition also
emits a generic `gov_status` event carrying `(old_status, new_status)`, so an
indexer that only tracks lifecycle state can rely on that single stream.

| Topic          | Topics tuple                          | Data payload                                          | Emitted when                        |
|----------------|---------------------------------------|-------------------------------------------------------|-------------------------------------|
| `gov_init`     | `(gov_init, admin)`                   | `(voting_period, timestamp)`                          | Contract initialized                |
| `gov_created`  | `(gov_created, proposer, proposal_id)`| `(title, voting_ends_at, timestamp)`                  | Proposal created                    |
| `gov_voted`    | `(gov_voted, voter, proposal_id)`     | `(choice, weight, votes_for, votes_against, timestamp)`| Vote cast                          |
| `gov_executed` | `(gov_executed, executor, proposal_id)`| `(votes_for, votes_against, timestamp)`              | Passing proposal executed           |
| `gov_rejected` | `(gov_rejected, proposal_id)`         | `(votes_for, votes_against, timestamp)`               | Proposal finalized as rejected      |
| `gov_canceled` | `(gov_canceled, caller, proposal_id)` | `timestamp`                                           | Proposal canceled                   |
| `gov_status`   | `(gov_status, proposal_id)`           | `(old_status, new_status, timestamp)`                 | Any proposal status change (generic)|
| `gov_admin_xf` | `(gov_admin_xf, previous_admin, new_admin)`| `timestamp`                                      | Admin transferred                   |

These topic strings are part of the contract's public API and never change.

## Entrypoints

| Function            | Auth                          | Description                                              |
|---------------------|-------------------------------|----------------------------------------------------------|
| `initialize`        | admin                         | One-time init with admin + default voting period (secs). |
| `create_proposal`   | proposer                      | Create a proposal; returns its id.                       |
| `cast_vote`         | voter                         | Cast a weighted `For`/`Against` vote (once per account). |
| `execute_proposal`  | any (permissionless)          | Finalize after voting closes → `Executed` or `Rejected`. |
| `cancel_proposal`   | proposer **or** admin         | Cancel an active proposal.                               |
| `transfer_admin`    | current admin                 | Transfer admin role.                                     |
| `version`           | none (read-only)              | Contract version (`u32`).                                |
| `get_admin`         | none (read-only)              | Current admin address.                                   |
| `get_proposal`      | none (read-only)              | Fetch a proposal by id.                                  |
| `has_voted`         | none (read-only)              | Whether an address has voted on a proposal.              |

Every state-changing entrypoint calls `require_auth()` on its acting principal.
Arithmetic on ids, deadlines, and vote tallies is overflow-safe (`checked_add`),
and production paths contain no `unwrap()`.

## Errors

Typed `#[contracterror]` codes (stable, never reassigned): `Unauthorized (1)`,
`NotInitialized (2)`, `AlreadyInitialized (3)`, `ProposalNotFound (4)`,
`InvalidStateTransition (5)`, `AlreadyVoted (6)`, `VotingOpen (7)`,
`VotingClosed (8)`, `InvalidVotingPeriod (9)`, `Overflow (10)`.

## Tests

- `tests/governance_lifecycle.rs` — creation, voting, execution/rejection,
  cancellation, admin transfer, and every guard rail.
- `tests/events.rs` — asserts each lifecycle event's stable topic and decodes
  event payloads to verify their fields.
- `tests/auth_boundary.rs` — asserts `require_auth()` is enforced on the correct
  principal and that role-gated entrypoints reject the wrong caller.

Run with `cargo test -p governance`.
