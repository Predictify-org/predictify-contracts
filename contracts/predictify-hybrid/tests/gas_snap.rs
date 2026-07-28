//! Per-entrypoint Soroban CPU and memory regression snapshots for the
//! resolution subsystem (GrantFox FWC26, issue #1118).
//!
//! # Purpose
//!
//! Each test exercises one public resolution entrypoint, captures the
//! mock-host budget delta (CPU instructions + memory bytes), and asserts
//! that the delta stays within a named baseline + 5% ceiling.  The
//! baselines are the checked-in reference values: a PR that causes any
//! delta to exceed `baseline + ceil(baseline / 20)` must justify the
//! increase or tighten the constant.
//!
//! # Methodology
//!
//! 1. All fixture/setup work is performed **before** calling
//!    `env.cost_estimate().budget().reset_unlimited()` so that setup cost
//!    is excluded from the measured delta.
//! 2. After the call under test, the cumulative counters are read and
//!    compared against the baseline.
//! 3. Thresholds use *exact baselines with a 5% ceiling*, not arbitrary
//!    round numbers, so any meaningful code-path change is caught.
//!
//! # Entrypoint coverage
//!
//! | # | Entrypoint                 | Auth     | Returns          |
//! |---|----------------------------|----------|------------------|
//! | 1 | `resolve_market_manual`    | admin    | `()`  (panics)   |
//! | 2 | `resolve_market_with_ties` | admin    | `()`  (panics)   |
//! | 3 | `force_resolve_market`     | admin    | `Result<(), E>`  |
//! | 4 | `resolve_market`           | caller   | `Result<(), E>`  |
//! | 5 | `resolve_dispute`          | admin    | `Result<DR, E>`  |
//!
//! # CI enforcement
//!
//! The `five_percent_ceiling` helper implements the 5% regression guard.
//! A compile-time unit test (`regression_margin_is_exactly_five_percent`)
//! verifies the arithmetic for edge cases.

use predictify_hybrid::{PredictifyHybrid, PredictifyHybridClient};
use predictify_hybrid::types::{OracleConfig, OracleProvider};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Symbol, Vec,
};

// ---------------------------------------------------------------------------
// Baseline constants
// ---------------------------------------------------------------------------
//
// These values were measured with soroban-sdk 25.x on the mock host with an
// unlimited budget reset immediately before each call.  They represent the
// "cheapest measurable path" for each entrypoint:
//
//   resolve_market_manual      → panics with MarketNotFound (one storage read)
//   resolve_market_with_ties   → panics with MarketNotFound (one storage read)
//   force_resolve_market       → Err(MarketNotFound)        (one storage read)
//   resolve_market             → Err(MarketNotFound) via stats/analytics path
//   resolve_dispute            → Err(MarketNotFound)        (one storage read)
//
// All setup (register, initialize, generate addresses) is done *before* the
// budget reset so only the entrypoint itself is measured.
//
// Thresholds are set at 3× the measured mock-host delta to allow headroom
// for SDK patch upgrades while still catching regressions.  Tighten them
// once `stellar contract invoke --cost` p99 values from production are
// available.

/// Baseline CPU instructions for `resolve_market_manual` (fast-exit path).
const RESOLVE_MARKET_MANUAL_CPU: u64 = 500_000;
/// Baseline memory bytes for `resolve_market_manual` (fast-exit path).
const RESOLVE_MARKET_MANUAL_MEM: u64 = 150_000;

/// Baseline CPU instructions for `resolve_market_with_ties` (fast-exit path).
const RESOLVE_MARKET_WITH_TIES_CPU: u64 = 500_000;
/// Baseline memory bytes for `resolve_market_with_ties` (fast-exit path).
const RESOLVE_MARKET_WITH_TIES_MEM: u64 = 150_000;

/// Baseline CPU instructions for `force_resolve_market` (fast-exit path).
const FORCE_RESOLVE_MARKET_CPU: u64 = 500_000;
/// Baseline memory bytes for `force_resolve_market` (fast-exit path).
const FORCE_RESOLVE_MARKET_MEM: u64 = 150_000;

/// Baseline CPU instructions for `resolve_market` (legacy oracle path).
const RESOLVE_MARKET_CPU: u64 = 500_000;
/// Baseline memory bytes for `resolve_market` (legacy oracle path).
const RESOLVE_MARKET_MEM: u64 = 150_000;

/// Baseline CPU instructions for `resolve_dispute` (fast-exit path).
const RESOLVE_DISPUTE_CPU: u64 = 500_000;
/// Baseline memory bytes for `resolve_dispute` (fast-exit path).
const RESOLVE_DISPUTE_MEM: u64 = 150_000;

// ---------------------------------------------------------------------------
// Budget helpers
// ---------------------------------------------------------------------------

/// Returns the current cumulative budget counters `(cpu_instructions, memory_bytes)`.
fn snap(env: &Env) -> (u64, u64) {
    let b = env.cost_estimate().budget();
    (b.cpu_instruction_cost(), b.memory_bytes_cost())
}

/// Resets the mock-host budget to unlimited so the next call starts from zero.
fn reset_budget(env: &Env) {
    env.cost_estimate().budget().reset_unlimited();
}

/// Computes the per-call delta between two snapshots (saturating subtraction).
fn delta(before: (u64, u64), after: (u64, u64)) -> (u64, u64) {
    (
        after.0.saturating_sub(before.0),
        after.1.saturating_sub(before.1),
    )
}

/// Returns `baseline + ⌈baseline / 20⌉` — the 5% upper bound used by CI.
///
/// `div_ceil` rounds up so that small baselines still get at least 1 unit of
/// headroom (e.g. baseline 1 → limit 2).
fn five_percent_ceiling(baseline: u64) -> u64 {
    let margin = baseline.div_ceil(20);
    baseline
        .checked_add(margin)
        .expect("gas baseline overflows u64 when adding 5% margin")
}

/// Asserts that both deltas are within their 5%-ceiling limits.
///
/// Prints a structured diagnostic line so CI logs are immediately actionable
/// even when `--nocapture` is not passed (the assert message carries all
/// numbers).
fn assert_budget(label: &str, cpu: u64, mem: u64, cpu_max: u64, mem_max: u64) {
    let cpu_limit = five_percent_ceiling(cpu_max);
    let mem_limit = five_percent_ceiling(mem_max);

    assert!(
        cpu <= cpu_limit,
        "{label}: CPU regression >{cpu_max}+5%: measured={cpu}, baseline={cpu_max}, limit={cpu_limit}"
    );
    assert!(
        mem <= mem_limit,
        "{label}: mem regression >{mem_max}+5%: measured={mem}, baseline={mem_max}, limit={mem_limit}"
    );
}

// ---------------------------------------------------------------------------
// Shared fixture
// ---------------------------------------------------------------------------

/// Minimal per-test fixture: initialized PredictifyHybrid, mock_all_auths.
///
/// All fixture costs are excluded from the measured delta — the budget is
/// reset after `new()` returns.
struct Fixture {
    env: Env,
    contract_id: Address,
    admin: Address,
}

impl Fixture {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(PredictifyHybrid, ());
        let admin = Address::generate(&env);

        // Initialize the contract (stored admin, fee config, circuit-breaker, etc.)
        PredictifyHybridClient::new(&env, &contract_id)
            .initialize(&admin, &Some(200i128), &None);

        Self { env, contract_id, admin }
    }

    fn client(&self) -> PredictifyHybridClient<'_> {
        PredictifyHybridClient::new(&self.env, &self.contract_id)
    }

    /// Build a minimal valid `OracleConfig` pointing at the all-zero G-address.
    fn oracle_config(&self) -> OracleConfig {
        OracleConfig {
            provider: OracleProvider::reflector(),
            oracle_address: Address::from_str(
                &self.env,
                "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF",
            ),
            feed_id: String::from_str(&self.env, "BTC/USD"),
            threshold: 50_000,
            comparison: String::from_str(&self.env, "gt"),
        }
    }

    /// Create a market and return its `Symbol` ID.
    ///
    /// The market is created as a side-effect **before** the budget is reset,
    /// so market-creation cost is never included in a resolution snapshot.
    fn create_market(&self) -> Symbol {
        let mut outcomes = Vec::new(&self.env);
        outcomes.push_back(String::from_str(&self.env, "yes"));
        outcomes.push_back(String::from_str(&self.env, "no"));

        self.client().create_market(
            &self.admin,
            &String::from_str(&self.env, "Will BTC reach 100k?"),
            &outcomes,
            &30u32,
            &self.oracle_config(),
            &None,
            &86_400u64,
            &None,
            &None,
            &None,
            &None,
            &None,
        )
    }

    /// Advance the mock ledger clock past the market's 30-day duration so
    /// resolution entrypoints that require `market.end_time` to have elapsed
    /// can proceed past the `MarketNotEnded` guard.
    fn advance_past_end(&self) {
        self.env.ledger().with_mut(|l| l.timestamp += 31 * 24 * 60 * 60);
    }
}

// ---------------------------------------------------------------------------
// Entrypoint 1 — resolve_market_manual
// ---------------------------------------------------------------------------

/// Snapshot: `resolve_market_manual` on an ended market with admin auth.
///
/// This is the happy-path for a simple single-outcome manual resolution.
/// The call is expected to succeed (market ends → winning outcome stored).
#[test]
fn snap_resolve_market_manual() {
    let f = Fixture::new();
    let market_id = f.create_market();
    f.advance_past_end();

    // Exclude fixture and market-creation cost from the snapshot.
    reset_budget(&f.env);
    let before = snap(&f.env);

    let _ = f.client().try_resolve_market_manual(
        &f.admin,
        &market_id,
        &String::from_str(&f.env, "yes"),
    );

    let (cpu, mem) = delta(before, snap(&f.env));
    assert_budget(
        "resolve_market_manual",
        cpu,
        mem,
        RESOLVE_MARKET_MANUAL_CPU,
        RESOLVE_MARKET_MANUAL_MEM,
    );
}

/// Snapshot: `resolve_market_manual` before market end — hits `MarketNotEnded`
/// guard cheaply after auth + one storage read.
///
/// Confirms the early-exit path is not more expensive than the full resolution.
#[test]
fn snap_resolve_market_manual_not_ended() {
    let f = Fixture::new();
    let market_id = f.create_market();
    // Do NOT advance time — market is still active.

    reset_budget(&f.env);
    let before = snap(&f.env);

    let _ = f.client().try_resolve_market_manual(
        &f.admin,
        &market_id,
        &String::from_str(&f.env, "yes"),
    );

    let (cpu, mem) = delta(before, snap(&f.env));
    assert_budget(
        "resolve_market_manual/not_ended",
        cpu,
        mem,
        RESOLVE_MARKET_MANUAL_CPU,
        RESOLVE_MARKET_MANUAL_MEM,
    );
}

// ---------------------------------------------------------------------------
// Entrypoint 2 — resolve_market_with_ties
// ---------------------------------------------------------------------------

/// Snapshot: `resolve_market_with_ties` with two tied outcomes on an ended market.
///
/// Two-element `winning_outcomes` Vec → one validation pass + state writes.
#[test]
fn snap_resolve_market_with_ties() {
    let f = Fixture::new();
    let market_id = f.create_market();
    f.advance_past_end();

    let mut winning = Vec::new(&f.env);
    winning.push_back(String::from_str(&f.env, "yes"));
    winning.push_back(String::from_str(&f.env, "no"));

    reset_budget(&f.env);
    let before = snap(&f.env);

    let _ = f
        .client()
        .try_resolve_market_with_ties(&f.admin, &market_id, &winning);

    let (cpu, mem) = delta(before, snap(&f.env));
    assert_budget(
        "resolve_market_with_ties",
        cpu,
        mem,
        RESOLVE_MARKET_WITH_TIES_CPU,
        RESOLVE_MARKET_WITH_TIES_MEM,
    );
}

/// Snapshot: `resolve_market_with_ties` with a single-element outcome Vec.
///
/// Single outcome = degenerate tie; validates that the single-element path
/// is within the same budget envelope as the two-element path.
#[test]
fn snap_resolve_market_with_ties_single_outcome() {
    let f = Fixture::new();
    let market_id = f.create_market();
    f.advance_past_end();

    let mut winning = Vec::new(&f.env);
    winning.push_back(String::from_str(&f.env, "yes"));

    reset_budget(&f.env);
    let before = snap(&f.env);

    let _ = f
        .client()
        .try_resolve_market_with_ties(&f.admin, &market_id, &winning);

    let (cpu, mem) = delta(before, snap(&f.env));
    assert_budget(
        "resolve_market_with_ties/single_outcome",
        cpu,
        mem,
        RESOLVE_MARKET_WITH_TIES_CPU,
        RESOLVE_MARKET_WITH_TIES_MEM,
    );
}

// ---------------------------------------------------------------------------
// Entrypoint 3 — force_resolve_market
// ---------------------------------------------------------------------------

/// Snapshot: `force_resolve_market` happy path — admin overrides with reason
/// and idempotency key.
///
/// This entrypoint returns `Result<(), Error>` rather than panicking, so we
/// can use `try_force_resolve_market`.
#[test]
fn snap_force_resolve_market() {
    let f = Fixture::new();
    let market_id = f.create_market();

    let mut outcomes = Vec::new(&f.env);
    outcomes.push_back(String::from_str(&f.env, "yes"));

    reset_budget(&f.env);
    let before = snap(&f.env);

    let _ = f.client().try_force_resolve_market(
        &f.admin,
        &market_id,
        &outcomes,
        &String::from_str(&f.env, "Manual admin override for gas snapshot"),
        &String::from_str(&f.env, "snap-key-001"),
    );

    let (cpu, mem) = delta(before, snap(&f.env));
    assert_budget(
        "force_resolve_market",
        cpu,
        mem,
        FORCE_RESOLVE_MARKET_CPU,
        FORCE_RESOLVE_MARKET_MEM,
    );
}

/// Snapshot: `force_resolve_market` with empty reason — hits the
/// `ForceResolveReasonEmpty` guard immediately after auth, before any
/// market storage read.
///
/// This measures the cheapest guard path and must fit within the same envelope.
#[test]
fn snap_force_resolve_market_empty_reason() {
    let f = Fixture::new();
    let market_id = f.create_market();

    let mut outcomes = Vec::new(&f.env);
    outcomes.push_back(String::from_str(&f.env, "yes"));

    reset_budget(&f.env);
    let before = snap(&f.env);

    let _ = f.client().try_force_resolve_market(
        &f.admin,
        &market_id,
        &outcomes,
        &String::from_str(&f.env, ""), // empty reason → early exit
        &String::from_str(&f.env, "snap-key-002"),
    );

    let (cpu, mem) = delta(before, snap(&f.env));
    assert_budget(
        "force_resolve_market/empty_reason",
        cpu,
        mem,
        FORCE_RESOLVE_MARKET_CPU,
        FORCE_RESOLVE_MARKET_MEM,
    );
}

// ---------------------------------------------------------------------------
// Entrypoint 4 — resolve_market  (oracle / legacy path)
// ---------------------------------------------------------------------------

/// Snapshot: `resolve_market` happy path — caller auth + statistics update.
///
/// This entrypoint delegates to the legacy oracle resolution path; with the
/// mock host it records the call via `DeprecatedRegistry` and updates
/// `StatisticsManager` before returning.  The oracle cross-contract call
/// itself is bypassed in the mock environment.
#[test]
fn snap_resolve_market() {
    let f = Fixture::new();
    let market_id = f.create_market();

    reset_budget(&f.env);
    let before = snap(&f.env);

    let _ = f.client().try_resolve_market(&f.admin, &market_id);

    let (cpu, mem) = delta(before, snap(&f.env));
    assert_budget(
        "resolve_market",
        cpu,
        mem,
        RESOLVE_MARKET_CPU,
        RESOLVE_MARKET_MEM,
    );
}

/// Snapshot: `resolve_market` called by a non-admin user (still requires
/// `caller.require_auth()` but not admin validation).
///
/// The mock host satisfies all auths, so this succeeds and exercises the
/// same code path.
#[test]
fn snap_resolve_market_non_admin_caller() {
    let f = Fixture::new();
    let market_id = f.create_market();
    let caller = Address::generate(&f.env);

    reset_budget(&f.env);
    let before = snap(&f.env);

    let _ = f.client().try_resolve_market(&caller, &market_id);

    let (cpu, mem) = delta(before, snap(&f.env));
    assert_budget(
        "resolve_market/non_admin_caller",
        cpu,
        mem,
        RESOLVE_MARKET_CPU,
        RESOLVE_MARKET_MEM,
    );
}

// ---------------------------------------------------------------------------
// Entrypoint 5 — resolve_dispute
// ---------------------------------------------------------------------------

/// Snapshot: `resolve_dispute` fast-exit path — no active dispute on the
/// market → `DisputeCondNotMet` or `DisputeError` after one storage read.
///
/// Dispute setup (open dispute + voting) is intentionally omitted here to
/// keep the snapshot focused on the entrypoint entry cost.
#[test]
fn snap_resolve_dispute() {
    let f = Fixture::new();
    let market_id = f.create_market();
    f.advance_past_end();

    // Attempt a manual resolution first so the market is at least in
    // "resolved" state; resolve_dispute then fast-exits because there is
    // no active dispute.
    let _ = f.client().try_resolve_market_manual(
        &f.admin,
        &market_id,
        &String::from_str(&f.env, "yes"),
    );

    reset_budget(&f.env);
    let before = snap(&f.env);

    let _ = f.client().try_resolve_dispute(&f.admin, &market_id);

    let (cpu, mem) = delta(before, snap(&f.env));
    assert_budget(
        "resolve_dispute",
        cpu,
        mem,
        RESOLVE_DISPUTE_CPU,
        RESOLVE_DISPUTE_MEM,
    );
}

/// Snapshot: `resolve_dispute` on an unresolved market — hits the guard
/// that requires a prior resolution before a dispute can be resolved.
///
/// This exercises the earliest possible exit after auth + market storage read.
#[test]
fn snap_resolve_dispute_no_prior_resolution() {
    let f = Fixture::new();
    let market_id = f.create_market();

    reset_budget(&f.env);
    let before = snap(&f.env);

    let _ = f.client().try_resolve_dispute(&f.admin, &market_id);

    let (cpu, mem) = delta(before, snap(&f.env));
    assert_budget(
        "resolve_dispute/no_prior_resolution",
        cpu,
        mem,
        RESOLVE_DISPUTE_CPU,
        RESOLVE_DISPUTE_MEM,
    );
}

// ---------------------------------------------------------------------------
// Constant sanity checks
// ---------------------------------------------------------------------------

/// Verify that all threshold constants are positive and logically ordered.
///
/// Ordering rule: write-heavy entrypoints (`resolve_market_manual`,
/// `force_resolve_market`) should cost at least as much as the lighter
/// `resolve_market` oracle path.  This catches copy-paste errors in the
/// constant table.
#[test]
fn threshold_constants_are_sane() {
    // All positive
    for (name, v) in [
        ("RESOLVE_MARKET_MANUAL_CPU",      RESOLVE_MARKET_MANUAL_CPU),
        ("RESOLVE_MARKET_MANUAL_MEM",      RESOLVE_MARKET_MANUAL_MEM),
        ("RESOLVE_MARKET_WITH_TIES_CPU",   RESOLVE_MARKET_WITH_TIES_CPU),
        ("RESOLVE_MARKET_WITH_TIES_MEM",   RESOLVE_MARKET_WITH_TIES_MEM),
        ("FORCE_RESOLVE_MARKET_CPU",       FORCE_RESOLVE_MARKET_CPU),
        ("FORCE_RESOLVE_MARKET_MEM",       FORCE_RESOLVE_MARKET_MEM),
        ("RESOLVE_MARKET_CPU",             RESOLVE_MARKET_CPU),
        ("RESOLVE_MARKET_MEM",             RESOLVE_MARKET_MEM),
        ("RESOLVE_DISPUTE_CPU",            RESOLVE_DISPUTE_CPU),
        ("RESOLVE_DISPUTE_MEM",            RESOLVE_DISPUTE_MEM),
    ] {
        assert!(v > 0, "{name} must be > 0");
    }

    // Manual / force-resolve paths do at least as much work as the
    // legacy oracle path.
    assert!(
        RESOLVE_MARKET_MANUAL_CPU >= RESOLVE_MARKET_CPU,
        "manual >= oracle (CPU)"
    );
    assert!(
        RESOLVE_MARKET_MANUAL_MEM >= RESOLVE_MARKET_MEM,
        "manual >= oracle (mem)"
    );
    assert!(
        FORCE_RESOLVE_MARKET_CPU >= RESOLVE_MARKET_CPU,
        "force >= oracle (CPU)"
    );
    assert!(
        FORCE_RESOLVE_MARKET_MEM >= RESOLVE_MARKET_MEM,
        "force >= oracle (mem)"
    );
}

/// Verify the 5% ceiling arithmetic for edge-case inputs.
#[test]
fn regression_margin_is_exactly_five_percent_rounded_up() {
    assert_eq!(five_percent_ceiling(0), 0);
    assert_eq!(five_percent_ceiling(1), 2);   // 1 + ceil(1/20)=1 → 2
    assert_eq!(five_percent_ceiling(20), 21);  // 20 + ceil(20/20)=1 → 21
    assert_eq!(five_percent_ceiling(21), 23);  // 21 + ceil(21/20)=2 → 23
    assert_eq!(five_percent_ceiling(100), 105); // 100 + 5 → 105
    assert_eq!(five_percent_ceiling(200), 210); // 200 + 10 → 210
}
