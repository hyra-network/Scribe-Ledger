use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use simple_scribe_ledger::SimpleScribeLedger;
use std::time::Duration;

fn benchmark_put_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("put_operations");

    // Configure to run for at least 10 seconds and at most 60 seconds
    group.measurement_time(Duration::from_secs(10));

    // Test different scales of operations
    for ops in [1, 10, 100, 1000, 5000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("put", ops), ops, |b, &ops| {
            // Pre-allocate keys and values for better performance
            let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
            let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

            b.iter(|| {
                let ledger = SimpleScribeLedger::temp().unwrap();

                // Warm-up phase
                ledger.put("warmup", "value").unwrap();

                // Use batching for better performance when ops > 10
                if ops > 10 {
                    let batch_size = std::cmp::min(100, ops / 4);
                    let mut i = 0;
                    while i < ops {
                        let mut batch = SimpleScribeLedger::new_batch();
                        let end = std::cmp::min(i + batch_size, ops);

                        for j in i..end {
                            batch.insert(keys[j].as_bytes(), values[j].as_bytes());
                        }

                        ledger.apply_batch(batch).unwrap();
                        i = end;
                    }
                } else {
                    // For small operations, use individual puts
                    for i in 0..ops {
                        ledger
                            .put(black_box(&keys[i]), black_box(&values[i]))
                            .unwrap();
                    }
                }

                ledger.flush().unwrap();
            });
        });
    }

    group.finish();
}

fn benchmark_get_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_operations");

    // Configure to run for at least 10 seconds and at most 60 seconds
    group.measurement_time(Duration::from_secs(10));

    // Test different scales of operations
    for ops in [1, 10, 100, 1000, 5000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("get", ops), ops, |b, &ops| {
            // Pre-allocate keys for better performance
            let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
            let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

            // Pre-populate the database using batching for better setup performance
            let ledger = SimpleScribeLedger::temp().unwrap();

            // Warm-up phase
            ledger.put("warmup", "value").unwrap();

            if ops > 10 {
                let batch_size = std::cmp::min(100, ops / 4);
                let mut i = 0;
                while i < ops {
                    let mut batch = SimpleScribeLedger::new_batch();
                    let end = std::cmp::min(i + batch_size, ops);

                    for j in i..end {
                        batch.insert(keys[j].as_bytes(), values[j].as_bytes());
                    }

                    ledger.apply_batch(batch).unwrap();
                    i = end;
                }
            } else {
                for i in 0..ops {
                    ledger.put(&keys[i], &values[i]).unwrap();
                }
            }

            ledger.flush().unwrap();

            b.iter(|| {
                for key in &keys {
                    let _result = ledger.get(black_box(key)).unwrap();
                }
            });
        });
    }

    group.finish();
}

fn benchmark_mixed_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_operations");

    // Configure to run for at least 10 seconds and at most 60 seconds
    group.measurement_time(Duration::from_secs(10));

    // Test mixed put/get operations
    for ops in [1, 10, 100, 1000, 5000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("mixed", ops), ops, |b, &ops| {
            // Pre-allocate keys and values for better performance
            let keys: Vec<String> = (0..ops).map(|i| format!("key{}", i)).collect();
            let values: Vec<String> = (0..ops).map(|i| format!("value{}", i)).collect();

            b.iter(|| {
                let ledger = SimpleScribeLedger::temp().unwrap();

                // Warm-up phase
                ledger.put("warmup", "value").unwrap();

                // Put operations (first half)
                let put_ops = ops / 2;
                if put_ops > 10 {
                    let batch_size = std::cmp::min(50, put_ops / 4);
                    let mut i = 0;
                    while i < put_ops {
                        let mut batch = SimpleScribeLedger::new_batch();
                        let end = std::cmp::min(i + batch_size, put_ops);

                        for j in i..end {
                            batch.insert(keys[j].as_bytes(), values[j].as_bytes());
                        }

                        ledger.apply_batch(batch).unwrap();
                        i = end;
                    }
                } else {
                    for i in 0..put_ops {
                        ledger
                            .put(black_box(&keys[i]), black_box(&values[i]))
                            .unwrap();
                    }
                }

                // Get operations (using pre-allocated keys)
                for i in 0..put_ops {
                    let _result = ledger.get(black_box(&keys[i])).unwrap();
                }

                ledger.flush().unwrap();
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
            let ledger = SimpleScribeLedger::temp().unwrap();

            // Warm-up phase
            ledger.put("warmup", "value").unwrap();

            // Use batching for optimal performance
            let batch_size = 100;
            let mut i = 0;
            while i < 10000 {
                let mut batch = SimpleScribeLedger::new_batch();
                let end = std::cmp::min(i + batch_size, 10000);

                for j in i..end {
                    batch.insert(keys[j].as_bytes(), values[j].as_bytes());
                }

                ledger.apply_batch(batch).unwrap();
                i = end;
            }

            ledger.flush().unwrap();
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

        // Pre-populate the database using batching
        let ledger = SimpleScribeLedger::temp().unwrap();

        // Warm-up phase
        ledger.put("warmup", "value").unwrap();

        // Use batching for optimal setup performance
        let batch_size = 100;
        let mut i = 0;
        while i < 10000 {
            let mut batch = SimpleScribeLedger::new_batch();
            let end = std::cmp::min(i + batch_size, 10000);

            for j in i..end {
                batch.insert(keys[j].as_bytes(), values[j].as_bytes());
            }

            ledger.apply_batch(batch).unwrap();
            i = end;
        }
        ledger.flush().unwrap();

        b.iter(|| {
            for key in &keys {
                let _result = ledger.get(black_box(key)).unwrap();
            }
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
