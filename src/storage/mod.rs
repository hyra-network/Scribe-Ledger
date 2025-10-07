//! Storage module for managing the underlying storage backend
//!
//! This module contains the storage abstraction layer and Sled implementation.

pub mod archival;
pub mod s3;
pub mod segment;

use crate::error::{Result, ScribeError};
use crate::types::{Key, Value};
use async_trait::async_trait;
use sled::Db;
use std::collections::HashMap;
use std::path::Path;

/// Storage backend trait for async operations
///
/// This trait provides an async abstraction over the underlying storage engine.
/// All operations are async to support non-blocking I/O in distributed systems.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Put a key-value pair into storage
    async fn put(&self, key: Key, value: Value) -> Result<()>;

    /// Get a value by key from storage
    async fn get(&self, key: &Key) -> Result<Option<Value>>;

    /// Delete a key from storage
    async fn delete(&self, key: &Key) -> Result<()>;

    /// Flush any pending writes to disk
    async fn flush(&self) -> Result<()>;

    /// Take a snapshot of all data in storage
    async fn snapshot(&self) -> Result<HashMap<Key, Value>>;
}

/// Sled-based storage implementation
///
/// This struct wraps the sled database and provides async operations
/// using tokio's spawn_blocking for non-blocking I/O.
pub struct SledStorage {
    db: Db,
}

impl SledStorage {
    /// Create a new SledStorage instance at the given path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Create a temporary SledStorage instance for testing
    pub fn temp() -> Result<Self> {
        let db = sled::Config::new().temporary(true).open()?;
        Ok(Self { db })
    }

    /// Get the number of entries in storage
    pub async fn len(&self) -> Result<usize> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || db.len())
            .await
            .map_err(|e| ScribeError::Other(format!("Task join error: {}", e)))
    }

    /// Check if storage is empty
    pub async fn is_empty(&self) -> Result<bool> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || db.is_empty())
            .await
            .map_err(|e| ScribeError::Other(format!("Task join error: {}", e)))
    }

    /// Clear all data from storage
    pub async fn clear(&self) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.clear()?;
            db.flush()?;
            Ok::<(), ScribeError>(())
        })
        .await
        .map_err(|e| ScribeError::Other(format!("Task join error: {}", e)))?
    }
}

#[async_trait]
impl StorageBackend for SledStorage {
    async fn put(&self, key: Key, value: Value) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.insert(key, value)?;
            Ok::<(), ScribeError>(())
        })
        .await
        .map_err(|e| ScribeError::Other(format!("Task join error: {}", e)))?
    }

    async fn get(&self, key: &Key) -> Result<Option<Value>> {
        let db = self.db.clone();
        let key = key.clone();
        tokio::task::spawn_blocking(move || match db.get(key)? {
            Some(ivec) => Ok(Some(ivec.to_vec())),
            None => Ok(None),
        })
        .await
        .map_err(|e| ScribeError::Other(format!("Task join error: {}", e)))?
    }

    async fn delete(&self, key: &Key) -> Result<()> {
        let db = self.db.clone();
        let key = key.clone();
        tokio::task::spawn_blocking(move || {
            db.remove(key)?;
            Ok::<(), ScribeError>(())
        })
        .await
        .map_err(|e| ScribeError::Other(format!("Task join error: {}", e)))?
    }

    async fn flush(&self) -> Result<()> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            db.flush()?;
            Ok::<(), ScribeError>(())
        })
        .await
        .map_err(|e| ScribeError::Other(format!("Task join error: {}", e)))?
    }

    async fn snapshot(&self) -> Result<HashMap<Key, Value>> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let mut snapshot = HashMap::new();
            for item in db.iter() {
                let (key, value) = item?;
                snapshot.insert(key.to_vec(), value.to_vec());
            }
            Ok::<HashMap<Key, Value>, ScribeError>(snapshot)
        })
        .await
        .map_err(|e| ScribeError::Other(format!("Task join error: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_storage_backend_put_get() {
        let storage = SledStorage::temp().unwrap();

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        storage.put(key.clone(), value.clone()).await.unwrap();
        let result = storage.get(&key).await.unwrap();

        assert_eq!(result, Some(value));
    }

    #[tokio::test]
    async fn test_storage_backend_delete() {
        let storage = SledStorage::temp().unwrap();

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        storage.put(key.clone(), value.clone()).await.unwrap();
        storage.delete(&key).await.unwrap();
        let result = storage.get(&key).await.unwrap();

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_storage_backend_flush() {
        let storage = SledStorage::temp().unwrap();

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        storage.put(key.clone(), value.clone()).await.unwrap();
        storage.flush().await.unwrap();

        let result = storage.get(&key).await.unwrap();
        assert_eq!(result, Some(value));
    }

    #[tokio::test]
    async fn test_storage_backend_snapshot() {
        let storage = SledStorage::temp().unwrap();

        let key1 = b"key1".to_vec();
        let value1 = b"value1".to_vec();
        let key2 = b"key2".to_vec();
        let value2 = b"value2".to_vec();

        storage.put(key1.clone(), value1.clone()).await.unwrap();
        storage.put(key2.clone(), value2.clone()).await.unwrap();

        let snapshot = storage.snapshot().await.unwrap();

        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot.get(&key1), Some(&value1));
        assert_eq!(snapshot.get(&key2), Some(&value2));
    }

    #[tokio::test]
    async fn test_storage_len_and_empty() {
        let storage = SledStorage::temp().unwrap();

        assert!(storage.is_empty().await.unwrap());
        assert_eq!(storage.len().await.unwrap(), 0);

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        storage.put(key, value).await.unwrap();

        assert!(!storage.is_empty().await.unwrap());
        assert_eq!(storage.len().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_storage_clear() {
        let storage = SledStorage::temp().unwrap();

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        storage.put(key, value).await.unwrap();

        assert_eq!(storage.len().await.unwrap(), 1);

        storage.clear().await.unwrap();

        assert!(storage.is_empty().await.unwrap());
        assert_eq!(storage.len().await.unwrap(), 0);
    }
}
