use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use hyra_scribe_ledger::storage_ops::{
    batched_get_operations, batched_mixed_operations, batched_put_operations, populate_ledger,
    throughput_get_10k, throughput_put_10k,
};
use hyra_scribe_ledger::HyraScribeLedger;
use std::time::Duration;

fn benchmark_put_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("put_operations");
    group.measurement_time(Duration::from_secs(10));

    // Test different scales of operations
    for ops in [1, 10, 100, 1000, 5000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("put", ops), ops, |b, &ops| {
            // Pre-allocate keys and values for better performance
            let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
            let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

            b.iter(|| {
                let ledger = HyraScribeLedger::temp().unwrap();
                batched_put_operations(&ledger, &keys, &values, true).unwrap();
                ledger.flush().unwrap();
                black_box(&ledger);
            });
        });
    }

    group.finish();
}

fn benchmark_get_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_operations");
    group.measurement_time(Duration::from_secs(10));

    // Test different scales of operations
    for ops in [1, 10, 100, 1000, 5000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("get", ops), ops, |b, &ops| {
            // Pre-allocate keys for better performance
            let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
            let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

            // Pre-populate the database using optimized batching
            let ledger = HyraScribeLedger::temp().unwrap();
            populate_ledger(&ledger, &keys, &values, true).unwrap();

            b.iter(|| {
                batched_get_operations(&ledger, &keys).unwrap();
            });
        });
    }

    group.finish();
}

fn benchmark_mixed_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_operations");
    group.measurement_time(Duration::from_secs(10));

    // Test mixed put/get operations
    for ops in [1, 10, 100, 1000, 5000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("mixed", ops), ops, |b, &ops| {
            // Pre-allocate keys and values for better performance
            let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
            let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

            b.iter(|| {
                let ledger = HyraScribeLedger::temp().unwrap();
                batched_mixed_operations(&ledger, &keys, &values, true).unwrap();
                ledger.flush().unwrap();
                black_box(&ledger);
            });
        });
    }

    group.finish();
}

fn benchmark_throughput_put(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_put");
    group.throughput(criterion::Throughput::Elements(10000));

    group.bench_function("put_10000_ops", |b| {
        // Pre-allocate keys and values for better performance
        let keys: Vec<String> = (0..10000).map(|i| format!("key{}", i)).collect();
        let values: Vec<String> = (0..10000).map(|i| format!("value{}", i)).collect();

        b.iter(|| {
            let ledger = HyraScribeLedger::temp().unwrap();
            throughput_put_10k(&ledger, &keys, &values).unwrap();
            black_box(&ledger);
        });
    });

    group.finish();
}

fn benchmark_throughput_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_get");
    group.throughput(criterion::Throughput::Elements(10000));

    group.bench_function("get_10000_ops", |b| {
        // Pre-allocate keys and values for better performance
        let keys: Vec<String> = (0..10000).map(|i| format!("key{}", i)).collect();
        let values: Vec<String> = (0..10000).map(|i| format!("value{}", i)).collect();

        // Pre-populate the database using optimized batching
        let ledger = HyraScribeLedger::temp().unwrap();
        populate_ledger(&ledger, &keys, &values, true).unwrap();

        b.iter(|| {
            throughput_get_10k(&ledger, &keys).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_put_operations,
    benchmark_get_operations,
    benchmark_mixed_operations,
    benchmark_throughput_put,
    benchmark_throughput_get
);
criterion_main!(benches);
