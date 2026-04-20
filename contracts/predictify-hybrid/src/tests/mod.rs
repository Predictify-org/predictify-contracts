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

mod metadata_validation_tests;
mod oracle_provider_compatibility_tests;
mod reflector_asset_test_utils;
