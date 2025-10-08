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

5. **Run E2E tests:**
   ```bash
   # Run comprehensive E2E tests
   cd tests/e2e
   python3 e2e_test.py
   
   # Run performance benchmarks
   python3 benchmark.py
   
   # Run quick performance tests
   python3 quick_perf.py
   
   # Run stress tests
   python3 stress_test.py
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

For development, use the provided development and testing scripts:

```bash
# Development commands
./dev.sh build    # Build the project
./dev.sh test     # Run tests
./dev.sh fmt      # Format code
./dev.sh clippy   # Run lints

# Comprehensive testing
./test_runner.sh unit           # Run unit tests only
./test_runner.sh performance    # Run performance tests
./test_runner.sh stress         # Run stress tests
./test_runner.sh server         # Start server and run all performance tests
./test_runner.sh all           # Run all tests (requires running server)
```

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

### End-to-End & Performance Tests
Navigate to the E2E testing directory:

```bash
cd tests/e2e
```

**Functional E2E Tests:**
```bash
# Multi-node cluster functionality
python3 e2e_test.py
```

**Performance Benchmarks:**
```bash
# Comprehensive performance analysis
python3 benchmark.py

# Quick performance check
python3 quick_perf.py

# Stress testing under load
python3 stress_test.py [server_url] [duration_seconds]
```

**Performance Test Examples:**
```bash
# Quick 30-second performance test
python3 quick_perf.py

# Full benchmark suite (15s per test)
python3 benchmark.py

# 60-second stress test with high concurrency
python3 stress_test.py http://localhost:8080 60
```

The test suite includes:
- **Unit tests** for core functionality (34 tests)
- **Consensus tests** for Raft cluster behavior (3 tests)
- **Integration tests** for HTTP endpoints
- **Storage tests** including S3 integration (7 S3-specific tests)
- **Cryptographic tests** for Merkle tree verification (10 tests)
- **E2E tests** for multi-node cluster scenarios
- **Performance benchmarks** with detailed metrics and tabular results
- **Stress tests** for high-load scenarios and system limits
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
