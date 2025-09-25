/// Manifest management for tracking segment metadata
use crate::types::{SegmentMetadata, KeyRange, SegmentId};
use crate::error::{Result, ScribeError};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

/// Cluster node information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterNode {
    pub id: u64,
    pub address: String,
    pub status: NodeStatus,
    pub last_seen: u64,
}

/// Node status in the cluster
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    Active,
    Inactive,
    Joining,
    Leaving,
}

/// Enhanced manifest with cluster information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterManifest {
    pub version: u64,
    pub updated_at: u64,
    pub leader_id: Option<u64>,
    pub segments: HashMap<SegmentId, SegmentMetadata>,
    pub nodes: HashMap<u64, ClusterNode>,
    pub cluster_size: usize,
}

impl ClusterManifest {
    pub fn new() -> Self {
        Self {
            version: 0,
            updated_at: current_timestamp(),
            leader_id: None,
            segments: HashMap::new(),
            nodes: HashMap::new(),
            cluster_size: 0,
        }
    }
}

/// Manifest manager for handling segment metadata and cluster state
pub struct ManifestManager {
    manifest: ClusterManifest,
    node_id: u64,
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl ManifestManager {
    /// Create a new manifest manager
    pub fn new(node_id: u64) -> Self {
        Self {
            manifest: ClusterManifest::new(),
            node_id,
        }
    }
    
    /// Add a new segment to the manifest
    pub fn add_segment(&mut self, metadata: SegmentMetadata) {
        self.manifest.segments.insert(metadata.id, metadata);
        self.manifest.version += 1;
        self.manifest.updated_at = current_timestamp();
    }
    
    /// Find segments that might contain a key
    pub fn find_segments_for_key(&self, key: &str) -> Vec<&SegmentMetadata> {
        self.manifest
            .segments
            .values()
            .filter(|segment| self.key_in_range(key, &segment.key_range))
            .collect()
    }
    
    /// Check if a key falls within a key range
    fn key_in_range(&self, key: &str, range: &KeyRange) -> bool {
        let start_ok = range.start.as_ref().is_none_or(|start| key >= start.as_str());
        let end_ok = range.end.as_ref().is_none_or(|end| key < end.as_str());
        start_ok && end_ok
    }
    
    /// Get the current manifest
    pub fn get_manifest(&self) -> &ClusterManifest {
        &self.manifest
    }
    
    /// Update the entire manifest
    pub fn update_manifest(&mut self, manifest: ClusterManifest) {
        self.manifest = manifest;
    }
    
    /// Add a node to the cluster
    pub fn add_node(&mut self, node: ClusterNode) -> Result<()> {
        self.manifest.nodes.insert(node.id, node);
        self.manifest.cluster_size = self.manifest.nodes.len();
        self.manifest.version += 1;
        self.manifest.updated_at = current_timestamp();
        Ok(())
    }
    
    /// Remove a node from the cluster
    pub fn remove_node(&mut self, node_id: u64) -> Result<()> {
        if let Some(_) = self.manifest.nodes.remove(&node_id) {
            self.manifest.cluster_size = self.manifest.nodes.len();
            self.manifest.version += 1;
            self.manifest.updated_at = current_timestamp();
            Ok(())
        } else {
            Err(ScribeError::Consensus(format!("Node {} not found", node_id)))
        }
    }
    
    /// Update node status
    pub fn update_node_status(&mut self, node_id: u64, status: NodeStatus) -> Result<()> {
        if let Some(node) = self.manifest.nodes.get_mut(&node_id) {
            node.status = status;
            node.last_seen = current_timestamp();
            self.manifest.version += 1;
            self.manifest.updated_at = current_timestamp();
            Ok(())
        } else {
            Err(ScribeError::Consensus(format!("Node {} not found", node_id)))
        }
    }
    
    /// Set cluster leader
    pub fn set_leader(&mut self, leader_id: u64) {
        self.manifest.leader_id = Some(leader_id);
        self.manifest.version += 1;
        self.manifest.updated_at = current_timestamp();
    }
    
    /// Get active nodes
    pub fn get_active_nodes(&self) -> Vec<&ClusterNode> {
        self.manifest
            .nodes
            .values()
            .filter(|node| node.status == NodeStatus::Active)
            .collect()
    }
    
    /// Check if node is leader
    pub fn is_leader(&self) -> bool {
        self.manifest.leader_id == Some(self.node_id)
    }
    
    /// Get segments ordered by creation time (newest first)
    pub fn get_segments_by_time(&self) -> Vec<&SegmentMetadata> {
        let mut segments: Vec<&SegmentMetadata> = self.manifest.segments.values().collect();
        segments.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        segments
    }
}

impl Default for ManifestManager {
    fn default() -> Self {
        Self::new(0) // Default node ID of 0
    }
}