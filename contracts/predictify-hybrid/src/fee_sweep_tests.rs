use crate::err::Error;
use crate::fee_sweep::FeeSweepManager;
use crate::recovery;
use crate::storage::BalanceStorage;
use crate::types::ReflectorAsset;
use crate::PredictifyHybrid;
use soroban_sdk::{testutils::Address as _, Address, Env, Symbol};

fn test_env() -> (Env, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let contract_id = env.register(PredictifyHybrid, ());

    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&Symbol::new(&env, "Admin"), &admin);
        recovery::UnclaimedWinningsPolicy::set_treasury(&env, &treasury);
    });

    (env, admin, treasury)
}

#[test]
fn sweep_protocol_fees_to_treasury_credits_treasury_and_clears_vault() {
    let (env, admin, treasury) = test_env();
    let contract_id = env.register(PredictifyHybrid, ());

    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "tot_fees"), &42_000i128);

        let swept = PredictifyHybrid::sweep_protocol_fees_to_treasury(env.clone(), admin.clone())
            .unwrap();

        assert_eq!(swept, 42_000);
        assert_eq!(
            BalanceStorage::get_balance(&env, &treasury, &ReflectorAsset::Stellar).amount,
            42_000
        );
        assert_eq!(
            env.storage()
                .persistent()
                .get::<Symbol, i128>(&Symbol::new(&env, "tot_fees"))
                .unwrap_or(0),
            0
        );
    });
}

#[test]
fn sweep_protocol_fees_to_treasury_is_noop_when_vault_is_empty() {
    let (env, admin, treasury) = test_env();
    let contract_id = env.register(PredictifyHybrid, ());

    env.as_contract(&contract_id, || {
        let swept = PredictifyHybrid::sweep_protocol_fees_to_treasury(env.clone(), admin.clone())
            .unwrap();

        assert_eq!(swept, 0);
        assert_eq!(
            BalanceStorage::get_balance(&env, &treasury, &ReflectorAsset::Stellar).amount,
            0
        );
    });
}

#[test]
fn sweep_protocol_fees_to_treasury_rejects_when_treasury_not_configured() {
    let (env, admin, _) = test_env();
    let contract_id = env.register(PredictifyHybrid, ());

    env.as_contract(&contract_id, || {
        env.storage()
            .persistent()
            .remove(&Symbol::new(&env, "treasury_addr"));
        env.storage()
            .persistent()
            .set(&Symbol::new(&env, "tot_fees"), &10_000i128);

        let result = PredictifyHybrid::sweep_protocol_fees_to_treasury(env.clone(), admin.clone());

        assert_eq!(result, Err(Error::ConfigNotFound));
    });
}
