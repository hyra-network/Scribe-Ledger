//! Manifest module for managing metadata and data structure
//!
//! This module implements distributed metadata management using consensus.
//! It tracks segment metadata, cluster state, and provides synchronization
//! mechanisms for distributed operations.

mod manager;

pub use manager::ManifestManager;

use crate::error::{Result, ScribeError};
use crate::types::{NodeId, SegmentId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Represents the state of a node in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Node is active and healthy
    Active,
    /// Node is suspected to be down
    Suspected,
    /// Node is confirmed down
    Down,
    /// Node is joining the cluster
    Joining,
    /// Node is leaving the cluster
    Leaving,
}

/// Information about a cluster node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNode {
    /// Unique identifier for the node
    pub id: NodeId,
    /// Network address of the node
    pub address: String,
    /// Current state of the node
    pub state: NodeState,
    /// Last heartbeat timestamp (Unix timestamp in milliseconds)
    pub last_heartbeat: u64,
}

impl ClusterNode {
    /// Create a new cluster node
    pub fn new(id: NodeId, address: String) -> Self {
        Self {
            id,
            address,
            state: NodeState::Joining,
            last_heartbeat: current_timestamp_ms(),
        }
    }

    /// Update the node's heartbeat timestamp
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = current_timestamp_ms();
    }

    /// Check if the node's heartbeat is stale (older than timeout_ms)
    pub fn is_heartbeat_stale(&self, timeout_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now.saturating_sub(self.last_heartbeat) > timeout_ms
    }

    /// Mark the node as active
    pub fn mark_active(&mut self) {
        self.state = NodeState::Active;
        self.update_heartbeat();
    }

    /// Mark the node as suspected
    pub fn mark_suspected(&mut self) {
        self.state = NodeState::Suspected;
    }

    /// Mark the node as down
    pub fn mark_down(&mut self) {
        self.state = NodeState::Down;
    }
}

/// Entry in the manifest tracking a data segment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestEntry {
    /// Unique identifier for the segment
    pub segment_id: SegmentId,
    /// Unix timestamp when the segment was created (in seconds)
    pub timestamp: u64,
    /// Merkle root hash of the segment data (for verification)
    pub merkle_root: Vec<u8>,
    /// Size of the segment in bytes
    pub size: usize,
}

impl ManifestEntry {
    /// Create a new manifest entry
    pub fn new(segment_id: SegmentId, timestamp: u64, merkle_root: Vec<u8>, size: usize) -> Self {
        Self {
            segment_id,
            timestamp,
            merkle_root,
            size,
        }
    }

    /// Create a manifest entry with current timestamp
    pub fn with_current_timestamp(
        segment_id: SegmentId,
        merkle_root: Vec<u8>,
        size: usize,
    ) -> Self {
        Self {
            segment_id,
            timestamp: current_timestamp_secs(),
            merkle_root,
            size,
        }
    }
}

/// Cluster-wide manifest tracking all segments and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterManifest {
    /// Version number of the manifest (incremented on each update)
    pub version: u64,
    /// List of all segment entries
    pub entries: Vec<ManifestEntry>,
    /// Timestamp when this manifest version was created
    pub created_at: u64,
}

impl ClusterManifest {
    /// Create a new empty manifest
    pub fn new() -> Self {
        Self {
            version: 0,
            entries: Vec::new(),
            created_at: current_timestamp_secs(),
        }
    }

    /// Create a new manifest with initial entries
    pub fn with_entries(entries: Vec<ManifestEntry>) -> Self {
        Self {
            version: 0,
            entries,
            created_at: current_timestamp_secs(),
        }
    }

    /// Add a new entry to the manifest
    pub fn add_entry(&mut self, entry: ManifestEntry) {
        self.entries.push(entry);
        self.increment_version();
    }

    /// Remove an entry by segment ID
    pub fn remove_entry(&mut self, segment_id: SegmentId) -> Option<ManifestEntry> {
        if let Some(pos) = self.entries.iter().position(|e| e.segment_id == segment_id) {
            let entry = self.entries.remove(pos);
            self.increment_version();
            Some(entry)
        } else {
            None
        }
    }

    /// Get an entry by segment ID
    pub fn get_entry(&self, segment_id: SegmentId) -> Option<&ManifestEntry> {
        self.entries.iter().find(|e| e.segment_id == segment_id)
    }

    /// Get all entries sorted by timestamp (newest first)
    pub fn get_entries_sorted(&self) -> Vec<ManifestEntry> {
        let mut sorted = self.entries.clone();
        sorted.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        sorted
    }

    /// Increment the manifest version
    fn increment_version(&mut self) {
        self.version = self.version.wrapping_add(1);
        self.created_at = current_timestamp_secs();
    }

    /// Get the total size of all segments
    pub fn total_size(&self) -> usize {
        self.entries.iter().map(|e| e.size).sum()
    }

    /// Get the number of entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Serialize the manifest to bytes using bincode
    pub fn serialize(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| ScribeError::Serialization(e.to_string()))
    }

    /// Deserialize a manifest from bytes using bincode
    pub fn deserialize(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).map_err(|e| ScribeError::Serialization(e.to_string()))
    }
}

impl Default for ClusterManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents the difference between two manifests
#[derive(Debug, Clone)]
pub struct ManifestDiff {
    /// Entries that were added
    pub added: Vec<ManifestEntry>,
    /// Entries that were removed (segment IDs)
    pub removed: Vec<SegmentId>,
    /// Entries that were modified
    pub modified: Vec<ManifestEntry>,
}

impl ManifestDiff {
    /// Create a new empty diff
    pub fn new() -> Self {
        Self {
            added: Vec::new(),
            removed: Vec::new(),
            modified: Vec::new(),
        }
    }

    /// Check if the diff is empty (no changes)
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }

    /// Count total number of changes
    pub fn change_count(&self) -> usize {
        self.added.len() + self.removed.len() + self.modified.len()
    }
}

impl Default for ManifestDiff {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute the difference between two manifests
///
/// Returns a ManifestDiff showing what changed from `old` to `new`.
pub fn compute_diff(old: &ClusterManifest, new: &ClusterManifest) -> ManifestDiff {
    let mut diff = ManifestDiff::new();

    // Build lookup maps for efficient comparison
    let old_map: HashMap<SegmentId, &ManifestEntry> =
        old.entries.iter().map(|e| (e.segment_id, e)).collect();
    let new_map: HashMap<SegmentId, &ManifestEntry> =
        new.entries.iter().map(|e| (e.segment_id, e)).collect();

    // Find added and modified entries
    for entry in &new.entries {
        match old_map.get(&entry.segment_id) {
            None => {
                // Entry exists in new but not in old - added
                diff.added.push(entry.clone());
            }
            Some(old_entry) => {
                // Entry exists in both - check if modified
                if old_entry != &entry {
                    diff.modified.push(entry.clone());
                }
            }
        }
    }

    // Find removed entries
    for entry in &old.entries {
        if !new_map.contains_key(&entry.segment_id) {
            diff.removed.push(entry.segment_id);
        }
    }

    diff
}

/// Merge two manifests with conflict resolution
///
/// Returns a new manifest that combines entries from both manifests.
/// In case of conflicts, the entry with the higher version manifest wins.
/// If versions are equal, the entry with newer timestamp is preferred.
pub fn merge_manifests(
    manifest1: &ClusterManifest,
    manifest2: &ClusterManifest,
) -> ClusterManifest {
    let mut merged_entries = HashMap::new();

    // Add all entries from manifest1
    for entry in &manifest1.entries {
        merged_entries.insert(entry.segment_id, entry.clone());
    }

    // Merge entries from manifest2, resolving conflicts
    for entry in &manifest2.entries {
        match merged_entries.get(&entry.segment_id) {
            None => {
                // No conflict, add the entry
                merged_entries.insert(entry.segment_id, entry.clone());
            }
            Some(existing_entry) => {
                // Conflict - resolve by version and timestamp
                if manifest2.version > manifest1.version {
                    // manifest2 has higher version, prefer its entry
                    merged_entries.insert(entry.segment_id, entry.clone());
                } else if manifest2.version == manifest1.version {
                    // Same version, prefer newer entry
                    if entry.timestamp > existing_entry.timestamp {
                        merged_entries.insert(entry.segment_id, entry.clone());
                    }
                }
                // Otherwise keep existing entry from manifest1
            }
        }
    }

    // Create merged manifest
    let mut entries: Vec<ManifestEntry> = merged_entries.into_values().collect();
    entries.sort_by_key(|e| e.segment_id);

    let version = std::cmp::max(manifest1.version, manifest2.version) + 1;

    ClusterManifest {
        version,
        entries,
        created_at: current_timestamp_secs(),
    }
}

/// Get the current Unix timestamp in seconds
fn current_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Get the current Unix timestamp in milliseconds
fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test constants to avoid hardcoded values
    const TEST_NODE_ID: u64 = 1;
    #[allow(dead_code)]
    const TEST_NODE_ID_2: u64 = 2;
    const TEST_NODE_ADDR: &str = "127.0.0.1:8080";
    #[allow(dead_code)]
    const TEST_NODE_ADDR_2: &str = "127.0.0.1:8081";

    #[test]
    fn test_cluster_node_new() {
        let node = ClusterNode::new(TEST_NODE_ID, TEST_NODE_ADDR.to_string());
        assert_eq!(node.id, TEST_NODE_ID);
        assert_eq!(node.address, TEST_NODE_ADDR);
        assert_eq!(node.state, NodeState::Joining);
        assert!(node.last_heartbeat > 0);
    }

    #[test]
    fn test_cluster_node_heartbeat() {
        let mut node = ClusterNode::new(TEST_NODE_ID, TEST_NODE_ADDR.to_string());
        let initial_heartbeat = node.last_heartbeat;

        std::thread::sleep(std::time::Duration::from_millis(10));
        node.update_heartbeat();

        assert!(node.last_heartbeat > initial_heartbeat);
    }

    #[test]
    fn test_cluster_node_state_transitions() {
        let mut node = ClusterNode::new(TEST_NODE_ID, TEST_NODE_ADDR.to_string());

        node.mark_active();
        assert_eq!(node.state, NodeState::Active);

        node.mark_suspected();
        assert_eq!(node.state, NodeState::Suspected);

        node.mark_down();
        assert_eq!(node.state, NodeState::Down);
    }

    #[test]
    fn test_cluster_node_heartbeat_stale() {
        let mut node = ClusterNode::new(TEST_NODE_ID, TEST_NODE_ADDR.to_string());

        // Fresh heartbeat should not be stale
        assert!(!node.is_heartbeat_stale(1000));

        // Set old heartbeat
        node.last_heartbeat = current_timestamp_ms() - 2000;
        assert!(node.is_heartbeat_stale(1000));
    }

    #[test]
    fn test_manifest_entry_new() {
        let entry = ManifestEntry::new(1, 1234567890, vec![1, 2, 3, 4], 1024);
        assert_eq!(entry.segment_id, 1);
        assert_eq!(entry.timestamp, 1234567890);
        assert_eq!(entry.merkle_root, vec![1, 2, 3, 4]);
        assert_eq!(entry.size, 1024);
    }

    #[test]
    fn test_manifest_entry_with_current_timestamp() {
        let entry = ManifestEntry::with_current_timestamp(1, vec![1, 2, 3, 4], 1024);
        assert_eq!(entry.segment_id, 1);
        assert!(entry.timestamp > 0);
        assert_eq!(entry.merkle_root, vec![1, 2, 3, 4]);
        assert_eq!(entry.size, 1024);
    }

    #[test]
    fn test_cluster_manifest_new() {
        let manifest = ClusterManifest::new();
        assert_eq!(manifest.version, 0);
        assert!(manifest.entries.is_empty());
        assert!(manifest.created_at > 0);
    }

    #[test]
    fn test_cluster_manifest_add_entry() {
        let mut manifest = ClusterManifest::new();
        let entry = ManifestEntry::new(1, 1234567890, vec![1, 2, 3, 4], 1024);

        manifest.add_entry(entry.clone());

        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.entries.len(), 1);
        assert_eq!(manifest.entries[0], entry);
    }

    #[test]
    fn test_cluster_manifest_remove_entry() {
        let mut manifest = ClusterManifest::new();
        let entry = ManifestEntry::new(1, 1234567890, vec![1, 2, 3, 4], 1024);

        manifest.add_entry(entry.clone());
        assert_eq!(manifest.version, 1);

        let removed = manifest.remove_entry(1);
        assert_eq!(manifest.version, 2);
        assert_eq!(removed, Some(entry));
        assert!(manifest.entries.is_empty());

        // Removing non-existent entry
        let removed = manifest.remove_entry(999);
        assert_eq!(removed, None);
        assert_eq!(manifest.version, 2); // Version unchanged
    }

    #[test]
    fn test_cluster_manifest_get_entry() {
        let mut manifest = ClusterManifest::new();
        let entry1 = ManifestEntry::new(1, 1234567890, vec![1, 2, 3, 4], 1024);
        let entry2 = ManifestEntry::new(2, 1234567891, vec![5, 6, 7, 8], 2048);

        manifest.add_entry(entry1.clone());
        manifest.add_entry(entry2.clone());

        assert_eq!(manifest.get_entry(1), Some(&entry1));
        assert_eq!(manifest.get_entry(2), Some(&entry2));
        assert_eq!(manifest.get_entry(999), None);
    }

    #[test]
    fn test_cluster_manifest_get_entries_sorted() {
        let mut manifest = ClusterManifest::new();
        let entry1 = ManifestEntry::new(1, 1000, vec![1], 1024);
        let entry2 = ManifestEntry::new(2, 3000, vec![2], 2048);
        let entry3 = ManifestEntry::new(3, 2000, vec![3], 3072);

        manifest.add_entry(entry1);
        manifest.add_entry(entry2.clone());
        manifest.add_entry(entry3.clone());

        let sorted = manifest.get_entries_sorted();
        assert_eq!(sorted.len(), 3);
        // Should be sorted by timestamp descending (newest first)
        assert_eq!(sorted[0], entry2);
        assert_eq!(sorted[1], entry3);
    }

    #[test]
    fn test_cluster_manifest_total_size() {
        let mut manifest = ClusterManifest::new();
        manifest.add_entry(ManifestEntry::new(1, 1000, vec![1], 1024));
        manifest.add_entry(ManifestEntry::new(2, 2000, vec![2], 2048));
        manifest.add_entry(ManifestEntry::new(3, 3000, vec![3], 3072));

        assert_eq!(manifest.total_size(), 1024 + 2048 + 3072);
        assert_eq!(manifest.entry_count(), 3);
    }

    #[test]
    fn test_cluster_manifest_serialization() {
        let mut manifest = ClusterManifest::new();
        manifest.add_entry(ManifestEntry::new(1, 1234567890, vec![1, 2, 3, 4], 1024));
        manifest.add_entry(ManifestEntry::new(2, 1234567891, vec![5, 6, 7, 8], 2048));

        let bytes = manifest.serialize().unwrap();
        let deserialized = ClusterManifest::deserialize(&bytes).unwrap();

        assert_eq!(deserialized.version, manifest.version);
        assert_eq!(deserialized.entries.len(), manifest.entries.len());
        assert_eq!(deserialized.entries, manifest.entries);
    }

    #[test]
    fn test_manifest_diff_empty() {
        let diff = ManifestDiff::new();
        assert!(diff.is_empty());
        assert_eq!(diff.change_count(), 0);
    }

    #[test]
    fn test_compute_diff_added() {
        let old = ClusterManifest::new();
        let mut new = ClusterManifest::new();
        new.add_entry(ManifestEntry::new(1, 1000, vec![1], 1024));

        let diff = compute_diff(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 0);
        assert_eq!(diff.modified.len(), 0);
    }

    #[test]
    fn test_compute_diff_removed() {
        let mut old = ClusterManifest::new();
        old.add_entry(ManifestEntry::new(1, 1000, vec![1], 1024));
        let new = ClusterManifest::new();

        let diff = compute_diff(&old, &new);
        assert_eq!(diff.added.len(), 0);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0], 1);
        assert_eq!(diff.modified.len(), 0);
    }

    #[test]
    fn test_compute_diff_modified() {
        let mut old = ClusterManifest::new();
        old.add_entry(ManifestEntry::new(1, 1000, vec![1], 1024));

        let mut new = ClusterManifest::new();
        new.add_entry(ManifestEntry::new(1, 1000, vec![2], 1024)); // Different merkle root

        let diff = compute_diff(&old, &new);
        assert_eq!(diff.added.len(), 0);
        assert_eq!(diff.removed.len(), 0);
        assert_eq!(diff.modified.len(), 1);
    }

    #[test]
    fn test_compute_diff_complex() {
        let mut old = ClusterManifest::new();
        old.add_entry(ManifestEntry::new(1, 1000, vec![1], 1024));
        old.add_entry(ManifestEntry::new(2, 2000, vec![2], 2048));
        old.add_entry(ManifestEntry::new(3, 3000, vec![3], 3072));

        let mut new = ClusterManifest::new();
        new.add_entry(ManifestEntry::new(1, 1000, vec![1], 1024)); // Unchanged
        new.add_entry(ManifestEntry::new(2, 2000, vec![99], 2048)); // Modified
        new.add_entry(ManifestEntry::new(4, 4000, vec![4], 4096)); // Added
                                                                   // 3 is removed

        let diff = compute_diff(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].segment_id, 4);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0], 3);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0].segment_id, 2);
    }

    #[test]
    fn test_merge_manifests_no_conflict() {
        let mut manifest1 = ClusterManifest::new();
        manifest1.add_entry(ManifestEntry::new(1, 1000, vec![1], 1024));

        let mut manifest2 = ClusterManifest::new();
        manifest2.add_entry(ManifestEntry::new(2, 2000, vec![2], 2048));

        let merged = merge_manifests(&manifest1, &manifest2);

        assert_eq!(merged.entries.len(), 2);
        assert!(merged.get_entry(1).is_some());
        assert!(merged.get_entry(2).is_some());
    }

    #[test]
    fn test_merge_manifests_with_conflict_by_version() {
        let mut manifest1 = ClusterManifest::new();
        manifest1.add_entry(ManifestEntry::new(1, 1000, vec![1], 1024));
        manifest1.version = 1;

        let mut manifest2 = ClusterManifest::new();
        manifest2.add_entry(ManifestEntry::new(1, 1000, vec![2], 2048)); // Different data
        manifest2.version = 2; // Higher version

        let merged = merge_manifests(&manifest1, &manifest2);

        assert_eq!(merged.entries.len(), 1);
        let entry = merged.get_entry(1).unwrap();
        // Should prefer manifest2 entry due to higher version
        assert_eq!(entry.merkle_root, vec![2]);
        assert_eq!(entry.size, 2048);
    }

    #[test]
    fn test_merge_manifests_with_conflict_by_timestamp() {
        let mut manifest1 = ClusterManifest::new();
        manifest1.add_entry(ManifestEntry::new(1, 1000, vec![1], 1024));
        manifest1.version = 1;

        let mut manifest2 = ClusterManifest::new();
        manifest2.add_entry(ManifestEntry::new(1, 2000, vec![2], 2048)); // Newer timestamp
        manifest2.version = 1; // Same version

        let merged = merge_manifests(&manifest1, &manifest2);

        assert_eq!(merged.entries.len(), 1);
        let entry = merged.get_entry(1).unwrap();
        // Should prefer manifest2 entry due to newer timestamp
        assert_eq!(entry.timestamp, 2000);
        assert_eq!(entry.merkle_root, vec![2]);
    }

    #[test]
    fn test_node_state_serialization() {
        let state = NodeState::Active;
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: NodeState = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, NodeState::Active);
    }

    #[test]
    fn test_cluster_node_serialization() {
        let node = ClusterNode::new(TEST_NODE_ID, TEST_NODE_ADDR.to_string());
        let json = serde_json::to_string(&node).unwrap();
        let deserialized: ClusterNode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, node.id);
        assert_eq!(deserialized.address, node.address);
        assert_eq!(deserialized.state, node.state);
    }
}
