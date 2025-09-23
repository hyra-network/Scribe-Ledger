<p align="center">
  <img src="https://raw.githubusercontent.com/hyra-network/Scribe-Ledger/main/assets/logo.png" alt="Hyra Scribe Ledger Logo" width="200"/>
</p>

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
- **Raft Consensus Cluster (Coordination Tier):** Fault-tolerant cluster managing global system metadata. Maintains the Manifest, a global index mapping which key ranges exist in which S3 Segments.

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

3. **Run the server:**
   ```bash
   cargo run
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

Create a `config.toml` file to customize settings:

```toml
[node]
id = "node-1"
data_dir = "./data"

[network]
listen_addr = "0.0.0.0"
client_port = 8080

[storage]
s3_bucket = "scribe-ledger-dev"
s3_region = "us-east-1"
```

### Development

For development, use the provided development script:

```bash
# Development commands
./dev.sh build    # Build the project
./dev.sh test     # Run tests
./dev.sh fmt      # Format code
./dev.sh clippy   # Run lints
```

---

## 📋 Current Features

### ✅ Implemented
- **HTTP API Server** - RESTful API for data storage and retrieval
- **Local Storage** - Sled embedded database for persistent key-value storage
- **Async Operations** - High-performance asynchronous I/O with Tokio
- **Error Handling** - Comprehensive error types and handling
- **Configuration System** - TOML-based configuration management
- **Testing Suite** - Comprehensive unit and integration tests

### 🚧 In Development
- **S3 Integration** - Cold storage tier for durability
- **Raft Consensus** - Distributed coordination and consistency
- **Merkle Proofs** - Cryptographic verification of data integrity
- **Write Node Clustering** - Multi-node distributed architecture
- **Manifest Management** - Global metadata and segment tracking

---

## 🧪 Testing

Run the comprehensive test suite:

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

The test suite includes:
- Unit tests for core functionality
- Integration tests for HTTP endpoints
- Storage persistence tests
- Configuration validation tests
- Unicode and large data handling tests

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