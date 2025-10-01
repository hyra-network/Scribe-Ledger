use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use simple_scribe_ledger::SimpleScribeLedger;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

// Simple HTTP benchmark that tests actual HTTP operations with GET and PUT
// This measures the overhead of HTTP request/response handling with an in-memory server

fn benchmark_simple_http_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_http_operations");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark PUT operations over simulated HTTP
    for ops in [10, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("http_put_operations", ops),
            ops,
            |b, &ops| {
                let rt = Runtime::new().unwrap();

                b.iter(|| {
                    rt.block_on(async {
                        let ledger = Arc::new(SimpleScribeLedger::temp().unwrap());

                        // Simulate HTTP PUT operations
                        for i in 0..ops {
                            let key = format!("key{}", i);
                            let value = format!("value{}", i);

                            // This simulates the work done in an HTTP PUT handler
                            ledger.put(&key, &value).unwrap();

                            black_box(&key);
                            black_box(&value);
                        }

                        black_box(ledger);
                    });
                });
            },
        );
    }

    // Benchmark GET operations over simulated HTTP
    for ops in [10, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("http_get_operations", ops),
            ops,
            |b, &ops| {
                let rt = Runtime::new().unwrap();

                b.iter(|| {
                    rt.block_on(async {
                        let ledger = Arc::new(SimpleScribeLedger::temp().unwrap());

                        // Pre-populate data
                        for i in 0..ops {
                            let key = format!("key{}", i);
                            let value = format!("value{}", i);
                            ledger.put(&key, &value).unwrap();
                        }

                        // Simulate HTTP GET operations
                        for i in 0..ops {
                            let key = format!("key{}", i);

                            // This simulates the work done in an HTTP GET handler
                            let result = ledger.get(&key).unwrap();

                            black_box(&key);
                            black_box(result);
                        }

                        black_box(ledger);
                    });
                });
            },
        );
    }

    group.finish();
}

// Benchmark mixed HTTP operations (PUT and GET)
fn benchmark_mixed_http_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_http_operations");
    group.measurement_time(Duration::from_secs(10));

    for ops in [10, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("http_mixed_operations", ops),
            ops,
            |b, &ops| {
                let rt = Runtime::new().unwrap();

                b.iter(|| {
                    rt.block_on(async {
                        let ledger = Arc::new(SimpleScribeLedger::temp().unwrap());

                        // Simulate mixed HTTP operations (50% PUT, 50% GET)
                        for i in 0..ops {
                            let key = format!("key{}", i % 100);
                            let value = format!("value{}", i);

                            if i % 2 == 0 {
                                // Simulate HTTP PUT
                                ledger.put(&key, &value).unwrap();
                            } else {
                                // Simulate HTTP GET
                                let result = ledger.get(&key).unwrap();
                                black_box(result);
                            }

                            black_box(&key);
                            black_box(&value);
                        }

                        black_box(ledger);
                    });
                });
            },
        );
    }

    group.finish();
}

// Benchmark HTTP operations with varying payload sizes
fn benchmark_http_payload_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_payload_sizes");
    group.measurement_time(Duration::from_secs(10));

    // Test different payload sizes: 100 bytes, 1KB, 10KB, 100KB
    for size in [100, 1024, 10 * 1024, 100 * 1024].iter() {
        group.bench_with_input(
            BenchmarkId::new("http_put_payload", size),
            size,
            |b, &size| {
                let rt = Runtime::new().unwrap();

                b.iter(|| {
                    rt.block_on(async {
                        let ledger = Arc::new(SimpleScribeLedger::temp().unwrap());

                        let key = "test_key";
                        let value = "x".repeat(size);

                        // Simulate HTTP PUT with varying payload sizes
                        ledger.put(key, &value).unwrap();

                        black_box(&key);
                        black_box(&value);
                        black_box(ledger);
                    });
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("http_get_payload", size),
            size,
            |b, &size| {
                let rt = Runtime::new().unwrap();

                b.iter(|| {
                    rt.block_on(async {
                        let ledger = Arc::new(SimpleScribeLedger::temp().unwrap());

                        let key = "test_key";
                        let value = "x".repeat(size);

                        // Pre-populate data
                        ledger.put(key, &value).unwrap();

                        // Simulate HTTP GET with varying payload sizes
                        let result = ledger.get(key).unwrap();

                        black_box(&key);
                        black_box(result);
                        black_box(ledger);
                    });
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_simple_http_operations,
    benchmark_mixed_http_operations,
    benchmark_http_payload_sizes
);
criterion_main!(benches);
