#![cfg(test)]

//! Invariant property test for the Allowlist contract.
//!
//! Exercises arbitrary valid action sequences and asserts that core
//! invariants hold after every operation:
//!
//! * **Membership**: An address added to an allowlist is reported as allowed,
//!   and an address removed is reported as not allowed.
//! * **Uniqueness**: An address cannot be added twice to the same allowlist
//!   (idempotent for batch, error for single).
//! * **Registry**: Every created allowlist ID appears in `list_allowlists()`;
//!   every deleted allowlist ID does not.
//! * **Admin**: Only the registered admin can perform state-changing
//!   operations; the admin address can be transferred.

use soroban_sdk::{testutils::Address as _, Address, Env, Symbol, Vec};
use allowlist::{AllowlistContract, AllowlistContractClient, AllowlistError};

struct Ctx {
    env: Env,
    client: AllowlistContractClient<'static>,
    admin: Address,
    users: Vec<Address>,
}

impl Ctx {
    fn new(num_users: u32) -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, AllowlistContract);
        let admin = Address::generate(&env);
        let client = AllowlistContractClient::new(&env, &contract_id);
        client.initialize(&admin);
        let mut users = Vec::new(&env);
        for _ in 0..num_users {
            users.push_back(Address::generate(&env));
        }
        Self { env, client, admin, users }
    }
    fn user(&self, idx: u32) -> Address { self.users.get(idx).unwrap() }
    fn sym(&self, s: &str) -> Symbol { Symbol::new(&self.env, s) }
}

// ---------------------------------------------------------------------------
// Membership invariant
// ---------------------------------------------------------------------------

#[test]
fn membership_invariant() {
    let ctx = Ctx::new(3);
    let list_id = ctx.sym("vip_list");
    ctx.client.create_allowlist(&ctx.admin, &list_id);
    ctx.client.add_address(&ctx.admin, &list_id, &ctx.user(0));
    assert!(ctx.client.is_allowed(&list_id, &ctx.user(0)), "user0 must be allowed after add");
    assert!(!ctx.client.is_allowed(&list_id, &ctx.user(1)), "user1 must NOT be allowed (never added)");
    ctx.client.remove_address(&ctx.admin, &list_id, &ctx.user(0));
    assert!(!ctx.client.is_allowed(&list_id, &ctx.user(0)), "user0 must NOT be allowed after remove");
}

#[test]
fn allowlist_content_matches_additions() {
    let ctx = Ctx::new(3);
    let list_id = ctx.sym("team");
    ctx.client.create_allowlist(&ctx.admin, &list_id);
    ctx.client.add_address(&ctx.admin, &list_id, &ctx.user(0));
    ctx.client.add_address(&ctx.admin, &list_id, &ctx.user(1));
    let all = ctx.client.get_allowlist(&list_id);
    assert_eq!(all.len(), 2);
    assert_eq!(all.get(0).unwrap(), ctx.user(0), "order: user0 first");
    assert_eq!(all.get(1).unwrap(), ctx.user(1), "order: user1 second");
}

// ---------------------------------------------------------------------------
// Uniqueness invariant
// ---------------------------------------------------------------------------

#[test]
fn duplicate_add_rejected() {
    let ctx = Ctx::new(1);
    let list_id = ctx.sym("solo");
    ctx.client.create_allowlist(&ctx.admin, &list_id);
    ctx.client.add_address(&ctx.admin, &list_id, &ctx.user(0));
    match ctx.client.try_add_address(&ctx.admin, &list_id, &ctx.user(0)) {
        Err(Ok(AllowlistError::AddressAlreadyInAllowlist)) => {}
        r => panic!("expected AddressAlreadyInAllowlist, got {:?}", r),
    }
}

#[test]
fn batch_add_is_idempotent() {
    let ctx = Ctx::new(2);
    let list_id = ctx.sym("batch");
    ctx.client.create_allowlist(&ctx.admin, &list_id);
    ctx.client.add_address(&ctx.admin, &list_id, &ctx.user(0));
    let addrs = soroban_sdk::vec![&ctx.env, ctx.user(0), ctx.user(1)];
    assert!(ctx.client.try_add_addresses(&ctx.admin, &list_id, &addrs).is_ok());
    let all = ctx.client.get_allowlist(&list_id);
    assert_eq!(all.len(), 2, "user0 must not be duplicated");
}

#[test]
fn remove_nonexistent_rejected() {
    let ctx = Ctx::new(1);
    let list_id = ctx.sym("ghost");
    ctx.client.create_allowlist(&ctx.admin, &list_id);
    match ctx.client.try_remove_address(&ctx.admin, &list_id, &ctx.user(0)) {
        Err(Ok(AllowlistError::AddressNotInAllowlist)) => {}
        r => panic!("expected AddressNotInAllowlist, got {:?}", r),
    }
}

// ---------------------------------------------------------------------------
// Registry consistency invariant
// ---------------------------------------------------------------------------

#[test]
fn registry_invariant() {
    let ctx = Ctx::new(0);
    let ids = [ctx.sym("a"), ctx.sym("b"), ctx.sym("c")];
    for id in &ids { ctx.client.create_allowlist(&ctx.admin, id); }
    let all = ctx.client.list_allowlists();
    assert_eq!(all.len(), 3);
    ctx.client.delete_allowlist(&ctx.admin, &ids[1]);
    let remaining = ctx.client.list_allowlists();
    assert_eq!(remaining.len(), 2);
    assert!(remaining.contains(&ids[0]));
    assert!(!remaining.contains(&ids[1]));
    assert!(remaining.contains(&ids[2]));
}

#[test]
fn clear_preserves_registration() {
    let ctx = Ctx::new(2);
    let list_id = ctx.sym("clear_me");
    ctx.client.create_allowlist(&ctx.admin, &list_id);
    ctx.client.add_address(&ctx.admin, &list_id, &ctx.user(0));
    ctx.client.add_address(&ctx.admin, &list_id, &ctx.user(1));
    ctx.client.clear_allowlist(&ctx.admin, &list_id);
    assert!(ctx.client.list_allowlists().contains(&list_id));
    assert_eq!(ctx.client.get_allowlist(&list_id).len(), 0);
    assert!(!ctx.client.is_allowed(&list_id, &ctx.user(0)));
    assert!(!ctx.client.is_allowed(&list_id, &ctx.user(1)));
}

// ---------------------------------------------------------------------------
// Admin ownership invariant
// ---------------------------------------------------------------------------

#[test]
fn ownership_transfer_invariant() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, AllowlistContract);
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let client = AllowlistContractClient::new(&env, &contract_id);
    client.initialize(&admin);
    client.transfer_ownership(&admin, &new_admin);
    assert_eq!(client.get_admin(), new_admin);
    assert!(client.try_create_allowlist(&admin, &Symbol::new(&env, "x")).is_err());
    assert!(client.try_create_allowlist(&new_admin, &Symbol::new(&env, "y")).is_ok());
}

// ---------------------------------------------------------------------------
// Full lifecycle
// ---------------------------------------------------------------------------

#[test]
fn full_lifecycle_sequence() {
    let ctx = Ctx::new(3);
    let list_id = ctx.sym("lifecycle");
    ctx.client.create_allowlist(&ctx.admin, &list_id);
    assert!(ctx.client.list_allowlists().contains(&list_id));
    ctx.client.add_address(&ctx.admin, &list_id, &ctx.user(0));
    ctx.client.add_address(&ctx.admin, &list_id, &ctx.user(1));
    assert!(ctx.client.is_allowed(&list_id, &ctx.user(0)));
    ctx.client.remove_address(&ctx.admin, &list_id, &ctx.user(0));
    assert!(!ctx.client.is_allowed(&list_id, &ctx.user(0)));
    let addrs = soroban_sdk::vec![&ctx.env, ctx.user(2)];
    ctx.client.add_addresses(&ctx.admin, &list_id, &addrs);
    assert!(ctx.client.is_allowed(&list_id, &ctx.user(2)));
    ctx.client.clear_allowlist(&ctx.admin, &list_id);
    assert!(!ctx.client.is_allowed(&list_id, &ctx.user(2)));
    ctx.client.delete_allowlist(&ctx.admin, &list_id);
    assert!(!ctx.client.list_allowlists().contains(&list_id));
}

// ---------------------------------------------------------------------------
// Independence
// ---------------------------------------------------------------------------

#[test]
fn multiple_allowlists_are_independent() {
    let ctx = Ctx::new(2);
    let a = ctx.sym("a"); let b = ctx.sym("b");
    ctx.client.create_allowlist(&ctx.admin, &a);
    ctx.client.create_allowlist(&ctx.admin, &b);
    ctx.client.add_address(&ctx.admin, &a, &ctx.user(0));
    ctx.client.add_address(&ctx.admin, &b, &ctx.user(1));
    assert!(ctx.client.is_allowed(&a, &ctx.user(0)));
    assert!(!ctx.client.is_allowed(&b, &ctx.user(0)));
    assert!(ctx.client.is_allowed(&b, &ctx.user(1)));
    assert!(!ctx.client.is_allowed(&a, &ctx.user(1)));
}

// ---------------------------------------------------------------------------
// Batch remove idempotency
// ---------------------------------------------------------------------------

#[test]
fn batch_remove_skips_absent_addresses() {
    let ctx = Ctx::new(2);
    let list_id = ctx.sym("batch_rm");
    ctx.client.create_allowlist(&ctx.admin, &list_id);
    ctx.client.add_address(&ctx.admin, &list_id, &ctx.user(0));
    let to_remove = soroban_sdk::vec![&ctx.env, ctx.user(0), ctx.user(1)];
    assert!(ctx.client.try_remove_addresses(&ctx.admin, &list_id, &to_remove).is_ok());
    assert!(!ctx.client.is_allowed(&list_id, &ctx.user(0)));
    assert!(!ctx.client.is_allowed(&list_id, &ctx.user(1)));
}
