//! Contract capability discovery via a u64 bitmap.
//!
//! Each publicly-advertised contract feature occupies one bit in a 64-bit
//! integer. Clients can discover which features are safe to call without
//! inspecting the Wasm or relying on version-number heuristics.
//!
//! # Usage
//!
//! ```text
//! let caps = client.capabilities();
//! if caps & capability::VERSIONING != 0 {
//!     // versioning is supported
//! }
//! ```
//!
//! # Bit assignments
//!
//! | Bit | Mask              | Capability            |
//! |-----|-------------------|-----------------------|
//! |  0  | 0x0000_0000_0001  | Versioning            |
//! |  1  | 0x0000_0000_0002  | Upgrade management    |
//! |  2  | 0x0000_0000_0004  | Query functions       |
//! |  3  | 0x0000_0000_0008  | Market management     |
//! |  4  | 0x0000_0000_0010  | Betting               |
//! |  5  | 0x0000_0000_0020  | Disputes              |
//! |  6  | 0x0000_0000_0040  | Oracle integration    |
//! |  7  | 0x0000_0000_0080  | Governance            |
//! |  8  | 0x0000_0000_0100  | Analytics             |
//! |  9  | 0x0000_0000_0200  | Monitoring            |
//! | 10  | 0x0000_0000_0400  | Fee management        |
//! | 11  | 0x0000_0000_0800  | Audit trail           |
//! | 12  | 0x0000_0000_1000  | Circuit breaker       |
//! | 13  | 0x0000_0000_2000  | Rate limiting         |
//! | 14  | 0x0000_0000_4000  | Event archive         |
//! | 15  | 0x0000_0000_8000  | Metadata commitment   |
//! | 16  | 0x0000_0001_0000  | Batch operations      |
//! | 17  | 0x0000_0002_0000  | Recovery              |
//! | 18  | 0x0000_0004_0000  | Multi-admin/multisig  |
//! | 19  | 0x0000_0008_0000  | Statistics            |
//! | 20  | 0x0000_0010_0000  | Token registry        |
//! | 21  | 0x0000_0020_0000  | Event visibility      |
//! | 22  | 0x0000_0040_0000  | Claim idempotency     |
//! | 23  | 0x0000_0080_0000  | Bet cancellation      |
//! | 24  | 0x0000_0100_0000  | Fee withdrawal        |
//! | 25  | 0x0000_0200_0000  | Payout distribution   |
//!
//! Unused bits (26–63) are reserved for future capabilities.

use soroban_sdk::Env;

/// Bit position constants for individual capabilities.
///
/// These constants define the bit mask for each capability in the u64
/// bitmap returned by [`capabilities()`].  Clients test for a capability
/// by masking with bitwise AND:
///
/// ```text
/// let caps = client.capabilities();
/// if caps & capability::VERSIONING != 0 {
///     // versioning is available
/// }
/// ```
pub mod capability {
    /// Contract version tracking and history (bit 0).
    pub const VERSIONING: u64 = 1 << 0;
    /// Upgrade management including migration support (bit 1).
    pub const UPGRADE_MANAGEMENT: u64 = 1 << 1;
    /// Public read-only query functions (bit 2).
    pub const QUERY_FUNCTIONS: u64 = 1 << 2;
    /// Market creation and lifecycle management (bit 3).
    pub const MARKET_MANAGEMENT: u64 = 1 << 3;
    /// Bet placement, cancellation, and analytics (bit 4).
    pub const BETTING: u64 = 1 << 4;
    /// Dispute filing, voting, and resolution (bit 5).
    pub const DISPUTES: u64 = 1 << 5;
    /// Oracle price-feed integration (Reflector, Pyth, etc.) (bit 6).
    pub const ORACLE_INTEGRATION: u64 = 1 << 6;
    /// On-chain governance proposals and voting (bit 7).
    pub const GOVERNANCE: u64 = 1 << 7;
    /// Platform analytics, statistics, and leaderboards (bit 8).
    pub const ANALYTICS: u64 = 1 << 8;
    /// Health monitoring, alerting, and graceful degradation (bit 9).
    pub const MONITORING: u64 = 1 << 9;
    /// Fee calculation, collection, and withdrawal (bit 10).
    pub const FEE_MANAGEMENT: u64 = 1 << 10;
    /// Immutable chained audit trail (bit 11).
    pub const AUDIT_TRAIL: u64 = 1 << 11;
    /// Circuit breaker for emergency pausing (bit 12).
    pub const CIRCUIT_BREAKER: u64 = 1 << 12;
    /// Per-operation rate limiting (bit 13).
    pub const RATE_LIMITING: u64 = 1 << 13;
    /// Historical event archive with pruning (bit 14).
    pub const EVENT_ARCHIVE: u64 = 1 << 14;
    /// SHA-256 metadata commitment for market integrity (bit 15).
    pub const METADATA_COMMITMENT: u64 = 1 << 15;
    /// Atomic batch operations (multi-bet, etc.) (bit 16).
    pub const BATCH_OPERATIONS: u64 = 1 << 16;
    /// Error recovery and unclaimed winnings sweep (bit 17).
    pub const RECOVERY: u64 = 1 << 17;
    /// Multi-admin role delegation and multisig (bit 18).
    pub const MULTI_ADMIN_MULTISIG: u64 = 1 << 18;
    /// Platform-wide statistics tracking (bit 19).
    pub const STATISTICS: u64 = 1 << 19;
    /// Token registry with allowed-asset enforcement (bit 20).
    pub const TOKEN_REGISTRY: u64 = 1 << 20;
    /// Public/private event visibility with allowlists (bit 21).
    pub const EVENT_VISIBILITY: u64 = 1 << 21;
    /// Idempotent claim tracking (bit 22).
    pub const CLAIM_IDEMPOTENCY: u64 = 1 << 22;
    /// Bet cancellation with full refund (bit 23).
    pub const BET_CANCELLATION: u64 = 1 << 23;
    /// Admin fee withdrawal with timelock and caps (bit 24).
    pub const FEE_WITHDRAWAL: u64 = 1 << 24;
    /// Automatic payout distribution on resolution (bit 25).
    pub const PAYOUT_DISTRIBUTION: u64 = 1 << 25;

    /// Returns the human-readable name for a capability bit, if known.
    ///
    /// Returns `None` when the bit does not correspond to any documented
    /// capability, or when multiple bits are set in the input.
    pub fn capability_name(bit: u64) -> Option<&'static str> {
        match bit {
            VERSIONING => Some("versioning"),
            UPGRADE_MANAGEMENT => Some("upgrade-management"),
            QUERY_FUNCTIONS => Some("query-functions"),
            MARKET_MANAGEMENT => Some("market-management"),
            BETTING => Some("betting"),
            DISPUTES => Some("disputes"),
            ORACLE_INTEGRATION => Some("oracle-integration"),
            GOVERNANCE => Some("governance"),
            ANALYTICS => Some("analytics"),
            MONITORING => Some("monitoring"),
            FEE_MANAGEMENT => Some("fee-management"),
            AUDIT_TRAIL => Some("audit-trail"),
            CIRCUIT_BREAKER => Some("circuit-breaker"),
            RATE_LIMITING => Some("rate-limiting"),
            EVENT_ARCHIVE => Some("event-archive"),
            METADATA_COMMITMENT => Some("metadata-commitment"),
            BATCH_OPERATIONS => Some("batch-operations"),
            RECOVERY => Some("recovery"),
            MULTI_ADMIN_MULTISIG => Some("multi-admin-multisig"),
            STATISTICS => Some("statistics"),
            TOKEN_REGISTRY => Some("token-registry"),
            EVENT_VISIBILITY => Some("event-visibility"),
            CLAIM_IDEMPOTENCY => Some("claim-idempotency"),
            BET_CANCELLATION => Some("bet-cancellation"),
            FEE_WITHDRAWAL => Some("fee-withdrawal"),
            PAYOUT_DISTRIBUTION => Some("payout-distribution"),
            _ => None,
        }
    }
}

/// Returns the full capabilities bitmap for this contract.
///
/// The returned `u64` is a bitwise-OR of all currently active capability
/// flags.  This function is a **pure read** that performs no storage writes
/// and emits no events — it is safe to call at any time on any network.
///
/// # Returns
///
/// A `u64` where each set bit represents an active contract capability.
///
/// # Example
///
/// ```text
/// let caps = contract.capabilities();
/// if caps & capability::BETTING != 0 {
///     // the betting subsystem is available
/// }
/// ```
pub fn capabilities(_env: &Env) -> u64 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    /// Verify the bitmap is non-zero (at minimum one capability is set).
    #[test]
    fn test_capabilities_bitmap_non_zero() {
        let env = Env::default();
        let caps = capabilities(&env);
        assert!(caps > 0, "capabilities bitmap must have at least one bit set");
    }

    /// Verify known capabilities are set in the bitmap.
    #[test]
    fn test_known_capabilities_are_set() {
        let env = Env::default();
        let caps = capabilities(&env);

        assert!(caps & capability::VERSIONING != 0, "versioning");
        assert!(caps & capability::BETTING != 0, "betting");
        assert!(caps & capability::DISPUTES != 0, "disputes");
        assert!(caps & capability::ORACLE_INTEGRATION != 0, "oracle-integration");
        assert!(caps & capability::GOVERNANCE != 0, "governance");
        assert!(caps & capability::ANALYTICS != 0, "analytics");
        assert!(caps & capability::MARKET_MANAGEMENT != 0, "market-management");
        assert!(caps & capability::QUERY_FUNCTIONS != 0, "query-functions");
        assert!(caps & capability::FEE_MANAGEMENT != 0, "fee-management");
        assert!(caps & capability::AUDIT_TRAIL != 0, "audit-trail");
    }

    /// Verify no undefined bits beyond the last defined are set.
    #[test]
    fn test_no_unexpected_reserved_bits() {
        let env = Env::default();
        let caps = capabilities(&env);

        // Bits 26..63 must be zero (reserved for future use).
        let reserved_mask = !((1u64 << 26) - 1);
        assert_eq!(
            caps & reserved_mask,
            0,
            "bits 26..63 are reserved and must be zero"
        );
    }

    /// Verify capability_name returns expected values for known bits.
    #[test]
    fn test_capability_name_known() {
        assert_eq!(
            capability::capability_name(capability::VERSIONING),
            Some("versioning")
        );
        assert_eq!(
            capability::capability_name(capability::BETTING),
            Some("betting")
        );
        assert_eq!(
            capability::capability_name(capability::PAYOUT_DISTRIBUTION),
            Some("payout-distribution")
        );
    }

    /// Verify capability_name returns None for unknown bits.
    #[test]
    fn test_capability_name_unknown() {
        assert_eq!(capability::capability_name(1u64 << 63), None);
        assert_eq!(capability::capability_name(1u64 << 50), None);
    }

    /// Verify capability_name returns None for multi-bit input.
    #[test]
    fn test_capability_name_multi_bit() {
        let multi = capability::VERSIONING | capability::BETTING;
        assert_eq!(capability::capability_name(multi), None);
    }

    /// Verify that capabilities() is deterministic.
    #[test]
    fn test_capabilities_deterministic() {
        let env = Env::default();
        let caps1 = capabilities(&env);
        let caps2 = capabilities(&env);
        assert_eq!(caps1, caps2);
    }

    /// Verify no panics on repeated calls.
    #[test]
    fn test_capabilities_repeated() {
        let env = Env::default();
        for _ in 0..100 {
            let _ = capabilities(&env);
        }
    }
}
