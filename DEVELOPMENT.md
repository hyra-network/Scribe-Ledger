# Hyra Scribe Ledger - Development Guide

## Project Overview

Hyra Scribe Ledger is a distributed, immutable, append-only key-value storage system designed to serve as the durable data layer for Hyra AI. The project currently implements a fully functional HTTP server with local storage capabilities.

## Current Implementation Status

### ✅ Phase 3: Distributed Consensus (COMPLETE)
- **Raft Consensus**: Production-ready multi-node cluster with leader election
- **Cluster Management**: Dynamic node membership with join/leave operations
- **Fault Tolerance**: Automatic failover and recovery from node failures
- **Manifest Synchronization**: Distributed metadata management across cluster
- **Network Transport**: HTTP-based inter-node communication
- **E2E Testing**: Comprehensive Python-based multi-node testing framework

### ✅ Phase 2: S3 Integration (COMPLETE)
- **S3 Cold Storage**: Complete S3-compatible storage with MinIO support
- **Hybrid Architecture**: Multi-tier storage (local cache + S3 backend)
- **Data Recovery**: Automatic data recovery from S3 on startup
- **Background Flush**: Asynchronous data migration to S3
- **Immutable Segments**: Read-only S3 objects ensuring data permanence

### ✅ Phase 1: Core Storage (COMPLETE)
- **HTTP Server**: Full REST API with PUT/GET endpoints
- **Local Storage**: Sled embedded database for persistent storage
- **Async Runtime**: Tokio-based asynchronous operations
- **Configuration System**: TOML + environment variable configuration
- **Error Handling**: Comprehensive error types and handling
- **Testing Suite**: 34 unit tests + consensus tests + E2E framework
- **Cryptographic Verification**: Complete Merkle tree implementation

### 🚧 Future Enhancements
- **HTTP Server Integration**: Complete REST API with consensus endpoints
- **Multi-Region Support**: Cross-region data replication
- **Advanced Security**: Authentication, authorization, encryption
- **Performance Optimization**: Log compaction, batch operations

## Build & Development

### Prerequisites
- **Rust 1.70+** (stable toolchain)
- **Git** for version control
- **AWS credentials** (for future S3 integration)

### Quick Start
```bash
# Clone the repository
git clone https://github.com/hyra-network/Scribe-Ledger.git
cd Scribe-Ledger

# Build the project
cargo build --release

# Run the server
cargo run

# Server will start on http://localhost:8080
```

### Development Workflow

#### Building
```bash
# Debug build (faster compilation)
cargo build

# Release build (optimized)
cargo build --release

# Check compilation without building
cargo check
```

#### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test modules
cargo test storage
cargo test lib::tests

# Run large data tests specifically
cargo test test_large -- --nocapture

# Run performance-critical tests
cargo test test_very_large_single_file_50mb -- --nocapture
cargo test test_multiple_large_files_concurrent -- --nocapture

# Run tests with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out html
```

#### Code Quality
```bash
# Format code
cargo fmt

# Run clippy lints
cargo clippy -- -D warnings

# Check for security vulnerabilities
cargo audit
```

### Single Node Testing

Test a single node deployment:

```bash
# Start single node
cargo run --bin scribe-node

# Store data
curl -X PUT http://localhost:8080/test-key \
  -H "Content-Type: application/octet-stream" \
  --data-binary "Hello, Scribe Ledger!"

# Retrieve data
curl http://localhost:8080/test-key

# Test with large payloads
dd if=/dev/zero bs=1M count=10 | curl -X PUT http://localhost:8080/large-data \
  --data-binary @-
```

### Multi-Node Cluster Testing

Deploy and test a 3-node distributed cluster:

```bash
# Terminal 1 - Start Node 1 (Leader)
cargo run --bin scribe-node -- --config config-node1.toml

# Terminal 2 - Start Node 2 (Follower)  
cargo run --bin scribe-node -- --config config-node2.toml

# Terminal 3 - Start Node 3 (Follower)
cargo run --bin scribe-node -- --config config-node3.toml

# Test cluster health
curl http://localhost:8001/health  # Node 1
curl http://localhost:8002/health  # Node 2
curl http://localhost:8003/health  # Node 3

# Test data replication
curl -X PUT http://localhost:8001/cluster-test \
  --data-binary "Distributed data"

# Verify replication across nodes
curl http://localhost:8002/cluster-test  # Should return same data
curl http://localhost:8003/cluster-test  # Should return same data
```

### E2E Testing Framework

Run comprehensive end-to-end tests:

```bash
# Install Python dependencies
pip3 install requests asyncio

# Run full E2E test suite
python3 e2e_test.py

# The framework will:
# 1. Start MinIO server for S3 testing
# 2. Launch 3-node cluster automatically
# 3. Run comprehensive test scenarios:
#    - Basic connectivity testing
#    - Data replication validation  
#    - Leader election verification
#    - Node failure and recovery
#    - Concurrent write testing
# 4. Generate detailed test report
# 5. Clean up all processes
```

### Configuration

The server uses `config.toml` for configuration. Default configuration:

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
buffer_size = 67108864  # 64MB
segment_size_limit = 1073741824  # 1GB

[consensus]
peers = []
election_timeout_ms = 5000
heartbeat_interval_ms = 1000
```

##### Multi-Node Cluster Configuration

Each node requires its own configuration file with unique settings:

**config-node1.toml** (Leader):
```toml
[node]
id = 1
address = "127.0.0.1:8001"
data_dir = "./data/node1"

[consensus]
election_timeout = 10
heartbeat_timeout = 3
cluster_members = [
    { id = 1, address = "127.0.0.1:8001" },
    { id = 2, address = "127.0.0.1:8002" },
    { id = 3, address = "127.0.0.1:8003" }
]

[storage]
s3_bucket = "scribe-ledger-cluster"
s3_endpoint = "http://localhost:9000"  # MinIO
```

**config-node2.toml** and **config-node3.toml** follow similar patterns with different IDs, addresses, and data directories.

##### Configuration Override
```bash
# Use specific node config
cargo run --bin scribe-node -- --config config-node1.toml

# Override via environment (applies to all nodes)
SCRIBE_S3_ENDPOINT=http://production-s3:9000 cargo run --bin scribe-node
```

### S3-Compatible Storage with MinIO

For development, we use MinIO as an S3-compatible storage backend. This allows us to develop and test S3 integration features locally without requiring AWS credentials.

#### MinIO Setup
```bash
# Start MinIO and create buckets
./dev.sh start-minio

# Check MinIO status
./dev.sh minio-status

# Stop MinIO
./dev.sh stop-minio

# View MinIO logs
./dev.sh minio-logs

# Reset all MinIO data (destructive)
./dev.sh minio-reset
```

#### MinIO Access
- **Console UI**: http://localhost:9001
- **S3 API Endpoint**: http://localhost:9000
- **Username**: `scribe-admin`
- **Password**: `scribe-password-123`

#### Development with MinIO
```bash
# Run Scribe Ledger with MinIO configuration
./dev.sh run-dev

# This automatically:
# 1. Starts MinIO if not running
# 2. Uses config-dev.toml with MinIO settings
# 3. Enables debug logging
```

#### MinIO Buckets
The setup automatically creates these buckets:
- `scribe-ledger-dev` - Development bucket
- `scribe-ledger-test` - Testing bucket  
- `scribe-ledger-prod` - Production-like bucket

### Configuration Management

Scribe Ledger supports flexible configuration through files and environment variables. Environment variables have the highest priority and will override file-based configuration.

#### Supported Environment Variables

**S3/MinIO Configuration:**
```bash
export SCRIBE_S3_BUCKET="my-custom-bucket"
export SCRIBE_S3_REGION="us-west-2"
export SCRIBE_S3_ENDPOINT="http://localhost:9000"  # For MinIO
export SCRIBE_S3_ACCESS_KEY="your-access-key"
export SCRIBE_S3_SECRET_KEY="your-secret-key"
export SCRIBE_S3_PATH_STYLE="true"  # Required for MinIO
```

**Node Configuration:**
```bash
export SCRIBE_NODE_ID="1"
export SCRIBE_NODE_ADDRESS="127.0.0.1:8001"  
export SCRIBE_DATA_DIR="/custom/data/path"
```

**Network Configuration:**
```bash
export SCRIBE_LISTEN_ADDRESS="127.0.0.1:8001"
export SCRIBE_MAX_CONNECTIONS="100"
export SCRIBE_REQUEST_TIMEOUT="30"
```

**Consensus Configuration:**
```bash
export SCRIBE_ELECTION_TIMEOUT="10"
export SCRIBE_HEARTBEAT_TIMEOUT="3"
export SCRIBE_MAX_LOG_ENTRIES="1000"
```

#### Configuration Priority (Highest to Lowest)
1. **Environment Variables** - `SCRIBE_*` prefixed variables
2. **Configuration Files** - `config.toml`, `config-dev.toml`
3. **Built-in Defaults** - Fallback values

#### Example: Custom MinIO Setup
```bash
# Set custom MinIO configuration
export SCRIBE_S3_ENDPOINT="http://my-minio:9000"
export SCRIBE_S3_ACCESS_KEY="my-admin"
export SCRIBE_S3_SECRET_KEY="my-password"
export SCRIBE_S3_BUCKET="my-test-bucket"
export SCRIBE_S3_PATH_STYLE="true"

# Run with environment overrides
cargo run --bin scribe-node
```

## Architecture & Implementation Details

### Distributed Consensus Architecture

The current implementation provides a complete distributed system with these components:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Node 1        │    │   Node 2        │    │   Node 3        │
│   (Leader)      │    │   (Follower)    │    │   (Follower)    │
├─────────────────┤    ├─────────────────┤    ├─────────────────┤
│ HTTP API Server │    │ HTTP API Server │    │ HTTP API Server │
│ ConsensusNode   │◄──►│ ConsensusNode   │◄──►│ ConsensusNode   │
│ ScribeLedger    │    │ ScribeLedger    │    │ ScribeLedger    │
│ ManifestManager │    │ ManifestManager │    │ ManifestManager │
├─────────────────┤    ├─────────────────┤    ├─────────────────┤
│ Local Storage   │    │ Local Storage   │    │ Local Storage   │
│ (Sled)          │    │ (Sled)          │    │ (Sled)          │
└─────────┬───────┘    └─────────┬───────┘    └─────────┬───────┘
          │                      │                      │
          └──────────────────────┼──────────────────────┘
                                 │
                        ┌─────────▼───────┐
                        │   S3 Storage    │
                        │   (MinIO)       │  
                        │ Immutable Logs  │
                        └─────────────────┘
```

### Consensus Layer Architecture

#### Raft Implementation
- **Leader Election**: Automatic leader selection using Raft algorithm
- **Log Replication**: Consistent state replication across all nodes
- **Fault Tolerance**: Cluster remains operational with majority of nodes
- **Network Transport**: HTTP-based inter-node communication

#### Cluster Management
- **Dynamic Membership**: Add/remove nodes from active cluster
- **Health Monitoring**: Continuous monitoring of node availability
- **Failover**: Automatic leader re-election on failure
- **State Synchronization**: Consistent manifest updates across cluster

### Key Components

#### 1. HTTP Server (`lib.rs`)
- **Framework**: Axum for high-performance async HTTP
- **Endpoints**: 
  - `PUT /:key` - Store data
  - `GET /:key` - Retrieve data
- **State Management**: Arc-wrapped ScribeLedger for shared state
- **Error Handling**: Proper HTTP status code mapping

#### 2. Storage Layer (`storage/mod.rs`)
- **Engine**: Sled embedded database
- **Operations**: Async put/get with proper error handling
- **Persistence**: Automatic flush for durability
- **Performance**: Optimized for high-throughput operations

#### 3. Configuration (`config.rs`)
- **Format**: TOML-based configuration
- **Environment**: Support for environment variable overrides
- **Validation**: Configuration validation on startup
- **Defaults**: Sensible defaults for development

#### 4. Error Handling (`error.rs`)
- **Types**: Comprehensive error types for all operations
- **Propagation**: Proper error propagation through Result types
- **HTTP Mapping**: Clean mapping to HTTP status codes
- **Debugging**: Detailed error messages for development

### Testing Strategy

The project includes comprehensive testing:

```bash
# Unit tests for core logic
cargo test lib::tests

# Storage persistence tests
cargo test storage

# Configuration tests
cargo test config

# Integration tests
cargo test --test integration
```

#### Test Coverage Areas
- ✅ PUT/GET operations
- ✅ Persistence across restarts
- ✅ Large data handling (up to 50MB payloads)
- ✅ Unicode key/value support
- ✅ Empty value handling
- ✅ Nonexistent key behavior
- ✅ Configuration validation
- ✅ Error scenarios
- ✅ Large text data (5MB Lorem Ipsum)
- ✅ Large binary data (10MB with patterns)
- ✅ Mixed text/binary data (25MB)
- ✅ JSON-structured data (15MB AI model outputs)
- ✅ Concurrent large file operations
- ✅ Very large single files (50MB with performance metrics)

## S3 Integration Implementation

### Hybrid Storage Architecture

Scribe Ledger implements a **multi-tier hybrid storage system**:

#### Hot Tier (Local Storage)
- **Technology**: Sled embedded database
- **Purpose**: High-speed local cache and write buffer
- **Characteristics**: 
  - Immediate write acknowledgment
  - Fast read access for recent data
  - Write-ahead logging for crash safety

#### Cold Tier (S3 Storage)
- **Technology**: S3-compatible object storage (AWS S3 or MinIO)
- **Purpose**: Durable, immutable long-term storage
- **Characteristics**:
  - Append-only immutable segments
  - High durability (11 nines)
  - Cost-effective for large datasets

### S3 Integration Features

#### 1. Immutable Segments
```rust
// Each S3 object is a timestamped, immutable segment
segments/{segment_id}
// Format: key1:base64_value1\nkey2:base64_value2\n...
```

#### 2. Write Path
1. **Local Write**: Data stored in Sled database immediately
2. **Acknowledgment**: Client receives immediate response
3. **Background Flush**: Periodic flush to S3 based on:
   - Size threshold (10MB)
   - Time threshold (30 seconds)
4. **Immutable Storage**: S3 objects become readonly after creation

#### 3. Read Path
1. **Local Cache Check**: Query Sled database first
2. **S3 Fallback**: Search S3 segments if not found locally
3. **Cache Population**: Store retrieved data locally for future reads
4. **Newest First**: Search segments in reverse chronological order

#### 4. Data Recovery
- **Startup Recovery**: Automatically recover from S3 on node restart
- **No Overwrites**: Preserve local data, only recover missing keys
- **Complete Restoration**: Full data set can be rebuilt from S3

### S3 Testing Suite

#### Integration Tests (MinIO Required)
```bash
# Full workflow test
cargo test test_s3_integration_full_workflow -- --ignored

# Data recovery test
cargo test test_s3_data_recovery -- --ignored

# Read-through cache test
cargo test test_s3_read_through_cache -- --ignored

# Immutable segments test
cargo test test_s3_immutable_segments -- --ignored
```

#### Test Coverage
- ✅ Complete write → flush → read workflow
- ✅ Data persistence across node restarts
- ✅ Read-through cache population
- ✅ Immutable segment verification
- ✅ Environment variable configuration
- ✅ MinIO compatibility

### Performance Metrics

#### Achieved Performance
- **Local Write**: Immediate acknowledgment (< 1ms)
- **Local Read**: Sub-millisecond access from cache
- **50MB Storage**: 318ms local + background S3 flush
- **50MB Retrieval**: 29ms from local cache
- **S3 Recovery**: Complete dataset restoration on startup
- **Background Flush**: Non-blocking, automatic sync

### Development Roadmap

#### Phase 2: S3 Integration ✅ COMPLETED
- [x] Set up MinIO for local S3-compatible development
- [x] Configure Docker Compose for development environment
- [x] Create development configuration with MinIO endpoints
- [x] Implement complete S3 storage backend in Rust code
- [x] Add segment-based immutable storage architecture
- [x] Implement background flush operations
- [x] Add data recovery from S3 on startup
- [x] Implement read-through cache pattern
- [x] Ensure S3 objects are readonly/immutable
- [x] Add comprehensive S3 integration tests
- [x] Support for append-only semantics


#### Phase 3: Consensus Layer ✅ COMPLETED
- [x] Implement Raft consensus cluster
- [x] Add manifest management
- [x] Implement distributed metadata synchronization
- [x] Add cluster membership management

#### Phase 4: Cryptographic Verification ✅ COMPLETED
- [x] Implement complete Merkle tree construction
- [x] Add cryptographic proof generation and verification
- [x] Implement comprehensive Merkle proof validation
- [x] Add edge case handling for Merkle trees
- [x] Complete test suite for cryptographic functions

#### Phase 5: Production Features
- [ ] Add monitoring and metrics
- [ ] Implement proper logging
- [ ] Add health check endpoints
- [ ] Performance optimization and benchmarking
- [ ] Security audit and hardening

### Development Best Practices

#### Code Style
- Follow Rust idioms and conventions
- Use `rustfmt` for consistent formatting
- Address all `clippy` warnings
- Write comprehensive documentation
- Include unit tests for new features

#### Git Workflow
```bash
# Create feature branch
git checkout -b feature/new-feature

# Make changes with clear commits
git commit -m "feat: implement new feature"

# Push and create PR
git push origin feature/new-feature
```

#### Commit Message Format
- `feat:` - New features
- `fix:` - Bug fixes
- `docs:` - Documentation updates
- `test:` - Test additions/modifications
- `refactor:` - Code refactoring
- `perf:` - Performance improvements

### Performance Considerations

#### Current Performance
- **Throughput**: Handles high-concurrency HTTP requests
- **Storage**: Sled provides excellent performance for local storage
- **Memory**: Efficient memory usage with streaming operations
- **Latency**: Sub-millisecond response times for cached data
- **Large Data**: Successfully tested up to 50MB single files
- **Concurrent Operations**: Handles multiple 5MB files simultaneously

#### Performance Test Results
Based on automated test suite:

| Test Case | File Size | Store Time | Retrieve Time | Notes |
|-----------|-----------|------------|---------------|-------|
| Text Data | 5MB | ~50ms | ~10ms | Lorem Ipsum pattern |
| Binary Data | 10MB | ~100ms | ~15ms | Repeating byte pattern |
| Mixed Data | 25MB | ~250ms | ~25ms | Text + binary chunks |
| JSON Data | 15MB | ~150ms | ~20ms | AI model output format |
| Concurrent 3x | 5MB each | ~200ms | ~30ms | Parallel operations |
| Very Large | 50MB | ~275ms | ~18ms | Single large file |

#### Optimization Areas
- [ ] Connection pooling for S3 operations
- [ ] Compression for large payloads
- [ ] Caching layer for frequently accessed data
- [ ] Batch operations for improved throughput
- [ ] Memory-mapped file I/O for very large files
- [ ] Background compression for cold storage

---

## Troubleshooting

### Common Issues

#### Port Already in Use
```bash
# Check what's using port 8080
lsof -i :8080

# Kill the process or use different port
SCRIBE_NETWORK_CLIENT_PORT=8081 cargo run
```

#### Database Corruption
```bash
# Remove corrupted database
rm -rf ./data/scribe.db

# Restart server (will create new database)
cargo run
```

#### Configuration Issues
```bash
# Validate configuration
cargo run -- --validate-config

# Use default configuration
rm config.toml && cargo run
```

### Debug Mode
```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run with trace logging
RUST_LOG=trace cargo run
```

The project is production-ready for local storage use cases and provides a solid foundation for distributed features.