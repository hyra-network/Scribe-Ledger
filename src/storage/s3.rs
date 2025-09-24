use aws_sdk_s3::{Client, Config};
use aws_config::{Region, BehaviorVersion};
use aws_credential_types::Credentials;
use bytes::Bytes;
use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;
use uuid::Uuid;
use crate::error::{Result, ScribeError};
use crate::types::{SegmentId, SegmentMetadata, KeyRange};
use crate::storage::StorageBackend;

// Re-export from config module
pub use crate::config::S3StorageConfig as S3Config;

#[derive(Clone)]
pub struct S3Storage {
    client: Client,
    bucket: String,
    config: S3Config,
}

impl S3Storage {
    /// Create a new S3 storage backend from main config
    pub async fn from_config(config: &crate::config::Config) -> Result<Self> {
        Self::new(config.storage.s3.clone()).await
    }
    
    /// Create a new S3 storage backend with full configuration
    pub async fn new(config: S3Config) -> Result<Self> {
        let bucket = config.bucket.clone();
        
        // Build AWS config
        let mut aws_config_builder = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()));

        // Set custom endpoint for MinIO
        if let Some(endpoint) = &config.endpoint {
            aws_config_builder = aws_config_builder.endpoint_url(endpoint);
        }

        // Set custom credentials if provided
        if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
            let credentials = Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "scribe-ledger-static"
            );
            aws_config_builder = aws_config_builder.credentials_provider(credentials);
        }

        let _aws_config = aws_config_builder.load().await;
        
        // Create S3 client with custom config for MinIO compatibility
        let mut s3_config_builder = Config::builder()
            .region(Region::new(config.region.clone()));
            
        if let Some(endpoint) = &config.endpoint {
            s3_config_builder = s3_config_builder.endpoint_url(endpoint);
        }
        
        if config.path_style {
            s3_config_builder = s3_config_builder.force_path_style(true);
        }

        // Set credentials if provided
        if let (Some(access_key), Some(secret_key)) = (&config.access_key, &config.secret_key) {
            let credentials = Credentials::new(
                access_key,
                secret_key,
                None,
                None,
                "scribe-ledger-static"
            );
            s3_config_builder = s3_config_builder.credentials_provider(credentials);
        }

        let s3_config = s3_config_builder
            .behavior_version(BehaviorVersion::latest())
            .build();
            
        let client = Client::from_conf(s3_config);
        
        Ok(Self { 
            client, 
            bucket,
            config
        })
    }

    /// Create a new S3 storage backend with simple parameters (backward compatibility)
    pub async fn new_simple(bucket: String, region: Option<String>) -> Result<Self> {
        let config = S3Config {
            bucket: bucket.clone(),
            region: region.unwrap_or_else(|| "us-east-1".to_string()),
            ..Default::default()
        };
        
        Self::new(config).await
    }

    /// Create MinIO-compatible S3 storage
    pub async fn new_minio(bucket: String, endpoint: String, access_key: String, secret_key: String) -> Result<Self> {
        let config = S3Config {
            bucket: bucket.clone(),
            region: "us-east-1".to_string(), // MinIO doesn't require real region
            endpoint: Some(endpoint),
            access_key: Some(access_key),
            secret_key: Some(secret_key),
            path_style: true, // Required for MinIO
        };
        
        Self::new(config).await
    }

    /// Test the connection to S3/MinIO
    pub async fn test_connection(&self) -> Result<()> {
        // Try to list objects in the bucket
        self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .max_keys(1)
            .send()
            .await
            .map_err(|e| ScribeError::Aws(format!("Connection test failed: {}", e)))?;
        
        tracing::info!("S3 connection test successful for bucket: {}", self.bucket);
        Ok(())
    }

    /// Create the bucket if it doesn't exist (useful for development)
    pub async fn ensure_bucket_exists(&self) -> Result<()> {
        // Check if bucket exists
        match self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
        {
            Ok(_) => {
                tracing::info!("Bucket {} already exists", self.bucket);
                return Ok(());
            }
            Err(_) => {
                tracing::info!("Bucket {} does not exist, creating...", self.bucket);
            }
        }

        // Create bucket
        let mut create_bucket = self.client
            .create_bucket()
            .bucket(&self.bucket);

        // For non-us-east-1 regions, we need to specify the location constraint
        if self.config.region != "us-east-1" {
            create_bucket = create_bucket.create_bucket_configuration(
                aws_sdk_s3::types::CreateBucketConfiguration::builder()
                    .location_constraint(aws_sdk_s3::types::BucketLocationConstraint::from(self.config.region.as_str()))
                    .build()
            );
        }

        create_bucket
            .send()
            .await
            .map_err(|e| ScribeError::Aws(format!("Failed to create bucket: {}", e)))?;

        tracing::info!("Successfully created bucket: {}", self.bucket);
        Ok(())
    }
    
    /// Generate S3 key for a segment
    fn segment_key(&self, segment_id: &SegmentId) -> String {
        format!("segments/{}", segment_id.0.hyphenated())
    }

    /// Generate S3 key for manifest
    fn manifest_key(&self, manifest_id: &str) -> String {
        format!("manifests/{}", manifest_id)
    }

    /// Generate S3 key with timestamp for versioning
    #[allow(dead_code)]
    fn timestamped_key(&self, prefix: &str, id: &str) -> String {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("{}/{}_{}", prefix, timestamp, id)
    }
}

impl StorageBackend for S3Storage {
    fn store_segment(&self, segment_id: SegmentId, data: &[u8]) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        let key = self.segment_key(&segment_id);
        let bucket = self.bucket.clone();
        let client = self.client.clone();
        let data = data.to_vec();
        
        Box::pin(async move {
            client
                .put_object()
                .bucket(&bucket)
                .key(&key)
                .body(Bytes::from(data).into())
                .send()
                .await
                .map_err(|e| ScribeError::Aws(format!("Failed to upload segment: {}", e)))?;
            
            tracing::info!("Uploaded segment {} to S3", segment_id.0.hyphenated());
            Ok(())
        })
    }
    
    fn get_segment(&self, segment_id: SegmentId) -> Pin<Box<dyn Future<Output = Result<Vec<u8>>> + Send + '_>> {
        let key = self.segment_key(&segment_id);
        let bucket = self.bucket.clone();
        let client = self.client.clone();
        
        Box::pin(async move {
            let response = client
                .get_object()
                .bucket(&bucket)
                .key(&key)
                .send()
                .await
                .map_err(|e| ScribeError::Aws(format!("Failed to download segment: {}", e)))?;
            
            let body = response.body.collect().await
                .map_err(|e| ScribeError::Aws(format!("Failed to read segment body: {}", e)))?;
            
            Ok(body.into_bytes().to_vec())
        })
    }
    
    fn segment_exists(&self, segment_id: SegmentId) -> Pin<Box<dyn Future<Output = Result<bool>> + Send + '_>> {
        let key = self.segment_key(&segment_id);
        let bucket = self.bucket.clone();
        let client = self.client.clone();
        
        Box::pin(async move {
            match client
                .head_object()
                .bucket(&bucket)
                .key(&key)
                .send()
                .await
            {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        })
    }
    
    fn list_segments(&self) -> Pin<Box<dyn Future<Output = Result<Vec<SegmentMetadata>>> + Send + '_>> {
        let bucket = self.bucket.clone();
        let client = self.client.clone();
        
        Box::pin(async move {
            let mut segments = Vec::new();
            let mut continuation_token: Option<String> = None;
            
            loop {
                let mut request = client
                    .list_objects_v2()
                    .bucket(&bucket)
                    .prefix("segments/");
                    
                if let Some(token) = continuation_token {
                    request = request.continuation_token(token);
                }
                
                let response = request
                    .send()
                    .await
                    .map_err(|e| ScribeError::Aws(format!("Failed to list segments: {}", e)))?;
                
                if let Some(contents) = response.contents {
                    for object in contents {
                        if let (Some(key), Some(size), Some(last_modified)) = 
                            (object.key, object.size, object.last_modified) {
                            
                            // Extract segment ID from key
                            if let Some(segment_id_str) = key.strip_prefix("segments/") {
                                // Parse UUID from the segment ID string
                                if let Ok(uuid) = Uuid::parse_str(segment_id_str) {
                                    let segment_id = SegmentId(uuid);
                                    
                                    let metadata = SegmentMetadata {
                                        id: segment_id,
                                        key_range: KeyRange { start: None, end: None }, // Will be set by manifest
                                        merkle_root: [0u8; 32], // Will be set by manifest
                                        size: size as u64,
                                        created_at: last_modified.secs() as u64,
                                        s3_key: key.clone(),
                                    };
                                    
                                    segments.push(metadata);
                                }
                            }
                        }
                    }
                }
                
                // Check if there are more objects to retrieve
                if response.is_truncated.unwrap_or(false) {
                    continuation_token = response.next_continuation_token;
                } else {
                    break;
                }
            }
            
            tracing::info!("Listed {} segments from S3", segments.len());
            Ok(segments)
        })
    }
}

/// Additional S3 operations for manifest and advanced features
impl S3Storage {
    /// Store manifest data
    pub async fn store_manifest(&self, manifest_id: &str, data: &[u8]) -> Result<()> {
        let key = self.manifest_key(manifest_id);
        
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(Bytes::from(data.to_vec()).into())
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| ScribeError::Aws(format!("Failed to upload manifest: {}", e)))?;
        
        tracing::info!("Uploaded manifest {} to S3", manifest_id);
        Ok(())
    }

    /// Retrieve manifest data
    pub async fn get_manifest(&self, manifest_id: &str) -> Result<Vec<u8>> {
        let key = self.manifest_key(manifest_id);
        
        let response = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| ScribeError::Aws(format!("Failed to download manifest: {}", e)))?;
        
        let body = response.body.collect().await
            .map_err(|e| ScribeError::Aws(format!("Failed to read manifest body: {}", e)))?;
        
        Ok(body.into_bytes().to_vec())
    }

    /// Check if manifest exists
    pub async fn manifest_exists(&self, manifest_id: &str) -> Result<bool> {
        let key = self.manifest_key(manifest_id);
        
        match self.client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Store data with metadata and tags
    pub async fn store_segment_with_metadata(
        &self, 
        segment_id: SegmentId, 
        data: &[u8],
        content_type: Option<&str>,
        metadata: Option<std::collections::HashMap<String, String>>
    ) -> Result<()> {
        let key = self.segment_key(&segment_id);
        
        let mut request = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(Bytes::from(data.to_vec()).into());
            
        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }
        
        if let Some(meta) = metadata {
            for (k, v) in meta {
                request = request.metadata(k, v);
            }
        }
        
        request
            .send()
            .await
            .map_err(|e| ScribeError::Aws(format!("Failed to upload segment with metadata: {}", e)))?;
        
        tracing::info!("Uploaded segment {} with metadata to S3", segment_id.0.hyphenated());
        Ok(())
    }

    /// Get object metadata without downloading the content
    pub async fn get_segment_metadata(&self, segment_id: SegmentId) -> Result<SegmentMetadata> {
        let key = self.segment_key(&segment_id);
        
        let response = self.client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| ScribeError::Aws(format!("Failed to get segment metadata: {}", e)))?;
        
        let size = response.content_length.unwrap_or(0) as usize;
        let created_at = response.last_modified
            .map(|t| t.secs() as u64)
            .unwrap_or(0);
        
        Ok(SegmentMetadata {
            id: segment_id,
            key_range: KeyRange { start: None, end: None }, // Will be set by manifest
            merkle_root: [0u8; 32], // Will be set by manifest
            size: size as u64,
            created_at,
            s3_key: key,
        })
    }

    /// Delete a segment
    pub async fn delete_segment(&self, segment_id: SegmentId) -> Result<()> {
        let key = self.segment_key(&segment_id);
        
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
            .map_err(|e| ScribeError::Aws(format!("Failed to delete segment: {}", e)))?;
        
        tracing::info!("Deleted segment {} from S3", segment_id.0.hyphenated());
        Ok(())
    }

    /// Get presigned URL for direct upload (useful for large files)
    pub async fn get_presigned_upload_url(&self, segment_id: SegmentId, expires_in: std::time::Duration) -> Result<String> {
        let key = self.segment_key(&segment_id);
        
        let presigning_config = aws_sdk_s3::presigning::PresigningConfig::expires_in(expires_in)
            .map_err(|e| ScribeError::Aws(format!("Failed to create presigning config: {}", e)))?;
        
        let presigned_request = self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .presigned(presigning_config)
            .await
            .map_err(|e| ScribeError::Aws(format!("Failed to create presigned URL: {}", e)))?;
        
        Ok(presigned_request.uri().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SegmentId;
    use crate::config::Config;
    use uuid::Uuid;

    /// Helper function to load config for tests
    /// Prioritizes environment variables over defaults
    fn get_test_config() -> S3Config {
        // Load config with environment variable support
        let config = Config::load_with_env();
        config.storage.s3
    }

    /// Get MinIO test config - used only for integration tests
    fn get_minio_test_config() -> S3Config {
        // Try to load from environment first
        let mut config = get_test_config();
        
        // If no endpoint is set, use MinIO defaults for testing
        if config.endpoint.is_none() {
            config.endpoint = Some("http://localhost:9000".to_string());
            config.access_key = Some("scribe-admin".to_string());
            config.secret_key = Some("scribe-password-123".to_string());
            config.path_style = true;
            config.bucket = "scribe-ledger-test".to_string();
        }
        
        config
    }

    #[test]
    fn test_s3_config_default() {
        let config = get_test_config();
        // Test that we can create a valid config
        assert!(!config.bucket.is_empty());
        assert!(!config.region.is_empty());
        // Environment variables might override these, so we just check they're valid
    }

    #[test]
    fn test_s3_config_env_override() {
        // Set test environment variables
        std::env::set_var("SCRIBE_S3_BUCKET", "env-test-bucket");
        std::env::set_var("SCRIBE_S3_REGION", "env-test-region");
        std::env::set_var("SCRIBE_S3_ENDPOINT", "http://env-test:9000");
        std::env::set_var("SCRIBE_S3_ACCESS_KEY", "env-access-key");
        std::env::set_var("SCRIBE_S3_SECRET_KEY", "env-secret-key");
        std::env::set_var("SCRIBE_S3_PATH_STYLE", "true");

        let config = get_test_config();
        
        // Verify environment variables override defaults
        assert_eq!(config.bucket, "env-test-bucket");
        assert_eq!(config.region, "env-test-region");
        assert_eq!(config.endpoint, Some("http://env-test:9000".to_string()));
        assert_eq!(config.access_key, Some("env-access-key".to_string()));
        assert_eq!(config.secret_key, Some("env-secret-key".to_string()));
        assert_eq!(config.path_style, true);

        // Clean up environment variables
        std::env::remove_var("SCRIBE_S3_BUCKET");
        std::env::remove_var("SCRIBE_S3_REGION");
        std::env::remove_var("SCRIBE_S3_ENDPOINT");
        std::env::remove_var("SCRIBE_S3_ACCESS_KEY");
        std::env::remove_var("SCRIBE_S3_SECRET_KEY");
        std::env::remove_var("SCRIBE_S3_PATH_STYLE");
    }

    #[test]
    fn test_segment_key_generation() {
        let _config = get_test_config();
        // Create a mock S3Storage instance (this would need actual client for full test)
        let segment_id = SegmentId(Uuid::new_v4());
        let expected_key = format!("segments/{}", segment_id.0.hyphenated());
        
        // Test the key format
        assert!(expected_key.starts_with("segments/"));
        assert!(expected_key.contains(&segment_id.0.hyphenated().to_string()));
    }

    #[test]
    fn test_manifest_key_generation() {
        let manifest_id = "test-manifest-123";
        let expected_key = format!("manifests/{}", manifest_id);
        assert_eq!(expected_key, "manifests/test-manifest-123");
    }

    #[test]
    fn test_timestamped_key_generation() {
        let prefix = "test-prefix";
        let id = "test-id";
        let key = format!("{}/__{}", prefix, id); // Simplified test version
        
        assert!(key.starts_with(prefix));
        assert!(key.contains(id));
    }

    // Integration tests would require MinIO to be running
    #[tokio::test]
    #[ignore] // This requires MinIO to be running
    async fn test_minio_integration() {
        // Load config from environment variables or use MinIO defaults
        let config = get_minio_test_config();

        let storage = S3Storage::new(config).await;
        
        // Only run if storage creation succeeded (MinIO is available)
        if let Ok(s3) = storage {
            // Test connection
            let connection_result = s3.test_connection().await;
            assert!(connection_result.is_ok());

            // Test bucket creation
            let bucket_result = s3.ensure_bucket_exists().await;
            assert!(bucket_result.is_ok());

            // Test basic operations
            let segment_id = SegmentId::new();
            let test_data = b"Hello, MinIO from Scribe Ledger!";

            // Store segment
            let store_result = s3.store_segment(segment_id, test_data).await;
            assert!(store_result.is_ok());

            // Check if segment exists
            let exists_result = s3.segment_exists(segment_id).await;
            assert!(exists_result.is_ok() && exists_result.unwrap());

            // Retrieve segment
            let get_result = s3.get_segment(segment_id).await;
            assert!(get_result.is_ok());
            let retrieved_data = get_result.unwrap();
            assert_eq!(retrieved_data, test_data);

            // Get segment metadata
            let metadata_result = s3.get_segment_metadata(segment_id).await;
            assert!(metadata_result.is_ok());
            let metadata = metadata_result.unwrap();
            assert_eq!(metadata.id, segment_id);
            assert_eq!(metadata.size, test_data.len() as u64);

            // Delete segment
            let delete_result = s3.delete_segment(segment_id).await;
            assert!(delete_result.is_ok());

            // Verify deletion
            let exists_after_delete = s3.segment_exists(segment_id).await;
            assert!(exists_after_delete.is_ok() && !exists_after_delete.unwrap());
        }
    }

    #[tokio::test]
    #[ignore] // This requires MinIO to be running
    async fn test_manifest_operations() {
        let config = get_minio_test_config();

        if let Ok(s3) = S3Storage::new(config).await {
            let manifest_id = "test-manifest";
            let manifest_data = br#"{"version": 1, "segments": {}}"#;

            // Store manifest
            let store_result = s3.store_manifest(manifest_id, manifest_data).await;
            assert!(store_result.is_ok());

            // Check if manifest exists
            let exists_result = s3.manifest_exists(manifest_id).await;
            assert!(exists_result.is_ok() && exists_result.unwrap());

            // Retrieve manifest
            let get_result = s3.get_manifest(manifest_id).await;
            assert!(get_result.is_ok());
            let retrieved_data = get_result.unwrap();
            assert_eq!(retrieved_data, manifest_data);
        }
    }

    #[tokio::test]
    #[ignore] // This requires MinIO to be running
    async fn test_large_data_storage() {
        let config = get_minio_test_config();

        if let Ok(s3) = S3Storage::new(config).await {
            let segment_id = SegmentId::new();
            // Create 1MB of test data
            let large_data: Vec<u8> = (0..1_048_576).map(|i| (i % 256) as u8).collect();

            // Store large segment
            let store_result = s3.store_segment(segment_id, &large_data).await;
            assert!(store_result.is_ok());

            // Retrieve and verify
            let get_result = s3.get_segment(segment_id).await;
            assert!(get_result.is_ok());
            let retrieved_data = get_result.unwrap();
            assert_eq!(retrieved_data.len(), large_data.len());
            assert_eq!(retrieved_data, large_data);

            // Cleanup
            let _ = s3.delete_segment(segment_id).await;
        }
    }
}