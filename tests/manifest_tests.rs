//! Manifest Integration Tests (Task 4.3)
//!
//! Comprehensive integration tests for the distributed manifest management layer.
//! These tests verify manifest behavior including:
//! - Manifest updates in single node
//! - Manifest consistency and versioning
//! - Concurrent manifest updates
//! - Manifest synchronization across nodes
//! - Recovery scenarios

use hyra_scribe_ledger::consensus::ConsensusNode;
use hyra_scribe_ledger::manifest::{
    compute_diff, merge_manifests, ClusterManifest, ManifestEntry, ManifestManager,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Helper function to create a test node with temporary storage
async fn create_test_node(node_id: u64) -> Arc<ConsensusNode> {
    let db = sled::Config::new().temporary(true).open().unwrap();
    Arc::new(ConsensusNode::new(node_id, db).await.unwrap())
}

/// Test 1: Manifest updates in single node with consensus integration
#[tokio::test]
async fn test_manifest_updates_single_node() {
    let node = create_test_node(1).await;

    // Initialize as single-node cluster
    node.initialize().await.unwrap();

    // Wait for election
    sleep(Duration::from_millis(2000)).await;

    // Should be leader
    assert!(node.is_leader().await);

    // Create manifest manager
    // Note: In production, manifest updates are coordinated through the
    // distributed API layer which uses Raft consensus
    let manager = ManifestManager::new();

    // Test adding segments
    let entry1 = ManifestEntry::new(1, 1234567890, vec![1, 2, 3, 4], 1024);
    let entry2 = ManifestEntry::new(2, 1234567891, vec![5, 6, 7, 8], 2048);

    manager.add_segment(entry1.clone()).await.unwrap();
    assert_eq!(manager.get_version().await, 1);
    assert_eq!(manager.get_segment_count().await, 1);

    manager.add_segment(entry2.clone()).await.unwrap();
    assert_eq!(manager.get_version().await, 2);
    assert_eq!(manager.get_segment_count().await, 2);

    // Verify segments are retrievable
    let retrieved1 = manager.get_segment(1).await;
    assert_eq!(retrieved1, Some(entry1.clone()));

    let retrieved2 = manager.get_segment(2).await;
    assert_eq!(retrieved2, Some(entry2.clone()));

    // Verify total size
    assert_eq!(manager.get_total_size().await, 1024 + 2048);

    // Cleanup
    node.shutdown().await.unwrap();
}

/// Test 2: Manifest versioning and conflict resolution
#[tokio::test]
async fn test_manifest_versioning() {
    let manager = ManifestManager::new();

    // Create entries
    let entry1 = ManifestEntry::new(1, 1000, vec![1, 2], 100);
    let entry2 = ManifestEntry::new(2, 2000, vec![3, 4], 200);
    let entry3 = ManifestEntry::new(3, 3000, vec![5, 6], 300);

    // Add entries sequentially
    manager.add_segment(entry1.clone()).await.unwrap();
    assert_eq!(manager.get_version().await, 1);

    manager.add_segment(entry2.clone()).await.unwrap();
    assert_eq!(manager.get_version().await, 2);

    manager.add_segment(entry3.clone()).await.unwrap();
    assert_eq!(manager.get_version().await, 3);

    // Test version-based cache updates
    let mut old_manifest = ClusterManifest::new();
    old_manifest.version = 1;
    old_manifest.add_entry(ManifestEntry::new(99, 9999, vec![9], 99));

    // Should reject older version
    let result = manager.update_cache(old_manifest).await;
    assert!(result.is_err());
    assert_eq!(manager.get_version().await, 3); // Version unchanged

    // Test with newer version - it replaces the entire manifest
    let mut new_manifest = ClusterManifest::new();
    // Note: add_entry increments version, so we set to 9 to end up at 10 after add
    new_manifest.version = 9;
    new_manifest.add_entry(ManifestEntry::new(100, 10000, vec![10], 1000));
    // Now new_manifest.version is 10

    manager.update_cache(new_manifest.clone()).await.unwrap();
    // After update_cache with higher version, the manifest is completely replaced
    assert_eq!(manager.get_version().await, 10);
    assert_eq!(manager.get_segment_count().await, 1); // Only the new segment from new_manifest
}

/// Test 3: Concurrent manifest updates
#[tokio::test]
async fn test_concurrent_manifest_updates() {
    let manager = Arc::new(ManifestManager::new());

    // Spawn multiple concurrent tasks adding segments
    let mut handles = vec![];

    for i in 0..10 {
        let mgr = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            let entry = ManifestEntry::new(
                i,
                1234567890 + i,
                vec![i as u8, (i + 1) as u8],
                1024 + (i as usize * 100),
            );
            mgr.add_segment(entry).await
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify all segments were added
    assert_eq!(manager.get_segment_count().await, 10);
    assert_eq!(manager.get_version().await, 10);

    // Verify each segment is present
    for i in 0..10 {
        let seg = manager.get_segment(i).await;
        assert!(seg.is_some(), "Segment {} should exist", i);
        assert_eq!(seg.unwrap().segment_id, i);
    }
}

/// Test 4: Concurrent reads during writes
#[tokio::test]
async fn test_concurrent_read_write() {
    let manager = Arc::new(ManifestManager::new());

    // Add initial segments
    for i in 0..5 {
        let entry = ManifestEntry::new(i, 1000 + i, vec![i as u8], 100);
        manager.add_segment(entry).await.unwrap();
    }

    // Spawn writers
    let mut writer_handles = vec![];
    for i in 5..10 {
        let mgr = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            let entry = ManifestEntry::new(i, 1000 + i, vec![i as u8], 100);
            mgr.add_segment(entry).await
        });
        writer_handles.push(handle);
    }

    // Spawn readers
    let mut reader_handles = vec![];
    for _ in 0..20 {
        let mgr = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            let _version = mgr.get_version().await;
            let _count = mgr.get_segment_count().await;
            let _segments = mgr.get_segments().await;
            let _manifest = mgr.get_latest().await;
        });
        reader_handles.push(handle);
    }

    // Wait for all writer tasks
    for handle in writer_handles {
        handle.await.unwrap().ok();
    }

    // Wait for all reader tasks
    for handle in reader_handles {
        handle.await.unwrap();
    }

    // Final verification
    assert_eq!(manager.get_segment_count().await, 10);
    assert_eq!(manager.get_version().await, 10);
}

/// Test 5: Manifest synchronization between nodes
#[tokio::test]
async fn test_manifest_synchronization() {
    let manager1 = ManifestManager::new();
    let manager2 = ManifestManager::new();

    // Add different segments to each manager
    let entry1 = ManifestEntry::new(1, 1000, vec![1], 100);
    let entry2 = ManifestEntry::new(2, 2000, vec![2], 200);
    let entry3 = ManifestEntry::new(3, 3000, vec![3], 300);

    manager1.add_segment(entry1.clone()).await.unwrap();
    manager1.add_segment(entry2.clone()).await.unwrap();

    manager2.add_segment(entry3.clone()).await.unwrap();

    // Synchronize manager2 with manager1's manifest
    let manifest1 = manager1.get_latest().await;
    manager2.sync_with(manifest1).await.unwrap();

    // After sync, since manifest1 has higher version (2) than manager2 (1),
    // manager2 adopts manifest1's entries entirely (segments 1 and 2)
    assert_eq!(manager2.get_segment_count().await, 2);

    // Verify segments from manifest1 are present
    assert!(manager2.get_segment(1).await.is_some());
    assert!(manager2.get_segment(2).await.is_some());
    // Segment 3 (from manager2) is lost because manifest1 had higher version
    assert!(manager2.get_segment(3).await.is_none());
}

/// Test 6: Manifest diff computation
#[tokio::test]
async fn test_manifest_diff_computation() {
    let mut manifest1 = ClusterManifest::new();
    manifest1.add_entry(ManifestEntry::new(1, 1000, vec![1], 100));
    manifest1.add_entry(ManifestEntry::new(2, 2000, vec![2], 200));

    let mut manifest2 = ClusterManifest::new();
    manifest2.add_entry(ManifestEntry::new(2, 2000, vec![2], 200)); // Same
    manifest2.add_entry(ManifestEntry::new(3, 3000, vec![3], 300)); // New

    let diff = compute_diff(&manifest1, &manifest2);

    // Should have 1 removed (segment 1) and 1 added (segment 3)
    assert_eq!(diff.removed.len(), 1);
    assert_eq!(diff.removed[0], 1);
    assert_eq!(diff.added.len(), 1);
    assert_eq!(diff.added[0].segment_id, 3);
    assert_eq!(diff.modified.len(), 0);
}

/// Test 7: Manifest merging with conflicts
#[tokio::test]
async fn test_manifest_merging() {
    let mut manifest1 = ClusterManifest::new();
    manifest1.version = 5;
    manifest1.add_entry(ManifestEntry::new(1, 1000, vec![1], 100));
    manifest1.add_entry(ManifestEntry::new(2, 2000, vec![2], 200));

    let mut manifest2 = ClusterManifest::new();
    manifest2.version = 3;
    manifest2.add_entry(ManifestEntry::new(2, 2500, vec![2, 2], 250)); // Conflict
    manifest2.add_entry(ManifestEntry::new(3, 3000, vec![3], 300));

    let merged = merge_manifests(&manifest1, &manifest2);

    // Merged should prefer manifest1's version for segment 2
    assert_eq!(merged.entries.len(), 3);
    assert!(merged.version >= manifest1.version);

    // Verify segment 2 uses manifest1's data (higher version)
    let seg2 = merged
        .entries
        .iter()
        .find(|e| e.segment_id == 2)
        .expect("Segment 2 should exist");
    assert_eq!(seg2.size, 200); // From manifest1
}

/// Test 8: Manifest consistency after updates
#[tokio::test]
async fn test_manifest_consistency() {
    let manager = ManifestManager::new();

    // Add segments
    for i in 1..=5 {
        let entry = ManifestEntry::new(i, 1000 * i, vec![i as u8], 100 * i as usize);
        manager.add_segment(entry).await.unwrap();
    }

    // Remove some segments
    manager.remove_segment(2).await.unwrap();
    manager.remove_segment(4).await.unwrap();

    // Verify consistency
    assert_eq!(manager.get_segment_count().await, 3);
    assert_eq!(manager.get_version().await, 7); // 5 adds + 2 removes

    // Verify correct segments remain
    assert!(manager.get_segment(1).await.is_some());
    assert!(manager.get_segment(2).await.is_none());
    assert!(manager.get_segment(3).await.is_some());
    assert!(manager.get_segment(4).await.is_none());
    assert!(manager.get_segment(5).await.is_some());

    // Verify total size
    let expected_size = 100 + 300 + 500; // Segments 1, 3, 5
    assert_eq!(manager.get_total_size().await, expected_size);
}

/// Test 9: Manifest recovery simulation
#[tokio::test]
async fn test_manifest_recovery() {
    // Simulate a node with existing manifest
    let manager1 = ManifestManager::new();

    for i in 1..=10 {
        let entry = ManifestEntry::new(i, 1000 * i, vec![i as u8], 100 * i as usize);
        manager1.add_segment(entry).await.unwrap();
    }

    let version_before = manager1.get_version().await;
    let manifest_snapshot = manager1.get_latest().await;

    // Simulate node failure and recovery with new manager
    let manager2 = ManifestManager::new();

    // Restore from snapshot
    manager2
        .update_cache(manifest_snapshot.clone())
        .await
        .unwrap();

    // Verify recovered state matches
    assert_eq!(manager2.get_version().await, version_before);
    assert_eq!(manager2.get_segment_count().await, 10);
    assert_eq!(
        manager2.get_total_size().await,
        manager1.get_total_size().await
    );

    // Verify all segments are present
    for i in 1..=10 {
        let seg = manager2.get_segment(i).await;
        assert!(seg.is_some(), "Segment {} should exist after recovery", i);
    }
}

/// Test 10: Large manifest with many segments
#[tokio::test]
async fn test_large_manifest() {
    let manager = ManifestManager::new();

    // Add many segments
    let segment_count = 1000;
    for i in 0..segment_count {
        let entry = ManifestEntry::new(i, 1000 + i, vec![(i % 256) as u8], 1024);
        manager.add_segment(entry).await.unwrap();
    }

    assert_eq!(manager.get_segment_count().await, segment_count as usize);
    assert_eq!(manager.get_version().await, segment_count);

    // Query random segments
    for i in (0..segment_count).step_by(100) {
        let seg = manager.get_segment(i).await;
        assert!(seg.is_some(), "Segment {} should exist", i);
    }

    // Test sorted retrieval performance
    let segments = manager.get_segments().await;
    assert_eq!(segments.len(), segment_count as usize);

    // Verify they're sorted
    for i in 1..segments.len() {
        assert!(
            segments[i - 1].segment_id < segments[i].segment_id,
            "Segments should be sorted"
        );
    }
}

/// Test 11: Manifest serialization and deserialization
#[tokio::test]
async fn test_manifest_serialization() {
    let manager = ManifestManager::new();

    // Add segments
    for i in 1..=5 {
        let entry = ManifestEntry::new(i, 1000 * i, vec![i as u8, i as u8 + 1], 100 * i as usize);
        manager.add_segment(entry).await.unwrap();
    }

    let manifest = manager.get_latest().await;

    // Serialize
    let serialized = manifest.serialize().unwrap();

    // Deserialize
    let deserialized = ClusterManifest::deserialize(&serialized).unwrap();

    // Verify equality
    assert_eq!(deserialized.version, manifest.version);
    assert_eq!(deserialized.entries.len(), manifest.entries.len());
    assert_eq!(deserialized.created_at, manifest.created_at);

    for (orig, deser) in manifest.entries.iter().zip(deserialized.entries.iter()) {
        assert_eq!(orig.segment_id, deser.segment_id);
        assert_eq!(orig.timestamp, deser.timestamp);
        assert_eq!(orig.merkle_root, deser.merkle_root);
        assert_eq!(orig.size, deser.size);
    }
}

/// Test 12: Manifest update race condition handling
#[tokio::test]
async fn test_manifest_update_race_conditions() {
    let manager = Arc::new(ManifestManager::new());

    // Simulate rapid concurrent updates to the same segment
    let mut handles = vec![];

    for i in 0..100 {
        let mgr = Arc::clone(&manager);
        let handle = tokio::spawn(async move {
            // All tasks try to add or update segment 1
            let entry = ManifestEntry::new(1, 1000 + i, vec![i as u8], 100 + i as usize);
            mgr.add_segment(entry).await
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap().ok(); // Some will fail, that's expected
    }

    // Verify manifest is still consistent
    let count = manager.get_segment_count().await;
    assert!(count >= 1, "At least one segment should exist");

    // Verify we can still read the manifest
    let manifest = manager.get_latest().await;
    assert!(manifest.version > 0);
}
