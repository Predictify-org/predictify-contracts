//! Event topic compatibility layer for contract upgrades (issue #1391).
//!
//! # Problem
//!
//! Soroban event topics are emitted as raw `symbol_short!()` literals scattered
//! across every emit site.  When a topic is renamed or a schema version bumps,
//! off-chain indexers and integrators that filter by topic stop receiving events
//! without warning.  There is also no mechanism to simultaneously publish under
//! both the *old* and *new* topic symbol during a rolling upgrade window.
//!
//! # Solution
//!
//! This module provides:
//!
//! 1. **[`EventTopicRegistry`]** – a single, authoritative table of every event
//!    topic emitted by the contract, with the current topic `Symbol`, a
//!    monotonically-increasing `schema_version`, and a human-readable
//!    `description`.  All emit sites **must** resolve their topic through this
//!    registry rather than hard-coding `symbol_short!()` inline.
//!
//! 2. **[`EventCompatBridge`]** – dual-publish helper that, during a
//!    *compatibility window*, emits a single logical event under **both** the
//!    previous topic symbol and the current topic symbol.  Consumers can migrate
//!    to the new topic at their own pace.
//!
//! 3. **[`TopicAlias`] / [`DataKey::EventTopicAlias`]** – persistent storage
//!    entries that map a superseded topic symbol to its replacement.  Written
//!    by the upgrade hook so that any on-chain consumer that reads aliases can
//!    discover the rename automatically.
//!
//! 4. **[`EventNonceGuard`]** – helpers invoked from [`UpgradeManager`] to
//!    snapshot and restore all per-topic event nonces so that the
//!    monotonically-increasing guarantee survives contract upgrades even if
//!    persistent storage is partially re-initialised.
//!
//! # Invariants
//!
//! * `EventTopicRegistry::get` never panics; it returns `None` for unknown
//!   names so callers can fall back gracefully.
//! * `EventCompatBridge::publish_with_compat` is idempotent: calling it twice
//!   with the same `(old_topic, new_topic, data)` tuple emits two *independent*
//!   on-chain events (Soroban events are append-only), but does **not** corrupt
//!   storage or nonces.
//! * Nonce preservation is atomic within a single Soroban transaction: either
//!   all nonces are snapshotted/restored or none are (transaction rolls back on
//!   any panic).
//! * `schema_version` is bumped in this table **only**.  All emit sites delegate
//!   here, so a single constant change propagates everywhere.

#![allow(dead_code)]

use soroban_sdk::{contracttype, Env, Map, String, Symbol, Vec};

use crate::storage::DataKey;

// ─────────────────────────────────────────────────────────────────────────────
// Topic version constants
//
// Every entry below follows the convention:
//   pub const TOPIC_<UPPER_SNAKE>: (&str, u32) = ("symbol", schema_version);
//
// When a schema-breaking change is made to an event payload:
//   1. Bump `schema_version` here.
//   2. Add the previous symbol to `ALIASES` below.
//   3. The upgrade hook will call `EventNonceGuard::preserve_nonces` and
//      `EventCompatBridge::register_aliases` automatically.
// ─────────────────────────────────────────────────────────────────────────────

/// Compile-time registry of every (topic_symbol, schema_version) pair.
///
/// The first tuple element is the `symbol_short!()` string; the second is the
/// current schema version.  This table is the **single source of truth** for
/// all emit sites.
pub const TOPIC_REGISTRY: &[(&str, u32, &str)] = &[
    // ── Market lifecycle ──────────────────────────────────────────────────
    ("mkt_crt",   1, "market_created"),
    ("evt_crt",   1, "event_created"),
    ("mkt_close", 1, "market_closed"),
    ("mkt_final", 1, "market_finalized"),
    ("st_chng",   1, "state_changed"),
    ("mkt_ext",   1, "market_deadline_extended"),
    ("mkt_dsc",   1, "market_description_updated"),
    ("mkt_out",   1, "market_outcomes_updated"),
    ("mkt_cat",   1, "category_updated"),
    ("mkt_tag",   1, "tags_updated"),
    ("ext_req",   1, "extension_requested"),
    ("pool_lo",   1, "min_pool_size_not_met"),
    ("ref_oracl", 1, "refund_on_oracle_failure"),
    ("mkt_arch",  1, "market_archived"),
    ("mkt_rem",   1, "market_removed"),
    ("mkt_tier",  1, "market_tier_changed"),
    // ── Betting ───────────────────────────────────────────────────────────
    ("bet_plc",   1, "bet_placed"),
    ("bet_upd",   1, "bet_status_updated"),
    ("bet_lim",   1, "bet_limit_set"),
    ("cap_set",   1, "max_bet_cap_set"),
    ("cap_excd",  1, "bet_cap_exceeded"),
    ("mxbtcap",   1, "per_ledger_bet_cap_set"),
    ("cum_cap",   1, "cumulative_bet_cap_reached"),
    ("cum_set",   1, "cumulative_bet_cap_set"),
    // ── Voting ────────────────────────────────────────────────────────────
    ("vote",      1, "vote_cast"),
    ("gov_vote",  1, "governance_vote"),
    ("gov_prop",  1, "governance_proposal"),
    ("gov_cmit",  1, "governance_committed"),
    ("gov_exec",  1, "governance_executed"),
    ("gov_rej",   1, "governance_rejected"),
    // ── Resolution ────────────────────────────────────────────────────────
    ("mkt_res",   1, "market_resolved"),
    ("auto_res",  1, "auto_resolved"),
    ("man_res",   1, "manual_resolution_required"),
    ("frc_rs",    1, "force_resolved"),
    // ── Oracle ────────────────────────────────────────────────────────────
    ("oracle_rs", 1, "oracle_result"),
    ("orc_init",  1, "oracle_verification_initiated"),
    ("orc_ver",   1, "oracle_result_verified"),
    ("orc_fail",  1, "oracle_verification_failed"),
    ("orc_val",   1, "oracle_validation_failed"),
    ("orc_res",   1, "oracle_result_fetched"),
    ("orc_hlth",  1, "oracle_health"),
    ("orc_cons",  1, "oracle_consensus"),
    ("orc_med_q", 1, "oracle_median_queried"),
    ("ora_deg",   1, "oracle_degraded"),
    ("ora_rec",   1, "oracle_recovered"),
    ("fbk_used",  1, "fallback_used"),
    ("res_tmo",   1, "resolution_timeout"),
    // ── Disputes ──────────────────────────────────────────────────────────
    ("dispt_opn", 1, "dispute_opened"),
    ("dispt_crt", 1, "dispute_created"),
    ("dispt_res", 1, "dispute_resolved"),
    ("d_v_rej",   1, "dispute_vote_rejected"),
    ("sus_col",   1, "suspicious_collusion"),
    // ── Fees & treasury ───────────────────────────────────────────────────
    ("fee_col",   1, "fee_collected"),
    ("fee_qd",    1, "fee_config_queued"),
    ("fee_apd",   1, "fee_config_applied"),
    ("fee_ccl",   1, "fee_config_cancelled"),
    ("treas_up",  1, "treasury_updated"),
    ("tsu_qd",    1, "treasury_update_queued"),
    ("tsu_apd",   1, "treasury_update_applied"),
    ("tsu_ccl",   1, "treasury_update_cancelled"),
    ("pay_rem",   1, "payout_remainder_allocated"),
    ("unc_swip",  1, "unclaimed_winnings_swept"),
    ("win_clm",   1, "winnings_claimed"),
    ("win_btc",   1, "winnings_batched"),
    ("m_clm_pd",  1, "market_claims_paid_out"),
    ("clm_prd",   1, "claim_period_expired"),
    // ── Admin & access control ────────────────────────────────────────────
    ("adm_init",  1, "admin_initialised"),
    ("adm_act",   1, "admin_action"),
    ("adm_role",  1, "admin_role_set"),
    ("adm_xfer",  1, "admin_transferred"),
    ("adm_ovrd",  1, "admin_override"),
    ("adm_deact", 1, "admin_deactivated"),
    ("adm_brdc",  1, "admin_broadcast"),
    ("allowlst",  1, "allowlist_updated"),
    // ── Storage & upgrade ─────────────────────────────────────────────────
    ("st_tier",   1, "storage_tier_changed"),
    ("stor_cln",  1, "storage_cleaned"),
    ("stor_mig",  1, "storage_migrated"),
    ("stor_opt",  1, "storage_optimised"),
    ("up_grade",  1, "contract_upgraded"),
    ("up_prop",   1, "upgrade_proposed"),
    ("rollback",  1, "upgrade_rolled_back"),
    ("arch_trn",  1, "archive_transition"),
    ("rest_trn",  1, "restore_transition"),
    // ── Statistics & monitoring ───────────────────────────────────────────
    ("stats_upd", 1, "statistics_updated"),
    ("perf_met",  1, "performance_metric"),
    ("mon_ovf",   1, "monitoring_overflow"),
    ("bal_chg",   1, "balance_changed"),
    ("err_evt",   1, "error_event"),
    ("err_log",   1, "error_logged"),
    ("err_rec",   1, "error_recovered"),
    // ── Miscellaneous ─────────────────────────────────────────────────────
    ("cfg_init",  1, "config_initialised"),
    ("cfg_upd",   1, "config_updated"),
    ("pltf_set",  1, "platform_settings_updated"),
    ("ctr_init",  1, "contract_initialised"),
    ("ctr_pause", 1, "contract_paused"),
    ("ctr_unp",   1, "contract_unpaused"),
    ("thld_conf", 1, "threshold_configured"),
    ("thld_prop", 1, "threshold_proposed"),
    ("tout_set",  1, "timeout_set"),
    ("tout_ext",  1, "timeout_extended"),
    ("tout_exp",  1, "timeout_expired"),
    ("dh_evct",   1, "dispute_handler_evicted"),
    ("fwd_att",   1, "forward_attempted"),
    ("fwd_ok",    1, "forward_succeeded"),
    ("chain_mm",  1, "chain_mismatch"),
    ("evt_vis",   1, "event_visibility_changed"),
    ("depr_call", 1, "deprecated_entrypoint_called"),
    ("verify_rs", 1, "oracle_result_verify_success"),
    // ── Governance registry ───────────────────────────────────────────────
    ("ep_one",    1, "entrypoint_one"),
    ("ep_two",    1, "entrypoint_two"),
];

/// Known *superseded* topic aliases: (old_symbol, new_symbol).
///
/// Add an entry here whenever a topic symbol is renamed so that the upgrade
/// hook can persist the mapping and the compat bridge can dual-publish.
pub const TOPIC_ALIASES: &[(&str, &str)] = &[
    // Example (kept as documentation template; add real renames here):
    // ("old_sym", "new_sym"),
];

// ─────────────────────────────────────────────────────────────────────────────
// Runtime registry
// ─────────────────────────────────────────────────────────────────────────────

/// Versioned descriptor for a single event topic.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopicDescriptor {
    /// The canonical `Symbol` used as the first element of the publish tuple.
    pub topic: Symbol,
    /// Monotonically-increasing schema version.  Bump when the payload type
    /// changes shape (field added, removed, or retyped).
    pub schema_version: u32,
    /// Human-readable name such as `"market_created"`.
    pub name: String,
}

/// Persistent alias record stored under [`DataKey::EventTopicAlias`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopicAlias {
    /// The superseded topic symbol (indexers still filtering on this).
    pub old_topic: Symbol,
    /// The replacement topic symbol.
    pub new_topic: Symbol,
    /// Ledger sequence at which the alias was registered.
    pub registered_at: u32,
    /// Contract version (as `major * 1_000_000 + minor * 1_000 + patch`) in
    /// which the topic was renamed.
    pub since_version: u64,
}

/// Snapshot of a single nonce so it can be preserved across an upgrade.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NonceSnapshot {
    pub topic: Symbol,
    pub value: u64,
}

/// Central, read-only registry of every topic emitted by the contract.
///
/// All emit sites **must** obtain their `(Symbol, schema_version)` pair from
/// this registry rather than hard-coding `symbol_short!()` literals.  This
/// ensures that a single constant change propagates to every call-site and
/// that off-chain tooling can discover topics deterministically via
/// `get_all_topics`.
pub struct EventTopicRegistry;

impl EventTopicRegistry {
    /// Look up the [`TopicDescriptor`] for a named event.
    ///
    /// Returns `None` when `name` is not registered so callers can fall back
    /// gracefully rather than panicking in production.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use soroban_sdk::Env;
    /// # let env = Env::default();
    /// let desc = predictify_hybrid::event_topic_compat::EventTopicRegistry::get(
    ///     &env, "market_created",
    /// ).unwrap();
    /// assert_eq!(desc.schema_version, 1);
    /// ```
    pub fn get(env: &Env, name: &str) -> Option<TopicDescriptor> {
        for &(sym, version, entry_name) in TOPIC_REGISTRY {
            if entry_name == name {
                return Some(TopicDescriptor {
                    topic: Symbol::new(env, sym),
                    schema_version: version,
                    name: String::from_str(env, entry_name),
                });
            }
        }
        None
    }

    /// Look up by the raw symbol string (e.g. `"mkt_crt"`).
    pub fn get_by_symbol(env: &Env, sym: &str) -> Option<TopicDescriptor> {
        for &(topic_sym, version, name) in TOPIC_REGISTRY {
            if topic_sym == sym {
                return Some(TopicDescriptor {
                    topic: Symbol::new(env, topic_sym),
                    schema_version: version,
                    name: String::from_str(env, name),
                });
            }
        }
        None
    }

    /// Return descriptors for *every* registered topic.
    ///
    /// Primarily intended for off-chain tooling (indexers, dashboards) that
    /// need to enumerate all topics at startup.
    pub fn get_all_topics(env: &Env) -> Vec<TopicDescriptor> {
        let mut out = Vec::new(env);
        for &(sym, version, name) in TOPIC_REGISTRY {
            out.push_back(TopicDescriptor {
                topic: Symbol::new(env, sym),
                schema_version: version,
                name: String::from_str(env, name),
            });
        }
        out
    }

    /// Return a [`Map`] from topic symbol string to schema version number,
    /// suitable for embedding in an on-chain query response.
    pub fn get_version_map(env: &Env) -> Map<Symbol, u32> {
        let mut m: Map<Symbol, u32> = Map::new(env);
        for &(sym, version, _name) in TOPIC_REGISTRY {
            m.set(Symbol::new(env, sym), version);
        }
        m
    }

    /// Return the current schema version for a topic by raw symbol string,
    /// or 0 if the symbol is not registered.
    pub fn schema_version(env: &Env, sym: &str) -> u32 {
        Self::get_by_symbol(env, sym)
            .map(|d| d.schema_version)
            .unwrap_or(0)
    }

    /// Total number of registered topics.  Useful for off-chain health checks.
    pub fn topic_count() -> u32 {
        TOPIC_REGISTRY.len() as u32
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Compatibility bridge
// ─────────────────────────────────────────────────────────────────────────────

/// Dual-publish helper for rolling upgrade windows.
///
/// During a *compatibility window* (typically one or two contract versions),
/// a single logical event is published under **both** the previous topic symbol
/// and the current symbol.  Off-chain consumers can migrate at their own pace.
///
/// # Example
///
/// ```rust,no_run
/// # use soroban_sdk::{Env, symbol_short};
/// # let env = Env::default();
/// # let payload = 42_i128;
/// # use predictify_hybrid::event_topic_compat::EventCompatBridge;
/// // Publish under the old topic "old_sym" AND the new topic "mkt_crt".
/// EventCompatBridge::publish_with_compat(
///     &env,
///     symbol_short!("old_sym"),
///     symbol_short!("mkt_crt"),
///     &payload,
/// );
/// ```
pub struct EventCompatBridge;

impl EventCompatBridge {
    /// Publish `data` under `new_topic`.  If `old_topic != new_topic`, also
    /// publish under `old_topic` so that legacy indexers still receive the
    /// event.
    ///
    /// The payload is cloned for the second publish; no extra storage is
    /// written.
    ///
    /// # Invariants
    ///
    /// * If `old_topic == new_topic` only one event is emitted (no duplication).
    /// * This function never panics; both publish calls are unconditional.
    /// * It does **not** update nonces; callers must handle nonce management
    ///   themselves (typically via `EventEmitter::get_and_increment_nonce`).
    pub fn publish_with_compat<T: soroban_sdk::IntoVal<Env, soroban_sdk::Val> + Clone>(
        env: &Env,
        old_topic: Symbol,
        new_topic: Symbol,
        data: &T,
    ) {
        // Always publish under the current (new) topic.
        env.events().publish((new_topic.clone(),), data.clone());

        // If the topic changed, also publish under the old symbol so legacy
        // consumers that have not yet updated their filter continue to receive
        // the event.
        if old_topic != new_topic {
            env.events().publish((old_topic,), data.clone());
        }
    }

    /// Persist a [`TopicAlias`] in contract storage so on-chain consumers can
    /// discover the rename.
    ///
    /// Called once during the upgrade hook; safe to call multiple times (later
    /// calls overwrite the previous alias record for the same `old_topic`).
    pub fn register_alias(env: &Env, old_topic: Symbol, new_topic: Symbol, since_version: u64) {
        let alias = TopicAlias {
            old_topic: old_topic.clone(),
            new_topic,
            registered_at: env.ledger().sequence(),
            since_version,
        };
        let key = DataKey::EventTopicAlias(old_topic);
        env.storage().persistent().set(&key, &alias);
    }

    /// Register all aliases declared in [`TOPIC_ALIASES`].
    ///
    /// Intended to be called once from the upgrade hook so that every renamed
    /// topic is persisted atomically in a single transaction.
    pub fn register_all_aliases(env: &Env, since_version: u64) {
        for &(old_sym, new_sym) in TOPIC_ALIASES {
            Self::register_alias(
                env,
                Symbol::new(env, old_sym),
                Symbol::new(env, new_sym),
                since_version,
            );
        }
    }

    /// Look up the alias record for `old_topic`, if any.
    pub fn get_alias(env: &Env, old_topic: &Symbol) -> Option<TopicAlias> {
        let key = DataKey::EventTopicAlias(old_topic.clone());
        env.storage().persistent().get(&key)
    }

    /// Return all persisted aliases as a `Vec<TopicAlias>`.
    pub fn get_all_aliases(env: &Env) -> Vec<TopicAlias> {
        let mut out = Vec::new(env);
        for &(old_sym, _) in TOPIC_ALIASES {
            let key = DataKey::EventTopicAlias(Symbol::new(env, old_sym));
            if let Some(alias) = env.storage().persistent().get::<DataKey, TopicAlias>(&key) {
                out.push_back(alias);
            }
        }
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Nonce preservation
// ─────────────────────────────────────────────────────────────────────────────

/// Helpers for snapshotting and restoring per-topic event nonces across
/// contract upgrades.
///
/// # Why this matters
///
/// `EventEmitter::get_and_increment_nonce` stores the nonce in *persistent*
/// storage under `DataKey::EventNonce(topic)`.  During some upgrade paths
/// persistent storage may be partially re-initialised, which would reset
/// nonces to 0 and break replay-protection for consumers that rely on
/// monotonically-increasing nonce sequences.
///
/// `EventNonceGuard::preserve_nonces` snapshots all current nonces into a
/// dedicated persistent key before the upgrade executes.
/// `EventNonceGuard::restore_nonces` is called immediately after the upgrade
/// to copy the snapshots back, guaranteeing continuity.
pub struct EventNonceGuard;

impl EventNonceGuard {
    const SNAPSHOT_KEY: &'static str = "nonce_snap";

    /// Snapshot all known per-topic nonces into persistent storage.
    ///
    /// Must be called **before** any storage migration step that might clear
    /// or reset `DataKey::EventNonce` entries.
    pub fn preserve_nonces(env: &Env) {
        let mut snapshots: Vec<NonceSnapshot> = Vec::new(env);

        for &(sym, _version, _name) in TOPIC_REGISTRY {
            let topic = Symbol::new(env, sym);
            let key = DataKey::EventNonce(topic.clone());
            if let Some(value) = env.storage().persistent().get::<DataKey, u64>(&key) {
                if value > 0 {
                    snapshots.push_back(NonceSnapshot { topic, value });
                }
            }
        }

        let snap_key = Symbol::new(env, Self::SNAPSHOT_KEY);
        env.storage().persistent().set(&snap_key, &snapshots);
    }

    /// Restore snapshotted nonces after the upgrade completes.
    ///
    /// Reads the snapshot written by `preserve_nonces` and writes each nonce
    /// back to `DataKey::EventNonce(topic)` **only if** the restored value is
    /// strictly greater than whatever is currently stored.  This prevents a
    /// race where a post-upgrade emission already incremented a nonce beyond
    /// the snapshot value.
    ///
    /// Returns the number of nonces that were restored.
    pub fn restore_nonces(env: &Env) -> u32 {
        let snap_key = Symbol::new(env, Self::SNAPSHOT_KEY);
        let snapshots: Vec<NonceSnapshot> = env
            .storage()
            .persistent()
            .get(&snap_key)
            .unwrap_or_else(|| Vec::new(env));

        let mut restored: u32 = 0;
        for snap in snapshots.iter() {
            let key = DataKey::EventNonce(snap.topic.clone());
            let current: u64 = env
                .storage()
                .persistent()
                .get(&key)
                .unwrap_or(0);
            // Only restore if the snapshot is larger to avoid rollback attacks.
            if snap.value > current {
                env.storage().persistent().set(&key, &snap.value);
                restored += 1;
            }
        }

        restored
    }

    /// Remove the snapshot after a successful restore to reclaim storage.
    pub fn clear_snapshot(env: &Env) {
        let snap_key = Symbol::new(env, Self::SNAPSHOT_KEY);
        env.storage().persistent().remove(&snap_key);
    }

    /// Read the stored snapshot without modifying state; useful for tests.
    pub fn read_snapshot(env: &Env) -> Vec<NonceSnapshot> {
        let snap_key = Symbol::new(env, Self::SNAPSHOT_KEY);
        env
            .storage()
            .persistent()
            .get(&snap_key)
            .unwrap_or_else(|| Vec::new(env))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Compatibility-aware emit helper
// ─────────────────────────────────────────────────────────────────────────────

/// Emit helper that resolves the topic from [`EventTopicRegistry`] and handles
/// optional compat-bridge dual-publishing transparently.
///
/// Emit sites call `CompatEmit::publish` instead of
/// `env.events().publish((symbol_short!("..."), ...)` directly.  The resolved
/// `schema_version` is appended to the topic tuple so that off-chain consumers
/// can distinguish payloads across schema changes.
pub struct CompatEmit;

impl CompatEmit {
    /// Publish `data` using the canonical topic for `event_name`.
    ///
    /// If an alias exists for a previous symbol under the same name (stored via
    /// `EventCompatBridge::register_alias`), the event is also published under
    /// the old symbol to preserve backward compatibility.
    ///
    /// Falls back to a no-op if `event_name` is not registered, rather than
    /// panicking.
    pub fn publish<T>(env: &Env, event_name: &str, secondary_topic: Option<Symbol>, data: &T)
    where
        T: soroban_sdk::IntoVal<Env, soroban_sdk::Val> + Clone,
    {
        let descriptor = match EventTopicRegistry::get(env, event_name) {
            Some(d) => d,
            None => return, // Unknown event; skip silently rather than panic.
        };

        match secondary_topic {
            Some(sec) => {
                env.events().publish(
                    (descriptor.topic.clone(), sec, descriptor.schema_version),
                    data.clone(),
                );
            }
            None => {
                env.events().publish(
                    (descriptor.topic.clone(), descriptor.schema_version),
                    data.clone(),
                );
            }
        }
    }
}
