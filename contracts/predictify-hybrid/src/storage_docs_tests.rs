//! Regression test for [`../docs/storage.md`](../docs/storage.md) [b#015].
//!
//! Keeps the storage-tier documentation aligned with the `DataKey` enum in
//! [`crate::storage`]: every variant name must appear in the docs, every
//! storage tier must be covered, and the doc must state its API impact.

const STORAGE_DOCUMENTATION: &str = include_str!("../docs/storage.md");

/// Every `DataKey` variant currently declared in `storage.rs`, kept in the
/// same order as the enum. If a variant is added, removed, or renamed, this
/// list (and the corresponding row in `docs/storage.md`) must be updated too.
const DATA_KEY_VARIANTS: &[&str] = &[
    "PlaceBetsIdem",
    "Whitelisted",
    "Blacklisted",
    "ArchivedMarket",
    "MarketExtensionTotal",
    "MarketMetadata",
    "MarketScratch",
    "DisputeHistoryCap",
    "DisputeHistory",
    "DisputeStakeCap",
    "DisputeCumulativeStakeCap",
    "MarketCache",
    "AntiGriefFloor",
    "DisputeCooldownSeconds",
    "DisputeAdminLastAction",
    "ResolutionCooldownSeconds",
    "ResolutionAdminLastAction",
    "GlobalConfig",
    "EventNonce",
    "UserStake",
    "MaxBetCap",
    "UserLastBetTime",
    "CoolOffPeriod",
    "PerMarketCoolOff",
];

#[test]
fn storage_documentation_covers_all_storage_tiers() {
    for required_section in ["Instance", "Persistent", "Temporary"] {
        assert!(
            STORAGE_DOCUMENTATION.contains(required_section),
            "storage documentation must describe the {required_section} tier"
        );
    }
}

#[test]
fn storage_documentation_covers_every_data_key_variant() {
    for variant in DATA_KEY_VARIANTS {
        assert!(
            STORAGE_DOCUMENTATION.contains(variant),
            "storage documentation must document DataKey::{variant}"
        );
    }
}

#[test]
fn storage_documentation_covers_ad_hoc_keys_and_ttl_tiers() {
    for required in [
        "storage_config",
        "\"Balance\"",
        "\"Event\"",
        "\"ActiveEvents\"",
        "BALANCE_TTL_LEDGERS",
        "MARKET_TTL_LEDGERS",
        "EVENT_TTL_LEDGERS",
        "ARCHIVE_TTL_LEDGERS",
    ] {
        assert!(
            STORAGE_DOCUMENTATION.contains(required),
            "storage documentation must cover {required}"
        );
    }
}

#[test]
fn storage_documentation_states_api_impact() {
    assert!(
        STORAGE_DOCUMENTATION.contains("does not change the contract's public API"),
        "storage documentation must state the API impact"
    );
}

#[test]
fn storage_documentation_flags_known_inconsistencies() {
    // `market_analytics.rs` references `DataKey::MarketLeaderboard`, which does
    // not exist on the `DataKey` enum. The docs must call this out rather than
    // silently document a key that isn't there, so this test doubles as a
    // tripwire: once the enum gains (or the reference is removed for) that
    // variant, this assertion — and the corresponding doc section — should be
    // updated together.
    assert!(
        STORAGE_DOCUMENTATION.contains("MarketLeaderboard"),
        "storage documentation must flag the MarketLeaderboard inconsistency"
    );
}
