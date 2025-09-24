use sha2::{Sha256, Digest};
use crate::types::Hash;
use serde::{Deserialize, Serialize};

/// Compute SHA256 hash of data
pub fn hash_data(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute SHA256 hash of two hashes (for internal nodes)
pub fn hash_pair(left: &Hash, right: &Hash) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

/// Direction in Merkle proof (left or right sibling)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofDirection {
    Left,
    Right,
}

/// A single step in a Merkle proof
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofStep {
    pub hash: Hash,
    pub direction: ProofDirection,
}

/// Complete Merkle proof for a leaf
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleProof {
    pub leaf_index: usize,
    pub leaf_hash: Hash,
    pub steps: Vec<ProofStep>,
    pub root: Hash,
}

/// Merkle tree implementation for data verification
#[derive(Debug, Clone)]
pub struct MerkleTree {
    leaves: Vec<Hash>,
    tree: Vec<Vec<Hash>>,
}

impl MerkleTree {
    /// Create a new Merkle tree from leaf data
    pub fn new(data: Vec<&[u8]>) -> Self {
        if data.is_empty() {
            return Self {
                leaves: vec![],
                tree: vec![],
            };
        }

        let leaves: Vec<Hash> = data.iter().map(|d| hash_data(d)).collect();
        let mut tree = vec![leaves.clone()];
        
        let mut current_level = leaves.clone();
        
        // Build tree bottom-up
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            // Process pairs and handle odd numbers by duplicating last element
            let mut i = 0;
            while i < current_level.len() {
                if i + 1 < current_level.len() {
                    // Hash pair of nodes
                    let combined_hash = hash_pair(&current_level[i], &current_level[i + 1]);
                    next_level.push(combined_hash);
                    i += 2;
                } else {
                    // Odd number of nodes - duplicate the last one
                    let combined_hash = hash_pair(&current_level[i], &current_level[i]);
                    next_level.push(combined_hash);
                    i += 1;
                }
            }
            
            tree.push(next_level.clone());
            current_level = next_level;
        }
        
        Self { leaves, tree }
    }

    /// Create tree from already computed leaf hashes
    pub fn from_hashes(leaf_hashes: Vec<Hash>) -> Self {
        if leaf_hashes.is_empty() {
            return Self {
                leaves: vec![],
                tree: vec![],
            };
        }

        let mut tree = vec![leaf_hashes.clone()];
        let mut current_level = leaf_hashes.clone();
        
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            
            let mut i = 0;
            while i < current_level.len() {
                if i + 1 < current_level.len() {
                    let combined_hash = hash_pair(&current_level[i], &current_level[i + 1]);
                    next_level.push(combined_hash);
                    i += 2;
                } else {
                    let combined_hash = hash_pair(&current_level[i], &current_level[i]);
                    next_level.push(combined_hash);
                    i += 1;
                }
            }
            
            tree.push(next_level.clone());
            current_level = next_level;
        }
        
        Self { 
            leaves: leaf_hashes, 
            tree 
        }
    }
    
    /// Get the Merkle root
    pub fn root(&self) -> Option<Hash> {
        if self.tree.is_empty() {
            return None;
        }
        self.tree.last()?.first().copied()
    }

    /// Get the leaf hashes
    pub fn leaves(&self) -> &[Hash] {
        &self.leaves
    }

    /// Get the number of leaves
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Check if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }
    
    /// Generate a Merkle proof for a given leaf index
    pub fn proof(&self, leaf_index: usize) -> Option<MerkleProof> {
        if leaf_index >= self.leaves.len() || self.tree.is_empty() {
            return None;
        }
        
        let leaf_hash = self.leaves[leaf_index];
        let root = self.root()?;
        let mut steps = Vec::new();
        let mut index = leaf_index;
        
        // Generate proof steps by traversing up the tree
        for level in &self.tree[..self.tree.len() - 1] {
            let sibling_index = if index % 2 == 0 { 
                index + 1 
            } else { 
                index - 1 
            };
            
            if sibling_index < level.len() {
                // Determine if sibling is on left or right
                let direction = if index % 2 == 0 {
                    ProofDirection::Right  // Sibling is to the right
                } else {
                    ProofDirection::Left   // Sibling is to the left
                };
                
                steps.push(ProofStep {
                    hash: level[sibling_index],
                    direction,
                });
            }
            
            index /= 2;
        }
        
        Some(MerkleProof {
            leaf_index,
            leaf_hash,
            steps,
            root,
        })
    }
    
    /// Verify a Merkle proof
    pub fn verify_proof(proof: &MerkleProof) -> bool {
        let mut current = proof.leaf_hash;
        
        for step in &proof.steps {
            current = match step.direction {
                ProofDirection::Left => hash_pair(&step.hash, &current),
                ProofDirection::Right => hash_pair(&current, &step.hash),
            };
        }
        
        current == proof.root
    }

    /// Verify a proof against a specific root (for external verification)
    pub fn verify_proof_against_root(leaf_hash: Hash, steps: &[ProofStep], expected_root: Hash) -> bool {
        let mut current = leaf_hash;
        
        for step in steps {
            current = match step.direction {
                ProofDirection::Left => hash_pair(&step.hash, &current),
                ProofDirection::Right => hash_pair(&current, &step.hash),
            };
        }
        
        current == expected_root
    }

    /// Get the sibling hash for a given leaf index (useful for debugging)
    pub fn get_sibling(&self, leaf_index: usize) -> Option<Hash> {
        if leaf_index >= self.leaves.len() || self.tree.is_empty() {
            return None;
        }
        
        let sibling_index = if leaf_index % 2 == 0 { 
            leaf_index + 1 
        } else { 
            leaf_index - 1 
        };
        
        self.tree[0].get(sibling_index).copied()
    }
}

/// Utility functions for working with Merkle trees and data
impl MerkleTree {
    /// Create a Merkle tree for key-value pairs (useful for storage verification)
    pub fn from_key_value_pairs(pairs: Vec<(&str, &[u8])>) -> Self {
        let data: Vec<Vec<u8>> = pairs.iter()
            .map(|(key, value)| {
                let mut combined = Vec::new();
                combined.extend_from_slice(key.as_bytes());
                combined.extend_from_slice(value);
                combined
            })
            .collect();
        
        let data_refs: Vec<&[u8]> = data.iter().map(|d| d.as_slice()).collect();
        Self::new(data_refs)
    }

    /// Create a proof for a key-value pair
    pub fn proof_for_key_value(&self, key: &str, value: &[u8], pairs: &[(&str, &[u8])]) -> Option<MerkleProof> {
        // Find the index of this key-value pair
        let target_data = {
            let mut combined = Vec::new();
            combined.extend_from_slice(key.as_bytes());
            combined.extend_from_slice(value);
            combined
        };
        
        for (i, (k, v)) in pairs.iter().enumerate() {
            let mut pair_data = Vec::new();
            pair_data.extend_from_slice(k.as_bytes());
            pair_data.extend_from_slice(v);
            
            if pair_data == target_data {
                return self.proof(i);
            }
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let tree = MerkleTree::new(vec![]);
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        assert!(tree.root().is_none());
    }

    #[test]
    fn test_single_leaf() {
        let data = vec![b"hello".as_slice()];
        let tree = MerkleTree::new(data);
        
        assert_eq!(tree.len(), 1);
        assert!(!tree.is_empty());
        
        let root = tree.root().unwrap();
        let expected_root = hash_data(b"hello");
        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_two_leaves() {
        let data = vec![b"hello".as_slice(), b"world".as_slice()];
        let tree = MerkleTree::new(data);
        
        assert_eq!(tree.len(), 2);
        
        let root = tree.root().unwrap();
        let left_hash = hash_data(b"hello");
        let right_hash = hash_data(b"world");
        let expected_root = hash_pair(&left_hash, &right_hash);
        
        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_four_leaves() {
        let data = vec![b"a".as_slice(), b"b".as_slice(), b"c".as_slice(), b"d".as_slice()];
        let tree = MerkleTree::new(data);
        
        assert_eq!(tree.len(), 4);
        
        // Verify tree structure
        let h_a = hash_data(b"a");
        let h_b = hash_data(b"b");
        let h_c = hash_data(b"c");
        let h_d = hash_data(b"d");
        
        let h_ab = hash_pair(&h_a, &h_b);
        let h_cd = hash_pair(&h_c, &h_d);
        let expected_root = hash_pair(&h_ab, &h_cd);
        
        assert_eq!(tree.root().unwrap(), expected_root);
    }

    #[test]
    fn test_odd_number_of_leaves() {
        let data = vec![b"a".as_slice(), b"b".as_slice(), b"c".as_slice()];
        let tree = MerkleTree::new(data);
        
        assert_eq!(tree.len(), 3);
        
        // With 3 leaves, the last one should be duplicated
        let h_a = hash_data(b"a");
        let h_b = hash_data(b"b");
        let h_c = hash_data(b"c");
        
        let h_ab = hash_pair(&h_a, &h_b);
        let h_cc = hash_pair(&h_c, &h_c); // c is duplicated
        let expected_root = hash_pair(&h_ab, &h_cc);
        
        assert_eq!(tree.root().unwrap(), expected_root);
    }

    #[test]
    fn test_merkle_proof_generation_and_verification() {
        let data = vec![b"a".as_slice(), b"b".as_slice(), b"c".as_slice(), b"d".as_slice()];
        let tree = MerkleTree::new(data);
        
        // Test proof for first leaf
        let proof = tree.proof(0).unwrap();
        assert_eq!(proof.leaf_index, 0);
        assert_eq!(proof.leaf_hash, hash_data(b"a"));
        assert!(MerkleTree::verify_proof(&proof));
        
        // Test proof for second leaf
        let proof = tree.proof(1).unwrap();
        assert_eq!(proof.leaf_index, 1);
        assert_eq!(proof.leaf_hash, hash_data(b"b"));
        assert!(MerkleTree::verify_proof(&proof));
        
        // Test proof for third leaf
        let proof = tree.proof(2).unwrap();
        assert_eq!(proof.leaf_index, 2);
        assert_eq!(proof.leaf_hash, hash_data(b"c"));
        assert!(MerkleTree::verify_proof(&proof));
    }

    #[test]
    fn test_invalid_proof() {
        let data = vec![b"a".as_slice(), b"b".as_slice(), b"c".as_slice(), b"d".as_slice()];
        let tree = MerkleTree::new(data);
        
        let mut proof = tree.proof(0).unwrap();
        
        // Tamper with the proof
        proof.leaf_hash = hash_data(b"tampered");
        
        // Verification should fail
        assert!(!MerkleTree::verify_proof(&proof));
    }

    #[test]
    fn test_proof_out_of_bounds() {
        let data = vec![b"a".as_slice(), b"b".as_slice()];
        let tree = MerkleTree::new(data);
        
        assert!(tree.proof(2).is_none());
        assert!(tree.proof(100).is_none());
    }

    #[test]
    fn test_key_value_merkle_tree() {
        let pairs = vec![
            ("key1", b"value1".as_slice()),
            ("key2", b"value2".as_slice()),
            ("key3", b"value3".as_slice()),
        ];
        
        let tree = MerkleTree::from_key_value_pairs(pairs.clone());
        assert_eq!(tree.len(), 3);
        
        // Test proof generation for key-value pair
        let proof = tree.proof_for_key_value("key2", b"value2", &pairs).unwrap();
        assert!(MerkleTree::verify_proof(&proof));
        
        // Test with non-existent key
        assert!(tree.proof_for_key_value("key4", b"value4", &pairs).is_none());
    }

    #[test]
    fn test_external_proof_verification() {
        let data = vec![b"test_data".as_slice()];
        let tree = MerkleTree::new(data);
        
        let proof = tree.proof(0).unwrap();
        let leaf_hash = hash_data(b"test_data");
        let root = tree.root().unwrap();
        
        // Test external verification
        assert!(MerkleTree::verify_proof_against_root(leaf_hash, &proof.steps, root));
        
        // Test with wrong root
        let wrong_root = hash_data(b"wrong_root");
        assert!(!MerkleTree::verify_proof_against_root(leaf_hash, &proof.steps, wrong_root));
    }
}