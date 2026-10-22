#![cfg(test)]

use crate::fees::{
    FeeConfig, FeeConfigCommit, FeeManager, HistoricalFeeConfig, MIN_DELAY_LEDGERS,
};
use crate::errors::Error;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    BytesN, Env, Address, Symbol, Map,
};

fn make_env() -> Env {
    Env::default()
}

fn register_contract(env: &Env) -> Address {
    env.register(crate::PredictifyHybrid, ())
}

fn set_admin(env: &Env, admin: &Address) {
    env.storage()
        .persistent()
                        .set(&Symbol::new(env, "Admin"), admin);
}

fn create_sample_config(platform_fee: i128) -> FeeConfig {
    FeeConfig {
        platform_fee_percentage: platform_fee,
        creation_fee: 10_000_000,
        min_fee_amount: 1_000_000,
        max_fee_amount: 1_000_000_000,
        collection_threshold: 100_000_000,
        fees_enabled: true,
    }
}

fn get_config_hash(env: &Env, config: &FeeConfig) -> BytesN<32> {
    use soroban_sdk::xdr::ToXdr;
    let bytes = config.to_xdr(env);
    env.crypto().sha256(&bytes).into()
}

#[test]
fn test_commit_saves_hash_and_sequence() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);

        let config = create_sample_config(300);
        let hash = get_config_hash(&env, &config);

        let commit_result = FeeManager::commit_fee_config(&env, admin.clone(), hash.clone());
        assert!(commit_result.is_ok());

        // Verify the commit is stored
        let commit_key = soroban_sdk::symbol_short!("fc_cmt");
        let stored_commit: FeeConfigCommit = env
            .storage()
            .persistent()
            .get(&commit_key)
            .expect("Commit not found in storage");

        assert_eq!(stored_commit.hash, hash);
        assert_eq!(stored_commit.committed_at, env.ledger().sequence());
    });
}

#[test]
fn test_recommit_overwrites_pending_commit() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);

        let config1 = create_sample_config(300);
        let hash1 = get_config_hash(&env, &config1);
        assert!(FeeManager::commit_fee_config(&env, admin.clone(), hash1).is_ok());

        // Fast-forward ledger
        env.ledger().with_mut(|li| li.sequence_number = li.sequence_number.saturating_add(10));

        let config2 = create_sample_config(400);
        let hash2 = get_config_hash(&env, &config2);
        assert!(FeeManager::commit_fee_config(&env, admin.clone(), hash2.clone()).is_ok());

        let commit_key = soroban_sdk::symbol_short!("fc_cmt");
        let stored_commit: FeeConfigCommit = env
            .storage()
            .persistent()
            .get(&commit_key)
            .unwrap();

        assert_eq!(stored_commit.hash, hash2);
        assert_eq!(stored_commit.committed_at, env.ledger().sequence());
    });
}

#[test]
fn test_reveal_without_commit_fails() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);

        let config = create_sample_config(300);
        let reveal_result = FeeManager::update_fee_config(&env, admin.clone(), config);
        assert_eq!(reveal_result, Err(Error::NoPendingFeeCommit));
    });
}

#[test]
fn test_reveal_before_delay_fails() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);

        let config = create_sample_config(300);
        let hash = get_config_hash(&env, &config);
        assert!(FeeManager::commit_fee_config(&env, admin.clone(), hash).is_ok());

        // Try revealing immediately
        let reveal_result = FeeManager::update_fee_config(&env, admin.clone(), config);
        assert_eq!(reveal_result, Err(Error::FeeRevealTooEarly));
    });
}

#[test]
fn test_mismatched_preimage_fails() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);

        let config = create_sample_config(300);
        let hash = get_config_hash(&env, &config);
        assert!(FeeManager::commit_fee_config(&env, admin.clone(), hash).is_ok());

        // Fast-forward delay
        env.ledger().with_mut(|li| li.sequence_number = li.sequence_number.saturating_add(MIN_DELAY_LEDGERS + 1));

        // Try revealing with a different config (different preimage)
        let mismatched_config = create_sample_config(400);
        let reveal_result = FeeManager::update_fee_config(&env, admin.clone(), mismatched_config);
        assert_eq!(reveal_result, Err(Error::FeePreimageMismatch));
    });
}

#[test]
fn test_successful_reveal_updates_config_and_history() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);

        let original_config = FeeManager::get_fee_config(&env).unwrap();
        let original_timestamp = env.ledger().timestamp();

        let new_config = create_sample_config(300);
        let hash = get_config_hash(&env, &new_config);
        assert!(FeeManager::commit_fee_config(&env, admin.clone(), hash).is_ok());

        // Fast-forward delay & time
        env.ledger().with_mut(|li| li.sequence_number = li.sequence_number.saturating_add(MIN_DELAY_LEDGERS + 1));
        env.ledger().set_timestamp(original_timestamp + 600);

        let reveal_result = FeeManager::update_fee_config(&env, admin.clone(), new_config.clone());
        assert_eq!(reveal_result, Ok(new_config.clone()));

        // Verify new config is active
        let active_config = FeeManager::get_fee_config(&env).unwrap();
        assert_eq!(active_config, new_config);

        // Verify history contains original config
        let history_key = soroban_sdk::symbol_short!("fc_hist");
        let history: soroban_sdk::Vec<HistoricalFeeConfig> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap();

        assert_eq!(history.len(), 1);
        let history_entry = history.get(0).unwrap();
        assert_eq!(history_entry.config, original_config);
        assert_eq!(history_entry.replaced_at, original_timestamp + 600);
    });
}

#[test]
fn test_fee_resolution_uses_correct_historical_config() {
    let env = make_env();
    let contract_id = register_contract(&env);
    let admin = Address::generate(&env);
    env.mock_all_auths();

    env.as_contract(&contract_id, || {
        set_admin(&env, &admin);

        let t0 = env.ledger().timestamp();

        // 1. Reveal a new config (Config 1, 300 bps)
        let config1 = create_sample_config(300);
        let hash1 = get_config_hash(&env, &config1);
        assert!(FeeManager::commit_fee_config(&env, admin.clone(), hash1).is_ok());
        env.ledger().with_mut(|li| li.sequence_number = li.sequence_number.saturating_add(MIN_DELAY_LEDGERS + 1));
        env.ledger().set_timestamp(t0 + 1000);
        assert!(FeeManager::update_fee_config(&env, admin.clone(), config1).is_ok());

        // 2. Reveal another config (Config 2, 400 bps)
        let config2 = create_sample_config(400);
        let hash2 = get_config_hash(&env, &config2);
        assert!(FeeManager::commit_fee_config(&env, admin.clone(), hash2).is_ok());
        env.ledger().with_mut(|li| li.sequence_number = li.sequence_number.saturating_add(MIN_DELAY_LEDGERS + 1));
        env.ledger().set_timestamp(t0 + 2000);
        assert!(FeeManager::update_fee_config(&env, admin.clone(), config2).is_ok());

        // Verify lookup:
        // - At t0 + 500 (before Config 1), should return original config (200 bps)
        assert_eq!(FeeManager::get_fee_percentage_for_timestamp(&env, t0 + 500), 200);

        // - At t0 + 1500 (between Config 1 and Config 2), should return Config 1 (300 bps)
        assert_eq!(FeeManager::get_fee_percentage_for_timestamp(&env, t0 + 1500), 300);

        // - At t0 + 2500 (after Config 2), should return Config 2 (400 bps)
        assert_eq!(FeeManager::get_fee_percentage_for_timestamp(&env, t0 + 2500), 400);
    });
}
