//! Criterion benchmarks for the Disputes contract's hot admin entrypoints.
//!
//! Run with `cargo bench -p disputes`.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};

use disputes::admin;

fn setup() -> (Env, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin_addr = Address::generate(&env);
    admin::set_admin(&env, &admin_addr);
    (env, admin_addr)
}

/// Benchmarks the admin-identity check performed before every gated action.
fn bench_require_admin(c: &mut Criterion) {
    let (env, admin_addr) = setup();
    c.bench_function("require_admin", |b| {
        b.iter(|| admin::require_admin(black_box(&env), black_box(&admin_addr)))
    });
}

/// Benchmarks recording a critical admin action (updates the cooldown clock).
fn bench_record_admin_action(c: &mut Criterion) {
    let (env, _admin_addr) = setup();
    c.bench_function("record_admin_action", |b| {
        b.iter(|| admin::record_admin_action(black_box(&env)))
    });
}

/// Benchmarks the cooldown check against a previously recorded action.
fn bench_validate_admin_cooldown(c: &mut Criterion) {
    let (env, _admin_addr) = setup();
    admin::record_admin_action(&env);
    c.bench_function("validate_admin_cooldown", |b| {
        b.iter(|| black_box(admin::validate_admin_cooldown(black_box(&env))))
    });
}

/// Benchmarks updating the configurable cooldown window.
fn bench_set_cooldown_period(c: &mut Criterion) {
    let (env, admin_addr) = setup();
    c.bench_function("set_cooldown_period", |b| {
        b.iter(|| {
            admin::set_cooldown_period(black_box(&env), black_box(&admin_addr), black_box(7_200))
        })
    });
}

/// Benchmarks reading the stored admin address.
fn bench_get_admin(c: &mut Criterion) {
    let (env, _admin_addr) = setup();
    c.bench_function("get_admin", |b| {
        b.iter(|| black_box(admin::get_admin(black_box(&env))))
    });
}

criterion_group!(
    benches,
    bench_require_admin,
    bench_record_admin_action,
    bench_validate_admin_cooldown,
    bench_set_cooldown_period,
    bench_get_admin
);
criterion_main!(benches);
