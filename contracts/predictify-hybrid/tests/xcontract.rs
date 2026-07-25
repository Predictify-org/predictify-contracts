#![cfg(test)]
//! Cross-contract failure tests for the event subsystem (`predictify_hybrid::events`).
//!
//! The contract emits events around cross-contract calls in several places — for
//! example [`GovernanceManager::execute_proposal`] invokes an arbitrary target and
//! only then emits its execution event, and `ReflectorOracleClient` invokes an
//! external oracle before the resolution events are emitted. This suite pins down
//! what happens to those emissions when the *callee* fails.
//!
//! # Properties under test
//!
//! Soroban gives each contract invocation its own rollback frame. When a frame
//! fails, the host restores the storage map it captured on entry and marks the
//! events published inside that frame as belonging to a failed call, so they are
//! excluded from the transaction's event stream. That yields four properties an
//! indexer depends on:
//!
//! 1. **Propagated failure erases the emission.** If the caller lets a callee
//!    failure propagate, events the caller published *before* the call are rolled
//!    back too — the whole caller frame failed. No indexer ever observes them.
//! 2. **The replay nonce is restored, not cleared.** `EventEmitter` keeps a
//!    per-topic nonce in persistent storage. A failed call must leave it at its
//!    previous value, so a later successful emission does not skip a nonce and
//!    does not reuse one.
//! 3. **Handled failure keeps the caller's own events.** When the caller uses
//!    `try_invoke_contract` and recovers, its own frame succeeds: its earlier
//!    events survive, and only the callee's frame is discarded.
//! 4. **Revert and abort behave identically for rollback.** A typed
//!    `panic_with_error!` revert and an untyped `panic!` abort roll back the same
//!    way; they differ only in the error surfaced to the caller.
//!
//! # Fixtures
//!
//! The mock contracts below are test-only fixtures. [`EventCaller`] stands in for
//! a production entrypoint and therefore does call `require_auth`; the failure
//! injectors ([`FailingCallee`], [`RelayCallee`]) are deliberately minimal and
//! skip auth, since their only job is to fail in a controlled way.
//!
//! [`GovernanceManager::execute_proposal`]: predictify_hybrid::governance

use predictify_hybrid::events::EventEmitter;
use predictify_hybrid::storage::DataKey;
use soroban_sdk::testutils::{Address as _, Events as _};
use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, symbol_short, vec, Address, Env,
    String, Symbol, TryFromVal, Val, Vec,
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
/// Every failing entrypoint publishes an event and/or writes storage *before*
/// failing, so the tests can assert that those effects are rolled back.
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
/// cross-contract call. Exercises the real [`EventEmitter`].
#[contract]
pub struct EventCaller;

#[contractimpl]
impl EventCaller {
    /// Emits `mkt_crt`, then calls `callee.func`, letting any failure propagate.
    ///
    /// # Panics
    /// Traps if the callee fails, discarding the emission along with it.
    pub fn emit_then_call(env: Env, admin: Address, callee: Address, func: Symbol) {
        admin.require_auth();
        emit_market_created(&env, &admin);
        let _result: () = env.invoke_contract(&callee, &func, Vec::<Val>::new(&env));
    }

    /// Calls `callee.func` first and emits `mkt_crt` only once it returns.
    ///
    /// Mirrors the ordering used by `GovernanceManager::execute_proposal`.
    ///
    /// # Panics
    /// Traps if the callee fails, before any event is emitted.
    pub fn call_then_emit(env: Env, admin: Address, callee: Address, func: Symbol) {
        admin.require_auth();
        let _result: () = env.invoke_contract(&callee, &func, Vec::<Val>::new(&env));
        emit_market_created(&env, &admin);
    }

    /// Emits `mkt_crt`, calls `callee.func` defensively, and emits `fbk_used`
    /// when the callee failed.
    ///
    /// # Returns
    /// `true` when the callee failed and the failure was recovered from,
    /// `false` when the callee succeeded.
    pub fn emit_then_try_call(env: Env, admin: Address, callee: Address, func: Symbol) -> bool {
        admin.require_auth();
        emit_market_created(&env, &admin);
        let failed = env
            .try_invoke_contract::<(), CalleeError>(&callee, &func, Vec::<Val>::new(&env))
            .is_err();
        if failed {
            EventEmitter::emit_fallback_used(&env, &symbol_short!("m1"), &admin, &admin);
        }
        failed
    }
}

/// Emit one `mkt_crt` event through the production emitter.
fn emit_market_created(env: &Env, admin: &Address) {
    EventEmitter::emit_market_created(
        env,
        &symbol_short!("m1"),
        &String::from_str(env, "Will it resolve?"),
        &vec![
            env,
            String::from_str(env, "Yes"),
            String::from_str(env, "No"),
        ],
        admin,
        1_000,
    );
}

// ===========================================================================
// Helpers
// ===========================================================================

/// Number of publishes each `EventEmitter::emit_*` helper produces.
///
/// The emitters publish twice per call: once via the generic `store_event`
/// archive helper, with topics `(topic,)`, and once via the typed emission,
/// with topics `(topic, id)`. Both carry `topic` as their first topic, so a
/// single successful emission is counted twice by [`topic_count`].
const PUBLISHES_PER_EMISSION: u32 = 2;

/// Counts the events published by `contract` whose first topic is `topic` and
/// that are still part of the transaction's event stream.
///
/// Events rolled back with a failed frame are excluded from that stream, so
/// this is the single place every "was it rolled back?" assertion goes
/// through.
fn topic_count(env: &Env, contract: &Address, topic: Symbol) -> u32 {
    let mut count = 0u32;
    for (source, topics, _data) in env.events().all().iter() {
        if &source != contract {
            continue;
        }
        if let Some(first) = topics.get(0) {
            if let Ok(name) = Symbol::try_from_val(env, &first) {
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
    let admin = Address::generate(&env);
    let caller = env.register(EventCaller, ());
    let callee = env.register(FailingCallee, ());
    (env, admin, caller, callee)
}

// ===========================================================================
// Baseline — the host semantics every assertion below relies on
// ===========================================================================

/// Characterizes the rollback semantics the rest of this file depends on:
/// events published inside a frame that fails are not part of the
/// transaction's event stream.
///
/// This is deliberately the smallest possible case — one contract, one direct
/// call, no `EventEmitter` involved. If it fails, the SDK is surfacing
/// failed-call events through [`soroban_sdk::testutils::Events::all`] and
/// [`topic_count`] needs to filter them out; fix that one helper rather than
/// the tests that use it.
#[test]
fn rolled_back_events_are_excluded_from_the_event_stream() {
    let env = Env::default();
    env.mock_all_auths();
    let callee = env.register(FailingCallee, ());
    let client = FailingCalleeClient::new(&env, &callee);

    // The event published just before the revert must not be observable...
    assert!(client.try_revert().is_err());
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        0,
        "events from a failed call must not appear in the event stream"
    );

    // ...while the same event from a successful call must be.
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

#[test]
fn callee_revert_propagates_and_discards_caller_event() {
    let (env, admin, caller, callee) = setup();

    let result = EventCallerClient::new(&env, &caller)
        .try_emit_then_call(&admin, &callee, &symbol_short!("revert"));

    assert!(result.is_err(), "a reverting callee must fail the caller");
    assert_eq!(
        topic_count(&env, &caller, symbol_short!("mkt_crt")),
        0,
        "the caller's emission must be rolled back with its failed frame"
    );
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        0,
        "the callee's own event must be rolled back too"
    );
    assert_eq!(
        nonce_of(&env, &caller, symbol_short!("mkt_crt")),
        None,
        "no nonce may be committed for a rolled-back emission"
    );
    assert!(
        !env.as_contract(&caller, || env
            .storage()
            .persistent()
            .has(&symbol_short!("mkt_crt"))),
        "the archived event record must be rolled back as well"
    );
}

#[test]
fn callee_abort_propagates_and_discards_caller_event() {
    let (env, admin, caller, callee) = setup();

    let result = EventCallerClient::new(&env, &caller)
        .try_emit_then_call(&admin, &callee, &symbol_short!("abort"));

    assert!(result.is_err(), "an aborting callee must fail the caller");
    assert_eq!(
        topic_count(&env, &caller, symbol_short!("mkt_crt")),
        0,
        "an untyped panic must roll back the emission exactly like a revert"
    );
    assert_eq!(
        nonce_of(&env, &caller, symbol_short!("mkt_crt")),
        None,
        "an untyped panic must not commit a nonce"
    );
}

#[test]
fn emit_after_call_never_emits_when_callee_fails() {
    let (env, admin, caller, callee) = setup();

    let result = EventCallerClient::new(&env, &caller)
        .try_call_then_emit(&admin, &callee, &symbol_short!("revert"));

    assert!(result.is_err(), "the failure must reach the caller");
    assert_eq!(
        topic_count(&env, &caller, symbol_short!("mkt_crt")),
        0,
        "emitting after the call means the event is never reached"
    );
    assert_eq!(
        nonce_of(&env, &caller, symbol_short!("mkt_crt")),
        None,
        "no nonce is consumed when the emission is never reached"
    );
}

#[test]
fn nested_callee_failure_rolls_back_every_frame() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let caller = env.register(EventCaller, ());
    let relay = env.register(RelayCallee, ());
    let callee = env.register(FailingCallee, ());
    RelayCalleeClient::new(&env, &relay).set_next(&callee);

    // caller -> relay -> callee, failing at the deepest frame.
    let result = EventCallerClient::new(&env, &caller)
        .try_emit_then_call(&admin, &relay, &symbol_short!("relay"));

    assert!(result.is_err(), "the deepest failure must reach the caller");
    assert_eq!(
        topic_count(&env, &caller, symbol_short!("mkt_crt")),
        0,
        "the caller's emission is rolled back from three frames deep"
    );
    assert_eq!(
        topic_count(&env, &relay, symbol_short!("relay")),
        0,
        "the intermediate frame's event is rolled back as well"
    );
    assert_eq!(
        nonce_of(&env, &caller, symbol_short!("mkt_crt")),
        None,
        "no nonce survives a failure at any depth"
    );
}

// ===========================================================================
// Property 2 — the replay nonce is restored, never skipped or reused
// ===========================================================================

#[test]
fn callee_failure_restores_previous_nonce() {
    let (env, admin, caller, callee) = setup();
    let client = EventCallerClient::new(&env, &caller);

    // Two successful emissions advance the nonce to 2.
    client.emit_then_call(&admin, &callee, &symbol_short!("ok"));
    client.emit_then_call(&admin, &callee, &symbol_short!("ok"));
    assert_eq!(nonce_of(&env, &caller, symbol_short!("mkt_crt")), Some(2));

    // A failing call must leave it there rather than clearing or advancing it.
    let result = client.try_emit_then_call(&admin, &callee, &symbol_short!("revert"));
    assert!(result.is_err());
    assert_eq!(
        nonce_of(&env, &caller, symbol_short!("mkt_crt")),
        Some(2),
        "rollback must restore the previous nonce, not clear it"
    );

    // The next success must take nonce 3 — no gap left by the failed attempt.
    client.emit_then_call(&admin, &callee, &symbol_short!("ok"));
    assert_eq!(
        nonce_of(&env, &caller, symbol_short!("mkt_crt")),
        Some(3),
        "a failed emission must not burn a nonce"
    );
    assert_eq!(
        topic_count(&env, &caller, symbol_short!("mkt_crt")),
        3 * PUBLISHES_PER_EMISSION,
        "only the three successful emissions may be observable"
    );
}

#[test]
fn repeated_callee_failures_do_not_advance_the_nonce() {
    let (env, admin, caller, callee) = setup();
    let client = EventCallerClient::new(&env, &caller);

    for _ in 0..3 {
        assert!(client
            .try_emit_then_call(&admin, &callee, &symbol_short!("revert"))
            .is_err());
        assert_eq!(
            nonce_of(&env, &caller, symbol_short!("mkt_crt")),
            None,
            "retrying a failing callee must never commit a nonce"
        );
    }

    // The first success still starts at 1.
    client.emit_then_call(&admin, &callee, &symbol_short!("ok"));
    assert_eq!(
        nonce_of(&env, &caller, symbol_short!("mkt_crt")),
        Some(1),
        "the first committed emission must be nonce 1 regardless of prior failures"
    );
}

// ===========================================================================
// Property 3 — a handled failure keeps the caller's events, drops the callee's
// ===========================================================================

#[test]
fn handled_callee_revert_keeps_caller_events_and_drops_callee_events() {
    let (env, admin, caller, callee) = setup();

    let recovered = EventCallerClient::new(&env, &caller).emit_then_try_call(
        &admin,
        &callee,
        &symbol_short!("revert"),
    );

    assert!(recovered, "the caller must observe the callee's revert");
    assert_eq!(
        topic_count(&env, &caller, symbol_short!("mkt_crt")),
        PUBLISHES_PER_EMISSION,
        "the caller's frame succeeded, so its earlier emission survives"
    );
    assert_eq!(
        topic_count(&env, &caller, symbol_short!("fbk_used")),
        PUBLISHES_PER_EMISSION,
        "the caller's failure-path emission must be observable"
    );
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        0,
        "the failed callee's event must not leak into the event stream"
    );
    assert_eq!(
        nonce_of(&env, &caller, symbol_short!("mkt_crt")),
        Some(1),
        "the surviving emission commits its nonce"
    );
    assert_eq!(
        nonce_of(&env, &caller, symbol_short!("fbk_used")),
        Some(1),
        "the failure-path topic keeps its own independent nonce"
    );
}

#[test]
fn handled_callee_abort_is_recovered_the_same_way() {
    let (env, admin, caller, callee) = setup();

    let recovered = EventCallerClient::new(&env, &caller).emit_then_try_call(
        &admin,
        &callee,
        &symbol_short!("abort"),
    );

    assert!(
        recovered,
        "an untyped panic must be catchable just like a typed revert"
    );
    assert_eq!(
        topic_count(&env, &caller, symbol_short!("mkt_crt")),
        PUBLISHES_PER_EMISSION,
        "the caller's emission survives a handled abort"
    );
    assert_eq!(
        topic_count(&env, &caller, symbol_short!("fbk_used")),
        PUBLISHES_PER_EMISSION,
        "the failure path runs for an abort as well"
    );
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        0,
        "the aborted callee's event is discarded"
    );
}

#[test]
fn handled_callee_failure_rolls_back_callee_storage() {
    let (env, admin, caller, callee) = setup();

    let recovered = EventCallerClient::new(&env, &caller).emit_then_try_call(
        &admin,
        &callee,
        &symbol_short!("wrevert"),
    );

    assert!(recovered);
    assert!(
        !env.as_contract(&callee, || env
            .storage()
            .persistent()
            .has(&symbol_short!("slot"))),
        "the failed callee's storage write must be rolled back"
    );
    assert_eq!(
        nonce_of(&env, &caller, symbol_short!("mkt_crt")),
        Some(1),
        "the caller's committed state is unaffected by the callee's rollback"
    );
}

// ===========================================================================
// Property 4 — success control, and revert vs abort at the boundary
// ===========================================================================

#[test]
fn successful_callee_keeps_both_contracts_events() {
    let (env, admin, caller, callee) = setup();

    EventCallerClient::new(&env, &caller).emit_then_call(&admin, &callee, &symbol_short!("ok"));

    assert_eq!(
        topic_count(&env, &caller, symbol_short!("mkt_crt")),
        PUBLISHES_PER_EMISSION,
        "a successful call keeps the caller's emission"
    );
    assert_eq!(
        topic_count(&env, &callee, symbol_short!("callee")),
        1,
        "a successful callee's own event is observable"
    );
    assert_eq!(nonce_of(&env, &caller, symbol_short!("mkt_crt")), Some(1));
}

#[test]
fn handled_success_takes_no_failure_path() {
    let (env, admin, caller, callee) = setup();

    let recovered = EventCallerClient::new(&env, &caller).emit_then_try_call(
        &admin,
        &callee,
        &symbol_short!("ok"),
    );

    assert!(!recovered, "a succeeding callee must not report failure");
    assert_eq!(
        topic_count(&env, &caller, symbol_short!("fbk_used")),
        0,
        "the failure-path event must not be emitted on success"
    );
    assert_eq!(
        nonce_of(&env, &caller, symbol_short!("fbk_used")),
        None,
        "the failure-path topic consumes no nonce on success"
    );
}

#[test]
fn typed_revert_and_untyped_abort_are_distinguishable_at_the_boundary() {
    let env = Env::default();
    env.mock_all_auths();
    let client = FailingCalleeClient::new(&env, &env.register(FailingCallee, ()));
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

#[test]
fn callee_failure_does_not_block_a_later_successful_emission() {
    let (env, admin, caller, callee) = setup();
    let client = EventCallerClient::new(&env, &caller);

    assert!(client
        .try_emit_then_call(&admin, &callee, &symbol_short!("revert"))
        .is_err());
    assert!(client
        .try_emit_then_call(&admin, &callee, &symbol_short!("abort"))
        .is_err());

    // The emitter is not left in a wedged state by the failed attempts.
    client.emit_then_call(&admin, &callee, &symbol_short!("ok"));

    assert_eq!(
        topic_count(&env, &caller, symbol_short!("mkt_crt")),
        PUBLISHES_PER_EMISSION,
        "exactly one emission is observable after two failures and one success"
    );
    assert_eq!(nonce_of(&env, &caller, symbol_short!("mkt_crt")), Some(1));
}
