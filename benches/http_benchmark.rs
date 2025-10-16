use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use hyra_scribe_ledger::json_ops::{
    batched_json_get_deserialization, batched_json_put_serialization, combined_json_operations,
    large_scale_json_serialization,
};
use std::time::Duration;
use tokio::runtime::Runtime;

// NOTE: These benchmarks measure JSON serialization overhead only, NOT actual HTTP network operations.
// They are useful for understanding the CPU overhead of preparing HTTP requests/responses,
// but do not include actual network latency, HTTP server processing, or database operations.
//
// For realistic HTTP benchmarks, use integration tests with an actual HTTP server running.

fn benchmark_json_serialization_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialization_overhead");
    group.measurement_time(Duration::from_secs(10));

    // Measures JSON serialization CPU overhead for PUT operations
    for ops in [10, 100, 500, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("put_json_serialization", ops),
            ops,
            |b, &ops| {
                let rt = Runtime::new().unwrap();

                // Pre-allocate reusable buffers for keys and values
                let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
                let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

                b.iter(|| {
                    rt.block_on(async {
                        let result = batched_json_put_serialization(&keys, &values);
                        black_box(result);
                    });
                });
            },
        );
    }

    group.finish();
}

fn benchmark_json_deserialization_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_deserialization_overhead");
    group.measurement_time(Duration::from_secs(10));

    // Measures JSON deserialization CPU overhead for GET operations
    for ops in [10, 100, 500, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("get_json_deserialization", ops),
            ops,
            |b, &ops| {
                let rt = Runtime::new().unwrap();

                // Pre-allocate reusable buffer for keys
                let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();

                b.iter(|| {
                    rt.block_on(async {
                        let result = batched_json_get_deserialization(&keys);
                        black_box(result);
                    });
                });
            },
        );
    }

    group.finish();
}

// Comparison benchmark: Direct library vs JSON serialization overhead
fn benchmark_library_vs_json_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("library_vs_json_serialization");
    group.measurement_time(Duration::from_secs(10));

    // Direct library access (actual database operations)
    group.bench_function("direct_library_100_ops", |b| {
        use hyra_scribe_ledger::storage_ops::batched_put_operations;
        use hyra_scribe_ledger::HyraScribeLedger;

        // Pre-allocate test data for better performance
        let keys: Vec<String> = (0..100).map(|i| format!("key{}", i)).collect();
        let values: Vec<String> = (0..100).map(|i| format!("value{}", i)).collect();

        b.iter(|| {
            let ledger = HyraScribeLedger::temp().unwrap();
            batched_put_operations(&ledger, &keys, &values, true).unwrap();

            // Read them back
            for key in &keys {
                let _result = ledger.get(black_box(key)).unwrap();
            }

            black_box(&ledger);
        });
    });

    // JSON serialization overhead only (no actual database or network operations)
    group.bench_function("json_serialization_100_ops", |b| {
        let rt = Runtime::new().unwrap();

        // Pre-allocate buffers for better performance
        let keys: Vec<String> = (0..100).map(|i| format!("key{}", i)).collect();
        let values: Vec<String> = (0..100).map(|i| format!("value{}", i)).collect();

        b.iter(|| {
            rt.block_on(async {
                let result = combined_json_operations(&keys, &values);
                black_box(result);
            });
        });
    });

    group.finish();
}

// Benchmark measuring JSON serialization for large batch operations
fn benchmark_json_serialization_10k_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialization_10k");
    group.measurement_time(Duration::from_secs(15));

    group.bench_function("json_serialization_10000_ops", |b| {
        let rt = Runtime::new().unwrap();

        // Pre-allocate all test data to avoid allocation overhead
        let keys: Vec<String> = (0..10000).map(|i| format!("key{}", i)).collect();
        let values: Vec<String> = (0..10000).map(|i| format!("value{}", i)).collect();

        b.iter(|| {
            rt.block_on(async {
                let result = large_scale_json_serialization(&keys, &values);
                black_box(result);
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_json_serialization_overhead,
    benchmark_json_deserialization_overhead,
    benchmark_library_vs_json_comparison,
    benchmark_json_serialization_10k_operations
);
criterion_main!(benches);
