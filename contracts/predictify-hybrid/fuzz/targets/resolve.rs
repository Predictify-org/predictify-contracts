//! Cargo-fuzz target for the automated `resolve_market` path.
//!
//! The actual fuzz implementation lives in
//! `src/tests/resolve_market_fuzz.rs` (a proptest-based test module)
//! to keep access to internal crate APIs.
//!
//! ## Run the proptest fuzz target
//!
//! ```bash
//! cargo test -p predictify-hybrid -- resolve_market_fuzz
//! ```
//!
//! ## Cargo-fuzz setup
//!
//! To run via `cargo fuzz`:
//!
//! 1. Install cargo-fuzz: `cargo install cargo-fuzz`
//! 2. Add a `fuzz/Cargo.toml` that depends on `libfuzzer-sys` and this crate
//! 3. Populate this file with a `fuzz_target!` macro wrapping the logic
//!    from the proptest tests
//!
//! ## Boundary conditions covered
//!
//! | Category | Conditions |
//! |----------|-----------|
//! | Oracle result | `None`, valid result, empty string, invalid outcome, long string |
//! | Market state | `Ended` (valid), `Active` (should fail), already `Resolved` (should fail) |
//! | Vote distribution | No votes, single voter, skewed, balanced, exact tie |
//! | Stakes | Zero, minimal, large, extreme via raw bytes |
//! | Outcome count | 2 (minimum), 3–7, 8 (near maximum) |
//! | Duration | 1 day (minimum), 30 days (typical), 365 days (maximum) |
//! | Double-resolve | Second call after successful resolution must refuse |
//! | Market without oracle result | Must fail with `OracleUnavailable` |
//! | Market before end_time | Must fail with `MarketClosed` |
//! | Min pool not met | Market with `min_pool_size` above `total_staked` |
//!
//! ## Security
//!
//! All fuzz cases assume `mock_all_auths()` is active (auth is tested separately).
//! No `unwrap()` is used in production paths.
//!
//! ## Invariants Checked
//!
//! | Invariant | Description |
//! |-----------|-------------|
//! | No panic on valid input | All paths handle valid inputs gracefully |
//! | Idempotency | Multiple resolve calls are handled correctly |
//! | State consistency | Market transitions to `Resolved` on success |
//! | Confidence bounds | `confidence_score` is always <= 100 |
//! | Outcome set | On success, `winning_outcomes` must be `Some` |
