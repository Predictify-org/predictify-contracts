#[cfg(test)]
mod tests {
    use soroban_sdk::{Env, String, Map};
    use crate::utils::{TimeUtils, NumericUtils, ConversionUtils};

    #[test]
    fn test_time_utils_format_duration_table() {
        let env = Env::default();
        let test_cases = [
            (86400 + 3600 + 60, "1d 1h 1m"),
            (3600 + 120, "1h 2m"),
            (120, "2m"),
            (0, "0m"),
            (86400 * 5, "5d 0h 0m"),
        ];

        for (seconds, expected) in test_cases {
            assert_eq!(
                TimeUtils::format_duration(&env, seconds),
                String::from_str(&env, expected)
            );
        }
    }

    #[test]
    fn test_numeric_utils_sqrt_table() {
        let test_cases = [
            (0, 0),
            (1, 1),
            (4, 2),
            (10, 3),
            (100, 10),
            (1000, 31),
            (-5, 0),
        ];

        for (input, expected) in test_cases {
            assert_eq!(NumericUtils::sqrt(&input), expected, "Sqrt failed for {}", input);
        }
    }

    #[test]
    fn test_numeric_utils_weighted_average_table() {
        let env = Env::default();
        
        // Success case: (10*1 + 20*2) / 3 = 50/3 = 16
        let mut vals = soroban_sdk::vec![&env, 10, 20];
        let mut weights = soroban_sdk::vec![&env, 1, 2];
        assert_eq!(NumericUtils::weighted_average(&vals, &weights), 16);

        // Zero weight case
        weights = soroban_sdk::vec![&env, 0, 0];
        assert_eq!(NumericUtils::weighted_average(&vals, &weights), 0);

        // Mismatch length case
        vals = soroban_sdk::vec![&env, 10];
        assert_eq!(NumericUtils::weighted_average(&vals, &weights), 0);

        // Empty case
        vals = soroban_sdk::vec![&env];
        weights = soroban_sdk::vec![&env];
        assert_eq!(NumericUtils::weighted_average(&vals, &weights), 0);
    }

    #[test]
    fn test_conversion_utils_maps_equal_table() {
        let env = Env::default();
        
        let mut m1 = Map::new(&env);
        let mut m2 = Map::new(&env);
        
        let k1 = String::from_str(&env, "k1");
        let v1 = String::from_str(&env, "v1");
        let v2 = String::from_str(&env, "v2");

        // Empty maps
        assert!(ConversionUtils::maps_equal(&m1, &m2));

        // Mismatch length
        m1.set(k1.clone(), v1.clone());
        assert!(!ConversionUtils::maps_equal(&m1, &m2));

        // Identical
        m2.set(k1.clone(), v1.clone());
        assert!(ConversionUtils::maps_equal(&m1, &m2));

        // Value mismatch
        m2.set(k1.clone(), v2.clone());
        assert!(!ConversionUtils::maps_equal(&m1, &m2));

        // Key mismatch (m1 has k1, m3 has k2)
        let mut m3 = Map::new(&env);
        m3.set(String::from_str(&env, "k2"), v1.clone());
        assert!(!ConversionUtils::maps_equal(&m1, &m3));
    }

    #[test]
    fn test_math_helpers_table() {
        // BPS Table
        let bps_cases = [
            (10000, 250, 250),
            (0, 500, 0),
            (-100, 500, 0),
            (1000000, 1, 100),
        ];
        for (amt, bps, exp) in bps_cases {
            assert_eq!(NumericUtils::calculate_bps(amt, bps), exp);
        }

        // Payout Share Table
        let share_cases = [
            (1000, 500, 1000, 500),
            (1000, 500, 0, 0),
            (0, 500, 1000, 0),
        ];
        for (pool, stake, total, exp) in share_cases {
            assert_eq!(NumericUtils::calculate_payout_share(pool, stake, total), exp);
        }
    }

    #[test]
    fn test_numeric_logic_branches() {
        assert_eq!(NumericUtils::clamp(&50, &10, &100), 50);
        assert_eq!(NumericUtils::clamp(&5, &10, &100), 10);
        assert_eq!(NumericUtils::clamp(&150, &10, &100), 100);

        assert!(NumericUtils::is_within_range(&50, &10, &100));
        assert!(!NumericUtils::is_within_range(&5, &10, &100));

        assert_eq!(NumericUtils::abs_difference(&10, &30), 20);
        assert_eq!(NumericUtils::abs_difference(&30, &10), 20);
        
        assert_eq!(NumericUtils::round_to_nearest(&123, &10), 120);
    }
}