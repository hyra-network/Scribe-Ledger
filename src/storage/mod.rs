/// Storage backends for Scribe Ledger
pub mod s3;

pub use s3::S3Storage;

use crate::error::Result;
use crate::types::{SegmentId, SegmentMetadata};

use std::future::Future;
use std::pin::Pin;

/// Trait for storage backends
pub trait StorageBackend {
    /// Store a segment
    fn store_segment(&self, segment_id: SegmentId, data: &[u8]) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    
    /// Retrieve a segment
    fn get_segment(&self, segment_id: SegmentId) -> Pin<Box<dyn Future<Output = Result<Vec<u8>>> + Send + '_>>;
    
    /// Check if a segment exists
    fn segment_exists(&self, segment_id: SegmentId) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + '_>>;
    
    /// List all segments
    fn list_segments(&self) -> Pin<Box<dyn Future<Output = Result<Vec<SegmentMetadata>>> + Send + '_>>;
}