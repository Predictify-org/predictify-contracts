#![cfg(test)]

use crate::err::Error;
use crate::oracles::OracleValidationConfigManager;
use crate::resolution::{MarketResolutionManager, OracleResolutionManager};
use crate::types::{
    GlobalOracleValidationConfig, MedianOracleConfig, OraclePriceData, OracleProvider,
    OracleQuote,
};
use crate::PredictifyHybrid;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{symbol_short, Address, Env, String, Symbol, Vec};

struct TestFixture {
    env: Env,
    contract_id: Address,
}

impl TestFixture {
    fn new() -> Self {
        let env = Env::default();
        let contract_id = env.register_contract(None, PredictifyHybrid);
        Self { env, contract_id }
    }
}

fn build_quote(
    provider: OracleProvider,
    price: i128,
    confidence_bps: u32,
    weight_bps: u32,
    included: bool,
) -> OracleQuote {
    OracleQuote {
        provider,
        price,
        confidence_bps,
        weight_bps,
        included,
    }
}

#[test]
fn test_stale_feed_cannot_settle() {
    let fixture = TestFixture::new();
    let env = &fixture.env;
    let market_id = Symbol::new(env, "stale_test");

    fixture.env.as_contract(&fixture.contract_id, || {
        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });

        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 60,
            max_confidence_bps: 500,
            max_deviation_bps: None,
            max_deviation_z_multiple: None,
            history_size: None,
            auto_pause_duration_secs: None,
        };
        OracleValidationConfigManager::set_global_config(env, &config).unwrap();

        let stale_data = OraclePriceData {
            price: 50_000,
            publish_time: 900,
            confidence: None,
            exponent: 0,
        };

        let result = OracleValidationConfigManager::validate_oracle_data(
            env,
            &market_id,
            &OracleProvider::reflector(),
            &String::from_str(env, "BTC/USD"),
            &stale_data,
        );

        assert_eq!(result.unwrap_err(), Error::OracleStale);
    });
}

#[test]
fn test_future_timestamp_rejected() {
    let fixture = TestFixture::new();
    let env = &fixture.env;
    let market_id = Symbol::new(env, "future_test");

    fixture.env.as_contract(&fixture.contract_id, || {
        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });

        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 60,
            max_confidence_bps: 500,
            max_deviation_bps: None,
            max_deviation_z_multiple: None,
            history_size: None,
            auto_pause_duration_secs: None,
        };
        OracleValidationConfigManager::set_global_config(env, &config).unwrap();

        let future_data = OraclePriceData {
            price: 50_000,
            publish_time: 1050,
            confidence: None,
            exponent: 0,
        };

        let result = OracleValidationConfigManager::validate_oracle_data(
            env,
            &market_id,
            &OracleProvider::reflector(),
            &String::from_str(env, "BTC/USD"),
            &future_data,
        );

        assert_eq!(result.unwrap_err(), Error::OracleStale);
    });
}

#[test]
fn test_non_positive_price_feed_rejected() {
    let fixture = TestFixture::new();
    let env = &fixture.env;
    let market_id = Symbol::new(env, "sign_test");

    fixture.env.as_contract(&fixture.contract_id, || {
        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });

        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 60,
            max_confidence_bps: 500,
            max_deviation_bps: None,
            max_deviation_z_multiple: None,
            history_size: None,
            auto_pause_duration_secs: None,
        };
        OracleValidationConfigManager::set_global_config(env, &config).unwrap();

        let zero_price_data = OraclePriceData {
            price: 0,
            publish_time: 980,
            confidence: None,
            exponent: 0,
        };

        let result_zero = OracleValidationConfigManager::validate_oracle_data(
            env,
            &market_id,
            &OracleProvider::reflector(),
            &String::from_str(env, "BTC/USD"),
            &zero_price_data,
        );
        assert_eq!(result_zero.unwrap_err(), Error::InvalidInput);

        let neg_price_data = OraclePriceData {
            price: -100,
            publish_time: 980,
            confidence: None,
            exponent: 0,
        };

        let result_neg = OracleValidationConfigManager::validate_oracle_data(
            env,
            &market_id,
            &OracleProvider::reflector(),
            &String::from_str(env, "BTC/USD"),
            &neg_price_data,
        );
        assert_eq!(result_neg.unwrap_err(), Error::InvalidInput);
    });
}

#[test]
fn test_valid_fresh_feed_accepted() {
    let fixture = TestFixture::new();
    let env = &fixture.env;
    let market_id = Symbol::new(env, "valid_feed");

    fixture.env.as_contract(&fixture.contract_id, || {
        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });

        let config = GlobalOracleValidationConfig {
            max_staleness_secs: 60,
            max_confidence_bps: 500,
            max_deviation_bps: None,
            max_deviation_z_multiple: None,
            history_size: None,
            auto_pause_duration_secs: None,
        };
        OracleValidationConfigManager::set_global_config(env, &config).unwrap();

        let valid_data = OraclePriceData {
            price: 52_000,
            publish_time: 980,
            confidence: None,
            exponent: 0,
        };

        let result = OracleValidationConfigManager::validate_oracle_data(
            env,
            &market_id,
            &OracleProvider::reflector(),
            &String::from_str(env, "BTC/USD"),
            &valid_data,
        );
        assert!(result.is_ok());
    });
}

#[test]
fn test_quorum_boundaries_insufficient_sources() {
    let fixture = TestFixture::new();
    let env = &fixture.env;

    let mut quotes: Vec<OracleQuote> = Vec::new(env);
    quotes.push_back(build_quote(OracleProvider::pyth(), 50_000, 100, 5_000, true));
    quotes.push_back(build_quote(OracleProvider::reflector(), 50_100, 100, 5_000, true));

    let min_sources: u32 = 3;
    let prices = MarketResolutionManager::collect_included_sorted(env, &quotes);
    assert_eq!(prices.len(), 2);
    assert!((prices.len() as u32) < min_sources);
}

#[test]
fn test_quorum_boundaries_exact_quorum() {
    let fixture = TestFixture::new();
    let env = &fixture.env;

    let mut quotes: Vec<OracleQuote> = Vec::new(env);
    quotes.push_back(build_quote(OracleProvider::pyth(), 50_000, 100, 3_333, true));
    quotes.push_back(build_quote(OracleProvider::reflector(), 50_100, 100, 3_334, true));
    quotes.push_back(build_quote(OracleProvider::band_protocol(), 50_200, 100, 3_333, true));

    let min_sources: u32 = 3;
    let prices = MarketResolutionManager::collect_included_sorted(env, &quotes);
    assert_eq!(prices.len() as u32, min_sources);

    let median = MarketResolutionManager::weighted_median(&quotes).unwrap();
    assert_eq!(median, 50_100);
}

#[test]
fn test_outlier_filtering_breaks_quorum() {
    let fixture = TestFixture::new();
    let env = &fixture.env;

    let mut quotes: Vec<OracleQuote> = Vec::new(env);
    quotes.push_back(build_quote(OracleProvider::pyth(), 100, 100, 5_000, true));
    quotes.push_back(build_quote(OracleProvider::reflector(), 102, 100, 5_000, true));
    quotes.push_back(build_quote(OracleProvider::band_protocol(), 500, 100, 5_000, true));

    let min_sources: u32 = 3;
    let max_dev_bps: u32 = 500;

    let baseline_prices = MarketResolutionManager::collect_included_sorted(env, &quotes);
    let baseline_median = MarketResolutionManager::simple_median(&baseline_prices);
    assert_eq!(baseline_median, 102);

    let mut filtered_quotes: Vec<OracleQuote> = Vec::new(env);
    let mut included_count: u32 = 0;
    for q in quotes.iter() {
        let mut out = q.clone();
        if out.price <= 0 {
            out.included = false;
        } else if out.included && baseline_median > 0 {
            let abs_diff = if out.price > baseline_median {
                out.price.saturating_sub(baseline_median)
            } else {
                baseline_median.saturating_sub(out.price)
            };
            let dev_bps = (abs_diff as u64)
                .saturating_mul(10_000)
                .saturating_div(baseline_median as u64);
            if dev_bps > max_dev_bps as u64 {
                out.included = false;
            }
        }
        if out.included {
            included_count += 1;
        }
        filtered_quotes.push_back(out);
    }

    assert_eq!(included_count, 2);
    assert!(included_count < min_sources);
}

#[test]
fn test_non_positive_quotes_excluded_from_quorum() {
    let fixture = TestFixture::new();
    let env = &fixture.env;

    let mut quotes: Vec<OracleQuote> = Vec::new(env);
    quotes.push_back(build_quote(OracleProvider::pyth(), 100, 100, 5_000, true));
    quotes.push_back(build_quote(OracleProvider::reflector(), 102, 100, 5_000, true));
    quotes.push_back(build_quote(OracleProvider::band_protocol(), 0, 100, 5_000, true));
    quotes.push_back(build_quote(OracleProvider::band_protocol(), -50, 100, 5_000, true));

    let positive_prices = MarketResolutionManager::collect_included_sorted(env, &quotes);
    assert_eq!(positive_prices.len(), 2);
    assert_eq!(positive_prices.get(0).unwrap(), 100);
    assert_eq!(positive_prices.get(1).unwrap(), 102);

    let median = MarketResolutionManager::weighted_median(&quotes).unwrap();
    assert_eq!(median, 100);
}

#[test]
fn test_canonical_price_atomic_persistence() {
    let fixture = TestFixture::new();
    let env = &fixture.env;
    let market_id = Symbol::new(env, "canon_test");

    fixture.env.as_contract(&fixture.contract_id, || {
        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });

        assert_eq!(OracleResolutionManager::get_canonical_price(env, &market_id), None);
        assert_eq!(OracleResolutionManager::get_canonical_price_record(env, &market_id), None);

        let outcome = String::from_str(env, "yes");
        OracleResolutionManager::store_canonical_price(env, &market_id, 55_000, 1000, &outcome);

        let price = OracleResolutionManager::get_canonical_price(env, &market_id);
        assert_eq!(price, Some(55_000));

        let record = OracleResolutionManager::get_canonical_price_record(env, &market_id).unwrap();
        assert_eq!(record.price, 55_000);
        assert_eq!(record.timestamp, 1000);
        assert_eq!(record.outcome, outcome);

        let val_res = OracleResolutionManager::validate_canonical_price_before_settlement(env, &market_id);
        assert!(val_res.is_ok());
    });
}

#[test]
fn test_outage_missing_canonical_price_fails_closed() {
    let fixture = TestFixture::new();
    let env = &fixture.env;
    let market_id = Symbol::new(env, "outage_missing");

    fixture.env.as_contract(&fixture.contract_id, || {
        let result = OracleResolutionManager::validate_canonical_price_before_settlement(env, &market_id);
        assert_eq!(result.unwrap_err(), Error::OracleUnavailable);
    });
}

#[test]
fn test_outage_non_positive_canonical_price_fails_closed() {
    let fixture = TestFixture::new();
    let env = &fixture.env;
    let market_id = Symbol::new(env, "outage_zero");

    fixture.env.as_contract(&fixture.contract_id, || {
        let outcome = String::from_str(env, "no");
        OracleResolutionManager::store_canonical_price(env, &market_id, 0, 1000, &outcome);

        let result = OracleResolutionManager::validate_canonical_price_before_settlement(env, &market_id);
        assert_eq!(result.unwrap_err(), Error::InvalidInput);
    });
}

#[test]
fn test_outage_recovery_path() {
    let fixture = TestFixture::new();
    let env = &fixture.env;
    let market_id = Symbol::new(env, "recovery_mkt");

    fixture.env.as_contract(&fixture.contract_id, || {
        let failure_before = OracleResolutionManager::validate_canonical_price_before_settlement(env, &market_id);
        assert_eq!(failure_before.unwrap_err(), Error::OracleUnavailable);

        let outcome = String::from_str(env, "yes");
        OracleResolutionManager::store_canonical_price(env, &market_id, 62_000, 1000, &outcome);

        env.ledger().with_mut(|li| {
            li.timestamp = 1000;
        });

        let success_after = OracleResolutionManager::validate_canonical_price_before_settlement(env, &market_id);
        assert!(success_after.is_ok());
        assert_eq!(OracleResolutionManager::get_canonical_price(env, &market_id), Some(62_000));
    });
}

#[test]
fn test_set_median_config_boundaries() {
    let fixture = TestFixture::new();
    let env = &fixture.env;

    fixture.env.as_contract(&fixture.contract_id, || {
        let dummy_addr = Address::generate(env);

        let zero_sources_cfg = MedianOracleConfig {
            pyth_address: dummy_addr.clone(),
            reflector_address: dummy_addr.clone(),
            band_address: dummy_addr.clone(),
            min_sources: 0,
            max_deviation_bps: 500,
        };
        let err_zero = OracleResolutionManager::set_median_config(env, &zero_sources_cfg);
        assert_eq!(err_zero.unwrap_err(), Error::InvalidOracleConfig);

        let excess_deviation_cfg = MedianOracleConfig {
            pyth_address: dummy_addr.clone(),
            reflector_address: dummy_addr.clone(),
            band_address: dummy_addr.clone(),
            min_sources: 2,
            max_deviation_bps: 10_001,
        };
        let err_dev = OracleResolutionManager::set_median_config(env, &excess_deviation_cfg);
        assert_eq!(err_dev.unwrap_err(), Error::InvalidOracleConfig);

        let valid_cfg = MedianOracleConfig {
            pyth_address: dummy_addr.clone(),
            reflector_address: dummy_addr.clone(),
            band_address: dummy_addr.clone(),
            min_sources: 2,
            max_deviation_bps: 500,
        };
        let ok_res = OracleResolutionManager::set_median_config(env, &valid_cfg);
        assert!(ok_res.is_ok());

        let retrieved = OracleResolutionManager::get_median_config(env).unwrap();
        assert_eq!(retrieved.min_sources, 2);
        assert_eq!(retrieved.max_deviation_bps, 500);
    });
}
