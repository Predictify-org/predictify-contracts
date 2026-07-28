#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, contracterror, Address, Env, Symbol, Vec, Map, BytesN};

/// Contract error types for migration operations
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    /// Caller is not authorized
    Unauthorized = 1,
    /// Contract not initialized
    NotInitialized = 2,
    /// Contract already initialized
    AlreadyInitialized = 3,
    /// Version mismatch - current version doesn't match expected
    VersionMismatch = 4,
    /// Target version is invalid (same as current or lower)
    InvalidTargetVersion = 5,
    /// Migration data validation failed
    InvalidMigrationData = 6,
    /// Storage migration failed
    MigrationFailed = 7,
}

/// Storage version key
const VERSION_KEY: Symbol = Symbol::new("VERSION");
/// Admin key
const ADMIN_KEY: Symbol = Symbol::new("ADMIN");
/// Migration data prefix
const MIGRATION_PREFIX: Symbol = Symbol::new("MIGRATION_");

/// Migration metadata stored per version upgrade
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MigrationRecord {
    pub from_version: u32,
    pub to_version: u32,
    pub timestamp: u64,
    pub migrated_by: Address,
    pub data_checksum: BytesN<32>,
}

/// Storage layout version 1 - initial schema
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageV1 {
    pub admin: Address,
    pub version: u32,
    pub data: Map<Symbol, Vec<u8>>,
}

/// Storage layout version 2 - added migration history
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageV2 {
    pub admin: Address,
    pub version: u32,
    pub data: Map<Symbol, Vec<u8>>,
    pub migration_history: Vec<MigrationRecord>,
}

/// Current storage version
const CURRENT_VERSION: u32 = 2;

#[contract]
pub struct MigrateContract;

#[contractimpl]
impl MigrateContract {
    /// Initialize the contract with admin and version
    /// Requires auth from admin
    pub fn initialize(env: Env, admin: Address, version: u32) -> Result<(), ContractError> {
        admin.require_auth();
        
        if version == 0 || version > CURRENT_VERSION {
            return Err(ContractError::InvalidTargetVersion);
        }
        
        if env.storage().instance().has(&VERSION_KEY) {
            return Err(ContractError::AlreadyInitialized);
        }
        
        env.storage().instance().set(&ADMIN_KEY, &admin);
        env.storage().instance().set(&VERSION_KEY, &version);
        
        // Initialize empty data map
        let data: Map<Symbol, Vec<u8>> = Map::new(&env);
        env.storage().instance().set(&Symbol::new("DATA\