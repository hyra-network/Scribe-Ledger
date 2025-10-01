use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};
use simple_scribe_ledger::SimpleScribeLedger;
use std::time::Duration;

// Simple HTTP benchmark that simulates actual HTTP server operations
// This includes JSON serialization/deserialization overhead that HTTP handlers perform

#[derive(Debug, Serialize, Deserialize)]
struct PutRequest {
    value: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GetResponse {
    value: Option<String>,
}

fn benchmark_simple_http_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_http_operations");
    group.measurement_time(Duration::from_secs(10));

    // Benchmark PUT operations (simulating HTTP PUT handlers with JSON serialization)
    for ops in [10, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("http_put_operations", ops),
            ops,
            |b, &ops| {
                // Pre-allocate test data
                let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
                let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

                b.iter(|| {
                    let ledger = SimpleScribeLedger::temp().unwrap();

                    // Simulate HTTP PUT operations with JSON overhead
                    for i in 0..ops {
                        // Simulate JSON deserialization of request
                        let request = PutRequest {
                            value: values[i].clone(),
                        };
                        let _json_request = serde_json::to_string(&request).unwrap();
                        let deserialized: PutRequest =
                            serde_json::from_str(&_json_request).unwrap();

                        // Perform database operation
                        ledger
                            .put(black_box(&keys[i]), black_box(&deserialized.value))
                            .unwrap();

                        // Simulate JSON serialization of response
                        let response = serde_json::json!({"status": "ok"});
                        let _json_response = serde_json::to_string(&response).unwrap();
                        black_box(_json_response);
                    }

                    ledger.flush().unwrap();
                    black_box(&ledger);
                });
            },
        );
    }

    // Benchmark GET operations (simulating HTTP GET handlers with JSON serialization)
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
                    // Simulate HTTP GET operations with JSON overhead
                    for key in &keys {
                        // Perform database operation
                        let result = ledger.get(black_box(key)).unwrap();

                        // Simulate JSON serialization of response
                        let value_str =
                            result.map(|bytes| String::from_utf8_lossy(&bytes).to_string());
                        let response = GetResponse { value: value_str };
                        let json_response = serde_json::to_string(&response).unwrap();
                        black_box(json_response);
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

                    // Simulate mixed HTTP operations (50% PUT, 50% GET) with JSON overhead
                    for i in 0..ops {
                        if i % 2 == 0 {
                            // Simulate HTTP PUT with JSON
                            let request = PutRequest {
                                value: values[i].clone(),
                            };
                            let _json_request = serde_json::to_string(&request).unwrap();
                            let deserialized: PutRequest =
                                serde_json::from_str(&_json_request).unwrap();

                            ledger
                                .put(black_box(&keys[i]), black_box(&deserialized.value))
                                .unwrap();

                            let response = serde_json::json!({"status": "ok"});
                            let _json_response = serde_json::to_string(&response).unwrap();
                            black_box(_json_response);
                        } else {
                            // Simulate HTTP GET with JSON
                            let result = ledger.get(black_box(&keys[i]));
                            if let Ok(Some(bytes)) = result {
                                let value_str = String::from_utf8_lossy(&bytes).to_string();
                                let response = GetResponse {
                                    value: Some(value_str),
                                };
                                let json_response = serde_json::to_string(&response).unwrap();
                                black_box(json_response);
                            }
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

                    // Simulate HTTP PUT with JSON and varying payload sizes
                    let request = PutRequest {
                        value: value.clone(),
                    };
                    let json_request = serde_json::to_string(&request).unwrap();
                    let deserialized: PutRequest = serde_json::from_str(&json_request).unwrap();

                    ledger
                        .put(black_box(key), black_box(&deserialized.value))
                        .unwrap();
                    ledger.flush().unwrap();

                    let response = serde_json::json!({"status": "ok"});
                    let json_response = serde_json::to_string(&response).unwrap();
                    black_box(json_response);

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
                    // Simulate HTTP GET with JSON and varying payload sizes
                    let result = ledger.get(black_box(key)).unwrap();
                    let value_str = result.map(|bytes| String::from_utf8_lossy(&bytes).to_string());
                    let response = GetResponse { value: value_str };
                    let json_response = serde_json::to_string(&response).unwrap();
                    black_box(json_response);
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
