use aws_sdk_s3::Client;
use aws_config::Region;
use bytes::Bytes;
use std::future::Future;
use std::pin::Pin;
use crate::error::{Result, ScribeError};
use crate::types::{SegmentId, SegmentMetadata};
use crate::storage::StorageBackend;

pub struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    /// Create a new S3 storage backend
    pub async fn new(bucket: String, region: Option<String>) -> Result<Self> {
        let config = if let Some(region_str) = region {
            aws_config::defaults(aws_config::BehaviorVersion::latest())
                .region(Region::new(region_str))
                .load()
                .await
        } else {
            aws_config::defaults(aws_config::BehaviorVersion::latest())
                .load()
                .await
        };
        
        let client = Client::new(&config);
        
        Ok(Self { client, bucket })
    }
    
    /// Generate S3 key for a segment
    fn segment_key(&self, segment_id: &SegmentId) -> String {
        format!("segments/{}", segment_id.0)
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
            
            tracing::info!("Uploaded segment {} to S3", segment_id.0);
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
        Box::pin(async move {
            // TODO: Implement segment listing and metadata parsing
            Ok(vec![])
        })
    }
}