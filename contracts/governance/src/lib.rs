//! `governance-events` — structured governance lifecycle events for
//! off-chain indexers (GrantFox FWC26 / Stellar Wave).
//!
//! # Design
//!
//! This crate is a companion to `predictify-hybrid`.  It owns the canonical
//! type definitions and emitter helpers for every governance lifecycle event.
//! The types are `#[contracttype]`-annotated so they decode identically whether
//! read from the host XDR stream or from the Soroban test environment.
//!
//! # Minimal contract stub
//!
//! A zero-entrypoint [`GovernanceEventsContract`] is registered only to satisfy
//! the Soroban requirement that event emission happens within a contract context.
//! Production callers invoke the emitters directly from their own contract impl.

#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

pub mod events;

/// Minimal no-op contract that provides a valid contract context for the
/// `GovernanceEventEmitter` helpers during tests.
///
/// This struct carries no state and exposes no public entrypoints.
/// It exists solely so that `env.register(GovernanceEventsContract, ())`
/// gives test code an [`Address`] to pass to `env.as_contract(...)`.
#[contract]
pub struct GovernanceEventsContract;

#[contractimpl]
impl GovernanceEventsContract {
    /// No-op initialiser — present only to satisfy the `#[contractimpl]` macro
    /// requirement that at least one function is defined.
    #[allow(dead_code)]
    pub fn ping(_env: Env) {}
}

#[cfg(test)]
mod events_tests;
