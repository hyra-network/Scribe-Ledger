//! Segment archival manager for automatic S3 archival and data tiering
//!
//! This module provides automatic segment archival to S3 with compression,
//! read-through caching, and tiering policies based on age and access patterns.

use crate::error::{Result, ScribeError};
use crate::storage::s3::{S3Storage, S3StorageConfig};
use crate::storage::segment::{Segment, SegmentManager};
use crate::types::SegmentId;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;

/// Default tiering age threshold in seconds (1 hour)
const DEFAULT_TIERING_AGE_SECS: u64 = 3600;

/// Default archival check interval in seconds (5 minutes)
const DEFAULT_ARCHIVAL_INTERVAL_SECS: u64 = 300;

/// Tiering policy configuration
#[derive(Debug, Clone)]
pub struct TieringPolicy {
    /// Age threshold in seconds for archiving segments
    pub age_threshold_secs: u64,
    /// Enable compression for archived segments
    pub enable_compression: bool,
    /// Compression level (0-9, where 9 is maximum compression)
    pub compression_level: u32,
    /// Enable automatic archival background task
    pub enable_auto_archival: bool,
    /// Interval for checking segments to archive (seconds)
    pub archival_check_interval_secs: u64,
}

impl Default for TieringPolicy {
    fn default() -> Self {
        Self {
            age_threshold_secs: DEFAULT_TIERING_AGE_SECS,
            enable_compression: true,
            compression_level: 6,
            enable_auto_archival: true,
            archival_check_interval_secs: DEFAULT_ARCHIVAL_INTERVAL_SECS,
        }
    }
}

/// Segment metadata stored alongside segments in S3
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SegmentMetadata {
    /// Segment ID
    pub segment_id: SegmentId,
    /// Creation timestamp
    pub created_at: u64,
    /// Archival timestamp
    pub archived_at: u64,
    /// Original size before compression
    pub original_size: usize,
    /// Compressed size
    pub compressed_size: usize,
    /// Whether the segment is compressed
    pub is_compressed: bool,
    /// Number of key-value pairs
    pub entry_count: usize,
}

/// Archival manager for automatic segment archival to S3
pub struct ArchivalManager {
    /// S3 storage backend
    s3_storage: Arc<S3Storage>,
    /// Segment manager for local segments
    segment_manager: Arc<SegmentManager>,
    /// Tiering policy
    policy: TieringPolicy,
    /// Cache for recently accessed segments
    segment_cache: Arc<RwLock<HashMap<SegmentId, Segment>>>,
    /// Cache for segment metadata
    metadata_cache: Arc<RwLock<HashMap<SegmentId, SegmentMetadata>>>,
}

impl ArchivalManager {
    /// Create a new archival manager
    pub async fn new(
        s3_config: S3StorageConfig,
        segment_manager: Arc<SegmentManager>,
        policy: TieringPolicy,
    ) -> Result<Self> {
        let s3_storage = Arc::new(S3Storage::new(s3_config).await?);

        Ok(Self {
            s3_storage,
            segment_manager,
            policy,
            segment_cache: Arc::new(RwLock::new(HashMap::new())),
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Archive a segment to S3 with optional compression
    pub async fn archive_segment(&self, segment: &Segment) -> Result<SegmentMetadata> {
        let original_size = segment.size;
        let entry_count = segment.len();

        // Serialize segment
        let data = segment.serialize()?;

        // Compress if enabled
        let (final_data, is_compressed, compressed_size) = if self.policy.enable_compression {
            let compressed = self.compress_data(&data)?;
            let compressed_size = compressed.len();
            (compressed, true, compressed_size)
        } else {
            let size = data.len();
            (data, false, size)
        };

        // Create metadata
        let metadata = SegmentMetadata {
            segment_id: segment.segment_id,
            created_at: segment.timestamp,
            archived_at: current_timestamp(),
            original_size,
            compressed_size,
            is_compressed,
            entry_count,
        };

        // Store segment data
        self.s3_storage
            .put_object(&Self::segment_key(segment.segment_id), final_data)
            .await?;

        // Store metadata
        let metadata_json =
            serde_json::to_vec(&metadata).map_err(|e| ScribeError::Serialization(e.to_string()))?;
        self.s3_storage
            .put_object(&Self::metadata_key(segment.segment_id), metadata_json)
            .await?;

        // Cache metadata
        self.metadata_cache
            .write()
            .await
            .insert(segment.segment_id, metadata.clone());

        Ok(metadata)
    }

    /// Retrieve a segment from S3 with decompression
    pub async fn retrieve_segment(&self, segment_id: SegmentId) -> Result<Option<Segment>> {
        // Check cache first
        {
            let cache = self.segment_cache.read().await;
            if let Some(segment) = cache.get(&segment_id) {
                return Ok(Some(segment.clone()));
            }
        }

        // Get metadata
        let metadata = self.get_metadata(segment_id).await?;
        if metadata.is_none() {
            return Ok(None);
        }
        let metadata = metadata.unwrap();

        // Get segment data from S3
        let data = self
            .s3_storage
            .get_object(&Self::segment_key(segment_id))
            .await?;

        if data.is_none() {
            return Ok(None);
        }
        let data = data.unwrap();

        // Decompress if needed
        let final_data = if metadata.is_compressed {
            self.decompress_data(&data)?
        } else {
            data
        };

        // Deserialize segment
        let segment = Segment::deserialize(&final_data)?;

        // Cache the segment
        self.segment_cache
            .write()
            .await
            .insert(segment_id, segment.clone());

        Ok(Some(segment))
    }

    /// Get metadata for a segment
    pub async fn get_metadata(&self, segment_id: SegmentId) -> Result<Option<SegmentMetadata>> {
        // Check cache first
        {
            let cache = self.metadata_cache.read().await;
            if let Some(metadata) = cache.get(&segment_id) {
                return Ok(Some(metadata.clone()));
            }
        }

        // Get from S3
        let data = self
            .s3_storage
            .get_object(&Self::metadata_key(segment_id))
            .await?;

        if data.is_none() {
            return Ok(None);
        }

        let metadata: SegmentMetadata = serde_json::from_slice(&data.unwrap())
            .map_err(|e| ScribeError::Serialization(e.to_string()))?;

        // Cache metadata
        self.metadata_cache
            .write()
            .await
            .insert(segment_id, metadata.clone());

        Ok(Some(metadata))
    }

    /// Archive old segments based on tiering policy
    pub async fn archive_old_segments(&self) -> Result<Vec<SegmentId>> {
        let mut archived_ids = Vec::new();
        let now = current_timestamp();
        let threshold = now.saturating_sub(self.policy.age_threshold_secs);

        // Get flushed segments from segment manager
        let segments = self.segment_manager.get_flushed_segments()?;

        for segment in segments {
            if segment.timestamp < threshold {
                // Archive the segment
                self.archive_segment(&segment).await?;
                archived_ids.push(segment.segment_id);
            }
        }

        // Clear archived segments from local storage
        if !archived_ids.is_empty() {
            self.segment_manager.clear_flushed()?;
        }

        Ok(archived_ids)
    }

    /// Start automatic archival background task
    pub fn start_auto_archival(&self) -> tokio::task::JoinHandle<()> {
        let manager = self.clone_arc();
        let interval_secs = self.policy.archival_check_interval_secs;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(interval_secs));

            loop {
                ticker.tick().await;

                if let Err(e) = manager.archive_old_segments().await {
                    eprintln!("Archival error: {}", e);
                }
            }
        })
    }

    /// Read-through: Get value from local or S3
    pub async fn get_value(&self, segment_id: SegmentId, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // Try local segment manager first
        if let Ok(Some(value)) = self.segment_manager.get(&key.to_vec()) {
            return Ok(Some(value));
        }

        // Try S3
        if let Some(segment) = self.retrieve_segment(segment_id).await? {
            return Ok(segment.get(&key.to_vec()).cloned());
        }

        Ok(None)
    }

    /// List all archived segment IDs
    pub async fn list_archived_segments(&self) -> Result<Vec<SegmentId>> {
        self.s3_storage.list_segments().await
    }

    /// Delete archived segment
    pub async fn delete_archived_segment(&self, segment_id: SegmentId) -> Result<()> {
        // Delete segment data
        self.s3_storage
            .delete_object(&Self::segment_key(segment_id))
            .await?;

        // Delete metadata
        self.s3_storage
            .delete_object(&Self::metadata_key(segment_id))
            .await?;

        // Remove from cache
        self.segment_cache.write().await.remove(&segment_id);
        self.metadata_cache.write().await.remove(&segment_id);

        Ok(())
    }

    /// Compress data using gzip
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut encoder =
            GzEncoder::new(Vec::new(), Compression::new(self.policy.compression_level));
        encoder
            .write_all(data)
            .map_err(|e| ScribeError::Other(format!("Compression error: {}", e)))?;
        encoder
            .finish()
            .map_err(|e| ScribeError::Other(format!("Compression error: {}", e)))
    }

    /// Decompress data using gzip
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| ScribeError::Other(format!("Decompression error: {}", e)))?;
        Ok(decompressed)
    }

    /// Generate S3 key for segment data
    fn segment_key(segment_id: SegmentId) -> String {
        format!("segments/segment-{:016x}.bin", segment_id)
    }

    /// Generate S3 key for segment metadata
    fn metadata_key(segment_id: SegmentId) -> String {
        format!("segments/segment-{:016x}.meta.json", segment_id)
    }

    /// Clone as Arc for background tasks
    fn clone_arc(&self) -> Arc<Self> {
        Arc::new(Self {
            s3_storage: self.s3_storage.clone(),
            segment_manager: self.segment_manager.clone(),
            policy: self.policy.clone(),
            segment_cache: self.segment_cache.clone(),
            metadata_cache: self.metadata_cache.clone(),
        })
    }
}

/// Get current Unix timestamp
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
    fn test_default_tiering_policy() {
        let policy = TieringPolicy::default();
        assert_eq!(policy.age_threshold_secs, DEFAULT_TIERING_AGE_SECS);
        assert!(policy.enable_compression);
        assert_eq!(policy.compression_level, 6);
        assert!(policy.enable_auto_archival);
    }

    #[test]
    fn test_segment_metadata_serialization() {
        let metadata = SegmentMetadata {
            segment_id: 42,
            created_at: 1000,
            archived_at: 2000,
            original_size: 1024,
            compressed_size: 512,
            is_compressed: true,
            entry_count: 10,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: SegmentMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.segment_id, metadata.segment_id);
        assert_eq!(deserialized.original_size, metadata.original_size);
        assert_eq!(deserialized.compressed_size, metadata.compressed_size);
        assert!(deserialized.is_compressed);
    }

    #[test]
    fn test_segment_key_generation() {
        let key = ArchivalManager::segment_key(42);
        assert_eq!(key, "segments/segment-000000000000002a.bin");
    }

    #[test]
    fn test_metadata_key_generation() {
        let key = ArchivalManager::metadata_key(42);
        assert_eq!(key, "segments/segment-000000000000002a.meta.json");
    }
}
