# Hyra Scribe Ledger

> **Verifiable, Durable Off-Chain Storage for the Hyra AI Ecosystem**

---

Hyra Scribe Ledger is a distributed, immutable, append-only key-value storage system designed to serve as the durable data layer for Hyra AI. It solves the critical challenge of handling large data payloads (AI model inputs and outputs) that are infeasible to store directly on-chain.

## 🚀 Vision: AI On-Chain, Data Off-Chain

> The core mission of Hyra AI is to execute AI inference tasks transparently and verifiably on-chain. However, this vision faces a fundamental obstacle: the on-chain data dilemma. Storing gigabytes, or even megabytes, of data on a blockchain is prohibitively expensive, slow, and detrimental to scalability.

**Hyra Scribe Ledger is the solution.** Instead of placing raw data on-chain, Hyra smart contracts store a lightweight, immutable cryptographic proof (e.g., a Merkle root or a key hash). The actual data resides in Scribe Ledger, an off-chain system engineered from the ground up for:

- **Durability:** Data, once committed, should be considered permanent.
- **Immutability:** Data cannot be altered or deleted, only appended.
- **Verifiability:** On-chain smart contracts can efficiently verify the integrity of off-chain data.

---

## 🏛️ Core Tenets

Scribe Ledger is built on four foundational principles:

1. **Immutability:** Write-once, read-forever. Data is stored in append-only logs, creating a permanent and auditable history, much like a traditional ledger.
2. **Extreme Durability:** By using S3-compatible object storage as the ultimate source of truth, we inherit its design for 11 nines (99.999999999%) of durability, ensuring data survival against nearly all failure scenarios.
3. **Verifiability:** Every data segment stored can be cryptographically verified. The system's global state (the "Manifest") links data segments to their Merkle roots, allowing any participant—including on-chain smart contracts—to confirm data integrity without needing to trust the storage layer.
4. **High Performance:** The architecture is decoupled to optimize for high-throughput ingestion. Writes are handled by a "hot" local tier for low latency, while the asynchronous flush to the durable "cold" tier happens in the background.

---

## 🏗️ System Architecture

Scribe Ledger employs a multi-tiered, log-structured architecture inspired by high-performance databases and distributed consensus systems.

### **Components**

- **Write Nodes (Ingestion Tier):** Entry point for all data. Each node runs a local instance of the Sled embedded database to buffer incoming writes in a crash-safe Write-Ahead Log (WAL) for immediate, low-latency acknowledgements.
- **S3-Compatible Storage (Durable Tier):** Permanent, cold storage layer. Data is flushed from Write Nodes to S3 in the form of sorted, immutable files called Segments.
- **Advanced Raft Consensus Cluster (Coordination Tier):** Fault-tolerant distributed cluster with sophisticated membership management, auto-discovery, and dual-transport architecture (HTTP + TCP) for optimal performance.
- **Cluster Discovery Service:** Dynamic node discovery and health monitoring system enabling automatic cluster formation and membership management.
- **Distributed Consensus Layer:** Production-ready multi-node Raft implementation with leader election, log replication, join/leave operations, and comprehensive failure handling.

### **Write Path**
1. **Ingest & Local Commit:** Client sends a `put(key, value)` request to a Write Node, which commits it to its local Sled WAL and acknowledges the client.
2. **Asynchronous Flush to S3:** When the local buffer reaches a threshold, the Write Node sorts its contents and streams them as a new, immutable Segment file to S3.
3. **Manifest Update:** After S3 upload, the Write Node proposes a metadata update to the Raft cluster: "Add Segment XYZ to the Manifest."
4. **Global Commit:** The Raft cluster reaches consensus on the new Manifest state. Data is now globally committed and visible across the system.

### **Read Path**
1. **Local Cache Check:** Request first checks the local Sled instance for "hot" data.
2. **Manifest Consultation:** On a local miss, the node consults its Manifest copy to identify which S3 Segments might contain the key.
3. **Segment Search:** Node fetches and searches relevant Segments from S3 in reverse chronological order (newest to oldest).
4. **Return Value:** The first value found is guaranteed to be the most recent version and is returned to the client.

---

## 🛠️ Technology Stack

This project is built on the shoulders of giants in the Rust ecosystem:

| Technology      | Role                                                                 |
|-----------------|----------------------------------------------------------------------|
| **Rust**        | Core programming language, providing memory safety and performance   |
| **Tokio**       | Asynchronous runtime for high-concurrency I/O operations            |
| **Sled**        | Modern, embedded B-tree database for high-performance local tier     |
| **raft-rs**     | Port of etcd's battle-tested Raft implementation for consensus      |
| **aws-sdk-rust**| Official AWS SDK for non-blocking interaction with S3               |
| **Serde**       | Standard for efficient serialization and deserialization            |

---

## 🏁 Getting Started

### Prerequisites
- **Rust 1.70+** - For building from source
- **Git** - For cloning the repository

### Quick Start

1. **Clone the repository:**
   ```bash
   git clone https://github.com/hyra-network/Scribe-Ledger.git
   cd Scribe-Ledger
   ```

2. **Build the project:**
   ```bash
   cargo build --release
   ```

3. **Run a single node:**
   ```bash
   cargo run --bin scribe-node
   ```

4. **Run a 3-node cluster:**
   ```bash
   # Terminal 1 - Node 1 (Leader)
   cargo run --bin scribe-node -- --config config-node1.toml
   
   # Terminal 2 - Node 2 (Follower)
   cargo run --bin scribe-node -- --config config-node2.toml
   
   # Terminal 3 - Node 3 (Follower) 
   cargo run --bin scribe-node -- --config config-node3.toml
   ```

The HTTP server will start on `http://localhost:8080` by default.

### HTTP API Usage

Scribe Ledger provides a simple HTTP API for storing and retrieving data:

#### Store Data (PUT)
```bash
# Store a value
curl -X PUT http://localhost:8080/my-key \
  -H "Content-Type: application/octet-stream" \
  --data-binary "my value data"
```

#### Retrieve Data (GET)
```bash
# Get a value
curl http://localhost:8080/my-key
```

### Configuration

#### Single Node Configuration
Create a `config.toml` file to customize settings:

```toml
[node]
id = 1
address = "127.0.0.1:8001"
data_dir = "./data"

[storage]
segment_size = 1048576  # 1MB
max_cache_size = 268435456  # 256MB
s3_bucket = "scribe-ledger"
s3_region = "us-east-1"
s3_endpoint = "http://localhost:9000"  # MinIO for development

[consensus]
election_timeout = 10
heartbeat_timeout = 3
max_log_entries = 1000

[network]
listen_addr = "127.0.0.1"
client_port = 8080          # HTTP API port for client requests
raft_tcp_port = 8081        # Dedicated TCP port for Raft consensus
```

#### Multi-Node Cluster Configuration
For production deployments, use the provided cluster configuration files:
- `config-node1.toml` - Primary leader node (HTTP: 8080, Raft TCP: 8081)
- `config-node2.toml` - Follower node (HTTP: 8090, Raft TCP: 8082)
- `config-node3.toml` - Follower node (HTTP: 8100, Raft TCP: 8083)

Each node configuration includes:
- **Separate ports**: HTTP API port for client communication and dedicated TCP port for Raft consensus
- **Cluster membership**: Peer discovery and automatic leader election
- **S3 integration**: Shared MinIO/S3 storage for distributed persistence
- **Health monitoring**: Heartbeat and failure detection mechanisms

### Development

For development, use standard Cargo commands:

```bash
# Build the project
cargo build

# Build in release mode
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Run lints
cargo clippy
```

---

## 📚 Tutorials & Usage Guides

This section provides comprehensive tutorials for all implemented features across Phases 1-9 of the development roadmap. Each tutorial shows you how to use the corresponding functionality with practical examples.

### Table of Contents

- [Phase 1: Foundation & Configuration](#phase-1-foundation--configuration)
  - [Task 1.1: Project Setup](#task-11-project-setup)
  - [Task 1.2: Configuration System](#task-12-configuration-system)
  - [Task 1.3: Error Handling & Types](#task-13-error-handling--types)
- [Phase 2: Storage Layer](#phase-2-storage-layer)
  - [Task 2.1: Storage Backend](#task-21-storage-backend)
  - [Task 2.2: Storage Tests](#task-22-storage-tests)
  - [Task 2.3: Segment-based Storage](#task-23-segment-based-storage)
- [Phase 3: OpenRaft Consensus](#phase-3-openraft-consensus)
- [Phase 4: Manifest Management](#phase-4-manifest-management)
- [Phase 5: HTTP API Server](#phase-5-http-api-server)
- [Phase 6: S3 Cold Storage Integration](#phase-6-s3-cold-storage-integration)
- [Phase 7: Node Discovery & Cluster Formation](#phase-7-node-discovery--cluster-formation)
- [Phase 8: Write Path & Data Replication](#phase-8-write-path--data-replication)
- [Phase 9: Binary & Multi-Node Deployment](#phase-9-binary--multi-node-deployment)

---

### Phase 1: Foundation & Configuration

Phase 1 established the project foundation with configuration management, error handling, and type systems.

#### Task 1.1: Project Setup

**What was implemented:**
- Complete project structure with OpenRaft dependencies
- Directory organization: `src/{consensus/, storage/, network/, manifest/, config/}`
- Build system with Cargo.toml configuration

**How to use:**

```bash
# Clone and build the project
git clone https://github.com/amogusdrip285/simple-scribe-ledger.git
cd simple-scribe-ledger

# Build in debug mode
cargo build

# Build optimized release
cargo build --release

# Run tests to verify setup
cargo test
```

**Example files:**
- See: [`Cargo.toml`](Cargo.toml) - Project dependencies and configuration
- See: [`src/lib.rs`](src/lib.rs) - Main library entry point

---

#### Task 1.2: Configuration System

**What was implemented:**
- TOML-based configuration system
- Environment variable overrides (SCRIBE_* prefix)
- Multi-node configuration support
- Validation and default values

**How to use:**

Run the configuration demo to see all features:

```bash
cargo run --example config_demo
```

**Configuration file example:**

```toml
# config.toml
[node]
id = 1
address = "127.0.0.1:8001"
data_dir = "./data"

[storage]
segment_size = 1048576          # 1MB
max_cache_size = 268435456      # 256MB

[consensus]
election_timeout = 10
heartbeat_timeout = 3

[network]
listen_addr = "127.0.0.1"
client_port = 8080              # HTTP API
raft_tcp_port = 8081            # Raft consensus
```

**Environment variable overrides:**

```bash
# Override configuration with environment variables
export SCRIBE_NODE_ID=2
export SCRIBE_NETWORK_CLIENT_PORT=8090
export SCRIBE_NETWORK_RAFT_TCP_PORT=8082

cargo run --bin scribe-node
```

**Example files:**
- See: [`examples/config_demo.rs`](examples/config_demo.rs) - Interactive configuration demo
- See: [`config.toml`](config.toml) - Single-node configuration
- See: [`config-node1.toml`](config-node1.toml), [`config-node2.toml`](config-node2.toml), [`config-node3.toml`](config-node3.toml) - Multi-node configurations

**Test files:**
- Run: `cargo test config` - Configuration system tests

---

#### Task 1.3: Error Handling & Types

**What was implemented:**
- Comprehensive `ScribeError` enum with all error types
- Type aliases for common types (Key, Value, NodeId, etc.)
- Request/Response type system for operations
- Serialization support with Serde

**How to use:**

The error handling system is used throughout the codebase. Here's an example:

```rust
use hyra_scribe_ledger::config::Config;
use hyra_scribe_ledger::error::ScribeError;
use hyra_scribe_ledger::types::{Request, Response};

fn main() -> Result<(), ScribeError> {
    // Error handling with proper types
    let config = Config::from_file("config.toml")?;
    
    // Type-safe requests
    let request = Request::Put {
        key: b"my_key".to_vec(),
        value: b"my_value".to_vec(),
    };
    
    // Serialize to JSON
    let json = serde_json::to_string_pretty(&request)?;
    println!("{}", json);
    
    Ok(())
}
```

**Example files:**
- See: [`examples/config_demo.rs`](examples/config_demo.rs) - Demonstrates error handling
- See: [`examples/data_types.rs`](examples/data_types.rs) - Shows type system usage

**Test files:**
- Run: `cargo test error` - Error handling tests
- Run: `cargo test types` - Type system tests

---

### Phase 2: Storage Layer

Phase 2 implemented the local storage layer with Sled embedded database and segment-based storage architecture.

#### Task 2.1: Storage Backend

**What was implemented:**
- `StorageBackend` trait for storage abstraction
- `SledStorage` implementation with async wrappers
- Basic operations: put, get, delete, flush, snapshot

**How to use:**

Run the basic usage example:

```bash
cargo run --example basic_usage
```

**Code example:**

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

**Example files:**
- See: [`examples/basic_usage.rs`](examples/basic_usage.rs) - Simple storage usage
- See: [`examples/cli_store.rs`](examples/cli_store.rs) - Interactive CLI application

**Test files:**
- Run: `cargo test storage_tests` - Basic storage operations
- Run: `cargo test sled_engine` - Sled-specific tests

---

#### Task 2.2: Storage Tests

**What was implemented:**
- Comprehensive test suite for storage operations
- Tests for concurrent operations, large data, persistence
- Edge case handling (empty keys, Unicode, etc.)

**How to run tests:**

```bash
# Run all storage tests
cargo test storage

# Run with output to see detailed results
cargo test storage -- --nocapture

# Run specific test modules
cargo test storage_tests::
cargo test sled_engine_tests::
```

**Test coverage includes:**
- Basic put/get/delete operations
- Large data handling (10MB+ values)
- Concurrent operations (100+ parallel requests)
- Persistence across restarts
- Unicode and special characters
- Error cases and edge conditions

**Test files:**
- See: [`tests/storage_tests.rs`](tests/storage_tests.rs) - Storage layer tests
- See: [`tests/sled_engine_tests.rs`](tests/sled_engine_tests.rs) - Sled engine tests

---

#### Task 2.3: Segment-based Storage

**What was implemented:**
- `Segment` data structure for grouping key-value pairs
- Segment serialization/deserialization
- Segment manager for tracking active/archived segments
- Foundation for S3 archival (Phase 6)

**How to use:**

Segments are used internally for data organization and archival:

```rust
use hyra_scribe_ledger::storage::segment::{Segment, SegmentManager};
use std::collections::HashMap;

// Create a segment
let mut data = HashMap::new();
data.insert(b"key1".to_vec(), b"value1".to_vec());
data.insert(b"key2".to_vec(), b"value2".to_vec());

let segment = Segment::from_data(1, data);

// Segment manager tracks multiple segments
let segment_mgr = SegmentManager::new();
segment_mgr.add_segment(segment);
```

**Test files:**
- Run: `cargo test segment` - Segment-related tests

---

### Phase 3: OpenRaft Consensus

Phase 3 implemented distributed consensus using OpenRaft for cluster coordination.

**What was implemented:**
- OpenRaft state machine for log replication
- Persistent Raft storage backend
- Network layer for Raft RPCs
- Cluster membership management
- Leader election and failover

**How to use:**

The consensus layer is automatically used when running multi-node clusters:

```bash
# Start a 3-node cluster (see Phase 9 for details)
./scripts/start-cluster.sh

# The nodes automatically:
# - Elect a leader
# - Replicate data across nodes
# - Handle node failures
# - Maintain consistency
```

**Test files:**
- Run: `cargo test consensus` - Consensus layer tests
- Run: `cargo test cluster` - Multi-node cluster tests

**Key features:**
- Automatic leader election
- Log replication across nodes
- Membership changes (join/leave)
- Graceful failover on leader failure
- Strong consistency guarantees

---

### Phase 4: Manifest Management

Phase 4 implemented the manifest system for tracking segment metadata across the cluster.

**What was implemented:**
- Manifest data structures for segment metadata
- Manifest manager with consensus integration
- Merkle root tracking for verification
- Distributed manifest updates via Raft

**How to use:**

The manifest is managed automatically by the system. It tracks:
- Segment locations (local or S3)
- Segment metadata (size, timestamp, checksums)
- Merkle roots for cryptographic verification

**Test files:**
- Run: `cargo test manifest` - Manifest management tests

---

### Phase 5: HTTP API Server

Phase 5 implemented the HTTP API server for client interactions.

**What was implemented:**
- RESTful HTTP API with Actix-web
- CRUD endpoints (PUT, GET, DELETE)
- Cluster management endpoints
- Health and metrics endpoints
- Binary data support

**How to use:**

Start a node and use the HTTP API:

```bash
# Start a single node
cargo run --bin scribe-node

# In another terminal, use curl or any HTTP client:

# Store data
curl -X PUT http://localhost:8080/my-key \
  -H "Content-Type: application/octet-stream" \
  --data-binary "my value data"

# Retrieve data
curl http://localhost:8080/my-key

# Delete data
curl -X DELETE http://localhost:8080/my-key

# Check health
curl http://localhost:8080/health

# View metrics
curl http://localhost:8080/metrics

# Cluster status
curl http://localhost:8080/cluster/status
```

**API Endpoints:**

**Data Operations:**
- `PUT /{key}` - Store a value
- `GET /{key}` - Retrieve a value
- `DELETE /{key}` - Delete a value

**Cluster Management:**
- `GET /cluster/status` - Get cluster status
- `GET /cluster/members` - List cluster members
- `GET /cluster/leader` - Get current leader
- `POST /cluster/join` - Join a node to cluster
- `POST /cluster/leave` - Remove a node from cluster

**Health & Monitoring:**
- `GET /health` - Health check endpoint
- `GET /metrics` - Prometheus-compatible metrics

**Test files:**
- Run: `cargo test http_tests` - HTTP API tests
- See: [`tests/http_tests.rs`](tests/http_tests.rs) - Complete API test suite

---

### Phase 6: S3 Cold Storage Integration

Phase 6 implemented S3-compatible object storage for cold data archival.

**What was implemented:**
- S3 storage backend with AWS SDK
- MinIO support for local development
- Automatic segment archival to S3
- Read-through caching for cold data
- Compression with configurable levels
- Data tiering policies based on age

**How to use:**

For detailed S3 usage, see the comprehensive documentation:

📖 **[S3 Storage Documentation](docs/S3_STORAGE.md)** - Complete S3 setup and usage guide  
📖 **[Archival & Tiering Documentation](docs/ARCHIVAL_TIERING.md)** - Data tiering and archival guide

**Quick Start with MinIO:**

```bash
# 1. Start MinIO (local S3-compatible storage)
docker run -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  minio/minio server /data --console-address ":9001"

# 2. Create a bucket
aws --endpoint-url http://localhost:9000 s3 mb s3://my-bucket

# 3. Configure S3 in config.toml
cat >> config.toml << EOF
[storage.s3]
bucket = "my-bucket"
region = "us-east-1"
endpoint = "http://localhost:9000"
access_key_id = "minioadmin"
secret_access_key = "minioadmin"
path_style = true
EOF

# 4. Start the node (S3 archival is automatic)
cargo run --bin scribe-node
```

**Configuration example:**

```toml
[storage.s3]
bucket = "my-scribe-bucket"
region = "us-east-1"
endpoint = "http://localhost:9000"    # MinIO endpoint
access_key_id = "minioadmin"
secret_access_key = "minioadmin"
path_style = true                      # Required for MinIO

[storage.tiering]
age_threshold_secs = 3600              # Archive after 1 hour
enable_compression = true
compression_level = 6                  # Balanced (0-9)
enable_auto_archival = true
archival_check_interval_secs = 300    # Check every 5 minutes
```

**Test files:**
- Run: `cargo test s3_storage_tests -- --ignored` - S3 integration tests (requires MinIO)
- Run: `cargo test segment_archival -- --ignored` - Archival tests
- Run: `cargo test data_tiering -- --ignored` - Tiering tests

---

### Phase 7: Node Discovery & Cluster Formation

Phase 7 implemented automatic node discovery and dynamic cluster formation.

**What was implemented:**
- Discovery service with UDP broadcast
- Peer list management
- Heartbeat protocol for health monitoring
- Automatic cluster joining
- Failure detection

**How to use:**

Node discovery is automatic when running multi-node clusters:

```bash
# Nodes automatically discover each other when started
./scripts/start-cluster.sh

# The discovery service:
# - Broadcasts node presence
# - Discovers peer nodes
# - Maintains peer list
# - Detects node failures
# - Triggers failover when needed
```

**Test files:**
- Run: `cargo test discovery` - Discovery service tests
- See: [`tests/discovery_tests.rs`](tests/discovery_tests.rs) - Discovery test suite

---

### Phase 8: Write Path & Data Replication

Phase 8 implemented the distributed write and read paths with data replication.

**What was implemented:**
- Write request handling with leader forwarding
- Read request handling with local-first strategy
- Data consistency across nodes
- Request batching and optimization
- Comprehensive consistency tests

**How to use:**

Write and read operations are transparent across the cluster:

```bash
# Start a 3-node cluster
./scripts/start-cluster.sh

# Write to any node (automatically forwarded to leader)
curl -X PUT http://localhost:8080/key1 --data "value1"    # Node 1
curl -X PUT http://localhost:8090/key2 --data "value2"    # Node 2
curl -X PUT http://localhost:8100/key3 --data "value3"    # Node 3

# Read from any node (served locally if available)
curl http://localhost:8080/key1   # Can read from any node
curl http://localhost:8090/key2   # Data is replicated
curl http://localhost:8100/key3   # Consistent across cluster
```

**How it works:**

**Write Path:**
1. Client sends PUT to any node
2. If not leader, request is forwarded to leader
3. Leader proposes write via Raft consensus
4. Waits for quorum (majority of nodes)
5. Applies to local storage on all nodes
6. Returns success to client

**Read Path:**
1. Client sends GET to any node
2. Node checks local storage first (fast path)
3. If not found locally, checks manifest
4. Retrieves from S3 if necessary (cold data)
5. Returns value to client

**Test files:**
- Run: `cargo test write_request` - Write path tests
- Run: `cargo test read_request` - Read path tests
- Run: `cargo test consistency` - Data consistency tests
- See: [`tests/consistency_tests.rs`](tests/consistency_tests.rs) - Consistency test suite

---

### Phase 9: Binary & Multi-Node Deployment

Phase 9 provided deployment scripts, systemd services, and comprehensive end-to-end testing.

#### Task 9.1: Node Binary

**What was implemented:**
- Production-ready `scribe-node` binary
- Command-line argument parsing
- Configuration file support
- Graceful shutdown handling

**How to use:**

```bash
# Build the binary
cargo build --release --bin scribe-node

# Run with default config
./target/release/scribe-node

# Run with custom config
./target/release/scribe-node --config config-node1.toml

# Run with environment variables
SCRIBE_NODE_ID=2 SCRIBE_NETWORK_CLIENT_PORT=8090 \
  ./target/release/scribe-node
```

---

#### Task 9.2: Multi-Node Testing Scripts

**What was implemented:**
- Cluster management scripts (start/stop/test)
- Systemd service files for production deployment
- Docker support (Dockerfile, docker-compose.yml)

**How to use:**

**Using Shell Scripts:**

```bash
# Start a 3-node cluster
./scripts/start-cluster.sh

# Test the cluster (health checks, data operations)
./scripts/test-cluster.sh

# Stop the cluster
./scripts/stop-cluster.sh
```

**Using Docker Compose:**

```bash
# Start cluster with Docker
docker-compose up -d

# View logs
docker-compose logs -f

# Stop cluster
docker-compose down
```

**Using Systemd (Production):**

```bash
# Install service files
sudo cp scripts/systemd/scribe-node-*.service /etc/systemd/system/

# Start services
sudo systemctl start scribe-node-1
sudo systemctl start scribe-node-2
sudo systemctl start scribe-node-3

# Enable auto-start on boot
sudo systemctl enable scribe-node-1
sudo systemctl enable scribe-node-2
sudo systemctl enable scribe-node-3

# Check status
sudo systemctl status scribe-node-1
```

**Example files:**
- See: [`scripts/start-cluster.sh`](scripts/start-cluster.sh) - Cluster startup script
- See: [`scripts/stop-cluster.sh`](scripts/stop-cluster.sh) - Cluster shutdown script
- See: [`scripts/test-cluster.sh`](scripts/test-cluster.sh) - Cluster testing script
- See: [`scripts/systemd/README.md`](scripts/systemd/README.md) - Systemd deployment guide
- See: [`Dockerfile`](Dockerfile) - Docker image definition
- See: [`docker-compose.yml`](docker-compose.yml) - Multi-node Docker setup

---

#### Task 9.3: End-to-End Tests

**What was implemented:**
- Python E2E test framework
- 7 comprehensive test scenarios
- CI/CD integration with GitHub Actions
- Stress testing and performance benchmarks

**How to use:**

For detailed E2E test documentation, see:

📖 **[E2E Test Documentation](tests/e2e/README.md)** - Complete testing guide

**Quick Start:**

```bash
# 1. Install Python dependencies
pip install -r tests/e2e/requirements.txt

# 2. Build the node binary
cargo build --bin scribe-node

# 3. Run E2E tests
python3 tests/e2e/cluster_e2e_test.py
```

**Test Coverage:**

The E2E test suite includes:

1. **Health Checks** - Verify all nodes respond correctly
2. **Node Connectivity** - Ensure inter-node communication
3. **Data Replication** - Test data propagation across nodes
4. **Metrics Endpoints** - Verify monitoring endpoints
5. **Concurrent Operations** - Test parallel operations (50+ requests)
6. **Performance Benchmark** - Measure latency and throughput
7. **Stress Test** - System behavior under load (100+ operations)

**Running specific tests:**

```bash
# Run just the health checks
python3 tests/e2e/cluster_e2e_test.py --test health

# Run with verbose output
python3 tests/e2e/cluster_e2e_test.py --verbose

# Run performance benchmarks only
python3 tests/e2e/cluster_e2e_test.py --test benchmark
```

**Alternative: Using Scripts:**

```bash
# Start cluster and run all tests
./scripts/start-cluster.sh
./scripts/test-cluster.sh
./scripts/stop-cluster.sh
```

**Test files:**
- See: [`tests/e2e/cluster_e2e_test.py`](tests/e2e/cluster_e2e_test.py) - Main E2E test suite
- See: [`tests/e2e_infrastructure_tests.rs`](tests/e2e_infrastructure_tests.rs) - Infrastructure tests
- Run: `cargo test e2e_infrastructure` - E2E Rust tests

---

## 🎓 Learning Path

If you're new to the project, we recommend following this learning path:

1. **Start with Phase 1** - Understand configuration and error handling
   - Run: `cargo run --example config_demo`
   - Run: `cargo run --example basic_usage`

2. **Explore Phase 2** - Learn storage operations
   - Run: `cargo run --example cli_store`
   - Experiment with the interactive CLI

3. **Try Phase 5** - Use the HTTP API
   - Start: `cargo run --bin scribe-node`
   - Test with curl commands

4. **Deploy Phase 9** - Run a multi-node cluster
   - Run: `./scripts/start-cluster.sh`
   - Explore cluster operations

5. **Advanced: Phase 6** - Set up S3 storage
   - Follow: [S3 Storage Documentation](docs/S3_STORAGE.md)
   - Set up MinIO for local testing

---

## 📋 Current Features

### ✅ Implemented
- **HTTP API Server** - RESTful API for data storage and retrieval
- **Local Storage** - Sled embedded database for persistent key-value storage (hot tier)
- **S3 Cold Storage** - Complete S3-compatible storage with automatic flush and recovery
- **Hybrid Architecture** - Multi-tier storage with local cache + durable S3 backend
- **Advanced Distributed Consensus** - Production-ready Raft cluster with TCP + HTTP dual transport
- **Dynamic Cluster Management** - Join/leave operations with leadership transfer and auto-discovery
- **Cluster Discovery Service** - Automatic node discovery and health monitoring
- **Fault Tolerance** - Comprehensive failure handling with graceful node recovery
- **Manifest Synchronization** - Distributed metadata management with strong consistency
- **Async Operations** - High-performance asynchronous I/O with Tokio
- **Error Handling** - Comprehensive error types and handling
- **Configuration System** - Flexible TOML + environment variable configuration
- **Environment Variable Support** - Complete runtime configuration via env vars
- **MinIO Integration** - Full S3-compatible development environment
- **E2E Testing Framework** - Python-based multi-node cluster testing
- **Comprehensive Testing** - Unit, integration, and S3 workflow tests
- **Merkle Trees** - Complete cryptographic proof generation and verification

### ✅ Phase 2 Complete: S3 Integration
- **S3 Cold Storage** - Complete S3-compatible storage integration with MinIO support
- **Data Recovery** - Automatic data recovery from S3 on startup
- **Read-through Cache** - Seamless data retrieval from S3 when not in local cache
- **Background Flush** - Automatic periodic flushing of data to S3
- **Immutable Segments** - Readonly S3 objects ensuring data immutability
- **Comprehensive Testing** - Full test suite for S3 integration workflows

### 🎯 Advanced Features
- **Merkle Tree Verification** - Complete cryptographic proof system for data integrity
- **Production-Ready Consensus** - Advanced Raft implementation with TCP server and connection pooling
- **Sophisticated Cluster Management** - Join/leave operations, leadership transfer, and graceful node handling
- **Dual Transport Architecture** - HTTP API + dedicated TCP server for optimal Raft performance
- **Auto-Discovery System** - Dynamic cluster formation with health monitoring and failure detection
- **Connection Management** - Connection pooling, retry logic, and comprehensive error handling
- **State Machine Replication** - Consistent manifest updates with distributed consensus guarantees

### 🚧 Future Enhancements
- **Multi-Region Support** - Cross-region data replication and disaster recovery
- **Advanced Monitoring** - Metrics, alerts, and observability dashboards
- **Security Hardening** - Authentication, authorization, and encryption at rest
- **Performance Optimization** - Log compaction, batch operations, and caching improvements

---

## 🧪 Testing

### Unit & Integration Tests
Run the comprehensive Rust test suite:

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test modules
cargo test storage
cargo test consensus
cargo test crypto
```

### End-to-End Tests

**Functional E2E Tests:**
```bash
# Multi-node cluster functionality
python3 tests/e2e/cluster_e2e_test.py
```

**Alternative: Using Scripts**
```bash
# Start the cluster
./scripts/start-cluster.sh

# Run tests (in another terminal)
./scripts/test-cluster.sh

# Stop the cluster
./scripts/stop-cluster.sh
```

The test suite includes:
- **Unit tests** for core functionality
- **Consensus tests** for Raft cluster behavior
- **Integration tests** for HTTP endpoints
- **Storage tests** including S3 integration
- **Cryptographic tests** for Merkle tree verification
- **E2E tests** for multi-node cluster scenarios
- **Configuration validation** and environment variable tests

---

## 🤝 Contributing

We welcome contributions from the community! Whether you're fixing a bug, improving documentation, or proposing a new feature, your help is valued.

Please read our `CONTRIBUTING.md` for details on our code of conduct and the process for submitting pull requests.

**How to contribute:**
1. Fork the repository
2. Create a new branch (`git checkout -b feature/YourFeature`)
3. Commit your changes (`git commit -am 'Add some feature'`)
4. Push to the branch (`git push origin feature/YourFeature`)
5. Open a Pull Request

---

## 📄 License

Hyra Scribe Ledger is distributed under the terms of the MIT license.

See the [LICENSE](LICENSE) file for details.
