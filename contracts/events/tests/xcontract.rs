#![cfg(test)]
//! Cross-contract failure tests for the `events` contract.
//!
//! This test suite verifies that the events contract correctly handles
//! failures from its cross-contract callees. The contract emits events
//! around cross-contract calls (`invoke_contract` / `try_invoke_contract`)
//! and this suite pins down what happens to those emissions when the callee
//! fails — either by reverting (`panic_with_error!`) or by aborting
//! (untyped `panic!`).
//!
//! # Properties under test
//!
//! Soroban gives each contract invocation its own rollback frame. When a
//! frame fails, the host restores the storage map it captured on entry and
//! marks the events published inside that frame as belonging to a failed
//! call, so they are excluded from the transaction's event stream.
//! That yields the following guarantees an indexer depends on:
//!
//! 1. **Propagated failure erases the emission.** If the caller lets a
//!    callee failure propagate, events the caller published *before* the
//!    call are rolled back too — the whole caller frame failed. No
//!    indexer ever observes them.
//! 2. **The replay nonce is restored, not cleared.** The contract keeps a
//!    per-topic nonce in persistent storage. A failed call must leave it
//!    at its previous value, so a later successful emission does not skip a
//!    nonce and does not reuse one.
//! 3. **Handled failure preserves the caller's own events.** When the
//!    caller uses `try_invoke_contract` and recovers, its own frame
//!    succeeds: its earlier events survive, and only the callee's frame
//!    is discarded.
//! 4. **Revert and abort behave identically for rollback.** A typed
//!    `panic_with_error!` revert and an untyped `panic!` abort roll back
//!    the same way; they differ only in the error surfaced to the caller.
//!
//! # Fixtures
//!
//! The mock contracts below are test-only fixtures. [`EventContract`]
//! stands in for a production entrypoint and therefore calls
//! `require_auth` on state-changing entrypoints. The failure injectors
//! ([`FailingCallee`], [`RelayCallee`]) are deliberately minimal and skip
//! auth, since their only job is to fail in a controlled way.
//!
//! [`EventContract`]: EventsContract
//! [`FailingCallee`]: FailingCallee
//! [`RelayCallee`]: RelayCallee

use events::{
    EventsContract, EventsContractClient, EventsError, DataKey, EventPayload,
};
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, Address, Env, Symbol,
    TryFromVal, Val, Vec,
};

// ===========================================================================
// Fixtures
// ===========================================================================

/// Error raised by [`FailingCallee::revert`], used to check that a typed
/// contract error survives the cross-contract boundary intact.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum CalleeError {
    /// Deliberate failure injected by the test callee.
    Boom = 7,
}

/// A callee that fails on demand, in each way a real callee can fail.
///
/// Every failing entrypoint publishes an event and/or writes storage
/// *before* failing, so the tests can assert that those effects are
/// rolled back.
#[contract]
pub struct FailingCallee;

#[contractimpl]
impl FailingCallee {
    /// Succeeds after publishing one event. Control case.
    pub fn ok(env: Env) {
        env.events().publish((symbol_short!("callee"),), 1u32);
    }

    /// Publishes an event, then reverts with a typed contract error.
    pub fn revert(env: Env) {
        env.events().publish((symbol_short!("callee"),), 1u32);
        panic_with_error!(&env, CalleeError::Boom);
    }

    /// Publishes an event, then aborts with an untyped panic (a host trap
    /// rather than a contract error).
    pub fn abort(env: Env) {
        env.events().publish((symbol_short!("callee"),), 1u32);
        panic!("callee aborted");
    }

    /// Writes to its own persistent storage, then reverts.
    pub fn wrevert(env: Env) {
        env.storage().persistent().set(&symbol_short!("slot"), &1u32);
        panic_with_error!(&env, CalleeError::Boom);
    }
}

/// A middle contract that publishes an event and then calls a third contract,
/// used to check that rollback applies at every depth of the call stack.
#[contract]
pub struct RelayCallee;

#[contractimpl]
impl RelayCallee {
    /// Records the contract this relay will forward to.
    pub fn set_next(env: Env, next: Address) {
        env.storage().instance().set(&symbol_short!("next"), &next);
    }

    /// Publishes an event, then invokes `revert` on the recorded target,
    /// letting the failure propagate.
    pub fn relay(env: Env) {
        env.events().publish((symbol_short!("relay"),), 1u32);
        let next: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("next"))
            .expect("relay target not set");
        let _result: () =
            env.invoke_contract(&next, &symbol_short!("revert"), Vec::<Val>::new(&env));
    }
}

/// Stands in for a production entrypoint that emits events around a
/// cross-contract call. Exercises the real [`EventsContract`].
#[contract]
pub struct EventContract;

#[contractimpl]
impl EventContract {
    /// Emits `mkt_crt`, then calls `callee.func`, letting any failure propagate.
    ///
    /// # Panics
    ///
    /// Traps if the callee fails, discarding the emission along with it.
    pub fn emit_then_call(env: Env, caller: Address, callee: Address, func: Symbol) {
        caller.require_auth();
        emit_market_created(&env, &caller);
        let _result: () = env.invoke_contract(&callee, &func, Vec::<Val>::new(&env));
    }

    /// Calls `callee.func` first and emits `mkt_crt` only once it returns.
    ///
    /// Mirrors the ordering used by `EventsContract::call_and_emit`.
    ///
    /// # Panics
    ///
    /// Traps if the callee fails, before any event is emitted.
    pub fn call_then_emit(env: Env, caller: Address, callee: Address, func: Symbol) {
        caller.require_auth();
        let _result: () = env.invoke_contract(&callee, &func, Vec::<Val>::new(&env));
        emit_market_created(&env, &caller);
    }

    /// Emits `mkt_crt`, calls `callee.func` defensively, and emits
    /// `fbk_used` when the callee failed.
    ///
    /// # Returns
    ///
    /// `true` when the callee failed and the failure was recovered from,
    /// `false` when the callee succeeded.
    pub fn emit_then_try_call(env: Env, caller: Address, callee: Address, func: Symbol) -> bool {
        caller.require_auth();
        emit_market_created(&env, &caller);
        let failed = env
            .try_invoke_contract::<(), CalleeError>(&callee, &func, Vec::<Val>::new(&env))
            .is_err();
        if failed {
            emit_fallback_used(&env, &symbol_short!("fbk_used"), &caller, &caller);
        }
        failed
    }
}

/// Emit one `mkt_crt` event through the production-style emitter.
fn emit_market_created(env: &Env, admin: &Address) {
    let nonce = env
        .storage()
        .persistent()
        .get::<DataKey, u64>(&DataKey::EventNonce(symbol_short!("mkt_crt")))
        .unwrap_or(0);
    let next_nonce = nonce.saturating_add(1);
    env.storage()
        .persistent()
        .set(&DataKey::EventNonce(symbol_short!("mkt_crt")), &next_nonce);
    env.events().publish(
        (symbol_short!("mkt_crt"),),
        EventPayload {
            admin: admin.clone(),
            nonce,
            timestamp: env.ledger().timestamp(),
        },
    );
}

/// Emit a `fbk_used` event for the failure recovery path.
fn emit_fallback_used(env: &Env, topic: &Symbol, _caller: &Address, admin: &Address) {
    let nonce = env
        .storage()
        .persistent()
        .get::<DataKey, u64>(&DataKey::EventNonce(topic.clone()))
        .unwrap_or(0);
    let next_nonce = nonce.saturating_add(1);
    env.storage()
        .persistent()
        .set(&DataKey::EventNonce(topic.clone()), &next_nonce);
    env.events().publish(
        (topic,),
        EventPayload {
            admin: admin.clone(),
            nonce,
            timestamp: env.ledger().timestamp(),
        },
    );
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Number of publishes each emission helper produces.
///
/// The test emitter publishes once via a single `env.events().publish` call.
const PUBLISHES_PER_EMISSION: u32 = 1;

/// Counts the events published by `contract` whose first topic is `topic`
/// and that are still part of the transaction's event stream.
///
/// Events rolled back with a failed frame are excluded from that stream, so
/// this is the single place every "was it rolled back?" assertion goes
/// through.
fn topic_count(env: &Env, contract: &Address, topic: Symbol) -> u32 {
    let mut count = 0u32;
    let filtered = env.events().all().filter_by_contract(contract);
    for event in filtered.events().iter() {
        let soroban_sdk::xdr::ContractEventBody::V0(v0) = &event.body;
        if let Some(first) = v0.topics.get(0) {
            if let Ok(name) = Symbol::try_from_val(env, first) {
                if name == topic {
                    count += 1;
                }
            }
        }
    }
    count
}

/// Reads the persisted replay nonce for `topic` from `contract`'s storage.
///
/// Returns `None` when no emission on that topic has ever been committed.
fn nonce_of(env: &Env, contract: &Address, topic: Symbol) -> Option<u64> {
    env.as_contract(contract, || {
        env.storage()
            .persistent()
            .get::<DataKey, u64>(&DataKey::EventNonce(topic))
    })
}

/// Registers the caller and the failing callee, with auth mocked.
fn setup() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let caller = Address::generate(&env);
    let callee = env.register(FailingCallee, ());
    let contract = env.register(EventContract, ());
    (env, caller, contract, callee)
}

// ===========================================================================
// Baseline — the host semantics every assertion below relies on
// ===========================================================================

/// Characterizes the rollback semantics the rest of this file depends on:
/// events published inside a frame that fails are not part of the
/// transaction's event stream.
///
/// This is deliberately the smallest possible case — one contract, one direct
/// call, no [`EventsContract`] involved. If it fails, the SDK is surfacing
/// failed-call events through [`soroban_sdk::testutils::Events::all`] and
/// [`topic_count`] needs to filter them out; fix that one helper rather than
/// the tests that use it.
#[test]
fn rolled_back_events_are_excluded_from_the_event_stream() {
    let env = Env::default();
    env.mock_all_auths();
    let callee = env.register(FailingCallee, ());
    let client = FailingCalleeClient::new(&env, &callee);

    assert!(client.try_revert().is_err());
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        0,
        "events from a failed call must not appear in the event stream"
    );

    client.ok();
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        1,
        "events from a successful call must appear exactly once"
    );
}

// ===========================================================================
// Property 1 — a propagated callee failure erases the caller's emission
// ===========================================================================

/// When the caller emits an event and then the callee reverts, the
/// emission is rolled back with the caller's frame.
#[test]
fn callee_revert_propagates_and_discards_caller_event() {
    let (env, caller, contract, callee) = setup();

    let result = EventContractClient::new(&env, &contract)
        .try_emit_then_call(&caller, &callee, &symbol_short!("revert"));

    assert!(result.is_err(), "a reverting callee must fail the caller");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("mkt_crt")),
        0,
        "the caller's emission must be rolled back with its failed frame"
    );
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        0,
        "the callee's own event must be rolled back too"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("mkt_crt")),
        None,
        "no nonce may be committed for a rolled-back emission"
    );
}

/// When the callee aborts (untyped `panic!`), the caller's emission is still
/// rolled back — abort and revert are identical for rollback purposes.
#[test]
fn callee_abort_propagates_and_discards_caller_event() {
    let (env, caller, contract, callee) = setup();

    let result = EventContractClient::new(&env, &contract)
        .try_emit_then_call(&caller, &callee, &symbol_short!("abort"));

    assert!(result.is_err(), "an aborting callee must fail the caller");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("mkt_crt")),
        0,
        "an untyped panic must roll back the emission exactly like a revert"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("mkt_crt")),
        None,
        "an untyped panic must not commit a nonce"
    );
}

/// When the ordering is call-then-emit and the callee fails, the event is
/// never reached at all so no nonce is consumed.
#[test]
fn emit_after_call_never_emits_when_callee_fails() {
    let (env, caller, contract, callee) = setup();

    let result = EventContractClient::new(&env, &contract)
        .try_call_then_emit(&caller, &callee, &symbol_short!("revert"));

    assert!(result.is_err(), "the failure must reach the caller");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("mkt_crt")),
        0,
        "emitting after the call means the event is never reached"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("mkt_crt")),
        None,
        "no nonce is consumed when the emission is never reached"
    );
}

/// A nested failure (caller → relay → callee) rolls back every frame.
#[test]
fn nested_callee_failure_rolls_back_every_frame() {
    let env = Env::default();
    env.mock_all_auths();
    let caller = Address::generate(&env);
    let contract = env.register(EventContract, ());
    let relay = env.register(RelayCallee, ());
    let callee = env.register(FailingCallee, ());

    RelayCalleeClient::new(&env, &relay).set_next(&callee);

    let result = EventContractClient::new(&env, &contract)
        .try_emit_then_call(&caller, &relay, &symbol_short!("relay"));

    assert!(result.is_err(), "the deepest failure must reach the caller");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("mkt_crt")),
        0,
        "the caller's emission is rolled back from three frames deep"
    );
    assert_eq!(
        topic_count(&env, &relay, symbol_short!("relay")),
        0,
        "the intermediate frame's event is rolled back as well"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("mkt_crt")),
        None,
        "no nonce survives a failure at any depth"
    );
}

// ===========================================================================
// Property 2 — the replay nonce is restored, never skipped or reused
// ===========================================================================

/// After a successful emission advances the nonce, a failing call must
/// leave it at its previous value rather than clearing or advancing it.
#[test]
fn callee_failure_restores_previous_nonce() {
    let (env, caller, contract, callee) = setup();
    let client = EventContractClient::new(&env, &contract);

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    assert_eq!(nonce_of(&env, &contract, symbol_short!("mkt_crt")), Some(2));

    let result = client.try_emit_then_call(&caller, &callee, &symbol_short!("revert"));
    assert!(result.is_err());
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("mkt_crt")),
        Some(2),
        "rollback must restore the previous nonce, not clear it"
    );

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("mkt_crt")),
        PUBLISHES_PER_EMISSION,
        "only the last successful emission is observable in the current frame"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("mkt_crt")),
        Some(3),
        "a failed emission must not burn a nonce"
    );
}
/// Repeated failing callees never commit a nonce at all, no matter how many
/// times they are retried.
#[test]
fn repeated_callee_failures_do_not_advance_the_nonce() {
    let (env, caller, contract, callee) = setup();
    let client = EventContractClient::new(&env, &contract);

    for _ in 0..3 {
        assert!(client
            .try_emit_then_call(&caller, &callee, &symbol_short!("revert"))
            .is_err());
        assert_eq!(
            nonce_of(&env, &contract, symbol_short!("mkt_crt")),
            None,
            "retrying a failing callee must never commit a nonce"
        );
    }

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("mkt_crt")),
        Some(1),
        "the first committed emission must be nonce 1 regardless of prior failures"
    );
}

// ===========================================================================
// Property 3 — a handled failure keeps the caller's events, drops the callee's
// ===========================================================================

/// When the callee reverts but the caller uses `try_invoke_contract`,
/// the caller's emission and `fbk_used` event survive; the callee's event
/// and storage write are discarded.
#[test]
fn handled_callee_revert_keeps_caller_events_and_drops_callee_events() {
    let (env, caller, contract, callee) = setup();

    let recovered = EventContractClient::new(&env, &contract).emit_then_try_call(
        &caller,
        &callee,
        &symbol_short!("revert"),
    );

    assert!(recovered, "the caller must observe the callee's revert");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("mkt_crt")),
        PUBLISHES_PER_EMISSION,
        "the caller's frame succeeded, so its earlier emission survives"
    );
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("fbk_used")),
        PUBLISHES_PER_EMISSION,
        "the caller's failure-path emission must be observable"
    );
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        0,
        "the failed callee's event must not leak into the event stream"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("mkt_crt")),
        Some(1),
        "the surviving emission commits its nonce"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("fbk_used")),
        Some(1),
        "the failure-path topic keeps its own independent nonce"
    );
}

/// An untyped abort in the callee is recoverable the same way as a typed revert.
#[test]
fn handled_callee_abort_is_recovered_the_same_way() {
    let (env, caller, contract, callee) = setup();

    let recovered = EventContractClient::new(&env, &contract).emit_then_try_call(
        &caller,
        &callee,
        &symbol_short!("abort"),
    );

    assert!(
        recovered,
        "an untyped panic must be catchable just like a typed revert"
    );
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("mkt_crt")),
        PUBLISHES_PER_EMISSION,
        "the caller's emission survives a handled abort"
    );
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("fbk_used")),
        PUBLISHES_PER_EMISSION,
        "the failure path runs for an abort as well"
    );
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        0,
        "the aborted callee's event is discarded"
    );
}

/// When the callee reverts via `try_invoke_contract`, the callee's storage
/// write is rolled back even though the caller's frame succeeded.
#[test]
fn handled_callee_failure_rolls_back_callee_storage() {
    let (env, caller, contract, callee) = setup();

    let recovered = EventContractClient::new(&env, &contract).emit_then_try_call(
        &caller,
        &callee,
        &symbol_short!("wrevert"),
    );

    assert!(recovered);
    assert!(!env.as_contract(&callee, || env
        .storage()
        .persistent()
        .has(&symbol_short!("slot"))),
        "the failed callee's storage write must be rolled back"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("mkt_crt")),
        Some(1),
        "the caller's committed state is unaffected by the callee's rollback"
    );
}

// ===========================================================================
// Property 4 — success control and revert vs abort at the boundary
// ===========================================================================

/// A successful callee keeps both the caller's and the callee's events.
#[test]
fn successful_callee_keeps_both_contracts_events() {
    let (env, caller, contract, callee) = setup();

    EventContractClient::new(&env, &contract)
        .emit_then_call(&caller, &callee, &symbol_short!("ok"));

    assert_eq!(
        topic_count(&env, &contract, symbol_short!("mkt_crt")),
        PUBLISHES_PER_EMISSION,
        "a successful call keeps the caller's emission"
    );
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        1,
        "a successful callee's own event is observable"
    );
    assert_eq!(nonce_of(&env, &contract, symbol_short!("mkt_crt")), Some(1));
}

/// A succeeded callee does not trigger the failure path in the caller.
#[test]
fn handled_success_takes_no_failure_path() {
    let (env, caller, contract, callee) = setup();

    let recovered = EventContractClient::new(&env, &contract)
        .emit_then_try_call(&caller, &callee, &symbol_short!("ok"));

    assert!(!recovered, "a succeeding callee must not report failure");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("fbk_used")),
        0,
        "the failure-path event must not be emitted on success"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("fbk_used")),
        None,
        "the failure-path topic consumes no nonce on success"
    );
}

/// A typed revert surfaces as its contract error code; an untyped abort
/// surfaces as a generic host error that is distinguishable from the contract error.
#[test]
fn typed_revert_and_untyped_abort_are_distinguishable_at_the_boundary() {
    let env = Env::default();
    env.mock_all_auths();
    let callee = env.register(FailingCallee, ());
    let client = FailingCalleeClient::new(&env, &callee);
    let boom = soroban_sdk::Error::from_contract_error(CalleeError::Boom as u32);

    assert_eq!(
        client.try_revert(),
        Err(Ok(boom)),
        "a typed revert must surface as its contract error code"
    );

    let aborted = client.try_abort();
    assert!(aborted.is_err(), "an untyped panic must surface as an error");
    assert_ne!(
        aborted,
        Err(Ok(boom)),
        "an untyped panic must not be mistaken for a contract error"
    );
}

/// After two propagated failures and one handled failure, the emitter is
/// not left wedged — the next success still works and the nonce sequence
/// is correct.
#[test]
fn callee_failure_does_not_block_a_later_successful_emission() {
    let (env, caller, contract, callee) = setup();
    let client = EventContractClient::new(&env, &contract);

    assert!(client
        .try_emit_then_call(&caller, &callee, &symbol_short!("revert"))
        .is_err());
    assert!(client
        .try_emit_then_call(&caller, &callee, &symbol_short!("abort"))
        .is_err());

    // The emitter is not wedged by the prior failures.
    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));

    assert_eq!(
        topic_count(&env, &contract, symbol_short!("mkt_crt")),
        PUBLISHES_PER_EMISSION,
        "exactly one emission is observable after two propagated failures"
    );
    assert_eq!(nonce_of(&env, &contract, symbol_short!("mkt_crt")), Some(1));
}

/// A handled callee revert keeps the caller's event and rolls back only
/// the callee's frame, including the callee's storage write.
#[test]
fn handled_callee_revert_does_not_affect_caller_storage() {
    let (env, caller, contract, callee) = setup();

    let recovered = EventContractClient::new(&env, &contract).emit_then_try_call(
        &caller,
        &callee,
        &symbol_short!("revert"),
    );

    assert!(recovered);
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("mkt_crt")),
        PUBLISHES_PER_EMISSION,
        "the caller's frame succeeded so its emission is preserved"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("mkt_crt")),
        Some(1),
        "the caller's nonce was committed"
    );
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        0,
        "the callee's event was rolled back"
    );
    assert_eq!(
        nonce_of(&env, &callee, symbol_short!("callee")),
        None,
        "the callee's nonce was never committed"
    );
}

// ===========================================================================
// EventsContract — the production contract from lib.rs
// ===========================================================================

/// Register and initialize the production `EventsContract`, returning the
/// testing quartet `(env, caller, contract, callee)`.
fn setup_events() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let caller = Address::generate(&env);
    let callee = env.register(FailingCallee, ());
    let contract = env.register(EventsContract, ());
    let client = EventsContractClient::new(&env, &contract);
    client.initialize(&caller);
    (env, caller, contract, callee)
}

/// A helper to read the persisted call count from the contract.
fn call_count_of(env: &Env, contract: &Address) -> u64 {
    env.as_contract(contract, || {
        env.storage()
            .persistent()
            .get::<DataKey, u64>(&DataKey::CallCount)
            .unwrap_or(0)
    })
}

/// EventsContract::emit_and_call fails the whole frame when the callee
/// reverts; the caller's emission is rolled back.
#[test]
fn emit_and_call_returns_error_on_callee_revert() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    let result = events_contract.try_emit_and_call(
        &caller, &callee, &symbol_short!("revert"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert!(result.is_err(), "a reverting callee must fail the emit_and_call frame");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("evtopic")),
        0,
        "no event must survive a propagated rollback"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("evtopic")),
        None,
        "no nonce must be committed for a rolled-back emission"
    );
}

/// EventsContract::call_and_emit fails the whole frame when the callee
/// reverts; no event is ever emitted.
#[test]
fn call_and_emit_returns_error_on_callee_revert_no_event() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    let result = events_contract.try_call_and_emit(
        &caller, &callee, &symbol_short!("revert"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert!(result.is_err(), "a reverting callee must fail the call_and_emit frame");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("evtopic")),
        0,
        "no event was emitted when the callee failed before it"
    );
}

/// EventsContract::emit_and_call fails the whole frame when the callee
/// aborts; the caller's emission is rolled back.
#[test]
fn emit_and_call_returns_error_on_callee_abort() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    let result = events_contract.try_emit_and_call(
        &caller, &callee, &symbol_short!("abort"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert!(result.is_err(), "an aborting callee must fail the emit_and_call frame");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("evtopic")),
        0,
        "an abort must roll back the emission exactly like a revert"
    );
}

/// EventsContract::call_and_emit fails the whole frame when the callee
/// aborts; no event is ever emitted.
#[test]
fn call_and_emit_returns_error_on_callee_abort_no_event() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    let result = events_contract.try_call_and_emit(
        &caller, &callee, &symbol_short!("abort"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert!(result.is_err(), "an aborting callee must fail the call_and_emit frame");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("evtopic")),
        0,
        "an abort must prevent emission in the call-then-emit path"
    );
}

/// EventsContract::emit_then_try_call recovers from a callee revert; the
/// caller's emission survives.
#[test]
fn emit_then_try_call_recovers_from_callee_revert() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    let result = events_contract.try_emit_then_try_call(
        &caller, &callee, &symbol_short!("revert"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert!(result.is_ok(), "the caller must observe the callee's revert");
    let inner = result.unwrap();
    assert_eq!(inner, Ok(true), "the caller must recover from a revert");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("evtopic")),
        1,
        "the caller's emission survives a handled revert"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("evtopic")),
        Some(1),
        "the surviving emission commits its nonce"
    );
}

/// EventsContract::emit_then_try_call recovers from a callee abort; the
/// caller's emission survives.
#[test]
fn emit_then_try_call_recovers_from_callee_abort() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    let result = events_contract.try_emit_then_try_call(
        &caller, &callee, &symbol_short!("abort"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert!(result.is_ok(), "the caller must observe the callee's abort");
    let inner = result.unwrap();
    assert_eq!(inner, Ok(true), "the caller must recover from an abort");
    assert_eq!(
        topic_count(&env, &contract, symbol_short!("evtopic")),
        1,
        "the caller's emission survives a handled abort"
    );
}

/// EventsContract::emit_then_try_call counts a handled failure as a call.
#[test]
fn emit_then_try_call_increments_call_count_on_revert() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    events_contract.emit_then_try_call(
        &caller, &callee, &symbol_short!("ok"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert_eq!(call_count_of(&env, &contract), 0, "success does not touch call_count");

    events_contract.try_emit_then_try_call(
        &caller, &callee, &symbol_short!("revert"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert_eq!(call_count_of(&env, &contract), 1);
}

/// EventsContract::emit_then_try_call returns `false` when the callee
/// succeeds.
#[test]
fn emit_then_try_call_returns_false_on_callee_success() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    let result = events_contract.emit_then_try_call(
        &caller, &callee, &symbol_short!("ok"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert_eq!(result, false, "a succeeding callee must not report failure");
}

/// EventsContract::emit_and_call succeeds and commits the nonce + call count.
#[test]
fn emit_and_call_succeeds_and_commits_state() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    events_contract.emit_and_call(
        &caller, &callee, &symbol_short!("ok"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert_eq!(nonce_of(&env, &contract, symbol_short!("evtopic")), Some(1));
    assert_eq!(call_count_of(&env, &contract), 1);
}

/// EventsContract::call_and_emit succeeds and commits the nonce + call count.
#[test]
fn call_and_emit_succeeds_and_commits_state() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    events_contract.call_and_emit(
        &caller, &callee, &symbol_short!("ok"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert_eq!(nonce_of(&env, &contract, symbol_short!("evtopic")), Some(1));
    assert_eq!(call_count_of(&env, &contract), 1);
}

/// Nonce is restored after a failed frame from emit_and_call.
#[test]
fn emit_and_call_restores_nonce_after_callee_failure() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    events_contract.emit_then_try_call(
        &caller, &callee, &symbol_short!("ok"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert_eq!(nonce_of(&env, &contract, symbol_short!("evtopic")), Some(1));

    let _ = events_contract.try_emit_and_call(
        &caller, &callee, &symbol_short!("revert"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert_eq!(nonce_of(&env, &contract, symbol_short!("evtopic")), Some(1));
}

/// When the callee writes storage then reverts, that storage write is rolled back
/// even though the caller recovers via `try_emit_then_try_call`.
#[test]
fn handled_callee_failure_rolls_back_callee_storage_in_events_contract() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    let recovered = events_contract.try_emit_then_try_call(
        &caller, &callee, &symbol_short!("wrevert"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );

    assert!(recovered.is_ok());
    let inner = recovered.unwrap();
    assert!(inner.is_ok());
    assert!(inner.unwrap());
    assert!(!env.as_contract(&callee, || env
        .storage()
        .persistent()
        .has(&symbol_short!("slot"))),
        "the failed callee's storage write must be rolled back"
    );
    assert_eq!(
        nonce_of(&env, &contract, symbol_short!("evtopic")),
        Some(1),
        "the caller's committed state is unaffected by the callee's rollback"
    );
}

/// Call count is NOT incremented when emit_and_call's callee reverts.
#[test]
fn emit_and_call_does_not_increment_call_count_on_revert() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    let _ = events_contract.try_emit_and_call(
        &caller, &callee, &symbol_short!("revert"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert_eq!(call_count_of(&env, &contract), 0);
}

/// Call count is NOT incremented when call_and_emit's callee reverts.
#[test]
fn call_and_emit_does_not_increment_call_count_on_revert() {
    let (env, caller, contract, callee) = setup_events();
    let events_contract = EventsContractClient::new(&env, &contract);

    let _ = events_contract.try_call_and_emit(
        &caller, &callee, &symbol_short!("revert"), &symbol_short!("evtopic"), &symbol_short!("m1"),
    );
    assert_eq!(call_count_of(&env, &contract), 0);
}

#[test]
fn debug_dump_events() {
    let (env, caller, contract, callee) = setup();
    let client = EventContractClient::new(&env, &contract);

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));

    let filtered = env.events().all().filter_by_contract(&contract);
    for event in filtered.events().iter() {
        let soroban_sdk::xdr::ContractEventBody::V0(v0) = &event.body;
        if let Some(first) = v0.topics.get(0) {
            if let Ok(name) = Symbol::try_from_val(&env, first) {
                eprintln!("DEBUG EVENT: first topic = {:?}, num_topics = {}, body_type = {:?}", name, v0.topics.len(), event.type_);
            } else {
                eprintln!("DEBUG EVENT: first topic FAILED PARSE, num_topics = {}", v0.topics.len());
            }
        } else {
            eprintln!("DEBUG EVENT: no topics, body_type = {:?}", event.type_);
        }
    }
    eprintln!("DEBUG contract_id present: {:?}", env.events().all().events().iter().any(|e| e.contract_id.is_some()));
}

#[test]
fn debug_callee_failure_restores_previous_nonce() {
    let (env, caller, contract, callee) = setup();
    let client = EventContractClient::new(&env, &contract);

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    eprintln!("After 2 ok: nonce = {:?}", nonce_of(&env, &contract, symbol_short!("mkt_crt")));

    let result = client.try_emit_then_call(&caller, &callee, &symbol_short!("revert"));
    eprintln!("try_emit_then_call result is_err = {}", result.is_err());
    eprintln!("After revert: nonce = {:?}", nonce_of(&env, &contract, symbol_short!("mkt_crt")));

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    eprintln!("After final ok: nonce = {:?}", nonce_of(&env, &contract, symbol_short!("mkt_crt")));
    eprintln!("topic_count = {}", topic_count(&env, &contract, symbol_short!("mkt_crt")));
}

#[test]
fn debug_two_ok_then_dump() {
    let (env, caller, contract, callee) = setup();
    let client = EventContractClient::new(&env, &contract);

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));

    let filtered = env.events().all().filter_by_contract(&contract);
    eprintln!("Total events count = {}", filtered.events().iter().len());
    for event in filtered.events().iter() {
        let soroban_sdk::xdr::ContractEventBody::V0(v0) = &event.body;
        if let Some(first) = v0.topics.get(0) {
            if let Ok(name) = Symbol::try_from_val(&env, first) {
                eprintln!("EVENT: topic = {:?}", name);
            }
        }
    }
    eprintln!("topic_count mkt_crt = {}", topic_count(&env, &contract, symbol_short!("mkt_crt")));
}

#[test]
fn debug_all_events() {
    let (env, caller, contract, callee) = setup();
    let client = EventContractClient::new(&env, &contract);

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));

    let all = env.events().all();
    eprintln!("ALL events count = {}", all.events().iter().len());
    for event in all.events().iter() {
        let soroban_sdk::xdr::ContractEventBody::V0(v0) = &event.body;
        if let Some(first) = v0.topics.get(0) {
            if let Ok(name) = Symbol::try_from_val(&env, first) {
                eprintln!("  topic = {:?}, contract_id = {:?}", name, event.contract_id);
            } else {
                eprintln!("  topic FAILED, contract_id = {:?}", event.contract_id);
            }
        }
    }
}

#[test]
fn debug_one_vs_two_events() {
    let (env, caller, contract, callee) = setup();
    let client = EventContractClient::new(&env, &contract);

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    eprintln!("After 1: nonce = {:?}, all_count = {}", nonce_of(&env, &contract, symbol_short!("mkt_crt")), env.events().all().events().iter().len());

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    eprintln!("After 2: nonce = {:?}, all_count = {}", nonce_of(&env, &contract, symbol_short!("mkt_crt")), env.events().all().events().iter().len());

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    eprintln!("After 3: nonce = {:?}, all_count = {}", nonce_of(&env, &contract, symbol_short!("mkt_crt")), env.events().all().events().iter().len());
}

#[test]
fn debug_direct_callee_ok() {
    let env = Env::default();
    env.mock_all_auths();
    let callee = env.register(FailingCallee, ());
    let client = FailingCalleeClient::new(&env, &callee);

    client.ok();
    eprintln!("After direct ok: all_count = {}", env.events().all().events().iter().len());
}

#[test]
fn debug_single_call() {
    let (env, caller, contract, callee) = setup();
    let client = EventContractClient::new(&env, &contract);

    client.emit_then_call(&caller, &callee, &symbol_short!("ok"));
    let all = env.events().all();
    eprintln!("single call: all_count = {}", all.events().iter().len());
    for event in all.events().iter() {
        let soroban_sdk::xdr::ContractEventBody::V0(v0) = &event.body;
        if let Some(first) = v0.topics.get(0) {
            if let Ok(name) = Symbol::try_from_val(&env, first) {
                eprintln!("  topic = {:?}", name);
            }
        }
    }
}
