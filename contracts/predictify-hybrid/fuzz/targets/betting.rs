//! Cargo-fuzz target for `place_bet` boundary cases.
//!
//! The actual fuzz implementation lives in
//! `src/tests/betting_fuzz.rs` (a proptest-based test module)
//! to keep access to internal crate APIs.
//!
//! ## Run the proptest fuzz target
//!
//! ```bash
//! cargo test -p predictify-hybrid -- betting_fuzz
//! ```
//!
//! ## Cargo-fuzz setup (future work)
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
//! | Amount | MIN_BET_AMOUNT (1_000_000), just below, just above, MAX_BET_AMOUNT, above max, zero, negative, i128::MAX |
//! | Outcomes | Valid outcome, invalid outcome, empty string |
//! | Market state | Active, Closed, Resolved |
//! | Market timing | Before end_time, at bet_deadline, after bet_deadline, after end_time |
//! | Bet deadlines | Explicit deadline before end_time, at end_time, after end_time (invalid) |
//! | Double betting | Same user placing bet twice (AlreadyBet) |
//! | Fee slippage | Fee at max, fee above max, fee below max |
//! | Per-market max bet cap | Amount below cap, at cap, above cap (BetExceedsCap) |
//! | Per-user max bet cap | Cumulative stake below cap, at cap, above cap (MaxBetCapExceeded) |
//! | Extreme values | i128::MIN, i128::MAX via raw bytes (no panic) |
//! | Outcome strings | Empty, very long, special chars |
//!
//! ## Security
//!
//! All cases assume `mock_all_auths()` is active (auth is tested separately).
//! No `unwrap()` is used in production paths.
