use soroban_sdk::{contracttype, Address, BytesN, Env, Map, String, Symbol, Vec};

/// Represents the type of sensitive action recorded in the audit trail.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditAction {
    // Admin Actions
    ContractInitialized,
    AdminAdded,
    AdminRemoved,
    AdminRoleUpdated,
    ContractPaused,
    ContractUnpaused,
    AdminTransferred,

    // Market/Event Actions
    MarketCreated,
    EventCreated,
    EventDescriptionUpdated,
    EventOutcomesUpdated,
    EventCategoryUpdated,
    EventTagsUpdated,
    EventCancelled,
    MarketUpdated,

    // Fee Actions
    FeesCollected,
    FeesWithdrawn,
    FeeConfigUpdated,

    // Token & Oracle Actions
    OracleConfigUpdated,
    TokenVerified,
    BetLimitsUpdated,

    // Resolution & Disputes
    MarketResolved,
    MarketForceResolved,
    DisputeCreated,
    DisputeResolved,
    OracleVerificationOverride,

    // Storage & System
    StorageOptimized,
    StorageMigrated,
    ContractUpgraded,
    UpgradeRolledBack,

    // Recovery
    ErrorRecovered,
    PartialRefundExecuted,
}

/// A single record in the immutable, tamper-evident audit trail.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditRecord {
    pub index: u64,
    pub action: AuditAction,
    pub actor: Address,
    pub timestamp: u64,
    pub details: Map<Symbol, String>,
    pub prev_record_hash: BytesN<32>,
    pub override_nonce: Option<u64>,
}

/// Head of the audit trail, tracking the latest state.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEntryV2 {
    pub action: Symbol,
    pub reason_idx: u8,
    pub actor: Address,
    pub ts: u64,
    pub ref_id: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditRecordVersioned {
    V1(AuditRecord),
    V2(AuditEntryV2),
}

pub struct AuditReasonTable;

impl AuditReasonTable {
    pub fn get_reasons(env: &Env) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&crate::storage::DataKey::AuditReasonTable)
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn add_reason(env: &Env, admin: &Address, reason: String) -> u8 {
        admin.require_auth();
        crate::admin::AdminManager::require_admin(env, admin);
        let mut reasons = Self::get_reasons(env);
        let idx = reasons.len() as u8;
        reasons.push_back(reason);
        env.storage()
            .persistent()
            .set(&crate::storage::DataKey::AuditReasonTable, &reasons);
        idx
    }
}

/// Head of the audit trail, tracking the latest state.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditTrailHead {
    pub latest_index: u64,
    pub latest_hash: BytesN<32>,
}

pub struct AuditTrailManager;

impl AuditTrailManager {
    /// Storage key for the audit trail head
    fn head_key(env: &Env) -> Symbol {
        Symbol::new(env, "AUDIT_HEAD")
    }

    /// Appends a new record to the audit trail.
    pub fn append_record(
        env: &Env,
        action: AuditAction,
        actor: Address,
        details: Map<Symbol, String>,
        override_nonce: Option<u64>,
    ) -> u64 {
        let mut head: AuditTrailHead = env
            .storage()
            .persistent()
            .get(&Self::head_key(env))
            .unwrap_or(AuditTrailHead {
                latest_index: 0,
                latest_hash: BytesN::from_array(env, &[0u8; 32]),
            });

        let new_index = head.latest_index + 1;

        let record = AuditRecord {
            index: new_index,
            action,
            actor,
            timestamp: env.ledger().timestamp(),
            details,
            prev_record_hash: head.latest_hash.clone(),
            override_nonce: override_nonce,
        };

        // Use a tuple key for distinct storage namespace (Symbol, index)
        let record_key = (Symbol::new(env, "AUDIT_REC"), new_index);
        env.storage().persistent().set(&record_key, &record);

        // Instead of xdr, let's just use the Soroban bytes macro or hash a simple representation
        // Since we want tamper evidence of the payload, we use ToXdr implemented by the SDK.
        use soroban_sdk::xdr::ToXdr;
        let record_bytes = record.clone().to_xdr(env);
        let new_hash: BytesN<32> = env.crypto().sha256(&record_bytes).into();

        head.latest_index = new_index;
        head.latest_hash = new_hash;
        env.storage().persistent().set(&Self::head_key(env), &head);

        new_index
    }

    /// Retrieves a specific audit record by index.

    pub fn append_record_v2(
        env: &Env,
        action: Symbol,
        actor: Address,
        reason_idx: u8,
    ) -> u64 {
        let mut head: AuditTrailHead = env
            .storage()
            .persistent()
            .get(&Self::head_key(env))
            .unwrap_or(AuditTrailHead {
                latest_index: 0,
                latest_hash: BytesN::from_array(env, &[0u8; 32]),
            });

        let new_index = head.latest_index + 1;

        let record = AuditEntryV2 {
            action,
            reason_idx,
            actor,
            ts: env.ledger().timestamp(),
            ref_id: head.latest_hash.clone(),
        };

        let versioned = AuditRecordVersioned::V2(record);
        let record_key = (Symbol::new(env, "AUDIT_REC"), new_index);
        env.storage().persistent().set(&record_key, &versioned);

        use soroban_sdk::xdr::ToXdr;
        let record_bytes = versioned.clone().to_xdr(env);
        let new_hash: BytesN<32> = env.crypto().sha256(&record_bytes).into();

        head.latest_index = new_index;
        head.latest_hash = new_hash;
        env.storage().persistent().set(&Self::head_key(env), &head);

        new_index
    }

    /// Retrieves a specific audit record by index.
    pub fn get_record(env: &Env, index: u64) -> Option<AuditRecordVersioned> {
        let record_key = (Symbol::new(env, "AUDIT_REC"), index);
        let val_opt: Option<soroban_sdk::Val> = env.storage().persistent().get(&record_key);
        if let Some(val) = val_opt {
            use soroban_sdk::TryFromVal;
            if let Ok(versioned) = AuditRecordVersioned::try_from_val(env, &val) {
                return Some(versioned);
            }
            if let Ok(v1) = AuditRecord::try_from_val(env, &val) {
                return Some(AuditRecordVersioned::V1(v1));
            }
        }
        None
    }

    /// Retrieves the latest records from the audit trail.
    pub fn get_latest_records(env: &Env, limit: u64) -> Vec<AuditRecordVersioned> {
        let head_opt = Self::get_head(env);
        if head_opt.is_none() {
            return Vec::new(env);
        }

        let head = head_opt.unwrap();
        let mut records = Vec::new(env);
        let mut current_index = head.latest_index;
        let mut count = 0;

        while current_index > 0 && count < limit {
            if let Some(record) = Self::get_record(env, current_index) {
                records.push_back(record);
            }
            current_index -= 1;
            count += 1;
        }

        records
    }

    /// Retrieves the head of the audit trail.
    pub fn get_head(env: &Env) -> Option<AuditTrailHead> {
        env.storage().persistent().get(&Self::head_key(env))
    }

    /// Verifies the integrity of the trail from the current head back to a certain depth.
    pub fn verify_integrity(env: &Env, depth: u64) -> bool {
        let head_opt: Option<AuditTrailHead> = env.storage().persistent().get(&Self::head_key(env));
        if head_opt.is_none() {
            return true;
        }

        let head = head_opt.unwrap();
        let mut current_index = head.latest_index;
        let mut expected_hash = head.latest_hash;
        let mut checked = 0;

        use soroban_sdk::xdr::ToXdr;

        while current_index > 0 && checked < depth {
            let record_opt = Self::get_record(env, current_index);
            if record_opt.is_none() {
                return false;
            }

            let versioned_record = record_opt.unwrap();
            let record_bytes = match &versioned_record {
                AuditRecordVersioned::V1(v1) => v1.clone().to_xdr(env),
                AuditRecordVersioned::V2(_) => versioned_record.clone().to_xdr(env),
            };
            let actual_hash: BytesN<32> = env.crypto().sha256(&record_bytes).into();

            if actual_hash != expected_hash {
                return false;
            }

            expected_hash = match &versioned_record {
                AuditRecordVersioned::V1(v1) => v1.prev_record_hash.clone(),
                AuditRecordVersioned::V2(v2) => v2.ref_id.clone(),
            };
            current_index -= 1;
            checked += 1;
        }

        true
    }
}
