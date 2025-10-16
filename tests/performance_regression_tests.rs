//! Performance regression tests
//!
//! These tests ensure that performance doesn't degrade below acceptable thresholds.
//! If any of these tests fail, it indicates a performance regression in the code.

use hyra_scribe_ledger::HyraScribeLedger;
use std::time::Instant;

/// Performance thresholds - if operations take longer than these, tests fail
const PUT_1_OPS_MAX_MS: u128 = 10; // 1 put operation should take < 10ms
const PUT_10_OPS_MAX_MS: u128 = 50; // 10 put operations should take < 50ms
const PUT_100_OPS_MAX_MS: u128 = 200; // 100 put operations should take < 200ms
const PUT_1000_OPS_MAX_MS: u128 = 2000; // 1000 put operations should take < 2s

const GET_1_OPS_MAX_MICROS: u128 = 1000; // 1 get operation should take < 1ms
const GET_10_OPS_MAX_MICROS: u128 = 5000; // 10 get operations should take < 5ms
const GET_100_OPS_MAX_MICROS: u128 = 50000; // 100 get operations should take < 50ms
const GET_1000_OPS_MAX_MICROS: u128 = 500000; // 1000 get operations should take < 500ms

const MIXED_100_OPS_MAX_MS: u128 = 300; // 100 mixed operations should take < 300ms
const MIXED_1000_OPS_MAX_MS: u128 = 3000; // 1000 mixed operations should take < 3s

#[test]
fn test_put_performance_1_ops() {
    let ledger = HyraScribeLedger::temp().unwrap();

    let start = Instant::now();
    ledger.put("test_key", "test_value").unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < PUT_1_OPS_MAX_MS,
        "PUT 1 operation took {}ms, threshold is {}ms - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_millis(),
        PUT_1_OPS_MAX_MS
    );
}

#[test]
fn test_put_performance_10_ops() {
    let ledger = HyraScribeLedger::temp().unwrap();

    let start = Instant::now();
    for i in 0..10 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < PUT_10_OPS_MAX_MS,
        "PUT 10 operations took {}ms, threshold is {}ms - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_millis(),
        PUT_10_OPS_MAX_MS
    );
}

#[test]
fn test_put_performance_100_ops() {
    let ledger = HyraScribeLedger::temp().unwrap();

    let start = Instant::now();
    for i in 0..100 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < PUT_100_OPS_MAX_MS,
        "PUT 100 operations took {}ms, threshold is {}ms - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_millis(),
        PUT_100_OPS_MAX_MS
    );
}

#[test]
fn test_put_performance_1000_ops() {
    let ledger = HyraScribeLedger::temp().unwrap();

    let start = Instant::now();
    for i in 0..1000 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < PUT_1000_OPS_MAX_MS,
        "PUT 1000 operations took {}ms, threshold is {}ms - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_millis(),
        PUT_1000_OPS_MAX_MS
    );
}

#[test]
fn test_get_performance_1_ops() {
    let ledger = HyraScribeLedger::temp().unwrap();
    ledger.put("test_key", "test_value").unwrap();

    let start = Instant::now();
    let _result = ledger.get("test_key").unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_micros() < GET_1_OPS_MAX_MICROS,
        "GET 1 operation took {}µs, threshold is {}µs - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_micros(),
        GET_1_OPS_MAX_MICROS
    );
}

#[test]
fn test_get_performance_10_ops() {
    let ledger = HyraScribeLedger::temp().unwrap();

    // Pre-populate data
    for i in 0..10 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }

    let start = Instant::now();
    for i in 0..10 {
        let _result = ledger.get(format!("key{}", i)).unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_micros() < GET_10_OPS_MAX_MICROS,
        "GET 10 operations took {}µs, threshold is {}µs - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_micros(),
        GET_10_OPS_MAX_MICROS
    );
}

#[test]
fn test_get_performance_100_ops() {
    let ledger = HyraScribeLedger::temp().unwrap();

    // Pre-populate data
    for i in 0..100 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }

    let start = Instant::now();
    for i in 0..100 {
        let _result = ledger.get(format!("key{}", i)).unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_micros() < GET_100_OPS_MAX_MICROS,
        "GET 100 operations took {}µs, threshold is {}µs - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_micros(),
        GET_100_OPS_MAX_MICROS
    );
}

#[test]
fn test_get_performance_1000_ops() {
    let ledger = HyraScribeLedger::temp().unwrap();

    // Pre-populate data
    for i in 0..1000 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }

    let start = Instant::now();
    for i in 0..1000 {
        let _result = ledger.get(format!("key{}", i)).unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_micros() < GET_1000_OPS_MAX_MICROS,
        "GET 1000 operations took {}µs, threshold is {}µs - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_micros(),
        GET_1000_OPS_MAX_MICROS
    );
}

#[test]
fn test_mixed_performance_100_ops() {
    let ledger = HyraScribeLedger::temp().unwrap();

    let start = Instant::now();
    for i in 0..50 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }
    for i in 0..50 {
        let _result = ledger.get(format!("key{}", i)).unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < MIXED_100_OPS_MAX_MS,
        "MIXED 100 operations took {}ms, threshold is {}ms - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_millis(),
        MIXED_100_OPS_MAX_MS
    );
}

#[test]
fn test_mixed_performance_1000_ops() {
    let ledger = HyraScribeLedger::temp().unwrap();

    let start = Instant::now();
    for i in 0..500 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }
    for i in 0..500 {
        let _result = ledger.get(format!("key{}", i)).unwrap();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < MIXED_1000_OPS_MAX_MS,
        "MIXED 1000 operations took {}ms, threshold is {}ms - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_millis(),
        MIXED_1000_OPS_MAX_MS
    );
}

#[test]
fn test_segment_manager_performance() {
    use hyra_scribe_ledger::storage::segment::SegmentManager;

    let manager = SegmentManager::with_threshold(1024 * 1024); // 1MB threshold

    // Test put performance
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("key{}", i).into_bytes();
        let value = format!("value{}", i).into_bytes();
        manager.put(key, value).unwrap();
    }
    let put_elapsed = start.elapsed();

    assert!(
        put_elapsed.as_millis() < 100,
        "SegmentManager PUT 100 operations took {}ms, threshold is 100ms - PERFORMANCE REGRESSION DETECTED",
        put_elapsed.as_millis()
    );

    // Test get performance
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("key{}", i).into_bytes();
        let _result = manager.get(&key).unwrap();
    }
    let get_elapsed = start.elapsed();

    assert!(
        get_elapsed.as_millis() < 50,
        "SegmentManager GET 100 operations took {}ms, threshold is 50ms - PERFORMANCE REGRESSION DETECTED",
        get_elapsed.as_millis()
    );
}

#[test]
fn test_storage_backend_performance() {
    use hyra_scribe_ledger::storage::SledStorage;
    use hyra_scribe_ledger::storage::StorageBackend;

    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let storage = SledStorage::temp().unwrap();

        // Test async put performance
        let start = Instant::now();
        for i in 0..100 {
            let key = format!("key{}", i).into_bytes();
            let value = format!("value{}", i).into_bytes();
            storage.put(key, value).await.unwrap();
        }
        let put_elapsed = start.elapsed();

        assert!(
            put_elapsed.as_millis() < 200,
            "StorageBackend PUT 100 operations took {}ms, threshold is 200ms - PERFORMANCE REGRESSION DETECTED",
            put_elapsed.as_millis()
        );

        // Test async get performance
        let start = Instant::now();
        for i in 0..100 {
            let key = format!("key{}", i).into_bytes();
            let _result = storage.get(&key).await.unwrap();
        }
        let get_elapsed = start.elapsed();

        assert!(
            get_elapsed.as_millis() < 100,
            "StorageBackend GET 100 operations took {}ms, threshold is 100ms - PERFORMANCE REGRESSION DETECTED",
            get_elapsed.as_millis()
        );
    });
}

#[test]
fn test_flush_performance() {
    let ledger = HyraScribeLedger::temp().unwrap();

    // Add some data
    for i in 0..100 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }

    // Test flush performance
    let start = Instant::now();
    ledger.flush().unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 100,
        "FLUSH operation took {}ms, threshold is 100ms - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_millis()
    );
}

#[test]
fn test_clear_performance() {
    let ledger = HyraScribeLedger::temp().unwrap();

    // Add some data
    for i in 0..1000 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }

    // Test clear performance
    let start = Instant::now();
    ledger.clear().unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 500,
        "CLEAR operation took {}ms, threshold is 500ms - PERFORMANCE REGRESSION DETECTED",
        elapsed.as_millis()
    );
}
