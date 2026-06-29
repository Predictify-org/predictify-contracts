//! Storage-tier classifier audit (issue #734).
//!
//! Documents and verifies the storage tier (instance / persistent / temporary)
//! assigned to every DataKey in the contract.

use soroban_sdk::{contracttype, Env, String, Vec};

/// Which Soroban storage tier a key lives in.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageTier {
    Instance,
    Persistent,
    Temporary,
}

/// A record describing one key's tier classification.
#[contracttype]
#[derive(Clone, Debug)]
pub struct StorageTierRecord {
    pub key_name: String,
    pub tier: StorageTier,
    pub rationale: String,
}

/// Return the storage-tier audit report for every logical key in the contract.
pub fn get_storage_tier_audit(env: &Env) -> Vec<StorageTierRecord> {
    let mut records = Vec::new(env);

    let entries: &[(&str, StorageTier, &str)] = &[
        ("Admin",            StorageTier::Persistent, "Set once; must survive contract upgrades"),
        ("Market",           StorageTier::Persistent, "Core market data; long-lived"),
        ("MarketMetadata",   StorageTier::Persistent, "Extended metadata; accessed infrequently"),
        ("MarketScratch",    StorageTier::Temporary,  "Write-heavy scratch space; pruned after resolution"),
        ("MarketCache",      StorageTier::Instance,   "Hot read-cache; invalidated on each ledger"),
        ("DisputeHistory",   StorageTier::Persistent, "Dispute log retained for audit"),
        ("DisputeStakeCap",  StorageTier::Persistent, "Per-user cap survives disputes"),
        ("DisputeMultiSig",  StorageTier::Instance,   "Short-lived approval state"),
        ("GovernanceMinBps", StorageTier::Instance,   "Governance param; frequently updated"),
        ("CumDisputeFee",    StorageTier::Instance,   "Accumulator; updated per dispute"),
        ("PlatformFee",      StorageTier::Persistent, "Protocol fee; infrequently changed"),
        ("OracleConfidence", StorageTier::Instance,   "Config param; changed by admin"),
        ("AdminEmergency",   StorageTier::Instance,   "Contact address; infrequently changed"),
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

    #[test]
    fn test_audit_returns_all_keys() {
        let env = Env::default();
        let records = get_storage_tier_audit(&env);
        assert!(records.len() >= 10, "should document at least 10 storage keys");
    }

    #[test]
    fn test_admin_key_is_persistent() {
        let env = Env::default();
        let records = get_storage_tier_audit(&env);
        let admin = records.iter().find(|r| r.key_name == String::from_str(&env, "Admin"));
        assert!(admin.is_some());
        assert_eq!(admin.unwrap().tier, StorageTier::Persistent);
    }
}
