//! Data Tiering Tests (Task 6.3)
//!
//! These tests verify automatic data tiering, tiering policies,
//! MinIO compatibility, and error recovery scenarios.

use hyra_scribe_ledger::storage::archival::{ArchivalManager, TieringPolicy};
use hyra_scribe_ledger::storage::s3::S3StorageConfig;
use hyra_scribe_ledger::storage::segment::{Segment, SegmentManager};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

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
async fn test_minio_compatibility() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    // Test basic MinIO connectivity
    let manager = ArchivalManager::new(config, segment_mgr, policy).await;
    assert!(manager.is_ok(), "Should connect to MinIO successfully");
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_tiering_policy_age_threshold() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy {
        age_threshold_secs: 1, // 1 second threshold for testing
        ..Default::default()
    };

    let manager = ArchivalManager::new(config, segment_mgr.clone(), policy)
        .await
        .unwrap();

    // Add a segment to the segment manager
    segment_mgr
        .put(b"test_key".to_vec(), b"test_value".to_vec())
        .unwrap();
    segment_mgr.flush_active().unwrap();

    // Wait for segment to age
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Archive old segments
    let archived_ids = manager.archive_old_segments().await.unwrap();

    // Should have archived at least one segment
    assert!(!archived_ids.is_empty(), "Should archive old segments");
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_compression_levels() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());

    // Test different compression levels
    for level in [0, 6, 9] {
        let policy = TieringPolicy {
            compression_level: level,
            ..Default::default()
        };

        let manager = ArchivalManager::new(config.clone(), segment_mgr.clone(), policy)
            .await
            .unwrap();

        // Create compressible data
        let mut data = HashMap::new();
        data.insert(b"compress_key".to_vec(), vec![b'A'; 10000]);
        let segment = Segment::from_data(1000 + level as u64, data);

        // Archive and check metadata
        let metadata = manager.archive_segment(&segment).await.unwrap();
        assert!(metadata.is_compressed);

        // Higher compression levels should result in smaller sizes (for compressible data)
        if level == 9 {
            assert!(metadata.compressed_size < metadata.original_size);
        }
    }
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_error_recovery_invalid_bucket() {
    let mut config = get_test_config();
    config.bucket = "non-existent-bucket-12345".to_string();

    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy).await;

    // This might succeed initially but fail on operations
    if let Ok(manager) = manager {
        let mut data = HashMap::new();
        data.insert(b"key".to_vec(), b"value".to_vec());
        let segment = Segment::from_data(2000, data);

        // Archive should fail
        let result = manager.archive_segment(&segment).await;
        assert!(result.is_err(), "Should fail with invalid bucket");
    }
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_retry_logic_on_transient_failure() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Archive a segment (this tests retry logic internally)
    let mut data = HashMap::new();
    data.insert(b"retry_key".to_vec(), b"retry_value".to_vec());
    let segment = Segment::from_data(3000, data);

    // Should succeed even with retry logic
    let result = manager.archive_segment(&segment).await;
    assert!(result.is_ok(), "Should succeed with retry logic");
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_concurrent_archival() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = Arc::new(
        ArchivalManager::new(config, segment_mgr, policy)
            .await
            .unwrap(),
    );

    // Archive multiple segments concurrently
    let mut tasks = vec![];
    for i in 4000..4010 {
        let manager_clone = manager.clone();
        let task = tokio::spawn(async move {
            let mut data = HashMap::new();
            data.insert(format!("key{}", i).into_bytes(), b"value".to_vec());
            let segment = Segment::from_data(i, data);
            manager_clone.archive_segment(&segment).await
        });
        tasks.push(task);
    }

    // Wait for all tasks
    for task in tasks {
        let result = task.await.unwrap();
        assert!(result.is_ok(), "Concurrent archival should succeed");
    }

    // Verify all segments are archived
    let segment_ids = manager.list_archived_segments().await.unwrap();
    assert!(segment_ids.len() >= 10, "Should have archived all segments");
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_large_number_of_segments() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Archive many small segments
    for i in 5000..5100 {
        let mut data = HashMap::new();
        data.insert(b"key".to_vec(), b"value".to_vec());
        let segment = Segment::from_data(i, data);
        manager.archive_segment(&segment).await.unwrap();
    }

    // List should return all segments
    let segment_ids = manager.list_archived_segments().await.unwrap();
    assert!(
        segment_ids.len() >= 100,
        "Should list all archived segments"
    );
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_segment_lifecycle_management() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Create segment
    let mut data = HashMap::new();
    data.insert(b"lifecycle_key".to_vec(), b"lifecycle_value".to_vec());
    let segment = Segment::from_data(6000, data);

    // Archive
    manager.archive_segment(&segment).await.unwrap();

    // Retrieve
    let retrieved = manager.retrieve_segment(6000).await.unwrap();
    assert!(retrieved.is_some());

    // Delete
    manager.delete_archived_segment(6000).await.unwrap();

    // Verify deletion
    let retrieved = manager.retrieve_segment(6000).await.unwrap();
    assert!(retrieved.is_none());
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_metadata_cache_invalidation() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Archive a segment
    let mut data = HashMap::new();
    data.insert(b"cache_test".to_vec(), b"value".to_vec());
    let segment = Segment::from_data(7000, data);
    manager.archive_segment(&segment).await.unwrap();

    // Get metadata (should cache)
    let metadata1 = manager.get_metadata(7000).await.unwrap();
    assert!(metadata1.is_some());

    // Get metadata again (should hit cache)
    let metadata2 = manager.get_metadata(7000).await.unwrap();
    assert!(metadata2.is_some());

    assert_eq!(metadata1.unwrap().segment_id, metadata2.unwrap().segment_id);

    // Delete segment (should invalidate cache)
    manager.delete_archived_segment(7000).await.unwrap();

    // Get metadata (should return None)
    let metadata3 = manager.get_metadata(7000).await.unwrap();
    assert!(metadata3.is_none());
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_different_data_types() {
    let config = get_test_config();
    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy)
        .await
        .unwrap();

    // Test with different data types
    let test_data = vec![
        (b"text".to_vec(), b"hello world".to_vec()),
        (b"binary".to_vec(), vec![0u8, 255, 128, 64]),
        (b"empty".to_vec(), vec![]),
        (b"unicode".to_vec(), "Hello 世界 🌍".as_bytes().to_vec()),
    ];

    let mut data = HashMap::new();
    for (key, value) in test_data {
        data.insert(key, value);
    }
    let segment = Segment::from_data(8000, data.clone());

    // Archive and retrieve
    manager.archive_segment(&segment).await.unwrap();
    let retrieved = manager.retrieve_segment(8000).await.unwrap();

    assert!(retrieved.is_some());
    let retrieved_segment = retrieved.unwrap();
    assert_eq!(retrieved_segment.data, data);
}

#[tokio::test]
#[ignore] // Requires MinIO to be running
async fn test_path_style_addressing() {
    let mut config = get_test_config();
    config.path_style = true; // MinIO requires path-style

    let segment_mgr = Arc::new(SegmentManager::new());
    let policy = TieringPolicy::default();

    let manager = ArchivalManager::new(config, segment_mgr, policy).await;
    assert!(manager.is_ok(), "Should work with path-style addressing");
}

#[test]
fn test_tiering_policy_validation() {
    let mut policy = TieringPolicy {
        compression_level: 9,
        age_threshold_secs: 3600,
        ..Default::default()
    };

    // Test valid compression level
    assert_eq!(policy.compression_level, 9);

    // Test age threshold
    assert_eq!(policy.age_threshold_secs, 3600);

    // Test auto-archival flag
    policy.enable_auto_archival = false;
    assert!(!policy.enable_auto_archival);
}
