#[cfg(any())]
mod oracle_provider_compatibility_tests;
pub mod security;
mod mocks;
//! Test module organization for Predictify Hybrid.
//!
//! This module organizes all test suites and utilities for structured testing
//! across the contract codebase.

#![cfg(test)]

// Common test utilities shared across all test modules
pub mod common;

// Error recovery scenario tests
pub mod error_scenarios;

// Integration test modules
pub mod integration {
    pub mod oracle_integration_tests;
    pub mod custom_token_tests;
    pub mod oracle_provider_compatibility_tests;
}

// Test mocks
pub mod mocks {
    pub mod oracle;
}

// Security tests
pub mod security {
    pub mod oracle_security_tests;
}
