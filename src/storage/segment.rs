//! Segment-based storage for preparing multi-tier storage architecture
//!
//! This module provides segment data structures for buffering writes and
//! preparing for future S3 integration.

use crate::error::{Result, ScribeError};
use crate::types::{Key, SegmentId, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default segment size threshold (10MB)
pub const DEFAULT_SEGMENT_SIZE_THRESHOLD: usize = 10 * 1024 * 1024;

/// A segment containing data with metadata
///
/// Segments are the unit of data organization for multi-tier storage.
/// Each segment contains a set of key-value pairs along with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /// Unique identifier for this segment
    pub segment_id: SegmentId,
    /// Unix timestamp when the segment was created
    pub timestamp: u64,
    /// Key-value data stored in this segment
    pub data: HashMap<Key, Value>,
    /// Total size of data in bytes
    pub size: usize,
}

impl Segment {
    /// Create a new segment with the given ID
    pub fn new(segment_id: SegmentId) -> Self {
        Self {
            segment_id,
            timestamp: current_timestamp(),
            data: HashMap::new(),
            size: 0,
        }
    }

    /// Create a segment from existing data
    pub fn from_data(segment_id: SegmentId, data: HashMap<Key, Value>) -> Self {
        let size = data.iter().fold(0, |acc, (k, v)| acc + k.len() + v.len());
        Self {
            segment_id,
            timestamp: current_timestamp(),
            data,
            size,
        }
    }

    /// Add a key-value pair to the segment
    pub fn put(&mut self, key: Key, value: Value) {
        let old_size = self
            .data
            .get(&key)
            .map(|v| key.len() + v.len())
            .unwrap_or(0);
        let new_size = key.len() + value.len();

        self.data.insert(key, value);
        self.size = self.size - old_size + new_size;
    }

    /// Get a value by key from the segment
    pub fn get(&self, key: &Key) -> Option<&Value> {
        self.data.get(key)
    }

    /// Remove a key from the segment
    pub fn remove(&mut self, key: &Key) -> Option<Value> {
        if let Some(value) = self.data.remove(key) {
            self.size -= key.len() + value.len();
            Some(value)
        } else {
            None
        }
    }

    /// Check if the segment is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the number of key-value pairs in the segment
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Serialize the segment to bytes using bincode
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| ScribeError::Serialization(e.to_string()))
    }

    /// Deserialize a segment from bytes using bincode
    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).map_err(|e| ScribeError::Serialization(e.to_string()))
    }
}

/// A pending segment for buffering writes before flushing
///
/// PendingSegment accumulates writes until it reaches a size threshold,
/// at which point it can be flushed to persistent storage.
#[derive(Debug)]
pub struct PendingSegment {
    /// The underlying segment data
    segment: Segment,
    /// Size threshold for triggering flush
    size_threshold: usize,
}

impl PendingSegment {
    /// Create a new pending segment with the given ID
    pub fn new(segment_id: SegmentId) -> Self {
        Self {
            segment: Segment::new(segment_id),
            size_threshold: DEFAULT_SEGMENT_SIZE_THRESHOLD,
        }
    }

    /// Create a new pending segment with a custom size threshold
    pub fn with_threshold(segment_id: SegmentId, size_threshold: usize) -> Self {
        Self {
            segment: Segment::new(segment_id),
            size_threshold,
        }
    }

    /// Add a key-value pair to the pending segment
    pub fn put(&mut self, key: Key, value: Value) {
        self.segment.put(key, value);
    }

    /// Get a value by key from the pending segment
    pub fn get(&self, key: &Key) -> Option<&Value> {
        self.segment.get(key)
    }

    /// Check if the segment should be flushed based on size threshold
    pub fn should_flush(&self) -> bool {
        self.segment.size >= self.size_threshold
    }

    /// Get the current size of the segment
    pub fn size(&self) -> usize {
        self.segment.size
    }

    /// Get the segment data (consuming the pending segment)
    pub fn into_segment(self) -> Segment {
        self.segment
    }

    /// Get a reference to the underlying segment
    pub fn segment(&self) -> &Segment {
        &self.segment
    }

    /// Clear the pending segment
    pub fn clear(&mut self) {
        self.segment = Segment::new(self.segment.segment_id + 1);
    }
}

/// Manager for tracking active and flushed segments
///
/// SegmentManager coordinates between active pending segments and
/// flushed segments that are ready for archival or retrieval.
#[derive(Debug)]
pub struct SegmentManager {
    /// Currently active pending segment
    active_segment: Arc<RwLock<PendingSegment>>,
    /// Flushed segments ready for archival
    flushed_segments: Arc<RwLock<Vec<Segment>>>,
    /// Next segment ID to use
    next_segment_id: Arc<AtomicU64>,
    /// Size threshold for segments
    size_threshold: usize,
}

impl SegmentManager {
    /// Create a new segment manager
    pub fn new() -> Self {
        Self::with_threshold(DEFAULT_SEGMENT_SIZE_THRESHOLD)
    }

    /// Create a new segment manager with a custom size threshold
    pub fn with_threshold(size_threshold: usize) -> Self {
        let segment_id = 0;
        Self {
            active_segment: Arc::new(RwLock::new(PendingSegment::with_threshold(
                segment_id,
                size_threshold,
            ))),
            flushed_segments: Arc::new(RwLock::new(Vec::new())),
            next_segment_id: Arc::new(AtomicU64::new(segment_id + 1)),
            size_threshold,
        }
    }

    /// Put a key-value pair, potentially triggering a segment flush
    pub fn put(&self, key: Key, value: Value) -> Result<()> {
        let mut active = self
            .active_segment
            .write()
            .map_err(|e| ScribeError::Other(format!("Failed to acquire write lock: {}", e)))?;

        active.put(key, value);

        // Check if we should flush the active segment
        if active.should_flush() {
            let segment_id = self.next_segment_id.fetch_add(1, Ordering::SeqCst);
            let old_segment = std::mem::replace(
                &mut *active,
                PendingSegment::with_threshold(segment_id, self.size_threshold),
            );

            // Move the old segment to flushed segments
            let mut flushed = self
                .flushed_segments
                .write()
                .map_err(|e| ScribeError::Other(format!("Failed to acquire write lock: {}", e)))?;
            flushed.push(old_segment.into_segment());
        }

        Ok(())
    }

    /// Get a value by key from active or flushed segments
    pub fn get(&self, key: &Key) -> Result<Option<Value>> {
        // First check active segment
        let active = self
            .active_segment
            .read()
            .map_err(|e| ScribeError::Other(format!("Failed to acquire read lock: {}", e)))?;
        if let Some(value) = active.get(key) {
            return Ok(Some(value.clone()));
        }

        // Then check flushed segments (most recent first)
        let flushed = self
            .flushed_segments
            .read()
            .map_err(|e| ScribeError::Other(format!("Failed to acquire read lock: {}", e)))?;
        for segment in flushed.iter().rev() {
            if let Some(value) = segment.get(key) {
                return Ok(Some(value.clone()));
            }
        }

        Ok(None)
    }

    /// Get the number of flushed segments
    pub fn flushed_count(&self) -> Result<usize> {
        let flushed = self
            .flushed_segments
            .read()
            .map_err(|e| ScribeError::Other(format!("Failed to acquire read lock: {}", e)))?;
        Ok(flushed.len())
    }

    /// Get all flushed segments
    pub fn get_flushed_segments(&self) -> Result<Vec<Segment>> {
        let flushed = self
            .flushed_segments
            .read()
            .map_err(|e| ScribeError::Other(format!("Failed to acquire read lock: {}", e)))?;
        Ok(flushed.clone())
    }

    /// Clear all flushed segments (e.g., after successful S3 upload)
    pub fn clear_flushed(&self) -> Result<()> {
        let mut flushed = self
            .flushed_segments
            .write()
            .map_err(|e| ScribeError::Other(format!("Failed to acquire write lock: {}", e)))?;
        flushed.clear();
        Ok(())
    }

    /// Force flush the active segment
    pub fn flush_active(&self) -> Result<()> {
        let mut active = self
            .active_segment
            .write()
            .map_err(|e| ScribeError::Other(format!("Failed to acquire write lock: {}", e)))?;

        if !active.segment().is_empty() {
            let segment_id = self.next_segment_id.fetch_add(1, Ordering::SeqCst);
            let old_segment = std::mem::replace(
                &mut *active,
                PendingSegment::with_threshold(segment_id, self.size_threshold),
            );

            let mut flushed = self
                .flushed_segments
                .write()
                .map_err(|e| ScribeError::Other(format!("Failed to acquire write lock: {}", e)))?;
            flushed.push(old_segment.into_segment());
        }

        Ok(())
    }
}

impl Default for SegmentManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the current Unix timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_new() {
        let segment = Segment::new(1);
        assert_eq!(segment.segment_id, 1);
        assert_eq!(segment.size, 0);
        assert!(segment.is_empty());
    }

    #[test]
    fn test_segment_put_get() {
        let mut segment = Segment::new(1);
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();

        segment.put(key.clone(), value.clone());

        assert_eq!(segment.get(&key), Some(&value));
        assert_eq!(segment.len(), 1);
        assert_eq!(segment.size, key.len() + value.len());
    }

    #[test]
    fn test_segment_overwrite() {
        let mut segment = Segment::new(1);
        let key = b"key".to_vec();
        let value1 = b"value1".to_vec();
        let value2 = b"value2_longer".to_vec();

        segment.put(key.clone(), value1.clone());
        let size1 = segment.size;

        segment.put(key.clone(), value2.clone());
        let size2 = segment.size;

        assert_eq!(segment.get(&key), Some(&value2));
        assert_eq!(segment.len(), 1);
        assert_eq!(size1, key.len() + value1.len());
        assert_eq!(size2, key.len() + value2.len());
    }

    #[test]
    fn test_segment_remove() {
        let mut segment = Segment::new(1);
        let key = b"key".to_vec();
        let value = b"value".to_vec();

        segment.put(key.clone(), value.clone());
        assert_eq!(segment.size, key.len() + value.len());

        let removed = segment.remove(&key);
        assert_eq!(removed, Some(value));
        assert_eq!(segment.size, 0);
        assert!(segment.is_empty());
    }

    #[test]
    fn test_segment_serialization() {
        let mut segment = Segment::new(1);
        segment.put(b"key1".to_vec(), b"value1".to_vec());
        segment.put(b"key2".to_vec(), b"value2".to_vec());

        let bytes = segment.serialize().unwrap();
        let deserialized = Segment::deserialize(&bytes).unwrap();

        assert_eq!(deserialized.segment_id, segment.segment_id);
        assert_eq!(deserialized.len(), segment.len());
        assert_eq!(deserialized.size, segment.size);
        assert_eq!(deserialized.data, segment.data);
    }

    #[test]
    fn test_segment_from_data() {
        let mut data = HashMap::new();
        data.insert(b"key1".to_vec(), b"value1".to_vec());
        data.insert(b"key2".to_vec(), b"value2".to_vec());

        let segment = Segment::from_data(1, data.clone());

        assert_eq!(segment.segment_id, 1);
        assert_eq!(segment.len(), 2);
        assert_eq!(segment.data, data);
        assert!(segment.size > 0);
    }

    #[test]
    fn test_pending_segment_new() {
        let pending = PendingSegment::new(1);
        assert_eq!(pending.size(), 0);
        assert!(!pending.should_flush());
    }

    #[test]
    fn test_pending_segment_put_get() {
        let mut pending = PendingSegment::new(1);
        let key = b"key".to_vec();
        let value = b"value".to_vec();

        pending.put(key.clone(), value.clone());

        assert_eq!(pending.get(&key), Some(&value));
        assert_eq!(pending.size(), key.len() + value.len());
    }

    #[test]
    fn test_pending_segment_threshold() {
        let threshold = 100;
        let mut pending = PendingSegment::with_threshold(1, threshold);

        // Add data below threshold
        pending.put(b"key".to_vec(), vec![0u8; 50]);
        assert!(!pending.should_flush());

        // Add more data to exceed threshold
        pending.put(b"key2".to_vec(), vec![0u8; 60]);
        assert!(pending.should_flush());
    }

    #[test]
    fn test_pending_segment_into_segment() {
        let mut pending = PendingSegment::new(1);
        pending.put(b"key".to_vec(), b"value".to_vec());

        let segment = pending.into_segment();
        assert_eq!(segment.segment_id, 1);
        assert_eq!(segment.len(), 1);
    }

    #[test]
    fn test_segment_manager_new() {
        let manager = SegmentManager::new();
        assert_eq!(manager.flushed_count().unwrap(), 0);
    }

    #[test]
    fn test_segment_manager_put_get() {
        let manager = SegmentManager::new();
        let key = b"key".to_vec();
        let value = b"value".to_vec();

        manager.put(key.clone(), value.clone()).unwrap();

        let result = manager.get(&key).unwrap();
        assert_eq!(result, Some(value));
    }

    #[test]
    fn test_segment_manager_auto_flush() {
        let threshold = 100;
        let manager = SegmentManager::with_threshold(threshold);

        // Add data that exceeds threshold
        manager.put(b"key1".to_vec(), vec![0u8; 60]).unwrap();
        assert_eq!(manager.flushed_count().unwrap(), 0);

        manager.put(b"key2".to_vec(), vec![0u8; 60]).unwrap();
        assert_eq!(manager.flushed_count().unwrap(), 1);
    }

    #[test]
    fn test_segment_manager_manual_flush() {
        let manager = SegmentManager::new();

        manager.put(b"key".to_vec(), b"value".to_vec()).unwrap();
        assert_eq!(manager.flushed_count().unwrap(), 0);

        manager.flush_active().unwrap();
        assert_eq!(manager.flushed_count().unwrap(), 1);
    }

    #[test]
    fn test_segment_manager_get_from_flushed() {
        let threshold = 100;
        let manager = SegmentManager::with_threshold(threshold);

        let key1 = b"key1".to_vec();
        let value1 = vec![0u8; 60];
        let key2 = b"key2".to_vec();
        let value2 = vec![1u8; 60];

        manager.put(key1.clone(), value1.clone()).unwrap();
        manager.put(key2.clone(), value2.clone()).unwrap();

        // Both keys should be retrievable even after flush
        assert_eq!(manager.get(&key1).unwrap(), Some(value1));
        assert_eq!(manager.get(&key2).unwrap(), Some(value2));
    }

    #[test]
    fn test_segment_manager_clear_flushed() {
        let manager = SegmentManager::new();

        manager.put(b"key".to_vec(), b"value".to_vec()).unwrap();
        manager.flush_active().unwrap();
        assert_eq!(manager.flushed_count().unwrap(), 1);

        manager.clear_flushed().unwrap();
        assert_eq!(manager.flushed_count().unwrap(), 0);
    }

    #[test]
    fn test_segment_manager_get_flushed_segments() {
        let manager = SegmentManager::new();

        manager.put(b"key1".to_vec(), b"value1".to_vec()).unwrap();
        manager.flush_active().unwrap();
        manager.put(b"key2".to_vec(), b"value2".to_vec()).unwrap();
        manager.flush_active().unwrap();

        let flushed = manager.get_flushed_segments().unwrap();
        assert_eq!(flushed.len(), 2);
        assert_eq!(flushed[0].segment_id, 0);
        assert_eq!(flushed[1].segment_id, 1);
    }

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        assert!(ts > 0);
        assert!(ts < u64::MAX);
    }
}
