//! Integration tests for distributed read path (Task 7.2)
//!
//! This test suite validates:
//! - Read request flow with different consistency levels
//! - Linearizable reads from leader
//! - Stale reads from local state machine
//! - Write-then-read consistency
//! - Read-your-writes consistency

use simple_scribe_ledger::api::{DistributedApi, ReadConsistency};
use simple_scribe_ledger::consensus::ConsensusNode;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_single_node_linearizable_read() {
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

    // Write a value
    api.put(b"test_key".to_vec(), b"test_value".to_vec())
        .await
        .unwrap();

    // Read with linearizable consistency
    let value = api
        .get(b"test_key".to_vec(), ReadConsistency::Linearizable)
        .await
        .unwrap();
    assert_eq!(value, Some(b"test_value".to_vec()));
}

#[tokio::test]
async fn test_single_node_stale_read() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Write a value
    api.put(b"test_key".to_vec(), b"test_value".to_vec())
        .await
        .unwrap();

    // Read with stale consistency
    let value = api
        .get(b"test_key".to_vec(), ReadConsistency::Stale)
        .await
        .unwrap();
    assert_eq!(value, Some(b"test_value".to_vec()));
}

#[tokio::test]
async fn test_write_then_read_consistency() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Write multiple key-value pairs
    for i in 0..100 {
        let key = format!("key{}", i).into_bytes();
        let value = format!("value{}", i).into_bytes();
        api.put(key, value).await.unwrap();
    }

    // Read them back with linearizable consistency
    for i in 0..100 {
        let key = format!("key{}", i).into_bytes();
        let expected_value = format!("value{}", i).into_bytes();
        let value = api.get(key, ReadConsistency::Linearizable).await.unwrap();
        assert_eq!(value, Some(expected_value));
    }
}

#[tokio::test]
async fn test_read_your_writes_consistency() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Write and immediately read
    for i in 0..50 {
        let key = format!("ryw_key{}", i).into_bytes();
        let value = format!("ryw_value{}", i).into_bytes();

        // Write
        api.put(key.clone(), value.clone()).await.unwrap();

        // Immediately read with linearizable consistency
        let read_value = api
            .get(key.clone(), ReadConsistency::Linearizable)
            .await
            .unwrap();
        assert_eq!(read_value, Some(value.clone()));

        // Also read with stale consistency
        let stale_value = api.get(key, ReadConsistency::Stale).await.unwrap();
        assert_eq!(stale_value, Some(value));
    }
}

#[tokio::test]
async fn test_read_non_existent_key() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Read non-existent key with linearizable consistency
    let value = api
        .get(b"non_existent".to_vec(), ReadConsistency::Linearizable)
        .await
        .unwrap();
    assert_eq!(value, None);

    // Read non-existent key with stale consistency
    let value = api
        .get(b"non_existent".to_vec(), ReadConsistency::Stale)
        .await
        .unwrap();
    assert_eq!(value, None);
}

#[tokio::test]
async fn test_read_after_overwrite() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    let key = b"overwrite_key".to_vec();

    // Write initial value
    api.put(key.clone(), b"value1".to_vec()).await.unwrap();

    // Read
    let value = api
        .get(key.clone(), ReadConsistency::Linearizable)
        .await
        .unwrap();
    assert_eq!(value, Some(b"value1".to_vec()));

    // Overwrite
    api.put(key.clone(), b"value2".to_vec()).await.unwrap();

    // Read again - should see new value
    let value = api
        .get(key.clone(), ReadConsistency::Linearizable)
        .await
        .unwrap();
    assert_eq!(value, Some(b"value2".to_vec()));

    // Overwrite again
    api.put(key.clone(), b"value3".to_vec()).await.unwrap();

    // Read again
    let value = api.get(key, ReadConsistency::Linearizable).await.unwrap();
    assert_eq!(value, Some(b"value3".to_vec()));
}

#[tokio::test]
async fn test_read_after_delete() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    let key = b"delete_key".to_vec();

    // Write a value
    api.put(key.clone(), b"value".to_vec()).await.unwrap();

    // Verify it exists
    let value = api
        .get(key.clone(), ReadConsistency::Linearizable)
        .await
        .unwrap();
    assert_eq!(value, Some(b"value".to_vec()));

    // Delete it
    api.delete(key.clone()).await.unwrap();

    // Read should return None
    let value = api.get(key, ReadConsistency::Linearizable).await.unwrap();
    assert_eq!(value, None);
}

#[tokio::test]
async fn test_stale_read_before_initialization() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    // Don't initialize
    let api = DistributedApi::new(consensus);

    // Stale read should work (return None) even before initialization
    let value = api
        .get(b"key".to_vec(), ReadConsistency::Stale)
        .await
        .unwrap();
    assert_eq!(value, None);
}

#[tokio::test]
async fn test_linearizable_read_before_initialization() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    // Don't initialize
    let api = DistributedApi::new(consensus);

    // Linearizable read should fail before initialization (not leader)
    let result = api
        .get(b"key".to_vec(), ReadConsistency::Linearizable)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_reads() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = Arc::new(DistributedApi::new(consensus));

    // Write some data
    for i in 0..20 {
        let key = format!("concurrent_key{}", i).into_bytes();
        let value = format!("concurrent_value{}", i).into_bytes();
        api.put(key, value).await.unwrap();
    }

    // Spawn multiple concurrent read tasks
    let mut handles = Vec::new();

    for i in 0..20 {
        let api_clone = Arc::clone(&api);
        let handle = tokio::spawn(async move {
            let key = format!("concurrent_key{}", i).into_bytes();
            let expected_value = format!("concurrent_value{}", i).into_bytes();

            let value = api_clone
                .get(key, ReadConsistency::Linearizable)
                .await
                .unwrap();
            assert_eq!(value, Some(expected_value));
        });
        handles.push(handle);
    }

    // Wait for all reads to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn test_mixed_consistency_reads() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Write data
    api.put(b"key1".to_vec(), b"value1".to_vec()).await.unwrap();
    api.put(b"key2".to_vec(), b"value2".to_vec()).await.unwrap();

    // Read with different consistency levels
    let v1_linearizable = api
        .get(b"key1".to_vec(), ReadConsistency::Linearizable)
        .await
        .unwrap();
    let v1_stale = api
        .get(b"key1".to_vec(), ReadConsistency::Stale)
        .await
        .unwrap();

    assert_eq!(v1_linearizable, Some(b"value1".to_vec()));
    assert_eq!(v1_stale, Some(b"value1".to_vec()));

    let v2_linearizable = api
        .get(b"key2".to_vec(), ReadConsistency::Linearizable)
        .await
        .unwrap();
    let v2_stale = api
        .get(b"key2".to_vec(), ReadConsistency::Stale)
        .await
        .unwrap();

    assert_eq!(v2_linearizable, Some(b"value2".to_vec()));
    assert_eq!(v2_stale, Some(b"value2".to_vec()));
}

#[tokio::test]
async fn test_sequential_writes_and_reads() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Alternating writes and reads
    for i in 0..30 {
        let key = format!("seq_key{}", i).into_bytes();
        let value = format!("seq_value{}", i).into_bytes();

        // Write
        api.put(key.clone(), value.clone()).await.unwrap();

        // Read
        let read_value = api.get(key, ReadConsistency::Linearizable).await.unwrap();
        assert_eq!(read_value, Some(value));
    }
}

#[tokio::test]
async fn test_batch_write_then_read() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Prepare batch
    let mut batch = Vec::new();
    for i in 0..50 {
        batch.push((
            format!("batch_key{}", i).into_bytes(),
            format!("batch_value{}", i).into_bytes(),
        ));
    }

    // Write batch
    api.put_batch(batch).await.unwrap();

    // Read all keys
    for i in 0..50 {
        let key = format!("batch_key{}", i).into_bytes();
        let expected_value = format!("batch_value{}", i).into_bytes();

        let value = api.get(key, ReadConsistency::Linearizable).await.unwrap();
        assert_eq!(value, Some(expected_value));
    }
}

#[tokio::test]
async fn test_large_value_read() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    // Create a large value (10KB)
    let large_value = vec![b'x'; 10 * 1024];
    let key = b"large_key".to_vec();

    // Write large value
    api.put(key.clone(), large_value.clone()).await.unwrap();

    // Read it back
    let value = api.get(key, ReadConsistency::Linearizable).await.unwrap();
    assert_eq!(value, Some(large_value));
}

#[tokio::test]
async fn test_empty_value_read() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    consensus.initialize().await.unwrap();
    tokio::time::sleep(Duration::from_millis(2000)).await;

    let api = DistributedApi::new(consensus);

    let key = b"empty_key".to_vec();
    let empty_value = vec![];

    // Write empty value
    api.put(key.clone(), empty_value.clone()).await.unwrap();

    // Read it back
    let value = api.get(key, ReadConsistency::Linearizable).await.unwrap();
    assert_eq!(value, Some(empty_value));
}
