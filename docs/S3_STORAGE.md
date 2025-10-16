# S3 Storage Backend

This document describes the S3 storage backend implementation for Hyra Scribe Ledger (Task 6.1).

## Overview

The S3 storage backend provides object storage capabilities for archiving segments to S3-compatible storage (AWS S3 or MinIO). This enables multi-tier storage architecture where hot data lives in local Sled storage and cold data is archived to S3.

## Features

- **AWS S3 Integration**: Full support for AWS S3 using the official AWS SDK
- **MinIO Support**: Compatible with MinIO for local development and testing
- **Connection Pooling**: Automatic connection pooling via AWS SDK
- **Retry Logic**: Exponential backoff retry for transient failures
- **Error Handling**: Comprehensive error handling with proper error types
- **Async Operations**: All operations are async for non-blocking I/O

## Configuration

### Using Configuration File

Add S3 configuration to your TOML config file:

```toml
[storage.s3]
bucket = "my-scribe-bucket"
region = "us-east-1"
# Optional: For MinIO or custom S3 endpoints
endpoint = "http://localhost:9000"
access_key_id = "minioadmin"
secret_access_key = "minioadmin"
path_style = true
pool_size = 10
timeout_secs = 30
max_retries = 3
```

### MinIO Configuration (Local Development)

For local development with MinIO:

```toml
[storage.s3]
bucket = "test-bucket"
region = "us-east-1"
endpoint = "http://localhost:9000"
access_key_id = "minioadmin"
secret_access_key = "minioadmin"
path_style = true  # Required for MinIO
```

### AWS S3 Configuration

For production with AWS S3:

```toml
[storage.s3]
bucket = "production-scribe-bucket"
region = "us-west-2"
# Credentials can be omitted if using IAM roles
# access_key_id = "YOUR_ACCESS_KEY"
# secret_access_key = "YOUR_SECRET_KEY"
```

## Usage

### Initializing S3 Storage

```rust
use hyra_scribe_ledger::storage::s3::{S3Storage, S3StorageConfig};

// Create configuration
let config = S3StorageConfig {
    bucket: "my-bucket".to_string(),
    region: "us-east-1".to_string(),
    endpoint: Some("http://localhost:9000".to_string()),
    access_key_id: Some("minioadmin".to_string()),
    secret_access_key: Some("minioadmin".to_string()),
    path_style: true,
    timeout_secs: 30,
    max_retries: 3,
};

// Create S3 storage instance
let storage = S3Storage::new(config).await?;
```

### Storing Segments

```rust
use hyra_scribe_ledger::storage::segment::Segment;
use std::collections::HashMap;

// Create a segment
let mut data = HashMap::new();
data.insert(b"key1".to_vec(), b"value1".to_vec());
let segment = Segment::from_data(1, data);

// Store to S3
storage.put_segment(&segment).await?;
```

### Retrieving Segments

```rust
// Retrieve a segment by ID
let segment = storage.get_segment(1).await?;

match segment {
    Some(seg) => println!("Retrieved segment: {:?}", seg),
    None => println!("Segment not found"),
}
```

### Deleting Segments

```rust
// Delete a segment
storage.delete_segment(1).await?;
```

### Listing Segments

```rust
// List all segment IDs
let segment_ids = storage.list_segments().await?;
println!("Found {} segments", segment_ids.len());
```

### Health Check

```rust
// Check if S3 is accessible
storage.health_check().await?;
```

## Running Tests

### Unit Tests

Run the S3 storage unit tests:

```bash
cargo test --lib 'storage::s3::'
```

### Integration Tests

Integration tests require a running MinIO instance or AWS S3 credentials:

```bash
# Start MinIO (with Docker)
docker run -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address ":9001"

# Create test bucket
aws --endpoint-url http://localhost:9000 s3 mb s3://test-bucket

# Run integration tests
cargo test --test s3_storage_tests -- --ignored
```

### Benchmarks

Run S3 storage benchmarks:

```bash
# Set environment variables for S3
export S3_BUCKET=benchmark-bucket
export S3_ENDPOINT=http://localhost:9000
export S3_ACCESS_KEY_ID=minioadmin
export S3_SECRET_ACCESS_KEY=minioadmin
export S3_PATH_STYLE=true

# Run benchmarks
cargo bench --bench s3_storage_benchmark
```

## Performance Considerations

- **Segment Size**: Larger segments reduce overhead but increase latency
- **Connection Pooling**: Configured via `pool_size` in config
- **Retry Logic**: Automatic retries with exponential backoff (configurable)
- **Compression**: Not yet implemented (planned for Task 6.2)
- **Caching**: Consider implementing local cache for frequently accessed segments

## Architecture

```
┌─────────────────┐
│  Sled Storage   │  Hot data (local)
│  (In-Memory)    │
└────────┬────────┘
         │
         │ Segment flush
         ↓
┌─────────────────┐
│  Segment Mgr    │  Manages active/flushed segments
└────────┬────────┘
         │
         │ Archive
         ↓
┌─────────────────┐
│  S3 Storage     │  Cold data (S3/MinIO)
│  (Task 6.1)     │
└─────────────────┘
```

## Future Enhancements (Task 6.2 & 6.3)

- Automatic segment archival based on age/access patterns
- Compression for S3-stored segments
- Segment metadata storage in S3
- Data tiering policies
- Read-through caching

## Troubleshooting

### Connection Issues

If you get connection errors:

1. Check that MinIO/S3 is running and accessible
2. Verify credentials are correct
3. Check bucket exists
4. Ensure network connectivity

### Performance Issues

If operations are slow:

1. Check network latency to S3 endpoint
2. Increase connection pool size
3. Adjust timeout settings
4. Consider using S3 Transfer Acceleration (AWS only)

### Size Limitations

- AWS S3: 5TB per object (use multipart for >5GB)
- MinIO: Configurable, default is 5TB

## References

- [AWS SDK for Rust](https://docs.aws.amazon.com/sdk-for-rust/latest/dg/welcome.html)
- [MinIO Documentation](https://min.io/docs/minio/linux/index.html)
- [Task 6.1 in DEVELOPMENT.md](../DEVELOPMENT.md#task-61-s3-storage-backend)
