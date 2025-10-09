//! Segment Archival Tests (Task 6.2)
//!
//! These tests verify the segment archival functionality with compression,
//! read-through, and lifecycle management.

use hyra_scribe_ledger::storage::archival::{ArchivalManager, SegmentMetadata, TieringPolicy};
use hyra_scribe_ledger::storage::s3::S3StorageConfig;
use hyra_scribe_ledger::storage::segment::{Segment, SegmentManager};
use std::collections::HashMap;
use std::sync::Arc;

/// Get test S3 configuration for MinIO
fn get_test_config() -> S3StorageConfig {
    S3StorageConfig {
        bucket: std::env::var("S3_BUCKET").unwrap_or_else(|_| "test-bucket".to_string()),
        region: std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        endpoint: std::env::var("S3_ENDPOINT")
            .ok()
            .or_else(|| Some("http://localhost:9000".to_string())),
        access_key_id: std::env::var("S3_ACCESS_KEY_ID")
            .ok()
            .or_else(|| Some("minioadmin".to_string())),
        secret_access_key: std::env::var("S3_SECRET_ACCESS_KEY")
            .ok()
            .or_else(|| Some("minioadmin".to_string())),
        path_style: true,
        timeout_secs: 30,
        max_retries: 3,
    }
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_archive_and_retrieve_segment() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Create a test segment
    let mut data = HashMap::new();
    data.insert(b"key1".to_vec(), b"value1".to_vec());
    data.insert(b"key2".to_vec(), b"value2".to_vec());
    let segment = Segment::from_data(100, data.clone());

    // Archive the segment
    let metadata = manager.archive_segment(&segment).await.unwrap();

    assert_eq!(metadata.segment_id, 100);
    assert!(metadata.is_compressed);
    assert!(metadata.compressed_size > 0);
    assert!(metadata.compressed_size < metadata.original_size);

    // Retrieve the segment
    let retrieved = manager.retrieve_segment(100).await.unwrap();
    assert!(retrieved.is_some());

    let retrieved_segment = retrieved.unwrap();
    assert_eq!(retrieved_segment.segment_id, segment.segment_id);
    assert_eq!(retrieved_segment.data, segment.data);
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_compression_reduces_size() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let mut policy = TieringPolicy::default();
    policy.enable_compression = true;

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Create a segment with compressible data
    let mut data = HashMap::new();
    for i in 0..100 {
        let key = format!("key_{}", i).into_bytes();
        let value = vec![b'A'; 1000]; // Highly compressible
        data.insert(key, value);
    }
    let segment = Segment::from_data(200, data);

    // Archive with compression
    let metadata = manager.archive_segment(&segment).await.unwrap();

    assert!(metadata.is_compressed);
    assert!(metadata.compressed_size < metadata.original_size);

    // Compression should be significant for repetitive data
    let compression_ratio = metadata.compressed_size as f64 / metadata.original_size as f64;
    assert!(
        compression_ratio < 0.5,
        "Compression ratio should be < 0.5 for repetitive data"
    );
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_no_compression_option() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let mut policy = TieringPolicy::default();
    policy.enable_compression = false;

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Create a test segment
    let mut data = HashMap::new();
    data.insert(b"test_key".to_vec(), b"test_value".to_vec());
    let segment = Segment::from_data(300, data);

    // Archive without compression
    let metadata = manager.archive_segment(&segment).await.unwrap();

    assert!(!metadata.is_compressed);
    assert_eq!(metadata.compressed_size, metadata.original_size);
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_metadata_storage_and_retrieval() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Create and archive a segment
    let mut data = HashMap::new();
    data.insert(b"meta_key".to_vec(), b"meta_value".to_vec());
    let segment = Segment::from_data(400, data);

    manager.archive_segment(&segment).await.unwrap();

    // Retrieve metadata
    let metadata = manager.get_metadata(400).await.unwrap();
    assert!(metadata.is_some());

    let metadata = metadata.unwrap();
    assert_eq!(metadata.segment_id, 400);
    assert_eq!(metadata.entry_count, 1);
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_list_archived_segments() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Archive multiple segments
    for i in 500..505 {
        let mut data = HashMap::new();
        data.insert(format!("key{}", i).into_bytes(), b"value".to_vec());
        let segment = Segment::from_data(i, data);
        manager.archive_segment(&segment).await.unwrap();
    }

    // List archived segments
    let segment_ids = manager.list_archived_segments().await.unwrap();

    assert!(segment_ids.contains(&500));
    assert!(segment_ids.contains(&501));
    assert!(segment_ids.contains(&502));
    assert!(segment_ids.contains(&503));
    assert!(segment_ids.contains(&504));
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_delete_archived_segment() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Archive a segment
    let mut data = HashMap::new();
    data.insert(b"delete_key".to_vec(), b"delete_value".to_vec());
    let segment = Segment::from_data(600, data);
    manager.archive_segment(&segment).await.unwrap();

    // Verify it exists
    let retrieved = manager.retrieve_segment(600).await.unwrap();
    assert!(retrieved.is_some());

    // Delete the segment
    manager.delete_archived_segment(600).await.unwrap();

    // Verify it's deleted
    let retrieved = manager.retrieve_segment(600).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_read_through_cache() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Archive a segment
    let mut data = HashMap::new();
    data.insert(b"cache_key".to_vec(), b"cache_value".to_vec());
    let segment = Segment::from_data(700, data);
    manager.archive_segment(&segment).await.unwrap();

    // First retrieval (should hit S3)
    let retrieved1 = manager.retrieve_segment(700).await.unwrap();
    assert!(retrieved1.is_some());

    // Second retrieval (should hit cache)
    let retrieved2 = manager.retrieve_segment(700).await.unwrap();
    assert!(retrieved2.is_some());

    assert_eq!(retrieved1.unwrap().data, retrieved2.unwrap().data);
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_large_segment_archival() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Create a large segment (5MB)
    let mut data = HashMap::new();
    for i in 0..100 {
        let key = format!("large_key_{}", i).into_bytes();
        let value = vec![0u8; 50 * 1024]; // 50KB per value
        data.insert(key, value);
    }
    let segment = Segment::from_data(800, data);

    // Archive and retrieve
    manager.archive_segment(&segment).await.unwrap();
    let retrieved = manager.retrieve_segment(800).await.unwrap();

    assert!(retrieved.is_some());
    let retrieved_segment = retrieved.unwrap();
    assert_eq!(retrieved_segment.data.len(), segment.data.len());
}

#[test]
fn test_tiering_policy_defaults() {
    let policy = TieringPolicy::default();

    assert!(policy.age_threshold_secs > 0);
    assert!(policy.enable_compression);
    assert!(policy.compression_level <= 9);
    assert!(policy.enable_auto_archival);
    assert!(policy.archival_check_interval_secs > 0);
}

#[test]
fn test_segment_metadata_serialization() {
    let metadata = SegmentMetadata {
        segment_id: 999,
        created_at: 1000,
        archived_at: 2000,
        original_size: 2048,
        compressed_size: 1024,
        is_compressed: true,
        entry_count: 50,
        merkle_root: vec![1, 2, 3, 4],
    };

    let json = serde_json::to_string(&metadata).unwrap();
    let deserialized: SegmentMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.segment_id, metadata.segment_id);
    assert_eq!(deserialized.original_size, metadata.original_size);
    assert_eq!(deserialized.compressed_size, metadata.compressed_size);
    assert_eq!(deserialized.merkle_root, metadata.merkle_root);
    assert_eq!(deserialized.entry_count, metadata.entry_count);
}
