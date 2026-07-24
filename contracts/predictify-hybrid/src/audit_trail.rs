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
pub struct AuditTrailHead {
    pub latest_index: u64,
    pub latest_hash: BytesN<32>,
}

pub struct AuditTrailManager;

impl AuditTrailManager {
    /// Storage key for the global audit trail head.
    fn head_key(env: &Env) -> Symbol {
        Symbol::new(env, "AUDIT_HEAD")
    }

    /// Storage key for a per-market audit trail head.
    fn market_head_key(env: &Env, market_id: &Symbol) -> (Symbol, Symbol) {
        (Symbol::new(env, "AUDIT_M_HEAD"), market_id.clone())
    }

    /// Storage key for a per-market audit record.
    fn market_record_key(env: &Env, market_id: &Symbol, index: u64) -> (Symbol, Symbol, u64) {
        (Symbol::new(env, "AUDIT_M_REC"), market_id.clone(), index)
    }

    /// Appends a new record to the audit trail.
    pub fn append_record(
        env: &Env,
        action: AuditAction,
        actor: Address,
        details: Map<Symbol, String>,
        override_nonce: Option<u64>,
    ) -> u64 {
        Self::append_record_with_market(env, &None, action, actor, details, override_nonce)
    }

    /// Appends a new per-market record to the audit trail when a market identifier is supplied.
    pub fn append_market_record(
        env: &Env,
        market_id: &Symbol,
        action: AuditAction,
        actor: Address,
        details: Map<Symbol, String>,
        override_nonce: Option<u64>,
    ) -> u64 {
        Self::append_record_with_market(env, &Some(market_id.clone()), action, actor, details, override_nonce)
    }

    fn append_record_with_market(
        env: &Env,
        market_id: &Option<Symbol>,
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
            action: action.clone(),
            actor: actor.clone(),
            timestamp: env.ledger().timestamp(),
            details: details.clone(),
            prev_record_hash: head.latest_hash.clone(),
            override_nonce,
        };

        let record_key = (Symbol::new(env, "AUDIT_REC"), new_index);
        env.storage().persistent().set(&record_key, &record);

        if let Some(market_id) = market_id {
            let mut market_head: AuditTrailHead = env
                .storage()
                .persistent()
                .get(&Self::market_head_key(env, market_id))
                .unwrap_or(AuditTrailHead {
                    latest_index: 0,
                    latest_hash: BytesN::from_array(env, &[0u8; 32]),
                });

            let market_index = market_head.latest_index + 1;
            let market_record = AuditRecord {
                index: market_index,
                action,
                actor,
                timestamp: env.ledger().timestamp(),
                details,
                prev_record_hash: market_head.latest_hash.clone(),
                override_nonce: None,
            };

            let market_record_key = Self::market_record_key(env, market_id, market_index);
            env.storage().persistent().set(&market_record_key, &market_record);

            use soroban_sdk::xdr::ToXdr;
            let market_record_bytes = market_record.clone().to_xdr(env);
            let market_new_hash: BytesN<32> = env.crypto().sha256(&market_record_bytes).into();

            market_head.latest_index = market_index;
            market_head.latest_hash = market_new_hash;
            env.storage().persistent().set(&Self::market_head_key(env, market_id), &market_head);
        }

        use soroban_sdk::xdr::ToXdr;
        let record_bytes = record.clone().to_xdr(env);
        let new_hash: BytesN<32> = env.crypto().sha256(&record_bytes).into();

        head.latest_index = new_index;
        head.latest_hash = new_hash;
        env.storage().persistent().set(&Self::head_key(env), &head);

        new_index
    }

    /// Retrieves a specific audit record by index.
    pub fn get_record(env: &Env, index: u64) -> Option<AuditRecord> {
        let record_key = (Symbol::new(env, "AUDIT_REC"), index);
        env.storage().persistent().get(&record_key)
    }

    /// Retrieves a specific per-market audit record by index.
    pub fn get_market_record(env: &Env, market_id: &Symbol, index: u64) -> Option<AuditRecord> {
        let record_key = Self::market_record_key(env, market_id, index);
        env.storage().persistent().get(&record_key)
    }

    /// Retrieves the latest records from the audit trail.
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

    /// Retrieves the latest records from a per-market audit trail.
    pub fn get_market_latest_records(env: &Env, market_id: &Symbol, limit: u64) -> Vec<AuditRecord> {
        let head_opt = Self::get_market_head(env, market_id);
        if head_opt.is_none() {
            return Vec::new(env);
        }

        let head = head_opt.unwrap();
        let mut records = Vec::new(env);
        let mut current_index = head.latest_index;
        let mut count = 0;

        while current_index > 0 && count < limit {
            if let Some(record) = Self::get_market_record(env, market_id, current_index) {
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

    /// Retrieves the head of a per-market audit trail.
    pub fn get_market_head(env: &Env, market_id: &Symbol) -> Option<AuditTrailHead> {
        env.storage().persistent().get(&Self::market_head_key(env, market_id))
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

            let record = record_opt.unwrap();
            let record_bytes = record.clone().to_xdr(env);
            let actual_hash: BytesN<32> = env.crypto().sha256(&record_bytes).into();

            if actual_hash != expected_hash {
                return false;
            }

            expected_hash = record.prev_record_hash;
            current_index -= 1;
            checked += 1;
        }

        true
    }
}
