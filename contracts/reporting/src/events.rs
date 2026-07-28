//! Events emitted by the Reporting contract.
//!
//! Organised by topic so that off-chain indexers can filter on the first
//! `Symbol` topic.

use soroban_sdk::{Address, Env, Symbol};

/// Publish a `reporting_paused` event.
///
/// Emitted by [`ReportingContract::pause_reporting`](crate::ReportingContract::pause_reporting)
/// when an admin successfully pauses the reporting mechanism.
pub fn emit_reporting_paused(env: &Env, admin: &Address) {
    let topics = (Symbol::new(env, "reporting_paused"), admin);
    env.events().publish(topics, ());
}

/// Publish a `reporting_unpaused` event.
///
/// Emitted by [`ReportingContract::unpause_reporting`](crate::ReportingContract::unpause_reporting)
/// when an admin successfully resumes the reporting mechanism.
pub fn emit_reporting_unpaused(env: &Env, admin: &Address) {
    let topics = (Symbol::new(env, "reporting_unpaused"), admin);
    env.events().publish(topics, ());
}
