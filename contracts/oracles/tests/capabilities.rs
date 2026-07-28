//! Focused tests for the `capabilities()` read-only view — issue #1120.
//!
//! # Coverage
//!
//! | # | Test                                          | What it verifies                                    |
//! |---|-----------------------------------------------|-----------------------------------------------------|
//! | 1 | `capabilities_returns_nonzero`                | Return value is non-zero                            |
//! | 2 | `capabilities_has_expected_flags`             | Every documented flag is set                        |
//! | 3 | `capabilities_no_reserved_bits_set`           | Bits 8-63 are all clear                             |
//! | 4 | `capabilities_is_pure_no_auth_required`       | Call succeeds without `mock_all_auths`              |
//! | 5 | `capabilities_is_idempotent`                  | Two consecutive calls return the same value         |
//! | 6 | `capabilities_unaffected_by_registry_state`   | Adding/removing oracles does not change the bitmap  |
//! | 7 | `capabilities_bit_positions_match_flag_consts`| Individual bit checks via `CapabilityFlag` constants|
//! | 8 | `capabilities_flag_subset_check`              | Subset mask helper pattern works correctly          |
//! | 9 | `capabilities_independent_of_version`         | `version()` and `capabilities()` are orthogonal     |

#![cfg(test)]

use oracles::{CapabilityFlag, OraclesContract, OraclesContractClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

/// Minimal test fixture: fresh env, deployed contract, client.
///
/// Auth is **not** mocked globally here; individual tests that need
/// state-changing calls add `env.mock_all_auths()` themselves.
struct Fx {
    env: Env,
    client: OraclesContractClient<'static>,
}

impl Fx {
    fn new() -> Self {
        let env = Env::default();
        let contract_id = env.register(OraclesContract, ());
        // SAFETY: the client borrows the contract_id which lives in `env`;
        // we keep both in the same struct so the lifetime is valid for the
        // test body.
        let client = OraclesContractClient::new(&env, &contract_id);
        Self { env, client }
    }
}

// ---------------------------------------------------------------------------
// 1. Non-zero return
// ---------------------------------------------------------------------------

/// `capabilities()` must never return zero — a zero bitmap would indicate
/// that the contract supports nothing, which is always incorrect.
#[test]
fn capabilities_returns_nonzero() {
    let f = Fx::new();
    assert_ne!(
        f.client.capabilities(),
        0,
        "capabilities() must not be zero"
    );
}

// ---------------------------------------------------------------------------
// 2. All documented flags are set
// ---------------------------------------------------------------------------

/// Every flag in the current feature table must be present in the bitmap.
///
/// If a flag is missing this test names which one, making CI output
/// immediately actionable.
#[test]
fn capabilities_has_expected_flags() {
    let f = Fx::new();
    let caps = f.client.capabilities();

    let required: &[(&str, u64)] = &[
        ("GET_PRICE",           CapabilityFlag::GET_PRICE),
        ("GET_PRICE_DATA",      CapabilityFlag::GET_PRICE_DATA),
        ("IS_ORACLE_HEALTHY",   CapabilityFlag::IS_ORACLE_HEALTHY),
        ("ORACLE_REGISTRY",     CapabilityFlag::ORACLE_REGISTRY),
        ("CONFIDENCE_INTERVAL", CapabilityFlag::CONFIDENCE_INTERVAL),
        ("PRICE_EXPONENT",      CapabilityFlag::PRICE_EXPONENT),
        ("TTL_MANAGEMENT",      CapabilityFlag::TTL_MANAGEMENT),
        ("VERSION_VIEW",        CapabilityFlag::VERSION_VIEW),
    ];

    for (name, flag) in required {
        assert_ne!(
            caps & flag,
            0,
            "expected flag {name} (bit {}) to be set in capabilities() = {caps:#018x}",
            flag.trailing_zeros()
        );
    }
}

// ---------------------------------------------------------------------------
// 3. No reserved bits are set
// ---------------------------------------------------------------------------

/// Bits 8-63 are reserved for future use and must be zero in this version.
///
/// A set reserved bit would be a client-visible API change that breaks any
/// consumer performing an equality check on the whole bitmap.
#[test]
fn capabilities_no_reserved_bits_set() {
    let f = Fx::new();
    let caps = f.client.capabilities();
    let defined_mask: u64 = (1 << 8) - 1; // bits 0-7 only

    assert_eq!(
        caps & !defined_mask,
        0,
        "reserved bits (8-63) must be zero; got capabilities() = {caps:#018x}"
    );
}

// ---------------------------------------------------------------------------
// 4. Pure read — no auth required
// ---------------------------------------------------------------------------

/// `capabilities()` must be callable without any authorization.
///
/// Read-only views should never gate access behind `require_auth`; doing so
/// would prevent clients from capability-checking before deciding whether to
/// call a state-changing entrypoint.
#[test]
fn capabilities_is_pure_no_auth_required() {
    // Deliberately do NOT call env.mock_all_auths().
    let env = Env::default();
    let contract_id = env.register(OraclesContract, ());
    let client = OraclesContractClient::new(&env, &contract_id);

    // Must not panic or return an auth error.
    let caps = client.capabilities();
    assert_ne!(caps, 0, "capabilities() must return a non-zero value without auth");
}

// ---------------------------------------------------------------------------
// 5. Idempotent
// ---------------------------------------------------------------------------

/// Two consecutive calls within the same transaction must return identical
/// values.  `capabilities()` is purely compile-time data — it must not vary
/// based on ledger state.
#[test]
fn capabilities_is_idempotent() {
    let f = Fx::new();
    let first  = f.client.capabilities();
    let second = f.client.capabilities();
    assert_eq!(first, second, "capabilities() must be idempotent");
}

// ---------------------------------------------------------------------------
// 6. Unaffected by registry state
// ---------------------------------------------------------------------------

/// Adding and removing oracles must not alter the capabilities bitmap.
///
/// The bitmap is a static property of the contract build, not of runtime
/// ledger state.
#[test]
fn capabilities_unaffected_by_registry_state() {
    let f = Fx::new();
    f.env.mock_all_auths();

    let caps_before = f.client.capabilities();

    // Mutate registry state.
    let admin  = Address::generate(&f.env);
    let oracle = Address::generate(&f.env);
    f.client.add_oracle(&admin, &oracle);

    let caps_after_add = f.client.capabilities();
    assert_eq!(
        caps_before, caps_after_add,
        "add_oracle must not change capabilities()"
    );

    f.client.remove_oracle(&admin, &oracle);
    let caps_after_remove = f.client.capabilities();
    assert_eq!(
        caps_before, caps_after_remove,
        "remove_oracle must not change capabilities()"
    );
}

// ---------------------------------------------------------------------------
// 7. Individual bit positions via CapabilityFlag constants
// ---------------------------------------------------------------------------

/// Validate each flag constant occupies the documented bit position.
///
/// This test freezes the bit layout as a client-facing stability contract:
/// changing a bit position is an API break and requires a version bump.
#[test]
fn capabilities_bit_positions_match_flag_consts() {
    let f = Fx::new();
    let caps = f.client.capabilities();

    // Exact bit-position assertions.
    assert_eq!(caps & (1u64 << 0), CapabilityFlag::GET_PRICE,           "bit 0 = GET_PRICE");
    assert_eq!(caps & (1u64 << 1), CapabilityFlag::GET_PRICE_DATA,      "bit 1 = GET_PRICE_DATA");
    assert_eq!(caps & (1u64 << 2), CapabilityFlag::IS_ORACLE_HEALTHY,   "bit 2 = IS_ORACLE_HEALTHY");
    assert_eq!(caps & (1u64 << 3), CapabilityFlag::ORACLE_REGISTRY,     "bit 3 = ORACLE_REGISTRY");
    assert_eq!(caps & (1u64 << 4), CapabilityFlag::CONFIDENCE_INTERVAL, "bit 4 = CONFIDENCE_INTERVAL");
    assert_eq!(caps & (1u64 << 5), CapabilityFlag::PRICE_EXPONENT,      "bit 5 = PRICE_EXPONENT");
    assert_eq!(caps & (1u64 << 6), CapabilityFlag::TTL_MANAGEMENT,      "bit 6 = TTL_MANAGEMENT");
    assert_eq!(caps & (1u64 << 7), CapabilityFlag::VERSION_VIEW,        "bit 7 = VERSION_VIEW");
}

// ---------------------------------------------------------------------------
// 8. Subset mask helper pattern
// ---------------------------------------------------------------------------

/// Demonstrate and validate the idiomatic subset-check pattern.
///
/// A client that requires a minimum set of features can combine flags with `|`
/// and assert the result equals the mask, ensuring every required flag is set.
#[test]
fn capabilities_flag_subset_check() {
    let f = Fx::new();
    let caps = f.client.capabilities();

    // Minimum set required by a price-feed consumer.
    let price_feed_minimum =
        CapabilityFlag::GET_PRICE | CapabilityFlag::ORACLE_REGISTRY;

    assert_eq!(
        caps & price_feed_minimum,
        price_feed_minimum,
        "contract does not satisfy price-feed minimum capability set"
    );

    // Minimum set required by a full-data consumer.
    let full_data_minimum = CapabilityFlag::GET_PRICE
        | CapabilityFlag::GET_PRICE_DATA
        | CapabilityFlag::ORACLE_REGISTRY
        | CapabilityFlag::CONFIDENCE_INTERVAL
        | CapabilityFlag::PRICE_EXPONENT;

    assert_eq!(
        caps & full_data_minimum,
        full_data_minimum,
        "contract does not satisfy full-data minimum capability set"
    );
}

// ---------------------------------------------------------------------------
// 9. Orthogonal to version()
// ---------------------------------------------------------------------------

/// `capabilities()` and `version()` are independent views and must both be
/// callable in sequence without interfering with each other.
///
/// This guards against accidental shared mutable state between the two views.
#[test]
fn capabilities_independent_of_version() {
    let f = Fx::new();

    let caps    = f.client.capabilities();
    let version = f.client.version();

    // version() must still return 7 (from lib.rs).
    assert_eq!(version, 7u32, "version() must return 7");

    // capabilities() must still be non-zero and contain all flags.
    assert_ne!(caps, 0u64, "capabilities() must be non-zero after version() call");
    assert_ne!(caps & CapabilityFlag::GET_PRICE, 0, "GET_PRICE must be set");
    assert_ne!(caps & CapabilityFlag::VERSION_VIEW, 0, "VERSION_VIEW must be set");
}
