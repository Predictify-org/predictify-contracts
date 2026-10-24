use crate::{Error, PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token::StellarAssetClient,
    Address, Env, IntoVal, String, Symbol, Vec,
};

fn register_contract(env: &Env) -> (Address, PredictifyHybridClient<'_>) {
    env.cost_estimate().disable_resource_limits();
    let contract_id = env.register(PredictifyHybrid, ());
    let client = PredictifyHybridClient::new(env, &contract_id);
    (contract_id, client)
}

fn store_admin(env: &Env, contract_id: &Address, admin: &Address) {
    env.as_contract(contract_id, || {
        env.storage()
            .persistent()
            .set(&Symbol::new(env, "Admin"), admin);
    });
}

fn setup_disputable_market(
    env: &Env,
    contract_id: &Address,
    user: &Address,
    market_id: &Symbol,
    stake: i128,
) -> Address {
    let token_admin = Address::generate(env);
    let token = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token.address();
    let token_admin_client = StellarAssetClient::new(env, &token_address);
    let mint_invoke = MockAuthInvoke {
        contract: &token_address,
        fn_name: "mint",
        args: (user, &stake).into_val(env),
        sub_invokes: &[],
    };
    token_admin_client
        .mock_auths(&[MockAuth {
            address: &token_admin,
            invoke: &mint_invoke,
        }])
        .mint(user, &stake);

    env.as_contract(contract_id, || {
        env.storage()
            .persistent()
            .set(&Symbol::new(env, "TokenID"), &token_address);

        let mut outcomes = Vec::new(env);
        outcomes.push_back(String::from_str(env, "yes"));
        outcomes.push_back(String::from_str(env, "no"));
        let mut market = crate::Market::new(
            env,
            Address::generate(env),
            String::from_str(env, "Single-frame auth regression"),
            outcomes,
            0,
            crate::OracleConfig::none_sentinel(env),
            None,
            86_400,
            crate::MarketState::Ended,
        );
        market.oracle_result = Some(String::from_str(env, "yes"));
        market.total_staked = stake;
        env.storage().persistent().set(market_id, &market);
    });

    token_address
}

#[test]
fn dispute_market_succeeds_with_one_auth_entry_and_updates_stakes() {
    let env = Env::default();
    let (contract_id, client) = register_contract(&env);
    let user = Address::generate(&env);
    let market_id = Symbol::new(&env, "dispute_market");
    let stake = 10_000_000i128;
    let reason: Option<String> = None;
    let token_address = setup_disputable_market(&env, &contract_id, &user, &market_id, stake);
    let token_client = soroban_sdk::token::Client::new(&env, &token_address);
    let contract_balance_before = token_client.balance(&contract_id);
    let token_transfer = MockAuthInvoke {
        contract: &token_address,
        fn_name: "transfer",
        args: (&user, &contract_id, &stake).into_val(&env),
        sub_invokes: &[],
    };
    let sub_invokes = [token_transfer];
    let invoke = MockAuthInvoke {
        contract: &contract_id,
        fn_name: "dispute_market",
        args: (&user, &market_id, &stake, &reason).into_val(&env),
        sub_invokes: &sub_invokes,
    };

    let result = client
        .mock_auths(&[MockAuth {
            address: &user,
            invoke: &invoke,
        }])
        .try_dispute_market(&user, &market_id, &stake, &reason);

    assert_eq!(result, Ok(Ok(())));
    let updated_market = env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .get::<Symbol, crate::Market>(&market_id)
            .unwrap()
    });
    assert_eq!(updated_market.dispute_stakes.get(user), Some(stake));
    assert_eq!(
        token_client.balance(&contract_id),
        contract_balance_before + stake
    );
}

#[test]
fn vote_on_dispute_reaches_business_validation_with_one_auth_entry() {
    let env = Env::default();
    let (contract_id, client) = register_contract(&env);
    let user = Address::generate(&env);
    let market_id = Symbol::new(&env, "missing_market");
    let dispute_id = Symbol::new(&env, "missing_dispute");
    let vote = true;
    let market_stake = 10_000_000i128;
    let vote_stake = 0i128;
    let reason: Option<String> = None;
    setup_disputable_market(&env, &contract_id, &user, &market_id, market_stake);
    env.as_contract(&contract_id, || {
        let voting = crate::disputes::DisputeVoting {
            dispute_id: dispute_id.clone(),
            voting_start: env.ledger().timestamp(),
            voting_end: env.ledger().timestamp() + 86_400,
            total_votes: 0,
            support_votes: 0,
            against_votes: 0,
            total_support_stake: 0,
            total_against_stake: 0,
            status: crate::disputes::DisputeVotingStatus::Active,
        };
        crate::disputes::DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();
    });
    let invoke = MockAuthInvoke {
        contract: &contract_id,
        fn_name: "vote_on_dispute",
        args: (&user, &market_id, &dispute_id, &vote, &vote_stake, &reason).into_val(&env),
        sub_invokes: &[],
    };

    let result = client
        .mock_auths(&[MockAuth {
            address: &user,
            invoke: &invoke,
        }])
        .try_vote_on_dispute(&user, &market_id, &dispute_id, &vote, &vote_stake, &reason);

    assert_eq!(result, Err(Ok(Error::InvalidInput)));
}

#[test]
fn vote_on_dispute_rejects_a_different_authenticated_address() {
    let env = Env::default();
    let (contract_id, client) = register_contract(&env);
    let user = Address::generate(&env);
    let other = Address::generate(&env);
    let market_id = Symbol::new(&env, "missing_market");
    let dispute_id = Symbol::new(&env, "missing_dispute");
    let vote = true;
    let stake = 1_000_000i128;
    let reason: Option<String> = None;
    let invoke = MockAuthInvoke {
        contract: &contract_id,
        fn_name: "vote_on_dispute",
        args: (&user, &market_id, &dispute_id, &vote, &stake, &reason).into_val(&env),
        sub_invokes: &[],
    };

    let result = client
        .mock_auths(&[MockAuth {
            address: &other,
            invoke: &invoke,
        }])
        .try_vote_on_dispute(&user, &market_id, &dispute_id, &vote, &stake, &reason);

    assert!(matches!(result, Err(Err(_))));
    let votes_after_failure = env.as_contract(&contract_id, || {
        crate::disputes::DisputeUtils::get_dispute_voting(&env, &dispute_id).unwrap()
    });
    assert_eq!(votes_after_failure.total_votes, 0);
}

#[test]
fn vote_on_dispute_succeeds_with_one_auth_entry_and_updates_voting() {
    let env = Env::default();
    let (contract_id, client) = register_contract(&env);
    let user = Address::generate(&env);
    let market_id = Symbol::new(&env, "vote_market");
    let dispute_id = Symbol::new(&env, "active_dispute");
    let vote = true;
    let stake = 10_000_000i128;
    let reason: Option<String> = None;
    let token_address = setup_disputable_market(&env, &contract_id, &user, &market_id, stake);

    env.as_contract(&contract_id, || {
        let voting = crate::disputes::DisputeVoting {
            dispute_id: dispute_id.clone(),
            voting_start: env.ledger().timestamp(),
            voting_end: env.ledger().timestamp() + 86_400,
            total_votes: 0,
            support_votes: 0,
            against_votes: 0,
            total_support_stake: 0,
            total_against_stake: 0,
            status: crate::disputes::DisputeVotingStatus::Active,
        };
        crate::disputes::DisputeUtils::store_dispute_voting(&env, &dispute_id, &voting).unwrap();
    });

    let token_transfer = MockAuthInvoke {
        contract: &token_address,
        fn_name: "transfer",
        args: (&user, &contract_id, &stake).into_val(&env),
        sub_invokes: &[],
    };
    let sub_invokes = [token_transfer];
    let invoke = MockAuthInvoke {
        contract: &contract_id,
        fn_name: "vote_on_dispute",
        args: (&user, &market_id, &dispute_id, &vote, &stake, &reason).into_val(&env),
        sub_invokes: &sub_invokes,
    };

    let result = client
        .mock_auths(&[MockAuth {
            address: &user,
            invoke: &invoke,
        }])
        .try_vote_on_dispute(&user, &market_id, &dispute_id, &vote, &stake, &reason);

    assert_eq!(result, Ok(Ok(())));
    let updated_voting = env.as_contract(&contract_id, || {
        crate::disputes::DisputeUtils::get_dispute_voting(&env, &dispute_id).unwrap()
    });
    assert_eq!(updated_voting.total_votes, 1);
    assert_eq!(updated_voting.total_support_stake, stake);
}

#[test]
fn resolve_dispute_reaches_business_validation_after_entrypoint_identity_check() {
    let env = Env::default();
    let (contract_id, client) = register_contract(&env);
    let admin = Address::generate(&env);
    let market_id = Symbol::new(&env, "missing_market");
    store_admin(&env, &contract_id, &admin);
    let invoke = MockAuthInvoke {
        contract: &contract_id,
        fn_name: "resolve_dispute",
        args: (&admin, &market_id).into_val(&env),
        sub_invokes: &[],
    };

    let result = client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &invoke,
        }])
        .try_resolve_dispute(&admin, &market_id);

    assert_eq!(result, Err(Ok(Error::MarketNotFound)));
}

#[test]
fn resolve_dispute_preserves_unauthorized_error_for_non_admin() {
    let env = Env::default();
    let (contract_id, client) = register_contract(&env);
    let admin = Address::generate(&env);
    let other = Address::generate(&env);
    let market_id = Symbol::new(&env, "missing_market");
    store_admin(&env, &contract_id, &admin);
    let invoke = MockAuthInvoke {
        contract: &contract_id,
        fn_name: "resolve_dispute",
        args: (&other, &market_id).into_val(&env),
        sub_invokes: &[],
    };

    let result = client
        .mock_auths(&[MockAuth {
            address: &other,
            invoke: &invoke,
        }])
        .try_resolve_dispute(&other, &market_id);

    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

#[test]
fn collect_fees_reaches_business_validation_with_one_auth_entry() {
    let env = Env::default();
    let (contract_id, client) = register_contract(&env);
    let admin = Address::generate(&env);
    let market_id = Symbol::new(&env, "missing_market");
    store_admin(&env, &contract_id, &admin);
    let invoke = MockAuthInvoke {
        contract: &contract_id,
        fn_name: "collect_fees",
        args: (&admin, &market_id).into_val(&env),
        sub_invokes: &[],
    };

    let result = client
        .mock_auths(&[MockAuth {
            address: &admin,
            invoke: &invoke,
        }])
        .try_collect_fees(&admin, &market_id);

    assert_eq!(result, Err(Ok(Error::MarketNotFound)));
}
