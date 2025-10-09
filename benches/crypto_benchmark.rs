//! Benchmark for Merkle tree cryptographic operations
//!
//! This benchmark measures the performance of:
//! - Tree construction from various sizes of data
//! - Proof generation
//! - Proof verification

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use hyra_scribe_ledger::crypto::MerkleTree;

fn merkle_tree_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_tree_construction");

    for size in [10, 100, 1000, 10000] {
        let pairs: Vec<_> = (0..size)
            .map(|i| {
                (
                    format!("key{}", i).into_bytes(),
                    format!("value{}", i).into_bytes(),
                )
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), &pairs, |b, pairs| {
            b.iter(|| {
                let tree = MerkleTree::from_pairs(black_box(pairs.clone()));
                black_box(tree);
            });
        });
    }

    group.finish();
}

fn merkle_proof_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_proof_generation");

    for size in [10, 100, 1000, 10000] {
        let pairs: Vec<_> = (0..size)
            .map(|i| {
                (
                    format!("key{}", i).into_bytes(),
                    format!("value{}", i).into_bytes(),
                )
            })
            .collect();

        let tree = MerkleTree::from_pairs(pairs);
        // Use a key that exists in all sizes (first key)
        let key = b"key0";

        group.bench_with_input(BenchmarkId::from_parameter(size), &tree, |b, tree| {
            b.iter(|| {
                let proof = tree.get_proof(black_box(key));
                black_box(proof);
            });
        });
    }

    group.finish();
}

fn merkle_proof_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_proof_verification");

    for size in [10, 100, 1000, 10000] {
        let pairs: Vec<_> = (0..size)
            .map(|i| {
                (
                    format!("key{}", i).into_bytes(),
                    format!("value{}", i).into_bytes(),
                )
            })
            .collect();

        let tree = MerkleTree::from_pairs(pairs);
        // Use a key that exists in all sizes (first key)
        let key = b"key0";
        let proof = tree.get_proof(key).unwrap();
        let root_hash = tree.root_hash().unwrap();

        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &(proof, root_hash),
            |b, (proof, root_hash)| {
                b.iter(|| {
                    let result = MerkleTree::verify_proof(black_box(proof), black_box(root_hash));
                    black_box(result);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    merkle_tree_construction,
    merkle_proof_generation,
    merkle_proof_verification
);
criterion_main!(benches);
