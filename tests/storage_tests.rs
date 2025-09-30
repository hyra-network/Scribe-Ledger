//! Comprehensive tests for the storage layer
//!
//! This test file covers Task 2.2 requirements including:
//! - Basic put/get operations
//! - Large data handling (10MB+)
//! - Concurrent operations
//! - Persistence across restarts
//! - Error cases and edge cases
//! - Async behavior verification

use simple_scribe_ledger::storage::{SledStorage, StorageBackend};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio;

/// Test basic put and get operations
#[tokio::test]
async fn test_basic_put_get() {
    let storage = SledStorage::temp().unwrap();

    let key = b"test_key".to_vec();
    let value = b"test_value".to_vec();

    storage.put(key.clone(), value.clone()).await.unwrap();
    let result = storage.get(&key).await.unwrap();

    assert_eq!(result, Some(value));
}

/// Test get for non-existent key returns None
#[tokio::test]
async fn test_get_nonexistent() {
    let storage = SledStorage::temp().unwrap();

    let key = b"nonexistent_key".to_vec();
    let result = storage.get(&key).await.unwrap();

    assert_eq!(result, None);
}

/// Test delete operation
#[tokio::test]
async fn test_delete() {
    let storage = SledStorage::temp().unwrap();

    let key = b"test_key".to_vec();
    let value = b"test_value".to_vec();

    storage.put(key.clone(), value).await.unwrap();
    storage.delete(&key).await.unwrap();
    let result = storage.get(&key).await.unwrap();

    assert_eq!(result, None);
}

/// Test update/overwrite existing key
#[tokio::test]
async fn test_update_value() {
    let storage = SledStorage::temp().unwrap();

    let key = b"test_key".to_vec();
    let value1 = b"value1".to_vec();
    let value2 = b"value2".to_vec();

    storage.put(key.clone(), value1).await.unwrap();
    storage.put(key.clone(), value2.clone()).await.unwrap();
    let result = storage.get(&key).await.unwrap();

    assert_eq!(result, Some(value2));
}

/// Test empty key
#[tokio::test]
async fn test_empty_key() {
    let storage = SledStorage::temp().unwrap();

    let key = vec![];
    let value = b"value".to_vec();

    storage.put(key.clone(), value.clone()).await.unwrap();
    let result = storage.get(&key).await.unwrap();

    assert_eq!(result, Some(value));
}

/// Test empty value
#[tokio::test]
async fn test_empty_value() {
    let storage = SledStorage::temp().unwrap();

    let key = b"key".to_vec();
    let value = vec![];

    storage.put(key.clone(), value.clone()).await.unwrap();
    let result = storage.get(&key).await.unwrap();

    assert_eq!(result, Some(value));
}

/// Test Unicode keys and values
#[tokio::test]
async fn test_unicode() {
    let storage = SledStorage::temp().unwrap();

    let key = "🔑 key with emoji 你好".as_bytes().to_vec();
    let value = "🎉 value with emoji 世界".as_bytes().to_vec();

    storage.put(key.clone(), value.clone()).await.unwrap();
    let result = storage.get(&key).await.unwrap();

    assert_eq!(result, Some(value));
}

/// Test large data handling (10MB+)
#[tokio::test]
async fn test_large_data() {
    let storage = SledStorage::temp().unwrap();

    // Create 10MB value
    let large_value = vec![0xAB; 10 * 1024 * 1024];
    let key = b"large_key".to_vec();

    storage.put(key.clone(), large_value.clone()).await.unwrap();
    let result = storage.get(&key).await.unwrap();

    assert_eq!(result, Some(large_value));
}

/// Test multiple large data entries (50MB+ total)
#[tokio::test]
async fn test_multiple_large_data() {
    let storage = SledStorage::temp().unwrap();

    // Create 5 entries of 10MB each
    for i in 0..5 {
        let key = format!("large_key_{}", i).into_bytes();
        let value = vec![i as u8; 10 * 1024 * 1024];
        storage.put(key, value).await.unwrap();
    }

    // Verify all entries
    for i in 0..5 {
        let key = format!("large_key_{}", i).into_bytes();
        let result = storage.get(&key).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 10 * 1024 * 1024);
    }
}

/// Test concurrent put operations
#[tokio::test]
async fn test_concurrent_put() {
    let storage = Arc::new(SledStorage::temp().unwrap());
    let mut handles = vec![];

    // Launch 10 concurrent put operations
    for i in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            let key = format!("key_{}", i).into_bytes();
            let value = format!("value_{}", i).into_bytes();
            storage_clone.put(key, value).await.unwrap();
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all values
    for i in 0..10 {
        let key = format!("key_{}", i).into_bytes();
        let value = format!("value_{}", i).into_bytes();
        let result = storage.get(&key).await.unwrap();
        assert_eq!(result, Some(value));
    }
}

/// Test concurrent get operations
#[tokio::test]
async fn test_concurrent_get() {
    let storage = Arc::new(SledStorage::temp().unwrap());

    // Pre-populate data
    for i in 0..10 {
        let key = format!("key_{}", i).into_bytes();
        let value = format!("value_{}", i).into_bytes();
        storage.put(key, value).await.unwrap();
    }

    let mut handles = vec![];

    // Launch 10 concurrent get operations
    for i in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            let key = format!("key_{}", i).into_bytes();
            let value = format!("value_{}", i).into_bytes();
            let result = storage_clone.get(&key).await.unwrap();
            assert_eq!(result, Some(value));
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

/// Test concurrent mixed operations (put, get, delete)
#[tokio::test]
async fn test_concurrent_mixed() {
    let storage = Arc::new(SledStorage::temp().unwrap());
    let mut handles = vec![];

    // Launch concurrent mixed operations
    for i in 0..20 {
        let storage_clone = Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            let key = format!("key_{}", i).into_bytes();
            let value = format!("value_{}", i).into_bytes();

            if i % 3 == 0 {
                // Put operation
                storage_clone.put(key, value).await.unwrap();
            } else if i % 3 == 1 {
                // Get operation (might be None)
                let _result = storage_clone.get(&key).await.unwrap();
            } else {
                // Delete operation
                storage_clone.delete(&key).await.unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

/// Test persistence across restarts
#[tokio::test]
async fn test_persistence() {
    let test_dir = "./test_storage_persistence";

    // Clean up any existing test data
    if Path::new(test_dir).exists() {
        fs::remove_dir_all(test_dir).ok();
    }

    // Phase 1: Create storage and add data
    {
        let storage = SledStorage::new(test_dir).unwrap();

        for i in 0..100 {
            let key = format!("persist_key_{}", i).into_bytes();
            let value = format!("persist_value_{}", i).into_bytes();
            storage.put(key, value).await.unwrap();
        }

        storage.flush().await.unwrap();
    } // Storage is dropped here

    // Phase 2: Reopen storage and verify data
    {
        let storage = SledStorage::new(test_dir).unwrap();

        for i in 0..100 {
            let key = format!("persist_key_{}", i).into_bytes();
            let value = format!("persist_value_{}", i).into_bytes();
            let result = storage.get(&key).await.unwrap();
            assert_eq!(result, Some(value));
        }
    }

    // Cleanup
    fs::remove_dir_all(test_dir).ok();
}

/// Test snapshot functionality
#[tokio::test]
async fn test_snapshot() {
    let storage = SledStorage::temp().unwrap();

    // Add multiple entries
    for i in 0..50 {
        let key = format!("snap_key_{}", i).into_bytes();
        let value = format!("snap_value_{}", i).into_bytes();
        storage.put(key, value).await.unwrap();
    }

    // Take snapshot
    let snapshot = storage.snapshot().await.unwrap();

    // Verify snapshot contains all entries
    assert_eq!(snapshot.len(), 50);
    for i in 0..50 {
        let key = format!("snap_key_{}", i).into_bytes();
        let value = format!("snap_value_{}", i).into_bytes();
        assert_eq!(snapshot.get(&key), Some(&value));
    }
}

/// Test snapshot with large data
#[tokio::test]
async fn test_snapshot_large_data() {
    let storage = SledStorage::temp().unwrap();

    // Add 5 entries with 1MB each
    for i in 0..5 {
        let key = format!("large_snap_key_{}", i).into_bytes();
        let value = vec![i as u8; 1024 * 1024];
        storage.put(key, value).await.unwrap();
    }

    // Take snapshot
    let snapshot = storage.snapshot().await.unwrap();

    // Verify snapshot
    assert_eq!(snapshot.len(), 5);
    for i in 0..5 {
        let key = format!("large_snap_key_{}", i).into_bytes();
        assert!(snapshot.contains_key(&key));
        assert_eq!(snapshot.get(&key).unwrap().len(), 1024 * 1024);
    }
}

/// Test flush operation
#[tokio::test]
async fn test_flush() {
    let storage = SledStorage::temp().unwrap();

    let key = b"flush_key".to_vec();
    let value = b"flush_value".to_vec();

    storage.put(key.clone(), value.clone()).await.unwrap();
    storage.flush().await.unwrap();

    let result = storage.get(&key).await.unwrap();
    assert_eq!(result, Some(value));
}

/// Test clear operation
#[tokio::test]
async fn test_clear() {
    let storage = SledStorage::temp().unwrap();

    // Add multiple entries
    for i in 0..10 {
        let key = format!("clear_key_{}", i).into_bytes();
        let value = format!("clear_value_{}", i).into_bytes();
        storage.put(key, value).await.unwrap();
    }

    assert_eq!(storage.len().await.unwrap(), 10);

    // Clear all data
    storage.clear().await.unwrap();

    // Verify storage is empty
    assert_eq!(storage.len().await.unwrap(), 0);
    assert!(storage.is_empty().await.unwrap());
}

/// Test len and is_empty
#[tokio::test]
async fn test_len_and_is_empty() {
    let storage = SledStorage::temp().unwrap();

    assert!(storage.is_empty().await.unwrap());
    assert_eq!(storage.len().await.unwrap(), 0);

    // Add entries
    for i in 0..5 {
        let key = format!("key_{}", i).into_bytes();
        let value = format!("value_{}", i).into_bytes();
        storage.put(key, value).await.unwrap();
        assert_eq!(storage.len().await.unwrap(), i + 1);
    }

    assert!(!storage.is_empty().await.unwrap());
    assert_eq!(storage.len().await.unwrap(), 5);

    // Delete entries
    for i in 0..5 {
        let key = format!("key_{}", i).into_bytes();
        storage.delete(&key).await.unwrap();
        assert_eq!(storage.len().await.unwrap(), 5 - i - 1);
    }

    assert!(storage.is_empty().await.unwrap());
    assert_eq!(storage.len().await.unwrap(), 0);
}

/// Test special characters in keys
#[tokio::test]
async fn test_special_characters() {
    let storage = SledStorage::temp().unwrap();

    let special_keys = vec![
        b"\x00\x01\x02\x03".to_vec(),
        b"\xFF\xFE\xFD".to_vec(),
        b"key/with/slashes".to_vec(),
        b"key\\with\\backslashes".to_vec(),
        b"key with spaces".to_vec(),
        b"key\nwith\nnewlines".to_vec(),
        b"key\twith\ttabs".to_vec(),
    ];

    for (i, key) in special_keys.iter().enumerate() {
        let value = format!("value_{}", i).into_bytes();
        storage.put(key.clone(), value.clone()).await.unwrap();
        let result = storage.get(key).await.unwrap();
        assert_eq!(result, Some(value));
    }
}

/// Test binary data
#[tokio::test]
async fn test_binary_data() {
    let storage = SledStorage::temp().unwrap();

    let key = vec![0xFF, 0xAB, 0xCD, 0xEF];
    let value = vec![0x12, 0x34, 0x56, 0x78, 0x90];

    storage.put(key.clone(), value.clone()).await.unwrap();
    let result = storage.get(&key).await.unwrap();

    assert_eq!(result, Some(value));
}

/// Test async behavior with delays
#[tokio::test]
async fn test_async_with_delays() {
    let storage = Arc::new(SledStorage::temp().unwrap());
    let mut handles = vec![];

    // Launch concurrent operations with delays
    for i in 0..5 {
        let storage_clone = Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(i * 10)).await;
            let key = format!("async_key_{}", i).into_bytes();
            let value = format!("async_value_{}", i).into_bytes();
            storage_clone.put(key.clone(), value.clone()).await.unwrap();

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            let result = storage_clone.get(&key).await.unwrap();
            assert_eq!(result, Some(value));
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

/// Test many small operations
#[tokio::test]
async fn test_many_small_operations() {
    let storage = SledStorage::temp().unwrap();

    // Perform 1000 small operations
    for i in 0..1000 {
        let key = format!("small_key_{}", i).into_bytes();
        let value = format!("small_value_{}", i).into_bytes();
        storage.put(key.clone(), value.clone()).await.unwrap();

        if i % 2 == 0 {
            let result = storage.get(&key).await.unwrap();
            assert_eq!(result, Some(value));
        }
    }

    assert_eq!(storage.len().await.unwrap(), 1000);
}

/// Test rapid sequential operations
#[tokio::test]
async fn test_rapid_operations() {
    let storage = SledStorage::temp().unwrap();

    let key = b"rapid_key".to_vec();

    // Rapidly update the same key
    for i in 0..100 {
        let value = format!("rapid_value_{}", i).into_bytes();
        storage.put(key.clone(), value).await.unwrap();
    }

    let final_value = b"rapid_value_99".to_vec();
    let result = storage.get(&key).await.unwrap();
    assert_eq!(result, Some(final_value));
}
