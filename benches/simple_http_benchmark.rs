use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use simple_scribe_ledger::SimpleScribeLedger;
use std::time::Duration;

// Simple HTTP benchmark that tests actual database operations
// This simulates the work done by an HTTP server handling requests

fn benchmark_simple_http_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_http_operations");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark PUT operations (simulating HTTP PUT handlers)
    for ops in [10, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("http_put_operations", ops),
            ops,
            |b, &ops| {
                // Pre-allocate test data
                let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
                let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

                b.iter(|| {
                    // Create ledger once per iteration (simulating a request to the server)
                    let ledger = SimpleScribeLedger::temp().unwrap();

                    // Simulate HTTP PUT operations
                    for i in 0..ops {
                        ledger
                            .put(black_box(&keys[i]), black_box(&values[i]))
                            .unwrap();
                    }

                    ledger.flush().unwrap();
                    black_box(&ledger);
                });
            },
        );
    }

    // Benchmark GET operations (simulating HTTP GET handlers)
    for ops in [10, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("http_get_operations", ops),
            ops,
            |b, &ops| {
                // Pre-allocate test data
                let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
                let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

                // Create and populate ledger outside the benchmark iteration
                let ledger = SimpleScribeLedger::temp().unwrap();
                for i in 0..ops {
                    ledger.put(&keys[i], &values[i]).unwrap();
                }
                ledger.flush().unwrap();

                b.iter(|| {
                    // Simulate HTTP GET operations
                    for key in &keys {
                        let result = ledger.get(black_box(key)).unwrap();
                        black_box(result);
                    }
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
                // Pre-allocate test data
                let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i % 100)).collect();
                let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

                b.iter(|| {
                    let ledger = SimpleScribeLedger::temp().unwrap();

                    // Simulate mixed HTTP operations (50% PUT, 50% GET)
                    for i in 0..ops {
                        if i % 2 == 0 {
                            // Simulate HTTP PUT
                            ledger
                                .put(black_box(&keys[i]), black_box(&values[i]))
                                .unwrap();
                        } else {
                            // Simulate HTTP GET
                            let _ = ledger.get(black_box(&keys[i]));
                        }
                    }

                    ledger.flush().unwrap();
                    black_box(&ledger);
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
                let key = "test_key";
                let value = "x".repeat(size);

                b.iter(|| {
                    let ledger = SimpleScribeLedger::temp().unwrap();

                    // Simulate HTTP PUT with varying payload sizes
                    ledger.put(black_box(key), black_box(&value)).unwrap();
                    ledger.flush().unwrap();

                    black_box(&ledger);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("http_get_payload", size),
            size,
            |b, &size| {
                let key = "test_key";
                let value = "x".repeat(size);

                // Pre-populate data outside the benchmark iteration
                let ledger = SimpleScribeLedger::temp().unwrap();
                ledger.put(key, &value).unwrap();
                ledger.flush().unwrap();

                b.iter(|| {
                    // Simulate HTTP GET with varying payload sizes
                    let result = ledger.get(black_box(key)).unwrap();
                    black_box(result);
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
