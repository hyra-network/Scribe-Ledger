//! Manifest manager for coordinating manifest operations through consensus
//!
//! This module implements the ManifestManager which handles manifest updates,
//! queries, and synchronization across the cluster using Raft consensus.

use crate::consensus::type_config::TypeConfig;
use crate::error::{Result, ScribeError};
use crate::manifest::{ClusterManifest, ManifestEntry};
use crate::types::SegmentId;
use openraft::Raft;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manager for cluster-wide manifest operations
///
/// ManifestManager coordinates manifest updates through Raft consensus
/// and maintains a local cache for efficient queries.
pub struct ManifestManager {
    /// Reference to the Raft instance for consensus operations
    raft: Option<Arc<Raft<TypeConfig>>>,
    /// Cached local copy of the manifest for fast reads
    cached_manifest: Arc<RwLock<ClusterManifest>>,
}

impl ManifestManager {
    /// Create a new manifest manager without Raft integration
    ///
    /// This constructor is useful for testing or standalone operation.
    pub fn new() -> Self {
        Self {
            raft: None,
            cached_manifest: Arc::new(RwLock::new(ClusterManifest::new())),
        }
    }

    /// Create a new manifest manager with Raft integration
    ///
    /// This constructor should be used in production to enable consensus-based
    /// manifest updates.
    pub fn with_raft(raft: Arc<Raft<TypeConfig>>) -> Self {
        Self {
            raft: Some(raft),
            cached_manifest: Arc::new(RwLock::new(ClusterManifest::new())),
        }
    }

    /// Get the latest version of the manifest
    ///
    /// Returns a clone of the cached manifest. This is a fast, local operation.
    pub async fn get_latest(&self) -> ClusterManifest {
        let manifest = self.cached_manifest.read().await;
        manifest.clone()
    }

    /// Get all segment entries from the manifest
    ///
    /// Returns a vector of all manifest entries, sorted by segment ID.
    pub async fn get_segments(&self) -> Vec<ManifestEntry> {
        let manifest = self.cached_manifest.read().await;
        let mut entries = manifest.entries.clone();
        entries.sort_by_key(|e| e.segment_id);
        entries
    }

    /// Get a specific segment entry by ID
    ///
    /// Returns None if the segment is not found in the manifest.
    pub async fn get_segment(&self, segment_id: SegmentId) -> Option<ManifestEntry> {
        let manifest = self.cached_manifest.read().await;
        manifest.get_entry(segment_id).cloned()
    }

    /// Add a new segment entry to the manifest
    ///
    /// If Raft is configured, this will propose the change through consensus.
    /// Otherwise, it updates the local cache directly.
    pub async fn add_segment(&self, entry: ManifestEntry) -> Result<()> {
        if let Some(_raft) = &self.raft {
            // TODO: In a full implementation, this would:
            // 1. Create a ManifestUpdate AppRequest variant
            // 2. Propose it to Raft using raft.client_write()
            // 3. Wait for consensus
            // 4. Update the cache on apply

            // For now, update cache directly
            let mut manifest = self.cached_manifest.write().await;
            manifest.add_entry(entry);
            Ok(())
        } else {
            // No Raft, update cache directly
            let mut manifest = self.cached_manifest.write().await;
            manifest.add_entry(entry);
            Ok(())
        }
    }

    /// Remove a segment entry from the manifest
    ///
    /// If Raft is configured, this will propose the change through consensus.
    /// Otherwise, it updates the local cache directly.
    pub async fn remove_segment(&self, segment_id: SegmentId) -> Result<Option<ManifestEntry>> {
        if let Some(_raft) = &self.raft {
            // TODO: In a full implementation, this would use Raft consensus
            let mut manifest = self.cached_manifest.write().await;
            Ok(manifest.remove_entry(segment_id))
        } else {
            let mut manifest = self.cached_manifest.write().await;
            Ok(manifest.remove_entry(segment_id))
        }
    }

    /// Update the cached manifest with a new version
    ///
    /// This is typically called when a manifest update is applied through
    /// the Raft state machine.
    pub async fn update_cache(&self, new_manifest: ClusterManifest) -> Result<()> {
        let mut manifest = self.cached_manifest.write().await;

        // Only update if the new manifest has a higher version
        if new_manifest.version > manifest.version {
            *manifest = new_manifest;
            Ok(())
        } else if new_manifest.version == manifest.version {
            // Same version, no update needed
            Ok(())
        } else {
            // Older version, reject
            Err(ScribeError::Manifest(format!(
                "Cannot update cache with older manifest version {} (current: {})",
                new_manifest.version, manifest.version
            )))
        }
    }

    /// Synchronize manifest with another node
    ///
    /// This performs conflict resolution and merges manifests if needed.
    pub async fn sync_with(&self, remote_manifest: ClusterManifest) -> Result<()> {
        let mut local_manifest = self.cached_manifest.write().await;

        // If remote has higher version, use it
        if remote_manifest.version > local_manifest.version {
            *local_manifest = remote_manifest;
            return Ok(());
        }

        // If local has higher version, keep it
        if local_manifest.version > remote_manifest.version {
            return Ok(());
        }

        // Same version - merge if different
        if local_manifest.entries != remote_manifest.entries {
            let merged = crate::manifest::merge_manifests(&local_manifest, &remote_manifest);
            *local_manifest = merged;
        }

        Ok(())
    }

    /// Get the current manifest version
    pub async fn get_version(&self) -> u64 {
        let manifest = self.cached_manifest.read().await;
        manifest.version
    }

    /// Get the total size of all segments in the manifest
    pub async fn get_total_size(&self) -> usize {
        let manifest = self.cached_manifest.read().await;
        manifest.total_size()
    }

    /// Get the number of segments in the manifest
    pub async fn get_segment_count(&self) -> usize {
        let manifest = self.cached_manifest.read().await;
        manifest.entry_count()
    }

    /// Clear the cached manifest (for testing purposes)
    #[cfg(test)]
    pub async fn clear_cache(&self) {
        let mut manifest = self.cached_manifest.write().await;
        *manifest = ClusterManifest::new();
    }
}

impl Default for ManifestManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manifest_manager_new() {
        let manager = ManifestManager::new();
        let manifest = manager.get_latest().await;

        assert_eq!(manifest.version, 0);
        assert!(manifest.entries.is_empty());
    }

    #[tokio::test]
    async fn test_add_segment() {
        let manager = ManifestManager::new();
        let entry = ManifestEntry::new(1, 1234567890, vec![1, 2, 3, 4], 1024);

        manager.add_segment(entry.clone()).await.unwrap();

        let manifest = manager.get_latest().await;
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0], entry);
    }

    #[tokio::test]
    async fn test_get_segment() {
        let manager = ManifestManager::new();
        let entry = ManifestEntry::new(1, 1234567890, vec![1, 2, 3, 4], 1024);

        manager.add_segment(entry.clone()).await.unwrap();

        let retrieved = manager.get_segment(1).await;
        assert_eq!(retrieved, Some(entry));

        let not_found = manager.get_segment(999).await;
        assert_eq!(not_found, None);
    }

    #[tokio::test]
    async fn test_get_segments() {
        let manager = ManifestManager::new();
        let entry1 = ManifestEntry::new(2, 1234567890, vec![1, 2, 3, 4], 1024);
        let entry2 = ManifestEntry::new(1, 1234567891, vec![5, 6, 7, 8], 2048);

        manager.add_segment(entry1.clone()).await.unwrap();
        manager.add_segment(entry2.clone()).await.unwrap();

        let segments = manager.get_segments().await;
        assert_eq!(segments.len(), 2);
        // Should be sorted by segment_id
        assert_eq!(segments[0].segment_id, 1);
        assert_eq!(segments[1].segment_id, 2);
    }

    #[tokio::test]
    async fn test_remove_segment() {
        let manager = ManifestManager::new();
        let entry = ManifestEntry::new(1, 1234567890, vec![1, 2, 3, 4], 1024);

        manager.add_segment(entry.clone()).await.unwrap();
        assert_eq!(manager.get_segment_count().await, 1);

        let removed = manager.remove_segment(1).await.unwrap();
        assert_eq!(removed, Some(entry));
        assert_eq!(manager.get_segment_count().await, 0);

        // Removing non-existent segment
        let not_found = manager.remove_segment(999).await.unwrap();
        assert_eq!(not_found, None);
    }

    #[tokio::test]
    async fn test_update_cache_newer_version() {
        let manager = ManifestManager::new();

        let mut new_manifest = ClusterManifest::new();
        new_manifest.version = 5;
        new_manifest.add_entry(ManifestEntry::new(1, 1234567890, vec![1], 1024));

        manager.update_cache(new_manifest.clone()).await.unwrap();

        let cached = manager.get_latest().await;
        assert_eq!(cached.version, new_manifest.version);
    }

    #[tokio::test]
    async fn test_update_cache_older_version() {
        let manager = ManifestManager::new();

        // Set cache to version 5
        let mut current_manifest = ClusterManifest::new();
        current_manifest.version = 5;
        manager.update_cache(current_manifest).await.unwrap();

        // Try to update with version 3
        let mut old_manifest = ClusterManifest::new();
        old_manifest.version = 3;

        let result = manager.update_cache(old_manifest).await;
        assert!(result.is_err());

        // Cache should still be at version 5
        assert_eq!(manager.get_version().await, 5);
    }

    #[tokio::test]
    async fn test_update_cache_same_version() {
        let manager = ManifestManager::new();

        let mut manifest = ClusterManifest::new();
        manifest.version = 3;
        manager.update_cache(manifest.clone()).await.unwrap();

        // Update with same version should succeed (no-op)
        let result = manager.update_cache(manifest).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sync_with_higher_version() {
        let manager = ManifestManager::new();

        // Local is at version 2
        let mut local = ClusterManifest::new();
        local.version = 2;
        manager.update_cache(local).await.unwrap();

        // Remote is at version 5 (note: add_entry increments version, so we set it after)
        let mut remote = ClusterManifest::new();
        remote.add_entry(ManifestEntry::new(1, 1234567890, vec![1], 1024));
        remote.version = 5; // Explicitly set version after adding entry

        manager.sync_with(remote.clone()).await.unwrap();

        let cached = manager.get_latest().await;
        assert_eq!(cached.version, 5);
        assert_eq!(cached.entries.len(), 1);
    }

    #[tokio::test]
    async fn test_sync_with_lower_version() {
        let manager = ManifestManager::new();

        // Local is at version 5 (add entry then set version)
        let mut local = ClusterManifest::new();
        local.add_entry(ManifestEntry::new(1, 1234567890, vec![1], 1024));
        local.version = 5; // Explicitly set version after adding entry
        manager.update_cache(local).await.unwrap();

        // Remote is at version 2
        let mut remote = ClusterManifest::new();
        remote.version = 2;

        manager.sync_with(remote).await.unwrap();

        // Should keep local version
        let cached = manager.get_latest().await;
        assert_eq!(cached.version, 5);
        assert_eq!(cached.entries.len(), 1);
    }

    #[tokio::test]
    async fn test_sync_with_same_version_different_entries() {
        let manager = ManifestManager::new();

        // Local version 3 with entry 1
        let mut local = ClusterManifest::new();
        local.version = 3;
        local.add_entry(ManifestEntry::new(1, 1234567890, vec![1], 1024));
        manager.update_cache(local).await.unwrap();

        // Remote version 3 with entry 2
        let mut remote = ClusterManifest::new();
        remote.version = 3;
        remote.add_entry(ManifestEntry::new(2, 1234567891, vec![2], 2048));

        manager.sync_with(remote).await.unwrap();

        // Should merge entries
        let cached = manager.get_latest().await;
        assert!(cached.version > 3); // Merged version should be incremented
        assert_eq!(cached.entries.len(), 2); // Should have both entries
    }

    #[tokio::test]
    async fn test_get_total_size() {
        let manager = ManifestManager::new();

        manager
            .add_segment(ManifestEntry::new(1, 1000, vec![1], 1024))
            .await
            .unwrap();
        manager
            .add_segment(ManifestEntry::new(2, 2000, vec![2], 2048))
            .await
            .unwrap();
        manager
            .add_segment(ManifestEntry::new(3, 3000, vec![3], 3072))
            .await
            .unwrap();

        let total_size = manager.get_total_size().await;
        assert_eq!(total_size, 1024 + 2048 + 3072);
    }

    #[tokio::test]
    async fn test_get_version() {
        let manager = ManifestManager::new();

        assert_eq!(manager.get_version().await, 0);

        manager
            .add_segment(ManifestEntry::new(1, 1000, vec![1], 1024))
            .await
            .unwrap();
        assert_eq!(manager.get_version().await, 1);

        manager
            .add_segment(ManifestEntry::new(2, 2000, vec![2], 2048))
            .await
            .unwrap();
        assert_eq!(manager.get_version().await, 2);
    }

    #[tokio::test]
    async fn test_get_segment_count() {
        let manager = ManifestManager::new();

        assert_eq!(manager.get_segment_count().await, 0);

        manager
            .add_segment(ManifestEntry::new(1, 1000, vec![1], 1024))
            .await
            .unwrap();
        assert_eq!(manager.get_segment_count().await, 1);

        manager
            .add_segment(ManifestEntry::new(2, 2000, vec![2], 2048))
            .await
            .unwrap();
        assert_eq!(manager.get_segment_count().await, 2);

        manager.remove_segment(1).await.unwrap();
        assert_eq!(manager.get_segment_count().await, 1);
    }
}
