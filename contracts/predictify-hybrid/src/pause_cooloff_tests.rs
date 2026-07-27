#[cfg(test)]
#[allow(unused_assignments)]
#[allow(unused_variables)]
#[allow(dead_code)]
mod pause_cooloff_tests {
    use crate::markets::{MarketPauseManager, MarketStateManager};
    use crate::types::{Market, MarketPauseInfo, MarketState};
    use crate::err::Error;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env, Symbol, vec, String};

    #[test]
    fn test_pause_cooloff_scenarios() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(crate::PredictifyHybrid, ());
        let admin = Address::generate(&env);

        env.as_contract(&contract_id, || {
            // Set up admin
            env.storage().persistent().set(&Symbol::new(&env, "Admin"), &admin);

            let market_id = Symbol::new(&env, "test_pause_market");
            let market = Market::new(
                &env,
                Address::generate(&env),
                String::from_str(&env, "Will it rain?"),
                vec![
                    &env,
                    String::from_str(&env, "yes"),
                    String::from_str(&env, "no"),
                ],
                env.ledger().timestamp() + 86400,
                crate::types::OracleConfig::new(
                    crate::types::OracleProvider::reflector(),
                    Address::generate(&env),
                    String::from_str(&env, "BTC/USD"),
                    2_500_000,
                    String::from_str(&env, "gt"),
                ),
                None,
                86400,
                MarketState::Active,
            );
            MarketStateManager::store_market(&env, &market_id, &market).unwrap();

            env.ledger().set_timestamp(100_000);

            // (d) resume on non-paused market still returns InvalidState unchanged
            assert_eq!(
                MarketPauseManager::resume_market(&env, admin.clone(), &market_id),
                Err(Error::InvalidState)
            );

            // Pause the market
            MarketPauseManager::pause_market(&env, admin.clone(), &market_id, 24).unwrap();

            // (e) non-admin caller still gets Unauthorized before the cool-off check runs
            let non_admin = Address::generate(&env);
            assert_eq!(
                MarketPauseManager::resume_market(&env, non_admin, &market_id),
                Err(Error::Unauthorized)
            );

            // (a) resume during cool-off returns CooloffActive
            assert_eq!(
                MarketPauseManager::resume_market(&env, admin.clone(), &market_id),
                Err(Error::CooloffActive)
            );

            // Move to right before the boundary
            env.ledger().set_timestamp(100_000 + 3599);
            assert_eq!(
                MarketPauseManager::resume_market(&env, admin.clone(), &market_id),
                Err(Error::CooloffActive)
            );

            // (c) exact boundary at MIN_UNPAUSE_COOLOFF_SECONDS
            env.ledger().set_timestamp(100_000 + 3600);
            
            // (b) resume after cool-off elapses succeeds
            assert_eq!(
                MarketPauseManager::resume_market(&env, admin.clone(), &market_id),
                Ok(())
            );
        });
    }
}
