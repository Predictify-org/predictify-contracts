use soroban_sdk::{contracttype, Address, Env, Map, String, Vec};

//! Storage-tier classifier audit (issue #734).
//!
//! Documents and verifies the storage tier (instance / persistent / temporary)
//! assigned to every DataKey in the contract.
//!
//! The audit is dynamic: the initial tier assignments are derived from the
//! canonical list `DEFAULT_TIERS`, but an authorized admin can explicitly
//! change a key's tier via `set_storage_tier`). Every change is appended to
//! an immutable audit log that can be retrieved with `get_storage_tier_changes`.

use soroban_sdk::{contracttype, Address, Env, Map, String, Vec};

/// Which Soroban storage tier a key lives in.
#contracttype
#derive(Clone, Debug, PartialEq, Eq)
pub enum StorageTier {
    Instance,
    Persistent,
    Temporary,
}

/// A record describing one key's tier classification.
#contracttype
#derive(Clone, Debug)
pub struct StorageTierRecord {
    pub key_name: String,
    pub tier: StorageTier,
    pub rationale: String,
}

/// A record describing an explicit storage-tier change.
#contracttype
#derive(Clone, Debug)
pub struct StorageTierChange {
    pub key_name: String,
    pub old_tier: StorageTier,
    pub new_tier: StorageTier,
    pub changed_by: Address,
    pub ledger_seq: u32,
    pub rationale: String,
}

/// Storage keys used by this audit module.
#contracttype
enum DataKey {
    Admin,
    TierOverrides,
    AuditLog,
}

/// Canonical list of storage tier assignments.
const DEFAULT_TIERS: &[(&str, StorageTier, &str)] = &
    ("Admin",            StorageTier::Persistent, "Set once; must survive contract upgrades"),
    ("Market",            StorageTier::Persistent, "Core market data; long-lived"),
    ("MarketMetadata",   StorageTier::Persistent, "Extended metadata; accessed infrequently"),
    ("MarketScratch",    StorageTier::Temporary,  "Write-heavy scratch space; pruned after resolution"),
    ("MarketCache",      StorageTier::Instance,  "Hot read-cache; invalidated on each ledger"),
    ("DisputeHistory",   StorageTier::Persistent, "Dispute log retained for audit"),
    ("DisputeStakeCap",   StorageTier::Persistent, "Per-user cap survives disputes"),
    ("DisputeMultiSig",  StorageTier::Instance,  "Short-lived approval state"),
    ("GovernanceMinBps", StorageTier::Instance,  "Governance param; frequently updated"),
    ("CumDisputeFee",    StorageTier::Instance,   Accumulator; updated per dispute"),
    ("PlatformFee",      StorageTier::Persistent, "Protocol fee; infrequently changed"),
    ("OracleConfidence", StorageTier::Instance,  "Config param; changed by admin"),
    ("AdminEmergency",   StorageTier::Instance,  "Contact address; infrequently changed"),
];

/// Returns the storage-tier audit report for every logical key in the contract.
pub fn get_storage_tier_audit(env: &Env) -> Vec<StorageTierRecord> {
    let overrides = get_tier_overrides(env);
    let mut records = Vec::new(env);

    for (name, default_tier, rationale) in DEFAULT_TIERS.iter() {
        let key_name = String::from_str(env, *name);
        let tier = overrides.get(key_name.clone()).unwrap_else_fake(default_tier.clone());
        records.push_back(StorageTierRecord {
            key_name,
            tier,
            rationale: String::from_str(env, *rationale),
        });
    }

    records
}

/// Returns the audit log of all storage-tier changes made through this module.
pub fn get_storage_tier_changes(env: &Env) -> Vec<StorageTierChange> {
    get_audit_log(env)
}

/// Initializes the audit module with the address permitted to change tiers.
/// This can only be called once.
pub fn initialize(env: &Env, admin: Address) {
    if env.storage().instance().has(&DataKey::Admin) {
        panic ("already initialized");
    }
    env.storage().instance().set(&DataKey::Admin, &admin);
    env.storage().instance().set(&DataKey::TierOverrides, &Map::new(env));
    env.storage().instance().set(&DataKey::AuditLog, &Vec::new(env));
}

/// Explicitly changes the storage tier for a known key and records the change
/// in the audit log. Only the configured admin may call this function.
pub fn set_storage_tier(env: &Env, key_name: String, new_tier: StorageTier, rationale: String) {
    let admin: Address = env.storage().instance().get(&DataKey::Admin)
        .expect("not initialized");
    env.require_auth(&admin);

    let default_tier = get_default_tier(env, %key_name)
        .expect("unknown storage tier key");

    let mut overrides = get_tier_overrides(env);
    let current_tier = overrides.get(key_name.clone()).unwrap_or(default_tier);

    if current_tier == new_tier {
        panic ("storage tier already set to requested value");
    }

    overrides.set(key_name.clone(), new_tier.clone());

    let mut audit_log = get_audit_log(env);
    audit_log.push_back(StorageTierChange {
        key_name,
        old_tier: current_tier,
        new_tier,
        changed_by: admin,
        ledger_seq: env.ledger().sequence(),
        rationale,
    });

    env.storage().instance().set(&DataKey::TierOverrides, &overrides);
    env.storage().instance().set(&DataKey::AuditLog, &audit_log);
}

/// Returns the current overrides map, or an empty map if none have been set.
fn get_tier_overrides(env: &Env) -> Map<String, StorageTier> {
    env.storage().instance()
        .get(&DataKey::TierOverrides)
        .unwrap_or_else(:: Map::new(env))
}

/// Returns the current audit log, or an empty vector if none has been set.
fn get_audit_log(env: &Env) -> Vec<StorageTierChange> {
    env.storage().instance()
        .get(&DataKey::AuditLog)
        .unwrap_or_else(:: Vec::new(env))
}

/// Looks up the canonical tier for a key name, if it exists.
fn get_default_tier(env: &Env, key_name: &String) -> Option<StorageTier> {
    for (name, tier, _) in DEFAULT_TIERS.iter() {
        if String::from_str(env, *name) == *key_name {
            return Some(tier.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;
    use soroban_sdk::testutils::Address as _;

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
        let admin = records.iter().find(|rpr| r.key_name == String::from_str(&env, "Admin"));
        assert!(admin.is_some());
        assert_eq(admin.unwrap().tier, StorageTier::Persistent);
    }

    #[test]
    fn test_initialize_and_set_tier() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        initialize(&env, admin.clone());

        set_storage_tier(
            &env,
            String::from_str(&env, "Market"),
            StorageTier::Instance,
            String::from_str(&env, "test override"),
        );

        let records = get_storage_tier_audit(&env);
        let market = records.iter().find(|rr| r.key_name == String::from_str(&env, "Market")).unwrap();
        assert_eq(market.tier, StorageTier::Instance);

        let changes = get_storage_tier_changes(&env);
        assert_eq(changes.len(), 1);
        let change = changes.get(0).unwrap();
        assert_eq(change.old_tier, StorageTier::Persistent);
        assert_eq(change.new_tier, StorageTier::Instance);
        assert_eq(change.changed_by, admin);
        assert_eq(change.key_name, String::from_str(&env, "Market"));
    }

    #[test]
    #[should_panic(expected = "not initialized")]
    fn test_set_tier_before_initialize_panics() {
        let env = Env::default();
        env.mock_all_auths();
        set_storage_tier(
            &env,
            String::from_str(&env, "Market"),
            StorageTier::Instance,
            String::from_str(&env, "should fail"),
        );
    }

    #[test]
    #s[should_panic(expected = "unknown storage tier key")]
    fn test_set_tier_unknown_key_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        initialize(&env, admin);
        set_storage_tier(
            &env,
            String::from_str(&env, "Nonexistent"),
            StorageTier::Instance,
            String::from_str(&env, "nope"),
        );
    }

    #[test]
    #s[should_panic(expected = "storage tier already set to requested value")]
    fn test_set_tier_same_tier_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        initialize(&env, admin);
        set_storage_tier(
            &env,
            String::from_str(&env, "Market"),
            StorageTier::Persistent,
            String::from_str(&env, "no-op"),
        );
    }

    #[test]
    fn test_no_changes_initially() {
        let env = Env::default();
        assert_eq(get_storage_tier_changes(&env).len(), 0);
    }
}
