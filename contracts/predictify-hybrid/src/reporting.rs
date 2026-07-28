use soroban_sdk::{contracttype, xdr::ToXdr, Address, Bytes, Env, String, Symbol};

use crate::err::Error;
use crate::types::PlatformStatistics;

/// Current schema version for [`SnapshotEnvelope`].
///
/// Bump this whenever the `PlatformStats` or `SnapshotEnvelope` wire format
/// changes in a backward-incompatible way. Off-chain indexers MUST check this
/// field before deserialising `stats_xdr`.
pub const SNAPSHOT_SCHEMA_VERSION: u32 = 1;

// ===== STORAGE KEYS =====

/// Persistent key for the live `PlatformStats`.
const PLATFORM_STATS_KEY: &str = "PlatformStats";
/// Persistent key for the XDR-serialised snapshot bytes.
const SNAPSHOT_XDR_KEY: &str = "SnapshotXdr";
/// Persistent key for the schema version stored alongside the XDR.
const SNAPSHOT_VERSION_KEY: &str = "SnapshotVersion";
/// Persistent key for the ledger timestamp of the last snapshot.
const SNAPSHOT_TIMESTAMP_KEY: &str = "SnapshotTimestamp";

// ===== TYPES =====

/// Platform-wide statistical snapshot.
///
/// Captures aggregate activity across all markets at a single point in time.
/// Stored in contract storage and included in [`SnapshotEnvelope`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformStats {
    /// Total number of events / markets created.
    pub total_events_created: u64,
    /// Total bets placed across all markets.
    pub total_bets_placed: u64,
    /// Total volume wagered (in smallest token unit).
    pub total_volume: i128,
    /// Total platform fees collected.
    pub total_fees_collected: i128,
    /// Number of currently active markets.
    pub active_events_count: u32,
    /// Unique users who have placed at least one bet.
    pub total_unique_users: u32,
    /// Ledger timestamp when this snapshot was taken.
    pub timestamp: u64,
}

/// Versioned, XDR-stable snapshot envelope for off-chain archiving.
///
/// Clients should check `schema_version` before attempting to decode
/// `stats_xdr` — a version mismatch means the wire format has changed and
/// older decoding logic may produce garbage.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotEnvelope {
    /// Schema version of the payload (`SNAPSHOT_SCHEMA_VERSION`).
    pub schema_version: u32,
    /// XDR-serialised [`PlatformStats`] bytes.
    pub stats_xdr: Bytes,
    /// Ledger timestamp when the envelope was created.
    pub ledger_timestamp: u64,
}

/// Manages the platform reporting lifecycle.
pub struct ReportingManager;

impl ReportingManager {
    /// Return the current [`SnapshotEnvelope`] from storage.
    ///
    /// If no snapshot has been taken yet the envelope is built on the fly from
    /// live [`PlatformStatistics`] so callers always get a meaningful result.
    ///
    /// # Errors
    ///
    /// - `Error::InvalidState` — stored XDR payload cannot be decoded.
    pub fn get_snapshot_envelope(env: &Env) -> Result<SnapshotEnvelope, Error> {
        if let Some(xdr) = env.storage().persistent().get::<_, Bytes>(
            &Symbol::new(env, SNAPSHOT_XDR_KEY),
        ) {
            let version: u32 = env
                .storage()
                .persistent()
                .get(&Symbol::new(env, SNAPSHOT_VERSION_KEY))
                .unwrap_or(SNAPSHOT_SCHEMA_VERSION);
            let ts: u64 = env
                .storage()
                .persistent()
                .get(&Symbol::new(env, SNAPSHOT_TIMESTAMP_KEY))
                .unwrap_or(0);
            Ok(SnapshotEnvelope {
                schema_version: version,
                stats_xdr: xdr,
                ledger_timestamp: ts,
            })
        } else {
            Self::build_live_envelope(env)
        }
    }

    /// Overwrite the stored platform snapshot.
    ///
    /// Only the admin address may call this. The caller is authenticated via
    /// `require_auth` before any storage is touched.
    ///
    /// # Parameters
    ///
    /// * `admin` — must match the stored contract admin (authorised via
    ///   `admin.require_auth()`).
    /// * `stats` — the new [`PlatformStats`] to persist.
    ///
    /// # Errors
    ///
    /// - `Error::Unauthorized` — caller does not match the stored admin.
    /// - `Error::AdminNotSet` — no admin has been initialised yet.
    pub fn update_platform_stats(
        env: &Env,
        admin: &Address,
        stats: &PlatformStats,
    ) -> Result<(), Error> {
        admin.require_auth();

        let stored: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "Admin"))
            .ok_or(Error::AdminNotSet)?;
        if admin != &stored {
            return Err(Error::Unauthorized);
        }

        let xdr: Bytes = stats.to_xdr(env);
        env.storage()
            .persistent()
            .set(&Symbol::new(env, SNAPSHOT_XDR_KEY), &xdr);
        env.storage()
            .persistent()
            .set(&Symbol::new(env, SNAPSHOT_VERSION_KEY), &SNAPSHOT_SCHEMA_VERSION);
        env.storage()
            .persistent()
            .set(
                &Symbol::new(env, SNAPSHOT_TIMESTAMP_KEY),
                &env.ledger().timestamp(),
            );
        env.storage()
            .persistent()
            .set(&Symbol::new(env, PLATFORM_STATS_KEY), stats);

        env.events().publish(
            (Symbol::new(env, "snapshot_updated"),),
            (stats.total_events_created, stats.total_bets_placed, stats.total_volume),
        );
        Ok(())
    }

    /// Record a new daily/historical snapshot.
    ///
    /// This is a state-changing entrypoint that persists the given `stats` in
    /// the snapshot envelope **and** emits a `snapshot_recorded` event.  The
    /// caller must authenticate as the contract admin.
    ///
    /// # Parameters
    ///
    /// * `admin` — contract admin (authenticated via `require_auth`).
    /// * `stats` — [`PlatformStats`] to record.
    ///
    /// # Errors
    ///
    /// - `Error::Unauthorized` — caller does not match the stored admin.
    /// - `Error::AdminNotSet` — no admin has been initialised.
    pub fn record_snapshot(
        env: &Env,
        admin: &Address,
        stats: &PlatformStats,
    ) -> Result<(), Error> {
        admin.require_auth();

        let stored: Address = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "Admin"))
            .ok_or(Error::AdminNotSet)?;
        if admin != &stored {
            return Err(Error::Unauthorized);
        }

        let xdr: Bytes = stats.to_xdr(env);
        let ts = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&Symbol::new(env, SNAPSHOT_XDR_KEY), &xdr);
        env.storage()
            .persistent()
            .set(&Symbol::new(env, SNAPSHOT_VERSION_KEY), &SNAPSHOT_SCHEMA_VERSION);
        env.storage()
            .persistent()
            .set(&Symbol::new(env, SNAPSHOT_TIMESTAMP_KEY), &ts);
        env.storage()
            .persistent()
            .set(&Symbol::new(env, PLATFORM_STATS_KEY), stats);

        env.events().publish(
            (Symbol::new(env, "snapshot_recorded"), admin.clone()),
            (stats.total_events_created, ts),
        );
        Ok(())
    }

    // ===== INTERNAL HELPERS =====

    /// Build a `SnapshotEnvelope` from live `PlatformStatistics`.
    fn build_live_envelope(env: &Env) -> Result<SnapshotEnvelope, Error> {
        let stats = Self::collect_live_platform_stats(env)?;
        let xdr: Bytes = stats.to_xdr(env);
        Ok(SnapshotEnvelope {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            stats_xdr: xdr,
            ledger_timestamp: env.ledger().timestamp(),
        })
    }

    /// Read the live `PlatformStatistics` from storage and convert to
    /// `PlatformStats`.
    fn collect_live_platform_stats(env: &Env) -> Result<PlatformStats, Error> {
        let live: PlatformStatistics = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "platform_stats"))
            .unwrap_or(PlatformStatistics {
                total_events_created: 0,
                total_bets_placed: 0,
                total_volume: 0,
                total_fees_collected: 0,
                active_events_count: 0,
            });

        let user_count: u32 = env
            .storage()
            .persistent()
            .get(&Symbol::new(env, "TotalUniqueUsers"))
            .unwrap_or(0);

        Ok(PlatformStats {
            total_events_created: live.total_events_created,
            total_bets_placed: live.total_bets_placed,
            total_volume: live.total_volume,
            total_fees_collected: live.total_fees_collected,
            active_events_count: live.active_events_count,
            total_unique_users: user_count,
            timestamp: env.ledger().timestamp(),
        })
    }
}

// ===== TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{testutils::Events, Address, Env};

    fn setup_env() -> (Env, Address) {
        let env = Env::default();
        let admin = Address::generate(&env);
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "Admin"), &admin);
        (env, admin)
    }

    fn sample_stats(env: &Env) -> PlatformStats {
        PlatformStats {
            total_events_created: 42,
            total_bets_placed: 10_000,
            total_volume: 1_000_000_000,
            total_fees_collected: 50_000_000,
            active_events_count: 12,
            total_unique_users: 500,
            timestamp: env.ledger().timestamp(),
        }
    }

    // ===== get_snapshot_envelope (read-only, no auth) =====

    #[test]
    fn test_get_snapshot_envelope_returns_default_when_empty() {
        let env = Env::default();
        let envelope = ReportingManager::get_snapshot_envelope(&env).unwrap();
        assert_eq!(envelope.schema_version, SNAPSHOT_SCHEMA_VERSION);
        // stats_xdr should decode to a zero-value PlatformStats
        let decoded: PlatformStats =
            soroban_sdk::xdr::FromXdr::from_xdr(&envelope.stats_xdr).unwrap();
        assert_eq!(decoded.total_events_created, 0);
        assert_eq!(decoded.total_bets_placed, 0);
        assert_eq!(decoded.total_volume, 0);
    }

    #[test]
    fn test_get_snapshot_envelope_returns_stored_when_present() {
        let (env, admin) = setup_env();
        let stats = sample_stats(&env);
        ReportingManager::update_platform_stats(&env, &admin, &stats).unwrap();

        let envelope = ReportingManager::get_snapshot_envelope(&env).unwrap();
        assert_eq!(envelope.schema_version, SNAPSHOT_SCHEMA_VERSION);
        let decoded: PlatformStats =
            soroban_sdk::xdr::FromXdr::from_xdr(&envelope.stats_xdr).unwrap();
        assert_eq!(decoded.total_events_created, 42);
        assert_eq!(decoded.total_bets_placed, 10_000);
        assert_eq!(decoded.total_volume, 1_000_000_000);
    }

    #[test]
    fn test_get_snapshot_envelope_no_auth_required() {
        let env = Env::default();
        let result = ReportingManager::get_snapshot_envelope(&env);
        assert!(result.is_ok());
    }

    // ===== update_platform_stats (state-changing, requires auth) =====

    #[test]
    fn test_update_platform_stats_succeeds_for_admin() {
        let (env, admin) = setup_env();
        let stats = sample_stats(&env);

        let result = ReportingManager::update_platform_stats(&env, &admin, &stats);
        assert!(result.is_ok());

        // Verify stored
        let stored: PlatformStats = env
            .storage()
            .persistent()
            .get(&Symbol::new(&env, PLATFORM_STATS_KEY))
            .unwrap();
        assert_eq!(stored.total_events_created, 42);
    }

    #[test]
    fn test_update_platform_stats_requires_auth() {
        let env = Env::default();
        let random = Address::generate(&env);
        let stats = sample_stats(&env);

        let result = ReportingManager::update_platform_stats(&env, &random, &stats);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::AdminNotSet);
    }

    #[test]
    fn test_update_platform_stats_wrong_admin_fails() {
        let (env, admin) = setup_env();
        let imposter = Address::generate(&env);
        let stats = sample_stats(&env);

        let result = ReportingManager::update_platform_stats(&env, &imposter, &stats);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::Unauthorized);
    }

    #[test]
    fn test_update_platform_stats_definitely_requires_auth() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let stats = sample_stats(&env);
        let result = ReportingManager::update_platform_stats(&env, &admin, &stats);
        assert_eq!(result.unwrap_err(), Error::AdminNotSet);
    }

    #[test]
    fn test_update_platform_stats_emits_event() {
        let (env, admin) = setup_env();
        let stats = sample_stats(&env);

        ReportingManager::update_platform_stats(&env, &admin, &stats).unwrap();

        let events = env.events().all();
        let found = events.iter().any(|e| {
            e.0.len() >= 1
                && e.0
                    .get(0)
                    .map(|s| s.to_string() == "snapshot_updated")
                    .unwrap_or(false)
        });
        assert!(found, "expected snapshot_updated event");
    }

    // ===== record_snapshot (state-changing, requires auth) =====

    #[test]
    fn test_record_snapshot_succeeds_for_admin() {
        let (env, admin) = setup_env();
        let stats = sample_stats(&env);

        let result = ReportingManager::record_snapshot(&env, &admin, &stats);
        assert!(result.is_ok());

        // Verify envelope was stored
        let envelope = ReportingManager::get_snapshot_envelope(&env).unwrap();
        let decoded: PlatformStats =
            soroban_sdk::xdr::FromXdr::from_xdr(&envelope.stats_xdr).unwrap();
        assert_eq!(decoded.total_events_created, 42);
    }

    #[test]
    fn test_record_snapshot_requires_auth() {
        let env = Env::default();
        let random = Address::generate(&env);
        let stats = sample_stats(&env);

        let result = ReportingManager::record_snapshot(&env, &random, &stats);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::AdminNotSet);
    }

    #[test]
    fn test_record_snapshot_wrong_admin_fails() {
        let (env, admin) = setup_env();
        let imposter = Address::generate(&env);
        let stats = sample_stats(&env);

        let result = ReportingManager::record_snapshot(&env, &imposter, &stats);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::Unauthorized);
    }

    #[test]
    fn test_record_snapshot_emits_event() {
        let (env, admin) = setup_env();
        let stats = sample_stats(&env);

        ReportingManager::record_snapshot(&env, &admin, &stats).unwrap();

        let events = env.events().all();
        let found = events.iter().any(|e| {
            e.0.len() >= 1
                && e.0
                    .get(0)
                    .map(|s| s.to_string() == "snapshot_recorded")
                    .unwrap_or(false)
        });
        assert!(found, "expected snapshot_recorded event");
    }

    // ===== PlatformStats XDR round-trip =====

    #[test]
    fn test_platform_stats_xdr_round_trip() {
        let env = Env::default();
        let original = sample_stats(&env);
        let xdr = original.to_xdr(&env);
        let decoded: PlatformStats = soroban_sdk::xdr::FromXdr::from_xdr(&xdr).unwrap();
        assert_eq!(original, decoded);
    }

    // ===== SnapshotEnvelope XDR round-trip =====

    #[test]
    fn test_snapshot_envelope_xdr_round_trip() {
        let env = Env::default();
        let stats = sample_stats(&env);
        let xdr: Bytes = stats.to_xdr(&env);
        let envelope = SnapshotEnvelope {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            stats_xdr: xdr,
            ledger_timestamp: env.ledger().timestamp(),
        };
        let encoded = envelope.to_xdr(&env);
        let decoded: SnapshotEnvelope =
            soroban_sdk::xdr::FromXdr::from_xdr(&encoded).unwrap();
        assert_eq!(envelope, decoded);
    }
}
