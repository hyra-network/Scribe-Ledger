use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use hyra_scribe_ledger::storage::archival::{ArchivalManager, TieringPolicy};
use hyra_scribe_ledger::storage::s3::S3StorageConfig;
use hyra_scribe_ledger::storage::segment::{Segment, SegmentManager};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Get test S3 configuration for MinIO
fn get_test_config() -> S3StorageConfig {
    S3StorageConfig {
        bucket: std::env::var("S3_BUCKET").unwrap_or_else(|_| "benchmark-bucket".to_string()),
        region: std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        endpoint: std::env::var("S3_ENDPOINT").ok(),
        access_key_id: std::env::var("S3_ACCESS_KEY_ID").ok(),
        secret_access_key: std::env::var("S3_SECRET_ACCESS_KEY").ok(),
        path_style: std::env::var("S3_PATH_STYLE").is_ok(),
        timeout_secs: 30,
        max_retries: 3,
    }
}

fn create_segment(segment_id: u64, size_kb: usize) -> Segment {
    let mut data = HashMap::new();
    let value_size = 1024; // 1KB per value
    let num_entries = size_kb;

    for i in 0..num_entries {
        let key = format!("key_{}", i).into_bytes();
        let value = vec![0u8; value_size];
        data.insert(key, value);
    }

    Segment::from_data(segment_id, data)
}

fn create_compressible_segment(segment_id: u64, size_kb: usize) -> Segment {
    let mut data = HashMap::new();
    let value_size = 1024;
    let num_entries = size_kb;

    for i in 0..num_entries {
        let key = format!("key_{}", i).into_bytes();
        let value = vec![b'A'; value_size]; // Highly compressible
        data.insert(key, value);
    }

    Segment::from_data(segment_id, data)
}

/// Check if S3 is available
fn is_s3_available(rt: &Runtime) -> bool {
    let config = get_test_config();
    if config.bucket.is_empty() {
        return false;
    }

    rt.block_on(async {
        ArchivalManager::new(
            config,
            Arc::new(SegmentManager::new()),
            TieringPolicy::default(),
        )
        .await
        .is_ok()
    })
}

fn bench_compression_levels(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Check if S3 is available
    if get_test_config().bucket.is_empty() || !is_s3_available(&rt) {
        println!("Skipping archival benchmarks - S3 not configured or not available");
        return;
    }

    let mut group = c.benchmark_group("compression_levels");

    for level in [0, 1, 6, 9].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("level_{}", level)),
            level,
            |b, &level| {
                b.iter(|| {
                    rt.block_on(async {
                        let policy = TieringPolicy {
                            compression_level: level,
                            enable_compression: true,
                            ..Default::default()
                        };

                        let manager = ArchivalManager::new(
                            get_test_config(),
                            Arc::new(SegmentManager::new()),
                            policy,
                        )
                        .await
                        .unwrap();

                        let segment = create_compressible_segment(
                            black_box(10000 + level as u64),
                            black_box(100),
                        );
                        manager.archive_segment(&segment).await.unwrap();
                    });
                });
            },
        );
    }

    group.finish();
}

fn bench_archive_segment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    if get_test_config().bucket.is_empty() || !is_s3_available(&rt) {
        println!("Skipping archival benchmarks - S3 not configured or not available");
        return;
    }

    let manager = rt.block_on(async {
        ArchivalManager::new(
            get_test_config(),
            Arc::new(SegmentManager::new()),
            TieringPolicy::default(),
        )
        .await
        .unwrap()
    });

    let mut group = c.benchmark_group("archive_segment");

    for size_kb in [1, 10, 100, 1024].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            size_kb,
            |b, &size_kb| {
                b.iter(|| {
                    rt.block_on(async {
                        let segment = create_segment(black_box(20000), black_box(size_kb));
                        manager.archive_segment(&segment).await.unwrap();
                    });
                });
            },
        );
    }

    group.finish();
}

fn bench_retrieve_segment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    if get_test_config().bucket.is_empty() || !is_s3_available(&rt) {
        println!("Skipping archival benchmarks - S3 not configured or not available");
        return;
    }

    let manager = rt.block_on(async {
        ArchivalManager::new(
            get_test_config(),
            Arc::new(SegmentManager::new()),
            TieringPolicy::default(),
        )
        .await
        .unwrap()
    });

    let mut group = c.benchmark_group("retrieve_segment");

    for size_kb in [1, 10, 100, 1024].iter() {
        // Pre-archive segments
        let segment_id = 30000 + (*size_kb as u64);
        rt.block_on(async {
            let segment = create_segment(segment_id, *size_kb);
            manager.archive_segment(&segment).await.unwrap();
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            size_kb,
            |b, &size_kb| {
                b.iter(|| {
                    rt.block_on(async {
                        let segment_id = 30000 + (size_kb as u64);
                        manager
                            .retrieve_segment(black_box(segment_id))
                            .await
                            .unwrap();
                    });
                });
            },
        );
    }

    group.finish();
}

fn bench_retrieve_cached_segment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    if get_test_config().bucket.is_empty() || !is_s3_available(&rt) {
        println!("Skipping archival benchmarks - S3 not configured or not available");
        return;
    }

    let manager = rt.block_on(async {
        ArchivalManager::new(
            get_test_config(),
            Arc::new(SegmentManager::new()),
            TieringPolicy::default(),
        )
        .await
        .unwrap()
    });

    let mut group = c.benchmark_group("retrieve_cached_segment");

    // Pre-archive and cache a segment
    let segment_id = 40000;
    rt.block_on(async {
        let segment = create_segment(segment_id, 100);
        manager.archive_segment(&segment).await.unwrap();
        // First retrieval to populate cache
        manager.retrieve_segment(segment_id).await.unwrap();
    });

    group.bench_function("100KB_cached", |b| {
        b.iter(|| {
            rt.block_on(async {
                manager
                    .retrieve_segment(black_box(segment_id))
                    .await
                    .unwrap();
            });
        });
    });

    group.finish();
}

fn bench_metadata_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    if get_test_config().bucket.is_empty() || !is_s3_available(&rt) {
        println!("Skipping archival benchmarks - S3 not configured or not available");
        return;
    }

    let manager = rt.block_on(async {
        ArchivalManager::new(
            get_test_config(),
            Arc::new(SegmentManager::new()),
            TieringPolicy::default(),
        )
        .await
        .unwrap()
    });

    let mut group = c.benchmark_group("metadata_operations");

    // Pre-archive segment
    let segment_id = 50000;
    rt.block_on(async {
        let segment = create_segment(segment_id, 10);
        manager.archive_segment(&segment).await.unwrap();
    });

    group.bench_function("get_metadata", |b| {
        b.iter(|| {
            rt.block_on(async {
                manager.get_metadata(black_box(segment_id)).await.unwrap();
            });
        });
    });

    group.finish();
}

fn bench_list_archived_segments(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    if get_test_config().bucket.is_empty() || !is_s3_available(&rt) {
        println!("Skipping archival benchmarks - S3 not configured or not available");
        return;
    }

    let manager = rt.block_on(async {
        ArchivalManager::new(
            get_test_config(),
            Arc::new(SegmentManager::new()),
            TieringPolicy::default(),
        )
        .await
        .unwrap()
    });

    // Pre-archive multiple segments
    rt.block_on(async {
        for i in 60000..60010 {
            let segment = create_segment(i, 1);
            manager.archive_segment(&segment).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("list_operations");

    group.bench_function("list_10_segments", |b| {
        b.iter(|| {
            rt.block_on(async {
                manager.list_archived_segments().await.unwrap();
            });
        });
    });

    group.finish();
}

fn bench_compression_ratio(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    if get_test_config().bucket.is_empty() || !is_s3_available(&rt) {
        println!("Skipping archival benchmarks - S3 not configured or not available");
        return;
    }

    let mut group = c.benchmark_group("compression_ratio");

    // Benchmark compressible data
    group.bench_function("compressible_data_100KB", |b| {
        b.iter(|| {
            rt.block_on(async {
                let manager = ArchivalManager::new(
                    get_test_config(),
                    Arc::new(SegmentManager::new()),
                    TieringPolicy::default(),
                )
                .await
                .unwrap();

                let segment = create_compressible_segment(black_box(70000), black_box(100));
                manager.archive_segment(&segment).await.unwrap();
            });
        });
    });

    // Benchmark random data (less compressible)
    group.bench_function("random_data_100KB", |b| {
        b.iter(|| {
            rt.block_on(async {
                let manager = ArchivalManager::new(
                    get_test_config(),
                    Arc::new(SegmentManager::new()),
                    TieringPolicy::default(),
                )
                .await
                .unwrap();

                let segment = create_segment(black_box(80000), black_box(100));
                manager.archive_segment(&segment).await.unwrap();
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_compression_levels,
    bench_archive_segment,
    bench_retrieve_segment,
    bench_retrieve_cached_segment,
    bench_metadata_operations,
    bench_list_archived_segments,
    bench_compression_ratio
);
criterion_main!(benches);
