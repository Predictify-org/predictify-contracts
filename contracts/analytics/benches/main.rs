use criterion::{black_box, criterion_group, criterion_main, Criterion};
use soroban_sdk::{Env, Vec};

/// Benchmarks the cost of constructing a representative event payload.
fn bench_record_event(c: &mut Criterion) {
    let env = Env::default();

    c.bench_function("record_event", |b| {
        b.iter(|| {
            let event = (
                black_box(1u32),
                black_box(100u64),
                black_box(1_000_000i128),
                black_box(true),
            );
            black_box(event)
        })
    });

    black_box(env);
}

/// Benchmarks aggregation over a representative event stream.
fn bench_aggregate(c: &mut Criterion) {
    let events: Vec<u64> = (0u64..1_000).collect();

    c.bench_function("aggregate_1000", |b| {
        b.iter(|| {
            let total = events.iter().fold(0u64, |total, value| {
                total.saturating_add(black_box(*value))
            });
            black_box(total)
        })
    });
}

/// Benchmarks percentile selection over a representative event stream.
fn bench_percentile(c: &mut Criterion) {
    c.bench_function("percentile_p95_1000", |b| {
        b.iter(|| {
            let mut values: std::vec::Vec<u64> = (0u64..1_000).collect();
            values.sort_unstable();
            black_box(values.get(949).copied().unwrap_or(0))
        })
    });
}

criterion_group!(benches, bench_record_event, bench_aggregate, bench_percentile);
criterion_main!(benches);
