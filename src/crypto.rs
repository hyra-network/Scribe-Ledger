use sha2::{Sha256, Digest};
use crate::types::Hash;

/// Compute SHA256 hash of data
pub fn hash_data(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Merkle tree implementation for data verification
pub struct MerkleTree {
    leaves: Vec<Hash>,
    tree: Vec<Vec<Hash>>,
}

impl MerkleTree {
    /// Create a new Merkle tree from leaf data
    pub fn new(data: Vec<&[u8]>) -> Self {
        let leaves: Vec<Hash> = data.iter().map(|d| hash_data(d)).collect();
        let mut tree = vec![leaves.clone()];
        
        let mut current_level = leaves.clone();
        
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in current_level.chunks(2) {
                let combined = if chunk.len() == 2 {
                    [chunk[0], chunk[1]].concat()
                } else {
                    [chunk[0], chunk[0]].concat()
                };
                next_level.push(hash_data(&combined));
            }
            
            tree.push(next_level.clone());
            current_level = next_level;
        }
        
        Self { leaves, tree }
    }
    
    /// Get the Merkle root
    pub fn root(&self) -> Option<Hash> {
        self.tree.last()?.first().copied()
    }
    
    /// Generate a Merkle proof for a given leaf index
    pub fn proof(&self, leaf_index: usize) -> Option<Vec<Hash>> {
        if leaf_index >= self.leaves.len() {
            return None;
        }
        
        let mut proof = Vec::new();
        let mut index = leaf_index;
        
        for level in &self.tree[..self.tree.len() - 1] {
            let sibling_index = if index.is_multiple_of(2) { index + 1 } else { index - 1 };
            
            if sibling_index < level.len() {
                proof.push(level[sibling_index]);
            }
            
            index /= 2;
        }
        
        Some(proof)
    }
    
    /// Verify a Merkle proof
    pub fn verify_proof(leaf: Hash, proof: &[Hash], root: Hash) -> bool {
        let mut current = leaf;
        
        for &sibling in proof {
            let combined = [current, sibling].concat();
            current = hash_data(&combined);
        }
        
        current == root
    }
}