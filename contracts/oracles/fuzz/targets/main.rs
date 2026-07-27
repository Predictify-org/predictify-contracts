#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    contract, contractimpl, Address, Env, String as SorobanString, Vec as SorobanVec,
    testutils::Address as _,
};
use oracles::{OraclesContract, OraclesContractClient, OraclePriceData};

/// A mock oracle contract whose behavior is dynamically configurable via instances storage.
#[contract]
pub struct FuzzMockOracle;

#[contractimpl]
impl FuzzMockOracle {
    /// Retrieve the current price, panicking if configured to fail.
    pub fn get_price(env: Env, _feed_id: SorobanString) -> i128 {
        if env.storage().instance().get(&soroban_sdk::symbol_short!("panic")).unwrap_or(false) {
            panic!("oracle failed");
        }
        env.storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("price"))
            .unwrap_or(0i128)
    }

    /// Retrieve the enriched price metadata, panicking if configured to fail either get_pdata or overall.
    pub fn get_pdata(env: Env, _feed_id: SorobanString) -> OraclePriceData {
        if env.storage().instance().get(&soroban_sdk::symbol_short!("panic")).unwrap_or(false) ||
           env.storage().instance().get(&soroban_sdk::symbol_short!("panic_pd")).unwrap_or(false) {
            panic!("oracle failed");
        }
        let price = env.storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("price"))
            .unwrap_or(0i128);
        let publish_time = env.storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("time"))
            .unwrap_or(0u64);
        let confidence = env.storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("conf"))
            .unwrap_or(None);
        let exponent = env.storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("exp"))
            .unwrap_or(0i32);

        OraclePriceData {
            price,
            publish_time,
            confidence,
            exponent,
        }
    }

    /// Retrieve health status, panicking if configured to fail.
    pub fn is_live(env: Env) -> bool {
        if env.storage().instance().get(&soroban_sdk::symbol_short!("panic")).unwrap_or(false) {
            panic!("oracle failed");
        }
        env.storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("live"))
            .unwrap_or(true)
    }
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, OraclesContract);
    let client = OraclesContractClient::new(&env, &contract_id);

    // Pre-register 5 mock oracle addresses.
    let mut oracle_addresses = SorobanVec::new(&env);
    for _ in 0..5 {
        oracle_addresses.push_back(env.register_contract(None, FuzzMockOracle));
    }

    let mut idx = 0;
    while idx < data.len() {
        let action_type = data[idx] % 5;
        idx += 1;

        match action_type {
            0 => {
                // AddOracle: registers an oracle and configures its mock behavior.
                if idx + 26 > data.len() {
                    break;
                }
                let oracle_idx = (data[idx] % 5) as u32;
                let price = i128::from_be_bytes(data[idx+1..idx+17].try_into().unwrap());
                let time = u64::from_be_bytes(data[idx+17..idx+25].try_into().unwrap());
                let live = data[idx+25] & 1 != 0;
                let panic_all = data[idx+25] & 2 != 0;
                let panic_pdata = data[idx+25] & 4 != 0;
                idx += 26;

                let oracle_addr = oracle_addresses.get(oracle_idx).unwrap();
                
                env.as_contract(&oracle_addr, || {
                    env.storage().instance().set(&soroban_sdk::symbol_short!("price"), &price);
                    env.storage().instance().set(&soroban_sdk::symbol_short!("time"), &time);
                    env.storage().instance().set(&soroban_sdk::symbol_short!("live"), &live);
                    env.storage().instance().set(&soroban_sdk::symbol_short!("panic"), &panic_all);
                    env.storage().instance().set(&soroban_sdk::symbol_short!("panic_pd"), &panic_pdata);
                    
                    let conf = if price % 2 == 0 { Some(price / 10) } else { None };
                    let exp = (price % 10) as i32;
                    env.storage().instance().set(&soroban_sdk::symbol_short!("conf"), &conf);
                    env.storage().instance().set(&soroban_sdk::symbol_short!("exp"), &exp);
                });

                let _ = client.add_oracle(&admin, &oracle_addr);
            }
            1 => {
                // RemoveOracle: removes a mock oracle.
                if idx + 1 > data.len() {
                    break;
                }
                let oracle_idx = (data[idx] % 5) as u32;
                idx += 1;
                let oracle_addr = oracle_addresses.get(oracle_idx).unwrap();
                let _ = client.remove_oracle(&admin, &oracle_addr);
            }
            2 => {
                // GetPrice: queries price for a registered or unregistered oracle.
                if idx + 9 > data.len() {
                    break;
                }
                let oracle_idx = (data[idx] % 6) as u32; // 0..4 = registered, 5 = unregistered
                let mut feed_bytes = [0u8; 8];
                feed_bytes.copy_from_slice(&data[idx+1..idx+9]);
                idx += 9;

                let oracle_addr = if oracle_idx < 5 {
                    oracle_addresses.get(oracle_idx).unwrap()
                } else {
                    Address::generate(&env)
                };

                let feed_str = match std::str::from_utf8(&feed_bytes) {
                    Ok(s) => SorobanString::from_str(&env, s),
                    Err(_) => SorobanString::from_str(&env, "BTC/USD"),
                };

                let _ = client.get_price(&oracle_addr, &feed_str);
            }
            3 => {
                // GetPriceData: queries enriched price data.
                if idx + 9 > data.len() {
                    break;
                }
                let oracle_idx = (data[idx] % 6) as u32;
                let mut feed_bytes = [0u8; 8];
                feed_bytes.copy_from_slice(&data[idx+1..idx+9]);
                idx += 9;

                let oracle_addr = if oracle_idx < 5 {
                    oracle_addresses.get(oracle_idx).unwrap()
                } else {
                    Address::generate(&env)
                };

                let feed_str = match std::str::from_utf8(&feed_bytes) {
                    Ok(s) => SorobanString::from_str(&env, s),
                    Err(_) => SorobanString::from_str(&env, "BTC/USD"),
                };

                let _ = client.get_price_data(&oracle_addr, &feed_str);
            }
            4 => {
                // IsOracleHealthy: queries liveness checks.
                if idx + 1 > data.len() {
                    break;
                }
                let oracle_idx = (data[idx] % 6) as u32;
                idx += 1;

                let oracle_addr = if oracle_idx < 5 {
                    oracle_addresses.get(oracle_idx).unwrap()
                } else {
                    Address::generate(&env)
                };

                let _ = client.is_oracle_healthy(&oracle_addr);
            }
            _ => unreachable!(),
        }
    }
});
