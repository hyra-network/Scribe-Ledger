use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use hyra_scribe_ledger::storage::s3::{S3Storage, S3StorageConfig};
use hyra_scribe_ledger::storage::segment::Segment;
use std::collections::HashMap;
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

fn bench_s3_put_segment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Check if S3 is available
    if get_test_config().bucket.is_empty() || !is_s3_available(&rt) {
        println!("Skipping S3 benchmarks - S3 not configured or not available");
        return;
    }

    let storage = rt.block_on(async { S3Storage::new(get_test_config()).await.unwrap() });

    let mut group = c.benchmark_group("s3_put_segment");

    for size_kb in [1, 10, 100, 1024].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            size_kb,
            |b, &size_kb| {
                b.iter(|| {
                    rt.block_on(async {
                        let segment = create_segment(black_box(1000), black_box(size_kb));
                        storage.put_segment(&segment).await.unwrap();
                    });
                });
            },
        );
    }

    group.finish();
}

fn bench_s3_get_segment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Check if S3 is available
    if get_test_config().bucket.is_empty() || !is_s3_available(&rt) {
        println!("Skipping S3 benchmarks - S3 not configured or not available");
        return;
    }

    let storage = rt.block_on(async { S3Storage::new(get_test_config()).await.unwrap() });

    let mut group = c.benchmark_group("s3_get_segment");

    for size_kb in [1, 10, 100, 1024].iter() {
        // Pre-populate segments
        let segment_id = 2000 + (*size_kb as u64);
        rt.block_on(async {
            let segment = create_segment(segment_id, *size_kb);
            storage.put_segment(&segment).await.unwrap();
        });

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size_kb)),
            size_kb,
            |b, &size_kb| {
                b.iter(|| {
                    rt.block_on(async {
                        let segment_id = 2000 + (size_kb as u64);
                        storage.get_segment(black_box(segment_id)).await.unwrap();
                    });
                });
            },
        );
    }

    group.finish();
}

fn bench_s3_delete_segment(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Check if S3 is available
    if get_test_config().bucket.is_empty() || !is_s3_available(&rt) {
        println!("Skipping S3 benchmarks - S3 not configured or not available");
        return;
    }

    let storage = rt.block_on(async { S3Storage::new(get_test_config()).await.unwrap() });

    let mut group = c.benchmark_group("s3_delete_segment");

    group.bench_function("delete_1KB", |b| {
        b.iter(|| {
            // Pre-populate and then delete
            rt.block_on(async {
                let segment = create_segment(black_box(3000), black_box(1));
                storage.put_segment(&segment).await.unwrap();
                storage.delete_segment(black_box(3000)).await.unwrap();
            });
        });
    });

    group.finish();
}

fn bench_s3_list_segments(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Check if S3 is available
    if get_test_config().bucket.is_empty() || !is_s3_available(&rt) {
        println!("Skipping S3 benchmarks - S3 not configured or not available");
        return;
    }

    let storage = rt.block_on(async { S3Storage::new(get_test_config()).await.unwrap() });

    // Pre-populate some segments
    rt.block_on(async {
        for i in 0..10 {
            let segment = create_segment(4000 + i, 1);
            storage.put_segment(&segment).await.unwrap();
        }
    });

    let mut group = c.benchmark_group("s3_list_segments");

    group.bench_function("list_10_segments", |b| {
        b.iter(|| {
            rt.block_on(async {
                storage.list_segments().await.unwrap();
            });
        });
    });

    group.finish();
}

/// Check if S3 is available
fn is_s3_available(rt: &Runtime) -> bool {
    rt.block_on(async {
        match S3Storage::new(get_test_config()).await {
            Ok(storage) => storage.health_check().await.is_ok(),
            Err(_) => false,
        }
    })
}

criterion_group!(
    benches,
    bench_s3_put_segment,
    bench_s3_get_segment,
    bench_s3_delete_segment,
    bench_s3_list_segments
);
criterion_main!(benches);
