use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

/// A key in the ledger
pub type Key = String;

/// A value in the ledger  
pub type Value = Vec<u8>;

/// Cryptographic hash
pub type Hash = [u8; 32];

/// Node identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Segment identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SegmentId(pub Uuid);

impl SegmentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SegmentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Key range for segments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyRange {
    pub start: Option<Key>,
    pub end: Option<Key>,
}

/// Metadata for a segment stored in S3
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMetadata {
    pub id: SegmentId,
    pub key_range: KeyRange,
    pub merkle_root: Hash,
    pub size: u64,
    pub created_at: u64,
    pub s3_key: String,
}

/// Global manifest containing all segment metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u64,
    pub segments: HashMap<SegmentId, SegmentMetadata>,
    pub updated_at: u64,
}

impl Manifest {
    pub fn new() -> Self {
        Self {
            version: 0,
            segments: HashMap::new(),
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

impl Default for Manifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Storage receipt for put operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageReceipt {
    pub key: Key,
    pub segment_id: SegmentId,
    pub merkle_proof: Vec<Hash>,
    pub timestamp: u64,
}

impl StorageReceipt {
    pub fn merkle_proof(&self) -> String {
        // Convert merkle proof to hex string for on-chain submission
        self.merkle_proof
            .iter()
            .map(hex::encode)
            .collect::<Vec<_>>()
            .join(",")
    }
}