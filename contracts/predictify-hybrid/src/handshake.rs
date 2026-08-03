//! `handshake` module — cross-contract handshake versioning between
//! the predictify contract and adapter contracts.
//!
//! This module provides infrastructure for negotiating a compatible
//! protocol version with external adapter contracts before performing
//! cross-contract operations.  It implements a simple state machine:
//!
//! ```text
//! Pending → Accepted | Rejected | Expired
//! ```
//!
//! # Design
//!
//! - Each adapter contract is identified by its Soroban `Address`.
//! - A `HandshakeRecord` tracks the proposed version, the negotiated
//!   version (if any), and the current state.
//! - The predictify contract stores a set of *supported* major versions.
//!   Only adapters whose major version is in this set can complete a
//!   handshake.
//! - All state-changing entrypoints enforce `require_auth`.
//! - Overflow-safe math is used throughout; no `unwrap()` appears in
//!   production paths.

use crate::err::Error;
use soroban_sdk::{
    contracttype, panic_with_error, symbol_short, Address, Env, Map, Symbol,
};

// =============================================================================
// Storage types
// =============================================================================

/// A semantic version tuple (major, minor, patch).
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct HandshakeVersion {
    /// Major version — must match for compatibility.
    pub major: u32,
    /// Minor version — higher is better, but minor alone does not break
    /// compatibility.
    pub minor: u32,
    /// Patch version — informational only.
    pub patch: u32,
}

impl HandshakeVersion {
    /// Create a new version.
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
}

/// The state of a handshake negotiation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum HandshakeState {
    /// The handshake has been initiated but not yet responded to.
    Pending,
    /// The adapter has accepted the proposed version.
    Accepted,
    /// The adapter has rejected the proposed version.
    Rejected,
    /// The handshake has expired without a response.
    Expired,
}

/// A single handshake record linking an adapter address to its
/// negotiation state and version information.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct HandshakeRecord {
    /// The adapter contract address.
    pub adapter: Address,
    /// The version proposed by the predictify contract.
    pub proposed_version: HandshakeVersion,
    /// The version accepted by the adapter (only set when state is
    /// `Accepted`).
    pub negotiated_version: Option<HandshakeVersion>,
    /// Current negotiation state.
    pub state: HandshakeState,
    /// Ledger timestamp when the handshake was initiated.
    pub initiated_at: u64,
    /// Ledger timestamp when the handshake was last updated.
    pub updated_at: u64,
}

// =============================================================================
// Events
// =============================================================================

/// Emitted when a new handshake is initiated with an adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct HandshakeInitiatedEvent {
    /// The adapter contract address.
    pub adapter: Address,
    /// The version proposed by the predictify contract.
    pub proposed_version: HandshakeVersion,
    /// The ledger timestamp at initiation.
    pub timestamp: u64,
}

/// Emitted when an adapter accepts a handshake.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct HandshakeAcceptedEvent {
    /// The adapter contract address.
    pub adapter: Address,
    /// The version negotiated and accepted.
    pub negotiated_version: HandshakeVersion,
    /// The ledger timestamp of acceptance.
    pub timestamp: u64,
}

/// Emitted when an adapter rejects a handshake.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct HandshakeRejectedEvent {
    /// The adapter contract address.
    pub adapter: Address,
    /// The version that was proposed and rejected.
    pub proposed_version: HandshakeVersion,
    /// The ledger timestamp of rejection.
    pub timestamp: u64,
}

/// Emitted when a handshake expires without a response.
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct HandshakeExpiredEvent {
    /// The adapter contract address.
    pub adapter: Address,
    /// The version that was proposed.
    pub proposed_version: HandshakeVersion,
    /// The ledger timestamp of expiry.
    pub timestamp: u64,
}

// =============================================================================
// Storage keys
// =============================================================================

const SUPPORTED_VERSIONS_KEY: &str = "supported_versions";
const HANDSHAKE_MAP_KEY: &str = "handshake_map";
const HANDSHAKE_EXPIRY_SECONDS: u64 = 86_400;

// =============================================================================
// Manager
// =============================================================================

/// Manages cross-contract handshake versioning.
pub struct HandshakeManager;

impl HandshakeManager {
    // -------------------------------------------------------------------------
    // Configuration
    // -------------------------------------------------------------------------

    /// Set the list of supported major versions (admin only).
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment.
    /// * `admin` - The administrator address (must be authorised).
    /// * `versions` - A map of major version → maximum allowed minor
    ///   version.  Entries with `major = 0` are rejected.
    ///
    /// # Errors
    ///
    /// - [`Error::HandshakeUnauthorized`] — caller is not the admin.
    /// - [`Error::HandshakeVersionMismatch`] — a major version of 0 is
    ///   provided.
    pub fn set_supported_versions(
        env: &Env,
        admin: &Address,
        versions: Map<u32, u32>,
    ) -> Result<(), Error> {
        admin.require_auth();

        for (major, max_minor) in versions.iter() {
            if major == 0 {
                panic_with_error!(env, Error::HandshakeVersionMismatch);
            }
            if max_minor == 0 {
                panic_with_error!(env, Error::HandshakeVersionMismatch);
            }
        }

        env.storage()
            .instance()
            .set(&Symbol::new(env, SUPPORTED_VERSIONS_KEY), &versions);

        Ok(())
    }

    /// Retrieve the supported versions map.
    ///
    /// Returns an empty map if no versions have been configured yet.
    pub fn get_supported_versions(env: &Env) -> Map<u32, u32> {
        env.storage()
            .instance()
            .get(&Symbol::new(env, SUPPORTED_VERSIONS_KEY))
            .unwrap_or_else(|| Map::new(env))
    }

    // -------------------------------------------------------------------------
    // Handshake lifecycle
    // -------------------------------------------------------------------------

    /// Initiate a handshake with an adapter contract.
    ///
    /// Proposes a specific version to the adapter and records the
    /// handshake as `Pending`.  The caller must be authorised.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment.
    /// * `caller` - The address initiating the handshake (must be
    ///   authorised).
    /// * `adapter` - The adapter contract address.
    /// * `proposed_version` - The version proposed to the adapter.
    ///
    /// # Errors
    ///
    /// - [`Error::HandshakeUnauthorized`] — caller is not authorised.
    /// - [`Error::HandshakeSupportedVersionsNotSet`] — no supported versions
    ///   have been configured.
    /// - [`Error::HandshakeVersionMismatch`] — the proposed major version
    ///   is not in the supported set.
    /// - [`Error::HandshakeVersionMismatch`] — the proposed minor version
    ///   exceeds the maximum allowed for that major version.
    /// - [`Error::HandshakeAlreadyCompleted`] — a handshake with
    ///   this adapter already exists and is terminal.
    pub fn initiate_handshake(
        env: &Env,
        caller: &Address,
        adapter: &Address,
        proposed_version: &HandshakeVersion,
    ) -> Result<(), Error> {
        caller.require_auth();

        let supported = Self::get_supported_versions(env);
        if supported.is_empty() {
            panic_with_error!(env, Error::HandshakeSupportedVersionsNotSet);
        }
        let max_minor = supported
            .get(&proposed_version.major)
            .ok_or(Error::HandshakeVersionMismatch)?;

        if proposed_version.minor > max_minor {
            panic_with_error!(env, Error::HandshakeVersionMismatch);
        }

        let mut handshakes = Self::get_handshake_map(env);
        if let Some(existing) = handshakes.get(adapter) {
            match existing.state {
                HandshakeState::Accepted | HandshakeState::Rejected => {
                    panic_with_error!(env, Error::HandshakeAlreadyCompleted);
                }
                _ => {}
            }
        }

        let now = env.ledger().timestamp();
        let record = HandshakeRecord {
            adapter: adapter.clone(),
            proposed_version: proposed_version.clone(),
            negotiated_version: None,
            state: HandshakeState::Pending,
            initiated_at: now,
            updated_at: now,
        };

        handshakes.set(adapter.clone(), &record);
        Self::set_handshake_map(env, &handshakes);

        event_emitter::emit_handshake_initiated(env, adapter, proposed_version, now);

        Ok(())
    }

    /// Accept a pending handshake (called by the adapter contract).
    ///
    /// The adapter must be the one that was proposed to, and the
    /// handshake must be in `Pending` state and not expired.
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment.
    /// * `adapter` - The adapter contract address (must be authorised).
    /// * `negotiated_version` - The version the adapter accepts.
    ///
    /// # Errors
    ///
    /// - [`Error::HandshakeUnauthorized`] — caller is not the adapter.
    /// - [`Error::HandshakeNotFound`] — no pending handshake
    ///   exists for this adapter.
    /// - [`Error::HandshakePending`] — the handshake is not
    ///   `Pending`.
    /// - [`Error::HandshakeExpired`] — the handshake has expired.
    /// - [`Error::HandshakeVersionMismatch`] — the negotiated major version
    ///   does not match the proposed major version.
    pub fn accept_handshake(
        env: &Env,
        adapter: &Address,
        negotiated_version: &HandshakeVersion,
    ) -> Result<(), Error> {
        adapter.require_auth();

        let mut handshakes = Self::get_handshake_map(env);
        let mut record = handshakes
            .get(adapter)
            .ok_or(Error::HandshakeNotFound)?;

        if record.state != HandshakeState::Pending {
            panic_with_error!(env, Error::HandshakePending);
        }

        let now = env.ledger().timestamp();
        if now.saturating_sub(record.initiated_at) > HANDSHAKE_EXPIRY_SECONDS {
            record.state = HandshakeState::Expired;
            record.updated_at = now;
            handshakes.set(adapter.clone(), &record);
            Self::set_handshake_map(env, &handshakes);
            event_emitter::emit_handshake_expired(env, adapter, &record.proposed_version, now);
            panic_with_error!(env, Error::HandshakeExpired);
        }

        if negotiated_version.major != record.proposed_version.major {
            panic_with_error!(env, Error::HandshakeVersionMismatch);
        }

        record.negotiated_version = Some(negotiated_version.clone());
        record.state = HandshakeState::Accepted;
        record.updated_at = now;

        handshakes.set(adapter.clone(), &record);
        Self::set_handshake_map(env, &handshakes);

        event_emitter::emit_handshake_accepted(env, adapter, negotiated_version, now);

        Ok(())
    }

    /// Reject a pending handshake (called by the adapter contract).
    ///
    /// # Parameters
    ///
    /// * `env` - The Soroban environment.
    /// * `adapter` - The adapter contract address (must be authorised).
    ///
    /// # Errors
    ///
    /// - [`Error::HandshakeUnauthorized`] — caller is not the adapter.
    /// - [`Error::HandshakeNotFound`] — no pending handshake
    ///   exists for this adapter.
    /// - [`Error::HandshakePending`] — the handshake is not
    ///   `Pending`.
    /// - [`Error::HandshakeExpired`] — the handshake has expired.
    pub fn reject_handshake(
        env: &Env,
        adapter: &Address,
    ) -> Result<(), Error> {
        adapter.require_auth();

        let mut handshakes = Self::get_handshake_map(env);
        let mut record = handshakes
            .get(adapter)
            .ok_or(Error::HandshakeNotFound)?;

        if record.state != HandshakeState::Pending {
            panic_with_error!(env, Error::HandshakePending);
        }

        let now = env.ledger().timestamp();
        if now.saturating_sub(record.initiated_at) > HANDSHAKE_EXPIRY_SECONDS {
            record.state = HandshakeState::Expired;
            record.updated_at = now;
            handshakes.set(adapter.clone(), &record);
            Self::set_handshake_map(env, &handshakes);
            event_emitter::emit_handshake_expired(env, adapter, &record.proposed_version, now);
            panic_with_error!(env, Error::HandshakeExpired);
        }

        record.state = HandshakeState::Rejected;
        record.updated_at = now;

        handshakes.set(adapter.clone(), &record);
        Self::set_handshake_map(env, &handshakes);

        event_emitter::emit_handshake_rejected(env, adapter, &record.proposed_version, now);

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Queries
    // -------------------------------------------------------------------------

    /// Get the handshake record for an adapter, if one exists.
    pub fn get_handshake(
        env: &Env,
        adapter: &Address,
    ) -> Result<HandshakeRecord, Error> {
        let handshakes = Self::get_handshake_map(env);
        handshakes
            .get(adapter)
            .ok_or(Error::HandshakeNotFound)
    }

    /// Check whether an adapter has a completed (accepted) handshake.
    ///
    /// Returns `true` only when the handshake exists and is in the
    /// `Accepted` state.
    pub fn is_handshake_accepted(env: &Env, adapter: &Address) -> bool {
        match Self::get_handshake(env, adapter) {
            Ok(record) => record.state == HandshakeState::Accepted,
            Err(_) => false,
        }
    }

    /// Check whether an adapter has a pending handshake that has not yet
    /// expired.
    pub fn is_handshake_pending(env: &Env, adapter: &Address) -> bool {
        match Self::get_handshake(env, adapter) {
            Ok(record) => {
                if record.state != HandshakeState::Pending {
                    return false;
                }
                let now = env.ledger().timestamp();
                now.saturating_sub(record.initiated_at) <= HANDSHAKE_EXPIRY_SECONDS
            }
            Err(_) => false,
        }
    }

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn get_handshake_map(env: &Env) -> Map<Address, HandshakeRecord> {
        env.storage()
            .instance()
            .get(&Symbol::new(env, HANDSHAKE_MAP_KEY))
            .unwrap_or_else(|| Map::new(env))
    }

    fn set_handshake_map(env: &Env, map: &Map<Address, HandshakeRecord>) {
        env.storage()
            .instance()
            .set(&Symbol::new(env, HANDSHAKE_MAP_KEY), map);
    }
}

// =============================================================================
// Event emission
// =============================================================================

mod event_emitter {
    use super::*;

    pub fn emit_handshake_initiated(
        env: &Env,
        adapter: &Address,
        proposed_version: &HandshakeVersion,
        timestamp: u64,
    ) {
        let event = HandshakeInitiatedEvent {
            adapter: adapter.clone(),
            proposed_version: proposed_version.clone(),
            timestamp,
        };
        env.events().publish(
            (symbol_short!("hs_init"), adapter.clone()),
            event,
        );
    }

    pub fn emit_handshake_accepted(
        env: &Env,
        adapter: &Address,
        negotiated_version: &HandshakeVersion,
        timestamp: u64,
    ) {
        let event = HandshakeAcceptedEvent {
            adapter: adapter.clone(),
            negotiated_version: negotiated_version.clone(),
            timestamp,
        };
        env.events().publish(
            (symbol_short!("hs_accept"), adapter.clone()),
            event,
        );
    }

    pub fn emit_handshake_rejected(
        env: &Env,
        adapter: &Address,
        proposed_version: &HandshakeVersion,
        timestamp: u64,
    ) {
        let event = HandshakeRejectedEvent {
            adapter: adapter.clone(),
            proposed_version: proposed_version.clone(),
            timestamp,
        };
        env.events().publish(
            (symbol_short!("hs_reject"), adapter.clone()),
            event,
        );
    }

    pub fn emit_handshake_expired(
        env: &Env,
        adapter: &Address,
        proposed_version: &HandshakeVersion,
        timestamp: u64,
    ) {
        let event = HandshakeExpiredEvent {
            adapter: adapter.clone(),
            proposed_version: proposed_version.clone(),
            timestamp,
        };
        env.events().publish(
            (symbol_short!("hs_expire"), adapter.clone()),
            event,
        );
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup(env: &Env) -> Address {
        let admin = Address::generate(env);
        env.mock_all_auths();
        admin
    }

    #[test]
    fn test_set_and_get_supported_versions() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        versions.set(2u32, 3u32);

        HandshakeManager::set_supported_versions(&env, &admin, versions.clone()).unwrap();

        let retrieved = HandshakeManager::get_supported_versions(&env);
        assert_eq!(retrieved.len(), 2);
        assert_eq!(retrieved.get(&1), Some(5));
        assert_eq!(retrieved.get(&2), Some(3));
    }

    #[test]
    fn test_set_supported_versions_rejects_zero_major() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(0u32, 5u32);

        let result = HandshakeManager::set_supported_versions(&env, &admin, versions);
        assert!(result.is_err());
    }

    #[test]
    fn test_initiate_handshake_success() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(1, 3, 0);

        let result = HandshakeManager::initiate_handshake(&env, &admin, &adapter, &proposed);
        assert!(result.is_ok());

        let record = HandshakeManager::get_handshake(&env, &adapter).unwrap();
        assert_eq!(record.state, HandshakeState::Pending);
        assert_eq!(record.proposed_version, proposed);
    }

    #[test]
    fn test_initiate_handshake_version_mismatch() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(3, 0, 0);

        let result = HandshakeManager::initiate_handshake(&env, &admin, &adapter, &proposed);
        assert!(result.is_err());
    }

    #[test]
    fn test_initiate_handshake_version_too_high() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(1, 10, 0);

        let result = HandshakeManager::initiate_handshake(&env, &admin, &adapter, &proposed);
        assert!(result.is_err());
    }

    #[test]
    fn test_accept_handshake_success() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(1, 3, 0);
        HandshakeManager::initiate_handshake(&env, &admin, &adapter, &proposed).unwrap();

        let negotiated = HandshakeVersion::new(1, 3, 0);
        let result = HandshakeManager::accept_handshake(&env, &adapter, &negotiated);
        assert!(result.is_ok());

        let record = HandshakeManager::get_handshake(&env, &adapter).unwrap();
        assert_eq!(record.state, HandshakeState::Accepted);
        assert_eq!(record.negotiated_version, Some(negotiated));
    }

    #[test]
    fn test_accept_handshake_wrong_major_version() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(1, 3, 0);
        HandshakeManager::initiate_handshake(&env, &admin, &adapter, &proposed).unwrap();

        let negotiated = HandshakeVersion::new(2, 3, 0);
        let result = HandshakeManager::accept_handshake(&env, &adapter, &negotiated);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_handshake_success() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(1, 3, 0);
        HandshakeManager::initiate_handshake(&env, &admin, &adapter, &proposed).unwrap();

        let result = HandshakeManager::reject_handshake(&env, &adapter);
        assert!(result.is_ok());

        let record = HandshakeManager::get_handshake(&env, &adapter).unwrap();
        assert_eq!(record.state, HandshakeState::Rejected);
    }

    #[test]
    fn test_handshake_expiry() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(1, 3, 0);
        HandshakeManager::initiate_handshake(&env, &admin, &adapter, &proposed).unwrap();

        // Simulate time passing beyond expiry
        env.ledger().set_timestamp(env.ledger().timestamp() + HANDSHAKE_EXPIRY_SECONDS + 1);

        let result = HandshakeManager::accept_handshake(&env, &adapter, &proposed);
        assert!(result.is_err());

        let record = HandshakeManager::get_handshake(&env, &adapter).unwrap();
        assert_eq!(record.state, HandshakeState::Expired);
    }

    #[test]
    fn test_is_handshake_accepted() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(1, 3, 0);
        HandshakeManager::initiate_handshake(&env, &admin, &adapter, &proposed).unwrap();

        assert!(!HandshakeManager::is_handshake_accepted(&env, &adapter));

        HandshakeManager::accept_handshake(&env, &adapter, &proposed).unwrap();

        assert!(HandshakeManager::is_handshake_accepted(&env, &adapter));
    }

    #[test]
    fn test_is_handshake_pending() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(1, 3, 0);

        assert!(!HandshakeManager::is_handshake_pending(&env, &adapter));

        HandshakeManager::initiate_handshake(&env, &admin, &adapter, &proposed).unwrap();

        assert!(HandshakeManager::is_handshake_pending(&env, &adapter));
    }

    #[test]
    fn test_accept_already_completed_handshake() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(1, 3, 0);
        HandshakeManager::initiate_handshake(&env, &admin, &adapter, &proposed).unwrap();
        HandshakeManager::accept_handshake(&env, &adapter, &proposed).unwrap();

        let result = HandshakeManager::reject_handshake(&env, &adapter);
        assert!(result.is_err());
    }

    #[test]
    fn test_unauthorised_initiate() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(1, 3, 0);
        let unauthorised = Address::generate(&env);

        let result = HandshakeManager::initiate_handshake(&env, &unauthorised, &adapter, &proposed);
        assert!(result.is_err());
    }

    #[test]
    fn test_unauthorised_accept() {
        let env = Env::default();
        let admin = setup(&env);

        let mut versions = Map::new(&env);
        versions.set(1u32, 5u32);
        HandshakeManager::set_supported_versions(&env, &admin, versions).unwrap();

        let adapter = Address::generate(&env);
        let proposed = HandshakeVersion::new(1, 3, 0);
        HandshakeManager::initiate_handshake(&env, &admin, &adapter, &proposed).unwrap();

        let unauthorised = Address::generate(&env);
        let result = HandshakeManager::accept_handshake(&env, &unauthorised, &proposed);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_nonexistent_handshake() {
        let env = Env::default();
        let adapter = Address::generate(&env);

        let result = HandshakeManager::get_handshake(&env, &adapter);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_supported_versions() {
        let env = Env::default();

        let versions = HandshakeManager::get_supported_versions(&env);
        assert_eq!(versions.len(), 0);
    }
}