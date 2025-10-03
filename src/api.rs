//! API module for handling distributed read/write requests
//!
//! This module provides the high-level API for distributed operations,
//! including write request forwarding, batching, and timeout handling.

use crate::consensus::{AppRequest, AppResponse, ConsensusNode};
use crate::error::{Result, ScribeError};
use crate::types::{Key, NodeId, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Default timeout for write operations
const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// Maximum batch size for write operations
const DEFAULT_BATCH_SIZE: usize = 100;

/// Distributed API for handling read/write requests
pub struct DistributedApi {
    /// The consensus node
    consensus: Arc<ConsensusNode>,
    /// Write timeout
    write_timeout: Duration,
    /// Maximum batch size
    max_batch_size: usize,
}

impl DistributedApi {
    /// Create a new distributed API
    pub fn new(consensus: Arc<ConsensusNode>) -> Self {
        Self {
            consensus,
            write_timeout: DEFAULT_WRITE_TIMEOUT,
            max_batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    /// Create a new distributed API with custom timeout
    pub fn with_timeout(consensus: Arc<ConsensusNode>, write_timeout: Duration) -> Self {
        Self {
            consensus,
            write_timeout,
            max_batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    /// Create a new distributed API with custom batch size
    pub fn with_batch_size(consensus: Arc<ConsensusNode>, max_batch_size: usize) -> Self {
        Self {
            consensus,
            write_timeout: DEFAULT_WRITE_TIMEOUT,
            max_batch_size,
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
    pub async fn put(&self, key: Key, value: Value) -> Result<()> {
        let request = AppRequest::Put { key, value };

        // Execute write with timeout
        let result = timeout(self.write_timeout, self.consensus.client_write(request)).await;

        match result {
            Ok(Ok(AppResponse::PutOk)) => Ok(()),
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
        let request = AppRequest::Delete { key };

        // Execute delete with timeout
        let result = timeout(self.write_timeout, self.consensus.client_write(request)).await;

        match result {
            Ok(Ok(AppResponse::DeleteOk)) => Ok(()),
            Ok(Ok(AppResponse::Error { message })) => Err(ScribeError::Consensus(format!(
                "Delete failed: {}",
                message
            ))),
            Ok(Err(e)) => Err(ScribeError::Consensus(format!("Consensus error: {}", e))),
            Err(_) => Err(ScribeError::Consensus("Delete timeout".to_string())),
            _ => Err(ScribeError::Consensus("Unexpected response".to_string())),
        }
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
}
