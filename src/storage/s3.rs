//! S3 storage backend for cold data and segment archival
//!
//! This module provides an S3-compatible storage backend for archiving segments
//! to object storage. It supports both AWS S3 and MinIO for local development.

use crate::error::{Result, ScribeError};
use crate::storage::segment::Segment;
use crate::types::SegmentId;
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use bytes::Bytes;
use std::sync::Arc;
use std::time::Duration;

/// S3 storage backend configuration
#[derive(Debug, Clone)]
pub struct S3StorageConfig {
    /// S3 bucket name
    pub bucket: String,
    /// S3 region
    pub region: String,
    /// S3 endpoint URL (for MinIO compatibility)
    pub endpoint: Option<String>,
    /// Access key ID
    pub access_key_id: Option<String>,
    /// Secret access key
    pub secret_access_key: Option<String>,
    /// Enable path-style addressing (required for MinIO)
    pub path_style: bool,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retry attempts
    pub max_retries: u32,
}

impl Default for S3StorageConfig {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            region: "us-east-1".to_string(),
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
            path_style: false,
            timeout_secs: 30,
            max_retries: 3,
        }
    }
}

/// S3 storage backend for segment archival
///
/// This backend provides async operations for storing and retrieving segments
/// from S3-compatible object storage. It includes connection pooling, retry logic,
/// and support for MinIO for local development.
#[derive(Debug)]
pub struct S3Storage {
    client: Arc<S3Client>,
    bucket: String,
    max_retries: u32,
}

impl S3Storage {
    /// Create a new S3 storage backend
    ///
    /// # Arguments
    ///
    /// * `config` - S3 storage configuration
    ///
    /// # Returns
    ///
    /// A new S3Storage instance or an error if configuration is invalid
    pub async fn new(config: S3StorageConfig) -> Result<Self> {
        if config.bucket.is_empty() {
            return Err(ScribeError::Configuration(
                "S3 bucket name cannot be empty".to_string(),
            ));
        }

        let client = Self::create_client(&config).await?;

        Ok(Self {
            client: Arc::new(client),
            bucket: config.bucket,
            max_retries: config.max_retries,
        })
    }

    /// Create an S3 client with the given configuration
    async fn create_client(config: &S3StorageConfig) -> Result<S3Client> {
        let mut aws_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()));

        // Set custom credentials if provided
        if let (Some(access_key), Some(secret_key)) =
            (&config.access_key_id, &config.secret_access_key)
        {
            let credentials =
                Credentials::new(access_key.clone(), secret_key.clone(), None, None, "static");
            aws_config = aws_config.credentials_provider(credentials);
        }

        let sdk_config = aws_config.load().await;
        let mut s3_config = aws_sdk_s3::config::Builder::from(&sdk_config).timeout_config(
            aws_sdk_s3::config::timeout::TimeoutConfig::builder()
                .operation_timeout(Duration::from_secs(config.timeout_secs))
                .build(),
        );

        // Set custom endpoint if provided (for MinIO)
        if let Some(endpoint) = &config.endpoint {
            s3_config = s3_config.endpoint_url(endpoint);
        }

        // Enable path-style addressing for MinIO compatibility
        if config.path_style {
            s3_config = s3_config.force_path_style(true);
        }

        Ok(S3Client::from_conf(s3_config.build()))
    }

    /// Upload a segment to S3
    ///
    /// # Arguments
    ///
    /// * `segment` - The segment to upload
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the upload fails
    pub async fn put_segment(&self, segment: &Segment) -> Result<()> {
        let key = Self::segment_key(segment.segment_id);
        let data = segment.serialize()?;

        self.put_with_retry(&key, data).await
    }

    /// Put an object to S3 with a custom key
    ///
    /// # Arguments
    ///
    /// * `key` - The S3 object key
    /// * `data` - The data to upload
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the upload fails
    pub async fn put_object(&self, key: &str, data: Vec<u8>) -> Result<()> {
        self.put_with_retry(key, data).await
    }

    /// Download a segment from S3
    ///
    /// # Arguments
    ///
    /// * `segment_id` - The ID of the segment to retrieve
    ///
    /// # Returns
    ///
    /// The segment if found, None if not found, or an error
    pub async fn get_segment(&self, segment_id: SegmentId) -> Result<Option<Segment>> {
        let key = Self::segment_key(segment_id);

        match self.get_with_retry(&key).await {
            Ok(data) => {
                let segment = Segment::deserialize(&data)?;
                Ok(Some(segment))
            }
            Err(ScribeError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Get an object from S3 with a custom key
    ///
    /// # Arguments
    ///
    /// * `key` - The S3 object key
    ///
    /// # Returns
    ///
    /// The data if found, None if not found, or an error
    pub async fn get_object(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self.get_with_retry(key).await {
            Ok(data) => Ok(Some(data)),
            Err(ScribeError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Delete a segment from S3
    ///
    /// # Arguments
    ///
    /// * `segment_id` - The ID of the segment to delete
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the deletion fails
    pub async fn delete_segment(&self, segment_id: SegmentId) -> Result<()> {
        let key = Self::segment_key(segment_id);

        self.delete_with_retry(&key).await
    }

    /// Delete an object from S3 with a custom key
    ///
    /// # Arguments
    ///
    /// * `key` - The S3 object key
    ///
    /// # Returns
    ///
    /// Ok(()) on success, or an error if the deletion fails
    pub async fn delete_object(&self, key: &str) -> Result<()> {
        self.delete_with_retry(key).await
    }

    /// List all segment IDs in S3
    ///
    /// # Returns
    ///
    /// A vector of segment IDs, or an error
    pub async fn list_segments(&self) -> Result<Vec<SegmentId>> {
        let mut segment_ids = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix("segments/");

            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }

            let response = request
                .send()
                .await
                .map_err(|e| ScribeError::Storage(format!("Failed to list S3 objects: {}", e)))?;

            for object in response.contents() {
                if let Some(key) = object.key() {
                    if let Some(segment_id) = Self::parse_segment_key(key) {
                        segment_ids.push(segment_id);
                    }
                }
            }

            if response.is_truncated().unwrap_or(false) {
                continuation_token = response.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(segment_ids)
    }

    /// Check if the S3 bucket is accessible
    ///
    /// # Returns
    ///
    /// Ok(()) if the bucket is accessible, or an error
    pub async fn health_check(&self) -> Result<()> {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .map_err(|e| ScribeError::Storage(format!("S3 health check failed: {}", e)))?;

        Ok(())
    }

    /// Generate S3 key for a segment
    fn segment_key(segment_id: SegmentId) -> String {
        format!("segments/segment-{:016x}.bin", segment_id)
    }

    /// Parse segment ID from S3 key
    fn parse_segment_key(key: &str) -> Option<SegmentId> {
        if let Some(filename) = key.strip_prefix("segments/segment-") {
            if let Some(id_str) = filename.strip_suffix(".bin") {
                return SegmentId::from_str_radix(id_str, 16).ok();
            }
        }
        None
    }

    /// Put data to S3 with retry logic
    async fn put_with_retry(&self, key: &str, data: Vec<u8>) -> Result<()> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let backoff = Duration::from_millis(100 * (1 << (attempt - 1)));
                tokio::time::sleep(backoff).await;
            }

            match self
                .client
                .put_object()
                .bucket(&self.bucket)
                .key(key)
                .body(ByteStream::from(Bytes::from(data.clone())))
                .send()
                .await
            {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(ScribeError::Storage(format!(
            "Failed to put S3 object after {} retries: {}",
            self.max_retries,
            last_error.unwrap()
        )))
    }

    /// Get data from S3 with retry logic
    async fn get_with_retry(&self, key: &str) -> Result<Vec<u8>> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let backoff = Duration::from_millis(100 * (1 << (attempt - 1)));
                tokio::time::sleep(backoff).await;
            }

            match self
                .client
                .get_object()
                .bucket(&self.bucket)
                .key(key)
                .send()
                .await
            {
                Ok(response) => {
                    let data = response
                        .body
                        .collect()
                        .await
                        .map_err(|e| {
                            ScribeError::Storage(format!("Failed to read S3 object body: {}", e))
                        })?
                        .to_vec();
                    return Ok(data);
                }
                Err(e) => {
                    // Check if it's a "not found" error
                    if e.to_string().contains("NoSuchKey") || e.to_string().contains("NotFound") {
                        return Err(ScribeError::NotFound(format!(
                            "S3 object not found: {}",
                            key
                        )));
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(ScribeError::Storage(format!(
            "Failed to get S3 object after {} retries: {}",
            self.max_retries,
            last_error.unwrap()
        )))
    }

    /// Delete data from S3 with retry logic
    async fn delete_with_retry(&self, key: &str) -> Result<()> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                let backoff = Duration::from_millis(100 * (1 << (attempt - 1)));
                tokio::time::sleep(backoff).await;
            }

            match self
                .client
                .delete_object()
                .bucket(&self.bucket)
                .key(key)
                .send()
                .await
            {
                Ok(_) => return Ok(()),
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(ScribeError::Storage(format!(
            "Failed to delete S3 object after {} retries: {}",
            self.max_retries,
            last_error.unwrap()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_key_generation() {
        let segment_id = 42;
        let key = S3Storage::segment_key(segment_id);
        assert_eq!(key, "segments/segment-000000000000002a.bin");
    }

    #[test]
    fn test_parse_segment_key() {
        let key = "segments/segment-000000000000002a.bin";
        let segment_id = S3Storage::parse_segment_key(key);
        assert_eq!(segment_id, Some(42));
    }

    #[test]
    fn test_parse_invalid_segment_key() {
        assert_eq!(S3Storage::parse_segment_key("invalid"), None);
        assert_eq!(S3Storage::parse_segment_key("segments/invalid.bin"), None);
        assert_eq!(S3Storage::parse_segment_key("segments/segment-.bin"), None);
    }

    #[test]
    fn test_default_config() {
        let config = S3StorageConfig::default();
        assert_eq!(config.region, "us-east-1");
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
        assert!(!config.path_style);
    }

    #[tokio::test]
    async fn test_new_s3storage_empty_bucket() {
        let config = S3StorageConfig {
            bucket: String::new(),
            ..Default::default()
        };

        let result = S3Storage::new(config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bucket"));
    }
}
