use soroban_sdk::{contracttype, vec, Address, Env, Map, String, Symbol, Vec};

use crate::errors::Error;

/// Comprehensive utility function system for Predictify Hybrid contract
///
/// This module provides a centralized collection of utility functions with:
/// - Time and date manipulation utilities
/// - String manipulation and formatting utilities
/// - Numeric calculation helpers
/// - Validation utility functions
/// - Conversion utility functions
/// - Testing utility functions
/// - Common helper functions for contract operations

// ===== TIME AND DATE UTILITIES =====

/// Time and date utility functions
pub struct TimeUtils;

impl TimeUtils {
    /// Convert days to seconds
    pub fn days_to_seconds(days: u32) -> u64 {
        days as u64 * 24 * 60 * 60
    }

    /// Convert hours to seconds
    pub fn hours_to_seconds(hours: u32) -> u64 {
        hours as u64 * 60 * 60
    }

    /// Convert minutes to seconds
    pub fn minutes_to_seconds(minutes: u32) -> u64 {
        minutes as u64 * 60
    }

    /// Calculate time difference in seconds
    pub fn time_difference(current_time: u64, past_time: u64) -> u64 {
        if current_time >= past_time {
            current_time - past_time
        } else {
            0
        }
    }

    /// Check if a timestamp is in the future
    pub fn is_future_timestamp(timestamp: u64, current_time: u64) -> bool {
        timestamp > current_time
    }

    /// Check if a timestamp is in the past
    pub fn is_past_timestamp(timestamp: u64, current_time: u64) -> bool {
        timestamp < current_time
    }

    /// Check if a timestamp is expired (older than max_age)
    pub fn is_timestamp_expired(timestamp: u64, current_time: u64, max_age: u64) -> bool {
        TimeUtils::time_difference(current_time, timestamp) > max_age
    }

    /// Calculate end time from start time and duration
    pub fn calculate_end_time(start_time: u64, duration_seconds: u64) -> u64 {
        start_time + duration_seconds
    }

    /// Calculate remaining time until end
    pub fn calculate_remaining_time(end_time: u64, current_time: u64) -> u64 {
        if end_time > current_time {
            end_time - current_time
        } else {
            0
        }
    }

    /// Format duration in human-readable format
    pub fn format_duration(seconds: u64) -> String {
        let days = seconds / (24 * 60 * 60);
        let hours = (seconds % (24 * 60 * 60)) / (60 * 60);
        let minutes = (seconds % (60 * 60)) / 60;
        
        if days > 0 {
            String::from_str(&Env::default(), &format!("{}d {}h {}m", days, hours, minutes))
        } else if hours > 0 {
            String::from_str(&Env::default(), &format!("{}h {}m", hours, minutes))
        } else {
            String::from_str(&Env::default(), &format!("{}m", minutes))
        }
    }
}

// ===== STRING UTILITIES =====

/// String manipulation utility functions
pub struct StringUtils;

impl StringUtils {
    /// Check if string is empty or whitespace only
    pub fn is_empty_or_whitespace(s: &String) -> bool {
        s.len() == 0 || s.trim().len() == 0
    }

    /// Truncate string to maximum length
    pub fn truncate_string(s: &String, max_length: usize) -> String {
        if s.len() <= max_length {
            s.clone()
        } else {
            let truncated = s.to_string().chars().take(max_length).collect::<String>();
            String::from_str(&s.env(), &truncated)
        }
    }

    /// Convert string to lowercase
    pub fn to_lowercase(s: &String) -> String {
        let lower = s.to_string().to_lowercase();
        String::from_str(&s.env(), &lower)
    }

    /// Convert string to uppercase
    pub fn to_uppercase(s: &String) -> String {
        let upper = s.to_string().to_uppercase();
        String::from_str(&s.env(), &upper)
    }

    /// Check if string contains substring
    pub fn contains_substring(s: &String, substring: &str) -> bool {
        s.to_string().contains(substring)
    }

    /// Replace substring in string
    pub fn replace_substring(s: &String, old: &str, new: &str) -> String {
        let replaced = s.to_string().replace(old, new);
        String::from_str(&s.env(), &replaced)
    }

    /// Split string by delimiter
    pub fn split_string(s: &String, delimiter: &str) -> Vec<String> {
        let parts: Vec<&str> = s.to_string().split(delimiter).collect();
        let mut result = Vec::new(&s.env());
        for part in parts {
            result.push_back(String::from_str(&s.env(), part));
        }
        result
    }

    /// Join strings with delimiter
    pub fn join_strings(strings: &Vec<String>, delimiter: &str) -> String {
        if strings.len() == 0 {
            return String::from_str(&strings.env(), "");
        }
        
        let mut result = strings.get(0).unwrap().to_string();
        for i in 1..strings.len() {
            result.push_str(delimiter);
            result.push_str(&strings.get(i).unwrap().to_string());
        }
        String::from_str(&strings.env(), &result)
    }

    /// Validate string length
    pub fn validate_string_length(s: &String, min_length: usize, max_length: usize) -> Result<(), Error> {
        let len = s.len();
        if len < min_length {
            return Err(Error::InvalidInput);
        }
        if len > max_length {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Sanitize string (remove special characters)
    pub fn sanitize_string(s: &String) -> String {
        let sanitized = s.to_string()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '_' || *c == '-')
            .collect::<String>();
        String::from_str(&s.env(), &sanitized)
    }
}

// ===== NUMERIC UTILITIES =====

/// Numeric calculation utility functions
pub struct NumericUtils;

impl NumericUtils {
    /// Calculate percentage
    pub fn calculate_percentage(part: i128, total: i128) -> i128 {
        if total == 0 {
            return 0;
        }
        (part * 100) / total
    }

    /// Calculate percentage with custom denominator
    pub fn calculate_percentage_with_denominator(part: i128, total: i128, denominator: i128) -> i128 {
        if total == 0 {
            return 0;
        }
        (part * denominator) / total
    }

    /// Calculate weighted average
    pub fn calculate_weighted_average(values: &Vec<i128>, weights: &Vec<i128>) -> i128 {
        if values.len() != weights.len() || values.len() == 0 {
            return 0;
        }
        
        let mut weighted_sum = 0i128;
        let mut total_weight = 0i128;
        
        for i in 0..values.len() {
            let value = values.get(i).unwrap();
            let weight = weights.get(i).unwrap();
            weighted_sum += value * weight;
            total_weight += weight;
        }
        
        if total_weight == 0 {
            return 0;
        }
        
        weighted_sum / total_weight
    }

    /// Calculate compound interest
    pub fn calculate_compound_interest(principal: i128, rate_percentage: i128, periods: u32) -> i128 {
        if rate_percentage == 0 || periods == 0 {
            return principal;
        }
        
        let rate_decimal = rate_percentage as f64 / 100.0;
        let result = principal as f64 * (1.0 + rate_decimal).powi(periods as i32);
        result as i128
    }

    /// Calculate simple interest
    pub fn calculate_simple_interest(principal: i128, rate_percentage: i128, periods: u32) -> i128 {
        if rate_percentage == 0 || periods == 0 {
            return principal;
        }
        
        let interest = (principal * rate_percentage as i128 * periods as i128) / 100;
        principal + interest
    }

    /// Round to nearest multiple
    pub fn round_to_nearest(value: i128, multiple: i128) -> i128 {
        if multiple == 0 {
            return value;
        }
        ((value + multiple / 2) / multiple) * multiple
    }

    /// Calculate minimum of two values
    pub fn min(a: i128, b: i128) -> i128 {
        if a < b { a } else { b }
    }

    /// Calculate maximum of two values
    pub fn max(a: i128, b: i128) -> i128 {
        if a > b { a } else { b }
    }

    /// Clamp value between min and max
    pub fn clamp(value: i128, min: i128, max: i128) -> i128 {
        NumericUtils::max(min, NumericUtils::min(value, max))
    }

    /// Check if value is within range
    pub fn is_within_range(value: i128, min: i128, max: i128) -> bool {
        value >= min && value <= max
    }

    /// Calculate absolute difference
    pub fn abs_difference(a: i128, b: i128) -> i128 {
        if a > b { a - b } else { b - a }
    }

    /// Calculate square root (integer approximation)
    pub fn sqrt(value: i128) -> i128 {
        if value <= 0 {
            return 0;
        }
        
        let mut x = value;
        let mut y = (x + 1) / 2;
        while y < x {
            x = y;
            y = (x + value / x) / 2;
        }
        x
    }
}

// ===== VALIDATION UTILITIES =====

/// Validation utility functions
pub struct ValidationUtils;

impl ValidationUtils {
    /// Validate address is not null/empty
    pub fn validate_address(address: &Address) -> Result<(), Error> {
        // In Soroban, Address is always valid if it exists
        Ok(())
    }

    /// Validate symbol is not empty
    pub fn validate_symbol(symbol: &Symbol) -> Result<(), Error> {
        if symbol.to_string().is_empty() {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Validate string is not empty
    pub fn validate_non_empty_string(s: &String) -> Result<(), Error> {
        if StringUtils::is_empty_or_whitespace(s) {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Validate positive number
    pub fn validate_positive_number(value: i128) -> Result<(), Error> {
        if value <= 0 {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Validate non-negative number
    pub fn validate_non_negative_number(value: i128) -> Result<(), Error> {
        if value < 0 {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Validate number is within range
    pub fn validate_number_range(value: i128, min: i128, max: i128) -> Result<(), Error> {
        if !NumericUtils::is_within_range(value, min, max) {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Validate vector is not empty
    pub fn validate_non_empty_vector<T>(vec: &Vec<T>) -> Result<(), Error> {
        if vec.len() == 0 {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Validate vector length
    pub fn validate_vector_length<T>(vec: &Vec<T>, expected_length: usize) -> Result<(), Error> {
        if vec.len() != expected_length {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Validate vector length range
    pub fn validate_vector_length_range<T>(vec: &Vec<T>, min_length: usize, max_length: usize) -> Result<(), Error> {
        let len = vec.len();
        if len < min_length || len > max_length {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Validate timestamp is in the future
    pub fn validate_future_timestamp(timestamp: u64, current_time: u64) -> Result<(), Error> {
        if !TimeUtils::is_future_timestamp(timestamp, current_time) {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Validate timestamp is in the past
    pub fn validate_past_timestamp(timestamp: u64, current_time: u64) -> Result<(), Error> {
        if !TimeUtils::is_past_timestamp(timestamp, current_time) {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }

    /// Validate timestamp is not expired
    pub fn validate_timestamp_not_expired(timestamp: u64, current_time: u64, max_age: u64) -> Result<(), Error> {
        if TimeUtils::is_timestamp_expired(timestamp, current_time, max_age) {
            return Err(Error::InvalidInput);
        }
        Ok(())
    }
}

// ===== CONVERSION UTILITIES =====

/// Conversion utility functions
pub struct ConversionUtils;

impl ConversionUtils {
    /// Convert i128 to string
    pub fn i128_to_string(env: &Env, value: i128) -> String {
        String::from_str(env, &value.to_string())
    }

    /// Convert u64 to string
    pub fn u64_to_string(env: &Env, value: u64) -> String {
        String::from_str(env, &value.to_string())
    }

    /// Convert u32 to string
    pub fn u32_to_string(env: &Env, value: u32) -> String {
        String::from_str(env, &value.to_string())
    }

    /// Convert bool to string
    pub fn bool_to_string(env: &Env, value: bool) -> String {
        String::from_str(env, if value { "true" } else { "false" })
    }

    /// Convert string to i128 (with error handling)
    pub fn string_to_i128(s: &String) -> Result<i128, Error> {
        s.to_string().parse::<i128>().map_err(|_| Error::InvalidInput)
    }

    /// Convert string to u64 (with error handling)
    pub fn string_to_u64(s: &String) -> Result<u64, Error> {
        s.to_string().parse::<u64>().map_err(|_| Error::InvalidInput)
    }

    /// Convert string to u32 (with error handling)
    pub fn string_to_u32(s: &String) -> Result<u32, Error> {
        s.to_string().parse::<u32>().map_err(|_| Error::InvalidInput)
    }

    /// Convert string to bool (with error handling)
    pub fn string_to_bool(s: &String) -> Result<bool, Error> {
        match s.to_string().to_lowercase().as_str() {
            "true" | "1" | "yes" => Ok(true),
            "false" | "0" | "no" => Ok(false),
            _ => Err(Error::InvalidInput),
        }
    }

    /// Convert address to string
    pub fn address_to_string(env: &Env, address: &Address) -> String {
        String::from_str(env, &address.to_string())
    }

    /// Convert symbol to string
    pub fn symbol_to_string(symbol: &Symbol) -> String {
        symbol.to_string()
    }
}

// ===== COMMON HELPER UTILITIES =====

/// Common helper utility functions
pub struct CommonUtils;

impl CommonUtils {
    /// Generate a unique identifier
    pub fn generate_unique_id(env: &Env, prefix: &str) -> String {
        let timestamp = env.ledger().timestamp();
        let sequence = env.ledger().sequence();
        let combined = format!("{}_{}_{}", prefix, timestamp, sequence);
        String::from_str(env, &combined)
    }

    /// Check if two addresses are equal
    pub fn addresses_equal(a: &Address, b: &Address) -> bool {
        a == b
    }

    /// Check if two symbols are equal
    pub fn symbols_equal(a: &Symbol, b: &Symbol) -> bool {
        a == b
    }

    /// Check if two strings are equal (case-insensitive)
    pub fn strings_equal_ignore_case(a: &String, b: &String) -> bool {
        a.to_string().to_lowercase() == b.to_string().to_lowercase()
    }

    /// Create a map from key-value pairs
    pub fn create_map_from_pairs(env: &Env, pairs: Vec<(String, String)>) -> Map<String, String> {
        let mut map = Map::new(env);
        for (key, value) in pairs.iter() {
            map.set(key.clone(), value.clone());
        }
        map
    }

    /// Get map keys as vector
    pub fn get_map_keys(map: &Map<String, String>) -> Vec<String> {
        let mut keys = Vec::new(&map.env());
        for key in map.keys() {
            keys.push_back(key);
        }
        keys
    }

    /// Get map values as vector
    pub fn get_map_values(map: &Map<String, String>) -> Vec<String> {
        let mut values = Vec::new(&map.env());
        for key in map.keys() {
            if let Some(value) = map.get(&key) {
                values.push_back(value);
            }
        }
        values
    }

    /// Check if map contains key
    pub fn map_contains_key(map: &Map<String, String>, key: &String) -> bool {
        map.has(&key)
    }

    /// Get map size
    pub fn get_map_size(map: &Map<String, String>) -> u32 {
        map.len()
    }

    /// Merge two maps
    pub fn merge_maps(env: &Env, map1: &Map<String, String>, map2: &Map<String, String>) -> Map<String, String> {
        let mut merged = Map::new(env);
        
        // Add all entries from map1
        for key in map1.keys() {
            if let Some(value) = map1.get(&key) {
                merged.set(key, value);
            }
        }
        
        // Add all entries from map2 (will override map1 entries with same key)
        for key in map2.keys() {
            if let Some(value) = map2.get(&key) {
                merged.set(key, value);
            }
        }
        
        merged
    }
}

// ===== TESTING UTILITIES =====

/// Testing utility functions
pub struct TestingUtils;

impl TestingUtils {
    /// Create a test address
    pub fn create_test_address(env: &Env) -> Address {
        Address::from_str(env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF")
    }

    /// Create a test symbol
    pub fn create_test_symbol(env: &Env, name: &str) -> Symbol {
        Symbol::new(env, name)
    }

    /// Create a test string
    pub fn create_test_string(env: &Env, content: &str) -> String {
        String::from_str(env, content)
    }

    /// Create a test vector of strings
    pub fn create_test_string_vector(env: &Env, items: &[&str]) -> Vec<String> {
        let mut vec = Vec::new(env);
        for item in items {
            vec.push_back(String::from_str(env, item));
        }
        vec
    }

    /// Create a test map
    pub fn create_test_map(env: &Env, pairs: &[(&str, &str)]) -> Map<String, String> {
        let mut map = Map::new(env);
        for (key, value) in pairs {
            map.set(String::from_str(env, key), String::from_str(env, value));
        }
        map
    }

    /// Validate test data structure
    pub fn validate_test_data_structure<T>(data: &T) -> Result<(), Error> {
        // Generic validation - always passes for testing
        Ok(())
    }

    /// Create random test data
    pub fn create_random_test_data(env: &Env, seed: u64) -> String {
        // Simple deterministic "random" string based on seed
        let hash = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let hex_string = format!("{:x}", hash);
        String::from_str(env, &hex_string)
    }

    /// Compare test results
    pub fn compare_test_results<T: PartialEq>(actual: &T, expected: &T) -> bool {
        actual == expected
    }

    /// Generate test timestamp
    pub fn generate_test_timestamp(base_time: u64, offset_seconds: i64) -> u64 {
        if offset_seconds >= 0 {
            base_time + offset_seconds as u64
        } else {
            base_time.saturating_sub((-offset_seconds) as u64)
        }
    }
}

// ===== MODULE TESTS =====

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_time_utils() {
        let env = Env::default();

        // Test days to seconds
        assert_eq!(TimeUtils::days_to_seconds(1), 86400);
        assert_eq!(TimeUtils::days_to_seconds(7), 604800);

        // Test hours to seconds
        assert_eq!(TimeUtils::hours_to_seconds(1), 3600);
        assert_eq!(TimeUtils::hours_to_seconds(24), 86400);

        // Test minutes to seconds
        assert_eq!(TimeUtils::minutes_to_seconds(1), 60);
        assert_eq!(TimeUtils::minutes_to_seconds(60), 3600);

        // Test time difference
        assert_eq!(TimeUtils::time_difference(100, 50), 50);
        assert_eq!(TimeUtils::time_difference(50, 100), 0);

        // Test future/past timestamp
        assert!(TimeUtils::is_future_timestamp(100, 50));
        assert!(!TimeUtils::is_future_timestamp(50, 100));
        assert!(TimeUtils::is_past_timestamp(50, 100));
        assert!(!TimeUtils::is_past_timestamp(100, 50));

        // Test timestamp expiration
        assert!(TimeUtils::is_timestamp_expired(50, 100, 30));
        assert!(!TimeUtils::is_timestamp_expired(80, 100, 30));

        // Test end time calculation
        assert_eq!(TimeUtils::calculate_end_time(100, 50), 150);

        // Test remaining time
        assert_eq!(TimeUtils::calculate_remaining_time(150, 100), 50);
        assert_eq!(TimeUtils::calculate_remaining_time(100, 150), 0);
    }

    #[test]
    fn test_string_utils() {
        let env = Env::default();

        // Test empty/whitespace check
        let empty = String::from_str(&env, "");
        let whitespace = String::from_str(&env, "   ");
        let normal = String::from_str(&env, "hello");

        assert!(StringUtils::is_empty_or_whitespace(&empty));
        assert!(StringUtils::is_empty_or_whitespace(&whitespace));
        assert!(!StringUtils::is_empty_or_whitespace(&normal));

        // Test string truncation
        let long_string = String::from_str(&env, "very long string");
        let truncated = StringUtils::truncate_string(&long_string, 10);
        assert_eq!(truncated.len(), 10);

        // Test case conversion
        let mixed = String::from_str(&env, "HeLLo WoRLd");
        let lower = StringUtils::to_lowercase(&mixed);
        let upper = StringUtils::to_uppercase(&mixed);
        assert_eq!(lower.to_string(), "hello world");
        assert_eq!(upper.to_string(), "HELLO WORLD");

        // Test substring operations
        let text = String::from_str(&env, "hello world");
        assert!(StringUtils::contains_substring(&text, "world"));
        assert!(!StringUtils::contains_substring(&text, "universe"));

        let replaced = StringUtils::replace_substring(&text, "world", "universe");
        assert_eq!(replaced.to_string(), "hello universe");

        // Test string splitting and joining
        let split_result = StringUtils::split_string(&text, " ");
        assert_eq!(split_result.len(), 2);
        assert_eq!(split_result.get(0).unwrap().to_string(), "hello");
        assert_eq!(split_result.get(1).unwrap().to_string(), "world");

        let joined = StringUtils::join_strings(&split_result, "-");
        assert_eq!(joined.to_string(), "hello-world");

        // Test string validation
        assert!(StringUtils::validate_string_length(&normal, 1, 10).is_ok());
        assert!(StringUtils::validate_string_length(&normal, 10, 20).is_err());

        // Test string sanitization
        let dirty = String::from_str(&env, "hello@world#123!");
        let clean = StringUtils::sanitize_string(&dirty);
        assert_eq!(clean.to_string(), "hello world 123");
    }

    #[test]
    fn test_numeric_utils() {
        // Test percentage calculations
        assert_eq!(NumericUtils::calculate_percentage(25, 100), 25);
        assert_eq!(NumericUtils::calculate_percentage(50, 200), 25);
        assert_eq!(NumericUtils::calculate_percentage(0, 100), 0);

        // Test weighted average
        let values = vec![10, 20, 30];
        let weights = vec![1, 2, 3];
        assert_eq!(NumericUtils::calculate_weighted_average(&values, &weights), 23);

        // Test interest calculations
        assert_eq!(NumericUtils::calculate_simple_interest(1000, 5, 2), 1100);
        assert_eq!(NumericUtils::calculate_simple_interest(1000, 0, 2), 1000);

        // Test rounding
        assert_eq!(NumericUtils::round_to_nearest(123, 10), 120);
        assert_eq!(NumericUtils::round_to_nearest(127, 10), 130);

        // Test min/max/clamp
        assert_eq!(NumericUtils::min(10, 20), 10);
        assert_eq!(NumericUtils::max(10, 20), 20);
        assert_eq!(NumericUtils::clamp(15, 10, 20), 15);
        assert_eq!(NumericUtils::clamp(5, 10, 20), 10);
        assert_eq!(NumericUtils::clamp(25, 10, 20), 20);

        // Test range validation
        assert!(NumericUtils::is_within_range(15, 10, 20));
        assert!(!NumericUtils::is_within_range(25, 10, 20));

        // Test absolute difference
        assert_eq!(NumericUtils::abs_difference(10, 20), 10);
        assert_eq!(NumericUtils::abs_difference(20, 10), 10);

        // Test square root
        assert_eq!(NumericUtils::sqrt(16), 4);
        assert_eq!(NumericUtils::sqrt(25), 5);
    }

    #[test]
    fn test_validation_utils() {
        let env = Env::default();

        // Test address validation
        let address = Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF");
        assert!(ValidationUtils::validate_address(&address).is_ok());

        // Test symbol validation
        let symbol = Symbol::new(&env, "test");
        assert!(ValidationUtils::validate_symbol(&symbol).is_ok());

        // Test string validation
        let empty = String::from_str(&env, "");
        let valid = String::from_str(&env, "hello");
        assert!(ValidationUtils::validate_non_empty_string(&empty).is_err());
        assert!(ValidationUtils::validate_non_empty_string(&valid).is_ok());

        // Test number validation
        assert!(ValidationUtils::validate_positive_number(10).is_ok());
        assert!(ValidationUtils::validate_positive_number(0).is_err());
        assert!(ValidationUtils::validate_positive_number(-10).is_err());

        assert!(ValidationUtils::validate_non_negative_number(10).is_ok());
        assert!(ValidationUtils::validate_non_negative_number(0).is_ok());
        assert!(ValidationUtils::validate_non_negative_number(-10).is_err());

        assert!(ValidationUtils::validate_number_range(15, 10, 20).is_ok());
        assert!(ValidationUtils::validate_number_range(25, 10, 20).is_err());

        // Test vector validation
        let empty_vec = Vec::new(&env);
        let valid_vec = TestingUtils::create_test_string_vector(&env, &["a", "b"]);
        assert!(ValidationUtils::validate_non_empty_vector(&empty_vec).is_err());
        assert!(ValidationUtils::validate_non_empty_vector(&valid_vec).is_ok());

        assert!(ValidationUtils::validate_vector_length(&valid_vec, 2).is_ok());
        assert!(ValidationUtils::validate_vector_length(&valid_vec, 3).is_err());

        assert!(ValidationUtils::validate_vector_length_range(&valid_vec, 1, 3).is_ok());
        assert!(ValidationUtils::validate_vector_length_range(&valid_vec, 3, 5).is_err());

        // Test timestamp validation
        let current_time = 100;
        assert!(ValidationUtils::validate_future_timestamp(150, current_time).is_ok());
        assert!(ValidationUtils::validate_future_timestamp(50, current_time).is_err());

        assert!(ValidationUtils::validate_past_timestamp(50, current_time).is_ok());
        assert!(ValidationUtils::validate_past_timestamp(150, current_time).is_err());

        assert!(ValidationUtils::validate_timestamp_not_expired(80, current_time, 30).is_ok());
        assert!(ValidationUtils::validate_timestamp_not_expired(50, current_time, 30).is_err());
    }

    #[test]
    fn test_conversion_utils() {
        let env = Env::default();

        // Test number to string conversions
        assert_eq!(ConversionUtils::i128_to_string(&env, 123).to_string(), "123");
        assert_eq!(ConversionUtils::u64_to_string(&env, 456).to_string(), "456");
        assert_eq!(ConversionUtils::u32_to_string(&env, 789).to_string(), "789");
        assert_eq!(ConversionUtils::bool_to_string(&env, true).to_string(), "true");
        assert_eq!(ConversionUtils::bool_to_string(&env, false).to_string(), "false");

        // Test string to number conversions
        let num_str = String::from_str(&env, "123");
        let bool_str = String::from_str(&env, "true");
        let invalid_str = String::from_str(&env, "invalid");

        assert_eq!(ConversionUtils::string_to_i128(&num_str).unwrap(), 123);
        assert_eq!(ConversionUtils::string_to_bool(&bool_str).unwrap(), true);
        assert!(ConversionUtils::string_to_i128(&invalid_str).is_err());

        // Test address and symbol conversions
        let address = Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF");
        let symbol = Symbol::new(&env, "test");

        assert_eq!(ConversionUtils::address_to_string(&env, &address).to_string(), address.to_string());
        assert_eq!(ConversionUtils::symbol_to_string(&symbol).to_string(), "test");
    }

    #[test]
    fn test_common_utils() {
        let env = Env::default();

        // Test unique ID generation
        let id1 = CommonUtils::generate_unique_id(&env, "test");
        let id2 = CommonUtils::generate_unique_id(&env, "test");
        assert_ne!(id1.to_string(), id2.to_string());

        // Test address comparison
        let addr1 = Address::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF");
        let addr2 = Address::from_str(&env, "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB");
        assert!(CommonUtils::addresses_equal(&addr1, &addr1));
        assert!(!CommonUtils::addresses_equal(&addr1, &addr2));

        // Test symbol comparison
        let sym1 = Symbol::new(&env, "test");
        let sym2 = Symbol::new(&env, "other");
        assert!(CommonUtils::symbols_equal(&sym1, &sym1));
        assert!(!CommonUtils::symbols_equal(&sym1, &sym2));

        // Test string comparison (case-insensitive)
        let str1 = String::from_str(&env, "Hello");
        let str2 = String::from_str(&env, "hello");
        let str3 = String::from_str(&env, "world");
        assert!(CommonUtils::strings_equal_ignore_case(&str1, &str2));
        assert!(!CommonUtils::strings_equal_ignore_case(&str1, &str3));

        // Test map operations
        let pairs = vec![
            (String::from_str(&env, "key1"), String::from_str(&env, "value1")),
            (String::from_str(&env, "key2"), String::from_str(&env, "value2")),
        ];
        let map = CommonUtils::create_map_from_pairs(&env, pairs);

        assert_eq!(CommonUtils::get_map_size(&map), 2);
        assert!(CommonUtils::map_contains_key(&map, &String::from_str(&env, "key1")));
        assert!(!CommonUtils::map_contains_key(&map, &String::from_str(&env, "key3")));

        let keys = CommonUtils::get_map_keys(&map);
        assert_eq!(keys.len(), 2);

        let values = CommonUtils::get_map_values(&map);
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_testing_utils() {
        let env = Env::default();

        // Test test data creation
        let address = TestingUtils::create_test_address(&env);
        let symbol = TestingUtils::create_test_symbol(&env, "test");
        let string = TestingUtils::create_test_string(&env, "hello");
        let vector = TestingUtils::create_test_string_vector(&env, &["a", "b", "c"]);
        let map = TestingUtils::create_test_map(&env, &[("key1", "value1"), ("key2", "value2")]);

        assert_eq!(address.to_string(), "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF");
        assert_eq!(symbol.to_string(), "test");
        assert_eq!(string.to_string(), "hello");
        assert_eq!(vector.len(), 3);
        assert_eq!(map.len(), 2);

        // Test validation
        assert!(TestingUtils::validate_test_data_structure(&string).is_ok());

        // Test random data generation
        let random1 = TestingUtils::create_random_test_data(&env, 1);
        let random2 = TestingUtils::create_random_test_data(&env, 2);
        assert_ne!(random1.to_string(), random2.to_string());

        // Test result comparison
        assert!(TestingUtils::compare_test_results(&10, &10));
        assert!(!TestingUtils::compare_test_results(&10, &20));

        // Test timestamp generation
        let base_time = 1000;
        assert_eq!(TestingUtils::generate_test_timestamp(base_time, 100), 1100);
        assert_eq!(TestingUtils::generate_test_timestamp(base_time, -100), 900);
    }
} 