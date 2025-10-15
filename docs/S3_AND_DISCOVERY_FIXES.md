# S3 Storage and Discovery Port Fixes

This document describes the fixes applied to enable S3 storage configuration and resolve discovery port collisions.

## Issues Addressed

### 1. Discovery Port Collision
**Problem**: All three node configuration files (config-node1.toml, config-node2.toml, config-node3.toml) used the same discovery port (17946), which could lead to port conflicts when running multiple nodes on the same host.

**Solution**: Updated each node configuration to use unique discovery ports:
- Node 1: port 17946
- Node 2: port 17947  
- Node 3: port 17948

This ensures that each node can bind to its own UDP port for cluster discovery without conflicts.

### 2. S3 Storage Configuration
**Problem**: S3 storage was commented out in all configuration files, and the node startup did not initialize or log S3 storage configuration.

**Solution**: 
- Enabled S3 storage configuration in all config files (config.toml, config-node1.toml, config-node2.toml, config-node3.toml)
- Each node now has a unique S3 bucket name:
  - Node 1: scribe-ledger-node1
  - Node 2: scribe-ledger-node2
  - Node 3: scribe-ledger-node3
- Added S3 initialization logic in scribe-node.rs that:
  - Detects S3 configuration from config file
  - Logs all S3 configuration parameters on startup
  - Attempts to initialize S3 storage client
  - Gracefully handles S3 initialization failures (node continues with local storage only)

### 3. Unused Import Warning
**Problem**: The `tokio::signal` import in scribe-node.rs was unused at the top level (only used inside the `wait_for_shutdown_signal()` function).

**Solution**: Removed the unused import to eliminate compiler warnings.

## Configuration Example

### MinIO S3 Configuration (Local Development)

Each node is now configured with S3 storage enabled by default for MinIO:

```toml
[storage.s3]
bucket = "scribe-ledger-node1"  # Unique per node
region = "us-east-1"
endpoint = "http://localhost:9000"
access_key_id = "minioadmin"
secret_access_key = "minioadmin"
path_style = true
pool_size = 10
timeout_secs = 30
max_retries = 3
```

## Startup Logging

When S3 storage is configured, the node now logs detailed information on startup:

```
INFO scribe_node: S3 storage configuration detected
INFO scribe_node:   Bucket: scribe-ledger-node1
INFO scribe_node:   Region: us-east-1
INFO scribe_node:   Endpoint: http://localhost:9000
INFO scribe_node:   Path style: true
INFO scribe_node:   Pool size: 10
INFO scribe_node:   Timeout: 30s
INFO scribe_node:   Max retries: 3
INFO scribe_node: ✓ S3 storage initialized successfully
```

If S3 is not configured, you'll see:
```
INFO scribe_node: S3 storage not configured (running with local storage only)
```

If S3 initialization fails (e.g., MinIO not running):
```
WARN scribe_node: Failed to initialize S3 storage: <error details>
WARN scribe_node: Node will continue without S3 archival support
```

## Files Modified

1. **config.toml** - Enabled S3 configuration with default bucket name
2. **config-node1.toml** - Unique discovery port (17946) and S3 bucket (scribe-ledger-node1)
3. **config-node2.toml** - Unique discovery port (17947) and S3 bucket (scribe-ledger-node2)
4. **config-node3.toml** - Unique discovery port (17948) and S3 bucket (scribe-ledger-node3)
5. **src/bin/scribe-node.rs** - Added S3 initialization and logging, removed unused import

## Testing

All existing tests continue to pass:
- 252 library tests passed
- No build warnings
- Code formatted with `cargo fmt`

## Usage

### Starting a Node with S3 Storage

```bash
# Start node 1 with S3 storage enabled
cargo run --bin scribe-node -- -c config-node1.toml --bootstrap

# Start node 2 to join the cluster
cargo run --bin scribe-node -- -c config-node2.toml

# Start node 3 to join the cluster  
cargo run --bin scribe-node -- -c config-node3.toml
```

### Running with MinIO

To use S3 storage, ensure MinIO is running:

```bash
# Using Docker
docker run -p 9000:9000 -p 9001:9001 \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  minio/minio server /data --console-address ":9001"
```

### Disabling S3 Storage

To disable S3 storage, simply comment out or remove the `[storage.s3]` section from the config file:

```toml
# [storage.s3]
# bucket = "scribe-ledger-node1"
# ...
```

The node will log: `S3 storage not configured (running with local storage only)`

## Notes

- Each node must have a unique discovery port when running on the same host
- Each node should have a unique S3 bucket to avoid data conflicts
- S3 initialization is non-blocking - if it fails, the node continues without S3 support
- S3 storage is used for archival tiers when needed by the storage layer
