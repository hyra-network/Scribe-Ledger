# Segment Archival and Data Tiering Documentation

This document describes the segment archival and data tiering implementation (Tasks 6.2 and 6.3).

## Overview

The archival system provides automatic segment archival to S3 with compression, read-through caching, and configurable tiering policies. This enables efficient multi-tier storage where hot data stays local and cold data is automatically archived to S3.

## Features

### Task 6.2: Segment Archival to S3

- **Automatic Archival**: Segments are automatically archived based on age thresholds
- **Compression**: Gzip compression with configurable levels (0-9)
- **Read-Through Caching**: Frequently accessed segments are cached locally
- **Metadata Storage**: Segment metadata stored alongside data in S3
- **Lifecycle Management**: Full CRUD operations for archived segments

### Task 6.3: Data Tiering

- **Age-Based Tiering**: Automatic archival based on segment age
- **Configurable Policies**: Customizable tiering thresholds and intervals
- **Background Archival**: Automatic background task for periodic archival
- **Cache Management**: LRU-style caching for hot segments
- **MinIO Compatibility**: Full support for local MinIO development

## Architecture

```
┌─────────────────┐
│ Segment Manager │  Active segments (local)
└────────┬────────┘
         │
         │ Age threshold exceeded
         ↓
┌─────────────────┐
│ Archival Manager│  Manages archival process
└────────┬────────┘
         │
         │ Compress & Archive
         ↓
┌─────────────────┐
│   S3 Storage    │  Cold segments (S3/MinIO)
│  + Metadata     │
└─────────────────┘
```

## Configuration

### Tiering Policy

```rust
use simple_scribe_ledger::storage::archival::TieringPolicy;

let policy = TieringPolicy {
    age_threshold_secs: 3600,           // Archive after 1 hour
    enable_compression: true,            // Enable gzip compression
    compression_level: 6,                // Compression level (0-9)
    enable_auto_archival: true,          // Enable background archival
    archival_check_interval_secs: 300,  // Check every 5 minutes
};
```

### Creating an Archival Manager

```rust
use simple_scribe_ledger::storage::archival::{ArchivalManager, TieringPolicy};
use simple_scribe_ledger::storage::s3::S3StorageConfig;
use simple_scribe_ledger::storage::segment::SegmentManager;
use std::sync::Arc;

// Configure S3
let s3_config = S3StorageConfig {
    bucket: "my-bucket".to_string(),
    region: "us-east-1".to_string(),
    endpoint: Some("http://localhost:9000".to_string()), // MinIO
    access_key_id: Some("minioadmin".to_string()),
    secret_access_key: Some("minioadmin".to_string()),
    path_style: true,
    timeout_secs: 30,
    max_retries: 3,
};

// Create segment manager
let segment_mgr = Arc::new(SegmentManager::new());

// Create tiering policy
let policy = TieringPolicy::default();

// Create archival manager
let manager = ArchivalManager::new(s3_config, segment_mgr, policy).await?;
```

## Usage

### Manual Archival

```rust
use simple_scribe_ledger::storage::segment::Segment;
use std::collections::HashMap;

// Create a segment
let mut data = HashMap::new();
data.insert(b"key1".to_vec(), b"value1".to_vec());
let segment = Segment::from_data(1, data);

// Archive to S3
let metadata = manager.archive_segment(&segment).await?;

println!("Archived segment {} ({} bytes -> {} bytes)",
    metadata.segment_id,
    metadata.original_size,
    metadata.compressed_size
);
```

### Automatic Archival

```rust
// Start background archival task
let handle = manager.start_auto_archival();

// The manager will now automatically archive old segments
// based on the configured age threshold

// To stop (optional)
handle.abort();
```

### Retrieving Archived Segments

```rust
// Retrieve a segment from S3 (with caching)
let segment = manager.retrieve_segment(1).await?;

match segment {
    Some(seg) => println!("Retrieved {} entries", seg.len()),
    None => println!("Segment not found"),
}
```

### Read-Through Access

```rust
// Access value from local or S3 automatically
let value = manager.get_value(1, b"key1").await?;

match value {
    Some(v) => println!("Value: {:?}", v),
    None => println!("Key not found"),
}
```

### Segment Lifecycle

```rust
// Archive old segments
let archived_ids = manager.archive_old_segments().await?;
println!("Archived {} segments", archived_ids.len());

// List all archived segments
let segment_ids = manager.list_archived_segments().await?;
println!("Total archived: {}", segment_ids.len());

// Get metadata
let metadata = manager.get_metadata(1).await?;
if let Some(meta) = metadata {
    println!("Segment created at: {}", meta.created_at);
    println!("Compression ratio: {:.2}%",
        100.0 * meta.compressed_size as f64 / meta.original_size as f64
    );
}

// Delete archived segment
manager.delete_archived_segment(1).await?;
```

## Compression

### How it Works

Segments are compressed using gzip before uploading to S3:

1. Segment is serialized to bytes
2. Bytes are compressed with gzip
3. Compressed data is uploaded to S3
4. Metadata records original and compressed sizes

On retrieval:

1. Compressed data is downloaded from S3
2. Data is decompressed with gzip
3. Segment is deserialized
4. Segment is cached for future access

### Compression Levels

- **0**: No compression (fastest)
- **1-3**: Fast compression (lower ratio)
- **4-6**: Balanced (default is 6)
- **7-9**: Best compression (slower)

### Performance Trade-offs

```rust
// Fast archival, larger storage
let mut policy = TieringPolicy::default();
policy.compression_level = 1;

// Best compression, slower archival
let mut policy = TieringPolicy::default();
policy.compression_level = 9;

// Disable compression entirely
let mut policy = TieringPolicy::default();
policy.enable_compression = false;
```

## Caching

The archival manager maintains two caches:

1. **Segment Cache**: Stores recently accessed segments
2. **Metadata Cache**: Stores segment metadata

Caching behavior:

- First access: Downloads from S3, caches locally
- Subsequent access: Returns from cache (instant)
- Cache invalidation: Automatic on delete operations

## Testing

### Unit Tests

```bash
# Test archival module
cargo test --lib 'storage::archival::'
```

### Integration Tests

```bash
# Start MinIO
docker run -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address ":9001"

# Create test bucket
aws --endpoint-url http://localhost:9000 s3 mb s3://test-bucket

# Run segment archival tests
cargo test --test segment_archival_tests -- --ignored

# Run data tiering tests
cargo test --test data_tiering_tests -- --ignored
```

### Test Coverage

**Segment Archival Tests (10 tests)**:
- Archive and retrieve
- Compression effectiveness
- No compression option
- Metadata storage
- List archived segments
- Delete segments
- Read-through cache
- Large segment handling
- Tiering policy defaults
- Metadata serialization

**Data Tiering Tests (12 tests)**:
- MinIO compatibility
- Age-based tiering
- Compression levels
- Error recovery
- Concurrent archival
- Large number of segments
- Lifecycle management
- Cache invalidation
- Different data types
- Path-style addressing
- Policy validation

## Performance

### Compression Ratios

Typical compression ratios:

- Text data: 70-90% reduction
- Repetitive data: 90-95% reduction
- Binary data: 10-30% reduction
- Already compressed: 0-5% reduction

### Archival Performance

Factors affecting performance:

1. **Compression Level**: Higher = slower but smaller
2. **Network Latency**: S3 upload/download speed
3. **Segment Size**: Larger segments = better throughput
4. **Concurrent Operations**: Parallel archival supported

### Optimization Tips

```rust
// For high throughput
let mut policy = TieringPolicy::default();
policy.compression_level = 1;  // Fast compression
policy.age_threshold_secs = 7200;  // Archive less frequently

// For storage efficiency
let mut policy = TieringPolicy::default();
policy.compression_level = 9;  // Best compression
policy.age_threshold_secs = 1800;  // Archive more frequently
```

## Troubleshooting

### Archival Failures

```rust
match manager.archive_segment(&segment).await {
    Ok(metadata) => println!("Archived successfully"),
    Err(e) => eprintln!("Archival failed: {}", e),
}
```

Common issues:

1. **S3 connection error**: Check endpoint and credentials
2. **Bucket not found**: Ensure bucket exists
3. **Permission denied**: Verify IAM permissions
4. **Network timeout**: Increase timeout_secs in config

### Cache Issues

If segments aren't caching:

1. Check memory limits
2. Monitor cache hit rate
3. Consider cache size configuration

### Compression Issues

If compression isn't effective:

1. Check data type (already compressed?)
2. Try different compression levels
3. Consider disabling for certain data types

## Best Practices

### 1. Tiering Policy

```rust
// Production recommendation
let policy = TieringPolicy {
    age_threshold_secs: 3600,     // 1 hour
    enable_compression: true,
    compression_level: 6,          // Balanced
    enable_auto_archival: true,
    archival_check_interval_secs: 300,  // 5 minutes
};
```

### 2. Error Handling

Always handle archival errors:

```rust
if let Err(e) = manager.archive_segment(&segment).await {
    // Log error
    eprintln!("Failed to archive segment {}: {}", segment.segment_id, e);
    
    // Retry or handle gracefully
    // Don't delete local copy until successfully archived
}
```

### 3. Background Archival

Use background archival for production:

```rust
// Start background task
let handle = manager.start_auto_archival();

// Keep handle alive for the lifetime of your application
// The task will run until the handle is dropped or aborted
```

### 4. Monitoring

Track archival metrics:

```rust
// Periodically check archival status
let archived = manager.list_archived_segments().await?;
println!("Total archived segments: {}", archived.len());

// Monitor compression effectiveness
for id in archived {
    if let Some(meta) = manager.get_metadata(id).await? {
        let ratio = meta.compressed_size as f64 / meta.original_size as f64;
        println!("Segment {}: {:.1}% compression", id, (1.0 - ratio) * 100.0);
    }
}
```

## Integration with Existing Code

### With Segment Manager

```rust
use simple_scribe_ledger::storage::segment::SegmentManager;
use simple_scribe_ledger::storage::archival::ArchivalManager;

let segment_mgr = Arc::new(SegmentManager::new());

// Use segment manager for local operations
segment_mgr.put(key, value)?;

// Periodically archive old segments
let manager = ArchivalManager::new(s3_config, segment_mgr.clone(), policy).await?;
manager.archive_old_segments().await?;
```

### With S3 Storage

The archival manager uses S3Storage internally but provides higher-level operations:

```rust
// Low-level S3 operations
s3_storage.put_segment(&segment).await?;

// High-level archival operations (preferred)
manager.archive_segment(&segment).await?;  // Includes compression, metadata, caching
```

## Future Enhancements

Potential improvements for future versions:

1. **LRU Cache Eviction**: Configurable cache size with LRU eviction
2. **Parallel Compression**: Multi-threaded compression for large segments
3. **Incremental Archival**: Only archive changed portions
4. **Access Pattern Analysis**: Smarter tiering based on access frequency
5. **Multi-Region Support**: Replicate to multiple S3 regions
6. **Encryption**: Encrypt segments before uploading

## References

- [Task 6.2 in DEVELOPMENT.md](../DEVELOPMENT.md#task-62-segment-archival-to-s3)
- [Task 6.3 in DEVELOPMENT.md](../DEVELOPMENT.md#task-63-data-tiering-and-s3-tests)
- [S3 Storage Documentation](S3_STORAGE.md)
- [flate2 crate](https://docs.rs/flate2/)
