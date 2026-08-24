#[cfg(test)]
mod tests {
    use soroban_sdk::{Env, String, Symbol};
    use crate::types::{ReflectorAsset, OracleProvider, MarketStatus, MarketState};
    use crate::admin::{AdminRole, AdminHierarchy, AdminUtils, AdminPermission};
    use crate::utils::{calculate_bps, calculate_payout_share};

    #[test]
    fn test_reflector_asset_properties_table() {
        let env = Env::default();
        
        struct TestCase {
            asset: ReflectorAsset,
            expected_symbol: &'static str,
            expected_name: &'static str,
            expected_decimals: u8,
            expected_feed_id: &'static str,
            is_supported: bool,
            is_xlm: bool,
        }

        let test_cases = [
            TestCase {
                asset: ReflectorAsset::Stellar,
                expected_symbol: "XLM",
                expected_name: "Stellar Lumens",
                expected_decimals: 7,
                expected_feed_id: "XLM/USD",
                is_supported: true,
                is_xlm: true,
            },
            TestCase {
                asset: ReflectorAsset::BTC,
                expected_symbol: "BTC",
                expected_name: "Bitcoin",
                expected_decimals: 8,
                expected_feed_id: "BTC/USD",
                is_supported: true,
                is_xlm: false,
            },
            TestCase {
                asset: ReflectorAsset::ETH,
                expected_symbol: "ETH",
                expected_name: "Ethereum",
                expected_decimals: 18,
                expected_feed_id: "ETH/USD",
                is_supported: true,
                is_xlm: false,
            },
            TestCase {
                asset: ReflectorAsset::Other(Symbol::new(&env, "GOLD")),
                expected_symbol: "GOLD",
                expected_name: "Custom Asset (GOLD)",
                expected_decimals: 7,
                expected_feed_id: "GOLD/USD",
                is_supported: false,
                is_xlm: false,
            },
        ];

        for case in test_cases {
            assert_eq!(case.asset.symbol(), String::from_str(&env, case.expected_symbol));
            assert_eq!(case.asset.name(), String::from_str(&env, case.expected_name));
            assert_eq!(case.asset.decimals(), case.expected_decimals);
            assert_eq!(case.asset.feed_id(), String::from_str(&env, case.expected_feed_id));
            assert_eq!(case.asset.is_supported(), case.is_supported);
            assert_eq!(case.asset.is_xlm(), case.is_xlm);
        }
    }

    #[test]
    fn test_admin_hierarchy_logic_table() {
        struct TestCase {
            manager: AdminRole,
            target: AdminRole,
            expected_can_manage: bool,
        }

        let test_cases = [
            TestCase { manager: AdminRole::SuperAdmin, target: AdminRole::SuperAdmin, expected_can_manage: true },
            TestCase { manager: AdminRole::SuperAdmin, target: AdminRole::MarketAdmin, expected_can_manage: true },
            TestCase { manager: AdminRole::MarketAdmin, target: AdminRole::ReadOnlyAdmin, expected_can_manage: true },
            TestCase { manager: AdminRole::MarketAdmin, target: AdminRole::SuperAdmin, expected_can_manage: false },
            TestCase { manager: AdminRole::MarketAdmin, target: AdminRole::ConfigAdmin, expected_can_manage: false },
            TestCase { manager: AdminRole::ReadOnlyAdmin, target: AdminRole::MarketAdmin, expected_can_manage: false },
        ];

        for case in test_cases {
            assert_eq!(
                AdminHierarchy::can_manage_role(&case.manager, &case.target),
                case.expected_can_manage,
                "Manager {:?} vs Target {:?}", case.manager, case.target
            );
        }
    }

    #[test]
    fn test_oracle_provider_names_table() {
        let env = Env::default();
        
        let test_cases = [
            (OracleProvider::reflector(), "Reflector", "reflector", true),
            (OracleProvider::pyth(), "Pyth Network", "pyth", false),
            (OracleProvider::band_protocol(), "Band Protocol", "band_protocol", false),
            (OracleProvider::dia(), "DIA", "dia", false),
        ];

        for (provider, expected_name, expected_str, supported) in test_cases {
            assert_eq!(provider.name(), String::from_str(&env, expected_name));
            assert_eq!(provider.as_str(), expected_str);
            assert_eq!(provider.is_supported(), supported);
        }
    }

    #[test]
    fn test_math_utils_table() {
        // Table for calculate_bps
        let bps_cases = [
            (10000, 250, 250),   // 2.5% of 10000
            (100, 1000, 10),     // 10% of 100
            (0, 500, 0),         // 5% of 0
            (1000000, 1, 100),   // 0.01% of 1M
            (-100, 500, 0),      // Negative amount check
        ];

        for (amt, bps, expected) in bps_cases {
            assert_eq!(calculate_bps(amt, bps), expected, "BPS failed for amt: {}", amt);
        }

        // Table for calculate_payout_share
        let share_cases = [
            (1000, 500, 1000, 500),  // 50% share of 1000
            (1000, 100, 1000, 100),  // 10% share of 1000
            (1000, 0, 1000, 0),      // 0 stake
            (1000, 500, 0, 0),       // 0 total winners
            (0, 500, 1000, 0),       // 0 pool
        ];

        for (pool, stake, total, expected) in share_cases {
            assert_eq!(calculate_payout_share(pool, stake, total), expected, "Share failed for stake: {}", stake);
        }
    }

    #[test]
    fn test_market_status_conversion() {
        let test_cases = [
            (MarketState::Active, MarketStatus::Active),
            (MarketState::Resolved, MarketStatus::Resolved),
            (MarketState::Cancelled, MarketStatus::Cancelled),
        ];
        for (state, expected_status) in test_cases {
            assert_eq!(MarketStatus::from_market_state(state), expected_status);
        }
    }
}