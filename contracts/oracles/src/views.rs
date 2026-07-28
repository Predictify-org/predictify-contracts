//! Read-only capability introspection for the oracles contract.
//!
//! # Overview
//!
//! [`capabilities()`] returns a `u64` bitmap that describes which features
//! this deployment of the oracles contract supports.  Clients can call this
//! once and cache the result to decide which entrypoints are safe to invoke
//! without trying them first.
//!
//! # Bitmap layout
//!
//! Each bit position is a named [`CapabilityFlag`] constant.  Bits are
//! assigned from LSB (bit 0) upward.  Unassigned bits are **always 0**;
//! clients must treat any unknown set bit as reserved and must not interpret
//! it.  The set of bits that can be set in a given contract version is the
//! stable API surface tracked by this module.
//!
//! | Bit | Name                  | Meaning                                              |
//! |----:|-----------------------|------------------------------------------------------|
//! |   0 | `GET_PRICE`           | `get_price(oracle, feed)` entrypoint present         |
//! |   1 | `GET_PRICE_DATA`      | `get_price_data(oracle, feed)` entrypoint present    |
//! |   2 | `IS_ORACLE_HEALTHY`   | `is_oracle_healthy(oracle)` entrypoint present       |
//! |   3 | `ORACLE_REGISTRY`     | `add_oracle` / `remove_oracle` / `list_oracles` present |
//! |   4 | `CONFIDENCE_INTERVAL` | `OraclePriceData.confidence` field is populated      |
//! |   5 | `PRICE_EXPONENT`      | `OraclePriceData.exponent` field is populated        |
//! |   6 | `TTL_MANAGEMENT`      | Contract bumps ledger TTL on every registry access   |
//! |   7 | `VERSION_VIEW`        | `version()` read-only view is present                |
//!
//! # Stability guarantee
//!
//! Once a bit is assigned it is **never re-used** for a different feature,
//! even if the corresponding feature is removed.  A removed feature's bit
//! will simply be cleared from the bitmap.  This lets clients detect
//! capability *deltas* across upgrades without false-positives.

/// Named bit-flag constants for oracle contract capabilities.
///
/// Each constant is a power-of-two `u64` that can be combined with `|` to
/// form a bitmap, and isolated with `&` to test a specific capability.
///
/// # Example
///
/// ```rust,ignore
/// let caps = client.capabilities();
/// if caps & CapabilityFlag::GET_PRICE != 0 {
///     // safe to call get_price
/// }
/// ```
pub struct CapabilityFlag;

impl CapabilityFlag {
    /// Bit 0 — `get_price(oracle, feed_id)` entrypoint is present.
    ///
    /// When set, callers may request a raw `i128` price from a registered
    /// oracle by feed identifier.
    pub const GET_PRICE: u64 = 1 << 0;

    /// Bit 1 — `get_price_data(oracle, feed_id)` entrypoint is present.
    ///
    /// When set, callers may request a full [`OraclePriceData`] struct
    /// containing price, publish time, confidence interval, and exponent.
    pub const GET_PRICE_DATA: u64 = 1 << 1;

    /// Bit 2 — `is_oracle_healthy(oracle)` entrypoint is present.
    ///
    /// When set, callers may query whether a registered oracle reports
    /// itself as live.
    pub const IS_ORACLE_HEALTHY: u64 = 1 << 2;

    /// Bit 3 — Oracle registry management is present.
    ///
    /// When set, `add_oracle`, `remove_oracle`, and `list_oracles` are all
    /// available.  Registry mutations require admin authorization.
    pub const ORACLE_REGISTRY: u64 = 1 << 3;

    /// Bit 4 — `OraclePriceData.confidence` field is populated.
    ///
    /// When set, the contract attempts to retrieve a confidence interval
    /// from the underlying oracle.  If the oracle does not provide one the
    /// field falls back to `None`; the bit reflects the *capability* of the
    /// contract to propagate confidence data, not of a specific oracle.
    pub const CONFIDENCE_INTERVAL: u64 = 1 << 4;

    /// Bit 5 — `OraclePriceData.exponent` field is populated.
    ///
    /// When set, the exponent (decimal shift) returned by the underlying
    /// oracle is propagated through `get_price_data`.
    pub const PRICE_EXPONENT: u64 = 1 << 5;

    /// Bit 6 — Ledger TTL is bumped on every oracle registry access.
    ///
    /// When set, the contract calls `env.storage().persistent().extend_ttl`
    /// on the registry key for read-heavy hot paths, preventing the entry
    /// from expiring under sustained load.
    pub const TTL_MANAGEMENT: u64 = 1 << 6;

    /// Bit 7 — `version()` read-only view is present.
    ///
    /// When set, callers may invoke `version()` to retrieve the contract's
    /// numeric version for compatibility checks.
    pub const VERSION_VIEW: u64 = 1 << 7;
}

/// Bitmap of all capability flags supported by this contract build.
///
/// This is the value returned by [`capabilities()`] and is computed as the
/// bitwise OR of every [`CapabilityFlag`] constant that this version of the
/// contract implements.
///
/// Keeping the constant here (rather than inlining it in the `#[contractimpl]`
/// block) lets the test suite assert both the individual bit values and the
/// aggregate without depending on the host environment.
pub const SUPPORTED_CAPABILITIES: u64 = CapabilityFlag::GET_PRICE
    | CapabilityFlag::GET_PRICE_DATA
    | CapabilityFlag::IS_ORACLE_HEALTHY
    | CapabilityFlag::ORACLE_REGISTRY
    | CapabilityFlag::CONFIDENCE_INTERVAL
    | CapabilityFlag::PRICE_EXPONENT
    | CapabilityFlag::TTL_MANAGEMENT
    | CapabilityFlag::VERSION_VIEW;

/// Return the `u64` bitmap of features supported by this oracle contract.
///
/// # Purpose
///
/// Clients can call `capabilities()` once per upgrade and diff the result
/// against a previously cached value to detect which features were added or
/// removed.  Because bits are never re-assigned, a cleared bit unambiguously
/// means the feature was removed, and a newly set bit means a feature was
/// added.
///
/// # No auth required
///
/// This is a pure read that touches no contract storage and requires no
/// authorization.  It is safe to call from any account or contract.
///
/// # Return value
///
/// A `u64` where each set bit corresponds to a [`CapabilityFlag`] constant.
/// Callers should mask individual bits with `&` rather than comparing the
/// whole value, so that future additions of new bits do not break existing
/// checks.
///
/// # Example
///
/// ```rust,ignore
/// let caps = client.capabilities();
/// assert!(caps & CapabilityFlag::GET_PRICE != 0);
/// assert!(caps & CapabilityFlag::ORACLE_REGISTRY != 0);
/// ```
pub fn capabilities() -> u64 {
    SUPPORTED_CAPABILITIES
}

// ---------------------------------------------------------------------------
// Unit-level tests (no Soroban host needed)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod unit_tests {
    use super::{capabilities, CapabilityFlag, SUPPORTED_CAPABILITIES};

    /// Every flag constant must be a distinct power of two.
    #[test]
    fn flag_constants_are_distinct_powers_of_two() {
        let flags = [
            ("GET_PRICE",          CapabilityFlag::GET_PRICE),
            ("GET_PRICE_DATA",     CapabilityFlag::GET_PRICE_DATA),
            ("IS_ORACLE_HEALTHY",  CapabilityFlag::IS_ORACLE_HEALTHY),
            ("ORACLE_REGISTRY",    CapabilityFlag::ORACLE_REGISTRY),
            ("CONFIDENCE_INTERVAL",CapabilityFlag::CONFIDENCE_INTERVAL),
            ("PRICE_EXPONENT",     CapabilityFlag::PRICE_EXPONENT),
            ("TTL_MANAGEMENT",     CapabilityFlag::TTL_MANAGEMENT),
            ("VERSION_VIEW",       CapabilityFlag::VERSION_VIEW),
        ];

        for (name, value) in flags {
            assert!(value.is_power_of_two(), "{name} is not a power of two: {value:#x}");
        }

        // All distinct — no two constants share a bit.
        let all: &[(&str, u64)] = &flags;
        for i in 0..all.len() {
            for j in (i + 1)..all.len() {
                assert_eq!(
                    all[i].1 & all[j].1,
                    0,
                    "flags {} and {} overlap",
                    all[i].0,
                    all[j].0
                );
            }
        }
    }

    /// The aggregate constant equals the OR of all individual flags.
    #[test]
    fn supported_capabilities_equals_or_of_all_flags() {
        let expected = CapabilityFlag::GET_PRICE
            | CapabilityFlag::GET_PRICE_DATA
            | CapabilityFlag::IS_ORACLE_HEALTHY
            | CapabilityFlag::ORACLE_REGISTRY
            | CapabilityFlag::CONFIDENCE_INTERVAL
            | CapabilityFlag::PRICE_EXPONENT
            | CapabilityFlag::TTL_MANAGEMENT
            | CapabilityFlag::VERSION_VIEW;

        assert_eq!(SUPPORTED_CAPABILITIES, expected);
    }

    /// `capabilities()` returns the same value as `SUPPORTED_CAPABILITIES`.
    #[test]
    fn capabilities_fn_returns_supported_capabilities() {
        assert_eq!(capabilities(), SUPPORTED_CAPABILITIES);
    }

    /// Bit positions are stable: each flag must occupy its documented bit.
    #[test]
    fn flag_bit_positions_are_stable() {
        assert_eq!(CapabilityFlag::GET_PRICE,           1 << 0, "GET_PRICE must be bit 0");
        assert_eq!(CapabilityFlag::GET_PRICE_DATA,      1 << 1, "GET_PRICE_DATA must be bit 1");
        assert_eq!(CapabilityFlag::IS_ORACLE_HEALTHY,   1 << 2, "IS_ORACLE_HEALTHY must be bit 2");
        assert_eq!(CapabilityFlag::ORACLE_REGISTRY,     1 << 3, "ORACLE_REGISTRY must be bit 3");
        assert_eq!(CapabilityFlag::CONFIDENCE_INTERVAL, 1 << 4, "CONFIDENCE_INTERVAL must be bit 4");
        assert_eq!(CapabilityFlag::PRICE_EXPONENT,      1 << 5, "PRICE_EXPONENT must be bit 5");
        assert_eq!(CapabilityFlag::TTL_MANAGEMENT,      1 << 6, "TTL_MANAGEMENT must be bit 6");
        assert_eq!(CapabilityFlag::VERSION_VIEW,        1 << 7, "VERSION_VIEW must be bit 7");
    }

    /// No reserved (undocumented) bits are set in the supported bitmap.
    #[test]
    fn no_reserved_bits_are_set() {
        // Bits 0-7 are assigned; bits 8-63 must remain clear.
        let defined_mask: u64 = (1 << 8) - 1; // 0x00FF
        assert_eq!(
            SUPPORTED_CAPABILITIES & !defined_mask,
            0,
            "bits above 7 must be zero; found {:#018x}",
            SUPPORTED_CAPABILITIES
        );
    }

    /// Combining flags with OR and isolating with AND works as expected.
    #[test]
    fn bitwise_operations_are_correct() {
        let caps = capabilities();

        // Every documented flag is present.
        assert_ne!(caps & CapabilityFlag::GET_PRICE,           0);
        assert_ne!(caps & CapabilityFlag::GET_PRICE_DATA,      0);
        assert_ne!(caps & CapabilityFlag::IS_ORACLE_HEALTHY,   0);
        assert_ne!(caps & CapabilityFlag::ORACLE_REGISTRY,     0);
        assert_ne!(caps & CapabilityFlag::CONFIDENCE_INTERVAL, 0);
        assert_ne!(caps & CapabilityFlag::PRICE_EXPONENT,      0);
        assert_ne!(caps & CapabilityFlag::TTL_MANAGEMENT,      0);
        assert_ne!(caps & CapabilityFlag::VERSION_VIEW,        0);
    }

    /// A hypothetical future flag (bit 8) is not yet set.
    #[test]
    fn future_flags_not_yet_set() {
        let hypothetical_future_flag: u64 = 1 << 8;
        assert_eq!(
            capabilities() & hypothetical_future_flag,
            0,
            "bit 8 must remain clear until officially assigned"
        );
    }
}
