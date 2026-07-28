//! Storage keys and on-chain data types for the Validators contract.

use soroban_sdk::contracttype;

/// Persistent-storage keys used by the Validators contract.
///
/// Every key variant maps to a single storage slot.  Compound keys (e.g.
/// `Validator(Address)`) include the discriminant in the XDR encoding so
/// different entries cannot collide.
#[contracttype]
pub enum DataKey {
    /// Whether the contract has been successfully initialized.
    Initialized,
    /// The current admin / owner address.
    Admin,
    /// Whether the validators subsystem is globally paused.
    ValidatorsPaused,
    /// Per-validator record keyed by the validator's [`soroban_sdk::Address`].
    Validator(soroban_sdk::Address),
    /// Total number of currently registered validators.
    ValidatorCount,
    /// Global minimum stake threshold (in stroops).
    MinStake,
    /// Global maximum stake cap per validator (in stroops).
    MaxStake,
}

/// On-chain representation of a registered validator.
#[contracttype]
#[derive(Clone)]
pub struct ValidatorInfo {
    /// The validator's Stellar address.
    pub address: soroban_sdk::Address,
    /// Current staked amount in stroops.
    pub stake: i128,
    /// Whether this validator is currently considered active.
    pub active: bool,
    /// Ledger sequence number when the validator was first registered.
    pub registered_at: u32,
    /// Accumulated performance score (application-defined unit).
    pub score: i128,
}
