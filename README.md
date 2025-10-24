<div align="center">

# 🔥 Hyra Scribe Ledger

### *Verifiable, Durable Off-Chain Storage for the AI Ecosystem*

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]() 
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()
[![S3](https://img.shields.io/badge/storage-S3%20Compatible-yellow)]()

**🚀 Production-Ready** | **⚡ High-Performance** | **🔒 Cryptographically Verified** | **☁️ Cloud-Native**

[Quick Start](#-quick-start) • [Features](#-features) • [Architecture](#-architecture) • [Documentation](#-documentation)

---

</div>

## 🎯 What is Hyra Scribe Ledger?

Hyra Scribe Ledger is a **distributed, immutable, append-only storage system** built with Rust for the modern AI and blockchain ecosystem. It combines the speed of local storage with the durability of cloud object storage (S3), providing a **multi-tier architecture** optimized for both hot and cold data.

### ✨ Key Highlights

<table>
<tr>
<td width="50%">

**🏗️ Multi-Tier Architecture**
- Hot Tier: Sled embedded database
- Cold Tier: S3-compatible object storage
- Automatic tiering & archival
- Seamless read-through caching

</td>
<td width="50%">

**🔄 Distributed Consensus**
- OpenRaft for strong consistency
- Automatic leader election
- Dynamic cluster membership
- Fault-tolerant replication

</td>
</tr>
<tr>
<td width="50%">

**⚡ High Performance**
- 200k+ writes/sec (batched)
- 1.8M+ reads/sec (cached)
- LRU hot data cache
- Async I/O with Tokio

</td>
<td width="50%">

**🔐 Cryptographically Verified**
- Merkle tree proofs
- SHA-256 hashing
- Tamper-proof integrity
- Audit trail support

</td>
</tr>
</table>

---

## 🚀 Quick Start

### Prerequisites

- **Rust 1.70+** - Install from [rustup.rs](https://rustup.rs/)
- **Docker** (Optional) - For S3-compatible storage with MinIO

### Installation & First Run

```bash
# Clone the repository
git clone https://github.com/hyra-network/Scribe-Ledger.git
cd Scribe-Ledger

# Build (release mode for best performance)
cargo build --release

# Start a single node
./target/release/scribe-node

# In another terminal, test the API
curl -X PUT http://localhost:8001/hello -d "Hello, Hyra!"
curl http://localhost:8001/hello
# Output: Hello, Hyra!
```

**🎉 You're up and running!** The node is now serving at `http://localhost:8001`

---

## 🌟 Features

### ✅ Production-Ready

<table>
<tr>
  <td>✅</td>
  <td><strong>HTTP RESTful API</strong></td>
  <td>Simple PUT/GET/DELETE operations</td>
</tr>
<tr>
  <td>✅</td>
  <td><strong>S3 Integration</strong></td>
  <td>AWS S3, MinIO, or any S3-compatible storage</td>
</tr>
<tr>
  <td>✅</td>
  <td><strong>Multi-Node Cluster</strong></td>
  <td>3+ nodes with automatic failover</td>
</tr>
<tr>
  <td>✅</td>
  <td><strong>Auto Discovery</strong></td>
  <td>Nodes discover each other automatically</td>
</tr>
<tr>
  <td>✅</td>
  <td><strong>Prometheus Metrics</strong></td>
  <td>Production-grade monitoring</td>
</tr>
<tr>
  <td>✅</td>
  <td><strong>Structured Logging</strong></td>
  <td>Advanced logging with tracing</td>
</tr>
<tr>
  <td>✅</td>
  <td><strong>Docker Support</strong></td>
  <td>Container-ready deployment</td>
</tr>
<tr>
  <td>✅</td>
  <td><strong>Systemd Integration</strong></td>
  <td>Production deployment scripts</td>
</tr>
</table>

---

## 🏗️ Architecture

### Multi-Tier Storage Design

```
┌──────────────────────────────────────────────────────────────┐
│                        CLIENT REQUESTS                       │
└──────────────────────────┬───────────────────────────────────┘
                           │
              ┌────────────▼───────────┐
              │     HTTP API Server    │
              │    (Axum Framework)    │
              └────────────┬───────────┘
                           │
          ┌────────────────┼────────────────┐
          │                │                │
  ┌───────▼──────┐ ┌──────▼───────┐ ┌─────▼──────┐
  │   Node 1     │ │   Node 2     │ │  Node 3    │
  │              │ │              │ │            │
  │ ┌──────────┐ │ │ ┌──────────┐ │ │ ┌────────┐ │
  │ │LRU Cache │ │ │ │LRU Cache │ │ │ │LRU Cache│ │
  │ └────┬─────┘ │ │ └────┬─────┘ │ │ └────┬───┘ │
  │      │       │ │      │       │ │      │     │
  │ ┌────▼─────┐ │ │ ┌────▼─────┐ │ │ ┌────▼───┐ │
  │ │   Sled   │ │ │ │   Sled   │ │ │ │  Sled  │ │
  │ │ Database │ │ │ │ Database │ │ │ │Database│ │
  │ └────┬─────┘ │ │ └────┬─────┘ │ │ └────┬───┘ │
  └──────┼───────┘ └──────┼───────┘ └──────┼─────┘
         │                │                │
         └────────────────┼────────────────┘
                          │
          ┌───────────────▼────────────────┐
          │    Raft Consensus Layer        │
          │   (Leader Election,            │
          │    Log Replication)            │
          └───────────────┬────────────────┘
                          │
         ┌────────────────┼────────────────┐
         │                │                │
    ┌────▼────┐      ┌────▼────┐     ┌────▼────┐
    │S3 Bucket│      │S3 Bucket│     │S3 Bucket│
    │ Node 1  │      │ Node 2  │     │ Node 3  │
    └─────────┘      └─────────┘     └─────────┘
         │                │                │
         └────────────────▼────────────────┘
                          │
              ┌───────────▼────────────┐
              │   MinIO / AWS S3       │
              │  (Cold Storage Tier)   │
              └────────────────────────┘
```

### Data Flow

#### 📝 Write Path (Strong Consistency)
1. Client → Any Node (HTTP PUT)
2. Forward to Leader (if necessary)
3. Leader proposes via Raft
4. Replicate to Quorum
5. Apply to local storage
6. Archive to S3 (async)
7. Success → Client

#### 📖 Read Path (High Performance)
1. Check LRU Cache → Instant response
2. Check Sled database → Fast local read
3. Fetch from S3 → Durable cold storage
4. Update cache → Optimize future reads

---

## 💻 API Reference

### 📡 Core Operations

```bash
# Store data
curl -X PUT http://localhost:8001/user:alice \
  -H "Content-Type: text/plain" \
  -d "Alice Johnson"

# Retrieve data
curl http://localhost:8001/user:alice
# Output: Alice Johnson

# Delete data
curl -X DELETE http://localhost:8001/user:alice
```

### 📊 Monitoring Endpoints

```bash
# Health check
curl http://localhost:8001/health
# {"status":"ok","node_id":1}

# Raft metrics
curl http://localhost:8001/metrics
# JSON with current_term, current_leader, last_applied...

# Prometheus metrics
curl http://localhost:8001/metrics/prometheus
# Prometheus-formatted metrics
```

### 🔍 Cluster Operations

```bash
# Cluster status
curl http://localhost:8001/cluster/info

# List nodes
curl http://localhost:8001/cluster/nodes

# Leader info
curl http://localhost:8001/cluster/leader/info
```

---

## 🌐 Multi-Node Cluster Setup

### Option 1: Automated Test Script (Recommended)

```bash
# Test 3-node cluster with Docker MinIO S3
./scripts/test-3node-cluster-docker-s3.sh
```

This script will:
- ✅ Start MinIO S3 storage in Docker
- ✅ Create S3 buckets for each node
- ✅ Start 3 Scribe Ledger nodes
- ✅ Test data replication
- ✅ Verify cluster health

**Test Report Available:** See [FINAL_TEST_REPORT.md](FINAL_TEST_REPORT.md) for detailed results.

### Option 2: Manual Cluster Setup

**Start Node 1 (Bootstrap):**
```bash
./target/release/scribe-node --bootstrap --config config-node1.toml
```

**Start Node 2:**
```bash
./target/release/scribe-node --config config-node2.toml
```

**Start Node 3:**
```bash
./target/release/scribe-node --config config-node3.toml
```

**Verify cluster:**
```bash
curl http://localhost:8001/health  # Node 1
curl http://localhost:8002/health  # Node 2
curl http://localhost:8003/health  # Node 3
```

### Option 3: Shell Scripts

```bash
./scripts/start-cluster.sh   # Start cluster
./scripts/test-cluster.sh    # Test operations
./scripts/stop-cluster.sh    # Stop cluster
```

---

## ☁️ S3 Storage Configuration

### Local Development with MinIO

**Start MinIO with Docker:**
```bash
docker run -d -p 9000:9000 -p 9001:9001 \
  -e MINIO_ROOT_USER=minioadmin \
  -e MINIO_ROOT_PASSWORD=minioadmin \
  --name minio \
  minio/minio server /data --console-address ":9001"
```

**Or use Docker Compose:**
```bash
docker-compose -f docker-compose-minio.yml up -d
```

**Access MinIO Console:** `http://localhost:9001` (minioadmin/minioadmin)

### Configure Node for S3

Edit `config.toml`:
```toml
[storage.s3]
bucket = "scribe-ledger-node1"
region = "us-east-1"
endpoint = "http://localhost:9000"  # MinIO
access_key_id = "minioadmin"
secret_access_key = "minioadmin"
path_style = true                    # Required for MinIO
pool_size = 10
timeout_secs = 30
max_retries = 3
```

### Data Tiering & Archival

```toml
[storage.tiering]
age_threshold_secs = 3600           # Archive after 1 hour
enable_compression = true
compression_level = 6                # 0-9 (balanced)
enable_auto_archival = true
archival_check_interval_secs = 300  # Check every 5 minutes
```

**Features:**
- ✅ Automatic age-based archival
- ✅ Gzip compression (configurable)
- ✅ Read-through caching
- ✅ S3 metadata storage
- ✅ Lifecycle management

**📚 Full Guide:** [S3 Storage Documentation](docs/S3_STORAGE.md)

---

## 🔐 Cryptographic Verification

### Merkle Tree Proofs

Every key-value pair can be cryptographically verified:

```bash
# Store data
curl -X PUT http://localhost:8001/test-key \
  -d "verified data"

# Get Merkle proof
curl http://localhost:8001/verify/test-key
```

**Response:**
```json
{
  "key": "test-key",
  "verified": true,
  "proof": {
    "root_hash": "a1b2c3d4e5f6...",
    "siblings": ["e5f6g7h8...", "i9j0k1l2..."]
  },
  "error": null
}
```

### Usage in Rust

```rust
use hyra_scribe_ledger::crypto::MerkleTree;

// Build tree
let pairs = vec![
    (b"key1".to_vec(), b"value1".to_vec()),
    (b"key2".to_vec(), b"value2".to_vec()),
];
let tree = MerkleTree::from_pairs(pairs);

// Get root hash
let root = tree.root_hash().unwrap();

// Generate & verify proof
let proof = tree.get_proof(b"key1").unwrap();
assert!(MerkleTree::verify_proof(&proof, &root));
```

---

## ⚡ Performance

### Benchmarks (Release Build)

| Operation | Throughput | Latency |
|-----------|------------|---------|
| **Local Writes** (batched) | 200k+ ops/sec | < 5μs |
| **Local Reads** (cached) | 1.8M+ ops/sec | < 1μs |
| **Mixed Workload** | 400k+ ops/sec | < 10μs |
| **Distributed Write** | 10k+ ops/sec | < 50ms |
| **Distributed Read** (linearizable) | 50k+ ops/sec | < 10ms |
| **Distributed Read** (stale) | 200k+ ops/sec | < 1ms |

### Optimizations

- ✅ **LRU Cache Layer** - Hot data stays in memory
- ✅ **Async I/O** - Tokio runtime for concurrency
- ✅ **Connection Pooling** - Reused HTTP/S3 connections
- ✅ **Bincode Serialization** - Faster than JSON
- ✅ **Batch Operations** - Reduced overhead
- ✅ **Compression** - Gzip for S3 transfers

**Run benchmarks:**
```bash
cargo bench
```

---

## 🛠️ Configuration

### Basic Node Configuration

```toml
[node]
id = 1
address = "127.0.0.1"
data_dir = "./node-1"

[network]
listen_addr = "127.0.0.1:8001"
client_port = 8001     # HTTP API
raft_port = 9001       # Raft consensus

[storage]
segment_size = 67108864      # 64 MB
max_cache_size = 268435456   # 256 MB

[consensus]
election_timeout_min = 1500  # milliseconds
election_timeout_max = 3000
heartbeat_interval_ms = 300
```

### Environment Variables

Override config with `SCRIBE_` prefix:

```bash
export SCRIBE_NODE_ID=2
export SCRIBE_NETWORK_CLIENT_PORT=8002
export SCRIBE_NETWORK_RAFT_PORT=9002
cargo run --bin scribe-node
```

**📚 Full Guide:** [Configuration Documentation](docs/CONFIGURATION.md)

---

## 🚢 Deployment

### Docker Compose

```bash
docker-compose up -d
docker-compose logs -f
docker-compose down
```

### Systemd (Production)

```bash
# Install services
sudo cp scripts/systemd/*.service /etc/systemd/system/

# Start cluster
sudo systemctl start scribe-node-{1,2,3}

# Enable on boot
sudo systemctl enable scribe-node-{1,2,3}

# Check status
sudo systemctl status scribe-node-1
```

**📚 Full Guide:** [Deployment Documentation](docs/DEPLOYMENT.md)

---

## 🧪 Testing

### Run All Tests

```bash
cargo test
```

### Test Categories

```bash
cargo test storage        # Storage layer
cargo test consensus      # Raft consensus
cargo test http_tests     # HTTP API
cargo test cluster        # Multi-node
cargo test crypto         # Merkle proofs
```

### End-to-End Testing

```bash
# Python E2E suite
pip install -r tests/e2e/requirements.txt
python3 tests/e2e/cluster_e2e_test.py

# Shell script tests
./scripts/test-cluster.sh
```

### S3 Integration Tests

```bash
# Start MinIO first
docker-compose -f docker-compose-minio.yml up -d

# Run S3 tests
cargo test s3_ -- --ignored
cargo test segment_archival -- --ignored
cargo test data_tiering -- --ignored
```

**✅ Test Report:** [FINAL_TEST_REPORT.md](FINAL_TEST_REPORT.md) shows 10/10 passing tests.

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [FINAL_TEST_REPORT.md](FINAL_TEST_REPORT.md) | ✅ Complete test results with S3 |
| [CLUSTER_TESTING_GUIDE.md](CLUSTER_TESTING_GUIDE.md) | 🧪 How to test multi-node clusters |
| [S3_STORAGE.md](docs/S3_STORAGE.md) | ☁️ S3 integration guide |
| [ARCHIVAL_TIERING.md](docs/ARCHIVAL_TIERING.md) | 📦 Data tiering & archival |
| [CONFIGURATION.md](docs/CONFIGURATION.md) | ⚙️ Configuration reference |
| [DEPLOYMENT.md](docs/DEPLOYMENT.md) | 🚢 Production deployment |
| [TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) | 🔧 Common issues & solutions |

---

## 🌈 Examples

### Basic Usage

```bash
cargo run --example basic_usage     # Simple PUT/GET
cargo run --example cli_store       # Interactive CLI
cargo run --example config_demo     # Configuration
cargo run --example data_types      # Type system
```

### Rust Library Usage

```rust
use hyra_scribe_ledger::HyraScribeLedger;

fn main() -> anyhow::Result<()> {
    // Create storage
    let ledger = HyraScribeLedger::new("./data")?;
    
    // Store data
    ledger.put("user:alice", "Alice Smith")?;
    
    // Batch operations
    let mut batch = HyraScribeLedger::new_batch();
    batch.insert(b"key1", b"value1");
    batch.insert(b"key2", b"value2");
    ledger.apply_batch(batch)?;
    
    // Retrieve
    if let Some(data) = ledger.get("user:alice")? {
        println!("Found: {}", String::from_utf8_lossy(&data));
    }
    
    // Flush to disk
    ledger.flush()?;
    
    Ok(())
}
```

---

## 🎯 Use Cases

### ✅ Perfect For:

- **AI Training Data** - Immutable training datasets with verification
- **Blockchain Off-Chain Storage** - Scalable storage for blockchain data
- **Audit Trails** - Tamper-proof logging with cryptographic proofs
- **Data Archival** - Hot/cold tiering for long-term storage
- **Distributed Ledgers** - Multi-node consistency with Raft

### 💡 Example Scenarios:

- **AI Model Registry** - Store models with versioning and provenance
- **Transaction Logs** - Immutable financial transaction records
- **Document Storage** - Verified document storage with S3 backing
- **IoT Data** - Time-series data with automatic archival
- **Compliance Storage** - Regulatory data with audit trails

---

## 🔒 Security

> **Note**: Security modules (TLS, authentication, rate limiting) are implemented as library components. Full integration into HTTP server is planned.

- ✅ Merkle tree verification (active)
- ✅ SHA-256 cryptographic hashing (active)
- 🚧 TLS encryption (module ready)
- 🚧 API key authentication (module ready)
- 🚧 Role-based access control (module ready)
- 🚧 Rate limiting (module ready)
- ✅ Audit logging (active)

**📚 Security Guide:** [Security Documentation](docs/SECURITY.md)

---

## 📊 Monitoring

### Prometheus Integration

**Scrape configuration:**
```yaml
scrape_configs:
  - job_name: 'hyra-scribe-ledger'
    static_configs:
      - targets: ['localhost:8001', 'localhost:8002', 'localhost:8003']
    metrics_path: '/metrics/prometheus'
    scrape_interval: 15s
```

**Available Metrics:**
- Request latency histograms (p50, p95, p99)
- Throughput counters (GET/PUT/DELETE)
- Storage metrics (keys, size)
- Raft consensus state (term, index, leader)
- Error counters
- Cache hit rates

### Structured Logging

```bash
# Set log level
export RUST_LOG=hyra_scribe_ledger=debug

# Run with tracing
cargo run --bin scribe-node
```

---

## 🤝 Contributing

We welcome contributions! Here's how:

1. **Fork** the repository
2. **Create** a feature branch (`git checkout -b feature/amazing`)
3. **Make** your changes
4. **Test** thoroughly (`cargo test`)
5. **Format** code (`cargo fmt`)
6. **Lint** code (`cargo clippy`)
7. **Commit** (`git commit -am 'Add amazing feature'`)
8. **Push** (`git push origin feature/amazing`)
9. **Open** a Pull Request

### Code Quality Standards

- ✅ No hardcoded values in tests
- ✅ Comprehensive documentation
- ✅ Type safety (minimal `unwrap()`)
- ✅ Consistent formatting (`cargo fmt`)
- ✅ Clean code (`cargo clippy`)

---

## 📄 License

This project is licensed under the **MIT License**. See [LICENSE](LICENSE) for details.

---

## 🎉 Acknowledgments

Built with ❤️ using:
- [Rust](https://www.rust-lang.org/) - Memory safety & performance
- [OpenRaft](https://github.com/datafuselabs/openraft) - Modern async Raft
- [Tokio](https://tokio.rs/) - Async runtime
- [Axum](https://github.com/tokio-rs/axum) - HTTP framework
- [Sled](https://github.com/spacejam/sled) - Embedded database
- [AWS SDK](https://github.com/awslabs/aws-sdk-rust) - S3 integration

---

<div align="center">

### ⭐ Star us on GitHub!

**Made with 🔥 by the Hyra Team**

[Documentation](docs/) • [Report Bug](https://github.com/hyra-network/Scribe-Ledger/issues) • [Request Feature](https://github.com/hyra-network/Scribe-Ledger/issues)

</div>
