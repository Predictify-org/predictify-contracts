//! Unit tests for [`governance_events::events`].
//!
//! Every test runs inside a synthetic Soroban environment created with
//! `Env::default()`.  Because the event structs are `#[contracttype]` they
//! can only be constructed and published while a contract context is active;
//! each test therefore registers [`GovernanceEventsContract`] and calls into it
//! via `env.as_contract(...)`.
//!
//! # Coverage goals
//! - Every `emit_*` helper fires without panicking.
//! - The published event is visible in `env.events().all()`.
//! - Topic symbols match the documented values (see events.rs header table).
//! - Nonces are monotone: two consecutive emits on the same topic produce 1, 2.
//! - `emit_quorum_decay_updated` correctly encodes both enabled and disabled states.
//! - `emit_delegate_set` / `emit_delegate_unset` round-trip addresses faithfully.
//! - `emit_registry_param_proposed` carries the correct value and key.
//! - Emitted timestamps match the ledger timestamp at call time.
//! - The full happy-path and commit-reveal lifecycle sequences produce events.

#![cfg(test)]
extern crate std;

use crate::{events::GovernanceEventEmitter, GovernanceEventsContract};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, String, Symbol,
};

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Create a default test environment and register the minimal contract stub.
fn setup() -> (Env, Address) {
    let env = Env::default();
    let id = env.register(GovernanceEventsContract, ());
    (env, id)
}

/// Count published events whose first topic equals `topic`.
fn count_topic(env: &Env, topic: &Symbol) -> usize {
    env.events()
        .all()
        .iter()
        .filter(|(_, topics, _)| {
            topics
                .iter()
                .any(|t| t == topic.clone().into_val(env))
        })
        .count()
}

// ─── proposal lifecycle ───────────────────────────────────────────────────────

#[test]
fn emit_proposal_created_publishes_gov_prop_event() {
    let (env, id) = setup();
    let admin = Address::generate(&env);
    let pid = Symbol::new(&env, "prop_01");
    let title = String::from_str(&env, "Enable fee reduction");
    let desc = String::from_str(&env, "Reduce platform fee by 10 bps");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_proposal_created(&env, &pid, &admin, &title, &desc);
    });

    assert_eq!(
        count_topic(&env, &symbol_short!("gov_prop")),
        1,
        "exactly one gov_prop event must be published"
    );
}

#[test]
fn emit_vote_cast_for_publishes_gov_vote_event() {
    let (env, id) = setup();
    let voter = Address::generate(&env);
    let pid = Symbol::new(&env, "prop_02");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_vote_cast(&env, &pid, &voter, true, 3);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_vote")), 1);
}

#[test]
fn emit_vote_cast_against_publishes_gov_vote_event() {
    let (env, id) = setup();
    let voter = Address::generate(&env);
    let pid = Symbol::new(&env, "prop_02b");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_vote_cast(&env, &pid, &voter, false, 1);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_vote")), 1);
}

#[test]
fn emit_vote_cast_weight_zero_does_not_panic() {
    // weight = 0 is unusual but must never panic.
    let (env, id) = setup();
    let voter = Address::generate(&env);
    let pid = Symbol::new(&env, "prop_w0");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_vote_cast(&env, &pid, &voter, false, 0);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_vote")), 1);
}

#[test]
fn emit_vote_committed_publishes_gov_cmit_event() {
    let (env, id) = setup();
    let voter = Address::generate(&env);
    let pid = Symbol::new(&env, "prop_03");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_vote_committed(&env, &pid, &voter);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_cmit")), 1);
}

#[test]
fn emit_vote_revealed_publishes_gov_rvl_event() {
    let (env, id) = setup();
    let voter = Address::generate(&env);
    let pid = Symbol::new(&env, "prop_04");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_vote_revealed(&env, &pid, &voter, true, 2);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_rvl")), 1);
}

#[test]
fn emit_proposal_executed_publishes_gov_exec_event() {
    let (env, id) = setup();
    let executor = Address::generate(&env);
    let pid = Symbol::new(&env, "prop_05");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_proposal_executed(&env, &pid, &executor);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_exec")), 1);
}

#[test]
fn emit_proposal_cancelled_publishes_gov_canc_event() {
    let (env, id) = setup();
    let admin = Address::generate(&env);
    let pid = Symbol::new(&env, "prop_06");
    let reason = String::from_str(&env, "spam proposal");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_proposal_cancelled(&env, &pid, &admin, &reason);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_canc")), 1);
}

#[test]
fn emit_proposal_auto_rejected_publishes_gov_rej_event() {
    let (env, id) = setup();
    let proposer = Address::generate(&env);
    let pid = Symbol::new(&env, "prop_07");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_proposal_auto_rejected(&env, &pid, &proposer, 1, 5);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_rej")), 1);
}

// ─── nonce monotonicity ───────────────────────────────────────────────────────

#[test]
fn nonces_are_strictly_increasing_within_same_topic() {
    use crate::events::ProposalCreatedEvent;

    let (env, id) = setup();
    let admin = Address::generate(&env);
    let t = String::from_str(&env, "T");
    let d = String::from_str(&env, "D");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_proposal_created(
            &env,
            &Symbol::new(&env, "n_a"),
            &admin,
            &t,
            &d,
        );
        GovernanceEventEmitter::emit_proposal_created(
            &env,
            &Symbol::new(&env, "n_b"),
            &admin,
            &t,
            &d,
        );
    });

    let mut nonces: std::vec::Vec<u64> = env
        .events()
        .all()
        .iter()
        .filter_map(|(_, topics, data)| {
            if topics
                .iter()
                .any(|t| t == symbol_short!("gov_prop").into_val(&env))
            {
                let ev: ProposalCreatedEvent = data.into_val(&env);
                Some(ev.nonce)
            } else {
                None
            }
        })
        .collect();

    nonces.sort_unstable();
    assert_eq!(nonces.len(), 2, "expected exactly 2 gov_prop events");
    assert_eq!(nonces[0], 1, "first nonce must be 1");
    assert_eq!(nonces[1], 2, "second nonce must be 2");
}

#[test]
fn different_topics_have_independent_nonce_counters() {
    use crate::events::{ProposalCreatedEvent, VoteCastEvent};

    let (env, id) = setup();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let pid = Symbol::new(&env, "ind_p");
    let t = String::from_str(&env, "T");
    let d = String::from_str(&env, "D");

    env.as_contract(&id, || {
        // emit two gov_prop events — their nonces should be 1, 2
        GovernanceEventEmitter::emit_proposal_created(&env, &pid, &admin, &t, &d);
        GovernanceEventEmitter::emit_proposal_created(&env, &pid, &admin, &t, &d);
        // then one gov_vote event — its nonce must start at 1, not 3
        GovernanceEventEmitter::emit_vote_cast(&env, &pid, &voter, true, 1);
    });

    let vote_nonces: std::vec::Vec<u64> = env
        .events()
        .all()
        .iter()
        .filter_map(|(_, topics, data)| {
            if topics
                .iter()
                .any(|t| t == symbol_short!("gov_vote").into_val(&env))
            {
                let ev: VoteCastEvent = data.into_val(&env);
                Some(ev.nonce)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(vote_nonces, [1u64], "vote nonce counter must be independent (starts at 1)");
}

// ─── configuration changes ────────────────────────────────────────────────────

#[test]
fn emit_voting_period_updated_publishes_event() {
    let (env, id) = setup();
    let admin = Address::generate(&env);

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_voting_period_updated(&env, &admin, 3_600, 7_200);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_vp_upd")), 1);
}

#[test]
fn emit_quorum_updated_publishes_event() {
    let (env, id) = setup();
    let admin = Address::generate(&env);

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_quorum_updated(&env, &admin, 100, 200);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_qrm")), 1);
}

#[test]
fn emit_quorum_decay_updated_enabled_encodes_correctly() {
    use crate::events::QuorumDecayUpdatedEvent;

    let (env, id) = setup();
    let admin = Address::generate(&env);

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_quorum_decay_updated(
            &env,
            &admin,
            Some(2000),   // floor_bps = 20 %
            Some(86_400), // halving = 1 day
        );
    });

    let ev: QuorumDecayUpdatedEvent = env
        .events()
        .all()
        .iter()
        .find_map(|(_, topics, data)| {
            if topics
                .iter()
                .any(|t| t == symbol_short!("gov_qdcy").into_val(&env))
            {
                Some(data.into_val(&env))
            } else {
                None
            }
        })
        .expect("gov_qdcy event must be present");

    assert!(ev.enabled, "enabled must be true when floor_bps is Some");
    assert_eq!(ev.floor_bps, 2000);
    assert_eq!(ev.halving_seconds, 86_400);
}

#[test]
fn emit_quorum_decay_updated_disabled_encodes_correctly() {
    use crate::events::QuorumDecayUpdatedEvent;

    let (env, id) = setup();
    let admin = Address::generate(&env);

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_quorum_decay_updated(&env, &admin, None, None);
    });

    let ev: QuorumDecayUpdatedEvent = env
        .events()
        .all()
        .iter()
        .find_map(|(_, topics, data)| {
            if topics
                .iter()
                .any(|t| t == symbol_short!("gov_qdcy").into_val(&env))
            {
                Some(data.into_val(&env))
            } else {
                None
            }
        })
        .expect("gov_qdcy event must be present even when disabled");

    assert!(!ev.enabled, "enabled must be false when decay is None");
    assert_eq!(ev.floor_bps, 0, "floor_bps should be 0 when disabled");
    assert_eq!(ev.halving_seconds, 0, "halving_seconds should be 0 when disabled");
}

// ─── delegation lifecycle ─────────────────────────────────────────────────────

#[test]
fn emit_delegate_set_round_trips_addresses() {
    use crate::events::DelegateSetEvent;

    let (env, id) = setup();
    let delegator = Address::generate(&env);
    let delegate = Address::generate(&env);

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_delegate_set(&env, &delegator, &delegate);
    });

    let ev: DelegateSetEvent = env
        .events()
        .all()
        .iter()
        .find_map(|(_, topics, data)| {
            if topics
                .iter()
                .any(|t| t == symbol_short!("gov_dlgset").into_val(&env))
            {
                Some(data.into_val(&env))
            } else {
                None
            }
        })
        .expect("gov_dlgset event must be present");

    assert_eq!(ev.delegator, delegator, "delegator address must round-trip");
    assert_eq!(ev.delegate, delegate, "delegate address must round-trip");
}

#[test]
fn emit_delegate_unset_round_trips_addresses() {
    use crate::events::DelegateUnsetEvent;

    let (env, id) = setup();
    let delegator = Address::generate(&env);
    let former = Address::generate(&env);

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_delegate_unset(&env, &delegator, &former);
    });

    let ev: DelegateUnsetEvent = env
        .events()
        .all()
        .iter()
        .find_map(|(_, topics, data)| {
            if topics
                .iter()
                .any(|t| t == symbol_short!("gov_dlguns").into_val(&env))
            {
                Some(data.into_val(&env))
            } else {
                None
            }
        })
        .expect("gov_dlguns event must be present");

    assert_eq!(ev.delegator, delegator);
    assert_eq!(ev.former_delegate, former);
}

// ─── registry lifecycle ───────────────────────────────────────────────────────

#[test]
fn emit_registry_initialized_publishes_event() {
    let (env, id) = setup();
    let admin = Address::generate(&env);

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_registry_initialized(&env, &admin, 86_400);
    });

    assert_eq!(count_topic(&env, &symbol_short!("reg_init")), 1);
}

#[test]
fn emit_registry_param_proposed_carries_correct_value() {
    use crate::events::RegistryParamProposedEvent;

    let (env, id) = setup();
    let admin = Address::generate(&env);
    let key = Symbol::new(&env, "min_bet");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_registry_param_proposed(
            &env,
            &admin,
            &key,
            500,
            env.ledger().timestamp().saturating_add(86_400),
        );
    });

    let ev: RegistryParamProposedEvent = env
        .events()
        .all()
        .iter()
        .find_map(|(_, topics, data)| {
            if topics
                .iter()
                .any(|t| t == symbol_short!("reg_prop").into_val(&env))
            {
                Some(data.into_val(&env))
            } else {
                None
            }
        })
        .expect("reg_prop event must be present");

    assert_eq!(ev.new_value, 500);
    assert_eq!(ev.key, key);
    assert_eq!(ev.admin, admin);
}

#[test]
fn emit_registry_param_executed_publishes_event() {
    let (env, id) = setup();
    let admin = Address::generate(&env);
    let key = Symbol::new(&env, "min_bet");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_registry_param_executed(&env, &admin, &key, 500);
    });

    assert_eq!(count_topic(&env, &symbol_short!("reg_exec")), 1);
}

#[test]
fn emit_registry_param_cancelled_publishes_event() {
    let (env, id) = setup();
    let admin = Address::generate(&env);
    let key = Symbol::new(&env, "min_bet");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_registry_param_cancelled(&env, &admin, &key);
    });

    assert_eq!(count_topic(&env, &symbol_short!("reg_canc")), 1);
}

// ─── timestamp coherence ─────────────────────────────────────────────────────

#[test]
fn emitted_timestamp_matches_ledger_at_call_time() {
    use crate::events::ProposalExecutedEvent;

    let (env, id) = setup();
    env.ledger().with_mut(|li| li.timestamp = 999_000);

    let executor = Address::generate(&env);
    let pid = Symbol::new(&env, "ts_prop");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_proposal_executed(&env, &pid, &executor);
    });

    let ev: ProposalExecutedEvent = env
        .events()
        .all()
        .iter()
        .find_map(|(_, topics, data)| {
            if topics
                .iter()
                .any(|t| t == symbol_short!("gov_exec").into_val(&env))
            {
                Some(data.into_val(&env))
            } else {
                None
            }
        })
        .expect("gov_exec event must be present");

    assert_eq!(ev.timestamp, 999_000, "timestamp must match ledger value");
}

// ─── full lifecycle integration sequences ─────────────────────────────────────

/// Happy-path governance lifecycle: create → vote → execute.
/// Asserts each stage produces exactly one event of the expected topic.
#[test]
fn full_happy_path_lifecycle_emits_correct_topics() {
    let (env, id) = setup();
    let admin = Address::generate(&env);
    let voter = Address::generate(&env);
    let pid = Symbol::new(&env, "lifecycle");
    let title = String::from_str(&env, "Lifecycle proposal");
    let desc = String::from_str(&env, "Tests the happy path end-to-end");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_proposal_created(&env, &pid, &admin, &title, &desc);
        GovernanceEventEmitter::emit_vote_cast(&env, &pid, &voter, true, 1);
        GovernanceEventEmitter::emit_proposal_executed(&env, &pid, &admin);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_prop")), 1, "proposal created");
    assert_eq!(count_topic(&env, &symbol_short!("gov_vote")), 1, "vote cast");
    assert_eq!(count_topic(&env, &symbol_short!("gov_exec")), 1, "proposal executed");
}

/// Commit-reveal voting sub-lifecycle: commit → reveal → vote_cast.
#[test]
fn commit_reveal_lifecycle_emits_correct_topics() {
    let (env, id) = setup();
    let voter = Address::generate(&env);
    let pid = Symbol::new(&env, "cr_prop");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_vote_committed(&env, &pid, &voter);
        GovernanceEventEmitter::emit_vote_revealed(&env, &pid, &voter, true, 2);
        // The contract also emits vote_cast at reveal time.
        GovernanceEventEmitter::emit_vote_cast(&env, &pid, &voter, true, 2);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_cmit")), 1, "commit event");
    assert_eq!(count_topic(&env, &symbol_short!("gov_rvl")), 1, "reveal event");
    assert_eq!(count_topic(&env, &symbol_short!("gov_vote")), 1, "vote_cast event");
}

/// Quorum-decay auto-rejection path.
#[test]
fn auto_rejection_path_emits_gov_rej_event() {
    let (env, id) = setup();
    let proposer = Address::generate(&env);
    let pid = Symbol::new(&env, "ar_prop");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_proposal_auto_rejected(&env, &pid, &proposer, 0, 10);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_rej")), 1);
}

/// Full registry lifecycle: init → propose → execute → cancel (different param).
#[test]
fn registry_full_lifecycle_emits_all_topics() {
    let (env, id) = setup();
    let admin = Address::generate(&env);
    let key1 = Symbol::new(&env, "fee_bps");
    let key2 = Symbol::new(&env, "min_stake");

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_registry_initialized(&env, &admin, 3_600);
        GovernanceEventEmitter::emit_registry_param_proposed(
            &env,
            &admin,
            &key1,
            250,
            env.ledger().timestamp().saturating_add(3_600),
        );
        GovernanceEventEmitter::emit_registry_param_executed(&env, &admin, &key1, 250);
        GovernanceEventEmitter::emit_registry_param_proposed(
            &env,
            &admin,
            &key2,
            1_000,
            env.ledger().timestamp().saturating_add(3_600),
        );
        GovernanceEventEmitter::emit_registry_param_cancelled(&env, &admin, &key2);
    });

    assert_eq!(count_topic(&env, &symbol_short!("reg_init")), 1);
    assert_eq!(count_topic(&env, &symbol_short!("reg_prop")), 2);
    assert_eq!(count_topic(&env, &symbol_short!("reg_exec")), 1);
    assert_eq!(count_topic(&env, &symbol_short!("reg_canc")), 1);
}

/// Delegation set then unset round-trip.
#[test]
fn delegation_set_then_unset_emits_both_events() {
    let (env, id) = setup();
    let delegator = Address::generate(&env);
    let delegate = Address::generate(&env);

    env.as_contract(&id, || {
        GovernanceEventEmitter::emit_delegate_set(&env, &delegator, &delegate);
        GovernanceEventEmitter::emit_delegate_unset(&env, &delegator, &delegate);
    });

    assert_eq!(count_topic(&env, &symbol_short!("gov_dlgset")), 1);
    assert_eq!(count_topic(&env, &symbol_short!("gov_dlguns")), 1);
}
