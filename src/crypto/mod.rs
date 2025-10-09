//! Cryptographic verification module for Merkle tree support
//!
//! This module provides Merkle tree implementation for data verification,
//! allowing generation of cryptographic proofs for key-value pairs and
//! verification of data integrity.

use sha2::{Digest, Sha256};

/// A Merkle tree node
#[derive(Debug, Clone, PartialEq, Eq)]
enum MerkleNode {
    /// Leaf node containing a hash of key-value pair
    Leaf { hash: Vec<u8> },
    /// Internal node containing combined hash of children
    Internal {
        hash: Vec<u8>,
        left: Box<MerkleNode>,
        right: Box<MerkleNode>,
    },
}

impl MerkleNode {
    /// Get the hash of this node
    fn hash(&self) -> &[u8] {
        match self {
            MerkleNode::Leaf { hash } => hash,
            MerkleNode::Internal { hash, .. } => hash,
        }
    }
}

/// A Merkle tree for cryptographic verification of key-value pairs
#[derive(Debug, Clone)]
pub struct MerkleTree {
    root: Option<MerkleNode>,
    leaves: Vec<(Vec<u8>, Vec<u8>)>, // Store original key-value pairs
}

/// A proof for a specific key in the Merkle tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerkleProof {
    /// The key being proven
    pub key: Vec<u8>,
    /// The value associated with the key
    pub value: Vec<u8>,
    /// Sibling hashes along the path from leaf to root
    pub siblings: Vec<Vec<u8>>,
    /// Directions (true = right, false = left) for path from leaf to root
    pub directions: Vec<bool>,
}

impl MerkleTree {
    /// Create a new empty Merkle tree
    pub fn new() -> Self {
        Self {
            root: None,
            leaves: Vec::new(),
        }
    }

    /// Create a Merkle tree from key-value pairs
    pub fn from_pairs(pairs: Vec<(Vec<u8>, Vec<u8>)>) -> Self {
        let mut tree = Self::new();
        tree.build(pairs);
        tree
    }

    /// Build the Merkle tree from key-value pairs
    pub fn build(&mut self, mut pairs: Vec<(Vec<u8>, Vec<u8>)>) {
        if pairs.is_empty() {
            self.root = None;
            self.leaves = Vec::new();
            return;
        }

        // Sort pairs by key for deterministic tree construction
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        self.leaves = pairs.clone();

        // Create leaf nodes
        let mut nodes: Vec<MerkleNode> = pairs
            .iter()
            .map(|(key, value)| {
                let hash = Self::hash_leaf(key, value);
                MerkleNode::Leaf { hash }
            })
            .collect();

        // Handle single element case
        if nodes.len() == 1 {
            self.root = Some(nodes.into_iter().next().unwrap());
            return;
        }

        // Build tree bottom-up
        while nodes.len() > 1 {
            let mut next_level = Vec::new();

            for chunk in nodes.chunks(2) {
                if chunk.len() == 2 {
                    // Combine two nodes
                    let left = chunk[0].clone();
                    let right = chunk[1].clone();
                    let combined_hash = Self::hash_internal(left.hash(), right.hash());
                    next_level.push(MerkleNode::Internal {
                        hash: combined_hash,
                        left: Box::new(left),
                        right: Box::new(right),
                    });
                } else {
                    // Odd node out - promote it as-is (duplicate if needed for balanced tree)
                    let node = chunk[0].clone();
                    let hash = Self::hash_internal(node.hash(), node.hash());
                    next_level.push(MerkleNode::Internal {
                        hash,
                        left: Box::new(node.clone()),
                        right: Box::new(node),
                    });
                }
            }

            nodes = next_level;
        }

        self.root = nodes.into_iter().next();
    }

    /// Get the root hash of the tree
    pub fn root_hash(&self) -> Option<Vec<u8>> {
        self.root.as_ref().map(|node| node.hash().to_vec())
    }

    /// Hash a leaf node (key-value pair)
    fn hash_leaf(key: &[u8], value: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"leaf:");
        hasher.update(key);
        hasher.update(b":");
        hasher.update(value);
        hasher.finalize().to_vec()
    }

    /// Hash an internal node (combination of two child hashes)
    fn hash_internal(left: &[u8], right: &[u8]) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(b"internal:");
        hasher.update(left);
        hasher.update(b":");
        hasher.update(right);
        hasher.finalize().to_vec()
    }

    /// Generate a proof for a specific key
    pub fn get_proof(&self, key: &[u8]) -> Option<MerkleProof> {
        // Find the key in leaves
        let leaf_index = self.leaves.iter().position(|(k, _)| k == key)?;
        let (_, value) = &self.leaves[leaf_index];

        // Generate proof by traversing from leaf to root
        let mut siblings = Vec::new();
        let mut directions = Vec::new();

        // Build all leaf nodes to find our position
        let leaf_nodes: Vec<MerkleNode> = self
            .leaves
            .iter()
            .map(|(k, v)| MerkleNode::Leaf {
                hash: Self::hash_leaf(k, v),
            })
            .collect();

        if leaf_nodes.len() == 1 {
            // Single element tree - no siblings
            return Some(MerkleProof {
                key: key.to_vec(),
                value: value.clone(),
                siblings: Vec::new(),
                directions: Vec::new(),
            });
        }

        // Traverse the tree and collect sibling hashes
        let mut current_nodes = leaf_nodes;
        let mut current_index = leaf_index;

        while current_nodes.len() > 1 {
            let mut next_level = Vec::new();

            for (chunk_idx, chunk) in current_nodes.chunks(2).enumerate() {
                let chunk_start = chunk_idx * 2;

                if chunk.len() == 2 {
                    let left = &chunk[0];
                    let right = &chunk[1];

                    // Check if current node is in this chunk
                    if current_index == chunk_start {
                        // Current node is left child
                        siblings.push(right.hash().to_vec());
                        directions.push(false); // We are on the left
                    } else if current_index == chunk_start + 1 {
                        // Current node is right child
                        siblings.push(left.hash().to_vec());
                        directions.push(true); // We are on the right
                    }

                    let combined_hash = Self::hash_internal(left.hash(), right.hash());
                    next_level.push(MerkleNode::Internal {
                        hash: combined_hash,
                        left: Box::new(left.clone()),
                        right: Box::new(right.clone()),
                    });
                } else {
                    // Odd node - duplicate it
                    let node = &chunk[0];

                    if current_index == chunk_start {
                        siblings.push(node.hash().to_vec());
                        directions.push(false);
                    }

                    let hash = Self::hash_internal(node.hash(), node.hash());
                    next_level.push(MerkleNode::Internal {
                        hash,
                        left: Box::new(node.clone()),
                        right: Box::new(node.clone()),
                    });
                }
            }

            current_nodes = next_level;
            current_index /= 2;
        }

        Some(MerkleProof {
            key: key.to_vec(),
            value: value.clone(),
            siblings,
            directions,
        })
    }

    /// Verify a proof against a root hash
    pub fn verify_proof(proof: &MerkleProof, root_hash: &[u8]) -> bool {
        if proof.siblings.len() != proof.directions.len() {
            return false;
        }

        // Start with leaf hash
        let mut current_hash = Self::hash_leaf(&proof.key, &proof.value);

        // Traverse up the tree using siblings and directions
        for (sibling, &is_right) in proof.siblings.iter().zip(proof.directions.iter()) {
            current_hash = if is_right {
                // Current node is on the right, sibling is on the left
                Self::hash_internal(sibling, &current_hash)
            } else {
                // Current node is on the left, sibling is on the right
                Self::hash_internal(&current_hash, sibling)
            };
        }

        // Check if computed hash matches root hash
        current_hash == root_hash
    }

    /// Get the number of leaf nodes in the tree
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Check if the tree is empty
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let tree = MerkleTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        assert_eq!(tree.root_hash(), None);
    }

    #[test]
    fn test_single_element_tree() {
        let pairs = vec![(b"key1".to_vec(), b"value1".to_vec())];
        let tree = MerkleTree::from_pairs(pairs);

        assert!(!tree.is_empty());
        assert_eq!(tree.len(), 1);
        assert!(tree.root_hash().is_some());

        // Test proof for single element
        let proof = tree.get_proof(b"key1").unwrap();
        assert_eq!(proof.key, b"key1");
        assert_eq!(proof.value, b"value1");
        assert!(proof.siblings.is_empty());
        assert!(proof.directions.is_empty());

        // Verify proof
        let root_hash = tree.root_hash().unwrap();
        assert!(MerkleTree::verify_proof(&proof, &root_hash));
    }

    #[test]
    fn test_two_element_tree() {
        let pairs = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
        ];
        let tree = MerkleTree::from_pairs(pairs);

        assert_eq!(tree.len(), 2);
        assert!(tree.root_hash().is_some());

        // Test proof for first key
        let proof1 = tree.get_proof(b"key1").unwrap();
        assert_eq!(proof1.siblings.len(), 1);
        assert_eq!(proof1.directions.len(), 1);

        let root_hash = tree.root_hash().unwrap();
        assert!(MerkleTree::verify_proof(&proof1, &root_hash));

        // Test proof for second key
        let proof2 = tree.get_proof(b"key2").unwrap();
        assert_eq!(proof2.siblings.len(), 1);
        assert!(MerkleTree::verify_proof(&proof2, &root_hash));
    }

    #[test]
    fn test_multiple_elements_tree() {
        let pairs = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
            (b"key4".to_vec(), b"value4".to_vec()),
        ];
        let tree = MerkleTree::from_pairs(pairs);

        assert_eq!(tree.len(), 4);

        let root_hash = tree.root_hash().unwrap();

        // Verify all proofs
        for i in 1..=4 {
            let key = format!("key{}", i);
            let proof = tree.get_proof(key.as_bytes()).unwrap();
            assert!(MerkleTree::verify_proof(&proof, &root_hash));
        }
    }

    #[test]
    fn test_odd_number_of_elements() {
        let pairs = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
        ];
        let tree = MerkleTree::from_pairs(pairs);

        assert_eq!(tree.len(), 3);

        let root_hash = tree.root_hash().unwrap();

        // Verify all proofs
        for i in 1..=3 {
            let key = format!("key{}", i);
            let proof = tree.get_proof(key.as_bytes()).unwrap();
            assert!(MerkleTree::verify_proof(&proof, &root_hash));
        }
    }

    #[test]
    fn test_proof_verification_fails_for_wrong_value() {
        let pairs = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
        ];
        let tree = MerkleTree::from_pairs(pairs);
        let root_hash = tree.root_hash().unwrap();

        // Create proof with wrong value
        let mut proof = tree.get_proof(b"key1").unwrap();
        proof.value = b"wrong_value".to_vec();

        assert!(!MerkleTree::verify_proof(&proof, &root_hash));
    }

    #[test]
    fn test_proof_verification_fails_for_wrong_root() {
        let pairs = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
        ];
        let tree = MerkleTree::from_pairs(pairs);

        let proof = tree.get_proof(b"key1").unwrap();
        let wrong_root = vec![0u8; 32]; // Wrong root hash

        assert!(!MerkleTree::verify_proof(&proof, &wrong_root));
    }

    #[test]
    fn test_nonexistent_key() {
        let pairs = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
        ];
        let tree = MerkleTree::from_pairs(pairs);

        assert!(tree.get_proof(b"nonexistent").is_none());
    }

    #[test]
    fn test_deterministic_tree_construction() {
        let pairs1 = vec![
            (b"key2".to_vec(), b"value2".to_vec()),
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
        ];

        let pairs2 = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
        ];

        let tree1 = MerkleTree::from_pairs(pairs1);
        let tree2 = MerkleTree::from_pairs(pairs2);

        // Trees should have same root hash despite different input order
        assert_eq!(tree1.root_hash(), tree2.root_hash());
    }

    #[test]
    fn test_large_tree() {
        let pairs: Vec<_> = (0..100)
            .map(|i| {
                (
                    format!("key{}", i).into_bytes(),
                    format!("value{}", i).into_bytes(),
                )
            })
            .collect();

        let tree = MerkleTree::from_pairs(pairs);
        assert_eq!(tree.len(), 100);

        let root_hash = tree.root_hash().unwrap();

        // Verify some random proofs
        for i in [0, 25, 50, 75, 99] {
            let key = format!("key{}", i);
            let proof = tree.get_proof(key.as_bytes()).unwrap();
            assert!(MerkleTree::verify_proof(&proof, &root_hash));
        }
    }
}
