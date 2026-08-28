#![cfg(test)]

//! Test module organization for Predictify Hybrid.
//!
//! This module organizes all test suites and utilities for structured testing across the contract
//! codebase.

pub mod common;
pub mod error_scenarios;
pub mod integration;
pub mod mocks;
pub mod security;

#[cfg(test)]
pub mod fee_calculator_proptest;

// DISABLED: API drift - re-enable after fixing
// mod fee_idempotency_tests;
mod rate_limiter_tests;
mod rate_limiter_invariants;
mod rate_limiter_halflife_tests;
// mod metadata_validation_tests;
// mod oracle_provider_compatibility_tests;
// mod oracle_validation_tests;
// mod reflector_asset_test_utils;

pub mod dispute_stake_tests;
pub mod fee_config_commit_reveal_tests;
pub mod reflector_twap_cache_tests;
pub mod dispute_anti_grief_tests;
pub mod dispute_open_fuzz;
pub mod oracle_differential_fuzz;
pub mod resolve_market_fuzz;

#[cfg(test)]
pub mod dispute_proptest;

#[cfg(test)]
pub mod disputes_gas_snap;

/// Focused tests for `Error::MarketClosed` (code 102).
///
/// Covers every code path that should return or panic with `MarketClosed`,
/// boundary conditions around `end_time`, and verifies the error metadata
/// (code, description, string code).
pub mod market_closed_tests;

/// Rollback-safety tests for batch bet placement.
///
/// Covers duplicate-market rejection, empty/oversized batch guards,
/// idempotency replay, and multi-market success paths.
pub mod batch_bet_rollback_tests;