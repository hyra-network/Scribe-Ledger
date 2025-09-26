use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use simple_scribe_ledger::SimpleScribeLedger;
use std::time::Duration;

fn benchmark_put_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("put_operations");
    
    // Configure to run for at least 10 seconds and at most 60 seconds
    group.measurement_time(Duration::from_secs(10));
    
    // Test different scales of operations
    for ops in [1, 10, 100, 1000, 5000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("put", ops), ops, |b, &ops| {
            b.iter(|| {
                let ledger = SimpleScribeLedger::temp().unwrap();
                for i in 0..ops {
                    let key = format!("key{}", i);
                    let value = format!("value{}", i);
                    ledger.put(black_box(&key), black_box(&value)).unwrap();
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
            // Pre-populate the database
            let ledger = SimpleScribeLedger::temp().unwrap();
            for i in 0..ops {
                let key = format!("key{}", i);
                let value = format!("value{}", i);
                ledger.put(&key, &value).unwrap();
            }
            ledger.flush().unwrap();
            
            b.iter(|| {
                for i in 0..ops {
                    let key = format!("key{}", i);
                    let _result = ledger.get(black_box(&key)).unwrap();
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
            b.iter(|| {
                let ledger = SimpleScribeLedger::temp().unwrap();
                
                // Put operations
                for i in 0..(ops / 2) {
                    let key = format!("key{}", i);
                    let value = format!("value{}", i);
                    ledger.put(black_box(&key), black_box(&value)).unwrap();
                }
                
                // Get operations
                for i in 0..(ops / 2) {
                    let key = format!("key{}", i);
                    let _result = ledger.get(black_box(&key)).unwrap();
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
        b.iter(|| {
            let ledger = SimpleScribeLedger::temp().unwrap();
            for i in 0..10000 {
                let key = format!("key{}", i);
                let value = format!("value{}", i);
                ledger.put(black_box(&key), black_box(&value)).unwrap();
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
        // Pre-populate the database
        let ledger = SimpleScribeLedger::temp().unwrap();
        for i in 0..10000 {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            ledger.put(&key, &value).unwrap();
        }
        ledger.flush().unwrap();
        
        b.iter(|| {
            for i in 0..10000 {
                let key = format!("key{}", i);
                let _result = ledger.get(black_box(&key)).unwrap();
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