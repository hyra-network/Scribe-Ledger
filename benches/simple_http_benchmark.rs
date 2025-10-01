use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};
use simple_scribe_ledger::SimpleScribeLedger;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

// Simple HTTP benchmark that simulates actual HTTP server operations
// This includes JSON serialization/deserialization overhead and HTTP processing that handlers perform

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

    // Benchmark PUT operations (simulating HTTP PUT handlers with full HTTP overhead)
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

                    // Warmup phase to match storage benchmark
                    ledger.put("warmup", "warmup_value").unwrap();

                    // Simulate HTTP PUT operations with full HTTP overhead
                    for i in 0..ops {
                        // Simulate HTTP request parsing (URL parsing, header parsing, routing, etc.)
                        let _method = "PUT";
                        let path = format!("/api/v1/data/{}", keys[i]);
                        let query_params = format!("timestamp={}&client_id=test", i);

                        // Simulate request hash computation for caching/routing
                        let mut hasher = DefaultHasher::new();
                        path.hash(&mut hasher);
                        query_params.hash(&mut hasher);
                        let _request_hash = hasher.finish();

                        let _content_type = "application/json; charset=utf-8";
                        let _accept = "application/json";
                        let _user_agent = "benchmark-client/1.0";
                        let _authorization = "Bearer test_token_12345";
                        black_box(&path);
                        black_box(&query_params);

                        // Simulate JSON deserialization of request body with validation
                        let request = PutRequest {
                            value: values[i].clone(),
                        };
                        let json_request = serde_json::to_string(&request).unwrap();

                        // Simulate HTTP framework overhead (parsing, validation, middleware)
                        let deserialized: PutRequest = serde_json::from_str(&json_request).unwrap();

                        // Simulate request validation and processing
                        let _is_valid = !deserialized.value.is_empty();
                        let _value_len = deserialized.value.len();
                        black_box(&deserialized);

                        // Perform database operation
                        ledger
                            .put(black_box(&keys[i]), black_box(&deserialized.value))
                            .unwrap();

                        // Simulate JSON serialization of response with metadata
                        let timestamp = format!("{}", i);
                        let response = serde_json::json!({
                            "status": "success",
                            "message": "Value stored successfully",
                            "key": &keys[i],
                            "timestamp": &timestamp,
                            "version": "1.0"
                        });
                        let json_response = serde_json::to_string(&response).unwrap();

                        // Simulate HTTP response preparation with multiple headers
                        let _status_code = 200;
                        let content_length = json_response.len().to_string();
                        let request_id = format!("req-{}", i);
                        let _response_headers = vec![
                            ("Content-Type", "application/json; charset=utf-8"),
                            ("Content-Length", content_length.as_str()),
                            ("X-Request-ID", request_id.as_str()),
                            ("X-Response-Time", "1ms"),
                            ("Cache-Control", "no-cache"),
                        ];

                        // Simulate response hash for caching
                        let mut resp_hasher = DefaultHasher::new();
                        json_response.hash(&mut resp_hasher);
                        let _response_hash = resp_hasher.finish();

                        black_box(&json_response);
                        black_box(&_response_headers);
                    }

                    ledger.flush().unwrap();
                    black_box(&ledger);
                });
            },
        );
    }

    // Benchmark GET operations (simulating HTTP GET handlers with full HTTP overhead)
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
                    // Simulate HTTP GET operations with full HTTP overhead
                    for key in &keys {
                        // Simulate HTTP request parsing (header parsing, routing, etc.)
                        let _path = format!("/data/{}", key);
                        let _accept = "application/json";

                        // Perform database operation
                        let result = ledger.get(black_box(key)).unwrap();

                        // Simulate JSON serialization of response
                        let value_str =
                            result.map(|bytes| String::from_utf8_lossy(&bytes).to_string());
                        let response = GetResponse { value: value_str };
                        let json_response = serde_json::to_string(&response).unwrap();

                        // Simulate HTTP response preparation
                        let _status_code = 200;
                        let _response_headers = vec![
                            ("Content-Type", "application/json"),
                            ("Content-Length", &json_response.len().to_string()),
                        ];
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

                    // Warmup phase
                    ledger.put("warmup", "warmup_value").unwrap();

                    // Simulate mixed HTTP operations (50% PUT, 50% GET) with full HTTP overhead
                    for i in 0..ops {
                        if i % 2 == 0 {
                            // Simulate HTTP PUT with full overhead
                            let _path = format!("/data/{}", keys[i]);
                            let _content_type = "application/json";

                            let request = PutRequest {
                                value: values[i].clone(),
                            };
                            let json_request = serde_json::to_string(&request).unwrap();
                            let deserialized: PutRequest =
                                serde_json::from_str(&json_request).unwrap();

                            ledger
                                .put(black_box(&keys[i]), black_box(&deserialized.value))
                                .unwrap();

                            let response = serde_json::json!({
                                "status": "ok",
                                "key": &keys[i]
                            });
                            let json_response = serde_json::to_string(&response).unwrap();
                            let _status_code = 200;
                            black_box(json_response);
                        } else {
                            // Simulate HTTP GET with full overhead
                            let _path = format!("/data/{}", keys[i]);
                            let _accept = "application/json";

                            let result = ledger.get(black_box(&keys[i]));
                            if let Ok(Some(bytes)) = result {
                                let value_str = String::from_utf8_lossy(&bytes).to_string();
                                let response = GetResponse {
                                    value: Some(value_str),
                                };
                                let json_response = serde_json::to_string(&response).unwrap();
                                let _status_code = 200;
                                let _response_headers = vec![
                                    ("Content-Type", "application/json"),
                                    ("Content-Length", &json_response.len().to_string()),
                                ];
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
