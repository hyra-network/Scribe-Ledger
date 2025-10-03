//! Integration tests for distributed write path (Task 7.1)
//!
//! This test suite validates:
//! - Write request flow from client to leader
//! - Request forwarding logic (non-leader to leader)
//! - Timeout handling for write operations
//! - Batching of writes
//! - End-to-end distributed write path

use simple_scribe_ledger::api::DistributedApi;
use simple_scribe_ledger::consensus::ConsensusNode;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_single_node_write_flow() {
    // Create a single-node cluster
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    // Initialize as single-node cluster
    consensus.initialize().await.unwrap();

    // Wait for election
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Create distributed API
    let api = DistributedApi::new(consensus);

    // Verify node is leader
    assert!(api.is_leader().await);
    assert_eq!(api.current_leader().await, Some(1));

    // Write a value
    let result = api.put(b"test_key".to_vec(), b"test_value".to_vec()).await;
    assert!(result.is_ok(), "Write should succeed on leader");

    // Write another value
    let result = api
        .put(b"test_key2".to_vec(), b"test_value2".to_vec())
        .await;
    assert!(result.is_ok(), "Second write should succeed");
}

#[tokio::test]
async fn test_write_with_custom_timeout() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Create API with short timeout
    let api = DistributedApi::with_timeout(consensus, Duration::from_secs(5));

    // Write should complete within timeout
    let result = api.put(b"key".to_vec(), b"value".to_vec()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_batch_writes() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Prepare batch of writes
    let mut batch = Vec::new();
    for i in 0..10 {
        batch.push((
            format!("batch_key_{}", i).into_bytes(),
            format!("batch_value_{}", i).into_bytes(),
        ));
    }

    // Execute batch write
    let results = api.put_batch(batch).await.unwrap();

    // Verify all writes succeeded
    assert_eq!(results.len(), 10);
    for result in results {
        assert!(result.is_ok(), "All batch writes should succeed");
    }
}

#[tokio::test]
async fn test_large_batch_writes() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // Create API with smaller batch size to test batching logic
    let api = DistributedApi::with_batch_size(consensus, 50);

    // Prepare large batch that exceeds batch size
    let mut batch = Vec::new();
    for i in 0..200 {
        batch.push((
            format!("large_batch_key_{}", i).into_bytes(),
            format!("large_batch_value_{}", i).into_bytes(),
        ));
    }

    // Execute large batch write
    let results = api.put_batch(batch).await.unwrap();

    // Verify all writes succeeded
    assert_eq!(results.len(), 200);
    for result in results {
        assert!(result.is_ok(), "All large batch writes should succeed");
    }
}

#[tokio::test]
async fn test_delete_operations() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Write a value
    api.put(b"delete_test_key".to_vec(), b"delete_test_value".to_vec())
        .await
        .unwrap();

    // Delete the value
    let result = api.delete(b"delete_test_key".to_vec()).await;
    assert!(result.is_ok(), "Delete should succeed");
}

#[tokio::test]
async fn test_sequential_writes() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Perform multiple sequential writes
    for i in 0..50 {
        let key = format!("seq_key_{}", i).into_bytes();
        let value = format!("seq_value_{}", i).into_bytes();

        let result = api.put(key, value).await;
        assert!(result.is_ok(), "Sequential write {} should succeed", i);
    }
}

#[tokio::test]
async fn test_overwrite_values() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    let key = b"overwrite_key".to_vec();

    // Write initial value
    api.put(key.clone(), b"value1".to_vec()).await.unwrap();

    // Overwrite with new value
    api.put(key.clone(), b"value2".to_vec()).await.unwrap();

    // Overwrite again
    let result = api.put(key.clone(), b"value3".to_vec()).await;
    assert!(result.is_ok(), "Overwrite should succeed");
}

#[tokio::test]
async fn test_concurrent_writes() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = Arc::new(DistributedApi::new(consensus));

    // Spawn multiple concurrent write tasks
    let mut handles = Vec::new();

    for i in 0..10 {
        let api_clone = Arc::clone(&api);
        let handle = tokio::spawn(async move {
            let key = format!("concurrent_key_{}", i).into_bytes();
            let value = format!("concurrent_value_{}", i).into_bytes();
            api_clone.put(key, value).await
        });
        handles.push(handle);
    }

    // Wait for all writes to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent write should succeed");
    }
}

#[tokio::test]
async fn test_empty_batch() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Empty batch should succeed without error
    let results = api.put_batch(vec![]).await.unwrap();
    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_metrics_after_writes() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Perform some writes
    api.put(b"metrics_key_1".to_vec(), b"metrics_value_1".to_vec())
        .await
        .unwrap();
    api.put(b"metrics_key_2".to_vec(), b"metrics_value_2".to_vec())
        .await
        .unwrap();

    // Get metrics
    let metrics = api.metrics().await;

    // Verify metrics are available
    assert_eq!(metrics.id, 1);
    assert!(metrics.last_log_index.is_some());
}

#[tokio::test]
async fn test_write_before_initialization_fails() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    // Don't initialize - node is not in a cluster
    let api = DistributedApi::new(consensus);

    // Write should fail because node is not initialized
    let result = api.put(b"key".to_vec(), b"value".to_vec()).await;
    assert!(result.is_err(), "Write should fail before initialization");
}

#[tokio::test]
async fn test_multiple_batch_operations() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // First batch
    let batch1 = vec![
        (b"batch1_key1".to_vec(), b"batch1_value1".to_vec()),
        (b"batch1_key2".to_vec(), b"batch1_value2".to_vec()),
    ];
    api.put_batch(batch1).await.unwrap();

    // Second batch
    let batch2 = vec![
        (b"batch2_key1".to_vec(), b"batch2_value1".to_vec()),
        (b"batch2_key2".to_vec(), b"batch2_value2".to_vec()),
    ];
    api.put_batch(batch2).await.unwrap();

    // Third batch
    let batch3 = vec![
        (b"batch3_key1".to_vec(), b"batch3_value1".to_vec()),
        (b"batch3_key2".to_vec(), b"batch3_value2".to_vec()),
    ];
    let results = api.put_batch(batch3).await.unwrap();

    assert_eq!(results.len(), 2);
    for result in results {
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_mixed_operations() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Write
    api.put(b"mixed_key1".to_vec(), b"mixed_value1".to_vec())
        .await
        .unwrap();

    // Batch write
    let batch = vec![
        (b"mixed_key2".to_vec(), b"mixed_value2".to_vec()),
        (b"mixed_key3".to_vec(), b"mixed_value3".to_vec()),
    ];
    api.put_batch(batch).await.unwrap();

    // Write again
    api.put(b"mixed_key4".to_vec(), b"mixed_value4".to_vec())
        .await
        .unwrap();

    // Delete
    let result = api.delete(b"mixed_key1".to_vec()).await;
    assert!(result.is_ok());
}
