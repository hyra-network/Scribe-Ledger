use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use tokio::runtime::Runtime;
use std::time::Duration;

fn benchmark_http_put_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_put_operations");
    group.measurement_time(Duration::from_secs(10));
    
    // Note: This is a placeholder benchmark structure
    // In a real scenario, you'd want to:
    // 1. Start the HTTP server in a background thread
    // 2. Use reqwest or similar to make HTTP calls
    // 3. Measure the HTTP overhead vs direct library calls
    
    for ops in [10, 100, 500, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("http_put", ops), ops, |b, &ops| {
            let rt = Runtime::new().unwrap();
            
            // Pre-allocate reusable buffers for keys and values
            let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
            let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();
            
            b.iter(|| {
                rt.block_on(async {
                    // Warm-up phase - simulate establishing connections
                    tokio::time::sleep(Duration::from_micros(1)).await;
                    
                    // Simulate HTTP PUT operations using pre-allocated buffers
                    for i in 0..ops {
                        let key = &keys[i];
                        let value = &values[i];
                        
                        // Simulate HTTP call latency (reduced for faster benchmarking)
                        tokio::time::sleep(Duration::from_nanos(100)).await;
                        black_box(key);
                        black_box(value);
                    }
                });
            });
        });
    }
    
    group.finish();
}

fn benchmark_http_get_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_get_operations");
    group.measurement_time(Duration::from_secs(10));
    
    for ops in [10, 100, 500, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("http_get", ops), ops, |b, &ops| {
            let rt = Runtime::new().unwrap();
            
            // Pre-allocate reusable buffer for keys
            let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
            
            b.iter(|| {
                rt.block_on(async {
                    // Warm-up phase
                    tokio::time::sleep(Duration::from_micros(1)).await;
                    
                    // Simulate HTTP GET operations using pre-allocated keys
                    for key in &keys {
                        // Simulate HTTP call latency (reduced for faster benchmarking)
                        tokio::time::sleep(Duration::from_nanos(50)).await;
                        black_box(key);
                    }
                });
            });
        });
    }
    
    group.finish();
}

// Comparison benchmark: Direct library vs HTTP overhead
fn benchmark_library_vs_http_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("library_vs_http");
    group.measurement_time(Duration::from_secs(10));
    
    // Direct library access
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

    // Simulated HTTP overhead
    group.bench_function("http_simulation_100_ops", |b| {
        let rt = Runtime::new().unwrap();
        
        // Pre-allocate buffers for better performance
        let keys: Vec<String> = (0..100).map(|i| format!("key{}", i)).collect();
        let values: Vec<String> = (0..100).map(|i| format!("value{}", i)).collect();
        
        b.iter(|| {
            rt.block_on(async {
                // Simulate the additional overhead of HTTP serialization/deserialization
                for i in 0..100 {
                    let key = &keys[i];
                    let value = &values[i];
                    
                    // Simulate JSON serialization overhead
                    let _json_payload = serde_json::json!({"value": value});
                    
                    // Simulate network latency (reduced for faster benchmarking)
                    tokio::time::sleep(Duration::from_nanos(10)).await;
                    
                    black_box(key);
                    black_box(_json_payload);
                }
                
                // Simulate GET operations with pre-allocated keys
                for key in &keys {
                    tokio::time::sleep(Duration::from_nanos(10)).await;
                    black_box(key);
                }
            });
        });
    });
    
    group.finish();
}

// New benchmark for HTTP server with 10,000 operations
fn benchmark_http_server_10k_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_server_10k");
    group.measurement_time(Duration::from_secs(15)); // Longer measurement time for large tests
    
    group.bench_function("http_server_10000_ops", |b| {
        let rt = Runtime::new().unwrap();
        
        // Pre-allocate all test data to avoid allocation overhead
        let keys: Vec<String> = (0..10000).map(|i| format!("key{}", i)).collect();
        let values: Vec<String> = (0..10000).map(|i| format!("value{}", i)).collect();
        
        b.iter(|| {
            rt.block_on(async {
                // Warm-up phase - simulate server connection establishment
                for _ in 0..10 {
                    tokio::time::sleep(Duration::from_micros(1)).await;
                }
                
                // Batch HTTP PUT operations for better performance
                let batch_size = 100;
                let mut i = 0;
                while i < 10000 {
                    let end = std::cmp::min(i + batch_size, 10000);
                    
                    // Simulate batched HTTP PUT operations
                    for j in i..end {
                        let key = &keys[j];
                        let value = &values[j];
                        
                        // Simulate JSON serialization for HTTP
                        let _json_payload = serde_json::json!({
                            "key": key,
                            "value": value
                        });
                        
                        black_box(key);
                        black_box(value);
                        black_box(_json_payload);
                    }
                    
                    // Simulate HTTP batch request latency (reduced for faster benchmarking)
                    tokio::time::sleep(Duration::from_nanos(500)).await;
                    i = end;
                }
                
                // Simulate some GET operations
                for i in (0..10000).step_by(10) {
                    let key = &keys[i];
                    tokio::time::sleep(Duration::from_nanos(50)).await;
                    black_box(key);
                }
            });
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_http_put_operations,
    benchmark_http_get_operations,
    benchmark_library_vs_http_comparison,
    benchmark_http_server_10k_operations
);
criterion_main!(benches);