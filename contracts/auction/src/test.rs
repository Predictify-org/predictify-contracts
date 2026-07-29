use super::*;
use soroban_sdk::testutils::storage::Persistent as _;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::Env;

fn setup(env: &Env) -> soroban_sdk::Address {
    env.register(Auction, ())
}

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

#[test]
fn test_create_auction_requires_seller_auth() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup(&env);
    let client = AuctionClient::new(&env, &contract_id);
    let seller = soroban_sdk::Address::generate(&env);

    let id = client.create_auction(&seller, &1_000);
    assert_eq!(id, 0);
    assert_eq!(
        env.auths()[0].0,
        seller,
        "create_auction must require the seller's authorization"
    );
}

#[test]
fn test_create_auction_rejects_non_positive_reserve_price() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup(&env);
    let client = AuctionClient::new(&env, &contract_id);
    let seller = soroban_sdk::Address::generate(&env);

    let zero_result = client.try_create_auction(&seller, &0);
    assert_eq!(zero_result, Err(Ok(Error::InvalidReservePrice)));

    let negative_result = client.try_create_auction(&seller, &-1);
    assert_eq!(negative_result, Err(Ok(Error::InvalidReservePrice)));
}

#[test]
fn test_auction_ids_increment_across_creations() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup(&env);
    let client = AuctionClient::new(&env, &contract_id);
    let seller = soroban_sdk::Address::generate(&env);

    let first = client.create_auction(&seller, &500);
    let second = client.create_auction(&seller, &750);
    assert_eq!(first, 0);
    assert_eq!(second, 1);
}

#[test]
fn test_get_auction_returns_stored_data() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup(&env);
    let client = AuctionClient::new(&env, &contract_id);
    let seller = soroban_sdk::Address::generate(&env);

    let id = client.create_auction(&seller, &1_234);
    let data = client.get_auction(&id);

    assert_eq!(data.seller, seller);
    assert_eq!(data.reserve_price, 1_234);
    assert!(data.active);
}

#[test]
fn test_get_auction_not_found() {
    let env = Env::default();
    let contract_id = setup(&env);
    let client = AuctionClient::new(&env, &contract_id);

    let result = client.try_get_auction(&42);
    assert_eq!(result, Err(Ok(Error::AuctionNotFound)));
}

#[test]
fn test_create_auction_rejects_id_overflow() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup(&env);
    let client = AuctionClient::new(&env, &contract_id);
    let seller = soroban_sdk::Address::generate(&env);

    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&DataKey::NextAuctionId, &u64::MAX);
    });

    let result = client.try_create_auction(&seller, &10);
    assert_eq!(result, Err(Ok(Error::AuctionIdOverflow)));
}

#[test]
fn test_get_auction_bumps_ttl_when_below_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup(&env);
    let client = AuctionClient::new(&env, &contract_id);
    let seller = soroban_sdk::Address::generate(&env);

    let id = client.create_auction(&seller, &1_000);

    env.as_contract(&contract_id, || {
        let key = DataKey::Auction(id);
        assert_eq!(
            env.storage().persistent().get_ttl(&key),
            AUCTION_TTL_EXTEND_TO
        );

        env.ledger().with_mut(|li| {
            li.sequence_number += AUCTION_TTL_EXTEND_TO - AUCTION_TTL_THRESHOLD + 1;
        });
        assert!(env.storage().persistent().get_ttl(&key) < AUCTION_TTL_THRESHOLD);
    });

    client.get_auction(&id);

    env.as_contract(&contract_id, || {
        let key = DataKey::Auction(id);
        assert_eq!(
            env.storage().persistent().get_ttl(&key),
            AUCTION_TTL_EXTEND_TO,
            "a hot read below the threshold must restore the full TTL"
        );
    });
}

#[test]
fn test_get_auction_does_not_shrink_ttl_when_above_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = setup(&env);
    let client = AuctionClient::new(&env, &contract_id);
    let seller = soroban_sdk::Address::generate(&env);

    let id = client.create_auction(&seller, &1_000);
    client.get_auction(&id);

    env.as_contract(&contract_id, || {
        let key = DataKey::Auction(id);
        assert_eq!(
            env.storage().persistent().get_ttl(&key),
            AUCTION_TTL_EXTEND_TO,
            "extend_ttl only ever extends, never shortens"
        );
    });
}
