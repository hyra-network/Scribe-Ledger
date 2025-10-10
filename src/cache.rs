//! Caching layer for hot data optimization
//!
//! This module provides an LRU cache for frequently accessed key-value pairs
//! to reduce the load on the storage backend and improve read performance.

use crate::types::{Key, Value};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

/// Default cache capacity (number of entries)
const DEFAULT_CACHE_CAPACITY: usize = 1000;

/// Hot data cache using LRU eviction policy
pub struct HotDataCache {
    cache: Mutex<LruCache<Key, Value>>,
}

impl HotDataCache {
    /// Create a new cache with default capacity
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CACHE_CAPACITY)
    }

    /// Create a new cache with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            cache: Mutex::new(LruCache::new(capacity)),
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &Key) -> Option<Value> {
        let mut cache = self.cache.lock().unwrap();
        cache.get(key).cloned()
    }

    /// Put a value into the cache
    pub fn put(&self, key: Key, value: Value) {
        let mut cache = self.cache.lock().unwrap();
        cache.put(key, value);
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &Key) -> Option<Value> {
        let mut cache = self.cache.lock().unwrap();
        cache.pop(key)
    }

    /// Clear all entries from the cache
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }

    /// Get the number of entries in the cache
    pub fn len(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        let cache = self.cache.lock().unwrap();
        cache.is_empty()
    }

    /// Get cache capacity
    pub fn capacity(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.cap().get()
    }
}

impl Default for HotDataCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache = HotDataCache::new();

        // Test empty cache
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);

        // Test put and get
        cache.put(b"key1".to_vec(), b"value1".to_vec());
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());

        let value = cache.get(&b"key1".to_vec());
        assert_eq!(value, Some(b"value1".to_vec()));

        // Test get non-existent key
        let missing = cache.get(&b"missing".to_vec());
        assert_eq!(missing, None);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = HotDataCache::with_capacity(2);

        cache.put(b"key1".to_vec(), b"value1".to_vec());
        cache.put(b"key2".to_vec(), b"value2".to_vec());
        assert_eq!(cache.len(), 2);

        // Adding third item should evict the least recently used (key1)
        cache.put(b"key3".to_vec(), b"value3".to_vec());
        assert_eq!(cache.len(), 2);

        assert_eq!(cache.get(&b"key1".to_vec()), None);
        assert_eq!(cache.get(&b"key2".to_vec()), Some(b"value2".to_vec()));
        assert_eq!(cache.get(&b"key3".to_vec()), Some(b"value3".to_vec()));
    }

    #[test]
    fn test_cache_remove() {
        let cache = HotDataCache::new();

        cache.put(b"key1".to_vec(), b"value1".to_vec());
        assert_eq!(cache.len(), 1);

        let removed = cache.remove(&b"key1".to_vec());
        assert_eq!(removed, Some(b"value1".to_vec()));
        assert_eq!(cache.len(), 0);

        let not_found = cache.remove(&b"key1".to_vec());
        assert_eq!(not_found, None);
    }

    #[test]
    fn test_cache_clear() {
        let cache = HotDataCache::new();

        cache.put(b"key1".to_vec(), b"value1".to_vec());
        cache.put(b"key2".to_vec(), b"value2".to_vec());
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_capacity() {
        let cache = HotDataCache::with_capacity(100);
        assert_eq!(cache.capacity(), 100);

        let default_cache = HotDataCache::new();
        assert_eq!(default_cache.capacity(), DEFAULT_CACHE_CAPACITY);
    }

    #[test]
    fn test_cache_update_existing() {
        let cache = HotDataCache::new();

        cache.put(b"key1".to_vec(), b"value1".to_vec());
        cache.put(b"key1".to_vec(), b"value2".to_vec());

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&b"key1".to_vec()), Some(b"value2".to_vec()));
    }

    #[test]
    fn test_cache_access_order() {
        let cache = HotDataCache::with_capacity(3);

        cache.put(b"key1".to_vec(), b"value1".to_vec());
        cache.put(b"key2".to_vec(), b"value2".to_vec());
        cache.put(b"key3".to_vec(), b"value3".to_vec());

        // Access key1 to make it most recently used
        let _ = cache.get(&b"key1".to_vec());

        // Add key4, should evict key2 (least recently used)
        cache.put(b"key4".to_vec(), b"value4".to_vec());

        assert_eq!(cache.get(&b"key1".to_vec()), Some(b"value1".to_vec()));
        assert_eq!(cache.get(&b"key2".to_vec()), None);
        assert_eq!(cache.get(&b"key3".to_vec()), Some(b"value3".to_vec()));
        assert_eq!(cache.get(&b"key4".to_vec()), Some(b"value4".to_vec()));
    }
}
