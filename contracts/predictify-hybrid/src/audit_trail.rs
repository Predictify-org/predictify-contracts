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

/// Legacy V1 record in the immutable, tamper-evident audit trail.
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

/// Compact representation of an audit trail entry (V2).
/// Uses a Symbol action code and a u8 reason index instead of verbose maps/strings
/// to significantly cut persistent-storage rent costs while preserving decodability and chain integrity.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEntryV2 {
    pub index: u64,
    pub action: Symbol,
    pub reason_idx: u32,
    pub actor: Address,
    pub ts: u64,
    pub ref_id: BytesN<32>,
    pub prev_record_hash: BytesN<32>,
    pub override_nonce: Option<u64>,
}

/// Versioned wrapper for audit trail entries supporting both legacy V1 and compact V2 formats.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuditRecordVersion {
    V1(AuditRecord),
    V2(AuditEntryV2),
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

    /// Storage key for the admin-managed reason table
    fn reason_table_key(env: &Env) -> Symbol {
        Symbol::new(env, "AUDIT_REASONS")
    }

    /// Registers a new reason in the append-only reason table (admin-gated).
    /// Returns the u32 index assigned to the reason.
    pub fn add_reason(
        env: &Env,
        admin: &Address,
        reason: String,
    ) -> Result<u32, crate::err::Error> {
        // Require admin authentication
        admin.require_auth();
        let stored_admin: Address = env.storage().persistent().get(&Symbol::new(env, "Admin")).ok_or(crate::err::Error::AdminNotSet)?;
        if admin != &stored_admin {
            return Err(crate::err::Error::Unauthorized);
        }

        let mut reasons: Vec<String> = env
            .storage()
            .persistent()
            .get(&Self::reason_table_key(env))
            .unwrap_or(Vec::new(env));

        if reasons.len() >= 256 {
            return Err(crate::err::Error::ReasonTableFull);
        }

        let idx = reasons.len() as u32;
        reasons.push_back(reason);

        env.storage()
            .persistent()
            .set(&Self::reason_table_key(env), &reasons);

        Ok(idx)
    }

    /// Retrieves a reason from the table by its u32 index.
    pub fn get_reason(env: &Env, index: u32) -> Option<String> {
        let reasons: Vec<String> = env
            .storage()
            .persistent()
            .get(&Self::reason_table_key(env))?;

        if (index as u32) < reasons.len() {
            Some(reasons.get(index as u32).unwrap())
        } else {
            None
        }
    }

    /// Returns all registered reasons in the table.
    pub fn get_reasons(env: &Env) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&Self::reason_table_key(env))
            .unwrap_or(Vec::new(env))
    }

    /// Appends a new legacy V1 record to the audit trail.
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

        use soroban_sdk::xdr::ToXdr;
        let record_bytes = record.clone().to_xdr(env);
        let new_hash: BytesN<32> = env.crypto().sha256(&record_bytes).into();

        head.latest_index = new_index;
        head.latest_hash = new_hash;
        env.storage().persistent().set(&Self::head_key(env), &head);

        new_index
    }

    /// Appends a compact V2 record to the tamper-evident audit trail.
    pub fn append_record_v2(
        env: &Env,
        action: Symbol,
        reason_idx: u32,
        actor: Address,
        ref_id: BytesN<32>,
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

        let record = AuditEntryV2 {
            index: new_index,
            action,
            reason_idx,
            actor,
            ts: env.ledger().timestamp(),
            ref_id,
            prev_record_hash: head.latest_hash.clone(),
            override_nonce,
        };

        let record_key = (Symbol::new(env, "AUDIT_REC"), new_index);
        env.storage().persistent().set(&record_key, &record);

        use soroban_sdk::xdr::ToXdr;
        let record_bytes = record.clone().to_xdr(env);
        let new_hash: BytesN<32> = env.crypto().sha256(&record_bytes).into();

        head.latest_index = new_index;
        head.latest_hash = new_hash;
        env.storage().persistent().set(&Self::head_key(env), &head);

        new_index
    }

    /// Retrieves a specific audit record by index (legacy V1).
    pub fn get_record(env: &Env, index: u64) -> Option<AuditRecord> {
        let record_key = (Symbol::new(env, "AUDIT_REC"), index);
        env.storage().persistent().get(&record_key)
    }

    /// Retrieves a specific audit record by index, supporting both V1 and V2 records.
    pub fn get_record_versioned(env: &Env, index: u64) -> Option<AuditRecordVersion> {
        let record_key = (Symbol::new(env, "AUDIT_REC"), index);
        if let Some(v2) = env.storage().persistent().get::<_, AuditEntryV2>(&record_key) {
            return Some(AuditRecordVersion::V2(v2));
        }
        if let Some(v1) = env.storage().persistent().get::<_, AuditRecord>(&record_key) {
            return Some(AuditRecordVersion::V1(v1));
        }
        None
    }

    /// Retrieves the latest records from the audit trail (legacy V1).
    pub fn get_latest_records(env: &Env, limit: u64) -> Vec<AuditRecord> {
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

    /// Retrieves the latest versioned records from the audit trail (V1 & V2).
    pub fn get_latest_records_versioned(env: &Env, limit: u64) -> Vec<AuditRecordVersion> {
        let head_opt = Self::get_head(env);
        if head_opt.is_none() {
            return Vec::new(env);
        }

        let head = head_opt.unwrap();
        let mut records = Vec::new(env);
        let mut current_index = head.latest_index;
        let mut count = 0;

        while current_index > 0 && count < limit {
            if let Some(record) = Self::get_record_versioned(env, current_index) {
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
            let versioned_opt = Self::get_record_versioned(env, current_index);
            if versioned_opt.is_none() {
                return false;
            }

            let versioned = versioned_opt.unwrap();
            let (actual_hash, prev_hash): (BytesN<32>, BytesN<32>) = match versioned {
                AuditRecordVersion::V1(v1) => {
                    let prev = v1.prev_record_hash.clone();
                    let record_bytes = v1.to_xdr(env);
                    (env.crypto().sha256(&record_bytes).into(), prev)
                }
                AuditRecordVersion::V2(v2) => {
                    let prev = v2.prev_record_hash.clone();
                    let record_bytes = v2.to_xdr(env);
                    (env.crypto().sha256(&record_bytes).into(), prev)
                }
            };

            if actual_hash != expected_hash {
                return false;
            }

            expected_hash = prev_hash;
            current_index -= 1;
            checked += 1;
        }

        true
    }
}
