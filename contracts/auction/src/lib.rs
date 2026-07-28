#![no_std]

use soroban_sdk::{contract, contractimpl, Env};

/// Bitmap of supported auction capabilities.
/// Clients can read this to detect feature deltas across deployments.
/// Current capabilities:
///   bit 0: basic auction (create, bid, finalize)
///   bit 1: reserve price support
///   bit 2: time extensions (anti-sniping)
///   bit 3: cancellable auctions
pub const CAPABILITIES: u64 = 0b1111;

#[contract]
pub struct Auction;

#[contractimpl]
impl Auction {
    /// Returns a u64 bitmap of supported features so clients can detect
    /// capability deltas without trial-and-error or version parsing.
    pub fn capabilities(_env: Env) -> u64 {
        CAPABILITIES
    }
}

#[cfg(test)]
mod test;

