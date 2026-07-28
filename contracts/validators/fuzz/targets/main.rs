//! Cargo-fuzz target for the validators contract.
//!
//! This target drives every public validator entrypoint with values decoded
//! directly from arbitrary bytes. In particular, it explores zero, negative,
//! and extreme `i128` values for stake, score, and configuration operations,
//! while also exercising repeated lifecycle transitions and all read-only
//! views. Contract errors are intentionally ignored: malformed input is
//! expected to be rejected by the contract, whereas an unexpected host panic
//! is reported by libFuzzer.
//!
//! Run with:
//!
//! ```text
//! cargo +nightly fuzz run --fuzz-dir contracts/validators/fuzz main
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::Address as _,
    Address, Env,
};
use validators::{ValidatorsContract, ValidatorsContractClient};

const MIN_STAKE: i128 = 100;
const MAX_STAKE: i128 = 1_000_000;
const ACTIONS: u8 = 11;

/// Decode an `i128` from sixteen bytes, returning `None` when the corpus does
/// not contain a complete value.
fn take_i128(data: &[u8], index: &mut usize) -> Option<i128> {
    let end = index.checked_add(16)?;
    if end > data.len() {
        return None;
    }
    let value = i128::from_be_bytes(data[*index..end].try_into().ok()?);
    *index = end;
    Some(value)
}

/// Select one of the stable addresses created for this fuzz invocation.
fn address_at(addresses: &[Address], byte: u8) -> &Address {
    &addresses[(byte as usize) % addresses.len()]
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, ValidatorsContract);
    let client = ValidatorsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let mut addresses = Vec::with_capacity(6);
    for _ in 0..6 {
        addresses.push(Address::generate(&env));
    }

    // Establish a valid baseline before applying malformed operations. If the
    // corpus later attempts to re-initialize, the contract must reject it
    // without panicking.
    let _ = client.try_initialize(&admin, &MIN_STAKE, &MAX_STAKE);

    let mut index = 0usize;
    while index < data.len() {
        let action = data[index] % ACTIONS;
        index += 1;

        match action {
            // initialize(caller, min_stake, max_stake)
            0 => {
                let Some(caller_byte) = data.get(index).copied() else { break };
                index += 1;
                let Some(min_stake) = take_i128(data, &mut index) else { break };
                let Some(max_stake) = take_i128(data, &mut index) else { break };
                let caller = if caller_byte & 1 == 0 {
                    &admin
                } else {
                    address_at(&addresses, caller_byte)
                };
                let _ = client.try_initialize(caller, &min_stake, &max_stake);
            }
            // register_validator(validator, stake)
            1 => {
                let Some(caller_byte) = data.get(index).copied() else { break };
                index += 1;
                let Some(stake) = take_i128(data, &mut index) else { break };
                let validator = address_at(&addresses, caller_byte);
                let _ = client.try_register_validator(validator, &stake);
            }
            // deregister_validator(validator)
            2 => {
                let Some(byte) = data.get(index).copied() else { break };
                index += 1;
                let validator = address_at(&addresses, byte);
                let _ = client.try_deregister_validator(validator);
            }
            // update_stake(validator, stake)
            3 => {
                let Some(byte) = data.get(index).copied() else { break };
                index += 1;
                let Some(stake) = take_i128(data, &mut index) else { break };
                let validator = address_at(&addresses, byte);
                let _ = client.try_update_stake(validator, &stake);
            }
            // set_validator_active(admin, validator, active)
            4 => {
                let Some(bytes) = data.get(index..index + 2) else { break };
                index += 2;
                let validator = address_at(&addresses, bytes[0]);
                let active = bytes[1] & 1 != 0;
                let _ = client.try_set_validator_active(&admin, validator, &active);
            }
            // update_score(admin, validator, score)
            5 => {
                let Some(byte) = data.get(index).copied() else { break };
                index += 1;
                let Some(score) = take_i128(data, &mut index) else { break };
                let validator = address_at(&addresses, byte);
                let _ = client.try_update_score(&admin, validator, &score);
            }
            // update_stake_limits(admin, min_stake, max_stake)
            6 => {
                let Some(min_stake) = take_i128(data, &mut index) else { break };
                let Some(max_stake) = take_i128(data, &mut index) else { break };
                let _ = client.try_update_stake_limits(&admin, &min_stake, &max_stake);
            }
            // pause_validators(admin)
            7 => {
                let _ = client.try_pause_validators(&admin);
            }
            // unpause_validators(admin)
            8 => {
                let _ = client.try_unpause_validators(&admin);
            }
            // transfer_ownership(admin, new_admin)
            9 => {
                let Some(byte) = data.get(index).copied() else { break };
                index += 1;
                let new_admin = address_at(&addresses, byte);
                let _ = client.try_transfer_ownership(&admin, new_admin);
            }
            // Exercise all read-only entrypoints with arbitrary addresses.
            10 => {
                let Some(byte) = data.get(index).copied() else { break };
                index += 1;
                let validator = address_at(&addresses, byte);
                let _ = client.try_get_validator(validator);
                let _ = client.try_is_validator(validator);
                let _ = client.try_validator_count();
                let _ = client.try_is_validators_paused();
                let _ = client.try_admin();
                let _ = client.try_version();
            }
            _ => unreachable!(),
        }
    }
});
