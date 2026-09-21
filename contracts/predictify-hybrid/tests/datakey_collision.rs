//! DataKey XDR Encoding Collision Detection Test
//!
//! This test verifies that all DataKey enum variants produce unique XDR encodings
//! when serialized by the Soroban environment. Storage keys must have distinct byte
//! representations to avoid collisions in persistent storage across upgrades.

use predictify_hybrid::storage::DataKey;
use soroban_sdk::{
    testutils::Address as _,
    xdr::ToXdr,
    Address, Bytes, BytesN, Env, Symbol,
};

#[test]
fn datakey_xdr_encodings_are_unique() {
    let env = Env::default();
    let addr = Address::generate(&env);
    let sym = Symbol::new(&env, "market_001");
    let bytes32 = BytesN::from_array(&env, &[1u8; 32]);

    let variants: [(&str, DataKey); 32] = [
        ("PlaceBetsIdem", DataKey::PlaceBetsIdem(addr.clone(), bytes32.clone())),
        ("CreateEventIdem", DataKey::CreateEventIdem(addr.clone(), bytes32.clone())),
        ("Whitelisted", DataKey::Whitelisted(addr.clone())),
        ("Blacklisted", DataKey::Blacklisted(addr.clone())),
        ("ArchivedMarket", DataKey::ArchivedMarket(sym.clone(), 1000u64)),
        ("MarketExtensionTotal", DataKey::MarketExtensionTotal(sym.clone())),
        ("MarketMetadata", DataKey::MarketMetadata(sym.clone())),
        ("MarketScratch", DataKey::MarketScratch(sym.clone())),
        ("DisputeHistoryCap", DataKey::DisputeHistoryCap),
        ("DisputeHistory", DataKey::DisputeHistory(sym.clone())),
        ("DisputeStakeCap", DataKey::DisputeStakeCap(sym.clone(), addr.clone())),
        ("DisputeCumulativeStakeCap", DataKey::DisputeCumulativeStakeCap(addr.clone())),
        ("MarketCache", DataKey::MarketCache(sym.clone())),
        ("AntiGriefFloor", DataKey::AntiGriefFloor),
        ("DisputeCooldownSeconds", DataKey::DisputeCooldownSeconds),
        ("DisputeAdminLastAction", DataKey::DisputeAdminLastAction(sym.clone())),
        ("ResolutionCooldownSeconds", DataKey::ResolutionCooldownSeconds),
        ("ResolutionAdminLastAction", DataKey::ResolutionAdminLastAction(sym.clone())),
        ("GlobalConfig", DataKey::GlobalConfig),
        ("MaxBetCap", DataKey::MaxBetCap),
        ("UserStake", DataKey::UserStake(addr.clone(), sym.clone())),
        ("EventNonce", DataKey::EventNonce(sym.clone())),
        ("MarketAuditHead", DataKey::MarketAuditHead(sym.clone())),
        ("MarketAuditLog", DataKey::MarketAuditLog(sym.clone(), 1u32)),
        ("OracleAdminCooldownState", DataKey::OracleAdminCooldownState),
        ("MultisigRotationState", DataKey::MultisigRotationState),
        ("AdminOverrideNonce", DataKey::AdminOverrideNonce),
        ("PerLedgerBetCap", DataKey::PerLedgerBetCap),
        ("PerLedgerBetCounter", DataKey::PerLedgerBetCounter),
        ("CollusionDetectorConfig", DataKey::CollusionDetectorConfig(sym.clone())),
        ("ClaimNonce", DataKey::ClaimNonce(addr.clone(), sym.clone())),
        ("EventTopicAlias", DataKey::EventTopicAlias(sym.clone())),
    ];

    let mut encoded_variants: Vec<(&str, Bytes)> = Vec::new();
    for (name, key) in variants.iter() {
        let xdr_bytes = key.to_xdr(&env);
        encoded_variants.push((*name, xdr_bytes));
    }

    let variant_count = encoded_variants.len();
    assert_eq!(variant_count, 32);

    for i in 0..variant_count {
        for j in (i + 1)..variant_count {
            let (name_i, bytes_i) = &encoded_variants[i];
            let (name_j, bytes_j) = &encoded_variants[j];

            assert_ne!(
                bytes_i, bytes_j,
                "DataKey collision detected: variant {} and variant {} have identical XDR encodings!",
                name_i, name_j
            );
        }
    }
}
