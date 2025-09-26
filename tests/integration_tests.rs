use simple_scribe_ledger::SimpleScribeLedger;
use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Integration test for real-world sled database scenarios
#[test]
fn test_database_lifecycle() -> Result<()> {
    let test_db = "./test_integration_db";
    
    // Clean up any existing test database
    if Path::new(test_db).exists() {
        fs::remove_dir_all(test_db).ok();
    }

    // Phase 1: Create database and populate it
    {
        let ledger = SimpleScribeLedger::new(test_db)?;
        
        // Simulate user data storage
        let users = vec![
            ("user:1", r#"{"name": "Alice", "age": 30, "email": "alice@example.com"}"#),
            ("user:2", r#"{"name": "Bob", "age": 25, "email": "bob@example.com"}"#),
            ("user:3", r#"{"name": "Charlie", "age": 35, "email": "charlie@example.com"}"#),
        ];

        for (key, value) in &users {
            ledger.put(key, value)?;
        }

        // Store some configuration
        ledger.put("config:version", "1.0.0")?;
        ledger.put("config:max_users", "1000")?;
        
        // Store some counters
        ledger.put("stats:total_requests", "0")?;
        ledger.put("stats:active_users", "3")?;
        
        ledger.flush()?;
        assert_eq!(ledger.len(), 7);
    }

    // Phase 2: Reopen database and verify persistence
    {
        let ledger = SimpleScribeLedger::new(test_db)?;
        assert_eq!(ledger.len(), 7);
        
        // Verify user data
        let alice = ledger.get("user:1")?;
        assert!(alice.is_some());
        let alice_data = alice.unwrap();
        let alice_str = String::from_utf8_lossy(&alice_data);
        assert!(alice_str.contains("Alice"));
        assert!(alice_str.contains("alice@example.com"));
        
        // Verify config
        let version = ledger.get("config:version")?;
        assert_eq!(version, Some(b"1.0.0".to_vec()));
        
        // Update some data
        ledger.put("stats:total_requests", "1542")?;
        ledger.put("user:4", r#"{"name": "Diana", "age": 28, "email": "diana@example.com"}"#)?;
        
        assert_eq!(ledger.len(), 8);
    }

    // Phase 3: Final verification and cleanup
    {
        let ledger = SimpleScribeLedger::new(test_db)?;
        assert_eq!(ledger.len(), 8);
        
        // Verify the updates persisted
        let requests = ledger.get("stats:total_requests")?;
        assert_eq!(requests, Some(b"1542".to_vec()));
        
        let diana = ledger.get("user:4")?;
        assert!(diana.is_some());
    }

    // Cleanup
    fs::remove_dir_all(test_db).ok();
    Ok(())
}

/// Test sled's behavior under high load
#[test]
fn test_high_load_sled_operations() -> Result<()> {
    let ledger = SimpleScribeLedger::temp()?;
    
    let batch_size = 1000;
    let num_batches = 10;
    
    // Insert data in batches
    for batch in 0..num_batches {
        for i in 0..batch_size {
            let key = format!("batch:{}:item:{}", batch, i);
            let value = format!("data_for_batch_{}_item_{}_with_timestamp_{}", 
                              batch, i, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
            ledger.put(&key, &value)?;
        }
        
        // Flush every batch to ensure persistence
        ledger.flush()?;
    }
    
    let expected_total = batch_size * num_batches;
    assert_eq!(ledger.len(), expected_total);
    
    // Verify random reads across batches
    for batch in (0..num_batches).step_by(2) {
        for i in (0..batch_size).step_by(100) {
            let key = format!("batch:{}:item:{}", batch, i);
            let result = ledger.get(&key)?;
            assert!(result.is_some());
            
            let value_data = result.unwrap();
            let value = String::from_utf8_lossy(&value_data);
            assert!(value.contains(&format!("batch_{}", batch)));
            assert!(value.contains(&format!("item_{}", i)));
        }
    }
    
    Ok(())
}

/// Test sled database recovery and consistency
#[test]
fn test_database_consistency() -> Result<()> {
    let test_db = "./test_consistency_db";
    
    // Clean up any existing test database
    if Path::new(test_db).exists() {
        fs::remove_dir_all(test_db).ok();
    }

    let initial_data: HashMap<String, String> = (0..100)
        .map(|i| (format!("consistency_key_{}", i), format!("consistency_value_{}", i)))
        .collect();

    // Phase 1: Write initial data
    {
        let ledger = SimpleScribeLedger::new(test_db)?;
        
        for (key, value) in &initial_data {
            ledger.put(key, value)?;
        }
        
        ledger.flush()?;
        assert_eq!(ledger.len(), initial_data.len());
    }

    // Phase 2: Verify all data is consistent after reopening
    {
        let ledger = SimpleScribeLedger::new(test_db)?;
        assert_eq!(ledger.len(), initial_data.len());
        
        // Verify all original data
        for (key, expected_value) in &initial_data {
            let stored_value = ledger.get(key)?;
            assert_eq!(stored_value, Some(expected_value.as_bytes().to_vec()));
        }
        
        // Modify some data
        let updates: HashMap<String, String> = (0..50)
            .map(|i| (format!("consistency_key_{}", i), format!("updated_value_{}", i)))
            .collect();
            
        for (key, value) in &updates {
            ledger.put(key, value)?;
        }
        
        // Length should remain the same (overwriting existing keys)
        assert_eq!(ledger.len(), initial_data.len());
        
        ledger.flush()?;
    }

    // Phase 3: Verify consistency after updates
    {
        let ledger = SimpleScribeLedger::new(test_db)?;
        assert_eq!(ledger.len(), initial_data.len());
        
        // Verify updated values
        for i in 0..50 {
            let key = format!("consistency_key_{}", i);
            let expected_value = format!("updated_value_{}", i);
            let stored_value = ledger.get(&key)?;
            assert_eq!(stored_value, Some(expected_value.as_bytes().to_vec()));
        }
        
        // Verify unchanged values
        for i in 50..100 {
            let key = format!("consistency_key_{}", i);
            let expected_value = format!("consistency_value_{}", i);
            let stored_value = ledger.get(&key)?;
            assert_eq!(stored_value, Some(expected_value.as_bytes().to_vec()));
        }
    }

    // Cleanup
    fs::remove_dir_all(test_db).ok();
    Ok(())
}

/// Test sled's memory usage and cleanup behavior
#[test]
fn test_memory_and_cleanup() -> Result<()> {
    let test_db = "./test_memory_db";
    
    // Clean up any existing test database
    if Path::new(test_db).exists() {
        fs::remove_dir_all(test_db).ok();
    }

    {
        let ledger = SimpleScribeLedger::new(test_db)?;
        
        // Create a large dataset
        let large_value = "x".repeat(1000); // 1KB per value
        
        for i in 0..1000 {
            let key = format!("large_data_{}", i);
            ledger.put(&key, &large_value)?;
        }
        
        assert_eq!(ledger.len(), 1000);
        
        // Clear half the data
        for i in 0..500 {
            let key = format!("large_data_{}", i);
            ledger.put(&key, "")?; // Overwrite with empty value
        }
        
        // Verify we still have 1000 keys but half have empty values
        assert_eq!(ledger.len(), 1000);
        
        // Verify empty values
        for i in 0..500 {
            let key = format!("large_data_{}", i);
            let result = ledger.get(&key)?;
            assert_eq!(result, Some(vec![]));
        }
        
        // Verify large values still exist for the rest
        for i in 500..1000 {
            let key = format!("large_data_{}", i);
            let result = ledger.get(&key)?;
            assert_eq!(result, Some(large_value.as_bytes().to_vec()));
        }
        
        // Clear everything
        ledger.clear()?;
        assert_eq!(ledger.len(), 0);
        assert!(ledger.is_empty());
        
        ledger.flush()?;
    }

    // Verify database is actually empty after reopening
    {
        let ledger = SimpleScribeLedger::new(test_db)?;
        assert_eq!(ledger.len(), 0);
        assert!(ledger.is_empty());
        
        // Verify we can still use the cleared database
        ledger.put("new_key", "new_value")?;
        let result = ledger.get("new_key")?;
        assert_eq!(result, Some(b"new_value".to_vec()));
    }

    // Cleanup
    fs::remove_dir_all(test_db).ok();
    Ok(())
}

/// Test edge cases and error conditions with sled
#[test]
fn test_sled_edge_cases() -> Result<()> {
    let ledger = SimpleScribeLedger::temp()?;
    
    // Test maximum key/value sizes that sled can handle
    let very_long_key = "k".repeat(65536); // 64KB key
    let very_long_value = "v".repeat(1048576); // 1MB value
    
    // This should work with sled
    ledger.put(&very_long_key, &very_long_value)?;
    let result = ledger.get(&very_long_key)?;
    assert_eq!(result, Some(very_long_value.as_bytes().to_vec()));
    
    // Test with keys that might cause issues
    let special_keys = vec![
        "\0", // null byte
        "\x01\x02\x03", // control characters
        "🔥💡🚀", // emoji
        "key with spaces",
        "key\nwith\nnewlines",
        "key\twith\ttabs",
    ];
    
    for (i, key) in special_keys.iter().enumerate() {
        let value = format!("value_for_special_key_{}", i);
        ledger.put(key, &value)?;
        
        let result = ledger.get(key)?;
        assert_eq!(result, Some(value.as_bytes().to_vec()));
    }
    
    // Test rapid put/get cycles
    for cycle in 0..100 {
        let key = format!("cycle_key_{}", cycle);
        let value = format!("cycle_value_{}", cycle);
        
        ledger.put(&key, &value)?;
        let result = ledger.get(&key)?;
        assert_eq!(result, Some(value.as_bytes().to_vec()));
    }
    
    Ok(())
}