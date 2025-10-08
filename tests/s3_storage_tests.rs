//! S3 Storage Integration Tests
//!
//! These tests verify the S3 storage backend functionality.
//! Note: These tests require a running MinIO instance or AWS S3 credentials.
//! They are marked with #[ignore] by default and should be run explicitly.

use simple_scribe_ledger::storage::s3::{S3Storage, S3StorageConfig};
use simple_scribe_ledger::storage::segment::Segment;
use std::collections::HashMap;

/// Get test S3 configuration for MinIO
fn get_test_config() -> S3StorageConfig {
    S3StorageConfig {
        bucket: "test-bucket".to_string(),
        region: "us-east-1".to_string(),
        endpoint: Some("http://localhost:9000".to_string()),
        access_key_id: Some("minioadmin".to_string()),
        secret_access_key: Some("minioadmin".to_string()),
        path_style: true,
        timeout_secs: 30,
        max_retries: 3,
    }
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_s3_put_get_segment() {
    let config = get_test_config();
    let storage = S3Storage::new(config).await.unwrap();

    // Create a test segment
    let mut data = HashMap::new();
    data.insert(b"key1".to_vec(), b"value1".to_vec());
    data.insert(b"key2".to_vec(), b"value2".to_vec());
    let segment = Segment::from_data(42, data);

    // Put segment to S3
    storage.put_segment(&segment).await.unwrap();

    // Get segment from S3
    let retrieved = storage.get_segment(42).await.unwrap();
    assert!(retrieved.is_some());

    let retrieved_segment = retrieved.unwrap();
    assert_eq!(retrieved_segment.segment_id, segment.segment_id);
    assert_eq!(retrieved_segment.data.len(), segment.data.len());
    assert_eq!(retrieved_segment.data, segment.data);
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_s3_get_nonexistent_segment() {
    let config = get_test_config();
    let storage = S3Storage::new(config).await.unwrap();

    // Try to get a non-existent segment
    let result = storage.get_segment(999999).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_s3_delete_segment() {
    let config = get_test_config();
    let storage = S3Storage::new(config).await.unwrap();

    // Create and put a test segment
    let mut data = HashMap::new();
    data.insert(b"test_key".to_vec(), b"test_value".to_vec());
    let segment = Segment::from_data(123, data);

    storage.put_segment(&segment).await.unwrap();

    // Verify it exists
    let retrieved = storage.get_segment(123).await.unwrap();
    assert!(retrieved.is_some());

    // Delete the segment
    storage.delete_segment(123).await.unwrap();

    // Verify it's deleted
    let result = storage.get_segment(123).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_s3_list_segments() {
    let config = get_test_config();
    let storage = S3Storage::new(config).await.unwrap();

    // Put multiple segments
    for i in 0..5 {
        let mut data = HashMap::new();
        data.insert(
            format!("key{}", i).into_bytes(),
            format!("value{}", i).into_bytes(),
        );
        let segment = Segment::from_data(1000 + i, data);
        storage.put_segment(&segment).await.unwrap();
    }

    // List all segments
    let segment_ids = storage.list_segments().await.unwrap();

    // Verify all segments are listed
    assert!(segment_ids.contains(&1000));
    assert!(segment_ids.contains(&1001));
    assert!(segment_ids.contains(&1002));
    assert!(segment_ids.contains(&1003));
    assert!(segment_ids.contains(&1004));

    // Cleanup
    for i in 0..5 {
        let _ = storage.delete_segment(1000 + i).await;
    }
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_s3_health_check() {
    let config = get_test_config();
    let storage = S3Storage::new(config).await.unwrap();

    // Health check should succeed if MinIO is running
    let result = storage.health_check().await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_s3_large_segment() {
    let config = get_test_config();
    let storage = S3Storage::new(config).await.unwrap();

    // Create a large segment (5MB of data)
    let mut data = HashMap::new();
    for i in 0..100 {
        let key = format!("large_key_{}", i).into_bytes();
        let value = vec![0u8; 50 * 1024]; // 50KB per value
        data.insert(key, value);
    }
    let segment = Segment::from_data(5000, data);

    // Put and get the large segment
    storage.put_segment(&segment).await.unwrap();
    let retrieved = storage.get_segment(5000).await.unwrap();

    assert!(retrieved.is_some());
    let retrieved_segment = retrieved.unwrap();
    assert_eq!(retrieved_segment.segment_id, segment.segment_id);
    assert_eq!(retrieved_segment.data.len(), segment.data.len());

    // Cleanup
    let _ = storage.delete_segment(5000).await;
}

#[tokio::test]
async fn test_s3_config_validation() {
    // Test with empty bucket name
    let config = S3StorageConfig {
        bucket: String::new(),
        ..get_test_config()
    };

    let result = S3Storage::new(config).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("bucket"));
}
