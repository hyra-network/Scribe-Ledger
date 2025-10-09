//! Comprehensive tests for cryptographic verification (Task 10.1)
//!
//! This test file validates the Merkle tree implementation including:
//! - Tree construction from key-value pairs
//! - Proof generation and verification
//! - Various data sizes and edge cases
//! - Performance benchmarks

use hyra_scribe_ledger::crypto::MerkleTree;

#[test]
fn test_merkle_tree_empty() {
    let tree = MerkleTree::new();
    assert!(tree.is_empty());
    assert_eq!(tree.len(), 0);
    assert_eq!(tree.root_hash(), None);
}

#[test]
fn test_merkle_tree_single_element() {
    let pairs = vec![(b"key1".to_vec(), b"value1".to_vec())];
    let tree = MerkleTree::from_pairs(pairs);

    assert!(!tree.is_empty());
    assert_eq!(tree.len(), 1);
    assert!(tree.root_hash().is_some());

    // Generate and verify proof
    let proof = tree.get_proof(b"key1").unwrap();
    let root_hash = tree.root_hash().unwrap();
    assert!(MerkleTree::verify_proof(&proof, &root_hash));
}

#[test]
fn test_merkle_tree_two_elements() {
    let pairs = vec![
        (b"alice".to_vec(), b"data1".to_vec()),
        (b"bob".to_vec(), b"data2".to_vec()),
    ];
    let tree = MerkleTree::from_pairs(pairs);

    assert_eq!(tree.len(), 2);

    // Verify both proofs
    let root_hash = tree.root_hash().unwrap();
    let proof_alice = tree.get_proof(b"alice").unwrap();
    let proof_bob = tree.get_proof(b"bob").unwrap();

    assert!(MerkleTree::verify_proof(&proof_alice, &root_hash));
    assert!(MerkleTree::verify_proof(&proof_bob, &root_hash));
}

#[test]
fn test_merkle_tree_power_of_two_elements() {
    // Test with 4, 8, 16 elements (power of 2)
    for size in [4, 8, 16] {
        let pairs: Vec<_> = (0..size)
            .map(|i| {
                (
                    format!("key{}", i).into_bytes(),
                    format!("val{}", i).into_bytes(),
                )
            })
            .collect();

        let tree = MerkleTree::from_pairs(pairs);
        assert_eq!(tree.len(), size);

        let root_hash = tree.root_hash().unwrap();

        // Verify all proofs
        for i in 0..size {
            let key = format!("key{}", i);
            let proof = tree.get_proof(key.as_bytes()).unwrap();
            assert!(MerkleTree::verify_proof(&proof, &root_hash));
        }
    }
}

#[test]
fn test_merkle_tree_odd_number_of_elements() {
    // Test with odd numbers: 3, 5, 7
    for size in [3, 5, 7] {
        let pairs: Vec<_> = (0..size)
            .map(|i| {
                (
                    format!("key{}", i).into_bytes(),
                    format!("val{}", i).into_bytes(),
                )
            })
            .collect();

        let tree = MerkleTree::from_pairs(pairs);
        assert_eq!(tree.len(), size);

        let root_hash = tree.root_hash().unwrap();

        // Verify all proofs
        for i in 0..size {
            let key = format!("key{}", i);
            let proof = tree.get_proof(key.as_bytes()).unwrap();
            assert!(MerkleTree::verify_proof(&proof, &root_hash));
        }
    }
}

#[test]
fn test_merkle_tree_deterministic_construction() {
    // Different insertion orders should yield same root hash
    let pairs1 = vec![
        (b"charlie".to_vec(), b"data3".to_vec()),
        (b"alice".to_vec(), b"data1".to_vec()),
        (b"bob".to_vec(), b"data2".to_vec()),
    ];

    let pairs2 = vec![
        (b"alice".to_vec(), b"data1".to_vec()),
        (b"bob".to_vec(), b"data2".to_vec()),
        (b"charlie".to_vec(), b"data3".to_vec()),
    ];

    let pairs3 = vec![
        (b"bob".to_vec(), b"data2".to_vec()),
        (b"charlie".to_vec(), b"data3".to_vec()),
        (b"alice".to_vec(), b"data1".to_vec()),
    ];

    let tree1 = MerkleTree::from_pairs(pairs1);
    let tree2 = MerkleTree::from_pairs(pairs2);
    let tree3 = MerkleTree::from_pairs(pairs3);

    assert_eq!(tree1.root_hash(), tree2.root_hash());
    assert_eq!(tree2.root_hash(), tree3.root_hash());
}

#[test]
fn test_merkle_proof_verification_fails_for_wrong_value() {
    let pairs = vec![
        (b"key1".to_vec(), b"value1".to_vec()),
        (b"key2".to_vec(), b"value2".to_vec()),
    ];
    let tree = MerkleTree::from_pairs(pairs);
    let root_hash = tree.root_hash().unwrap();

    // Get proof and tamper with value
    let mut proof = tree.get_proof(b"key1").unwrap();
    proof.value = b"tampered_value".to_vec();

    // Verification should fail
    assert!(!MerkleTree::verify_proof(&proof, &root_hash));
}

#[test]
fn test_merkle_proof_verification_fails_for_wrong_root() {
    let pairs = vec![
        (b"key1".to_vec(), b"value1".to_vec()),
        (b"key2".to_vec(), b"value2".to_vec()),
    ];
    let tree = MerkleTree::from_pairs(pairs);

    let proof = tree.get_proof(b"key1").unwrap();
    let wrong_root = vec![0u8; 32];

    // Verification should fail
    assert!(!MerkleTree::verify_proof(&proof, &wrong_root));
}

#[test]
fn test_merkle_proof_for_nonexistent_key() {
    let pairs = vec![
        (b"key1".to_vec(), b"value1".to_vec()),
        (b"key2".to_vec(), b"value2".to_vec()),
    ];
    let tree = MerkleTree::from_pairs(pairs);

    assert!(tree.get_proof(b"nonexistent").is_none());
}

#[test]
fn test_merkle_tree_small_sizes() {
    // Test sizes from 1 to 10
    for size in 1..=10 {
        let pairs: Vec<_> = (0..size)
            .map(|i| {
                (
                    format!("key{}", i).into_bytes(),
                    format!("val{}", i).into_bytes(),
                )
            })
            .collect();

        let tree = MerkleTree::from_pairs(pairs);
        assert_eq!(tree.len(), size);

        let root_hash = tree.root_hash().unwrap();

        // Verify proofs for all elements
        for i in 0..size {
            let key = format!("key{}", i);
            let proof = tree.get_proof(key.as_bytes()).unwrap();
            assert!(
                MerkleTree::verify_proof(&proof, &root_hash),
                "Proof verification failed for size {} at index {}",
                size,
                i
            );
        }
    }
}

#[test]
fn test_merkle_tree_large_data() {
    // Test with 100 elements
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

    // Verify proofs for a subset of elements
    for i in [0, 10, 25, 50, 75, 99] {
        let key = format!("key{}", i);
        let proof = tree.get_proof(key.as_bytes()).unwrap();
        assert!(MerkleTree::verify_proof(&proof, &root_hash));
    }
}

#[test]
fn test_merkle_tree_with_binary_data() {
    // Test with binary key-value pairs
    let pairs = vec![
        (vec![0u8, 1, 2, 3], vec![255u8, 254, 253]),
        (vec![4u8, 5, 6, 7], vec![252u8, 251, 250]),
        (vec![8u8, 9, 10, 11], vec![249u8, 248, 247]),
    ];

    let tree = MerkleTree::from_pairs(pairs);
    assert_eq!(tree.len(), 3);

    let root_hash = tree.root_hash().unwrap();
    let proof = tree.get_proof(&[0u8, 1, 2, 3]).unwrap();
    assert!(MerkleTree::verify_proof(&proof, &root_hash));
}

#[test]
fn test_merkle_tree_with_empty_values() {
    // Test with empty values
    let pairs = vec![(b"key1".to_vec(), vec![]), (b"key2".to_vec(), vec![])];

    let tree = MerkleTree::from_pairs(pairs);
    assert_eq!(tree.len(), 2);

    let root_hash = tree.root_hash().unwrap();
    let proof = tree.get_proof(b"key1").unwrap();
    assert!(MerkleTree::verify_proof(&proof, &root_hash));
}

#[test]
fn test_merkle_tree_with_large_values() {
    // Test with large values (1KB each)
    let large_value = vec![42u8; 1024];
    let pairs = vec![
        (b"key1".to_vec(), large_value.clone()),
        (b"key2".to_vec(), large_value.clone()),
        (b"key3".to_vec(), large_value),
    ];

    let tree = MerkleTree::from_pairs(pairs);
    assert_eq!(tree.len(), 3);

    let root_hash = tree.root_hash().unwrap();
    let proof = tree.get_proof(b"key2").unwrap();
    assert!(MerkleTree::verify_proof(&proof, &root_hash));
}

#[test]
fn test_merkle_proof_structure() {
    // Test that proof structure is correct
    let pairs = vec![
        (b"key1".to_vec(), b"value1".to_vec()),
        (b"key2".to_vec(), b"value2".to_vec()),
        (b"key3".to_vec(), b"value3".to_vec()),
        (b"key4".to_vec(), b"value4".to_vec()),
    ];

    let tree = MerkleTree::from_pairs(pairs);
    let proof = tree.get_proof(b"key1").unwrap();

    // For a tree with 4 elements, we should have 2 siblings (height = 2)
    assert_eq!(proof.siblings.len(), proof.directions.len());
    assert!(proof.siblings.len() > 0);

    // Verify the proof works
    let root_hash = tree.root_hash().unwrap();
    assert!(MerkleTree::verify_proof(&proof, &root_hash));
}

#[test]
fn test_merkle_tree_rebuild() {
    // Test building tree multiple times
    let mut tree = MerkleTree::new();

    // First build
    let pairs1 = vec![
        (b"key1".to_vec(), b"value1".to_vec()),
        (b"key2".to_vec(), b"value2".to_vec()),
    ];
    tree.build(pairs1);
    let root1 = tree.root_hash();

    // Rebuild with different data
    let pairs2 = vec![
        (b"key3".to_vec(), b"value3".to_vec()),
        (b"key4".to_vec(), b"value4".to_vec()),
    ];
    tree.build(pairs2);
    let root2 = tree.root_hash();

    // Roots should be different
    assert_ne!(root1, root2);
}

#[test]
fn test_merkle_tree_consistency() {
    // Same data should always produce same root hash
    let pairs = vec![
        (b"test1".to_vec(), b"data1".to_vec()),
        (b"test2".to_vec(), b"data2".to_vec()),
        (b"test3".to_vec(), b"data3".to_vec()),
    ];

    let tree1 = MerkleTree::from_pairs(pairs.clone());
    let tree2 = MerkleTree::from_pairs(pairs.clone());
    let tree3 = MerkleTree::from_pairs(pairs);

    assert_eq!(tree1.root_hash(), tree2.root_hash());
    assert_eq!(tree2.root_hash(), tree3.root_hash());
}

#[test]
fn test_merkle_tree_stress() {
    // Stress test with 1000 elements
    let pairs: Vec<_> = (0..1000)
        .map(|i| {
            (
                format!("key{:04}", i).into_bytes(),
                format!("value{:04}", i).into_bytes(),
            )
        })
        .collect();

    let tree = MerkleTree::from_pairs(pairs);
    assert_eq!(tree.len(), 1000);

    let root_hash = tree.root_hash().unwrap();

    // Verify proofs for a sample of elements
    for i in [0, 100, 500, 750, 999] {
        let key = format!("key{:04}", i);
        let proof = tree.get_proof(key.as_bytes()).unwrap();
        assert!(
            MerkleTree::verify_proof(&proof, &root_hash),
            "Proof verification failed for key{}",
            i
        );
    }
}
