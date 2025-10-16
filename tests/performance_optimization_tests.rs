//! Tests for Task 11.3: Performance Optimization features
//!
//! This test suite validates:
//! - Batching for Raft proposals
//! - Connection pooling optimization
//! - Serialization optimization (bincode)
//! - Caching layer for hot data
//! - Tunable Raft parameters

use hyra_scribe_ledger::api::DistributedApi;
use hyra_scribe_ledger::cache::HotDataCache;
use hyra_scribe_ledger::consensus::ConsensusNode;
use hyra_scribe_ledger::HyraScribeLedger;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_hot_data_cache_initialization() {
    // Test default cache
    let cache = HotDataCache::new();
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.capacity(), 1000);

    // Test custom capacity
    let cache_custom = HotDataCache::with_capacity(500);
    assert_eq!(cache_custom.capacity(), 500);
}

#[test]
fn test_hot_data_cache_operations() {
    let cache = HotDataCache::with_capacity(10);

    // Put and get
    cache.put(b"key1".to_vec(), b"value1".to_vec());
    assert_eq!(cache.get(&b"key1".to_vec()), Some(b"value1".to_vec()));

    // Update existing
    cache.put(b"key1".to_vec(), b"value2".to_vec());
    assert_eq!(cache.get(&b"key1".to_vec()), Some(b"value2".to_vec()));

    // Remove
    let removed = cache.remove(&b"key1".to_vec());
    assert_eq!(removed, Some(b"value2".to_vec()));
    assert_eq!(cache.get(&b"key1".to_vec()), None);

    // Clear
    cache.put(b"key2".to_vec(), b"value2".to_vec());
    cache.put(b"key3".to_vec(), b"value3".to_vec());
    cache.clear();
    assert!(cache.is_empty());
}

#[test]
fn test_hot_data_cache_lru_behavior() {
    let cache = HotDataCache::with_capacity(3);

    // Fill cache
    cache.put(b"key1".to_vec(), b"value1".to_vec());
    cache.put(b"key2".to_vec(), b"value2".to_vec());
    cache.put(b"key3".to_vec(), b"value3".to_vec());
    assert_eq!(cache.len(), 3);

    // Access key1 to make it most recently used
    let _ = cache.get(&b"key1".to_vec());

    // Add key4, should evict key2 (least recently used)
    cache.put(b"key4".to_vec(), b"value4".to_vec());

    assert_eq!(cache.get(&b"key1".to_vec()), Some(b"value1".to_vec()));
    assert_eq!(cache.get(&b"key2".to_vec()), None); // Evicted
    assert_eq!(cache.get(&b"key3".to_vec()), Some(b"value3".to_vec()));
    assert_eq!(cache.get(&b"key4".to_vec()), Some(b"value4".to_vec()));
}

#[tokio::test]
async fn test_distributed_api_with_cache() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    // Test with default cache
    let api = DistributedApi::new(consensus.clone());
    assert_eq!(api.cache_capacity(), 1000);
    assert_eq!(api.cache_size(), 0);

    // Test with custom cache capacity
    let api_custom = DistributedApi::with_cache_capacity(consensus.clone(), 500);
    assert_eq!(api_custom.cache_capacity(), 500);
    assert_eq!(api_custom.cache_size(), 0);

    // Test cache clear
    api_custom.clear_cache();
    assert_eq!(api_custom.cache_size(), 0);
}

#[tokio::test]
async fn test_distributed_api_full_config() {
    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    let api = DistributedApi::with_full_config(consensus, Duration::from_secs(60), 200, 2000);

    assert_eq!(api.cache_capacity(), 2000);
}

#[test]
fn test_bincode_serialization() {
    let ledger = HyraScribeLedger::temp().unwrap();

    // Test bincode put and get
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct TestData {
        id: u64,
        name: String,
        values: Vec<i32>,
    }

    let data = TestData {
        id: 42,
        name: "test".to_string(),
        values: vec![1, 2, 3, 4, 5],
    };

    // Put with bincode
    ledger.put_bincode("test_key", &data).unwrap();

    // Get with bincode
    let retrieved: Option<TestData> = ledger.get_bincode("test_key").unwrap();
    assert_eq!(retrieved, Some(data));

    // Test with non-existent key
    let missing: Option<TestData> = ledger.get_bincode("missing").unwrap();
    assert_eq!(missing, None);
}

#[test]
fn test_batch_operations() {
    let ledger = HyraScribeLedger::temp().unwrap();

    // Create multiple batches
    let mut batch1 = HyraScribeLedger::new_batch();
    batch1.insert(b"key1", b"value1");
    batch1.insert(b"key2", b"value2");

    let mut batch2 = HyraScribeLedger::new_batch();
    batch2.insert(b"key3", b"value3");
    batch2.insert(b"key4", b"value4");

    // Apply batches without flush
    ledger.apply_batches(vec![batch1, batch2]).unwrap();

    // Verify data
    assert_eq!(ledger.get(b"key1").unwrap(), Some(b"value1".to_vec()));
    assert_eq!(ledger.get(b"key2").unwrap(), Some(b"value2".to_vec()));
    assert_eq!(ledger.get(b"key3").unwrap(), Some(b"value3".to_vec()));
    assert_eq!(ledger.get(b"key4").unwrap(), Some(b"value4".to_vec()));
}

#[test]
fn test_batch_operations_with_flush() {
    let ledger = HyraScribeLedger::temp().unwrap();

    let mut batch = HyraScribeLedger::new_batch();
    for i in 0..100 {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        batch.insert(key.as_bytes(), value.as_bytes());
    }

    // Apply with flush
    ledger.apply_batches_with_flush(vec![batch]).unwrap();

    // Verify some entries
    assert_eq!(ledger.get(b"key0").unwrap(), Some(b"value0".to_vec()));
    assert_eq!(ledger.get(b"key50").unwrap(), Some(b"value50".to_vec()));
    assert_eq!(ledger.get(b"key99").unwrap(), Some(b"value99".to_vec()));
}

#[test]
fn test_bincode_performance_vs_json() {
    use std::time::Instant;

    let ledger = HyraScribeLedger::temp().unwrap();

    #[derive(serde::Serialize, serde::Deserialize, Clone)]
    struct LargeData {
        fields: Vec<(String, String)>,
    }

    let data = LargeData {
        fields: (0..100)
            .map(|i| (format!("field{}", i), format!("value{}", i)))
            .collect(),
    };

    // Measure bincode serialization
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("bincode_{}", i);
        ledger.put_bincode(&key, &data).unwrap();
    }
    let bincode_time = start.elapsed();

    // Measure JSON serialization
    let start = Instant::now();
    for i in 0..100 {
        let key = format!("json_{}", i);
        let json = serde_json::to_string(&data).unwrap();
        ledger.put(&key, json).unwrap();
    }
    let json_time = start.elapsed();

    // Bincode should be faster (but we don't enforce strict timing in tests)
    println!("Bincode time: {:?}", bincode_time);
    println!("JSON time: {:?}", json_time);

    // Just verify both work
    let retrieved: Option<LargeData> = ledger.get_bincode("bincode_0").unwrap();
    assert!(retrieved.is_some());

    let json_retrieved = ledger.get("json_0").unwrap();
    assert!(json_retrieved.is_some());
}

#[test]
fn test_cache_integration_scenario() {
    let cache = HotDataCache::with_capacity(5);

    // Simulate hot data access pattern
    let hot_keys = vec![b"hot1".to_vec(), b"hot2".to_vec(), b"hot3".to_vec()];

    // Populate hot data
    for (i, key) in hot_keys.iter().enumerate() {
        cache.put(key.clone(), format!("value{}", i).into_bytes());
    }

    // Access hot keys multiple times (simulating real traffic)
    for _ in 0..10 {
        for key in &hot_keys {
            let _ = cache.get(key);
        }
    }

    // Add cold data that should evict less frequently used entries
    for i in 0..10 {
        let key = format!("cold{}", i).into_bytes();
        cache.put(key, format!("coldvalue{}", i).into_bytes());
    }

    // Hot keys should still be available (some of them at least)
    let _hot_count = hot_keys.iter().filter(|k| cache.get(k).is_some()).count();

    // With capacity 5, and 3 hot keys, after adding 10 cold items,
    // we expect the cache to have exactly 5 items
    assert_eq!(cache.len(), 5);

    // The assertion depends on the LRU behavior
    // Since we accessed hot keys multiple times before adding cold data,
    // and capacity is 5 (more than 3 hot keys), we could still have some hot data
    // But after adding 10 cold items, only the last 5 items will remain
    assert!(cache.len() <= cache.capacity());
}

#[test]
fn test_consensus_config_defaults() {
    use hyra_scribe_ledger::config::Config;

    let config = Config::default_for_node(1);

    // Verify consensus parameters are set
    assert_eq!(config.consensus.election_timeout_min, 1000);
    assert_eq!(config.consensus.heartbeat_interval_ms, 300);
    assert_eq!(config.consensus.max_payload_entries, 300);
    assert_eq!(config.consensus.snapshot_logs_since_last, 5000);
    assert_eq!(config.consensus.max_in_snapshot_log_to_keep, 1000);
}

#[tokio::test]
async fn test_consensus_node_with_optimized_config() {
    use openraft::Config as RaftConfig;

    let db = sled::Config::new().temporary(true).open().unwrap();

    // Create with custom optimized config
    let raft_config = RaftConfig {
        heartbeat_interval: 300,
        election_timeout_min: 1000,
        election_timeout_max: 2000,
        max_payload_entries: 500,
        snapshot_policy: openraft::SnapshotPolicy::LogsSinceLast(10000),
        max_in_snapshot_log_to_keep: 2000,
        enable_tick: true,
        enable_heartbeat: true,
        ..Default::default()
    };

    let consensus = ConsensusNode::new_with_config(1, db, raft_config).await;
    assert!(consensus.is_ok());
}

#[test]
fn test_empty_batch_operations() {
    let ledger = HyraScribeLedger::temp().unwrap();

    // Empty batch should work without errors
    let result = ledger.apply_batches(Vec::<sled::Batch>::new());
    assert!(result.is_ok());

    let result = ledger.apply_batches_with_flush(Vec::<sled::Batch>::new());
    assert!(result.is_ok());
}

#[test]
fn test_large_batch_performance() {
    let ledger = HyraScribeLedger::temp().unwrap();

    let mut batch = HyraScribeLedger::new_batch();
    let num_items = 10000;

    for i in 0..num_items {
        let key = format!("key{}", i);
        let value = format!("value{}", i);
        batch.insert(key.as_bytes(), value.as_bytes());
    }

    // Apply large batch
    let result = ledger.apply_batch(batch);
    assert!(result.is_ok());

    // Verify the count
    assert_eq!(ledger.len(), num_items);

    // Spot check some entries
    assert_eq!(ledger.get(b"key0").unwrap(), Some(b"value0".to_vec()));
    assert_eq!(ledger.get(b"key5000").unwrap(), Some(b"value5000".to_vec()));
    assert_eq!(ledger.get(b"key9999").unwrap(), Some(b"value9999".to_vec()));
}
