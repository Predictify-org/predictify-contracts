#![cfg(test)]

use super::*;
use soroban_sdk::Env;

#[test]
fn test_capabilities_returns_bitmap() {
    let env = Env::default();
    let contract_id = env.register(Auction, ());
    let client = AuctionClient::new(&env, &contract_id);
    let caps = client.capabilities();
    assert_eq!(caps, CAPABILITIES);
    assert!(caps & 1 != 0, "basic auction must be supported");
    assert!(caps & 2 != 0, "reserve price must be supported");
    assert!(caps & 4 != 0, "time extensions must be supported");
    assert!(caps & 8 != 0, "cancellable must be supported");
}

#[test]
fn test_capabilities_is_read_only() {
    let env = Env::default();
    let contract_id = env.register(Auction, ());
    let client = AuctionClient::new(&env, &contract_id);
    let a = client.capabilities();
    let b = client.capabilities();
    assert_eq!(a, b, "capabilities must be idempotent");
}

