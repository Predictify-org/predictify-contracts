//! # Structured lifecycle events for betting
//!
//! This module defines the **public, off-chain-indexer-facing event
//! surface** for the betting subsystem.  All events published here share a
//! common contract:
//!
//! 1. They use a frozen, ≤9-character [`Symbol`] **topic** suitable for the
//!    `symbol_short!` macro.
//! 2. They carry a `schema_version: u32` field (frozen at `1` for v1) so
//!    indexers can route on `(topic, schema_version)`.
//! 3. They carry a monotonically-incrementing `nonce: u64` per topic,
//!    persisted in instance storage and returned in each event payload.
//! 4. They carry the Soroban ledger `timestamp: u64` at emission.
//! 5. The persisted per-topic nonce counter lets an indexer detect
//!    out-of-order or replayed events without scanning the entire log.
//!
//! # Lifecycle coverage
//!
//! The five events cover the full bet lifecycle:
//!
//! | name                    | topic       | when emitted                              |
//! |-------------------------|-------------|-------------------------------------------|
//! | `bet_created`           | `lifebtcr`  | single bet successfully placed            |
//! | `bet_batch_created`     | `lifebtch`  | batch of bets successfully placed         |
//! | `bet_status_changed`    | `lifebtsc`  | Won / Lost / Refunded / Cancelled         |
//! | `bet_claimed`           | `lifebcml`  | winnings successfully claimed             |
//! | `bet_stats_updated`     | `lifebtst`  | aggregate stats snapshot updated          |
//!
//! Every entrypoint is panic-free for valid input and `unwrap()`-free in
//! production paths; arithmetic that could overflow is handled with
//! `checked_add` / saturating semantics and explicit `Result` returns.
//!
//! # Topic + schema registry
//!
//! [`BettingEventSchema::get_schema`] maps the public logical event
//! name -> `(topic, schema_version)`.  This indirection means a future
//! schema bump can change both the topic symbol and the version number
//! atomically — indexers reading the registry can depalettise entries
//! without code changes.
//!

extern crate alloc;

use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Symbol, Vec};

/// Reserved namespace prefix used inside tuple storage keys for the
/// nonce counter.  Frozen: never change without a
/// `BettingEventSchema` version bump.
pub const NS_NONCE: Symbol = symbol_short!("BtNgNs");

/// Default schema version baked into v1 of this module.  Frozen:
/// bumping `1 -> 2` is a deliberate ABI break and must be advertised
/// to indexer operators.
pub const BETTING_EVENT_SCHEMA_VERSION: u32 = 1;

// ===================================================================
// §1  Public logical event-name constants
// ===================================================================

/// Frozen logical name of the [`BetCreatedEvent`].
pub const EVENT_NAME_BET_CREATED: &str = "bet_created";
/// Frozen logical name of the [`BetBatchCreatedEvent`].
pub const EVENT_NAME_BET_BATCH_CREATED: &str = "bet_batch_created";
/// Frozen logical name of the [`BetStatusChangedEvent`].
pub const EVENT_NAME_BET_STATUS_CHANGED: &str = "bet_status_changed";
/// Frozen logical name of the [`BetClaimedEvent`].
pub const EVENT_NAME_BET_CLAIMED: &str = "bet_claimed";
/// Frozen logical name of the [`BetStatsUpdatedEvent`].
pub const EVENT_NAME_BET_STATS_UPDATED: &str = "bet_stats_updated";

/// Frozen topic symbols used at v1.  These are emitted as the first
/// element of the publish-topic tuple.  Indexers may pin against them.
pub const TOPIC_BET_CREATED: Symbol = symbol_short!("lifebtcr");
pub const TOPIC_BET_BATCH_CREATED: Symbol = symbol_short!("lifebtch");
pub const TOPIC_BET_STATUS_CHANGED: Symbol = symbol_short!("lifebtsc");
pub const TOPIC_BET_CLAIMED: Symbol = symbol_short!("lifebcml");
pub const TOPIC_BET_STATS_UPDATED: Symbol = symbol_short!("lifebtst");

// ===================================================================
// §2  Status string constants
// ===================================================================

/// Bet lifecycle stage strings.  These are the only legal values
/// accepted by `BetStatusChangedEvent::new_status` / returned in
/// `old_status`.  Indexers can hard-code them.
pub const STATUS_ACTIVE: &str = "Active";
pub const STATUS_WON: &str = "Won";
pub const STATUS_LOST: &str = "Lost";
pub const STATUS_REFUNDED: &str = "Refunded";
pub const STATUS_CANCELLED: &str = "Cancelled";

// ===================================================================
// §3  Event struct definitions
// ===================================================================

/// Versioned topic / schema descriptor returned by
/// [`BettingEventSchema::get_schema`].
///
/// **Stable:** indexers may rely on `topic` and `schema_version`
/// together to demultiplex event kinds.  Bumping `schema_version` for
/// an existing event is a deliberate, advertised ABI break.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BettingEventSchemaEntry {
    /// Frozen topic symbol for this event kind.
    pub topic: Symbol,
    /// Frozen schema version for this topic.
    pub schema_version: u32,
}

/// Emitted when a single bet is successfully placed.
///
/// Fields are stable at schema version `1`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetCreatedEvent {
    /// Market on which the bet was placed.
    pub market_id: Symbol,
    /// Bettor's address.
    pub bettor: Address,
    /// Selected outcome identifier.
    pub outcome: String,
    /// Stake amount in base token units.
    pub amount: i128,
    /// End time of the market, copied for indexer convenience.
    pub market_end_time: u64,
    /// Monotonically-increasing nonce for the `lifebtcr` topic.
    pub nonce: u64,
    /// Ledger timestamp at emit time.
    pub timestamp: u64,
}

/// Emitted when a batch of bets is successfully placed.
///
/// Fields are stable at schema version `1`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetBatchCreatedEvent {
    /// Bettor's address (single user for the whole batch).
    pub bettor: Address,
    /// Number of bets in the batch.
    pub bet_count: u32,
    /// Total amount staked across the batch.
    pub total_amount: i128,
    /// Markets covered by the batch.
    pub market_ids: Vec<Symbol>,
    /// Monotonically-increasing nonce for the `lifebtch` topic.
    pub nonce: u64,
    /// Ledger timestamp at emit time.
    pub timestamp: u64,
}

/// Emitted whenever a bet transitions between lifecycle stages
/// (`Won`, `Lost`, `Refunded`, `Cancelled`).  Initial creation is
/// covered by [`BetCreatedEvent`] instead.
///
/// `old_status` and `new_status` are one of [`STATUS_ACTIVE`],
/// [`STATUS_WON`], [`STATUS_LOST`], [`STATUS_REFUNDED`],
/// [`STATUS_CANCELLED`].
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetStatusChangedEvent {
    /// Market on which the bet lives.
    pub market_id: Symbol,
    /// Bettor's address.
    pub bettor: Address,
    /// Pre-transition status (`"Active"` for the first transition).
    pub old_status: String,
    /// Post-transition status.  One of `Won` / `Lost` / `Refunded`
    /// / `Cancelled`.
    pub new_status: String,
    /// Payout amount if the bet is won and a payout is being
    /// processed by this transition.  Always `None` for `Lost`,
    /// `Refunded`, `Cancelled` transitions.
    pub payout_amount: Option<i128>,
    /// Monotonically-increasing nonce for the `lifebtsc` topic.
    pub nonce: u64,
    /// Ledger timestamp at emit time.
    pub timestamp: u64,
}

/// Emitted when a bettor successfully claims their payout from a
/// resolved market.  Separate from `BetStatusChangedEvent` because a
/// `Won` bet may not be claimed in the same transaction it was
/// resolved, and a claim does not change the bet's *bet* status (it
/// remains `Won`).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetClaimedEvent {
    /// Market from which winnings were claimed.
    pub market_id: Symbol,
    /// Claiming user's address.
    pub user: Address,
    /// Gross payout (before fees), in base token units.
    pub gross_payout: i128,
    /// Fee paid by the claimer, in base token units.
    pub fee_paid: i128,
    /// Net payout actually transferred (`gross - fee`).
    pub net_payout: i128,
    /// Monotonically-increasing nonce for the `lifebcml` topic.
    pub nonce: u64,
    /// Ledger timestamp at emit time.
    pub timestamp: u64,
}

/// Emitted whenever aggregated market-bet statistics change after a
/// state transition.  Acts as a low-cost aggregate stream that complements
/// per-bet [`BetStatusChangedEvent`]s.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BetStatsUpdatedEvent {
    /// Market whose stats changed.
    pub market_id: Symbol,
    /// Number of bet records stored in the registry.
    pub total_bets: u64,
    /// Total stake locked across all bets on the market.
    pub total_amount_locked: i128,
    /// Unique-bettor count.
    pub unique_bettors: u32,
    /// Monotonically-increasing nonce for the `lifebtst` topic.
    pub nonce: u64,
    /// Ledger timestamp at emit time.
    pub timestamp: u64,
}

// ===================================================================
// §4  Schema registry
// ===================================================================

/// Schema registry: maps a stable logical event name to its
/// `(topic, schema_version)` pair at the current contract revision.
///
/// # Stability
///
/// Returned entries are frozen at v1.  A schema bump for an existing
/// event requires the entry to return a different `schema_version` —
/// indexers SHOULD refuse events whose `(topic, schema_version)` is
/// not in the registry.
///
/// # Errors
///
/// Returns `None` when `name` is not a known event.  Callers that
/// treat a missing entry as a hard error must `unwrap_or` with a
/// panic or surface a typed error.
pub struct BettingEventSchema;

impl BettingEventSchema {
    /// Look up the [`BettingEventSchemaEntry`] for a logical event name.
    ///
    /// # Arguments
    ///
    /// * `_env` – Soroban environment.  Reserved for future versions
    ///   that may resolve entries from storage; current v1 is
    ///   compile-time.
    pub fn get_schema(_env: &Env, name: &str) -> Option<BettingEventSchemaEntry> {
        match name {
            EVENT_NAME_BET_CREATED => Some(BettingEventSchemaEntry {
                topic: TOPIC_BET_CREATED,
                schema_version: BETTING_EVENT_SCHEMA_VERSION,
            }),
            EVENT_NAME_BET_BATCH_CREATED => Some(BettingEventSchemaEntry {
                topic: TOPIC_BET_BATCH_CREATED,
                schema_version: BETTING_EVENT_SCHEMA_VERSION,
            }),
            EVENT_NAME_BET_STATUS_CHANGED => Some(BettingEventSchemaEntry {
                topic: TOPIC_BET_STATUS_CHANGED,
                schema_version: BETTING_EVENT_SCHEMA_VERSION,
            }),
            EVENT_NAME_BET_CLAIMED => Some(BettingEventSchemaEntry {
                topic: TOPIC_BET_CLAIMED,
                schema_version: BETTING_EVENT_SCHEMA_VERSION,
            }),
            EVENT_NAME_BET_STATS_UPDATED => Some(BettingEventSchemaEntry {
                topic: TOPIC_BET_STATS_UPDATED,
                schema_version: BETTING_EVENT_SCHEMA_VERSION,
            }),
            _ => None,
        }
    }
}

// ===================================================================
// §5  Storage helpers (no unwrap in production paths)
// ===================================================================

/// Build the tuple `(Symbol, Symbol)` storage key used to store the
/// monotonically-incrementing nonce for a given topic.
///
/// The key is `(NS_NONCE, topic)` so that two emitters (e.g. an admin
/// path and a user path) cannot accidentally share nonces with any
/// other subsystem that uses a single-key layout.
fn nonce_key(topic: Symbol) -> (Symbol, Symbol) {
    (NS_NONCE, topic)
}

/// Atomically read-and-increment the per-topic nonce, returning the
/// *new* (post-increment) value.
///
/// Uses [`u64::saturating_add`] so that counter overflow (impossible
/// in any practical contract lifetime, but spelled out for
/// correctness) plateaus at [`u64::MAX`] rather than wrapping or
/// silently resetting to 0.  Indexers should treat repeated
/// `u64::MAX` nonces for the same topic as a configuration error.
///
/// # Panics
/// This function never panics.
fn next_nonce(env: &Env, topic: Symbol) -> u64 {
    let key = nonce_key(topic);
    let current: u64 = env.storage().instance().get(&key).unwrap_or(0u64);
    
    env.storage().instance().extend_ttl(crate::INSTANCE_TTL_LEDGERS, crate::INSTANCE_TTL_LEDGERS);
    
    let incremented = current.saturating_add(1);
    env.storage().instance().set(&key, &incremented);
    incremented
}

// ===================================================================
// §6  Emitter
// ===================================================================

/// Single entry point for emitting structured betting-lifecycle events.
///
/// All public methods are pure emission: they neither perform `unwrap`
/// on user-controlled data nor panic under well-formed inputs that
/// the contract is documented to accept.  Misuse (e.g. zero / negative
/// amounts) is rejected upstream by the calling betting entrypoint,
/// not here.
pub struct BettingEventEmitter;

impl BettingEventEmitter {
    // ----- helpers (private) ---------------------------------------

    /// Stamp common fields: produce the
    /// `(nonce, timestamp)` pair that gets embedded in every event
    /// payload.
    fn stamp(env: &Env, topic: Symbol) -> (u64, u64) {
        let ts = env.ledger().timestamp();
        let nonce = next_nonce(env, topic);
        (nonce, ts)
    }

    // ----- public emitters -----------------------------------------

    /// Emit a [`BetCreatedEvent`] for a single successful bet placement.
    ///
    /// # Parameters
    ///
    /// * `env`         – Soroban environment
    /// * `market_id`   – market identifier
    /// * `bettor`      – placing user's address
    /// * `outcome`     – chosen outcome
    /// * `amount`      – stake in base token units
    /// * `market_end_time` – copied from the market so indexers do not
    ///   need to fetch the market just to render a deadline UI.
    ///
    /// # Publishes
    ///
    /// `(TOPIC_BET_CREATED, market_id, schema_version=1)` → event.
    pub fn emit_bet_created(
        env: &Env,
        market_id: &Symbol,
        bettor: &Address,
        outcome: &String,
        amount: i128,
        market_end_time: u64,
    ) {
        let (nonce, ts) = Self::stamp(env, TOPIC_BET_CREATED);
        let event = BetCreatedEvent {
            market_id: market_id.clone(),
            bettor: bettor.clone(),
            outcome: outcome.clone(),
            amount,
            market_end_time,
            nonce,
            timestamp: ts,
        };
        env.events().publish(
            (TOPIC_BET_CREATED, market_id.clone(), BETTING_EVENT_SCHEMA_VERSION),
            event,
        );
    }

    /// Emit a [`BetBatchCreatedEvent`] for a successful batch placement.
    ///
    /// # Publishes
    ///
    /// `(TOPIC_BET_BATCH_CREATED, bettor, schema_version=1)` → event.
    /// Note the second topic element is the bettor (not the market,
    /// because the batch may span many markets).
    pub fn emit_bet_batch_created(
        env: &Env,
        bettor: &Address,
        market_ids: &Vec<Symbol>,
        total_amount: i128,
    ) {
        let (nonce, ts) = Self::stamp(env, TOPIC_BET_BATCH_CREATED);
        let bet_count = market_ids.len();
        let event = BetBatchCreatedEvent {
            bettor: bettor.clone(),
            bet_count,
            total_amount,
            market_ids: market_ids.clone(),
            nonce,
            timestamp: ts,
        };
        env.events().publish(
            (TOPIC_BET_BATCH_CREATED, bettor.clone(), BETTING_EVENT_SCHEMA_VERSION),
            event,
        );
    }

    /// Emit a [`BetStatusChangedEvent`] for any non-creation lifecycle
    /// transition.
    ///
    /// `payout_amount` should be `Some(amount)` only for transitions
    /// that settle funds (e.g. `Active → Won` with an attached payout).
    /// For `Lost`, `Refunded`, `Cancelled`, pass `None`.
    pub fn emit_bet_status_changed(
        env: &Env,
        market_id: &Symbol,
        bettor: &Address,
        old_status: &str,
        new_status: &str,
        payout_amount: Option<i128>,
    ) {
        let (nonce, ts) = Self::stamp(env, TOPIC_BET_STATUS_CHANGED);
        let event = BetStatusChangedEvent {
            market_id: market_id.clone(),
            bettor: bettor.clone(),
            old_status: String::from_str(env, old_status),
            new_status: String::from_str(env, new_status),
            payout_amount,
            nonce,
            timestamp: ts,
        };
        env.events().publish(
            (TOPIC_BET_STATUS_CHANGED, market_id.clone(), BETTING_EVENT_SCHEMA_VERSION),
            event,
        );
    }

    /// Emit a [`BetClaimedEvent`] for a successful payout.
    ///
    /// # Parameters
    ///
    /// * `gross_payout` – payout before fees
    /// * `fee_paid`     – platform fee deducted from `gross_payout`
    /// * `net_payout`   – actually transferred (`gross - fee`)
    pub fn emit_bet_claimed(
        env: &Env,
        market_id: &Symbol,
        user: &Address,
        gross_payout: i128,
        fee_paid: i128,
        net_payout: i128,
    ) {
        let (nonce, ts) = Self::stamp(env, TOPIC_BET_CLAIMED);
        let event = BetClaimedEvent {
            market_id: market_id.clone(),
            user: user.clone(),
            gross_payout,
            fee_paid,
            net_payout,
            nonce,
            timestamp: ts,
        };
        env.events().publish(
            (TOPIC_BET_CLAIMED, user.clone(), BETTING_EVENT_SCHEMA_VERSION),
            event,
        );
    }

    /// Emit a [`BetStatsUpdatedEvent`] for aggregate-stats changes.
    pub fn emit_bet_stats_updated(
        env: &Env,
        market_id: &Symbol,
        total_bets: u64,
        total_amount_locked: i128,
        unique_bettors: u32,
    ) {
        let (nonce, ts) = Self::stamp(env, TOPIC_BET_STATS_UPDATED);
        let event = BetStatsUpdatedEvent {
            market_id: market_id.clone(),
            total_bets,
            total_amount_locked,
            unique_bettors,
            nonce,
            timestamp: ts,
        };
        env.events().publish(
            (TOPIC_BET_STATS_UPDATED, market_id.clone(), BETTING_EVENT_SCHEMA_VERSION),
            event,
        );
    }
}
