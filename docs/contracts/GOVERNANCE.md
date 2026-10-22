# Governance Lifecycle (`predictify-hybrid`)

This document specifies proposal lifecycle behavior for `contracts/predictify-hybrid/src/governance.rs`.

## Scope

- Proposal creation and storage
- Voting windows and vote accounting — including commit-reveal (salted votes)
- Quorum, quorum decay, and pass/fail conditions
- Proposal execution semantics
- Delegation
- Security invariants and explicit non-goals

## Lifecycle

### 1) Initialize

`GovernanceContract::initialize(env, admin, voting_period_seconds, quorum_votes, quorum_decay)`

- One-time, idempotent setup.
- Rejects invalid config where:
  - `voting_period_seconds <= 0`
  - `quorum_votes == 0`
  - `quorum_decay.floor_bps > 10000` or `quorum_decay.halving_seconds == 0`
- Stores:
  - admin address
  - voting period (seconds)
  - quorum threshold (`for_votes` minimum)
  - optional `QuorumDecay` configuration
  - empty proposal list

### 2) Create Proposal

`GovernanceContract::create_proposal(...) -> Result<Symbol, GovernanceError>`

A proposal is accepted only when all checks pass:

- caller authorization: `proposer.require_auth()`
- governance initialized
- unique proposal id
- non-empty `title` and `description`
- execution fields coherent:
  - both `target` and `call_fn` set, or
  - both omitted (no-op execution)

A proposal starts immediately at current ledger timestamp and ends at `start + voting_period`.

### 3a) Direct Vote

`GovernanceContract::vote(env, voter, proposal_id, support) -> Result<(), GovernanceError>`

Voting rules:

- caller authorization: `voter.require_auth()`
- one address, one vote per proposal (includes revealed commit-reveal votes)
- voting window: `[start_time, end_time)`
- rejected if proposal already executed

### 3b) Commit-Reveal Vote (Salted)

Two-step process that prevents front-running and precomputation of vote outcomes:

#### Commit Phase

`GovernanceContract::commit_vote(env, voter, proposal_id, commitment) -> Result<(), GovernanceError>`

- `commitment = sha256(salt ++ support_byte)` where `support_byte = 0x01` (FOR) or `0x00` (AGAINST)
- Stored on-chain; vote preference is hidden until reveal.
- One commitment per `(proposal, voter)` pair — cannot recommit.
- Rejects if voter already cast a direct vote.
- Voting window must be open.
- Emits `GovernanceVoteCommittedEvent`.

#### Reveal Phase

`GovernanceContract::reveal_vote(env, voter, proposal_id, salt, support) -> Result<(), GovernanceError>`

- Recomputes `sha256(salt ++ support_byte)` and verifies against stored commitment.
- Tallies vote with delegation weight on match.
- Returns `InvalidReveal` if salt or support do not match the commitment.
- Returns `NoCommitment` if no commitment exists.
- Voting window must still be open.
- Removes commitment from storage after reveal (prevents double-reveal).
- Re-uses `GovernanceVoteCastEvent` for the actual vote record.

> **Security property**: Because voters only publish a hash, observers cannot determine vote
> direction during the commit window.  Votes become legible only after reveal, protecting
> against strategic last-second voting based on observed tally trends.

### 4) Validate

`GovernanceContract::validate_proposal(...) -> Result<(bool, String), GovernanceError>`

Validation availability and outcomes:

- reject while voting still active (`VotingStillActive`) when `now < end_time`
- uses *effective quorum* (see Quorum Decay below)
- fail when `for_votes < effective_quorum` (`"quorum not reached"`)
  - if `for_votes < floor_quorum` an auto-rejection event is emitted
- fail when `for_votes <= against_votes` (`"not enough for votes"`)
- pass otherwise (`"passed"`)

### 5) Execute / Fail

`GovernanceContract::execute_proposal(...) -> Result<(), GovernanceError>`

Execution rules:

- caller authorization: `caller.require_auth()`
- proposal must exist and not already be executed
- proposal must validate as passed
- execution mode:
  - no-op execution when both `target` and `call_fn` are absent
  - contract invocation when both are present (`invoke_contract` with no args)
- marks proposal as executed and emits execution event on success

If validation fails (quorum or vote majority), execution returns `NotPassed`.

## Quorum Decay

When initialized or configured with a `QuorumDecay` object, the effective quorum required for a proposal to pass decreases linearly over the proposal lifetime:

```
effective_quorum(elapsed) = base_quorum - (base_quorum - floor) * elapsed / (2 * halving_seconds)
```

- `floor = base_quorum * floor_bps / 10000`
- Decay is complete (quorum equals floor) after `elapsed >= 2 * halving_seconds`
- Quorum is monotone non-increasing

### Fields

| Field | Type | Meaning |
|-------|------|---------|
| `floor_bps` | `u32` | Floor as basis points of base quorum (≤ 10 000) |
| `halving_seconds` | `u64` | Seconds to reach midpoint quorum (must be > 0) |

### Admin API

| Function | Description |
|----------|-------------|
| `set_quorum_decay(env, admin, decay)` | Set or disable decay config |
| `get_quorum_decay(env)` | Read current decay config |

## Delegation

| Function | Description |
|----------|-------------|
| `set_delegate(env, delegator, delegate)` | Delegate vote weight to another address |
| `unset_delegate(env, delegator)` | Remove active delegation |
| `get_delegate(env, delegator)` | Query current delegate |

Rules:
- Each delegator holds at most **one** active delegation.
- A delegate accepts at most **50** incoming delegations (griefing guard).
- Self-delegation and two-hop cycles (A→B while B→A exists) are rejected.
- Vote weight = `1 + incoming_delegation_count`.

## Events

| Symbol | Struct | Trigger |
|--------|--------|---------|
| `gov_prop` | `GovernanceProposalCreatedEvent` | Proposal created |
| `gov_cmit` | `GovernanceVoteCommittedEvent` | Commit phase of commit-reveal |
| `gov_vote` | `GovernanceVoteCastEvent` | Direct vote or revealed commit-reveal vote |
| `gov_exec` | `GovernanceProposalExecutedEvent` | Proposal executed |
| `gov_rej` | `GovernanceProposalAutoRejectedEvent` | Proposal auto-rejected (below floor quorum) |

## Error Reference

| Error | Cause |
|-------|-------|
| `ProposalExists` | Duplicate proposal id |
| `ProposalNotFound` | Unknown proposal id |
| `VotingNotStarted` | `now < start_time` |
| `VotingEnded` | `now >= end_time` |
| `VotingStillActive` | Validate/execute called before `end_time` |
| `AlreadyVoted` | Voter already tallied |
| `NotPassed` | Proposal failed validation |
| `AlreadyExecuted` | Execute called twice |
| `NotAdmin` | Non-admin called admin function |
| `NotInitialized` | Governance not initialized |
| `InvalidParams` | Zero/invalid config value |
| `SelfDelegation` | Delegator == delegate |
| `DelegationAlreadySet` | Delegator already has an active delegation |
| `DelegateLimitReached` | Delegate at incoming-delegation cap |
| `NoDelegationSet` | Unset called with no active delegation |
| `DelegationCycle` | Two-hop cycle detected |
| `NoCommitment` | Reveal called without a prior commit |
| `InvalidReveal` | Hash mismatch — wrong salt or support value |
| `CommitmentExists` | Second commit by same (proposal, voter) |

## Security Notes

### Threat Model (covered)

- Unauthorized state changes via forged caller identities
- Duplicate voting (both direct and commit-reveal paths are guarded)
- Front-running / vote precomputation (mitigated by commit-reveal)
- Parameter misuse causing ambiguous execution paths
- Configuration mistakes that disable governance constraints
- Premature execution before voting window closes
- Delegation griefing (cap on incoming delegations + one-delegation-per-delegator limit)

### Invariants Enforced

- `voting_period_seconds > 0`
- `quorum_votes > 0`
- Proposal IDs are unique
- Each `(proposal_id, voter)` pair contributes at most one vote to the tally
- Voting only occurs in `[start_time, end_time)`
- Execution occurs at most once per proposal
- Passing requires both:
  - `for_votes >= effective_quorum`
  - `for_votes > against_votes`
- Commit-reveal: commitment is removed after reveal, preventing double-count

### Explicit Non-Goals

- Token-weighted voting (all votes are weight-1 + delegations)
- Vote revocation after cast
- Batched/multi-call proposal payloads
- Snapshot-based historical balance governance
- Timelock queue semantics after passing

## Regression Coverage

Implemented in `contracts/predictify-hybrid/src/governance_tests.rs`:

**Lifecycle tests:**
- Create → vote → execute success path
- Quorum failure path
- Tie/majority failure path
- Timing failures (before start, after end, execute while active)
- Duplicate vote rejection
- Invalid create/admin parameter handling
- Uninitialized governance rejection

**Quorum decay tests:**
- `compute_effective_quorum` values at 0%, 25%, 50%, 75%, 100% elapsed
- Proposal passes with decayed quorum
- Floor respected after full decay
- Auto-rejection event when below floor
- Admin can configure and disable decay
- Non-admin decay config rejected
- Invalid decay params rejected
- Monotone non-increasing property

**Vote salt (commit-reveal) tests:**
- Full commit → reveal → validate lifecycle
- Wrong salt rejected (`InvalidReveal`)
- Wrong support value rejected (`InvalidReveal`)
- Duplicate commit rejected (`CommitmentExists`)
- Reveal without commit rejected (`NoCommitment`)
- Commit after voting ends rejected (`VotingEnded`)
- Reveal after voting ends rejected (`VotingEnded`)
- Double-reveal rejected (`AlreadyVoted`)
- Direct vote then commit rejected (`AlreadyVoted`)
- Commit for nonexistent proposal rejected (`ProposalNotFound`)
