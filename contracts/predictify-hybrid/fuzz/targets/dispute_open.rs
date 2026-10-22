//! Cargo-fuzz target for `dispute_open` boundary cases.
//!
//! The actual fuzz implementation lives in
//! `src/tests/dispute_open_fuzz.rs` (a proptest-based test module)
//! to keep access to internal crate APIs.
//!
//! ## Run the proptest fuzz target
//!
//! ```bash
//! cargo test -p predictify-hybrid -- dispute_open_fuzz
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
//! | Stake | `MIN_DISPUTE_STAKE` (10_000_000), just below, just above, i128::MAX, zero, negative |
//! | Market timing | Before `end_time`, after `end_time`, after dispute window closes |
//! | Anti-grief | Stake below floor, at floor, above floor |
//! | Duplicates | Same user disputing twice |
//! | Stake caps | Per-market per-user cap exceeded |
//! | Reason | `None`, empty `Some`, long string |
//!
//! ## Security
//!
//! All cases assume `mock_all_auths()` is active (auth is tested separately).
//! No `unwrap()` is used in production paths.
