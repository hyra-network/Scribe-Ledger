# Hyra Scribe Ledger

> **A distributed, immutable, append-only key-value storage system**

Hyra Scribe Ledger is a production-ready distributed storage system built with Rust, designed for durability, consistency, and high performance. It combines local storage with S3-compatible object storage to provide a robust multi-tier architecture for persistent data management.

## Overview

Hyra Scribe Ledger provides a complete distributed storage solution with:

- **Immutability:** Write-once, read-forever append-only storage
- **Durability:** Multi-tier architecture with local caching and S3 persistence
- **Distribution:** OpenRaft-based consensus for cluster coordination
- **Performance:** Async I/O with Tokio for high-throughput operations
- **Verifiability:** Cryptographic proofs for data integrity

## Architecture

The system uses a multi-tiered architecture combining local and cloud storage:

### Storage Tiers

**Local Tier (Hot Data)**
- Sled embedded database for fast local access
- Optimized for high-throughput writes and low-latency reads
- Configurable cache size (default: 256MB)

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
1. Request served from local storage (fast path)
2. Falls back to S3 if not in local cache
3. Transparent read-through caching
4. Consistent reads across the cluster

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

## Quick Start

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

The HTTP API will be available at `http://localhost:8080`.

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
client_port = 8080          # HTTP API port
raft_tcp_port = 8081        # Raft consensus port

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
export SCRIBE_NETWORK_CLIENT_PORT=8090
export SCRIBE_NETWORK_RAFT_TCP_PORT=8082

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

- **Node 1**: `config-node1.toml` - HTTP: 8080, Raft: 8081
- **Node 2**: `config-node2.toml` - HTTP: 8090, Raft: 8082  
- **Node 3**: `config-node3.toml` - HTTP: 8100, Raft: 8083

## HTTP API

### Data Operations

**Store Data (PUT)**
```bash
curl -X PUT http://localhost:8080/my-key \
  -H "Content-Type: application/octet-stream" \
  --data-binary "my value data"
```

**Retrieve Data (GET)**
```bash
curl http://localhost:8080/my-key
```

**Delete Data (DELETE)**
```bash
curl -X DELETE http://localhost:8080/my-key
```

### Cluster Management

**Check Health**
```bash
curl http://localhost:8080/health
```

**View Metrics**
```bash
curl http://localhost:8080/metrics
```

**Cluster Status**
```bash
curl http://localhost:8080/cluster/status
```

**List Members**
```bash
curl http://localhost:8080/cluster/members
```

**Get Leader**
```bash
curl http://localhost:8080/cluster/leader
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

**Test coverage:**
```bash
# Crypto module tests
cargo test --lib crypto::

# Comprehensive crypto tests
cargo test crypto_tests
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
curl -X PUT http://localhost:8080/key1 --data "value1"
curl -X PUT http://localhost:8090/key2 --data "value2"
curl -X PUT http://localhost:8100/key3 --data "value3"

# Read from any node (data is replicated)
curl http://localhost:8080/key1
curl http://localhost:8090/key2
curl http://localhost:8100/key3
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

### 🚧 Future Enhancements

- Multi-region replication
- Advanced monitoring and metrics
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

### Running Tests During Development

```bash
# Run tests on save (install cargo-watch first)
cargo watch -x test

# Run specific test
cargo test test_name

# Run tests with verbose output
cargo test -- --nocapture --test-threads=1
```

## Performance

### Optimizations

- **Async I/O:** Tokio runtime for concurrent operations
- **Connection Pooling:** Reused connections for S3
- **Caching:** Configurable local cache (default 256MB)
- **Compression:** Gzip compression for S3 segments
- **Batching:** Request batching where applicable
- **High Throughput Mode:** Optimized Sled configuration

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
lsof -i :8080,8090,8100
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
