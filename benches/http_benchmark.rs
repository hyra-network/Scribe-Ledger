use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
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
    // This represents the minimum CPU cost to prepare an HTTP PUT request body
    for ops in [10, 100, 500, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("put_json_serialization", ops), ops, |b, &ops| {
            let rt = Runtime::new().unwrap();

            // Pre-allocate reusable buffers for keys and values
            let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
            let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

            b.iter(|| {
                rt.block_on(async {
                    // JSON serialization overhead only (no network, no server)
                    for i in 0..ops {
                        let key = &keys[i];
                        let value = &values[i];

                        let _json_payload = serde_json::json!({"value": value});

                        black_box(key);
                        black_box(value);
                        black_box(_json_payload);
                    }
                });
            });
        });
    }

    group.finish();
}

fn benchmark_json_deserialization_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_deserialization_overhead");
    group.measurement_time(Duration::from_secs(10));

    // Measures JSON deserialization CPU overhead for GET operations
    // This represents the minimum CPU cost to parse an HTTP GET response
    for ops in [10, 100, 500, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("get_json_deserialization", ops), ops, |b, &ops| {
            let rt = Runtime::new().unwrap();

            // Pre-allocate reusable buffer for keys
            let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();

            b.iter(|| {
                rt.block_on(async {
                    // JSON response parsing overhead only (no network, no server)
                    for key in &keys {
                        let _json_response = serde_json::json!({"value": "some_value"});
                        black_box(key);
                        black_box(_json_response);
                    }
                });
            });
        });
    }

    group.finish();
}

// Comparison benchmark: Direct library vs JSON serialization overhead
fn benchmark_library_vs_json_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("library_vs_json_serialization");
    group.measurement_time(Duration::from_secs(10));

    // Direct library access (actual database operations)
    group.bench_function("direct_library_100_ops", |b| {
        use simple_scribe_ledger::SimpleScribeLedger;

        // Pre-allocate test data for better performance
        let keys: Vec<String> = (0..100).map(|i| format!("key{}", i)).collect();
        let values: Vec<String> = (0..100).map(|i| format!("value{}", i)).collect();

        b.iter(|| {
            let ledger = SimpleScribeLedger::temp().unwrap();

            // Warm-up phase
            let warmup_key = "warmup";
            let warmup_value = "warmup_value";
            ledger.put(warmup_key, warmup_value).unwrap();

            // Batch operations for optimal performance using pre-allocated data
            let mut batch = SimpleScribeLedger::new_batch();
            for i in 0..100 {
                batch.insert(keys[i].as_bytes(), values[i].as_bytes());
            }
            ledger.apply_batch(batch).unwrap();

            // Read them back using pre-allocated keys
            for key in &keys {
                let _result = ledger.get(black_box(key)).unwrap();
            }

            black_box(ledger);
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
                // JSON serialization/deserialization overhead (no database or network)
                for i in 0..100 {
                    let key = &keys[i];
                    let value = &values[i];

                    // Simulate JSON serialization overhead
                    let _json_payload = serde_json::json!({"value": value});

                    black_box(key);
                    black_box(_json_payload);
                }

                // Simulate GET operations with JSON response parsing
                for key in &keys {
                    let _json_response = serde_json::json!({"value": "some_value"});
                    black_box(key);
                    black_box(_json_response);
                }
            });
        });
    });

    group.finish();
}

// Benchmark measuring JSON serialization for large batch operations
// This shows the CPU overhead of preparing 10,000 operations for HTTP transmission
fn benchmark_json_serialization_10k_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialization_10k");
    group.measurement_time(Duration::from_secs(15)); // Longer measurement time for large tests

    group.bench_function("json_serialization_10000_ops", |b| {
        let rt = Runtime::new().unwrap();

        // Pre-allocate all test data to avoid allocation overhead
        let keys: Vec<String> = (0..10000).map(|i| format!("key{}", i)).collect();
        let values: Vec<String> = (0..10000).map(|i| format!("value{}", i)).collect();

        b.iter(|| {
            rt.block_on(async {
                // Batch JSON serialization for PUT operations (no network or database)
                let batch_size = 100;
                let mut i = 0;
                while i < 10000 {
                    let end = std::cmp::min(i + batch_size, 10000);

                    // JSON serialization only (no HTTP or database operations)
                    for j in i..end {
                        let key = &keys[j];
                        let value = &values[j];

                        // JSON serialization
                        let _json_payload = serde_json::json!({
                            "key": key,
                            "value": value
                        });

                        black_box(key);
                        black_box(value);
                        black_box(_json_payload);
                    }

                    i = end;
                }

                // Sample GET operations with JSON deserialization (no network or database)
                for i in (0..10000).step_by(10) {
                    let key = &keys[i];
                    let _json_response = serde_json::json!({"value": "some_value"});
                    black_box(key);
                    black_box(_json_response);
                }
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
