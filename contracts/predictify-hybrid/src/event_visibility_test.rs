#[cfg(test)]
mod event_visibility_tests {
    use crate::types::{EventVisibility, OracleConfig, OracleProvider};
    use crate::{PredictifyHybrid, PredictifyHybridClient};
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::token::StellarAssetClient;
    use soroban_sdk::{Address, Env, String, Symbol, Vec};

    fn setup_test_env() -> (Env, Address, Address, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        // Set a non-zero timestamp to avoid overflow in tests
        env.ledger().with_mut(|li| {
            li.timestamp = 10000;
        });

        let admin = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        let contract_id = env.register(PredictifyHybrid, ());

        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None);

        // Configure token used for fees and staking
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
        });

        // Fund admin with tokens so creation fees can be paid
        let stellar_client = StellarAssetClient::new(&env, &token_id);
        env.mock_all_auths();
        stellar_client.mint(&admin, &1_000_000_0000000); // 1,000 XLM

        (env, contract_id, admin, user1, user2)
    }

    fn create_test_oracle_config(env: &Env) -> OracleConfig {
        OracleConfig::new(
            OracleProvider::Reflector,
            Address::generate(env),
            String::from_str(env, "BTC/USD"),
            50_000_00,
            String::from_str(env, "gt"),
        )
    }

    #[test]
    fn test_create_public_event() {
        let (env, contract_id, admin, _, _) = setup_test_env();
        let client = PredictifyHybridClient::new(&env, &contract_id);

        let event_id = client.create_event(
            &admin,
            &String::from_str(&env, "Will BTC reach $50k?"),
            &Vec::from_array(&env, [
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ]),
            &(env.ledger().timestamp() + 86400),
            &create_test_oracle_config(&env),
            &None,
            &3600,
            &EventVisibility::Public,
        );

        let event = client.get_event(&event_id).unwrap();
        assert_eq!(event.visibility, EventVisibility::Public);
        assert_eq!(event.allowlist.len(), 0);
    }

    #[test]
    fn test_create_private_event() {
        let (env, contract_id, admin, _, _) = setup_test_env();
        let client = PredictifyHybridClient::new(&env, &contract_id);

        let event_id = client.create_event(
            &admin,
            &String::from_str(&env, "Private event"),
            &Vec::from_array(&env, [
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ]),
            &(env.ledger().timestamp() + 86400),
            &create_test_oracle_config(&env),
            &None,
            &3600,
            &EventVisibility::Private,
        );

        let event = client.get_event(&event_id).unwrap();
        assert_eq!(event.visibility, EventVisibility::Private);
        assert_eq!(event.allowlist.len(), 0);
    }

    #[test]
    fn test_add_to_allowlist() {
        let (env, contract_id, admin, user1, user2) = setup_test_env();
        let client = PredictifyHybridClient::new(&env, &contract_id);

        let event_id = client.create_event(
            &admin,
            &String::from_str(&env, "Private event"),
            &Vec::from_array(&env, [
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ]),
            &(env.ledger().timestamp() + 86400),
            &create_test_oracle_config(&env),
            &None,
            &3600,
            &EventVisibility::Private,
        );

        let addresses = Vec::from_array(&env, [user1.clone(), user2.clone()]);
        client.add_to_allowlist(&admin, &event_id, &addresses);

        let event = client.get_event(&event_id).unwrap();
        assert_eq!(event.allowlist.len(), 2);
        assert!(event.allowlist.contains(&user1));
        assert!(event.allowlist.contains(&user2));
    }

    #[test]
    fn test_remove_from_allowlist() {
        let (env, contract_id, admin, user1, user2) = setup_test_env();
        let client = PredictifyHybridClient::new(&env, &contract_id);

        let event_id = client.create_event(
            &admin,
            &String::from_str(&env, "Private event"),
            &Vec::from_array(&env, [
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ]),
            &(env.ledger().timestamp() + 86400),
            &create_test_oracle_config(&env),
            &None,
            &3600,
            &EventVisibility::Private,
        );

        let addresses = Vec::from_array(&env, [user1.clone(), user2.clone()]);
        client.add_to_allowlist(&admin, &event_id, &addresses);

        let remove_addresses = Vec::from_array(&env, [user1.clone()]);
        client.remove_from_allowlist(&admin, &event_id, &remove_addresses);

        let event = client.get_event(&event_id).unwrap();
        assert_eq!(event.allowlist.len(), 1);
        assert!(!event.allowlist.contains(&user1));
        assert!(event.allowlist.contains(&user2));
    }

    #[test]
    fn test_set_event_visibility() {
        let (env, contract_id, admin, _, _) = setup_test_env();
        let client = PredictifyHybridClient::new(&env, &contract_id);

        let event_id = client.create_event(
            &admin,
            &String::from_str(&env, "Test event"),
            &Vec::from_array(&env, [
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ]),
            &(env.ledger().timestamp() + 86400),
            &create_test_oracle_config(&env),
            &None,
            &3600,
            &EventVisibility::Public,
        );

        client.set_event_visibility(&admin, &event_id, &EventVisibility::Private);

        let event = client.get_event(&event_id).unwrap();
        assert_eq!(event.visibility, EventVisibility::Private);
    }

    #[test]
    #[should_panic]
    fn test_private_event_blocks_non_allowlisted_user() {
        let (env, contract_id, admin, user1, user2) = setup_test_env();
        let client = PredictifyHybridClient::new(&env, &contract_id);

        let event_id = client.create_event(
            &admin,
            &String::from_str(&env, "Private event"),
            &Vec::from_array(&env, [
                String::from_str(&env, "yes"),
                String::from_str(&env, "no"),
            ]),
            &(env.ledger().timestamp() + 86400),
            &create_test_oracle_config(&env),
            &None,
            &3600,
            &EventVisibility::Private,
        );

        let addresses = Vec::from_array(&env, [user1.clone()]);
        client.add_to_allowlist(&admin, &event_id, &addresses);

        // Try to bet with user2 (not on allowlist) - should panic
        client.place_bet(
            &user2,
            &event_id,
            &String::from_str(&env, "yes"),
            &1_000_000,
        );
    }
}
