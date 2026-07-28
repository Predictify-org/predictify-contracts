//! # Betting crate
//!
//! The `betting` crate is a thin, indexing-focused wrapper around the
//! betting subsystems of [`predictify_hybrid`].  Its purpose is to expose a
//! **stable, structured, off-chain-indexer-friendly event surface** for
//! the bet lifecycle:
//!
//! * creation (single and batch),
//! * status transitions (resolve / cancel / refund),
//! * payout (claim),
//! * aggregated statistics updates.
//!
//! Every event in this module is:
//! * versioned through [`events::BettingEventSchema`] so indexers can
//!   route on `(topic, schema_version)`,
//! * monotonically nonce-stamped per topic so out-of-order / replayed
//!   events from any indexer source can be detected,
//! * stamped with the Soroban ledger timestamp so wall-clock-independent
//!   sequences can be reconstructed.
//!
//! See [`events`] for the public API.

#![no_std]

extern crate alloc;

pub mod events;

/// Default instance storage TTL bump in ledgers (~30 days).
pub const INSTANCE_TTL_LEDGERS: u32 = 535_680;
