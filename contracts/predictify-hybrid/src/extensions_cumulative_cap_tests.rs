//! Tests for the per-market cumulative extension cap (issue #672).
//!
//! Coverage:
//! - No cap set → extensions are unrestricted by cumulative logic
//! - Cap set → first extension within cap succeeds
//! - Cap set → extension that would exceed cap is rejected with CumulativeExtensionCapHit
//! - Audit event is emitted when the cap rejects an extension
//! - Cumulative total is monotone-increasing across successive extensions
//! - Unauthorized caller cannot change the cap
//! - get_cumulative_extension_total returns correct running total
//! - Exact cap boundary is allowed; one-over is rejected

#![cfg(test)]

use crate::err::Error;
use crate::{PredictifyHybrid, PredictifyHybridClient};
use soroban_sdk::{
    testutils::{Address as _, Events},
    vec, Address, Env, String, Symbol, TryFromVal, Val,
};

// ===== TEST SETUP =====

struct Setup {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl Setup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register(PredictifyHybrid, ());

        let token_admin = Address::generate(&env);
        let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
        let token_id = token_contract.address();

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&Symbol::new(&env, "TokenID"), &token_id);
            crate::circuit_breaker::CircuitBreaker::initialize(&env).unwrap();
        });

        let client = PredictifyHybridClient::new(&env, &contract_id);
        client.initialize(&admin, &None, &None);

        env.as_contract(&contract_id, || {
            crate::circuit_breaker::CircuitBreaker::initialize(&env).unwrap();
        });

        Self { env, contract_id, admin }
    }

    fn create_market(&self, duration_days: u32) -> Symbol {
        use crate::types::{OracleConfig, OracleProvider};
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        let oracle_config = OracleConfig::new(
            OracleProvider::reflector(),
            Address::from_str(
                &self.env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            String::from_str(&self.env, "BTC/USD"),
            5_000_000,
            String::from_str(&self.env, "gt"),
        );
        client.create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC hit 100k?"),
            &vec![
                &self.env,
                String::from_str(&self.env, "Yes"),
                String::from_str(&self.env, "No"),
            ],
            &duration_days,
            &oracle_config,
            &None,
            &86400u64,
            &None,
            &None,
            &None,
        )
    }

    fn extend(
        &self,
        market_id: &Symbol,
        days: u32,
    ) -> Result<Result<(), soroban_sdk::ConversionError>, Result<Error, soroban_sdk::InvokeError>> {
        let client = PredictifyHybridClient::new(&self.env, &self.contract_id);
        client.try_extend_deadline(
            &self.admin,
            market_id,
            &days,
            &String::from_str(&self.env, "test extension"),
        )
    }
}

// ===== TESTS =====

/// When no cumulative cap is configured (default 0), extensions are unrestricted.
#[test]
fn test_no_cap_allows_multiple_extensions() {
    let s = Setup::new();
    let market_id = s.create_market(90);

    assert!(s.extend(&market_id, 5).is_ok());
    assert!(s.extend(&market_id, 5).is_ok());
}

/// After setting a cap, an extension within the cap succeeds.
#[test]
fn test_cap_set_extension_within_cap_succeeds() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let market_id = s.create_market(60);

    client.set_cumulative_extension_cap(&s.admin, &20u32);

    assert!(s.extend(&market_id, 10).is_ok());
}

/// Extending beyond the configured cap is rejected with CumulativeExtensionCapHit.
#[test]
fn test_cap_exceeded_returns_error() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let market_id = s.create_market(60);

    client.set_cumulative_extension_cap(&s.admin, &15u32);

    // First extension: 10 days — within cap
    assert!(s.extend(&market_id, 10).is_ok());

    // Second extension: 6 days — would push total to 16, exceeding 15-day cap
    let result = s.extend(&market_id, 6);
    assert_eq!(result, Err(Ok(Error::CumulativeExtensionCapHit)));
}

/// An audit event is emitted with topic `cum_cap` when the cap rejects an extension.
#[test]
fn test_audit_event_emitted_on_cap_hit() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let market_id = s.create_market(60);

    client.set_cumulative_extension_cap(&s.admin, &10u32);
    assert!(s.extend(&market_id, 7).is_ok());

    // This call hits the cap and should emit the audit event
    let _ = s.extend(&market_id, 5);

    let cap_topic = soroban_sdk::symbol_short!("cum_cap");
    let all = s.env.events().all();
    let found = all.events().iter().any(|e| {
        let body = match &e.body {
            soroban_sdk::xdr::ContractEventBody::V0(v0) => v0,
        };
        body.topics.iter().any(|t| {
            let sym: Option<Symbol> = Symbol::try_from_val(&s.env, t).ok();
            sym.map(|s| s == cap_topic).unwrap_or(false)
        })
    });
    assert!(found, "expected cum_cap audit event to be emitted on cap hit");
}

/// Cumulative total is monotone-increasing: each successful extension increments it.
#[test]
fn test_cumulative_total_is_monotone_increasing() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let market_id = s.create_market(90);

    client.set_cumulative_extension_cap(&s.admin, &30u32);

    let before = client.get_cumulative_extension_total(&market_id);

    assert!(s.extend(&market_id, 5).is_ok());
    let after_first = client.get_cumulative_extension_total(&market_id);

    assert!(s.extend(&market_id, 3).is_ok());
    let after_second = client.get_cumulative_extension_total(&market_id);

    assert!(after_first > before, "total must increase after first extension");
    assert!(after_second > after_first, "total must increase after second extension");
    assert_eq!(after_second - before, 8, "total must equal sum of extended days");
}

/// Non-admin cannot set the cumulative extension cap.
#[test]
fn test_unauthorized_cannot_set_cap() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let rando = Address::generate(&s.env);

    let result = client.try_set_cumulative_extension_cap(&rando, &20u32);
    assert_eq!(result, Err(Ok(Error::Unauthorized)));
}

/// get_cumulative_extension_total returns 0 before any extensions.
#[test]
fn test_cumulative_total_starts_at_zero() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let market_id = s.create_market(30);

    let total = client.get_cumulative_extension_total(&market_id);
    assert_eq!(total, 0);
}

/// Exactly hitting the cap (new_total == cap) is allowed.
#[test]
fn test_extension_exactly_at_cap_succeeds() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let market_id = s.create_market(60);

    client.set_cumulative_extension_cap(&s.admin, &10u32);

    // Exactly 10 days == cap → must succeed
    assert!(s.extend(&market_id, 10).is_ok());
}

/// Once the cap is hit exactly, any further extension is rejected.
#[test]
fn test_extension_beyond_cap_after_exact_hit_rejected() {
    let s = Setup::new();
    let client = PredictifyHybridClient::new(&s.env, &s.contract_id);
    let market_id = s.create_market(90);

    client.set_cumulative_extension_cap(&s.admin, &10u32);
    assert!(s.extend(&market_id, 10).is_ok());

    // Now at cap; 1 more day must be rejected
    let result = s.extend(&market_id, 1);
    assert_eq!(result, Err(Ok(Error::CumulativeExtensionCapHit)));
}
