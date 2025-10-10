# Simple Scribe Ledger - Development Roadmap

## Project Overview

This document outlines the development roadmap for Simple Scribe Ledger, a distributed, immutable, append-only key-value storage system inspired by [Hyra Scribe Ledger](https://github.com/hyra-network/Scribe-Ledger). Our implementation uses **OpenRaft** for optimized consensus performance while maintaining the same architectural principles.

## 🚀 Vision

Simple Scribe Ledger provides a durable data layer for distributed applications. The system is designed for:

- **Durability:** Data, once committed, is considered permanent.
- **Immutability:** Data cannot be altered or deleted, only appended.
- **Verifiability:** Cryptographic proofs ensure data integrity.
- **Performance:** Optimized throughput with OpenRaft consensus.

### Key Implementation Differences
- **Consensus Library**: Using `openraft` (modern, async-first) instead of `raft-rs` for better performance
- **Optimized Architecture**: Focused on high-throughput distributed storage operations
- **Modern Rust Patterns**: Leveraging async/await patterns with Tokio for maximum efficiency

---

## Development Phases

Each phase is broken down into small, focused tasks that can be completed within a single Copilot agent request.

---

## Phase 1: Project Foundation & Configuration (2-3 tasks)

**Goal**: Set up project structure, configuration system, and error handling for distributed operations.

### Task 1.1: Project Structure and Dependencies ✅
- [x] Add openraft dependency to Cargo.toml (version ~0.9 or latest stable)
- [x] Add required dependencies: tokio, serde, serde_json, anyhow, thiserror, tracing
- [x] Remove or update conflicting dependencies if any
- [x] Create directory structure: src/{consensus/, storage/, network/, manifest/, config/}
- [x] Update .gitignore for distributed node data directories

**Deliverables**: Updated Cargo.toml with all dependencies, directory structure created  
**Status**: ✅ Complete

---

### Task 1.2: Configuration System ✅
- [x] Create src/config.rs with Config struct supporting:
  - Node configuration (id, address, data_dir)
  - Network configuration (listen_addr, client_port, raft_port)
  - Storage configuration (segment_size, max_cache_size)
  - Consensus configuration (election_timeout, heartbeat_interval)
- [x] Implement TOML file parsing
- [x] Add environment variable override support (SCRIBE_* prefix)
- [x] Create example config files: config.toml, config-node1.toml, config-node2.toml, config-node3.toml
- [x] Add configuration validation logic

**Deliverables**: Fully functional configuration system with TOML and env var support  
**Status**: ✅ Complete

---

### Task 1.3: Error Handling and Type System ✅
- [x] Create src/error.rs with ScribeError enum covering:
  - Storage errors
  - Consensus errors  
  - Network errors
  - Configuration errors
  - Serialization errors
- [x] Implement From traits for converting third-party errors
- [x] Create src/types.rs with common types:
  - NodeId, SegmentId, ManifestId
  - Key and Value type aliases
  - Request/Response types
- [x] Add comprehensive error context using anyhow

**Deliverables**: Robust error handling system and type definitions  
**Status**: ✅ Complete

---

## Phase 2: Storage Layer (3-4 tasks)

**Goal**: Implement local storage with Sled and prepare for multi-tier storage architecture.

### Task 2.1: Enhanced Storage Backend ✅
- [x] Create src/storage/mod.rs with StorageBackend trait:
  - async fn put(&self, key: Key, value: Value) -> Result<()>
  - async fn get(&self, key: &Key) -> Result<Option<Value>>
  - async fn delete(&self, key: &Key) -> Result<()>
  - async fn flush(&self) -> Result<()>
  - async fn snapshot(&self) -> Result<HashMap<Key, Value>>
- [x] Implement SledStorage struct with StorageBackend trait
- [x] Add async wrappers around sled operations using tokio::task::spawn_blocking
- [x] Implement proper error handling and conversion

**Deliverables**: Storage abstraction layer with Sled implementation  
**Status**: ✅ Complete

---

### Task 2.2: Storage Tests and Benchmarks ✅
- [x] Create tests/storage_tests.rs with comprehensive tests:
  - Basic put/get operations
  - Large data handling (10MB+)
  - Concurrent operations
  - Persistence across restarts
  - Error cases
- [x] Add benchmarks in benches/storage_benchmark.rs
- [x] Test edge cases (empty keys, empty values, Unicode)
- [x] Verify async behavior is correct

**Deliverables**: Complete test coverage for storage layer  
**Status**: ✅ Complete

---

### Task 2.3: Segment-based Storage Preparation ✅
- [x] Create src/storage/segment.rs with Segment struct:
  - timestamp: u64
  - data: HashMap<Key, Value>
  - size: usize
  - segment_id: SegmentId
- [x] Implement segment serialization/deserialization
- [x] Add PendingSegment struct for buffering writes
- [x] Create segment manager for tracking active/flushed segments
- [x] Add segment size threshold logic

**Deliverables**: Segment data structures ready for S3 integration (Phase 6)  
**Status**: ✅ Complete

---

## Phase 3: OpenRaft Consensus Layer (5-6 tasks)

**Goal**: Implement distributed consensus using OpenRaft for cluster coordination.

### Task 3.1: OpenRaft State Machine ✅
- [x] Create src/consensus/state_machine.rs implementing openraft::RaftStateMachine
- [x] Define AppData type for log entries (put/delete operations)
- [x] Define AppDataResponse type for operation results
- [x] Implement apply() method to apply committed entries to storage
- [x] Implement snapshot() for state machine snapshots
- [x] Add restore_snapshot() for recovering from snapshots

**Deliverables**: OpenRaft state machine implementation  
**Status**: ✅ Complete

---

### Task 3.2: OpenRaft Storage Backend ✅
- [x] Create src/consensus/storage.rs implementing openraft::RaftStorage
- [x] Implement log storage (append, get, delete entries)
- [x] Implement hard state storage (term, vote)
- [x] Implement snapshot storage
- [x] Use Sled for persistent raft storage
- [x] Add proper error handling and conversions

**Deliverables**: Persistent storage for Raft log and metadata  
**Status**: ✅ Complete

---

### Task 3.3: OpenRaft Network Layer ✅
- [x] Create src/consensus/network.rs implementing openraft::RaftNetwork
- [x] Implement send_append_entries RPC
- [x] Implement send_vote RPC
- [x] Implement send_install_snapshot RPC
- [x] Use tokio TcpStream for network communication
- [x] Add retry logic and timeout handling
- [x] Implement connection pooling

**Deliverables**: Network layer for Raft RPCs  
**Status**: ✅ Complete

---

### Task 3.4: Consensus Node Integration ✅
- [x] Create src/consensus/mod.rs with ConsensusNode struct
- [x] Initialize OpenRaft instance with state machine, storage, network
- [x] Implement cluster membership management (add_learner, change_membership)
- [x] Add leader/follower role tracking
- [x] Implement graceful shutdown
- [x] Add health check methods

**Deliverables**: Fully integrated OpenRaft node  
**Status**: ✅ Complete

---

### Task 3.5: Consensus Tests ✅
- [x] Create tests/consensus_tests.rs:
  - Single node startup
  - Leader election in 3-node cluster
  - Log replication
  - Follower failure and recovery
  - Leader failure and re-election
- [x] Add test utilities for multi-node setup
- [x] Test membership changes
- [x] Verify state machine consistency

**Deliverables**: Comprehensive consensus layer tests  
**Status**: ✅ Complete

---

## Phase 4: Manifest Management (2-3 tasks)

**Goal**: Implement distributed metadata management using consensus.

### Task 4.1: Manifest Data Structures ✅
- [x] Create src/manifest/mod.rs with:
  - ManifestEntry struct (segment_id, timestamp, merkle_root, size)
  - ClusterManifest struct (version, entries: Vec<ManifestEntry>)
  - ClusterNode struct (id, address, state, last_heartbeat)
- [x] Implement serialization/deserialization
- [x] Add manifest versioning logic
- [x] Create manifest diff/merge utilities

**Deliverables**: Manifest data structures and utilities  
**Status**: ✅ Complete

---

### Task 4.2: Manifest Manager ✅
- [x] Create ManifestManager struct
- [x] Implement manifest updates through consensus (propose to Raft)
- [x] Add manifest query methods (get_latest, get_segments)
- [x] Implement manifest synchronization across nodes
- [x] Add conflict resolution logic
- [x] Cache manifest locally for performance

**Deliverables**: Manifest management with consensus backing  
**Status**: ✅ Complete

---

### Task 4.3: Manifest Tests ✅
- [x] Test manifest updates in single node
- [x] Test manifest replication across cluster
- [x] Test manifest consistency after node failure
- [x] Test concurrent manifest updates
- [x] Verify manifest versioning

**Deliverables**: Complete manifest tests  
**Status**: ✅ Complete

---

## Phase 5: HTTP API Server (3-4 tasks)

**Goal**: Implement REST API for client interactions.

**Important Note on Data Immutability**: In production deployments using distributed consensus, data stored in the ledger is designed to be immutable and permanent. The DELETE operation is provided for development and testing purposes but should be used with caution in production environments. In a true distributed ledger, all operations are append-only and data is never actually deleted, only marked as deleted in newer log entries.

### Task 5.1: Basic HTTP Server ✅
- [x] Create src/lib.rs with main ScribeLedger struct
- [x] Set up Axum router with routes:
  - PUT /:key - Store data
  - GET /:key - Retrieve data
  - DELETE /:key - Remove data (if supported)
  - GET /health - Health check
  - GET /metrics - Basic metrics
- [x] Implement request handlers
- [x] Add proper error to HTTP status code mapping
- [x] Support binary data (Content-Type: application/octet-stream)

**Deliverables**: Functional HTTP API server  
**Status**: ✅ Complete

---

### Task 5.2: Cluster API Endpoints ✅
- [x] Add cluster management endpoints:
  - POST /cluster/join - Join cluster
  - POST /cluster/leave - Leave cluster
  - GET /cluster/status - Cluster status
  - GET /cluster/members - List members
  - GET /cluster/leader - Current leader
- [x] Implement request forwarding to leader (stub for standalone mode)
- [x] Add cluster metrics endpoint
- [x] Handle raft role changes (stub for standalone mode)

**Deliverables**: Cluster management API  
**Status**: ✅ Complete - Stub implementations ready for full distributed mode

**Notes**: Current implementation provides stub endpoints that work in standalone mode. When full distributed consensus is integrated (Tasks 7.x and 8.x), these endpoints will be connected to the actual OpenRaft consensus layer.

---

### Task 5.3: HTTP API Tests ✅
- [x] Create tests/http_tests.rs with comprehensive test coverage:
  - Test all CRUD endpoints (PUT, GET, DELETE)
  - Test cluster endpoints (join, leave, status, members, leader)
  - Test error responses
  - Test concurrent requests
  - Test large payloads (1MB+)
  - Test binary data support
  - Test special characters in keys
  - Test multiple overwrites
- [x] Add integration tests with real HTTP clients (reqwest)
- [x] Test leader forwarding (stub for standalone mode)

**Deliverables**: Complete HTTP API test coverage  
**Status**: ✅ Complete - 19 tests passing

**Test Coverage**:
- 13 tests for basic CRUD operations
- 6 tests for cluster management endpoints
- All tests use real HTTP client (reqwest)
- Tests run in parallel with isolated test servers

---

## Phase 6: S3 Cold Storage Integration (2-3 tasks)

**Goal**: Implement S3-compatible object storage for cold data and segment archival. This phase prepares the foundation for production-ready multi-tier storage architecture.

### Task 6.1: S3 Storage Backend ✅
- [x] Integrate S3 storage backend (AWS SDK or rusoto)
- [x] Add S3 configuration (bucket, region, credentials)
- [x] Support MinIO for local development and testing
- [x] Implement S3 connection pooling and retry logic
- [x] Add proper error handling for S3 operations

**Deliverables**: S3 storage backend with configuration support  
**Status**: ✅ Complete

**Implementation Details**:
- Added AWS SDK S3 dependencies (aws-sdk-s3, aws-config)
- Created S3StorageConfig in config module with support for MinIO
- Implemented S3Storage backend in src/storage/s3.rs with:
  - Async operations for put/get/delete segments
  - Automatic retry logic with exponential backoff
  - Connection pooling via AWS SDK
  - Path-style addressing for MinIO compatibility
  - Comprehensive error handling
- Added unit tests for S3 operations
- Added integration tests (marked as ignored, require MinIO/S3)
- Added S3 storage benchmark
- Updated GitHub workflow to include S3 tests

---

### Task 6.2: Segment Archival to S3 ✅
- [x] Implement segment flushing to S3
- [x] Add read-through from S3 for cold data
- [x] Support segment metadata storage in S3
- [x] Implement segment lifecycle management
- [x] Add compression for S3-stored segments

**Deliverables**: Segment archival and retrieval from S3  
**Status**: ✅ Complete

**Implementation Details**:
- Created ArchivalManager in src/storage/archival.rs
- Implemented automatic segment archival with gzip compression
- Added read-through caching for frequently accessed segments
- Segment metadata stored alongside data in S3
- Full lifecycle management: archive, retrieve, delete
- Configurable compression levels (0-9)
- Added 10 comprehensive integration tests

---

### Task 6.3: Data Tiering and S3 Tests ✅
- [x] Implement automatic data tiering based on age/access patterns
- [x] Add tiering policy configuration
- [x] Create comprehensive S3 integration tests
- [x] Test MinIO compatibility
- [x] Add performance benchmarks for S3 operations
- [x] Test error recovery and retry scenarios

**Deliverables**: Data tiering system with complete test coverage  
**Status**: ✅ Complete

**Implementation Details**:
- Implemented TieringPolicy with age-based archival thresholds
- Automatic background archival task with configurable intervals
- Cache invalidation and segment lifecycle management
- 12 comprehensive integration tests for data tiering
- MinIO compatibility testing with path-style addressing
- Error recovery and concurrent archival testing
- Metadata caching for performance optimization

---

## Phase 7: Node Discovery & Cluster Formation (2-3 tasks)

**Goal**: Implement automatic cluster discovery and dynamic membership.

### Task 7.1: Discovery Service ✅
- [x] Create src/discovery.rs with DiscoveryService
- [x] Implement UDP broadcast for node discovery
- [x] Add peer list management
- [x] Implement heartbeat protocol
- [x] Add failure detection logic
- [x] Support configurable discovery endpoints

**Deliverables**: Node discovery service

---

### Task 7.2: Cluster Initialization ✅
- [x] Implement bootstrap logic for first node
- [x] Add automatic cluster joining for new nodes
- [x] Support manual cluster seeding via config
- [x] Implement leader discovery
- [x] Add join request/response handling
- [x] Handle network partitions gracefully

**Deliverables**: Automatic cluster formation

---

### Task 7.3: Discovery Tests ✅
- [x] Test single node bootstrap
- [x] Test 3-node cluster auto-discovery
- [x] Test node joining running cluster
- [x] Test failure detection
- [x] Test network partition scenarios

**Deliverables**: Discovery and cluster formation tests

---

## Phase 8: Write Path & Data Replication (3-4 tasks)

**Goal**: Implement distributed write path with consensus.

### Task 8.1: Write Request Handling ✅
- [x] Create write request flow:
  - Client sends PUT request to any node
  - Node forwards to leader if not leader
  - Leader proposes write to Raft
  - Wait for consensus
  - Apply to local storage
  - Return success to client
- [x] Implement request forwarding logic
- [x] Add timeout handling
- [x] Support batching of writes

**Deliverables**: Distributed write path  
**Status**: ✅ Complete

---

### Task 8.2: Read Request Handling ✅
- [x] Implement read flow:
  - Check local storage first
  - Support linearizable reads (query leader)
  - Support stale reads from followers (optional)
  - Cache frequently accessed data
- [x] Add read consistency options
- [x] Implement read-through caching

**Deliverables**: Distributed read path  
**Status**: ✅ Complete

---

### Task 8.3: Data Consistency Tests ✅
- [x] Test write-then-read consistency
- [x] Test replication across all nodes
- [x] Test read-your-writes consistency
- [x] Test network partition scenarios
- [x] Verify data durability after crashes

**Deliverables**: Consistency and replication tests  
**Status**: ✅ Complete

---

## Phase 9: Binary & Node Implementation (2-3 tasks)

**Goal**: Create runnable node binary and deployment scripts.

### Task 9.1: Node Binary ✅
- [x] Create src/bin/scribe-node.rs
- [x] Implement CLI argument parsing with clap:
  - --config <path> - Config file path
  - --node-id <id> - Override node ID
  - --bootstrap - Bootstrap new cluster
  - --log-level <level> - Log level (trace, debug, info, warn, error)
- [x] Add graceful shutdown handling (SIGTERM, SIGINT)
- [x] Implement logging with tracing/tracing-subscriber
- [x] Add startup banner and version info
- [x] Create comprehensive test suite (12 tests in tests/node_binary_tests.rs)
- [x] Add tests to GitHub workflow

**Deliverables**: Runnable node binary  
**Status**: ✅ Complete

---

### Task 9.2: Multi-Node Testing Scripts ✅
- [x] Create scripts/start-cluster.sh for starting 3-node cluster
- [x] Create scripts/stop-cluster.sh for clean shutdown
- [x] Add scripts/test-cluster.sh for basic cluster testing
- [x] Create example systemd service files
- [x] Add Docker support (Dockerfile)

**Deliverables**: Deployment and testing scripts  
**Status**: ✅ Complete

---

### Task 9.3: End-to-End Tests ✅
- [x] Create tests/e2e/ directory
- [x] Write Python E2E test script:
  - Start 3-node cluster
  - Test data replication
  - Test leader election
  - Test node failure recovery
  - Test concurrent operations
- [x] Add performance benchmarks
- [x] Create stress tests
- [x] Add E2E infrastructure tests to GitHub workflow

**Deliverables**: Complete E2E test suite  
**Status**: ✅ Complete

---

## Phase 10: Cryptographic Verification (2-3 tasks)

**Goal**: Add Merkle tree support for data verification.

### Task 10.1: Merkle Tree Implementation ✅
- [x] Create src/crypto/mod.rs with MerkleTree struct
- [x] Implement tree construction from key-value pairs
- [x] Add proof generation (get_proof for specific key)
- [x] Implement proof verification
- [x] Use SHA-256 for hashing
- [x] Handle edge cases (empty tree, single element)

**Deliverables**: Merkle tree implementation  
**Status**: ✅ Complete

---

### Task 10.2: Manifest Merkle Root Integration ✅
- [x] Add merkle_root field to ManifestEntry
- [x] Compute Merkle root during segment creation
- [x] Store Merkle root in manifest
- [x] Implement verification API endpoint
- [x] Add GET /verify/:key endpoint

**Deliverables**: Merkle root in manifest  
**Status**: ✅ Complete

---

### Task 10.3: Crypto Tests ✅
- [x] Test Merkle tree construction
- [x] Test proof generation and verification
- [x] Test with various data sizes
- [x] Test edge cases
- [x] Benchmark performance

**Deliverables**: Cryptographic verification tests  
**Status**: ✅ Complete

---

## Phase 11: Advanced Features & Optimization (4-5 tasks)

**Goal**: Add production-ready features and optimizations.

### Task 11.1: Monitoring & Metrics ✅
- [x] Add Prometheus metrics collection
- [x] Track key metrics:
  - Request latency (p50, p95, p99)
  - Throughput (ops/sec)
  - Storage size
  - Raft metrics (term, commit index)
  - Node health
- [x] Add /metrics/prometheus endpoint
- [x] Create comprehensive test suite (17 tests)

**Deliverables**: Monitoring and metrics ✅

**Status**: ✅ Complete

**Implementation details:**
- Created `src/metrics.rs` with Prometheus integration
- Implemented latency histograms with configurable buckets
- Added request counters (GET, PUT, DELETE)
- Integrated metrics into HTTP handlers
- Added storage and Raft metrics tracking
- Created 17 comprehensive unit tests
- Added `/metrics/prometheus` endpoint for Prometheus scraping
- Maintained backward compatibility with `/metrics` JSON endpoint

---

### Task 11.2: Advanced Logging ✅
- [x] Implement structured logging with tracing
- [x] Add log levels (debug, info, warn, error)
- [x] Support log rotation
- [x] Add request tracing with correlation IDs
- [x] Configure log output (console, file, JSON)

**Deliverables**: Production-ready logging ✅

**Status**: ✅ Complete

**Implementation details:**
- Created `src/logging.rs` with tracing-based logging
- Implemented configurable log levels (TRACE, DEBUG, INFO, WARN, ERROR)
- Added log rotation support using tracing-appender
- Implemented correlation ID generation for request tracing
- Configured multiple output formats (Console, JSON)
- Added file and console output options
- Integrated structured logging into HTTP handlers
- All logging tests passing

---

### Task 11.3: Performance Optimization ✅
- [x] Implement batching for Raft proposals
- [x] Add connection pooling optimization
- [x] Optimize serialization (use bincode for internal)
- [x] Add caching layer for hot data
- [x] Tune Raft parameters (batch size, heartbeat)
- [x] Profile and optimize hot paths

**Deliverables**: Performance improvements  
**Status**: ✅ Complete

---

### Task 11.4: Security Hardening
- [ ] Add TLS support for node-to-node communication
- [ ] Implement basic authentication for HTTP API
- [ ] Add request rate limiting
- [ ] Implement access control (read/write permissions)
- [ ] Add audit logging

**Deliverables**: Security features

---

### Task 11.5: Documentation
- [ ] Update README.md with new architecture
- [ ] Add API documentation
- [ ] Create deployment guide
- [ ] Write operational runbook
- [ ] Add architecture diagrams
- [ ] Document configuration options
- [ ] Create troubleshooting guide

**Deliverables**: Comprehensive documentation

---

## Phase 12: Snapshot & Compaction (2-3 tasks)

**Goal**: Implement log compaction and snapshot mechanisms for storage efficiency and faster recovery.

### Task 12.1: Log Compaction
- [ ] Implement log compaction to reduce storage overhead
- [ ] Add automatic compaction triggers based on log size
- [ ] Create compaction policies (time-based, size-based)
- [ ] Optimize compaction performance
- [ ] Add compaction metrics and monitoring

**Deliverables**: Log compaction system

---

### Task 12.2: Snapshot Creation & Management
- [ ] Add snapshot creation for faster recovery
- [ ] Implement snapshot-based node recovery
- [ ] Add snapshot versioning and metadata
- [ ] Support incremental snapshots
- [ ] Add snapshot cleanup policies

**Deliverables**: Snapshot creation and management

---

### Task 12.3: Snapshot Transfer & Tests
- [ ] Optimize snapshot transfer between nodes
- [ ] Implement snapshot streaming for large datasets
- [ ] Add compression for snapshot transfer
- [ ] Create comprehensive snapshot tests
- [ ] Test snapshot-based recovery scenarios
- [ ] Benchmark snapshot performance

**Deliverables**: Snapshot transfer optimization with test coverage

---

## Phase 13: Multi-Region Support (2-3 tasks)

**Goal**: Enable multi-region deployments for disaster recovery and geo-distributed applications.

### Task 13.1: Cross-Region Replication
- [ ] Add cross-region replication for disaster recovery
- [ ] Implement region-aware data placement strategies
- [ ] Add replication lag monitoring
- [ ] Support configurable replication policies
- [ ] Implement conflict resolution for multi-region writes

**Deliverables**: Cross-region replication system

---

### Task 13.2: Geo-Aware Routing
- [ ] Implement geo-aware routing for read replicas
- [ ] Support read-only replicas for scaling reads
- [ ] Add regional failover capabilities
- [ ] Implement latency-based routing
- [ ] Add region health monitoring

**Deliverables**: Geo-aware routing and read replicas

---

### Task 13.3: Multi-Region Tests
- [ ] Test cross-region replication
- [ ] Test regional failover scenarios
- [ ] Test geo-routing behavior
- [ ] Benchmark cross-region latency
- [ ] Test multi-region consistency guarantees

**Deliverables**: Complete multi-region test coverage

---

## Development Guidelines

### Code Quality Standards
- **Formatting**: Use `cargo fmt` before every commit
- **Linting**: Run `cargo clippy` and fix all warnings
- **Testing**: Maintain >80% code coverage
- **Documentation**: Add doc comments for all public APIs
- **Error Handling**: Use `Result` types, avoid panics in production code

### Git Workflow
- Create feature branch for each task: `feature/task-X.Y-description`
- Write clear commit messages: `feat(consensus): implement raft state machine`
- Keep commits small and focused
- Run tests before pushing

### Testing Strategy
- **Unit Tests**: Test individual components in isolation
- **Integration Tests**: Test component interactions
- **E2E Tests**: Test full system behavior
- **Performance Tests**: Benchmark critical paths
- **Stress Tests**: Test system under load

### Performance Targets
- **Write Latency**: < 10ms local, < 50ms distributed
- **Read Latency**: < 1ms local, < 10ms distributed
- **Throughput**: > 10,000 ops/sec per node
- **Cluster Formation**: < 5 seconds for 3-node cluster
- **Leader Election**: < 2 seconds

---

## Success Criteria

Each phase is considered complete when:
1. All tasks in the phase are completed
2. All tests pass (unit, integration, E2E)
3. Code passes `cargo clippy` with no warnings
4. Code is formatted with `cargo fmt`
5. Documentation is updated
6. Performance targets are met (if applicable)

---

## Getting Started

To begin development:

```bash
# Start with Phase 1, Task 1.1
git checkout -b feature/task-1.1-project-setup

# Complete the task following the checklist
# Run tests: cargo test
# Format: cargo fmt
# Lint: cargo clippy

# Commit and move to next task
git commit -am "feat(setup): complete project structure and dependencies"
git checkout -b feature/task-1.2-config-system
```

---

## Notes

- **OpenRaft vs raft-rs**: OpenRaft provides a more modern, async-first API compared to raft-rs
- **Task Granularity**: Each task should take 1-2 hours for an experienced developer, fitting within a single Copilot agent session
- **Dependencies**: Some tasks have dependencies on previous tasks - follow the order within phases
- **Flexibility**: Phases can be reordered if needed, but maintain task dependencies
- **Testing**: Always test incrementally - don't wait until the end of a phase

---

## References

- [OpenRaft Documentation](https://docs.rs/openraft/)
- [Original Scribe-Ledger](https://github.com/hyra-network/Scribe-Ledger)
- [Raft Consensus Algorithm](https://raft.github.io/)
- [Sled Database](https://docs.rs/sled/)
- [Tokio Async Runtime](https://tokio.rs/)
