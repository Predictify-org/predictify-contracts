//! Capability flags for the storage subsystem.
//!
//! These flags occupy bits 11-20 of the contract-wide `u64` bitmap returned by
//! `PredictifyHybrid::capabilities()`. Bits 0-10 remain assigned to recovery
//! features. Once assigned, a bit must not be reused for a different feature.

/// Automatic TTL extension and storage-rent preflight checks.
pub const TTL_MANAGEMENT: u64 = 1 << 11;
/// Market-data compression and compressed-record support.
pub const DATA_COMPRESSION: u64 = 1 << 12;
/// Removal of expired or obsolete market data.
pub const DATA_CLEANUP: u64 = 1 << 13;
/// Versioned storage-format migrations.
pub const FORMAT_MIGRATION: u64 = 1 << 14;
/// Promotion and demotion between persistent and temporary storage tiers.
pub const TIER_MIGRATION: u64 = 1 << 15;
/// Storage-usage monitoring and statistics views.
pub const USAGE_MONITORING: u64 = 1 << 16;
/// Per-market storage-layout optimization.
pub const LAYOUT_OPTIMIZATION: u64 = 1 << 17;
/// Per-market storage integrity validation.
pub const INTEGRITY_VALIDATION: u64 = 1 << 18;
/// Read and update support for storage configuration.
pub const CONFIGURATION: u64 = 1 << 19;
/// Storage cost, efficiency-score, and recommendation views.
pub const COST_ANALYTICS: u64 = 1 << 20;

/// Bitmap of all storage features supported by this contract build.
pub const SUPPORTED: u64 = TTL_MANAGEMENT
    | DATA_COMPRESSION
    | DATA_CLEANUP
    | FORMAT_MIGRATION
    | TIER_MIGRATION
    | USAGE_MONITORING
    | LAYOUT_OPTIMIZATION
    | INTEGRITY_VALIDATION
    | CONFIGURATION
    | COST_ANALYTICS;

#[cfg(test)]
mod tests {
    use super::*;

    const FLAGS: [u64; 10] = [
        TTL_MANAGEMENT,
        DATA_COMPRESSION,
        DATA_CLEANUP,
        FORMAT_MIGRATION,
        TIER_MIGRATION,
        USAGE_MONITORING,
        LAYOUT_OPTIMIZATION,
        INTEGRITY_VALIDATION,
        CONFIGURATION,
        COST_ANALYTICS,
    ];

    #[test]
    fn bit_positions_are_stable() {
        assert_eq!(TTL_MANAGEMENT, 1 << 11);
        assert_eq!(DATA_COMPRESSION, 1 << 12);
        assert_eq!(DATA_CLEANUP, 1 << 13);
        assert_eq!(FORMAT_MIGRATION, 1 << 14);
        assert_eq!(TIER_MIGRATION, 1 << 15);
        assert_eq!(USAGE_MONITORING, 1 << 16);
        assert_eq!(LAYOUT_OPTIMIZATION, 1 << 17);
        assert_eq!(INTEGRITY_VALIDATION, 1 << 18);
        assert_eq!(CONFIGURATION, 1 << 19);
        assert_eq!(COST_ANALYTICS, 1 << 20);
    }

    #[test]
    fn flags_are_distinct_single_bits() {
        for (index, flag) in FLAGS.iter().enumerate() {
            assert!(
                flag.is_power_of_two(),
                "flag at index {index} is not a single bit"
            );

            for other in FLAGS.iter().skip(index + 1) {
                assert_eq!(flag & other, 0, "storage capability flags overlap");
            }
        }
    }

    #[test]
    fn supported_bitmap_contains_exactly_the_documented_flags() {
        let expected = FLAGS
            .iter()
            .copied()
            .fold(0u64, |bitmap, flag| bitmap | flag);

        assert_eq!(SUPPORTED, expected);
        assert_eq!(SUPPORTED & ((1u64 << 11) - 1), 0);
        assert_eq!(SUPPORTED & !((1u64 << 21) - 1), 0);
    }
}
