//! Cargo-fuzz target for the reporting contract.
//!
//! Drives every state-changing entrypoint with values decoded directly from
//! arbitrary bytes: report/dispute IDs, status codes, and report/reason text
//! (including empty and unusually long strings) are all exercised, alongside
//! repeated pause/unpause and ownership-transfer sequences. Contract errors
//! are intentionally ignored: malformed input is expected to be rejected by
//! the contract, whereas an unexpected host panic is reported by libFuzzer.
//!
//! Run with:
//!
//! ```text
//! cargo +nightly fuzz run --fuzz-dir contracts/reporting/fuzz main
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use soroban_sdk::{testutils::Address as _, Address, Env, String as SorobanString};
use reporting::{ReportingContract, ReportingContractClient};

const ACTIONS: u8 = 11;
const STRINGS: [&str; 4] = [
    "",
    "a",
    "a normal report body describing an incident",
    "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
];

/// Decode a `u32` from four bytes, returning `None` when the corpus does not
/// contain a complete value.
fn take_u32(data: &[u8], index: &mut usize) -> Option<u32> {
    let end = index.checked_add(4)?;
    if end > data.len() {
        return None;
    }
    let value = u32::from_be_bytes(data[*index..end].try_into().ok()?);
    *index = end;
    Some(value)
}

/// Select one of the stable addresses created for this fuzz invocation.
fn address_at(addresses: &[Address], byte: u8) -> &Address {
    &addresses[(byte as usize) % addresses.len()]
}

/// Select one of a small set of boundary-condition strings.
fn string_at(env: &Env, byte: u8) -> SorobanString {
    SorobanString::from_str(env, STRINGS[(byte as usize) % STRINGS.len()])
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(ReportingContract, ());
    let client = ReportingContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let mut addresses = Vec::with_capacity(6);
    for _ in 0..6 {
        addresses.push(Address::generate(&env));
    }

    // Establish a valid baseline before applying malformed operations. If the
    // corpus later attempts to re-initialize, the contract must reject it
    // without panicking.
    let _ = client.try_initialize(&admin);

    let mut index = 0usize;
    while index < data.len() {
        let action = data[index] % ACTIONS;
        index += 1;

        match action {
            // initialize(caller)
            0 => {
                let Some(byte) = data.get(index).copied() else { break };
                index += 1;
                let caller = address_at(&addresses, byte);
                let _ = client.try_initialize(caller);
            }
            // submit_report(reporter, market_id, report_data, report_hash)
            1 => {
                let Some(caller_byte) = data.get(index).copied() else { break };
                index += 1;
                let Some(market_id) = take_u32(data, &mut index) else { break };
                let Some(data_byte) = data.get(index).copied() else { break };
                index += 1;
                let Some(hash_byte) = data.get(index).copied() else { break };
                index += 1;

                let reporter = address_at(&addresses, caller_byte);
                let report_data = string_at(&env, data_byte);
                let report_hash = string_at(&env, hash_byte);
                let _ = client.try_submit_report(reporter, &market_id, &report_data, &report_hash);
            }
            // verify_report(admin, report_id, verification_result)
            2 => {
                let Some(report_id) = take_u32(data, &mut index) else { break };
                let Some(byte) = data.get(index).copied() else { break };
                index += 1;
                let result = byte & 1 != 0;
                let _ = client.try_verify_report(&admin, &report_id, &result);
            }
            // dispute_report(reporter, report_id, dispute_reason)
            3 => {
                let Some(caller_byte) = data.get(index).copied() else { break };
                index += 1;
                let Some(report_id) = take_u32(data, &mut index) else { break };
                let Some(reason_byte) = data.get(index).copied() else { break };
                index += 1;

                let reporter = address_at(&addresses, caller_byte);
                let reason = string_at(&env, reason_byte);
                let _ = client.try_dispute_report(reporter, &report_id, &reason);
            }
            // resolve_dispute(admin, dispute_id, resolution)
            4 => {
                let Some(dispute_id) = take_u32(data, &mut index) else { break };
                let Some(byte) = data.get(index).copied() else { break };
                index += 1;
                let resolution = byte & 1 != 0;
                let _ = client.try_resolve_dispute(&admin, &dispute_id, &resolution);
            }
            // update_report_status(admin, report_id, new_status)
            5 => {
                let Some(report_id) = take_u32(data, &mut index) else { break };
                let Some(new_status) = take_u32(data, &mut index) else { break };
                let _ = client.try_update_report_status(&admin, &report_id, &new_status);
            }
            // delete_report(admin, report_id)
            6 => {
                let Some(report_id) = take_u32(data, &mut index) else { break };
                let _ = client.try_delete_report(&admin, &report_id);
            }
            // pause_reporting(admin)
            7 => {
                let _ = client.try_pause_reporting(&admin);
            }
            // unpause_reporting(admin)
            8 => {
                let _ = client.try_unpause_reporting(&admin);
            }
            // transfer_ownership(admin, new_owner)
            9 => {
                let Some(byte) = data.get(index).copied() else { break };
                index += 1;
                let new_owner = address_at(&addresses, byte);
                let _ = client.try_transfer_ownership(&admin, new_owner);
            }
            // Exercise all read-only entrypoints.
            10 => {
                let _ = client.try_is_reporting_paused();
                let _ = client.try_admin();
            }
            _ => unreachable!(),
        }
    }
});
