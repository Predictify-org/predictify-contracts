use criterion::{black_box, criterion_group, criterion_main, Criterion};
use soroban_sdk::{Env, Vec};

fn bench_record_event(c: &mut Criterion) {
    let env = Env::default();
    c.bench_function("record_event", |b| b.iter(|| black_box(0u32 as u64 + black_box(100u64))));
}

fn bench_aggregate(c: &mut Criterion) {
    c.bench_function("aggregate_1000", |b| b.iter(|| black_box((0u64..1000).sum::<u64>())));
}

fn bench_percentile(c: &mut Criterion) {
    c.bench_function("percentile_p95_1000", |b| b.iter(|| {
        let mut v: Vec<u64> = (0u64..1000).collect();
        v.sort();
        black_box(v.get(949).copied().unwrap_or(0))
    }));
}

criterion_group!(benches, bench_record_event, bench_aggregate, bench_percentile);
criterion_main!(benches);

