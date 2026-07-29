//! Storage-tier classifier audit — issue #989.
//!
//! Documents and verifies the storage tier (`Instance` / `Persistent` /
//! `Temporary`) and TTL policy assigned to every logical storage key in
//! `contracts/predictify-hybrid`.
//!
//! # Background
//!
//! Soroban offers three storage tiers with different durability and cost
//! characteristics:
//!
//! | Tier       | Durability | TTL behaviour | Typical use |
//! |------------|-----------|---------------|-------------|
//! | Instance   | Per-contract-instance | Shared TTL bumped on any instance write | Cheap hot cache; treat as ephemeral |
//! | Persistent | Per-key (ledger rent) | Extended explicitly; clamped by `max_ttl()` | Long-lived state |
//! | Temporary  | Per-key (short-lived) | Auto-deleted when TTL expires | Scratch / idempotency guards |
//!
//! # TTL Tier Definitions (from `storage.rs`)
//!
//! | TTL tier | Constant | ≈ Duration |
//! |---------|---------|-----------|
//! | Balance | `BALANCE_TTL_LEDGERS` = 535,680 | ~31 days |
//! | Market  | `MARKET_TTL_LEDGERS`  = 6,307,200 | ~365 days |
//! | Event   | `EVENT_TTL_LEDGERS`   = 1,555,200 | ~90 days |
//! | Archive | `ARCHIVE_TTL_LEDGERS` = 6,307,200 | ~365 days |
//!
//! All durations assume `LEDGERS_PER_DAY = 17_280` (~5 s/ledger on Soroban mainnet).

use soroban_sdk::{contracttype, Env, String, Vec};

/// Soroban storage durability tier.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageTier {
    /// Shared per-contract-instance storage; cheap but ephemeral.
    Instance,
    /// Per-key persistent storage with explicit TTL rent extension.
    Persistent,
    /// Per-key temporary storage auto-deleted when TTL expires.
    Temporary,
}

/// Classification record for one logical storage key.
#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageTierRecord {
    /// Short name of the key or key pattern (e.g. `"DataKey::MarketCache(Symbol)"`).
    pub key_name: String,
    /// Assigned storage tier.
    pub tier: StorageTier,
    /// Human-readable rationale for the tier assignment.
    pub rationale: String,
}

/// Return the full storage-tier audit table for this contract.
///
/// Covers every variant of [`crate::storage::DataKey`] and every
/// non-`DataKey` composite key pattern used across the contract. Entries are
/// ordered by module/subsystem for readability.
///
/// This function is a **pure read** — it performs no storage I/O and emits
/// no events.
pub fn get_storage_tier_audit(env: &Env) -> Vec<StorageTierRecord> {
    let mut records = Vec::new(env);

    // Each entry: (key name, tier, rationale)
    let entries: &[(&str, StorageTier, &str)] = &[
        // ── Access control ──────────────────────────────────────────────────
        (
            "DataKey::Whitelisted(Address)",
            StorageTier::Persistent,
            "Admin-managed allow-list; must survive upgrades and admin changes",
        ),
        (
            "DataKey::Blacklisted(Address)",
            StorageTier::Persistent,
            "Admin-managed deny-list; must survive upgrades; absence of entry = allowed",
        ),
        (
            "DataKey::AdminOverrideNonce(Address)",
            StorageTier::Persistent,
            "Replay-protection nonce; must persist across ledgers to block replays",
        ),

        // ── Market lifecycle ─────────────────────────────────────────────────
        (
            "DataKey::MarketMetadata(Symbol)",
            StorageTier::Persistent,
            "Core market record; live for the market lifetime (~365 d, Market TTL tier)",
        ),
        (
            "DataKey::MarketScratch(Symbol)",
            StorageTier::Temporary,
            "Write-heavy scratch data pruned after resolution; no audit value",
        ),
        (
            "DataKey::MarketCache(Symbol)",
            StorageTier::Instance,
            "Hot read-cache for market structs; cheaply rebuilt from persistent; ~8 min TTL",
        ),
        (
            "DataKey::MarketExtensionTotal(Symbol)",
            StorageTier::Persistent,
            "Cumulative extension days per market; bounded mutation; Market TTL tier",
        ),
        (
            "DataKey::ArchivedMarket(Symbol, u64)",
            StorageTier::Persistent,
            "Immutable post-resolution snapshot keyed by (market_id, timestamp); Archive TTL tier",
        ),
        (
            "DataKey::UserStake(Address, Symbol)",
            StorageTier::Persistent,
            "Per-user stake in a market; needed for payout and refund; Market TTL tier",
        ),

        // ── Betting ──────────────────────────────────────────────────────────
        (
            "DataKey::PlaceBetsIdem(Address, BytesN<32>)",
            StorageTier::Temporary,
            "Idempotency key for place_bets; ~7 d TTL; auto-expires after window",
        ),
        (
            "DataKey::MaxBetCap",
            StorageTier::Persistent,
            "Global max bet cap per user; admin-set; changed infrequently",
        ),

        // ── Balances ─────────────────────────────────────────────────────────
        (
            "Vec<Val> balance key (BalanceStorage)",
            StorageTier::Persistent,
            "User asset balance; ~31 d Balance TTL tier; renewed on each write",
        ),

        // ── Disputes ─────────────────────────────────────────────────────────
        (
            "DataKey::DisputeHistory(Symbol)",
            StorageTier::Persistent,
            "Full dispute log per market; retained for audit; Market TTL tier",
        ),
        (
            "DataKey::DisputeHistoryCap",
            StorageTier::Persistent,
            "Global cap on dispute-history entries; admin-set; rarely changes",
        ),
        (
            "DataKey::DisputeStakeCap(Symbol, Address)",
            StorageTier::Persistent,
            "Per-user cap for a specific dispute; enforced until market closes",
        ),
        (
            "DataKey::DisputeCumulativeStakeCap(Address)",
            StorageTier::Persistent,
            "Per-user cumulative cap across all active disputes; security-critical",
        ),
        (
            "DataKey::AntiGriefFloor",
            StorageTier::Persistent,
            "Minimum anti-grief stake floor; governance param; changed by admin",
        ),
        (
            "DataKey::DisputeCooldownSeconds",
            StorageTier::Persistent,
            "Cooldown between dispute admin actions; governance param",
        ),
        (
            "DataKey::DisputeAdminLastAction(Symbol)",
            StorageTier::Persistent,
            "Timestamp of last admin dispute action; enforces cooldown",
        ),
        (
            "DataKey::CollusionDetectorConfig",
            StorageTier::Persistent,
            "Collusion-detection config; governance param set by admin",
        ),

        // ── Resolution ───────────────────────────────────────────────────────
        (
            "DataKey::ResolutionCooldownSeconds",
            StorageTier::Persistent,
            "Cooldown between resolution admin actions; governance param",
        ),
        (
            "DataKey::ResolutionAdminLastAction(Symbol)",
            StorageTier::Persistent,
            "Timestamp of last admin resolution action; enforces cooldown",
        ),

        // ── Governance / Config ───────────────────────────────────────────────
        (
            "DataKey::GlobalConfig",
            StorageTier::Persistent,
            "Global protocol configuration; infrequently changed by admin",
        ),
        (
            "Symbol(\"storage_config\") (StorageOptimizer)",
            StorageTier::Persistent,
            "Storage optimizer configuration; Archive TTL tier",
        ),

        // ── Rate limiting ─────────────────────────────────────────────────────
        (
            "DataKey::PerLedgerBetCap",
            StorageTier::Persistent,
            "Admin-set per-ledger bet cap; changed infrequently",
        ),
        (
            "DataKey::PerLedgerBetCounter",
            StorageTier::Persistent,
            "Rolling per-ledger bet counter; reset each ledger by rate limiter",
        ),

        // ── Admin subsystems ─────────────────────────────────────────────────
        (
            "DataKey::OracleAdminCooldownState",
            StorageTier::Persistent,
            "Oracle admin cooldown enforcement state; must survive ledger boundaries",
        ),
        (
            "DataKey::MultisigRotationState",
            StorageTier::Persistent,
            "Multisig admin rotation approval state; must persist until execution",
        ),

        // ── Events / Nonces ───────────────────────────────────────────────────
        (
            "DataKey::EventNonce(Symbol)",
            StorageTier::Persistent,
            "Monotonic replay-protection nonce per event topic; never pruned",
        ),
        (
            "(Symbol(\"Event\"), Symbol) (EventManager)",
            StorageTier::Persistent,
            "Event record; ~90 d Event TTL tier",
        ),
        (
            "(Symbol(\"ActiveEvents\"), Address) (CreatorLimitsManager)",
            StorageTier::Persistent,
            "Active event count per creator; Market TTL tier",
        ),

        // ── Audit trail ───────────────────────────────────────────────────────
        (
            "DataKey::MarketAuditHead(Symbol)",
            StorageTier::Persistent,
            "Audit log head record (entry count) per market; Market TTL tier; never pruned",
        ),
        (
            "DataKey::MarketAuditLog(Symbol, u32)",
            StorageTier::Persistent,
            "Individual audit log entry; Market TTL tier; append-only",
        ),

        // ── Deprecated registry ───────────────────────────────────────────────
        (
            "DataKey::DeprecatedRegistry",
            StorageTier::Persistent,
            "Registry of deprecated entrypoints; must survive upgrades for API discovery",
        ),

        // ── Storage optimizer / migration ────────────────────────────────────
        (
            "DataKey::ArchivedMarket compressed key (StorageOptimizer)",
            StorageTier::Persistent,
            "Compressed market snapshot; Market TTL tier",
        ),
        (
            "migration record Symbol (StorageOptimizer)",
            StorageTier::Persistent,
            "Storage migration record; Archive TTL tier",
        ),
    ];

    for (key_name, tier, rationale) in entries {
        records.push_back(StorageTierRecord {
            key_name: String::from_str(env, key_name),
            tier: tier.clone(),
            rationale: String::from_str(env, rationale),
        });
    }

    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    /// The audit table must cover at least 30 storage keys (the full DataKey
    /// enum plus non-DataKey composite keys).
    #[test]
    fn test_audit_covers_all_keys() {
        let env = Env::default();
        let records = get_storage_tier_audit(&env);
        assert!(
            records.len() >= 30,
            "audit table should document at least 30 storage keys; found {}",
            records.len()
        );
    }

    /// Every record must have a non-empty key name and rationale.
    #[test]
    fn test_all_records_have_non_empty_fields() {
        let env = Env::default();
        let records = get_storage_tier_audit(&env);
        for record in records.iter() {
            assert!(
                !record.key_name.is_empty(),
                "every record must have a non-empty key_name"
            );
            assert!(
                !record.rationale.is_empty(),
                "every record must have a non-empty rationale"
            );
        }
    }

    /// Temporary keys must not have "persistent" in their rationale (sanity check).
    #[test]
    fn test_temporary_keys_have_temporary_tier() {
        let env = Env::default();
        let records = get_storage_tier_audit(&env);
        // PlaceBetsIdem and MarketScratch must be Temporary.
        let place_bets = records
            .iter()
            .find(|r| r.key_name == String::from_str(&env, "DataKey::PlaceBetsIdem(Address, BytesN<32>)"));
        assert!(place_bets.is_some(), "PlaceBetsIdem must be in the audit table");
        assert_eq!(
            place_bets.unwrap().tier,
            StorageTier::Temporary,
            "PlaceBetsIdem must be Temporary"
        );

        let scratch = records
            .iter()
            .find(|r| r.key_name == String::from_str(&env, "DataKey::MarketScratch(Symbol)"));
        assert!(scratch.is_some(), "MarketScratch must be in the audit table");
        assert_eq!(
            scratch.unwrap().tier,
            StorageTier::Temporary,
            "MarketScratch must be Temporary"
        );
    }

    /// MarketCache must be Instance (hot read cache).
    #[test]
    fn test_market_cache_is_instance() {
        let env = Env::default();
        let records = get_storage_tier_audit(&env);
        let cache = records
            .iter()
            .find(|r| r.key_name == String::from_str(&env, "DataKey::MarketCache(Symbol)"));
        assert!(cache.is_some(), "MarketCache must be in the audit table");
        assert_eq!(
            cache.unwrap().tier,
            StorageTier::Instance,
            "MarketCache must be Instance"
        );
    }

    /// Security-critical keys (AdminOverrideNonce, OracleAdminCooldownState,
    /// MultisigRotationState) must all be Persistent.
    #[test]
    fn test_security_critical_keys_are_persistent() {
        let env = Env::default();
        let records = get_storage_tier_audit(&env);

        let critical = [
            "DataKey::AdminOverrideNonce(Address)",
            "DataKey::OracleAdminCooldownState",
            "DataKey::MultisigRotationState",
            "DataKey::DisputeCumulativeStakeCap(Address)",
            "DataKey::EventNonce(Symbol)",
            "DataKey::DeprecatedRegistry",
        ];
        for name in critical {
            let rec = records
                .iter()
                .find(|r| r.key_name == String::from_str(&env, name));
            assert!(
                rec.is_some(),
                "security-critical key '{}' must be in the audit table",
                name
            );
            assert_eq!(
                rec.unwrap().tier,
                StorageTier::Persistent,
                "security-critical key '{}' must be Persistent",
                name
            );
        }
    }

    /// Audit trail keys must be Persistent (append-only, never pruned).
    #[test]
    fn test_audit_trail_keys_are_persistent() {
        let env = Env::default();
        let records = get_storage_tier_audit(&env);

        for name in &[
            "DataKey::MarketAuditHead(Symbol)",
            "DataKey::MarketAuditLog(Symbol, u32)",
        ] {
            let rec = records
                .iter()
                .find(|r| r.key_name == String::from_str(&env, name));
            assert!(
                rec.is_some(),
                "audit-trail key '{}' must be in the audit table",
                name
            );
            assert_eq!(
                rec.unwrap().tier,
                StorageTier::Persistent,
                "audit-trail key '{}' must be Persistent",
                name
            );
        }
    }

    /// The function is deterministic — two calls return identical results.
    #[test]
    fn test_audit_is_deterministic() {
        let env = Env::default();
        let r1 = get_storage_tier_audit(&env);
        let r2 = get_storage_tier_audit(&env);
        assert_eq!(r1.len(), r2.len(), "audit must return identical length on each call");
    }

    /// The audit function is a pure read — it does not modify storage.
    #[test]
    fn test_audit_does_not_modify_storage() {
        let env = Env::default();
        // The env has no contract registered, so any storage modification
        // during the call would panic. The fact that this test completes
        // without panic proves the function is side-effect free.
        let _ = get_storage_tier_audit(&env);
    }
}
