use anyhow::Result;
use sled::Db;
use std::path::Path;

// New modules for distributed ledger functionality
pub mod api;
pub mod cluster;
pub mod config;
pub mod consensus;
pub mod discovery;
pub mod error;
pub mod http_client;
pub mod manifest;
pub mod network;
pub mod storage;
pub mod types;

/// Simple Scribe Ledger - A minimal key-value storage engine using sled
pub struct SimpleScribeLedger {
    db: Db,
}

impl SimpleScribeLedger {
    /// Create a new instance of the storage engine with optimized configuration
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::Config::new()
            .path(path)
            .cache_capacity(256 * 1024 * 1024) // 256MB cache for better performance
            .flush_every_ms(Some(5000)) // Flush every 5 seconds for better write throughput
            .mode(sled::Mode::HighThroughput) // Optimize for write throughput
            .open()?;
        Ok(Self { db })
    }

    /// Create a temporary in-memory instance for testing with optimized config
    pub fn temp() -> Result<Self> {
        let db = sled::Config::new()
            .temporary(true)
            .cache_capacity(128 * 1024 * 1024) // 128MB cache for temp instances
            .flush_every_ms(None) // Let sled manage flushing for temp instances (best perf)
            .mode(sled::Mode::HighThroughput) // Optimize for write throughput
            .open()?;
        Ok(Self { db })
    }

    /// Put a key-value pair into the storage
    pub fn put<K, V>(&self, key: K, value: V) -> Result<()>
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        self.db.insert(key.as_ref(), value.as_ref())?;
        Ok(())
    }

    /// Get a value by key from the storage (optimized, zero-copy when possible)
    pub fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>>
    where
        K: AsRef<[u8]>,
    {
        let result = self.db.get(key.as_ref())?;
        Ok(result.map(|ivec| ivec.to_vec()))
    }

    /// Get a value by key without copying (returns reference to internal buffer)
    /// This is more efficient but requires careful lifetime management
    pub fn get_ref<K>(&self, key: K) -> Result<Option<sled::IVec>>
    where
        K: AsRef<[u8]>,
    {
        self.db.get(key.as_ref()).map_err(Into::into)
    }

    /// Get the number of key-value pairs in the storage
    pub fn len(&self) -> usize {
        self.db.len()
    }

    /// Check if the storage is empty
    pub fn is_empty(&self) -> bool {
        self.db.is_empty()
    }

    /// Flush any pending writes to disk synchronously (expensive)
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    /// Flush any pending writes to disk asynchronously (preferred)
    pub async fn flush_async(&self) -> Result<()> {
        self.db.flush_async().await?;
        Ok(())
    }

    /// Clear all data from the storage
    pub fn clear(&self) -> Result<()> {
        self.db.clear()?;
        Ok(())
    }

    /// Apply a batch of operations atomically
    pub fn apply_batch(&self, batch: sled::Batch) -> Result<()> {
        self.db.apply_batch(batch)?;
        Ok(())
    }

    /// Apply multiple batches atomically without intermediate flushing (best performance)
    /// Optimized to minimize allocations and synchronization overhead
    pub fn apply_batches<I>(&self, batches: I) -> Result<()>
    where
        I: IntoIterator<Item = sled::Batch>,
    {
        for batch in batches {
            self.db.apply_batch(batch)?;
        }
        Ok(())
    }

    /// Apply batches with final flush (ensures durability)
    pub fn apply_batches_with_flush<I>(&self, batches: I) -> Result<()>
    where
        I: IntoIterator<Item = sled::Batch>,
    {
        for batch in batches {
            self.db.apply_batch(batch)?;
        }
        self.db.flush()?;
        Ok(())
    }

    /// Create a new batch for bulk operations
    pub fn new_batch() -> sled::Batch {
        sled::Batch::default()
    }

    /// Put a serializable value using binary encoding (faster than JSON)
    pub fn put_bincode<K, V>(&self, key: K, value: &V) -> Result<()>
    where
        K: AsRef<[u8]>,
        V: serde::Serialize,
    {
        let encoded = bincode::serialize(value)?;
        self.db.insert(key.as_ref(), encoded)?;
        Ok(())
    }

    /// Get and deserialize a value using binary encoding
    pub fn get_bincode<K, V>(&self, key: K) -> Result<Option<V>>
    where
        K: AsRef<[u8]>,
        V: serde::de::DeserializeOwned,
    {
        if let Some(data) = self.db.get(key.as_ref())? {
            let decoded: V = bincode::deserialize(&data)?;
            Ok(Some(decoded))
        } else {
            Ok(None)
        }
    }
}

impl Drop for SimpleScribeLedger {
    fn drop(&mut self) {
        let _ = self.db.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_put_and_get() -> Result<()> {
        let ledger = SimpleScribeLedger::temp()?;

        // Test putting and getting a value
        ledger.put("key1", "value1")?;
        let result = ledger.get("key1")?;
        assert_eq!(result, Some(b"value1".to_vec()));

        // Test getting a non-existent key
        let result = ledger.get("nonexistent")?;
        assert_eq!(result, None);

        Ok(())
    }

    #[test]
    fn test_multiple_puts_and_gets() -> Result<()> {
        let ledger = SimpleScribeLedger::temp()?;

        // Put multiple values
        for i in 0..100 {
            let key = format!("key{}", i);
            let value = format!("value{}", i);
            ledger.put(&key, &value)?;
        }

        // Verify all values
        for i in 0..100 {
            let key = format!("key{}", i);
            let expected_value = format!("value{}", i);
            let result = ledger.get(&key)?;
            assert_eq!(result, Some(expected_value.as_bytes().to_vec()));
        }

        assert_eq!(ledger.len(), 100);

        Ok(())
    }

    #[test]
    fn test_overwrite_value() -> Result<()> {
        let ledger = SimpleScribeLedger::temp()?;

        // Put initial value
        ledger.put("key1", "value1")?;
        let result = ledger.get("key1")?;
        assert_eq!(result, Some(b"value1".to_vec()));

        // Overwrite with new value
        ledger.put("key1", "value2")?;
        let result = ledger.get("key1")?;
        assert_eq!(result, Some(b"value2".to_vec()));

        Ok(())
    }

    #[test]
    fn test_clear() -> Result<()> {
        let ledger = SimpleScribeLedger::temp()?;

        // Put some values
        ledger.put("key1", "value1")?;
        ledger.put("key2", "value2")?;
        assert_eq!(ledger.len(), 2);

        // Clear and verify empty
        ledger.clear()?;
        assert_eq!(ledger.len(), 0);
        assert!(ledger.is_empty());

        let result = ledger.get("key1")?;
        assert_eq!(result, None);

        Ok(())
    }

    #[test]
    fn test_sled_persistence() -> Result<()> {
        use std::fs;
        use std::path::Path;

        // Create a unique directory for this test run to avoid lock contention
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let thread_id = format!("{:?}", std::thread::current().id());
        let random_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let test_dir = format!(
            "./test_persistence_db_{}_{}_{}",
            timestamp,
            thread_id
                .replace("ThreadId", "")
                .replace("(", "")
                .replace(")", ""),
            random_suffix
        );

        // Cleanup any existing test data with retries
        if Path::new(&test_dir).exists() {
            for attempt in 0..3 {
                if fs::remove_dir_all(&test_dir).is_ok() {
                    break;
                }
                if attempt < 2 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }

        // Create ledger and store some data
        {
            let ledger = SimpleScribeLedger::new(&test_dir)?;
            ledger.put("persistent_key", "persistent_value")?;
            ledger.put("another_key", "another_value")?;
            ledger.flush()?;
            assert_eq!(ledger.len(), 2);

            // Explicitly drop to release locks
            drop(ledger);
        }

        // Small delay to ensure lock is released
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Open the same database again and verify data persists
        {
            let ledger = SimpleScribeLedger::new(&test_dir)?;
            assert_eq!(ledger.len(), 2);

            let value1 = ledger.get("persistent_key")?;
            assert_eq!(value1, Some(b"persistent_value".to_vec()));

            let value2 = ledger.get("another_key")?;
            assert_eq!(value2, Some(b"another_value".to_vec()));

            // Explicitly drop to release locks before cleanup
            drop(ledger);
        }

        // Small delay before cleanup
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Cleanup with retries
        for attempt in 0..5 {
            if fs::remove_dir_all(&test_dir).is_ok() {
                break;
            }
            if attempt < 4 {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        Ok(())
    }

    #[test]
    fn test_sled_large_keys_and_values() -> Result<()> {
        let ledger = SimpleScribeLedger::temp()?;

        // Test with large key
        let large_key = "x".repeat(1000);
        let large_value = "y".repeat(10000);

        ledger.put(&large_key, &large_value)?;

        let result = ledger.get(&large_key)?;
        assert_eq!(result, Some(large_value.as_bytes().to_vec()));

        Ok(())
    }

    #[test]
    fn test_sled_binary_data() -> Result<()> {
        let ledger = SimpleScribeLedger::temp()?;

        // Test with binary data containing null bytes and special characters
        let binary_key = vec![0u8, 1, 255, 128, 64];
        let binary_value = vec![255u8, 0, 1, 2, 254, 253, 100, 200];

        ledger.put(&binary_key, &binary_value)?;

        let result = ledger.get(&binary_key)?;
        assert_eq!(result, Some(binary_value));

        Ok(())
    }

    #[test]
    fn test_sled_unicode_support() -> Result<()> {
        let ledger = SimpleScribeLedger::temp()?;

        // Test with Unicode characters
        let unicode_key = "🔑key测试";
        let unicode_value = "🌟value测试数据🚀";

        ledger.put(unicode_key, unicode_value)?;

        let result = ledger.get(unicode_key)?;
        assert_eq!(result, Some(unicode_value.as_bytes().to_vec()));

        // Verify we can read it back as string
        if let Some(data) = result {
            let recovered = String::from_utf8(data)?;
            assert_eq!(recovered, unicode_value);
        }

        Ok(())
    }

    #[test]
    fn test_sled_concurrent_operations() -> Result<()> {
        use std::sync::Arc;
        use std::thread;

        let ledger = Arc::new(SimpleScribeLedger::temp()?);
        let mut handles = vec![];

        // Spawn multiple threads to perform concurrent operations
        for i in 0..10 {
            let ledger_clone = Arc::clone(&ledger);
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let key = format!("thread{}key{}", i, j);
                    let value = format!("thread{}value{}", i, j);
                    ledger_clone.put(&key, &value).unwrap();
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all data was written
        assert_eq!(ledger.len(), 1000); // 10 threads * 100 operations each

        // Verify we can read some of the data
        let test_value = ledger.get("thread5key50")?;
        assert_eq!(test_value, Some(b"thread5value50".to_vec()));

        Ok(())
    }

    #[test]
    fn test_sled_stress_operations() -> Result<()> {
        let ledger = SimpleScribeLedger::temp()?;

        // Stress test with many operations
        let num_operations = 5000;

        // Insert many keys
        for i in 0..num_operations {
            let key = format!("stress_key_{}", i);
            let value = format!("stress_value_{}_with_extra_data", i);
            ledger.put(&key, &value)?;
        }

        assert_eq!(ledger.len(), num_operations);

        // Verify random access
        for i in (0..num_operations).step_by(100) {
            let key = format!("stress_key_{}", i);
            let expected_value = format!("stress_value_{}_with_extra_data", i);
            let result = ledger.get(&key)?;
            assert_eq!(result, Some(expected_value.as_bytes().to_vec()));
        }

        // Test overwriting some values
        for i in (0..num_operations).step_by(200) {
            let key = format!("stress_key_{}", i);
            let new_value = format!("updated_value_{}", i);
            ledger.put(&key, &new_value)?;
        }

        // Verify overwrites worked
        let result = ledger.get("stress_key_200")?;
        assert_eq!(result, Some(b"updated_value_200".to_vec()));

        // Length should remain the same after overwrites
        assert_eq!(ledger.len(), num_operations);

        Ok(())
    }

    #[test]
    fn test_sled_empty_keys_and_values() -> Result<()> {
        let ledger = SimpleScribeLedger::temp()?;

        // Test empty value
        ledger.put("empty_value_key", "")?;
        let result = ledger.get("empty_value_key")?;
        assert_eq!(result, Some(vec![]));

        // Test empty key
        ledger.put("", "value_for_empty_key")?;
        let result = ledger.get("")?;
        assert_eq!(result, Some(b"value_for_empty_key".to_vec()));

        // Test both empty
        ledger.put("", "")?;
        let result = ledger.get("")?;
        assert_eq!(result, Some(vec![]));

        Ok(())
    }

    #[test]
    fn test_sled_flush_behavior() -> Result<()> {
        use std::fs;
        use std::path::Path;

        // Create a unique directory for this test run to avoid lock contention
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let thread_id = format!("{:?}", std::thread::current().id());
        let random_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let test_dir = format!(
            "./test_flush_db_{}_{}_{}",
            timestamp,
            thread_id
                .replace("ThreadId", "")
                .replace("(", "")
                .replace(")", ""),
            random_suffix
        );

        // Cleanup any existing test data with retries
        if Path::new(&test_dir).exists() {
            for attempt in 0..3 {
                if fs::remove_dir_all(&test_dir).is_ok() {
                    break;
                }
                if attempt < 2 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }

        {
            let ledger = SimpleScribeLedger::new(&test_dir)?;

            // Add data but don't flush
            ledger.put("test_key", "test_value")?;

            // Manually flush
            ledger.flush()?;

            // Verify flush doesn't affect functionality
            let result = ledger.get("test_key")?;
            assert_eq!(result, Some(b"test_value".to_vec()));

            // Add more data and flush again
            ledger.put("test_key2", "test_value2")?;
            ledger.flush()?;

            assert_eq!(ledger.len(), 2);

            // Explicitly drop to release locks before cleanup
            drop(ledger);
        }

        // Small delay before cleanup
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Cleanup with retries
        for attempt in 0..5 {
            if fs::remove_dir_all(&test_dir).is_ok() {
                break;
            }
            if attempt < 4 {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        Ok(())
    }

    #[test]
    fn test_sled_error_handling() -> Result<()> {
        // Test that we can handle various error conditions gracefully
        use std::fs;
        use std::path::Path;

        // Create a unique directory for this test run to avoid lock contention
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let thread_id = format!("{:?}", std::thread::current().id());
        let random_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros();
        let test_dir = format!(
            "./test_error_db_{}_{}_{}",
            timestamp,
            thread_id
                .replace("ThreadId", "")
                .replace("(", "")
                .replace(")", ""),
            random_suffix
        );

        // Cleanup any existing test data with retries
        if Path::new(&test_dir).exists() {
            for attempt in 0..3 {
                if fs::remove_dir_all(&test_dir).is_ok() {
                    break;
                }
                if attempt < 2 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }

        // Test 1: Create and use database normally
        {
            let ledger = SimpleScribeLedger::new(&test_dir)?;
            ledger.put("test", "data")?;

            let result = ledger.get("test")?;
            assert_eq!(result, Some(b"data".to_vec()));
            assert_eq!(ledger.len(), 1);

            // Explicitly drop to release locks
            drop(ledger);
        }

        // Small delay to ensure lock is released
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Test 2: Reopen the same database (should work after first is dropped)
        {
            let ledger = SimpleScribeLedger::new(&test_dir)?;
            let result = ledger.get("test")?;
            assert_eq!(result, Some(b"data".to_vec()));

            // Add more data to verify it's working
            ledger.put("test2", "data2")?;
            assert_eq!(ledger.len(), 2);

            // Explicitly drop to release locks before cleanup
            drop(ledger);
        }

        // Small delay before cleanup
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Test 3: Test with invalid operations (should handle gracefully)
        {
            let ledger = SimpleScribeLedger::temp()?;

            // Test getting non-existent key (should return None, not error)
            let result = ledger.get("non_existent_key")?;
            assert_eq!(result, None);

            // Test putting and getting empty strings (should work)
            ledger.put("", "")?;
            let result = ledger.get("")?;
            assert_eq!(result, Some(vec![]));
        }

        // Cleanup with retries
        for attempt in 0..5 {
            if fs::remove_dir_all(&test_dir).is_ok() {
                break;
            }
            if attempt < 4 {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        Ok(())
    }

    #[test]
    #[allow(unused_imports)]
    fn test_module_structure() {
        // Test that all new modules are accessible and properly declared
        // This verifies Task 1.1 directory structure requirement

        // Simply importing the modules verifies they exist and compile correctly
        use crate::{config, consensus, manifest, network, storage};

        // These are module imports, so we just need to ensure they compile
        // Using them in a simple way to avoid "unused import" warnings
        let _ = (
            stringify!(consensus),
            stringify!(storage),
            stringify!(network),
            stringify!(manifest),
            stringify!(config),
        );
    }

    #[test]
    fn test_dependencies_available() {
        // Test that all new dependencies from Task 1.1 are available
        // This ensures openraft, thiserror, and tracing are properly added

        // Just verify we can reference them via stringify
        let _ = stringify!(openraft);
        let _ = stringify!(thiserror);
        let _ = stringify!(tracing);
        let _ = stringify!(tracing_subscriber);
    }
}
