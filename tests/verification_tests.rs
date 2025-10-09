//! Tests for Merkle proof verification API endpoint (Task 10.2)
//!
//! This test file validates the verification endpoint implementation.

use hyra_scribe_ledger::crypto::MerkleTree;
use hyra_scribe_ledger::SimpleScribeLedger;

#[test]
fn test_compute_merkle_root_empty_ledger() {
    let ledger = SimpleScribeLedger::temp().unwrap();
    let root = ledger.compute_merkle_root().unwrap();
    assert_eq!(root, None);
}

#[test]
fn test_compute_merkle_root_single_key() {
    let ledger = SimpleScribeLedger::temp().unwrap();
    ledger.put("key1", "value1").unwrap();

    let root = ledger.compute_merkle_root().unwrap();
    assert!(root.is_some());
    assert_eq!(root.unwrap().len(), 32); // SHA-256 hash
}

#[test]
fn test_compute_merkle_root_multiple_keys() {
    let ledger = SimpleScribeLedger::temp().unwrap();
    ledger.put("key1", "value1").unwrap();
    ledger.put("key2", "value2").unwrap();
    ledger.put("key3", "value3").unwrap();

    let root = ledger.compute_merkle_root().unwrap();
    assert!(root.is_some());
    assert_eq!(root.unwrap().len(), 32); // SHA-256 hash
}

#[test]
fn test_compute_merkle_root_deterministic() {
    // Create two ledgers with same data
    let ledger1 = SimpleScribeLedger::temp().unwrap();
    ledger1.put("alice", "data1").unwrap();
    ledger1.put("bob", "data2").unwrap();

    let ledger2 = SimpleScribeLedger::temp().unwrap();
    ledger2.put("alice", "data1").unwrap();
    ledger2.put("bob", "data2").unwrap();

    let root1 = ledger1.compute_merkle_root().unwrap().unwrap();
    let root2 = ledger2.compute_merkle_root().unwrap().unwrap();

    assert_eq!(root1, root2);
}

#[test]
fn test_generate_merkle_proof_success() {
    let ledger = SimpleScribeLedger::temp().unwrap();
    ledger.put("key1", "value1").unwrap();
    ledger.put("key2", "value2").unwrap();

    let proof = ledger.generate_merkle_proof("key1").unwrap();
    assert!(proof.is_some());

    let proof = proof.unwrap();
    assert_eq!(proof.key, b"key1");
    assert_eq!(proof.value, b"value1");
}

#[test]
fn test_generate_merkle_proof_nonexistent_key() {
    let ledger = SimpleScribeLedger::temp().unwrap();
    ledger.put("key1", "value1").unwrap();

    let proof = ledger.generate_merkle_proof("nonexistent").unwrap();
    assert_eq!(proof, None);
}

#[test]
fn test_generate_merkle_proof_empty_ledger() {
    let ledger = SimpleScribeLedger::temp().unwrap();

    let proof = ledger.generate_merkle_proof("key1").unwrap();
    assert_eq!(proof, None);
}

#[test]
fn test_merkle_proof_verification() {
    let ledger = SimpleScribeLedger::temp().unwrap();
    ledger.put("alice", "data1").unwrap();
    ledger.put("bob", "data2").unwrap();
    ledger.put("charlie", "data3").unwrap();

    // Generate proof for alice
    let proof = ledger.generate_merkle_proof("alice").unwrap().unwrap();
    let root_hash = ledger.compute_merkle_root().unwrap().unwrap();

    // Verify the proof
    let verified = MerkleTree::verify_proof(&proof, &root_hash);
    assert!(verified);
}

#[test]
fn test_merkle_proof_verification_fails_wrong_root() {
    let ledger = SimpleScribeLedger::temp().unwrap();
    ledger.put("key1", "value1").unwrap();
    ledger.put("key2", "value2").unwrap();

    let proof = ledger.generate_merkle_proof("key1").unwrap().unwrap();

    // Use wrong root hash
    let wrong_root = vec![0u8; 32];
    let verified = MerkleTree::verify_proof(&proof, &wrong_root);
    assert!(!verified);
}

#[test]
fn test_merkle_proof_all_keys_verified() {
    let ledger = SimpleScribeLedger::temp().unwrap();

    // Insert multiple keys
    let keys = vec!["key1", "key2", "key3", "key4", "key5"];
    for key in &keys {
        ledger.put(*key, format!("value_{}", key)).unwrap();
    }

    let root_hash = ledger.compute_merkle_root().unwrap().unwrap();

    // Verify all keys can be proven
    for key in &keys {
        let proof = ledger.generate_merkle_proof(*key).unwrap().unwrap();
        let verified = MerkleTree::verify_proof(&proof, &root_hash);
        assert!(verified, "Failed to verify proof for key: {}", key);
    }
}

#[test]
fn test_get_all_keys() {
    let ledger = SimpleScribeLedger::temp().unwrap();
    ledger.put("key1", "value1").unwrap();
    ledger.put("key2", "value2").unwrap();
    ledger.put("key3", "value3").unwrap();

    let all_pairs = ledger.get_all().unwrap();
    assert_eq!(all_pairs.len(), 3);

    // Check that all keys are present
    let keys: Vec<Vec<u8>> = all_pairs.iter().map(|(k, _)| k.clone()).collect();
    assert!(keys.contains(&b"key1".to_vec()));
    assert!(keys.contains(&b"key2".to_vec()));
    assert!(keys.contains(&b"key3".to_vec()));
}

#[test]
fn test_verification_with_large_dataset() {
    let ledger = SimpleScribeLedger::temp().unwrap();

    // Insert 100 keys
    for i in 0..100 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }

    let root_hash = ledger.compute_merkle_root().unwrap().unwrap();

    // Verify a few random keys
    for i in [0, 25, 50, 75, 99] {
        let key = format!("key{}", i);
        let proof = ledger.generate_merkle_proof(&key).unwrap().unwrap();
        let verified = MerkleTree::verify_proof(&proof, &root_hash);
        assert!(verified, "Failed to verify proof for key: {}", key);
    }
}

#[test]
fn test_merkle_root_changes_on_update() {
    let ledger = SimpleScribeLedger::temp().unwrap();
    ledger.put("key1", "value1").unwrap();

    let root1 = ledger.compute_merkle_root().unwrap().unwrap();

    // Update the ledger
    ledger.put("key2", "value2").unwrap();

    let root2 = ledger.compute_merkle_root().unwrap().unwrap();

    // Root should change
    assert_ne!(root1, root2);
}

#[test]
fn test_proof_structure() {
    let ledger = SimpleScribeLedger::temp().unwrap();
    for i in 0..4 {
        ledger
            .put(format!("key{}", i), format!("value{}", i))
            .unwrap();
    }

    let proof = ledger.generate_merkle_proof("key0").unwrap().unwrap();

    // For 4 elements, we need log2(4) = 2 levels in the tree
    // So we should have siblings in the proof
    assert!(!proof.siblings.is_empty());
    assert_eq!(proof.siblings.len(), proof.directions.len());
}
