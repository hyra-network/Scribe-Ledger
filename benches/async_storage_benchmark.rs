use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use simple_scribe_ledger::async_storage_ops::{
    batched_async_put_operations, batched_async_get_operations,
    batched_async_mixed_operations, populate_async_storage, concurrent_async_operations,
};
use simple_scribe_ledger::storage::{SledStorage, StorageBackend};
use std::time::Duration;
use tokio::runtime::Runtime;

fn benchmark_async_put_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("async_put_operations");
    group.measurement_time(Duration::from_secs(10));

    // Test different scales of operations
    for ops in [10, 100, 1000, 5000].iter() {
        group.bench_with_input(BenchmarkId::new("put", ops), ops, |b, &ops| {
            // Pre-allocate keys and values
            let keys: Vec<Vec<u8>> = (0..ops).map(|i| format!("key{}", i).into_bytes()).collect();
            let values: Vec<Vec<u8>> = (0..ops).map(|i| format!("value{}", i).into_bytes()).collect();

            b.iter(|| {
                rt.block_on(async {
                    let storage = SledStorage::temp().unwrap();
                    batched_async_put_operations(&storage, &keys, &values).await.unwrap();
                    black_box(&storage);
                });
            });
        });
    }

    group.finish();
}

fn benchmark_async_get_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("async_get_operations");
    group.measurement_time(Duration::from_secs(10));

    // Test different scales of operations
    for ops in [10, 100, 1000, 5000].iter() {
        group.bench_with_input(BenchmarkId::new("get", ops), ops, |b, &ops| {
            // Pre-allocate and populate data
            let keys: Vec<Vec<u8>> = (0..ops).map(|i| format!("key{}", i).into_bytes()).collect();
            let values: Vec<Vec<u8>> = (0..ops).map(|i| format!("value{}", i).into_bytes()).collect();
            
            let storage = rt.block_on(async {
                let storage = SledStorage::temp().unwrap();
                populate_async_storage(&storage, &keys, &values).await.unwrap();
                storage
            });

            b.iter(|| {
                rt.block_on(async {
                    batched_async_get_operations(&storage, &keys).await.unwrap();
                });
            });
        });
    }

    group.finish();
}

fn benchmark_async_mixed_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("async_mixed_operations");
    group.measurement_time(Duration::from_secs(10));

    // Test mixed put/get operations
    for ops in [10, 100, 1000, 5000].iter() {
        group.bench_with_input(BenchmarkId::new("mixed", ops), ops, |b, &ops| {
            // Pre-allocate keys and values
            let keys: Vec<Vec<u8>> = (0..ops).map(|i| format!("key{}", i).into_bytes()).collect();
            let values: Vec<Vec<u8>> = (0..ops).map(|i| format!("value{}", i).into_bytes()).collect();

            b.iter(|| {
                rt.block_on(async {
                    let storage = SledStorage::temp().unwrap();
                    batched_async_mixed_operations(&storage, &keys, &values).await.unwrap();
                    black_box(&storage);
                });
            });
        });
    }

    group.finish();
}

fn benchmark_async_snapshot(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("async_snapshot");

    // Configure to run for at least 10 seconds
    group.measurement_time(Duration::from_secs(10));

    // Test snapshot at different data sizes
    for entries in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("snapshot", entries),
            entries,
            |b, &entries| {
                // Pre-populate data
                let storage = rt.block_on(async {
                    let storage = SledStorage::temp().unwrap();
                    for i in 0..entries {
                        let key = format!("key{}", i).into_bytes();
                        let value = format!("value{}", i).into_bytes();
                        storage.put(key, value).await.unwrap();
                    }
                    storage.flush().await.unwrap();
                    storage
                });

                b.iter(|| {
                    rt.block_on(async {
                        let _snapshot = storage.snapshot().await.unwrap();
                    });
                });
            },
        );
    }

    group.finish();
}

fn benchmark_async_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("async_concurrent_operations");
    group.measurement_time(Duration::from_secs(10));

    // Test concurrent operations
    for concurrent in [5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent", concurrent),
            concurrent,
            |b, &concurrent| {
                b.iter(|| {
                    rt.block_on(async {
                        let storage = std::sync::Arc::new(SledStorage::temp().unwrap());
                        concurrent_async_operations(storage, concurrent).await.unwrap();
                    });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_async_put_operations,
    benchmark_async_get_operations,
    benchmark_async_mixed_operations,
    benchmark_async_snapshot,
    benchmark_async_concurrent_operations
);
criterion_main!(benches);
