# Hyra Scribe Ledger

> **A distributed, immutable, append-only key-value storage system**

Hyra Scribe Ledger is a production-ready distributed storage system built with Rust, designed for durability, consistency, and high performance. It combines local storage with S3-compatible object storage to provide a robust multi-tier architecture for persistent data management.

## Overview

Hyra Scribe Ledger provides a complete distributed storage solution with:

- **Immutability:** Write-once, read-forever append-only storage
- **Durability:** Multi-tier architecture with local caching and S3 persistence
- **Distribution:** OpenRaft-based consensus for cluster coordination
- **Performance:** Async I/O with Tokio, hot data caching, and optimized batching
- **Verifiability:** Cryptographic proofs for data integrity
- **Optimization:** Tunable Raft parameters, bincode serialization, and LRU caching

## Architecture

The system uses a multi-tiered architecture combining local and cloud storage:

### Storage Tiers

**Local Tier (Hot Data)**
- Sled embedded database for fast local access
- Optimized for high-throughput writes and low-latency reads
- Configurable cache size (default: 256MB)
- LRU cache layer for frequently accessed data (Task 11.3)
- Bincode serialization for internal operations

**S3 Tier (Cold Data)**
- S3-compatible object storage for long-term persistence
- Automatic segment archival with compression
- Age-based tiering policies
- MinIO support for local development

### Distributed Consensus

**OpenRaft Cluster**
- Modern async-first Raft implementation
- Automatic leader election and failover
- Strong consistency guarantees
- Dynamic membership management

**Node Discovery**
- Automatic cluster formation
- Health monitoring and failure detection
- Peer-to-peer discovery protocol

### Data Flow

**Write Path:**
1. Client sends data to any cluster node via HTTP API
2. Request forwarded to leader if necessary
3. Leader proposes write via Raft consensus
4. Data replicated to quorum of nodes
5. Applied to local storage on all nodes
6. Success returned to client

**Read Path:**
1. Check hot data cache (LRU) for immediate response
2. Request served from local storage if not cached
3. Falls back to S3 if not in local storage
4. Transparent read-through caching with automatic cache updates
5. Consistent reads across the cluster

## Technology Stack

| Technology      | Purpose                                                              |
|-----------------|----------------------------------------------------------------------|
| **Rust**        | Core language for memory safety and performance                     |
| **Tokio**       | Async runtime for high-concurrency I/O                              |
| **Sled**        | Embedded database for local storage tier                            |
| **OpenRaft**    | Modern async Raft consensus implementation                          |
| **AWS SDK**     | S3-compatible object storage integration                            |
| **Axum**        | High-performance HTTP framework                                     |
| **Serde**       | Serialization and deserialization                                   |
| **Bincode**     | Fast binary serialization for internal operations                   |
| **LRU Cache**   | Hot data caching for performance optimization                       |

## Quick Start

> **Windows Users**: See [Windows Setup Guide](docs/WINDOWS.md) for detailed Windows-specific instructions.

### Prerequisites

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs/)
- **Git** - For cloning the repository

### Installation

```bash
# Clone the repository
git clone https://github.com/amogusdrip285/simple-scribe-ledger.git
cd simple-scribe-ledger

# Build the project
cargo build --release

# Run tests
cargo test
```

### Running a Single Node

```bash
# Start a node with default configuration
cargo run --bin scribe-node

# Or use a custom configuration file
cargo run --bin scribe-node -- --config config.toml
```

The HTTP API will be available at `http://localhost:8001`.

### Running a Multi-Node Cluster

Start a 3-node cluster by running each node in a separate terminal:

```bash
# Terminal 1 - Node 1 (Leader)
cargo run --bin scribe-node -- --config config-node1.toml

# Terminal 2 - Node 2 (Follower)
cargo run --bin scribe-node -- --config config-node2.toml

# Terminal 3 - Node 3 (Follower)
cargo run --bin scribe-node -- --config config-node3.toml
```

Alternatively, use the provided cluster management scripts:

```bash
./scripts/start-cluster.sh   # Start 3-node cluster
./scripts/test-cluster.sh    # Run cluster tests
./scripts/stop-cluster.sh    # Stop the cluster
```

## Configuration

### Basic Configuration

Create a `config.toml` file:

```toml
[node]
id = 1
address = "127.0.0.1:8001"
data_dir = "./data"

[network]
listen_addr = "127.0.0.1"
client_port = 8001          # HTTP API port
raft_tcp_port = 9001        # Raft consensus port

[storage]
segment_size = 1048576      # 1MB segments
max_cache_size = 268435456  # 256MB cache

[consensus]
election_timeout = 10
heartbeat_timeout = 3
```

### Environment Variables

Override configuration using environment variables with the `SCRIBE_` prefix:

```bash
export SCRIBE_NODE_ID=2
export SCRIBE_NETWORK_CLIENT_PORT=8002
export SCRIBE_NETWORK_RAFT_TCP_PORT=9002

cargo run --bin scribe-node
```

**Configuration Demo:**
```bash
cargo run --example config_demo
```

This demonstrates:
- Loading TOML configuration files
- Environment variable overrides
- Configuration validation
- Error handling

### Multi-Node Configuration

For cluster deployments, configure each node with unique ports and IDs:

- **Node 1**: `config-node1.toml` - HTTP: 8001, Raft: 9001
- **Node 2**: `config-node2.toml` - HTTP: 8002, Raft: 9002  
- **Node 3**: `config-node3.toml` - HTTP: 8003, Raft: 9003

## HTTP API

> **Note**: The examples below use port 8001 which is the default for scribe-node. Adjust the port based on your configuration.

### Monitoring & Metrics

**Health Check**
```bash
curl http://localhost:8001/health
```

**Legacy Metrics (JSON)**
```bash
curl http://localhost:8001/metrics
```

Returns JSON with:
- Total keys in storage
- Total GET/PUT/DELETE requests
- Storage status

**Prometheus Metrics**
```bash
curl http://localhost:8001/metrics/prometheus
```

Returns Prometheus-formatted metrics including:
- Request latency histograms (p50, p95, p99)
- Request counters (GET, PUT, DELETE)
- Throughput metrics (operations/sec)
- Storage metrics (keys, size)
- Raft consensus metrics (term, commit index, last applied)
- Node health status
- Error counters

**Example Prometheus configuration:**
```yaml
scrape_configs:
  - job_name: 'scribe-ledger'
    static_configs:
      - targets: ['localhost:8001', 'localhost:8002', 'localhost:8003']
    metrics_path: '/metrics/prometheus'
    scrape_interval: 15s
```

### Data Operations

**Store Data (PUT)**
```bash
curl -X PUT http://localhost:8001/my-key \
  -H "Content-Type: application/octet-stream" \
  --data-binary "my value data"
```

**Retrieve Data (GET)**
```bash
curl http://localhost:8001/my-key
```

**Delete Data (DELETE)**
```bash
curl -X DELETE http://localhost:8001/my-key
```

### Cluster Management

**Check Health**
```bash
curl http://localhost:8001/health
```

**View Metrics**
```bash
curl http://localhost:8001/metrics
```

**Cluster Status**
```bash
curl http://localhost:8001/cluster/info
```

**List Members**
```bash
curl http://localhost:8001/cluster/nodes
```

**Get Leader**
```bash
curl http://localhost:8001/cluster/leader/info
```

## Storage Backend

### Local Storage with Sled

The embedded Sled database provides high-performance local storage:

```rust
use hyra_scribe_ledger::SimpleScribeLedger;

fn main() -> anyhow::Result<()> {
    // Create a new storage instance
    let ledger = SimpleScribeLedger::new("./my_data")?;
    
    // Store data
    ledger.put("user:alice", "Alice Smith")?;
    ledger.put("user:bob", "Bob Johnson")?;
    
    // Retrieve data
    if let Some(value) = ledger.get("user:alice")? {
        println!("Found: {}", String::from_utf8_lossy(&value));
    }
    
    // Flush to disk
    ledger.flush()?;
    
    Ok(())
}
```

**Run the basic example:**
```bash
cargo run --example basic_usage
```

**Interactive CLI:**
```bash
cargo run --example cli_store
```

The CLI provides an interactive shell with commands:
- `put <key> <value>` - Store a key-value pair
- `get <key>` - Retrieve a value
- `list` - Show number of stored keys
- `clear` - Remove all data
- `quit` - Exit

### Segment-Based Storage

Data is organized into segments for efficient management:

- Segments group related key-value pairs
- Automatic segment creation based on size thresholds
- Serialization support for persistence
- Foundation for S3 archival

**Test coverage:**
```bash
# Storage layer tests
cargo test storage_tests

# Sled engine tests
cargo test sled_engine_tests

# Segment tests
cargo test segment
```

## S3 Cold Storage

### Setup with MinIO (Local Development)

```bash
# Start MinIO
docker run -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address ":9001"

# Create bucket
aws --endpoint-url http://localhost:9000 s3 mb s3://my-bucket
```

### S3 Configuration

Add S3 settings to your `config.toml`:

```toml
[storage.s3]
bucket = "my-bucket"
region = "us-east-1"
endpoint = "http://localhost:9000"    # MinIO
access_key_id = "minioadmin"
secret_access_key = "minioadmin"
path_style = true                      # Required for MinIO
```

### Data Tiering

Configure automatic archival to S3:

```toml
[storage.tiering]
age_threshold_secs = 3600              # Archive after 1 hour
enable_compression = true
compression_level = 6                  # 0-9 (balanced)
enable_auto_archival = true
archival_check_interval_secs = 300    # Check every 5 minutes
```

**Features:**
- Automatic age-based archival
- Gzip compression (configurable levels)
- Read-through caching
- Metadata storage in S3
- Full lifecycle management

**Documentation:**
- [S3 Storage Guide](docs/S3_STORAGE.md)
- [Archival & Tiering Guide](docs/ARCHIVAL_TIERING.md)

**Test coverage:**
```bash
# S3 integration tests (requires MinIO)
cargo test s3_storage_tests -- --ignored

# Archival tests
cargo test segment_archival -- --ignored

# Tiering tests
cargo test data_tiering -- --ignored
```

## Cryptographic Verification

### Merkle Tree Implementation

The ledger includes cryptographic verification through Merkle trees, providing tamper-proof data integrity guarantees.

**Features:**
- SHA-256 based Merkle tree construction
- Cryptographic proof generation for any key-value pair
- Proof verification against root hash
- Deterministic tree construction (consistent across nodes)
- Efficient proof size (logarithmic in tree size)

**Usage Example:**

```rust
use hyra_scribe_ledger::crypto::MerkleTree;

// Build Merkle tree from key-value pairs
let pairs = vec![
    (b"key1".to_vec(), b"value1".to_vec()),
    (b"key2".to_vec(), b"value2".to_vec()),
    (b"key3".to_vec(), b"value3".to_vec()),
];

let tree = MerkleTree::from_pairs(pairs);

// Get the root hash for verification
let root_hash = tree.root_hash().unwrap();

// Generate proof for a specific key
let proof = tree.get_proof(b"key2").unwrap();

// Verify the proof
assert!(MerkleTree::verify_proof(&proof, &root_hash));
```

**Integration with Manifest:**

Each segment's Merkle root is stored in the manifest for verification:

```rust
use hyra_scribe_ledger::manifest::ManifestEntry;

// Create manifest entry with Merkle root
let entry = ManifestEntry::new(
    segment_id,
    timestamp,
    root_hash,  // Merkle root from tree.root_hash()
    size
);
```

**HTTP Verification Endpoint:**

The HTTP server provides a verification endpoint to check data integrity:

```bash
# Store data
curl -X PUT http://localhost:3000/test \
  -H 'Content-Type: application/json' \
  -d '{"value": "hello world"}'

# Verify the key with Merkle proof
curl http://localhost:3000/verify/test
```

**Response:**
```json
{
  "key": "test",
  "verified": true,
  "proof": {
    "root_hash": "a1b2c3d4e5f6...",
    "siblings": ["e5f6g7h8...", "i9j0k1l2..."]
  },
  "error": null
}
```

The verification endpoint:
- Generates a Merkle proof for the requested key
- Verifies the proof against the current root hash
- Returns hex-encoded proof data for inspection
- Provides cryptographic guarantees of data integrity

**Ledger Verification Methods:**

```rust
use hyra_scribe_ledger::SimpleScribeLedger;

let ledger = SimpleScribeLedger::temp()?;
ledger.put("alice", "data1")?;
ledger.put("bob", "data2")?;

// Compute Merkle root of all data
let root_hash = ledger.compute_merkle_root()?.unwrap();

// Generate proof for specific key
let proof = ledger.generate_merkle_proof("alice")?.unwrap();

// Verify the proof
let verified = MerkleTree::verify_proof(&proof, &root_hash);
assert!(verified);
```

**Test coverage:**
```bash
# Crypto module tests
cargo test --lib crypto::

# Comprehensive crypto tests
cargo test crypto_tests

# Verification endpoint tests
cargo test verification_tests
```

## Distributed Consensus

### OpenRaft Integration

The cluster uses OpenRaft for distributed coordination:

- **Leader Election:** Automatic leader selection
- **Log Replication:** Strong consistency across nodes
- **Membership Changes:** Dynamic join/leave operations
- **Failure Recovery:** Automatic failover on node failures

**Test coverage:**
```bash
# Consensus layer tests
cargo test consensus

# Multi-node cluster tests
cargo test cluster
```

### Node Discovery

Automatic cluster formation with discovery service:

- UDP broadcast for peer discovery
- Heartbeat protocol for health monitoring
- Failure detection and recovery
- Dynamic peer list management

```bash
# Discovery service tests
cargo test discovery
```

### Data Replication

**Write Replication:**
1. Client writes to any node
2. Leader receives request (forwarded if needed)
3. Leader proposes via Raft
4. Data replicated to quorum
5. Applied on all nodes
6. Success returned to client

**Read Consistency:**
- Reads served from local storage (low latency)
- Eventual consistency across cluster
- Fallback to S3 for cold data
- Transparent data recovery

**Example across cluster:**
```bash
# Start cluster
./scripts/start-cluster.sh

# Write to different nodes (all forwarded to leader)
curl -X PUT http://localhost:8001/key1 --data "value1"
curl -X PUT http://localhost:8002/key2 --data "value2"
curl -X PUT http://localhost:8003/key3 --data "value3"

# Read from any node (data is replicated)
curl http://localhost:8001/key1
curl http://localhost:8002/key2
curl http://localhost:8003/key3
```

**Test coverage:**
```bash
# Write path tests
cargo test write_request

# Read path tests
cargo test read_request

# Data consistency tests
cargo test consistency
```

## Security Features

> **Note**: The security module provides TLS, authentication, rate limiting, and audit logging components. These features are available as library modules but are not yet integrated into the HTTP server or scribe-node binaries. Integration requires additional implementation work to wire these components into the server middleware and configuration system.

### TLS Encryption

TLS configuration module for secure node-to-node communication:

**Configuration:**
```toml
[security.tls]
enabled = true
cert_path = "/path/to/cert.pem"
key_path = "/path/to/key.pem"

# Optional: Mutual TLS
ca_cert_path = "/path/to/ca.pem"
require_client_cert = true
```

**Generating Certificates:**
```bash
# Generate self-signed certificate for development
openssl req -x509 -newkey rsa:4096 \
  -keyout key.pem -out cert.pem \
  -days 365 -nodes \
  -subj "/CN=localhost"

# For production, use certificates from a trusted CA
```

**Rust Configuration:**
```rust
use hyra_scribe_ledger::security::TlsConfig;

let tls_config = TlsConfig::new(
    PathBuf::from("/path/to/cert.pem"),
    PathBuf::from("/path/to/key.pem")
);

// Enable mutual TLS
let mutual_tls = tls_config.with_mutual_tls(
    PathBuf::from("/path/to/ca.pem")
);
```

### Authentication

Authentication module with API key and bearer token support:

> **Note**: Authentication is available as a module but requires integration into the HTTP server to be functional.

**Configuration:**
```rust
use hyra_scribe_ledger::security::{AuthConfig, Role};

let mut auth_config = AuthConfig::new(true);

// Add API keys with roles
auth_config.add_api_key("admin-key".to_string(), Role::admin());
auth_config.add_api_key("read-key".to_string(), Role::read_only());
auth_config.add_api_key("write-key".to_string(), Role::read_write());
```

**Role-Based Access Control (RBAC):**

| Role | Read | Write | Delete | Admin (Cluster/Metrics) |
|------|------|-------|--------|------------------------|
| **Read-only** | ✓ | ✗ | ✗ | ✗ |
| **Read-write** | ✓ | ✓ | ✗ | ✗ |
| **Admin** | ✓ | ✓ | ✓ | ✓ |



### Rate Limiting

Token bucket rate limiting module for request throttling:

**Configuration:**
```rust
use hyra_scribe_ledger::security::RateLimiterConfig;

// 100 requests per minute per client, with burst of 10
let rate_config = RateLimiterConfig::new(100, 60)
    .with_burst_size(10);
```

> **Note**: Rate limiting requires integration into HTTP server middleware to be functional.

### Audit Logging

Structured audit logging module for security events:

**Audit Events:**
- Authentication attempts (success/failure)
- Authorization decisions (granted/denied)
- Data access (read/write/delete)
- Rate limit violations
- Configuration changes
- System events

**Example:**
```rust
use hyra_scribe_ledger::logging::{audit_log, AuditEvent};

audit_log(
    AuditEvent::AuthSuccess,
    Some("user@example.com"),
    "login",
    Some("/auth"),
    "success",
    Some("User authenticated successfully")
);
```

> **Note**: Audit logging is available as a module. Integration into the application requires calling audit_log at appropriate security checkpoints.

**Test Coverage:**
```bash
# Security module tests
cargo test --lib security::

# Integration tests
cargo test security_tests
```

## Deployment

### Using Shell Scripts

```bash
# Start cluster
./scripts/start-cluster.sh

# Test cluster functionality
./scripts/test-cluster.sh

# Stop cluster
./scripts/stop-cluster.sh
```

### Using Docker Compose

```bash
# Start multi-node cluster
docker-compose up -d

# View logs
docker-compose logs -f

# Stop cluster
docker-compose down
```

### Using Systemd (Production)

```bash
# Install service files
sudo cp scripts/systemd/scribe-node-*.service /etc/systemd/system/

# Start services
sudo systemctl start scribe-node-1
sudo systemctl start scribe-node-2
sudo systemctl start scribe-node-3

# Enable on boot
sudo systemctl enable scribe-node-{1,2,3}

# Check status
sudo systemctl status scribe-node-1
```

See [Systemd Deployment Guide](scripts/systemd/README.md) for details.

## Testing

### Unit & Integration Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific modules
cargo test storage
cargo test consensus
cargo test http_tests
```

### End-to-End Tests

**Python E2E Test Framework:**

```bash
# Install dependencies
pip install -r tests/e2e/requirements.txt

# Build binary
cargo build --bin scribe-node

# Run E2E tests
python3 tests/e2e/cluster_e2e_test.py
```

**Test Coverage:**
- Health checks across all nodes
- Node connectivity verification
- Data replication testing
- Metrics endpoint validation
- Concurrent operations (50+ parallel)
- Performance benchmarking
- Stress testing (100+ operations)

See [E2E Test Guide](tests/e2e/README.md) for details.

**Alternative using scripts:**
```bash
./scripts/start-cluster.sh
./scripts/test-cluster.sh
./scripts/stop-cluster.sh
```

### Test Organization

- `tests/storage_tests.rs` - Storage layer tests
- `tests/consensus_tests.rs` - Raft consensus tests
- `tests/http_tests.rs` - HTTP API tests
- `tests/cluster_tests.rs` - Multi-node cluster tests
- `tests/consistency_tests.rs` - Data consistency tests
- `tests/s3_storage_tests.rs` - S3 integration tests
- `tests/e2e/` - End-to-end test suite

## Examples

### Basic Usage

```bash
cargo run --example basic_usage
```

Simple example demonstrating:
- Creating a storage instance
- Storing and retrieving data
- Flushing to disk

### Interactive CLI

```bash
cargo run --example cli_store
```

Interactive command-line interface with:
- `put <key> <value>` - Store data
- `get <key>` - Retrieve data
- `list` - Show key count
- `clear` - Remove all data
- `quit` - Exit application

### Configuration Demo

```bash
cargo run --example config_demo
```

Demonstrates:
- Loading TOML configuration
- Environment variable overrides
- Configuration validation
- Error handling

### Data Types

```bash
cargo run --example data_types
```

Shows:
- Request/Response types
- Serialization with Serde
- Type system usage

## Error Handling

Comprehensive error types covering:

- **Storage Errors:** Sled operations, I/O failures
- **Consensus Errors:** Raft operations, leadership issues
- **Network Errors:** Connection failures, timeouts
- **Configuration Errors:** Invalid settings, missing files
- **Serialization Errors:** JSON/binary encoding issues

All errors implement proper context and are converted to user-friendly messages.

**Test coverage:**
```bash
# Error handling tests
cargo test error

# Type system tests
cargo test types
```

## Manifest Management

The manifest tracks segment metadata across the cluster:

- Segment locations (local or S3)
- Segment metadata (size, timestamp, checksums)
- Merkle roots for cryptographic verification
- Distributed updates via Raft consensus
- Strong consistency guarantees

**Test coverage:**
```bash
cargo test manifest
```

## Features

### ✅ Implemented

- **HTTP API Server** - RESTful API for data operations
- **Local Storage** - Sled embedded database (hot tier)
- **S3 Cold Storage** - Complete S3-compatible storage
- **Hybrid Architecture** - Multi-tier storage system
- **Distributed Consensus** - OpenRaft cluster coordination
- **Dynamic Clustering** - Join/leave operations, auto-discovery
- **Node Discovery** - Automatic cluster formation
- **Fault Tolerance** - Comprehensive failure handling
- **Manifest System** - Distributed metadata management
- **Async Operations** - High-performance async I/O
- **Configuration System** - TOML + environment variables
- **Error Handling** - Comprehensive error types
- **E2E Testing** - Python-based cluster testing
- **Deployment Tools** - Scripts, Docker, systemd support
- **Prometheus Metrics** - Production-ready monitoring (Task 11.1)
- **Structured Logging** - Advanced logging with tracing (Task 11.2)

### 🚧 Future Enhancements

- Multi-region replication
- Enhanced security (TLS, authentication)
- Performance optimizations
- Log compaction and snapshots

## Development

### Building from Source

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Format code
cargo fmt

# Lint code
cargo clippy
```

### Code Quality Standards

The codebase follows strict quality standards to ensure maintainability and reliability:

- **No Hardcoded Values in Tests:** All test data uses named constants for clarity and maintainability
- **Type Safety:** Strong typing throughout with minimal use of `unwrap()`
- **Documentation:** Comprehensive inline documentation and external docs
- **Consistent Style:** Enforced via `cargo fmt` and `cargo clippy`

**Test Constants Pattern:**
```rust
// Example from src/discovery.rs tests
const TEST_NODE_ID: u64 = 1;
const TEST_NODE_ID_2: u64 = 2;
const TEST_RAFT_PORT: u16 = 9001;
const TEST_CLIENT_PORT: u16 = 8001;
const TEST_IP: &str = "127.0.0.1";
```

This pattern improves test readability and makes it easier to update test configurations.

### Running Tests During Development

```bash
# Run tests on save (install cargo-watch first)
cargo watch -x test

# Run specific test
cargo test test_name

# Run tests with verbose output
cargo test -- --nocapture --test-threads=1
```

### Documentation Organization

- **README.md** - Main project documentation (this file)
- **DEVELOPMENT.md** - Development guide and task tracking
- **docs/** - All other documentation files
  - Configuration guides, deployment guides, troubleshooting, etc.

## Performance

### Optimizations (Task 11.3)

The system includes several performance optimizations for high-throughput and low-latency operations:

#### Core Optimizations

- **Async I/O:** Tokio runtime for concurrent operations
- **Connection Pooling:** Reused HTTP connections for S3 and cluster communication
- **Caching:** Configurable local cache (default 256MB) + LRU hot data cache
- **Compression:** Gzip compression for S3 segments
- **Batching:** Request batching for Raft proposals and storage operations
- **High Throughput Mode:** Optimized Sled configuration

#### Hot Data Caching Layer

A dedicated LRU cache for frequently accessed keys provides:

- **Fast Read Performance:** Cache hits avoid storage backend access
- **Configurable Capacity:** Default 1,000 entries, customizable per deployment
- **Automatic Cache Invalidation:** Write/delete operations update cache coherently
- **Thread-Safe:** Mutex-protected for concurrent access

```rust
use hyra_scribe_ledger::api::{DistributedApi, ReadConsistency};
use std::sync::Arc;

// Create API with custom cache capacity
let api = DistributedApi::with_cache_capacity(consensus, 5000);

// Cache is used automatically for reads
let value = api.get(key, ReadConsistency::Stale).await?;

// Monitor cache usage
println!("Cache size: {}/{}", api.cache_size(), api.cache_capacity());
```

#### Optimized Serialization

- **Bincode for Internal Operations:** Faster than JSON for Raft state and snapshots
- **Zero-Copy Reads:** Direct buffer access where possible
- **Pre-allocated Buffers:** Reduced allocation overhead

```rust
use hyra_scribe_ledger::SimpleScribeLedger;

let ledger = SimpleScribeLedger::temp()?;

// Use bincode for complex data structures
#[derive(serde::Serialize, serde::Deserialize)]
struct ComplexData {
    id: u64,
    metadata: Vec<String>,
}

let data = ComplexData { id: 42, metadata: vec!["tag1".into()] };
ledger.put_bincode("key", &data)?;
let retrieved: Option<ComplexData> = ledger.get_bincode("key")?;
```

#### Tunable Raft Parameters

Consensus performance can be tuned via configuration:

```toml
[consensus]
# Election timeout (milliseconds)
election_timeout_ms = 1000

# Heartbeat interval (milliseconds)
heartbeat_interval_ms = 300

# Maximum entries per Raft proposal (batching)
max_payload_entries = 300

# Snapshot policy (logs to keep before snapshot)
snapshot_logs_since_last = 5000

# Max logs to keep after snapshot
max_in_snapshot_log_to_keep = 1000
```

#### Batch Operations

Efficient batch processing for high throughput:

```rust
// Batch writes
let mut batch = SimpleScribeLedger::new_batch();
for i in 0..1000 {
    batch.insert(format!("key{}", i).as_bytes(), b"value");
}
ledger.apply_batch(batch)?;

// Multiple batches with single flush
ledger.apply_batches_with_flush(vec![batch1, batch2, batch3])?;
```

### Performance Targets

Based on release builds with optimizations enabled:

- **Write Throughput:** 200k+ ops/sec (batched, local)
- **Read Throughput:** 1.8M+ ops/sec (cached, local)
- **Mixed Workload:** 400k+ ops/sec (local)
- **Distributed Write Latency:** < 50ms (3-node cluster)
- **Distributed Read Latency:** < 10ms (linearizable), < 1ms (stale)

### Benchmarks

Run performance benchmarks:

```bash
cargo bench
```

Benchmark categories:
- Storage operations (put/get/delete)
- S3 operations (upload/download)
- Consensus operations
- HTTP endpoint latency

## Troubleshooting

### Common Issues

**Port Already in Use:**
```bash
# Stop existing cluster
./scripts/stop-cluster.sh

# Check for processes
lsof -i :8001,8002,8003
```

**Build Errors:**
```bash
# Clean build
cargo clean
cargo build
```

**Test Failures:**
```bash
# Check logs
ls -la logs/

# Run tests individually
cargo test --test storage_tests
```

**S3 Connection Issues:**
- Verify MinIO is running
- Check endpoint and credentials
- Ensure bucket exists
- Verify network connectivity

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/your-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Format code (`cargo fmt`)
6. Lint code (`cargo clippy`)
7. Commit changes (`git commit -am 'Add feature'`)
8. Push to branch (`git push origin feature/your-feature`)
9. Open a Pull Request

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) file for details.

## Acknowledgments

Hyra Scribe Ledger is built using OpenRaft for modern async Rust patterns and optimized performance.
