/// Manifest management for tracking segment metadata
use crate::types::{Manifest, SegmentMetadata, KeyRange};

/// Manifest manager for handling segment metadata
pub struct ManifestManager {
    manifest: Manifest,
}

impl ManifestManager {
    /// Create a new manifest manager
    pub fn new() -> Self {
        Self {
            manifest: Manifest::new(),
        }
    }
    
    /// Add a new segment to the manifest
    pub fn add_segment(&mut self, metadata: SegmentMetadata) {
        self.manifest.segments.insert(metadata.id.clone(), metadata);
        self.manifest.version += 1;
        self.manifest.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
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
    pub fn get_manifest(&self) -> &Manifest {
        &self.manifest
    }
    
    /// Update the entire manifest
    pub fn update_manifest(&mut self, manifest: Manifest) {
        self.manifest = manifest;
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
        Self::new()
    }
}