# Governance Lifecycle (`predictify-hybrid`)

This document specifies proposal lifecycle behavior for `contracts/predictify-hybrid/src/governance.rs`.

## Scope

- Proposal creation and storage
- Voting windows and vote accounting
- Quorum and pass/fail conditions
- Proposal execution semantics
- Security invariants and explicit non-goals

## Lifecycle

### 1) Initialize

`GovernanceContract::initialize(env, admin, voting_period_seconds, quorum_votes)`

- One-time, idempotent setup.
- Rejects invalid config where:
  - `voting_period_seconds <= 0`
  - `quorum_votes == 0`
- Stores:
  - admin address
  - voting period (seconds)
  - quorum threshold (`for_votes` minimum)
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

### 3) Vote

`GovernanceContract::vote(...) -> Result<(), GovernanceError>`

Voting rules:

- caller authorization: `voter.require_auth()`
- one address, one vote per proposal
- voting window is **inclusive start / exclusive end**:
  - reject if `now < start_time` (`VotingNotStarted`)
  - reject if `now >= end_time` (`VotingEnded`)
- reject if proposal already executed

### 4) Validate

`GovernanceContract::validate_proposal(...) -> Result<(bool, String), GovernanceError>`

Validation availability and outcomes:

- reject while voting still active (`VotingStillActive`) when `now < end_time`
- fail when `for_votes < quorum_votes` (`"quorum not reached"`)
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

## Security Notes

### Threat Model (covered)

- Unauthorized state changes via forged caller identities
- Duplicate voting by same address
- Parameter misuse that can cause ambiguous execution paths
- Configuration mistakes that disable governance constraints
- Premature execution before voting window closes

### Invariants Enforced

- `voting_period_seconds > 0`
- `quorum_votes > 0`
- Proposal IDs are unique
- Votes are single-use per `(proposal_id, voter)`
- Voting only occurs in `[start_time, end_time)`
- Execution occurs at most once per proposal
- Passing requires both:
  - `for_votes >= quorum_votes`
  - `for_votes > against_votes`

### Explicit Non-Goals

- Weighted or token-based voting
- Delegation and vote revocation
- Batched/multi-call proposal payloads
- Snapshot-based historical balance governance
- Timelock queue semantics after passing

## Regression Coverage

Implemented in `contracts/predictify-hybrid/src/governance_tests.rs`:

- Create → vote → execute success path
- Quorum failure path
- Tie/majority failure path
- Timing failures:
  - vote before start
  - vote at/after end
  - execute while voting still active
- Duplicate vote rejection
- Invalid create/admin parameter handling
- Uninitialized governance rejection
