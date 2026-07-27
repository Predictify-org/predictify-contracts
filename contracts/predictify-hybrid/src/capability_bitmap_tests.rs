//! Focused tests for the capabilities() read-only view — issue #984.
//!
//! These tests exercise the `capabilities()` contract entrypoint through the
//! generated client to verify:
//!
//!  - The RECOVERY bit (bit 17) is always set.
//!  - The call is truly side-effect free (no storage writes, no events).
//!  - The complete set of expected capability bits is present.
//!  - No reserved bits (26–63) are ever set.
//!  - The bitmap is stable across repeated calls and across a fresh contract.
//!
//! The low-level unit tests for the `capability` constants themselves live
//! inside `capabilities.rs`; this module exercises the *public contract
//! entrypoint* path end-to-end.

#![cfg(test)]

use crate::{
    capabilities::capability,
    test::PredictifyTest,
    PredictifyHybridClient,
};
use soroban_sdk::Symbol;

// ── helpers ─────────────────────────────────────────────────────────────────

/// The expected full set of capability bits as of this contract version.
/// Any future capability addition must also appear here.
fn expected_caps() -> u64 {
    capability::VERSIONING
        | capability::UPGRADE_MANAGEMENT
        | capability::QUERY_FUNCTIONS
        | capability::MARKET_MANAGEMENT
        | capability::BETTING
        | capability::DISPUTES
        | capability::ORACLE_INTEGRATION
        | capability::GOVERNANCE
        | capability::ANALYTICS
        | capability::MONITORING
        | capability::FEE_MANAGEMENT
        | capability::AUDIT_TRAIL
        | capability::CIRCUIT_BREAKER
        | capability::RATE_LIMITING
        | capability::EVENT_ARCHIVE
        | capability::METADATA_COMMITMENT
        | capability::BATCH_OPERATIONS
        | capability::RECOVERY
        | capability::MULTI_ADMIN_MULTISIG
        | capability::STATISTICS
        | capability::TOKEN_REGISTRY
        | capability::EVENT_VISIBILITY
        | capability::CLAIM_IDEMPOTENCY
        | capability::BET_CANCELLATION
        | capability::FEE_WITHDRAWAL
        | capability::PAYOUT_DISTRIBUTION
}

/// Mask covering all reserved bits (26–63).
const RESERVED_MASK: u64 = !((1u64 << 26) - 1);

// ── Issue #984: RECOVERY bit ─────────────────────────────────────────────────

/// The RECOVERY capability (bit 17) must be advertised.
///
/// Clients use this bit to decide whether the contract supports the recovery
/// subsystem (`recover_market_state`, `partial_refund_mechanism`, etc.)
/// without inspecting the Wasm binary.
#[test]
fn test_recovery_capability_is_set() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let caps = client.capabilities();

    assert!(
        caps & capability::RECOVERY != 0,
        "RECOVERY bit (bit 17 / 0x0002_0000) must be set; got caps = {:#018x}",
        caps
    );
}

/// RECOVERY maps to exactly bit 17 (mask 0x0002_0000).
#[test]
fn test_recovery_bit_position() {
    assert_eq!(
        capability::RECOVERY,
        1u64 << 17,
        "RECOVERY must occupy bit 17"
    );
    assert_eq!(
        capability::RECOVERY,
        0x0002_0000u64,
        "RECOVERY mask must equal 0x0002_0000"
    );
}

/// Calling capabilities() emits no events and writes nothing to storage.
///
/// This is the primary safety property of a read-only view: it must be safe
/// to call at any time on any network without side effects.
#[test]
fn test_recovery_capabilities_is_side_effect_free() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    // Snapshot storage and event state before the call.
    let version_key = Symbol::new(&test.env, "VERSION_HISTORY");
    let had_version_history = test.env.storage().persistent().has(&version_key);
    let events_before = test.env.events().all().len();

    let caps = client.capabilities();

    assert!(caps & capability::RECOVERY != 0);

    // No storage mutations.
    let has_version_history = test.env.storage().persistent().has(&version_key);
    assert_eq!(
        had_version_history, has_version_history,
        "capabilities() must not write to persistent storage"
    );

    // No events emitted.
    let events_after = test.env.events().all().len();
    assert_eq!(
        events_before, events_after,
        "capabilities() must not emit any events"
    );
}

/// The RECOVERY bit survives repeated invocations (idempotency / determinism).
#[test]
fn test_recovery_bit_is_stable_across_calls() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let first = client.capabilities();
    let second = client.capabilities();
    let third = client.capabilities();

    assert_eq!(first, second, "capabilities() must be deterministic");
    assert_eq!(second, third, "capabilities() must be deterministic");
    assert!(first & capability::RECOVERY != 0, "RECOVERY must remain set");
}

// ── Full bitmap correctness ──────────────────────────────────────────────────

/// The full returned bitmap must exactly match the expected set.
#[test]
fn test_full_capabilities_bitmap_matches_expected() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);

    let caps = client.capabilities();
    let expected = expected_caps();

    assert_eq!(
        caps, expected,
        "capabilities bitmap mismatch.\n\
         extra bits set  : {:#018x}\n\
         missing bits    : {:#018x}",
        caps & !expected,
        expected & !caps,
    );
}

/// Every individually-named capability constant must be present.
#[test]
fn test_all_named_capabilities_are_set() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    let caps = client.capabilities();

    let named: &[(&str, u64)] = &[
        ("VERSIONING",         capability::VERSIONING),
        ("UPGRADE_MANAGEMENT", capability::UPGRADE_MANAGEMENT),
        ("QUERY_FUNCTIONS",    capability::QUERY_FUNCTIONS),
        ("MARKET_MANAGEMENT",  capability::MARKET_MANAGEMENT),
        ("BETTING",            capability::BETTING),
        ("DISPUTES",           capability::DISPUTES),
        ("ORACLE_INTEGRATION", capability::ORACLE_INTEGRATION),
        ("GOVERNANCE",         capability::GOVERNANCE),
        ("ANALYTICS",          capability::ANALYTICS),
        ("MONITORING",         capability::MONITORING),
        ("FEE_MANAGEMENT",     capability::FEE_MANAGEMENT),
        ("AUDIT_TRAIL",        capability::AUDIT_TRAIL),
        ("CIRCUIT_BREAKER",    capability::CIRCUIT_BREAKER),
        ("RATE_LIMITING",      capability::RATE_LIMITING),
        ("EVENT_ARCHIVE",      capability::EVENT_ARCHIVE),
        ("METADATA_COMMITMENT",capability::METADATA_COMMITMENT),
        ("BATCH_OPERATIONS",   capability::BATCH_OPERATIONS),
        ("RECOVERY",           capability::RECOVERY),
        ("MULTI_ADMIN_MULTISIG",capability::MULTI_ADMIN_MULTISIG),
        ("STATISTICS",         capability::STATISTICS),
        ("TOKEN_REGISTRY",     capability::TOKEN_REGISTRY),
        ("EVENT_VISIBILITY",   capability::EVENT_VISIBILITY),
        ("CLAIM_IDEMPOTENCY",  capability::CLAIM_IDEMPOTENCY),
        ("BET_CANCELLATION",   capability::BET_CANCELLATION),
        ("FEE_WITHDRAWAL",     capability::FEE_WITHDRAWAL),
        ("PAYOUT_DISTRIBUTION",capability::PAYOUT_DISTRIBUTION),
    ];

    for (name, mask) in named {
        assert!(
            caps & mask != 0,
            "capability {} (mask {:#018x}) is not set in bitmap {:#018x}",
            name, mask, caps
        );
    }
}

/// No reserved bits (26–63) may be set.
#[test]
fn test_no_reserved_bits_set() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    let caps = client.capabilities();

    assert_eq!(
        caps & RESERVED_MASK,
        0,
        "bits 26–63 are reserved and must be zero; got {:#018x}",
        caps & RESERVED_MASK
    );
}

/// The bitmap must be non-zero.
#[test]
fn test_capabilities_bitmap_nonzero() {
    let test = PredictifyTest::setup();
    let client = PredictifyHybridClient::new(&test.env, &test.contract_id);
    assert!(client.capabilities() > 0, "bitmap must not be zero");
}

// ── Bit-constant invariants (compile-time style) ─────────────────────────────

/// All 26 defined capability masks must be powers of two (single-bit).
#[test]
fn test_all_capability_masks_are_single_bit() {
    let all_masks: &[u64] = &[
        capability::VERSIONING,
        capability::UPGRADE_MANAGEMENT,
        capability::QUERY_FUNCTIONS,
        capability::MARKET_MANAGEMENT,
        capability::BETTING,
        capability::DISPUTES,
        capability::ORACLE_INTEGRATION,
        capability::GOVERNANCE,
        capability::ANALYTICS,
        capability::MONITORING,
        capability::FEE_MANAGEMENT,
        capability::AUDIT_TRAIL,
        capability::CIRCUIT_BREAKER,
        capability::RATE_LIMITING,
        capability::EVENT_ARCHIVE,
        capability::METADATA_COMMITMENT,
        capability::BATCH_OPERATIONS,
        capability::RECOVERY,
        capability::MULTI_ADMIN_MULTISIG,
        capability::STATISTICS,
        capability::TOKEN_REGISTRY,
        capability::EVENT_VISIBILITY,
        capability::CLAIM_IDEMPOTENCY,
        capability::BET_CANCELLATION,
        capability::FEE_WITHDRAWAL,
        capability::PAYOUT_DISTRIBUTION,
    ];
    for &mask in all_masks {
        assert!(
            mask != 0 && mask.count_ones() == 1,
            "capability mask {:#018x} is not a power of two",
            mask
        );
    }
}

/// All 26 defined capability masks must be distinct (no two share a bit).
#[test]
fn test_capability_masks_are_unique() {
    let all_masks: &[(&str, u64)] = &[
        ("VERSIONING",          capability::VERSIONING),
        ("UPGRADE_MANAGEMENT",  capability::UPGRADE_MANAGEMENT),
        ("QUERY_FUNCTIONS",     capability::QUERY_FUNCTIONS),
        ("MARKET_MANAGEMENT",   capability::MARKET_MANAGEMENT),
        ("BETTING",             capability::BETTING),
        ("DISPUTES",            capability::DISPUTES),
        ("ORACLE_INTEGRATION",  capability::ORACLE_INTEGRATION),
        ("GOVERNANCE",          capability::GOVERNANCE),
        ("ANALYTICS",           capability::ANALYTICS),
        ("MONITORING",          capability::MONITORING),
        ("FEE_MANAGEMENT",      capability::FEE_MANAGEMENT),
        ("AUDIT_TRAIL",         capability::AUDIT_TRAIL),
        ("CIRCUIT_BREAKER",     capability::CIRCUIT_BREAKER),
        ("RATE_LIMITING",       capability::RATE_LIMITING),
        ("EVENT_ARCHIVE",       capability::EVENT_ARCHIVE),
        ("METADATA_COMMITMENT", capability::METADATA_COMMITMENT),
        ("BATCH_OPERATIONS",    capability::BATCH_OPERATIONS),
        ("RECOVERY",            capability::RECOVERY),
        ("MULTI_ADMIN_MULTISIG",capability::MULTI_ADMIN_MULTISIG),
        ("STATISTICS",          capability::STATISTICS),
        ("TOKEN_REGISTRY",      capability::TOKEN_REGISTRY),
        ("EVENT_VISIBILITY",    capability::EVENT_VISIBILITY),
        ("CLAIM_IDEMPOTENCY",   capability::CLAIM_IDEMPOTENCY),
        ("BET_CANCELLATION",    capability::BET_CANCELLATION),
        ("FEE_WITHDRAWAL",      capability::FEE_WITHDRAWAL),
        ("PAYOUT_DISTRIBUTION", capability::PAYOUT_DISTRIBUTION),
    ];

    for i in 0..all_masks.len() {
        for j in (i + 1)..all_masks.len() {
            let (name_a, mask_a) = all_masks[i];
            let (name_b, mask_b) = all_masks[j];
            assert_ne!(
                mask_a, mask_b,
                "capability {} and {} share the same mask {:#018x}",
                name_a, name_b, mask_a
            );
        }
    }
}

/// All 26 defined bits fall within bits 0–25 (the documented range).
#[test]
fn test_all_capability_bits_within_documented_range() {
    let all_masks: &[u64] = &[
        capability::VERSIONING,
        capability::UPGRADE_MANAGEMENT,
        capability::QUERY_FUNCTIONS,
        capability::MARKET_MANAGEMENT,
        capability::BETTING,
        capability::DISPUTES,
        capability::ORACLE_INTEGRATION,
        capability::GOVERNANCE,
        capability::ANALYTICS,
        capability::MONITORING,
        capability::FEE_MANAGEMENT,
        capability::AUDIT_TRAIL,
        capability::CIRCUIT_BREAKER,
        capability::RATE_LIMITING,
        capability::EVENT_ARCHIVE,
        capability::METADATA_COMMITMENT,
        capability::BATCH_OPERATIONS,
        capability::RECOVERY,
        capability::MULTI_ADMIN_MULTISIG,
        capability::STATISTICS,
        capability::TOKEN_REGISTRY,
        capability::EVENT_VISIBILITY,
        capability::CLAIM_IDEMPOTENCY,
        capability::BET_CANCELLATION,
        capability::FEE_WITHDRAWAL,
        capability::PAYOUT_DISTRIBUTION,
    ];
    for &mask in all_masks {
        assert!(
            mask < (1u64 << 26),
            "capability mask {:#018x} falls outside the documented range (bits 0–25)",
            mask
        );
    }
}
