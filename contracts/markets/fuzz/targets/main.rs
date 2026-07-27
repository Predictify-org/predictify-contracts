//! Cargo-fuzz target for prediction markets boundary cases.
//!
//! The actual fuzz implementation lives in
//! `src/tests/markets_fuzz.rs` (a proptest-based test module)
//! to keep access to internal crate APIs.
//!
//! ## Run the proptest fuzz target
//!
//! ```bash
//! cargo test -p predictify-hybrid -- markets_fuzz
//! ```
//!
//! ## Boundary conditions covered
//!
//! | Category | Conditions |
//! |----------|-----------|
//! | Question | Empty, valid length, too short, too long, whitespace-only |
//! | Outcomes | Too few (< 2), too many (> 10), empty outcome strings, duplicates, length limits |
//! | Duration | Zero days, valid durations (1-365 days), invalid durations (> 365 days) |
//! | Oracle configs | Valid/invalid configurations |
//! | Extreme bounds | Very large values and invalid inputs that must not cause panics |
//!
//! ## Security
//!
//! All cases assume `mock_all_auths()` is active (auth is tested separately).
//! No `unwrap()` is used in production paths.
