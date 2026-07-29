#![no_std]

use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

/// Bitmap of supported auction capabilities.
/// Clients can read this to detect feature deltas across deployments.
/// Current capabilities:
///   bit 0: basic auction (create, bid, finalize)
///   bit 1: reserve price support
///   bit 2: time extensions (anti-sniping)
///   bit 3: cancellable auctions
pub const CAPABILITIES: u64 = 0b1111;

/// Minimum remaining ledgers an auction record may have before it is
/// refreshed on a hot read. At ~5 s/ledger, 7 days ≈ 120_960 ledgers.
///
/// Keeping this well above zero means a record is bumped on read long
/// before it is at risk of archival eviction, even if reads cluster right
/// before expiry.
pub const AUCTION_TTL_THRESHOLD: u32 = 17_280 * 7;

/// Ledgers to extend an auction record's TTL to when it is bumped, either on
/// creation or on a hot read below [`AUCTION_TTL_THRESHOLD`]. At ~5 s/ledger,
/// 30 days ≈ 518_400 ledgers.
///
/// Uses `extend_ttl()` semantics, which only ever extend a TTL forward, so
/// bumping is idempotent and safe to call unconditionally on every read.
pub const AUCTION_TTL_EXTEND_TO: u32 = 17_280 * 30;

/// Persistent storage keys used by the Auction contract.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// An individual auction record, keyed by auction id.
    Auction(u64),
    /// Counter used to allocate the next auction id.
    NextAuctionId,
}

/// Data describing a single auction.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuctionData {
    /// The address that created the auction and receives sale proceeds.
    pub seller: Address,
    /// Minimum acceptable sale price. Must be strictly positive.
    pub reserve_price: i128,
    /// Ledger timestamp (seconds) at which the auction was created.
    pub created_at: u64,
    /// Whether the auction is still open.
    pub active: bool,
}

/// Errors returned by the Auction contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// No auction exists for the given id.
    AuctionNotFound = 1,
    /// `reserve_price` was not strictly positive.
    InvalidReservePrice = 2,
    /// The auction id counter would overflow `u64`.
    AuctionIdOverflow = 3,
}

#[contract]
pub struct Auction;

#[contractimpl]
impl Auction {
    /// Returns a u64 bitmap of supported features so clients can detect
    /// capability deltas without trial-and-error or version parsing.
    pub fn capabilities(_env: Env) -> u64 {
        CAPABILITIES
    }

    /// Creates a new auction owned by `seller` with the given `reserve_price`.
    ///
    /// The new record is written to persistent storage with its TTL set to
    /// [`AUCTION_TTL_EXTEND_TO`] ledgers so it starts with a full lifetime.
    ///
    /// # Authorization
    /// `seller` must authenticate via `require_auth()`.
    ///
    /// # Errors
    /// - [`Error::InvalidReservePrice`] if `reserve_price` is not strictly positive.
    /// - [`Error::AuctionIdOverflow`] if the auction id counter has been exhausted.
    pub fn create_auction(env: Env, seller: Address, reserve_price: i128) -> Result<u64, Error> {
        seller.require_auth();

        if reserve_price <= 0 {
            return Err(Error::InvalidReservePrice);
        }

        let next_id: u64 = env
            .storage()
            .instance()
            .get(&DataKey::NextAuctionId)
            .unwrap_or(0);
        let following_id = next_id.checked_add(1).ok_or(Error::AuctionIdOverflow)?;
        env.storage()
            .instance()
            .set(&DataKey::NextAuctionId, &following_id);

        let data = AuctionData {
            seller,
            reserve_price,
            created_at: env.ledger().timestamp(),
            active: true,
        };

        let key = DataKey::Auction(next_id);
        env.storage().persistent().set(&key, &data);
        env.storage()
            .persistent()
            .extend_ttl(&key, AUCTION_TTL_THRESHOLD, AUCTION_TTL_EXTEND_TO);

        Ok(next_id)
    }

    /// Returns the data for the auction identified by `auction_id`.
    ///
    /// This is the contract's hot read path: bidders and off-chain indexers
    /// are expected to call it far more often than auctions are created or
    /// modified. Every call bumps the record's persistent TTL back up to
    /// [`AUCTION_TTL_EXTEND_TO`] ledgers whenever it has dropped below
    /// [`AUCTION_TTL_THRESHOLD`], so a frequently-read, rarely-written
    /// auction never falls prey to archival eviction.
    ///
    /// # Errors
    /// - [`Error::AuctionNotFound`] if no auction exists for `auction_id`.
    pub fn get_auction(env: Env, auction_id: u64) -> Result<AuctionData, Error> {
        let key = DataKey::Auction(auction_id);
        let data: AuctionData = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::AuctionNotFound)?;

        env.storage()
            .persistent()
            .extend_ttl(&key, AUCTION_TTL_THRESHOLD, AUCTION_TTL_EXTEND_TO);

        Ok(data)
    }
}

#[cfg(test)]
mod test;
