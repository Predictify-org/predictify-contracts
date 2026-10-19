//! Surface and fail-closed tests for the `edge_cases` entry points.
//!
//! Background: the full build used to export several "edge case" entry points
//! whose internals were placeholders returning `Ok(())` without acting (and one
//! that trapped on past `end_time`). Those entry points are now removed from
//! the contract surface, and the helpers that remain fail closed instead of
//! silently succeeding.
//!
//! This suite pins:
//!
//! 1. `handle_zero_stake_scenario`, `handle_partial_resolution` and
//!    `test_edge_case_scenarios` are absent from the exported client surface.
//! 2. `implement_tie_breaking_mechanism` returns `InvalidInput` on empty input
//!    (it never panics) and applies its documented deterministic rule.
//! 3. `detect_orphaned_markets` fails closed with `InvalidState`.

use crate::edge_cases::EdgeCaseHandler;
use crate::err::Error;
#[cfg(feature = "full")]
use crate::PredictifyHybrid;
#[cfg(feature = "full")]
use soroban_sdk::testutils::ContractFunctionSet;
use soroban_sdk::{Env, String, Vec};

/// `true` when `name` is registered as an exported contract entry point.
///
/// The generated `ContractFunctionSet` registry is populated by constructors
/// emitted by `#[contractimpl]` in full builds, so it reflects exactly the
/// exported surface.
#[cfg(feature = "full")]
fn is_exported(env: &Env, name: &str) -> bool {
    PredictifyHybrid.call(name, env.clone(), &[]).is_some()
}

// ---------------------------------------------------------------------------
// Row 7: removed entry points are absent from the client surface
// ---------------------------------------------------------------------------

#[cfg(feature = "full")]
#[test]
fn test_edge_case_scenarios_absent_from_client_surface() {
    let env = Env::default();
    assert!(!is_exported(&env, "test_edge_case_scenarios"));
}

#[cfg(feature = "full")]
#[test]
fn handle_zero_stake_scenario_absent_from_client_surface() {
    let env = Env::default();
    assert!(!is_exported(&env, "handle_zero_stake_scenario"));
}

#[cfg(feature = "full")]
#[test]
fn handle_partial_resolution_absent_from_client_surface() {
    let env = Env::default();
    assert!(!is_exported(&env, "handle_partial_resolution"));
}

/// Sanity control: the probe above does find a still-exported entry point, so
/// the negative assertions are meaningful and not passing on an empty registry.
#[cfg(feature = "full")]
#[test]
fn surface_probe_finds_exported_entry_point() {
    let env = Env::default();
    assert!(is_exported(&env, "detect_orphaned_markets"));
}

// ---------------------------------------------------------------------------
// Row 6: tie-breaking never panics, is deterministic, and documents one rule
// ---------------------------------------------------------------------------

#[test]
fn implement_tie_breaking_mechanism_empty_returns_invalid_input() {
    let env = Env::default();
    let result = EdgeCaseHandler::implement_tie_breaking_mechanism(&env, Vec::new(&env));
    assert_eq!(result, Err(Error::InvalidInput));
}

#[test]
fn implement_tie_breaking_mechanism_picks_lexicographically_smallest() {
    let env = Env::default();
    let outcomes = soroban_sdk::vec![
        &env,
        String::from_str(&env, "zebra"),
        String::from_str(&env, "apple"),
        String::from_str(&env, "mango"),
    ];

    let winner = EdgeCaseHandler::implement_tie_breaking_mechanism(&env, outcomes).unwrap();
    assert_eq!(winner, String::from_str(&env, "apple"));
}

// ---------------------------------------------------------------------------
// Row 8: remaining helper fails closed with a typed error (no silent success)
// ---------------------------------------------------------------------------

#[test]
fn detect_orphaned_markets_fails_closed() {
    let env = Env::default();
    let result = EdgeCaseHandler::detect_orphaned_markets(&env);
    assert_eq!(result, Err(Error::InvalidState));
}
