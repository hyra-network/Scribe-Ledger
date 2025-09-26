use anyhow::Result;
use sled::Db;
use std::path::Path;

/// Simple Scribe Ledger - A minimal key-value storage engine using sled
pub struct SimpleScribeLedger {
    db: Db,
}

impl SimpleScribeLedger {
    /// Create a new instance of the storage engine
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Create a temporary in-memory instance for testing
    pub fn temp() -> Result<Self> {
        let db = sled::Config::new().temporary(true).open()?;
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

    /// Get a value by key from the storage
    pub fn get<K>(&self, key: K) -> Result<Option<Vec<u8>>>
    where
        K: AsRef<[u8]>,
    {
        let result = self.db.get(key.as_ref())?;
        Ok(result.map(|ivec| ivec.to_vec()))
    }

    /// Get the number of key-value pairs in the storage
    pub fn len(&self) -> usize {
        self.db.len()
    }

    /// Check if the storage is empty
    pub fn is_empty(&self) -> bool {
        self.db.is_empty()
    }

    /// Flush any pending writes to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    /// Clear all data from the storage
    pub fn clear(&self) -> Result<()> {
        self.db.clear()?;
        Ok(())
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
}