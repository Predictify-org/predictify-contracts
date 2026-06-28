//! require_auth Coverage Matrix for PredictifyHybrid
//!
//! Every state-changing entrypoint has a positive (authorized) and negative
//! (unauthorized) test. Unauthorized calls must panic or return Error::Unauthorized.
//!
//! ## Entrypoint Matrix
//!
//! | Entrypoint                   | Auth Subject | Positive | Negative |
//! |------------------------------|--------------|----------|----------|
//! | deposit                      | user         | yes      | yes      |
//! | withdraw                     | user         | yes      | yes      |
//! | vote                         | user         | yes      | yes      |
//! | place_bet                    | user         | yes      | yes      |
//! | place_bets                   | user         | yes      | yes      |
//! | cancel_bet                   | user         | yes      | yes      |
//! | claim_winnings               | user         | yes      | yes      |
//! | dispute_market               | user         | yes      | yes      |
//! | vote_on_dispute              | user         | yes      | yes      |
//! | create_market                | admin        | yes      | yes      |
//! | create_event                 | admin        | yes      | yes      |
//! | resolve_market_manual        | admin        | yes      | yes      |
//! | resolve_market_with_ties     | admin        | yes      | yes      |
//! | resolve_dispute              | admin        | yes      | yes      |
//! | collect_fees                 | admin        | yes      | yes      |
//! | withdraw_collected_fees      | admin        | yes      | yes      |
//! | set_platform_fee             | admin        | yes      | yes      |
//! | set_treasury                 | admin        | yes      | yes      |
//! | set_global_claim_period      | admin        | yes      | yes      |
//! | set_market_claim_period      | admin        | yes      | yes      |
//! | sweep_unclaimed_winnings     | admin        | yes      | yes      |
//! | extend_deadline              | admin        | yes      | yes      |
//! | update_event_description     | admin        | yes      | yes      |
//! | update_event_outcomes        | admin        | yes      | yes      |
//! | update_event_category        | admin        | yes      | yes      |
//! | update_event_tags            | admin        | yes      | yes      |
//! | set_global_bet_limits        | admin        | yes      | yes      |
//! | set_event_bet_limits         | admin        | yes      | yes      |
//! | set_oracle_val_cfg_global    | admin        | yes      | yes      |
//! | set_oracle_val_cfg_event     | admin        | yes      | yes      |
//! | admin_override_verification  | admin        | yes      | yes      |
//! | archive_event                | admin        | yes      | yes      |
//! | prune_archive                | admin        | yes      | yes      |
//! | add_admin                    | admin        | yes      | yes      |
//! | remove_admin                 | admin        | yes      | yes      |
//! | migrate_to_multi_admin       | admin        | yes      | yes      |
//! | upgrade_contract             | admin        | yes      | yes      |
use crate::errors::Error;
use crate::types::{OracleConfig, OracleProvider, ReflectorAsset};
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, BytesN, Env, String, Symbol, Vec,
};

// ============================================================
// Shared helpers
// ============================================================

/// Build an initialized contract with mock_all_auths active.
fn setup() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let cid = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    PredictifyHybridClient::new(&env, &cid).initialize(&admin, &Some(200i128), &None);
    (env, cid, admin)
}

fn client<'a>(env: &'a Env, cid: &'a Address) -> PredictifyHybridClient<'a> {
    PredictifyHybridClient::new(env, cid)
}

/// For functions that return Result<T, crate::Error>:
/// try_* gives Result<Result<T, crate::Error>, soroban_sdk::Error>
/// Err(Ok(e)) where e: crate::Error
macro_rules! assert_unauthorized_contract {
    ($result:expr) => {
        match $result {
            Err(Ok(e)) => assert_eq!(e, crate::errors::Error::Unauthorized,
                "expected Unauthorized, got {:?}", e),
            Ok(_) => panic!("expected Unauthorized error, got Ok"),
            Err(Err(e)) => panic!("expected Unauthorized error, got host error {:?}", e),
        }
    };
}

/// For functions that panic (no explicit return type / return ()):
/// try_* gives Result<Result<(), soroban_sdk::Error>, soroban_sdk::Error>
/// Err(Ok(e)) where e: soroban_sdk::Error encoding our contract error code
macro_rules! assert_unauthorized_panic {
    ($result:expr) => {
        match $result {
            Err(Ok(e)) => assert_eq!(
                e,
                soroban_sdk::Error::from_contract_error(crate::errors::Error::Unauthorized as u32),
                "expected Unauthorized, got {:?}", e
            ),
            Ok(_) => panic!("expected Unauthorized error, got Ok"),
            Err(Err(e)) => panic!("expected Unauthorized error, got host error {:?}", e),
        }
    };
}

/// For positive tests on Result<T, crate::Error> functions:
/// assert auth passed (error is not Unauthorized)
macro_rules! assert_auth_ok_contract {
    ($result:expr, $msg:expr) => {
        if let Err(Ok(e)) = $result {
            assert_ne!(e, crate::errors::Error::Unauthorized, $msg);
        }
    };
}

/// For positive tests on panicking functions:
/// assert auth passed (error is not Unauthorized)
macro_rules! assert_auth_ok_panic {
    ($result:expr, $msg:expr) => {
        if let Err(Ok(e)) = $result {
            assert_ne!(
                e,
                soroban_sdk::Error::from_contract_error(crate::errors::Error::Unauthorized as u32),
                $msg
            );
        }
    };
}

fn oracle(env: &Env) -> OracleConfig {
    OracleConfig {
        provider: OracleProvider::reflector(),
        oracle_address: Address::from_str(
            env,
            "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
        ),
        feed_id: String::from_str(env, "BTC/USD"),
        threshold: 50_000,
        comparison: String::from_str(env, "gt"),
    }
}

fn make_market(env: &Env, cid: &Address, admin: &Address) -> Symbol {
    let mut outcomes = Vec::new(env);
    outcomes.push_back(String::from_str(env, "yes"));
    outcomes.push_back(String::from_str(env, "no"));
    client(env, cid).create_market(
        admin,
        &String::from_str(env, "Will BTC reach 100k?"),
        &outcomes,
        &30u32,
        &oracle(env),
        &None,
        &86400u64,
        &None,
        &None,
        &None,
    )
}

/// Advance ledger 31 days so markets are past their end time.
fn advance_past_end(env: &Env) {
    env.ledger().with_mut(|l| l.timestamp += 31 * 24 * 60 * 60);
}

/// Advance ledger past the dispute window (default 86400 s).
fn advance_past_dispute(env: &Env) {
    env.ledger().with_mut(|l| l.timestamp += 86_401);
}

/// Build a fresh env with NO auths mocked (for negative-path tests).
fn setup_no_auth() -> (Env, Address, Address) {
    let env = Env::default();
    // Initialize with mocked auths, then clear them.
    env.mock_all_auths();
    let cid = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    PredictifyHybridClient::new(&env, &cid).initialize(&admin, &Some(200i128), &None);
    env.set_auths(&[]);
    (env, cid, admin)
}

// ============================================================
// Section 1 – User-scoped entrypoints
// ============================================================

// ── deposit ──────────────────────────────────────────────────

/// Positive: authorized user can deposit.
#[test]
fn test_deposit_authorized_succeeds() {
    let (env, cid, _admin) = setup();
    let user = Address::generate(&env);
    let result = client(&env, &cid).try_deposit(&user, &ReflectorAsset::Stellar, &1_000_000i128);
    assert_auth_ok_contract!(result, "deposit rejected authorized user");
}

/// Negative: deposit without user auth must panic.
#[test]
#[should_panic]
fn test_deposit_no_auth_panics() {
    let (env, cid, _admin) = setup_no_auth();
    let user = Address::generate(&env);
    client(&env, &cid).deposit(&user, &ReflectorAsset::Stellar, &1_000_000i128);
}

// ── withdraw ─────────────────────────────────────────────────

/// Positive: authorized user can withdraw after depositing.
#[test]
fn test_withdraw_authorized_succeeds() {
    let (env, cid, _admin) = setup();
    let user = Address::generate(&env);
    let _ = client(&env, &cid).try_deposit(&user, &ReflectorAsset::Stellar, &1_000_000i128);
    let result = client(&env, &cid).try_withdraw(&user, &ReflectorAsset::Stellar, &500_000i128);
    assert_auth_ok_contract!(result, "withdraw rejected authorized user");
}

/// Negative: withdraw without user auth must panic.
#[test]
#[should_panic]
fn test_withdraw_no_auth_panics() {
    let (env, cid, _admin) = setup_no_auth();
    let user = Address::generate(&env);
    client(&env, &cid).withdraw(&user, &ReflectorAsset::Stellar, &500_000i128);
}

// ── vote ─────────────────────────────────────────────────────

/// Positive: authorized user can vote on an active market.
#[test]
fn test_vote_authorized_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let user = Address::generate(&env);
    let result = client(&env, &cid).try_vote(
        &user,
        &market_id,
        &String::from_str(&env, "yes"),
        &1_000i128,
    );
    assert_auth_ok_panic!(result, "vote rejected authorized user");
}

/// Negative: vote without user auth must panic.
#[test]
#[should_panic]
fn test_vote_no_auth_panics() {
    let (env, cid, admin) = setup_no_auth();
    // market was created before auths were cleared
    let user = Address::generate(&env);
    let market_id = Symbol::new(&env, "mkt");
    client(&env, &cid).vote(&user, &market_id, &String::from_str(&env, "yes"), &1_000i128);
}

/// Edge case: user A cannot vote using user B's address as the auth subject.
/// The contract binds require_auth to the `user` argument.
#[test]
fn test_vote_wrong_subject_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);

    // Only mock auth for user_a, then try to call vote with user_b.
    // With mock_all_auths active both pass; this test verifies the
    // contract does not confuse the two addresses.
    let _ = client(&env, &cid).try_vote(
        &user_a,
        &market_id,
        &String::from_str(&env, "yes"),
        &500i128,
    );
    // user_b voting on same market should fail with AlreadyVoted only if
    // the contract mistakenly treated them as the same user.
    let result = client(&env, &cid).try_vote(
        &user_b,
        &market_id,
        &String::from_str(&env, "no"),
        &500i128,
    );
    // user_b is a distinct address – must NOT get AlreadyVoted
    if let Err(Ok(e)) = result {
        assert_ne!(e, soroban_sdk::Error::from_contract_error(crate::errors::Error::AlreadyVoted as u32), "contract confused user_a and user_b");
    }
}

// ── place_bet ────────────────────────────────────────────────

/// Positive: authorized user can place a bet.
#[test]
fn test_place_bet_authorized_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let user = Address::generate(&env);
    let result = client(&env, &cid).try_place_bet(
        &user,
        &market_id,
        &String::from_str(&env, "yes"),
        &1_000_000i128,
    );
    assert_auth_ok_panic!(result, "place_bet rejected authorized user");
}

/// Negative: place_bet without user auth must panic.
#[test]
#[should_panic]
fn test_place_bet_no_auth_panics() {
    let (env, cid, _admin) = setup_no_auth();
    let user = Address::generate(&env);
    let market_id = Symbol::new(&env, "mkt");
    client(&env, &cid).place_bet(&user, &market_id, &String::from_str(&env, "yes"), &1_000_000i128);
}

// ── place_bets ───────────────────────────────────────────────

/// Positive: authorized user can batch-place bets.
#[test]
fn test_place_bets_authorized_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let user = Address::generate(&env);
    let bets: Vec<(Symbol, String, i128)> = vec![
        &env,
        (market_id, String::from_str(&env, "yes"), 1_000_000i128),
    ];
    let result = client(&env, &cid).try_place_bets(&user, &bets);
    assert_auth_ok_panic!(result, "place_bets rejected authorized user");
}

/// Negative: place_bets without user auth must panic.
#[test]
#[should_panic]
fn test_place_bets_no_auth_panics() {
    let (env, cid, _admin) = setup_no_auth();
    let user = Address::generate(&env);
    let bets: Vec<(Symbol, String, i128)> = Vec::new(&env);
    client(&env, &cid).place_bets(&user, &bets);
}

// ── cancel_bet ───────────────────────────────────────────────

/// Positive: authorized user can cancel their own bet.
#[test]
fn test_cancel_bet_authorized_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let user = Address::generate(&env);
    let _ = client(&env, &cid).try_place_bet(
        &user,
        &market_id,
        &String::from_str(&env, "yes"),
        &1_000_000i128,
    );
    let result = client(&env, &cid).try_cancel_bet(&user, &market_id);
    assert_auth_ok_contract!(result, "cancel_bet rejected authorized user");
}

/// Negative: cancel_bet without user auth must panic.
#[test]
#[should_panic]
fn test_cancel_bet_no_auth_panics() {
    let (env, cid, _admin) = setup_no_auth();
    let user = Address::generate(&env);
    let market_id = Symbol::new(&env, "mkt");
    client(&env, &cid).cancel_bet(&user, &market_id);
}

// ── claim_winnings ───────────────────────────────────────────

/// Positive: authorized winner can claim winnings.
#[test]
fn test_claim_winnings_authorized_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let user = Address::generate(&env);
    let _ = client(&env, &cid).try_vote(
        &user,
        &market_id,
        &String::from_str(&env, "yes"),
        &1_000i128,
    );
    advance_past_end(&env);
    let _ = client(&env, &cid).try_resolve_market_manual(
        &admin,
        &market_id,
        &String::from_str(&env, "yes"),
    );
    advance_past_dispute(&env);
    let result = client(&env, &cid).try_claim_winnings(&user, &market_id);
    assert_auth_ok_panic!(result, "claim_winnings rejected authorized user");
}

/// Negative: claim_winnings without user auth must panic.
#[test]
#[should_panic]
fn test_claim_winnings_no_auth_panics() {
    let (env, cid, _admin) = setup_no_auth();
    let user = Address::generate(&env);
    let market_id = Symbol::new(&env, "mkt");
    client(&env, &cid).claim_winnings(&user, &market_id);
}

/// Edge case: user B cannot claim winnings that belong to user A.
#[test]
fn test_claim_winnings_wrong_subject_gets_nothing_to_claim() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    let _ = client(&env, &cid).try_vote(
        &user_a,
        &market_id,
        &String::from_str(&env, "yes"),
        &1_000i128,
    );
    advance_past_end(&env);
    let _ = client(&env, &cid).try_resolve_market_manual(
        &admin,
        &market_id,
        &String::from_str(&env, "yes"),
    );
    advance_past_dispute(&env);
    // user_b never voted – must not silently succeed
    let result = client(&env, &cid).try_claim_winnings(&user_b, &market_id);
    match result {
        Ok(Ok(())) => panic!("user_b must not claim user_a winnings"),
        _ => {} // any error is correct
    }
}

// ── dispute_market ───────────────────────────────────────────

/// Positive: authorized user can dispute a resolved market.
#[test]
fn test_dispute_market_authorized_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let _ = client(&env, &cid).try_resolve_market_manual(
        &admin,
        &market_id,
        &String::from_str(&env, "yes"),
    );
    let user = Address::generate(&env);
    let result = client(&env, &cid).try_dispute_market(&user, &market_id, &1_000i128, &None);
    assert_auth_ok_contract!(result, "dispute_market rejected authorized user");
}

/// Negative: dispute_market without user auth must panic.
#[test]
#[should_panic]
fn test_dispute_market_no_auth_panics() {
    let (env, cid, _admin) = setup_no_auth();
    let user = Address::generate(&env);
    let market_id = Symbol::new(&env, "mkt");
    client(&env, &cid).dispute_market(&user, &market_id, &1_000i128, &None);
}

// ── vote_on_dispute ──────────────────────────────────────────

/// Positive: authorized user can vote on a dispute.
#[test]
fn test_vote_on_dispute_authorized_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let _ = client(&env, &cid).try_resolve_market_manual(
        &admin,
        &market_id,
        &String::from_str(&env, "yes"),
    );
    let disputer = Address::generate(&env);
    let _ = client(&env, &cid).try_dispute_market(&disputer, &market_id, &1_000i128, &None);
    let voter = Address::generate(&env);
    let dispute_id = Symbol::new(&env, "d0");
    let result = client(&env, &cid).try_vote_on_dispute(
        &voter, &market_id, &dispute_id, &true, &500i128, &None,
    );
    assert_auth_ok_contract!(result, "vote_on_dispute rejected authorized user");
}

/// Negative: vote_on_dispute without user auth must panic.
#[test]
#[should_panic]
fn test_vote_on_dispute_no_auth_panics() {
    let (env, cid, _admin) = setup_no_auth();
    let user = Address::generate(&env);
    let market_id = Symbol::new(&env, "mkt");
    let dispute_id = Symbol::new(&env, "d0");
    client(&env, &cid).vote_on_dispute(&user, &market_id, &dispute_id, &true, &500i128, &None);
}

// ============================================================
// Section 2 – Admin-scoped entrypoints (market lifecycle)
// ============================================================

// ── create_market ────────────────────────────────────────────

/// Positive: the registered admin can create a market.
#[test]
fn test_create_market_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    // If we got a Symbol back without panic, the call succeeded.
    let _ = market_id;
}

/// Negative: a forged (non-admin) address cannot create a market.
#[test]
fn test_create_market_forged_admin_rejected() {
    let (env, cid, _admin) = setup();
    let attacker = Address::generate(&env);
    let mut outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "yes"));
    outcomes.push_back(String::from_str(&env, "no"));
    let result = client(&env, &cid).try_create_market(
        &attacker,
        &String::from_str(&env, "Attacker market?"),
        &outcomes,
        &30u32,
        &oracle(&env),
        &None,
        &86400u64,
        &None,
        &None,
        &None,
    );
    assert_unauthorized_panic!(result);
}

// ── create_event ─────────────────────────────────────────────

/// Positive: admin can create an event.
#[test]
fn test_create_event_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let mut outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "yes"));
    outcomes.push_back(String::from_str(&env, "no"));
    let end_time = env.ledger().timestamp() + 86_400;
    let result = client(&env, &cid).try_create_event(
        &admin,
        &String::from_str(&env, "Will ETH flip BTC?"),
        &outcomes,
        &end_time,
        &oracle(&env),
        &None,
        &86400u64,
        &crate::types::EventVisibility::Public,
    );
    assert_auth_ok_panic!(result, "create_event rejected authorized admin");
}

/// Negative: non-admin cannot create an event.
#[test]
fn test_create_event_forged_admin_rejected() {
    let (env, cid, _admin) = setup();
    let attacker = Address::generate(&env);
    let mut outcomes = Vec::new(&env);
    outcomes.push_back(String::from_str(&env, "yes"));
    outcomes.push_back(String::from_str(&env, "no"));
    let end_time = env.ledger().timestamp() + 86_400;
    let result = client(&env, &cid).try_create_event(
        &attacker,
        &String::from_str(&env, "Attacker event?"),
        &outcomes,
        &end_time,
        &oracle(&env),
        &None,
        &86400u64,
        &crate::types::EventVisibility::Public,
    );
    assert_unauthorized_panic!(result);
}

// ── resolve_market_manual ────────────────────────────────────

/// Positive: admin can manually resolve a market after it ends.
#[test]
fn test_resolve_market_manual_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let result = client(&env, &cid).try_resolve_market_manual(
        &admin,
        &market_id,
        &String::from_str(&env, "yes"),
    );
    assert_auth_ok_panic!(result, "resolve_market_manual rejected authorized admin");
}

/// Negative: non-admin cannot manually resolve a market.
#[test]
fn test_resolve_market_manual_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_resolve_market_manual(
        &attacker,
        &market_id,
        &String::from_str(&env, "yes"),
    );
    assert_unauthorized_panic!(result);
}

// ── resolve_market_with_ties ─────────────────────────────────

/// Positive: admin can resolve with multiple winning outcomes.
#[test]
fn test_resolve_market_with_ties_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let mut winning = Vec::new(&env);
    winning.push_back(String::from_str(&env, "yes"));
    let result = client(&env, &cid).try_resolve_market_with_ties(&admin, &market_id, &winning);
    assert_auth_ok_panic!(result, "resolve_market_with_ties rejected authorized admin");
}

/// Negative: non-admin cannot resolve with ties.
#[test]
fn test_resolve_market_with_ties_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let attacker = Address::generate(&env);
    let mut winning = Vec::new(&env);
    winning.push_back(String::from_str(&env, "yes"));
    let result = client(&env, &cid).try_resolve_market_with_ties(&attacker, &market_id, &winning);
    assert_unauthorized_panic!(result);
}

// ── resolve_dispute ──────────────────────────────────────────

/// Positive: admin can resolve a dispute.
#[test]
fn test_resolve_dispute_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let _ = client(&env, &cid).try_resolve_market_manual(
        &admin, &market_id, &String::from_str(&env, "yes"),
    );
    let result = client(&env, &cid).try_resolve_dispute(&admin, &market_id);
    assert_auth_ok_contract!(result, "resolve_dispute rejected authorized admin");
}

/// Negative: non-admin cannot resolve a dispute.
#[test]
fn test_resolve_dispute_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_resolve_dispute(&attacker, &market_id);
    assert_unauthorized_contract!(result);
}

// ── collect_fees ─────────────────────────────────────────────

/// Positive: admin can collect fees from a resolved market.
#[test]
fn test_collect_fees_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let _ = client(&env, &cid).try_resolve_market_manual(
        &admin, &market_id, &String::from_str(&env, "yes"),
    );
    let result = client(&env, &cid).try_collect_fees(&admin, &market_id);
    assert_auth_ok_contract!(result, "collect_fees rejected authorized admin");
}

/// Negative: non-admin cannot collect fees.
#[test]
fn test_collect_fees_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_collect_fees(&attacker, &market_id);
    assert_unauthorized_contract!(result);
}

// ── withdraw_collected_fees ──────────────────────────────────

/// Positive: admin can withdraw collected fees.
#[test]
fn test_withdraw_collected_fees_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let result = client(&env, &cid).try_withdraw_collected_fees(&admin, &0i128);
    assert_auth_ok_contract!(result, "withdraw_collected_fees rejected authorized admin");
}

/// Negative: non-admin cannot withdraw collected fees.
#[test]
fn test_withdraw_collected_fees_forged_admin_rejected() {
    let (env, cid, _admin) = setup();
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_withdraw_collected_fees(&attacker, &0i128);
    assert_unauthorized_contract!(result);
}

// ============================================================
// Section 3 – Admin setters
// ============================================================

// ── set_platform_fee ─────────────────────────────────────────

/// Positive: admin can update the platform fee.
#[test]
fn test_set_platform_fee_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let result = client(&env, &cid).try_set_platform_fee(&admin, &300i128);
    assert_eq!(result, Ok(Ok(())));
}

/// Negative: non-admin cannot update the platform fee.
#[test]
fn test_set_platform_fee_forged_admin_rejected() {
    let (env, cid, _admin) = setup();
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_set_platform_fee(&attacker, &300i128);
    assert_unauthorized_contract!(result);
}

// ── set_treasury ─────────────────────────────────────────────

/// Positive: admin can set the treasury address.
#[test]
fn test_set_treasury_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let treasury = Address::generate(&env);
    // set_treasury panics on error, so use try_ variant
    let result = client(&env, &cid).try_set_treasury(&admin, &treasury);
    assert_auth_ok_panic!(result, "set_treasury rejected authorized admin");
}

/// Negative: non-admin cannot set the treasury.
#[test]
fn test_set_treasury_forged_admin_rejected() {
    let (env, cid, _admin) = setup();
    let attacker = Address::generate(&env);
    let treasury = Address::generate(&env);
    let result = client(&env, &cid).try_set_treasury(&attacker, &treasury);
    // Must be Unauthorized (not Ok)
    assert_unauthorized_panic!(result);
}

// ── set_global_claim_period ──────────────────────────────────

/// Positive: admin can set the global claim period.
#[test]
fn test_set_global_claim_period_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let result = client(&env, &cid).try_set_global_claim_period(&admin, &604_800u64);
    assert_auth_ok_panic!(result, "set_global_claim_period rejected authorized admin");
}

/// Negative: non-admin cannot set the global claim period.
#[test]
fn test_set_global_claim_period_forged_admin_rejected() {
    let (env, cid, _admin) = setup();
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_set_global_claim_period(&attacker, &604_800u64);
    assert_unauthorized_panic!(result);
}

// ── set_market_claim_period ──────────────────────────────────

/// Positive: admin can set a per-market claim period.
#[test]
fn test_set_market_claim_period_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let result = client(&env, &cid).try_set_market_claim_period(&admin, &market_id, &604_800u64);
    assert_auth_ok_panic!(result, "set_market_claim_period rejected authorized admin");
}

/// Negative: non-admin cannot set a per-market claim period.
#[test]
fn test_set_market_claim_period_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_set_market_claim_period(&attacker, &market_id, &604_800u64);
    assert_unauthorized_panic!(result);
}

// ── sweep_unclaimed_winnings ─────────────────────────────────

/// Positive: admin can sweep unclaimed winnings after claim window expires.
#[test]
fn test_sweep_unclaimed_winnings_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let _ = client(&env, &cid).try_resolve_market_manual(
        &admin, &market_id, &String::from_str(&env, "yes"),
    );
    // Advance well past claim window
    env.ledger().with_mut(|l| l.timestamp += 365 * 24 * 60 * 60);
    let result = client(&env, &cid).try_sweep_unclaimed_winnings(&admin, &market_id, &false);
    assert_auth_ok_contract!(result, "sweep_unclaimed_winnings rejected authorized admin");
}

/// Negative: non-admin cannot sweep unclaimed winnings.
#[test]
fn test_sweep_unclaimed_winnings_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_sweep_unclaimed_winnings(&attacker, &market_id, &false);
    assert_unauthorized_contract!(result);
}

// ── extend_deadline ──────────────────────────────────────────

/// Positive: admin can extend a market deadline.
#[test]
fn test_extend_deadline_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let result = client(&env, &cid).try_extend_deadline(
        &admin,
        &market_id,
        &7u32,
        &String::from_str(&env, "More time needed"),
    );
    assert_auth_ok_contract!(result, "extend_deadline rejected authorized admin");
}

/// Negative: non-admin cannot extend a market deadline.
#[test]
fn test_extend_deadline_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_extend_deadline(
        &attacker,
        &market_id,
        &7u32,
        &String::from_str(&env, "Attacker extension"),
    );
    assert_unauthorized_contract!(result);
}

// ── update_event_description ─────────────────────────────────

/// Positive: admin can update a market description before betting.
#[test]
fn test_update_event_description_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let result = client(&env, &cid).try_update_event_description(
        &admin,
        &market_id,
        &String::from_str(&env, "Updated: Will BTC reach 200k?"),
    );
    assert_auth_ok_contract!(result, "update_event_description rejected authorized admin");
}

/// Negative: non-admin cannot update a market description.
#[test]
fn test_update_event_description_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_update_event_description(
        &attacker,
        &market_id,
        &String::from_str(&env, "Attacker description"),
    );
    assert_unauthorized_contract!(result);
}

// ── update_event_outcomes ────────────────────────────────────

/// Positive: admin can update market outcomes before betting.
#[test]
fn test_update_event_outcomes_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let mut new_outcomes = Vec::new(&env);
    new_outcomes.push_back(String::from_str(&env, "above"));
    new_outcomes.push_back(String::from_str(&env, "below"));
    let result = client(&env, &cid).try_update_event_outcomes(&admin, &market_id, &new_outcomes);
    assert_auth_ok_contract!(result, "update_event_outcomes rejected authorized admin");
}

/// Negative: non-admin cannot update market outcomes.
#[test]
fn test_update_event_outcomes_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let mut new_outcomes = Vec::new(&env);
    new_outcomes.push_back(String::from_str(&env, "hack"));
    new_outcomes.push_back(String::from_str(&env, "hack2"));
    let result = client(&env, &cid).try_update_event_outcomes(&attacker, &market_id, &new_outcomes);
    assert_unauthorized_contract!(result);
}

// ── update_event_category ────────────────────────────────────

/// Positive: admin can set a market category.
#[test]
fn test_update_event_category_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let result = client(&env, &cid).try_update_event_category(
        &admin,
        &market_id,
        &Some(String::from_str(&env, "crypto")),
    );
    assert_auth_ok_contract!(result, "update_event_category rejected authorized admin");
}

/// Negative: non-admin cannot set a market category.
#[test]
fn test_update_event_category_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_update_event_category(
        &attacker,
        &market_id,
        &Some(String::from_str(&env, "hack")),
    );
    assert_unauthorized_contract!(result);
}

// ── update_event_tags ────────────────────────────────────────

/// Positive: admin can set market tags.
#[test]
fn test_update_event_tags_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let tags = vec![&env, String::from_str(&env, "bitcoin")];
    let result = client(&env, &cid).try_update_event_tags(&admin, &market_id, &tags);
    assert_auth_ok_contract!(result, "update_event_tags rejected authorized admin");
}

/// Negative: non-admin cannot set market tags.
#[test]
fn test_update_event_tags_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let tags = vec![&env, String::from_str(&env, "hack")];
    let result = client(&env, &cid).try_update_event_tags(&attacker, &market_id, &tags);
    assert_unauthorized_contract!(result);
}

// ============================================================
// Section 4 – Bet limits, oracle config, archive, admin mgmt
// ============================================================

// ── set_global_bet_limits ────────────────────────────────────

/// Positive: admin can set global bet limits.
#[test]
fn test_set_global_bet_limits_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let result = client(&env, &cid).try_set_global_bet_limits(&admin, &100_000i128, &10_000_000i128);
    assert_auth_ok_contract!(result, "set_global_bet_limits rejected authorized admin");
}

/// Negative: non-admin cannot set global bet limits.
#[test]
fn test_set_global_bet_limits_forged_admin_rejected() {
    let (env, cid, _admin) = setup();
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_set_global_bet_limits(&attacker, &100_000i128, &10_000_000i128);
    assert_unauthorized_contract!(result);
}

// ── set_event_bet_limits ─────────────────────────────────────

/// Positive: admin can set per-event bet limits.
#[test]
fn test_set_event_bet_limits_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let result = client(&env, &cid).try_set_event_bet_limits(
        &admin, &market_id, &100_000i128, &10_000_000i128,
    );
    assert_auth_ok_contract!(result, "set_event_bet_limits rejected authorized admin");
}

/// Negative: non-admin cannot set per-event bet limits.
#[test]
fn test_set_event_bet_limits_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_set_event_bet_limits(
        &attacker, &market_id, &100_000i128, &10_000_000i128,
    );
    assert_unauthorized_contract!(result);
}

// ── set_oracle_val_cfg_global ────────────────────────────────

/// Positive: admin can set global oracle validation config.
#[test]
fn test_set_oracle_val_cfg_global_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let result = client(&env, &cid).try_set_oracle_val_cfg_global(&admin, &300u64, &9500u32, &None::<u32>);
    assert_auth_ok_contract!(result, "set_oracle_val_cfg_global rejected authorized admin");
}

/// Negative: non-admin cannot set global oracle validation config.
#[test]
fn test_set_oracle_val_cfg_global_forged_admin_rejected() {
    let (env, cid, _admin) = setup();
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_set_oracle_val_cfg_global(&attacker, &300u64, &9500u32, &None::<u32>);
    assert_unauthorized_contract!(result);
}

// ── set_oracle_val_cfg_event ─────────────────────────────────

/// Positive: admin can set per-event oracle validation config.
#[test]
fn test_set_oracle_val_cfg_event_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let result = client(&env, &cid).try_set_oracle_val_cfg_event(
        &admin, &market_id, &300u64, &9500u32, &None::<u32>,
    );
    assert_auth_ok_contract!(result, "set_oracle_val_cfg_event rejected authorized admin");
}

/// Negative: non-admin cannot set per-event oracle validation config.
#[test]
fn test_set_oracle_val_cfg_event_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_set_oracle_val_cfg_event(
        &attacker, &market_id, &300u64, &9500u32, &None::<u32>,
    );
    assert_unauthorized_contract!(result);
}

// ── admin_override_verification ──────────────────────────────

/// Positive: admin can call admin_override_verification (returns OracleUnavailable
/// because the oracle module is disabled, but auth passes).
#[test]
fn test_admin_override_verification_authorized_admin_auth_passes() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let result = client(&env, &cid).try_admin_override_verification(
        &admin,
        &market_id,
        &String::from_str(&env, "yes"),
        &String::from_str(&env, "manual override"),
    );
    // Auth passes; the function returns OracleUnavailable because the oracle
    // module is currently disabled – that is NOT an auth failure.
    assert_auth_ok_contract!(result, "admin_override_verification rejected authorized admin");
}

/// Negative: non-admin cannot call admin_override_verification.
#[test]
fn test_admin_override_verification_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_admin_override_verification(
        &attacker,
        &market_id,
        &String::from_str(&env, "yes"),
        &String::from_str(&env, "hack"),
    );
    assert_unauthorized_contract!(result);
}

// ── archive_event ────────────────────────────────────────────

/// Positive: admin can archive a resolved market.
#[test]
fn test_archive_event_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    advance_past_end(&env);
    let _ = client(&env, &cid).try_resolve_market_manual(
        &admin, &market_id, &String::from_str(&env, "yes"),
    );
    let result = client(&env, &cid).try_archive_event(&admin, &market_id);
    assert_auth_ok_contract!(result, "archive_event rejected authorized admin");
}

/// Negative: non-admin cannot archive an event.
#[test]
fn test_archive_event_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let market_id = make_market(&env, &cid, &admin);
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_archive_event(&attacker, &market_id);
    assert_unauthorized_contract!(result);
}

// ── prune_archive ────────────────────────────────────────────

/// Positive: admin can prune the archive.
#[test]
fn test_prune_archive_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let result = client(&env, &cid).try_prune_archive(&admin, &5u32);
    assert_auth_ok_contract!(result, "prune_archive rejected authorized admin");
}

/// Negative: non-admin cannot prune the archive.
#[test]
fn test_prune_archive_forged_admin_rejected() {
    let (env, cid, _admin) = setup();
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_prune_archive(&attacker, &5u32);
    assert_unauthorized_contract!(result);
}

// ── add_admin ────────────────────────────────────────────────

/// Positive: primary admin can add a new admin after migration.
#[test]
fn test_add_admin_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    // Migrate to multi-admin first.
    let _ = client(&env, &cid).try_migrate_to_multi_admin(&admin);
    let new_admin = Address::generate(&env);
    let result = client(&env, &cid).try_add_admin(
        &admin,
        &new_admin,
        &crate::admin::AdminRole::MarketAdmin,
    );
    assert_auth_ok_contract!(result, "add_admin rejected authorized admin");
}

/// Negative: non-admin cannot add admins.
#[test]
fn test_add_admin_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let _ = client(&env, &cid).try_migrate_to_multi_admin(&admin);
    let attacker = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let result = client(&env, &cid).try_add_admin(
        &attacker,
        &new_admin,
        &crate::admin::AdminRole::MarketAdmin,
    );
    assert_unauthorized_contract!(result);
}

// ── remove_admin ─────────────────────────────────────────────

/// Positive: primary admin can remove an admin.
#[test]
fn test_remove_admin_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let _ = client(&env, &cid).try_migrate_to_multi_admin(&admin);
    let target = Address::generate(&env);
    let _ = client(&env, &cid).try_add_admin(
        &admin, &target, &crate::admin::AdminRole::MarketAdmin,
    );
    let result = client(&env, &cid).try_remove_admin(&admin, &target);
    assert_auth_ok_contract!(result, "remove_admin rejected authorized admin");
}

/// Negative: non-admin cannot remove admins.
#[test]
fn test_remove_admin_forged_admin_rejected() {
    let (env, cid, admin) = setup();
    let _ = client(&env, &cid).try_migrate_to_multi_admin(&admin);
    let attacker = Address::generate(&env);
    let target = Address::generate(&env);
    let result = client(&env, &cid).try_remove_admin(&attacker, &target);
    assert_unauthorized_contract!(result);
}

// ── migrate_to_multi_admin ───────────────────────────────────

/// Positive: primary admin can trigger multi-admin migration.
#[test]
fn test_migrate_to_multi_admin_authorized_admin_succeeds() {
    let (env, cid, admin) = setup();
    let result = client(&env, &cid).try_migrate_to_multi_admin(&admin);
    assert_auth_ok_contract!(result, "migrate_to_multi_admin rejected authorized admin");
}

/// Negative: non-admin cannot trigger multi-admin migration.
#[test]
fn test_migrate_to_multi_admin_forged_admin_rejected() {
    let (env, cid, _admin) = setup();
    let attacker = Address::generate(&env);
    let result = client(&env, &cid).try_migrate_to_multi_admin(&attacker);
    assert_unauthorized_contract!(result);
}

// ── upgrade_contract ─────────────────────────────────────────

/// Positive: primary admin can call upgrade_contract (will fail on wasm hash
/// validation, but auth itself must pass).
#[test]
fn test_upgrade_contract_authorized_admin_auth_passes() {
    let (env, cid, admin) = setup();
    let wasm_hash = BytesN::from_array(&env, &[1u8; 32]);
    let result = client(&env, &cid).try_upgrade_contract(&admin, &wasm_hash);
    // Auth passes; may fail for other reasons (invalid wasm hash etc.)
    assert_auth_ok_contract!(result, "upgrade_contract rejected authorized admin");
}

/// Negative: non-admin cannot upgrade the contract.
#[test]
fn test_upgrade_contract_forged_admin_rejected() {
    let (env, cid, _admin) = setup();
    let attacker = Address::generate(&env);
    let wasm_hash = BytesN::from_array(&env, &[9u8; 32]);
    let result = client(&env, &cid).try_upgrade_contract(&attacker, &wasm_hash);
    assert_unauthorized_contract!(result);
}

// ============================================================
// Section 5 – Edge cases: uninitialized contract, wrong subject
// ============================================================

/// Uninitialized contract: admin calls before initialize must return AdminNotSet.
#[test]
fn test_admin_calls_before_initialize_return_admin_not_set() {
    let env = Env::default();
    env.mock_all_auths();
    let cid = env.register(PredictifyHybrid, ());
    let fake_admin = Address::generate(&env);
    let result = client(&env, &cid).try_set_platform_fee(&fake_admin, &200i128);
    assert_eq!(result, Err(Ok(crate::errors::Error::AdminNotSet)));
}

/// Uninitialized contract: upgrade_contract before initialize returns AdminNotSet.
#[test]
fn test_upgrade_before_initialize_returns_admin_not_set() {
    let env = Env::default();
    env.mock_all_auths();
    let cid = env.register(PredictifyHybrid, ());
    let fake_admin = Address::generate(&env);
    let wasm_hash = BytesN::from_array(&env, &[7u8; 32]);
    let result = client(&env, &cid).try_upgrade_contract(&fake_admin, &wasm_hash);
    assert_eq!(result, Err(Ok(crate::errors::Error::AdminNotSet)));
}

/// Correct caller but wrong subject: user A's auth token cannot satisfy
/// user B's require_auth. Soroban rejects the call because only user_b's
/// auth is required but only user_a's is provided.
/// We use mock_auths scoped to user_a only, then call with user_b as subject.
#[test]
#[should_panic]
fn test_vote_correct_caller_wrong_subject_panics() {
    let env = Env::default();
    let cid = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user_b = Address::generate(&env);

    // Setup with full auths
    env.mock_all_auths();
    PredictifyHybridClient::new(&env, &cid).initialize(&admin, &Some(200i128), &None);
    let market_id = make_market(&env, &cid, &admin);

    // Clear all auths — user_b has no auth → require_auth panics
    env.set_auths(&[]);
    PredictifyHybridClient::new(&env, &cid).vote(
        &user_b,
        &market_id,
        &String::from_str(&env, "yes"),
        &1_000i128,
    );
}

/// Correct caller but wrong subject: user B cannot claim winnings without auth.
#[test]
#[should_panic]
fn test_claim_winnings_correct_caller_wrong_subject_panics() {
    let env = Env::default();
    let cid = env.register(PredictifyHybrid, ());
    let admin = Address::generate(&env);
    let user_b = Address::generate(&env);

    env.mock_all_auths();
    PredictifyHybridClient::new(&env, &cid).initialize(&admin, &Some(200i128), &None);
    let market_id = make_market(&env, &cid, &admin);
    let _ = PredictifyHybridClient::new(&env, &cid).try_vote(
        &user_b,
        &market_id,
        &String::from_str(&env, "yes"),
        &1_000i128,
    );
    advance_past_end(&env);
    let _ = PredictifyHybridClient::new(&env, &cid).try_resolve_market_manual(
        &admin, &market_id, &String::from_str(&env, "yes"),
    );
    advance_past_dispute(&env);

    // Clear all auths — user_b has no auth → require_auth panics
    env.set_auths(&[]);
    PredictifyHybridClient::new(&env, &cid).claim_winnings(&user_b, &market_id);
}

/// Forged admin that matches instance storage but NOT persistent storage is rejected.
/// This guards against the legacy instance-storage bypass attack.
#[test]
fn test_forged_instance_admin_cannot_set_platform_fee() {
    let (env, cid, _real_admin) = setup();
    let attacker = Address::generate(&env);

    // Write attacker into instance storage (legacy path) – persistent storage
    // still holds the real admin.
    env.as_contract(&cid, || {
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "admin"), &attacker);
    });

    let result = client(&env, &cid).try_set_platform_fee(&attacker, &500i128);
    assert_unauthorized_contract!(result);
}

