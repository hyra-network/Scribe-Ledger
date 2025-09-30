use anyhow::Result;
use simple_scribe_ledger::SimpleScribeLedger;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

/// Test sled's concurrent read/write capabilities
#[test]
fn test_sled_concurrent_read_write() -> Result<()> {
    let ledger = Arc::new(SimpleScribeLedger::temp()?);
    let barrier = Arc::new(Barrier::new(5)); // 4 threads + main thread
    let mut handles = vec![];

    // Writer thread 1
    {
        let ledger = Arc::clone(&ledger);
        let barrier = Arc::clone(&barrier);
        let handle = thread::spawn(move || -> Result<()> {
            barrier.wait();
            for i in 0..1000 {
                let key = format!("writer1_key_{}", i);
                let value = format!("writer1_value_{}", i);
                ledger.put(&key, &value)?;
            }
            Ok(())
        });
        handles.push(handle);
    }

    // Writer thread 2
    {
        let ledger = Arc::clone(&ledger);
        let barrier = Arc::clone(&barrier);
        let handle = thread::spawn(move || -> Result<()> {
            barrier.wait();
            for i in 0..1000 {
                let key = format!("writer2_key_{}", i);
                let value = format!("writer2_value_{}", i);
                ledger.put(&key, &value)?;
            }
            Ok(())
        });
        handles.push(handle);
    }

    // Reader thread 1
    {
        let ledger = Arc::clone(&ledger);
        let barrier = Arc::clone(&barrier);
        let handle = thread::spawn(move || -> Result<()> {
            barrier.wait();
            thread::sleep(Duration::from_millis(50)); // Let writers get started

            let mut successful_reads = 0;
            for i in 0..1000 {
                let key = format!("writer1_key_{}", i);
                if ledger.get(&key)?.is_some() {
                    successful_reads += 1;
                }
            }

            // We should be able to read at least some data
            assert!(successful_reads > 0);
            Ok(())
        });
        handles.push(handle);
    }

    // Reader thread 2
    {
        let ledger = Arc::clone(&ledger);
        let barrier = Arc::clone(&barrier);
        let handle = thread::spawn(move || -> Result<()> {
            barrier.wait();
            thread::sleep(Duration::from_millis(50)); // Let writers get started

            let mut successful_reads = 0;
            for i in 0..1000 {
                let key = format!("writer2_key_{}", i);
                if ledger.get(&key)?.is_some() {
                    successful_reads += 1;
                }
            }

            // We should be able to read at least some data
            assert!(successful_reads > 0);
            Ok(())
        });
        handles.push(handle);
    }

    // Signal all threads to start
    barrier.wait();

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap()?;
    }

    // Final verification
    assert_eq!(ledger.len(), 2000);

    // Verify data integrity
    for i in (0..1000).step_by(100) {
        let key1 = format!("writer1_key_{}", i);
        let value1 = ledger.get(&key1)?;
        assert_eq!(
            value1,
            Some(format!("writer1_value_{}", i).as_bytes().to_vec())
        );

        let key2 = format!("writer2_key_{}", i);
        let value2 = ledger.get(&key2)?;
        assert_eq!(
            value2,
            Some(format!("writer2_value_{}", i).as_bytes().to_vec())
        );
    }

    Ok(())
}

/// Test sled's performance under different workloads
#[test]
fn test_sled_performance_characteristics() -> Result<()> {
    let ledger = SimpleScribeLedger::temp()?;

    // Sequential write performance
    let start = Instant::now();
    for i in 0..10000 {
        let key = format!("seq_key_{:06}", i);
        let value = format!("seq_value_{:06}", i);
        ledger.put(&key, &value)?;
    }
    let sequential_write_time = start.elapsed();

    // Sequential read performance
    let start = Instant::now();
    for i in 0..10000 {
        let key = format!("seq_key_{:06}", i);
        let _ = ledger.get(&key)?;
    }
    let sequential_read_time = start.elapsed();

    // Random access performance
    let start = Instant::now();
    for i in (0..10000).rev() {
        let key = format!("seq_key_{:06}", i);
        let _ = ledger.get(&key)?;
    }
    let random_read_time = start.elapsed();

    println!("Sequential write: {:?}", sequential_write_time);
    println!("Sequential read: {:?}", sequential_read_time);
    println!("Random read: {:?}", random_read_time);

    // Performance assertions (should be reasonably fast)
    assert!(sequential_write_time < Duration::from_secs(5));
    assert!(sequential_read_time < Duration::from_secs(1));
    assert!(random_read_time < Duration::from_secs(2));

    Ok(())
}

/// Test sled's behavior with transactions and consistency
#[test]
fn test_sled_data_consistency() -> Result<()> {
    let ledger = SimpleScribeLedger::temp()?;

    // Create a scenario where we update related data
    ledger.put("account:alice:balance", "1000")?;
    ledger.put("account:bob:balance", "500")?;
    ledger.put("total_funds", "1500")?;

    // Simulate a transfer: Alice sends 200 to Bob
    let alice_balance: i32 =
        String::from_utf8_lossy(&ledger.get("account:alice:balance")?.unwrap())
            .parse()
            .unwrap();
    let bob_balance: i32 = String::from_utf8_lossy(&ledger.get("account:bob:balance")?.unwrap())
        .parse()
        .unwrap();
    let total_funds: i32 = String::from_utf8_lossy(&ledger.get("total_funds")?.unwrap())
        .parse()
        .unwrap();

    let transfer_amount = 200;

    // Update balances
    ledger.put(
        "account:alice:balance",
        &(alice_balance - transfer_amount).to_string(),
    )?;
    ledger.put(
        "account:bob:balance",
        &(bob_balance + transfer_amount).to_string(),
    )?;
    // Note: total_funds should remain the same

    ledger.flush()?;

    // Verify consistency
    let final_alice: i32 = String::from_utf8_lossy(&ledger.get("account:alice:balance")?.unwrap())
        .parse()
        .unwrap();
    let final_bob: i32 = String::from_utf8_lossy(&ledger.get("account:bob:balance")?.unwrap())
        .parse()
        .unwrap();
    let final_total: i32 = String::from_utf8_lossy(&ledger.get("total_funds")?.unwrap())
        .parse()
        .unwrap();

    assert_eq!(final_alice, 800);
    assert_eq!(final_bob, 700);
    assert_eq!(final_alice + final_bob, total_funds); // Conservation of funds
    assert_eq!(final_total, total_funds); // Total unchanged

    Ok(())
}

/// Test sled's durability guarantees
#[test]
fn test_sled_durability() -> Result<()> {
    use std::fs;
    use std::path::Path;

    // Use timestamp + thread ID to ensure unique path for each test run
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let thread_id = format!("{:?}", std::thread::current().id());
    let test_db = format!("./test_durability_db_{}_{}", timestamp, thread_id.replace("ThreadId", "").replace("(", "").replace(")", ""));

    // Clean up any existing test database
    if Path::new(&test_db).exists() {
        fs::remove_dir_all(&test_db).ok();
    }

    let test_data = vec![
        ("durable:key1", "This data must survive"),
        ("durable:key2", "This data is critical"),
        ("durable:key3", "This data is important"),
    ];

    // Phase 1: Write data and explicitly flush
    {
        let ledger = SimpleScribeLedger::new(&test_db)?;

        for (key, value) in &test_data {
            ledger.put(key, value)?;
        }

        // Explicit flush to ensure durability
        ledger.flush()?;
    } // Ledger is dropped here, simulating process termination

    // Phase 2: Verify data survived process restart
    {
        let ledger = SimpleScribeLedger::new(&test_db)?;
        assert_eq!(ledger.len(), test_data.len());

        for (key, expected_value) in &test_data {
            let stored_value = ledger.get(key)?;
            assert_eq!(stored_value, Some(expected_value.as_bytes().to_vec()));
        }
    }

    // Phase 3: Test durability without explicit flush
    {
        let ledger = SimpleScribeLedger::new(&test_db)?;

        // Add more data but rely on Drop trait for flushing
        ledger.put("auto_flush:key1", "Data written without explicit flush")?;
        ledger.put("auto_flush:key2", "Another piece of data")?;

        // Don't call flush explicitly - rely on Drop implementation
    } // Drop should flush automatically

    // Phase 4: Verify auto-flush worked
    {
        let ledger = SimpleScribeLedger::new(&test_db)?;
        assert_eq!(ledger.len(), test_data.len() + 2);

        let auto_data = ledger.get("auto_flush:key1")?;
        assert_eq!(
            auto_data,
            Some(b"Data written without explicit flush".to_vec())
        );
    }

    // Cleanup
    fs::remove_dir_all(&test_db).ok();
    Ok(())
}

/// Test sled's handling of different data patterns
#[test]
fn test_sled_data_patterns() -> Result<()> {
    let ledger = SimpleScribeLedger::temp()?;

    // Pattern 1: Hierarchical keys (simulating directory structure)
    let hierarchical_data = vec![
        ("users/active/alice", "Alice Smith"),
        ("users/active/bob", "Bob Johnson"),
        ("users/inactive/charlie", "Charlie Brown"),
        ("config/database/host", "localhost"),
        ("config/database/port", "5432"),
        ("config/app/name", "SimpleScribeLedger"),
        ("stats/daily/2024-01-01", "1000"),
        ("stats/daily/2024-01-02", "1200"),
    ];

    for (key, value) in &hierarchical_data {
        ledger.put(key, value)?;
    }

    // Verify hierarchical access
    for (key, expected_value) in &hierarchical_data {
        let stored_value = ledger.get(key)?;
        assert_eq!(stored_value, Some(expected_value.as_bytes().to_vec()));
    }

    // Pattern 2: Timestamp-based keys
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for i in 0..100 {
        let timestamp_key = format!("events:{}:{:03}", now + i, i);
        let event_data = format!(
            r#"{{"event_id": {}, "timestamp": {}, "data": "event_{}"}} "#,
            i,
            now + i,
            i
        );
        ledger.put(&timestamp_key, &event_data)?;
    }

    // Verify timestamp-based access
    let test_key = format!("events:{}:{:03}", now + 50, 50);
    let result = ledger.get(&test_key)?;
    assert!(result.is_some());
    let result_data = result.unwrap();
    let result_str = String::from_utf8_lossy(&result_data);
    assert!(result_str.contains("event_50"));

    // Pattern 3: Counter/sequence patterns
    for seq in 0..1000 {
        let seq_key = format!("seq:{:010}", seq); // Zero-padded for sorting
        let seq_value = format!("sequence_item_{}", seq);
        ledger.put(&seq_key, &seq_value)?;
    }

    // Verify sequence access
    let mid_seq_key = "seq:0000000500";
    let result = ledger.get(mid_seq_key)?;
    assert_eq!(result, Some(b"sequence_item_500".to_vec()));

    // Final count should include all patterns
    let expected_total = hierarchical_data.len() + 100 + 1000;
    assert_eq!(ledger.len(), expected_total);

    Ok(())
}

/// Test sled's memory efficiency with large datasets
#[test]
fn test_sled_memory_efficiency() -> Result<()> {
    let ledger = SimpleScribeLedger::temp()?;

    // Create dataset with varying value sizes
    let small_values = 5000;
    let medium_values = 1000;
    let large_values = 100;

    // Small values (100 bytes each)
    let small_data = "x".repeat(100);
    for i in 0..small_values {
        let key = format!("small:{:05}", i);
        ledger.put(&key, &small_data)?;
    }

    // Medium values (10KB each)
    let medium_data = "y".repeat(10000);
    for i in 0..medium_values {
        let key = format!("medium:{:05}", i);
        ledger.put(&key, &medium_data)?;
    }

    // Large values (100KB each)
    let large_data = "z".repeat(100000);
    for i in 0..large_values {
        let key = format!("large:{:05}", i);
        ledger.put(&key, &large_data)?;
    }

    let total_items = small_values + medium_values + large_values;
    assert_eq!(ledger.len(), total_items);

    // Verify we can still access all data efficiently
    // Random access to verify no memory issues
    for i in (0..small_values).step_by(500) {
        let key = format!("small:{:05}", i);
        let result = ledger.get(&key)?;
        assert_eq!(result, Some(small_data.as_bytes().to_vec()));
    }

    for i in (0..medium_values).step_by(100) {
        let key = format!("medium:{:05}", i);
        let result = ledger.get(&key)?;
        assert_eq!(result, Some(medium_data.as_bytes().to_vec()));
    }

    for i in (0..large_values).step_by(10) {
        let key = format!("large:{:05}", i);
        let result = ledger.get(&key)?;
        assert_eq!(result, Some(large_data.as_bytes().to_vec()));
    }

    // Clear large values to test memory reclamation
    for i in 0..large_values {
        let key = format!("large:{:05}", i);
        ledger.put(&key, "")?; // Replace with empty value
    }

    // Verify replacement worked
    let key = "large:00050";
    let result = ledger.get(key)?;
    assert_eq!(result, Some(vec![]));

    // Total count should remain the same
    assert_eq!(ledger.len(), total_items);

    Ok(())
}
