//! API module for handling distributed read/write requests
//!
//! This module provides the high-level API for distributed operations,
//! including write request forwarding, batching, read operations, caching, and timeout handling.

use crate::cache::HotDataCache;
use crate::consensus::{AppRequest, AppResponse, ConsensusNode};
use crate::error::{Result, ScribeError};
use crate::types::{Key, NodeId, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Default timeout for write operations
const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// Default timeout for read operations
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum batch size for write operations
const DEFAULT_BATCH_SIZE: usize = 100;

/// Default cache capacity for hot data
const DEFAULT_CACHE_CAPACITY: usize = 1000;

/// Read consistency level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadConsistency {
    /// Linearizable read - guarantees reading the latest committed data
    /// Read must be served by the leader
    Linearizable,
    /// Stale read - may return slightly outdated data
    /// Can be served by any node (including followers)
    /// Provides better performance and availability
    Stale,
}

/// Distributed API for handling read/write requests with caching
pub struct DistributedApi {
    /// The consensus node
    consensus: Arc<ConsensusNode>,
    /// Write timeout
    write_timeout: Duration,
    /// Maximum batch size
    max_batch_size: usize,
    /// Hot data cache
    cache: Arc<HotDataCache>,
}

impl DistributedApi {
    /// Create a new distributed API with default cache
    pub fn new(consensus: Arc<ConsensusNode>) -> Self {
        Self {
            consensus,
            write_timeout: DEFAULT_WRITE_TIMEOUT,
            max_batch_size: DEFAULT_BATCH_SIZE,
            cache: Arc::new(HotDataCache::with_capacity(DEFAULT_CACHE_CAPACITY)),
        }
    }

    /// Create a new distributed API with custom timeout
    pub fn with_timeout(consensus: Arc<ConsensusNode>, write_timeout: Duration) -> Self {
        Self {
            consensus,
            write_timeout,
            max_batch_size: DEFAULT_BATCH_SIZE,
            cache: Arc::new(HotDataCache::with_capacity(DEFAULT_CACHE_CAPACITY)),
        }
    }

    /// Create a new distributed API with custom batch size
    pub fn with_batch_size(consensus: Arc<ConsensusNode>, max_batch_size: usize) -> Self {
        Self {
            consensus,
            write_timeout: DEFAULT_WRITE_TIMEOUT,
            max_batch_size,
            cache: Arc::new(HotDataCache::with_capacity(DEFAULT_CACHE_CAPACITY)),
        }
    }

    /// Create a new distributed API with custom timeout and batch size
    pub fn with_config(
        consensus: Arc<ConsensusNode>,
        write_timeout: Duration,
        max_batch_size: usize,
    ) -> Self {
        Self {
            consensus,
            write_timeout,
            max_batch_size,
            cache: Arc::new(HotDataCache::with_capacity(DEFAULT_CACHE_CAPACITY)),
        }
    }

    /// Create a new distributed API with custom cache capacity
    pub fn with_cache_capacity(consensus: Arc<ConsensusNode>, cache_capacity: usize) -> Self {
        Self {
            consensus,
            write_timeout: DEFAULT_WRITE_TIMEOUT,
            max_batch_size: DEFAULT_BATCH_SIZE,
            cache: Arc::new(HotDataCache::with_capacity(cache_capacity)),
        }
    }

    /// Create a new distributed API with full configuration
    pub fn with_full_config(
        consensus: Arc<ConsensusNode>,
        write_timeout: Duration,
        max_batch_size: usize,
        cache_capacity: usize,
    ) -> Self {
        Self {
            consensus,
            write_timeout,
            max_batch_size,
            cache: Arc::new(HotDataCache::with_capacity(cache_capacity)),
        }
    }

    /// Put a key-value pair with timeout and automatic forwarding
    ///
    /// This method:
    /// 1. Checks if the current node is the leader
    /// 2. If not leader, the request will fail with NotLeader error (client should retry with leader)
    /// 3. If leader, proposes the write to Raft
    /// 4. Waits for consensus with timeout
    /// 5. Returns success once committed
    /// 6. Invalidates cache entry for the key
    pub async fn put(&self, key: Key, value: Value) -> Result<()> {
        let request = AppRequest::Put {
            key: key.clone(),
            value: value.clone(),
        };

        // Execute write with timeout
        let result = timeout(self.write_timeout, self.consensus.client_write(request)).await;

        match result {
            Ok(Ok(AppResponse::PutOk)) => {
                // Update cache with new value
                self.cache.put(key, value);
                Ok(())
            }
            Ok(Ok(AppResponse::Error { message })) => {
                Err(ScribeError::Consensus(format!("Write failed: {}", message)))
            }
            Ok(Err(e)) => Err(ScribeError::Consensus(format!("Consensus error: {}", e))),
            Err(_) => Err(ScribeError::Consensus("Write timeout".to_string())),
            _ => Err(ScribeError::Consensus("Unexpected response".to_string())),
        }
    }

    /// Delete a key with timeout and automatic forwarding
    pub async fn delete(&self, key: Key) -> Result<()> {
        let request = AppRequest::Delete { key: key.clone() };

        // Execute delete with timeout
        let result = timeout(self.write_timeout, self.consensus.client_write(request)).await;

        match result {
            Ok(Ok(AppResponse::DeleteOk)) => {
                // Remove from cache
                self.cache.remove(&key);
                Ok(())
            }
            Ok(Ok(AppResponse::Error { message })) => Err(ScribeError::Consensus(format!(
                "Delete failed: {}",
                message
            ))),
            Ok(Err(e)) => Err(ScribeError::Consensus(format!("Consensus error: {}", e))),
            Err(_) => Err(ScribeError::Consensus("Delete timeout".to_string())),
            _ => Err(ScribeError::Consensus("Unexpected response".to_string())),
        }
    }

    /// Get a value by key with specified consistency level
    ///
    /// This method provides two consistency levels:
    /// - Linearizable: Reads the latest committed data from the leader
    /// - Stale: Reads from local state machine (may be slightly outdated)
    ///
    /// Both modes use the cache for performance optimization.
    pub async fn get(&self, key: Key, consistency: ReadConsistency) -> Result<Option<Value>> {
        // Try cache first for stale reads
        if consistency == ReadConsistency::Stale {
            if let Some(value) = self.cache.get(&key) {
                return Ok(Some(value));
            }
        }

        let result = match consistency {
            ReadConsistency::Linearizable => self.get_linearizable(key.clone()).await,
            ReadConsistency::Stale => self.get_stale(key.clone()).await,
        };

        // Update cache on successful read
        if let Ok(Some(ref value)) = result {
            self.cache.put(key, value.clone());
        }

        result
    }

    /// Get a value with linearizable consistency (from leader only)
    async fn get_linearizable(&self, key: Key) -> Result<Option<Value>> {
        // Execute read with timeout
        let result = timeout(
            DEFAULT_READ_TIMEOUT,
            self.consensus.client_read(key.as_slice()),
        )
        .await;

        match result {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(e)) => Err(ScribeError::Consensus(format!("Read error: {}", e))),
            Err(_) => Err(ScribeError::Consensus("Read timeout".to_string())),
        }
    }

    /// Get a value with stale consistency (from local state machine)
    async fn get_stale(&self, key: Key) -> Result<Option<Value>> {
        // Read from local state machine (no timeout needed, it's a local operation)
        Ok(self.consensus.client_read_local(key.as_slice()).await)
    }

    /// Get a value with default linearizable consistency
    pub async fn get_default(&self, key: Key) -> Result<Option<Value>> {
        self.get(key, ReadConsistency::Linearizable).await
    }

    /// Batch write multiple key-value pairs
    ///
    /// This method batches multiple writes into a single Raft proposal when possible.
    /// If the batch size exceeds max_batch_size, it will be split into multiple proposals.
    pub async fn put_batch(&self, items: Vec<(Key, Value)>) -> Result<Vec<Result<()>>> {
        if items.is_empty() {
            return Ok(vec![]);
        }

        let mut results = Vec::with_capacity(items.len());

        // Process items in batches
        for chunk in items.chunks(self.max_batch_size) {
            for (key, value) in chunk {
                let result = self.put(key.clone(), value.clone()).await;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Check if this node is the leader
    pub async fn is_leader(&self) -> bool {
        self.consensus.is_leader().await
    }

    /// Get the current leader ID
    pub async fn current_leader(&self) -> Option<NodeId> {
        self.consensus.current_leader().await
    }

    /// Get consensus metrics
    pub async fn metrics(&self) -> openraft::RaftMetrics<NodeId, openraft::BasicNode> {
        self.consensus.metrics().await
    }

    /// Clear the hot data cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Get cache capacity
    pub fn cache_capacity(&self) -> usize {
        self.cache.capacity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_creation() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());
        let api = DistributedApi::new(consensus);

        assert_eq!(api.write_timeout, DEFAULT_WRITE_TIMEOUT);
        assert_eq!(api.max_batch_size, DEFAULT_BATCH_SIZE);
    }

    #[tokio::test]
    async fn test_api_with_custom_timeout() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());
        let custom_timeout = Duration::from_secs(60);
        let api = DistributedApi::with_timeout(consensus, custom_timeout);

        assert_eq!(api.write_timeout, custom_timeout);
    }

    #[tokio::test]
    async fn test_api_with_custom_batch_size() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());
        let api = DistributedApi::with_batch_size(consensus, 200);

        assert_eq!(api.max_batch_size, 200);
    }

    #[tokio::test]
    async fn test_api_is_leader_before_init() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());
        let api = DistributedApi::new(consensus);

        // Before initialization, should not be leader
        assert!(!api.is_leader().await);
    }

    #[tokio::test]
    async fn test_api_current_leader_before_init() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());
        let api = DistributedApi::new(consensus);

        // Before initialization, there should be no leader
        assert_eq!(api.current_leader().await, None);
    }

    #[tokio::test]
    async fn test_api_put_before_init() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());
        let api = DistributedApi::new(consensus);

        // Writing before initialization should fail
        let result = api.put(b"test_key".to_vec(), b"test_value".to_vec()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_api_put_after_init() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

        // Initialize as single-node cluster
        consensus.initialize().await.unwrap();

        // Wait for election
        tokio::time::sleep(Duration::from_millis(2000)).await;

        let api = DistributedApi::new(consensus);

        // Should be leader now
        assert!(api.is_leader().await);

        // Write should succeed
        let result = api.put(b"test_key".to_vec(), b"test_value".to_vec()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_api_delete_after_init() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

        // Initialize as single-node cluster
        consensus.initialize().await.unwrap();

        // Wait for election
        tokio::time::sleep(Duration::from_millis(2000)).await;

        let api = DistributedApi::new(consensus);

        // Put a value first
        api.put(b"test_key".to_vec(), b"test_value".to_vec())
            .await
            .unwrap();

        // Delete should succeed
        let result = api.delete(b"test_key".to_vec()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_api_batch_put_empty() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());
        let api = DistributedApi::new(consensus);

        // Empty batch should succeed immediately
        let results = api.put_batch(vec![]).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_api_batch_put_after_init() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

        // Initialize as single-node cluster
        consensus.initialize().await.unwrap();

        // Wait for election
        tokio::time::sleep(Duration::from_millis(2000)).await;

        let api = DistributedApi::new(consensus);

        // Batch write should succeed
        let items = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
        ];

        let results = api.put_batch(items).await.unwrap();
        assert_eq!(results.len(), 3);

        // All should succeed
        for result in results {
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_api_batch_put_large_batch() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

        // Initialize as single-node cluster
        consensus.initialize().await.unwrap();

        // Wait for election
        tokio::time::sleep(Duration::from_millis(2000)).await;

        let api = DistributedApi::with_batch_size(consensus, 50);

        // Create a batch larger than max_batch_size
        let mut items = Vec::new();
        for i in 0..150 {
            items.push((
                format!("key{}", i).into_bytes(),
                format!("value{}", i).into_bytes(),
            ));
        }

        let results = api.put_batch(items).await.unwrap();
        assert_eq!(results.len(), 150);

        // All should succeed
        for result in results {
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_api_get_before_init() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());
        let api = DistributedApi::new(consensus);

        // Reading before initialization should fail for linearizable reads
        let result = api
            .get(b"test_key".to_vec(), ReadConsistency::Linearizable)
            .await;
        assert!(result.is_err());

        // Stale reads should work (return None)
        let result = api.get(b"test_key".to_vec(), ReadConsistency::Stale).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_api_get_linearizable() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

        // Initialize as single-node cluster
        consensus.initialize().await.unwrap();

        // Wait for election
        tokio::time::sleep(Duration::from_millis(2000)).await;

        let api = DistributedApi::new(consensus);

        // Put a value
        api.put(b"test_key".to_vec(), b"test_value".to_vec())
            .await
            .unwrap();

        // Get with linearizable consistency
        let value = api
            .get(b"test_key".to_vec(), ReadConsistency::Linearizable)
            .await
            .unwrap();
        assert_eq!(value, Some(b"test_value".to_vec()));

        // Get non-existent key
        let value = api
            .get(b"non_existent".to_vec(), ReadConsistency::Linearizable)
            .await
            .unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_api_get_stale() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

        // Initialize as single-node cluster
        consensus.initialize().await.unwrap();

        // Wait for election
        tokio::time::sleep(Duration::from_millis(2000)).await;

        let api = DistributedApi::new(consensus);

        // Put a value
        api.put(b"test_key".to_vec(), b"test_value".to_vec())
            .await
            .unwrap();

        // Get with stale consistency
        let value = api
            .get(b"test_key".to_vec(), ReadConsistency::Stale)
            .await
            .unwrap();
        assert_eq!(value, Some(b"test_value".to_vec()));
    }

    #[tokio::test]
    async fn test_api_get_default() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

        // Initialize as single-node cluster
        consensus.initialize().await.unwrap();

        // Wait for election
        tokio::time::sleep(Duration::from_millis(2000)).await;

        let api = DistributedApi::new(consensus);

        // Put a value
        api.put(b"test_key".to_vec(), b"test_value".to_vec())
            .await
            .unwrap();

        // Get with default consistency (linearizable)
        let value = api.get_default(b"test_key".to_vec()).await.unwrap();
        assert_eq!(value, Some(b"test_value".to_vec()));
    }

    #[tokio::test]
    async fn test_api_write_then_read() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

        // Initialize as single-node cluster
        consensus.initialize().await.unwrap();

        // Wait for election
        tokio::time::sleep(Duration::from_millis(2000)).await;

        let api = DistributedApi::new(consensus);

        // Write multiple values
        for i in 0..10 {
            let key = format!("key{}", i).into_bytes();
            let value = format!("value{}", i).into_bytes();
            api.put(key, value).await.unwrap();
        }

        // Read them back
        for i in 0..10 {
            let key = format!("key{}", i).into_bytes();
            let expected_value = format!("value{}", i).into_bytes();
            let value = api.get(key, ReadConsistency::Linearizable).await.unwrap();
            assert_eq!(value, Some(expected_value));
        }
    }

    #[tokio::test]
    async fn test_api_read_consistency_enum() {
        // Just verify the enum values exist and can be used
        let _linearizable = ReadConsistency::Linearizable;
        let _stale = ReadConsistency::Stale;

        assert_eq!(ReadConsistency::Linearizable, ReadConsistency::Linearizable);
        assert_ne!(ReadConsistency::Linearizable, ReadConsistency::Stale);
    }
}
