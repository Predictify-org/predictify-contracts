//! DataKey XDR Encoding Collision Detection Test
//!
//! This test verifies that all DataKey enum variants produce unique XDR encodings
//! when serialized by the Soroban environment. This is critical because storage keys
//! must have distinct byte representations to avoid collisions in persistent storage.
//!
//! The test constructs all DataKey variants with placeholder values, encodes each
//! to XDR, and performs pairwise comparison to detect any collisions.
//!
//! If a collision is detected, the test panics with a detailed error message.
//! If all encodings are unique, the test succeeds and reports the count of variants checked.

use predictify_hybrid::storage::DataKey;
use soroban_sdk::{Address, Bytes, Env, IntoVal, Symbol};

#[test]
fn datakey_xdr_encodings_are_unique() {
    // Create a test environment for serialization operations
    let env = Env::new();

    // Step 1: Construct all DataKey variants with dummy/placeholder values
    
    // Variant 1: Whitelisted(Address)
    // Create a dummy address from a contract ID
    let dummy_contract_bytes = [1u8; 32];
    let dummy_address = Address::from_contract_id(&env, &soroban_sdk::BytesN::from_array(&env, &dummy_contract_bytes));
    let datakey_whitelisted = DataKey::Whitelisted(dummy_address.clone());

    // Variant 2: Blacklisted(Address)
    // Use a different address to ensure proper differentiation
    let blacklist_contract_bytes = [2u8; 32];
    let blacklist_address = Address::from_contract_id(&env, &soroban_sdk::BytesN::from_array(&env, &blacklist_contract_bytes));
    let datakey_blacklisted = DataKey::Blacklisted(blacklist_address.clone());

    // Variant 3: ArchivedMarket(Symbol, u64)
    // Use placeholder symbol and timestamp
    let archive_symbol = Symbol::new(&env, "market_001");
    let archive_timestamp = 1000u64;
    let datakey_archived = DataKey::ArchivedMarket(archive_symbol, archive_timestamp);

    // Collect all variants into a vector for pairwise comparison
    let variants: Vec<(&str, DataKey)> = vec![
        ("Whitelisted", datakey_whitelisted),
        ("Blacklisted", datakey_blacklisted),
        ("ArchivedMarket", datakey_archived),
    ];

    // Step 2: Encode each variant to XDR bytes
    let mut encoded_variants: Vec<(String, Bytes)> = Vec::new();

    for (name, key) in variants.iter() {
        // Encode the DataKey variant to XDR format
        let xdr_bytes = env.to_xdr(key)
            .expect("failed to encode DataKey variant to XDR");
        encoded_variants.push((name.to_string(), xdr_bytes));
    }

    // Step 3: Perform pairwise comparison of all encodings (O(n²))
    let variant_count = encoded_variants.len();
    
    for i in 0..variant_count {
        for j in (i + 1)..variant_count {
            let (name_i, bytes_i) = &encoded_variants[i];
            let (name_j, bytes_j) = &encoded_variants[j];

            // Step 4: Check for identical encodings
            if bytes_i == bytes_j {
                // If collision detected, panic with clear error message
                panic!(
                    "DataKey collision detected: variant {} and variant {} have identical XDR encodings!\n\
                     Encoding (hex): {}\n\
                     This indicates a critical storage key collision that must be resolved.",
                    name_i,
                    name_j,
                    hex_encode(bytes_i)
                );
            }
        }
    }

    // Step 5: All encodings are unique — report success
    println!(
        "✓ All {} DataKey variants have unique XDR encodings (no collisions detected)",
        variant_count
    );
    
    // Print each variant's encoding for reference (informational)
    for (name, bytes) in encoded_variants.iter() {
        println!("  - {}: {} bytes", name, bytes.len());
    }
}

/// Helper function to convert bytes to hexadecimal string representation
fn hex_encode(bytes: &Bytes) -> String {
    let mut hex_string = String::new();
    for byte in bytes.iter() {
        hex_string.push_str(&format!("{:02x}", byte));
    }
    hex_string
}
