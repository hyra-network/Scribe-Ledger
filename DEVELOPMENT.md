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

### Task 1.1: Project Structure and Dependencies
- [ ] Add openraft dependency to Cargo.toml (version ~0.9 or latest stable)
- [ ] Add required dependencies: tokio, serde, serde_json, anyhow, thiserror, tracing
- [ ] Remove or update conflicting dependencies if any
- [ ] Create directory structure: src/{consensus/, storage/, network/, manifest/, config/}
- [ ] Update .gitignore for distributed node data directories

**Deliverables**: Updated Cargo.toml with all dependencies, directory structure created

---

### Task 1.2: Configuration System
- [ ] Create src/config.rs with Config struct supporting:
  - Node configuration (id, address, data_dir)
  - Network configuration (listen_addr, client_port, raft_port)
  - Storage configuration (segment_size, max_cache_size)
  - Consensus configuration (election_timeout, heartbeat_interval)
- [ ] Implement TOML file parsing
- [ ] Add environment variable override support (SCRIBE_* prefix)
- [ ] Create example config files: config.toml, config-node1.toml, config-node2.toml, config-node3.toml
- [ ] Add configuration validation logic

**Deliverables**: Fully functional configuration system with TOML and env var support

---

### Task 1.3: Error Handling and Type System
- [ ] Create src/error.rs with ScribeError enum covering:
  - Storage errors
  - Consensus errors  
  - Network errors
  - Configuration errors
  - Serialization errors
- [ ] Implement From traits for converting third-party errors
- [ ] Create src/types.rs with common types:
  - NodeId, SegmentId, ManifestId
  - Key and Value type aliases
  - Request/Response types
- [ ] Add comprehensive error context using anyhow

**Deliverables**: Robust error handling system and type definitions

---

## Phase 2: Storage Layer (3-4 tasks)

**Goal**: Implement local storage with Sled and prepare for multi-tier storage architecture.

### Task 2.1: Enhanced Storage Backend
- [ ] Create src/storage/mod.rs with StorageBackend trait:
  - async fn put(&self, key: Key, value: Value) -> Result<()>
  - async fn get(&self, key: &Key) -> Result<Option<Value>>
  - async fn delete(&self, key: &Key) -> Result<()>
  - async fn flush(&self) -> Result<()>
  - async fn snapshot(&self) -> Result<HashMap<Key, Value>>
- [ ] Implement SledStorage struct with StorageBackend trait
- [ ] Add async wrappers around sled operations using tokio::task::spawn_blocking
- [ ] Implement proper error handling and conversion

**Deliverables**: Storage abstraction layer with Sled implementation

---

### Task 2.2: Storage Tests and Benchmarks
- [ ] Create tests/storage_tests.rs with comprehensive tests:
  - Basic put/get operations
  - Large data handling (10MB+)
  - Concurrent operations
  - Persistence across restarts
  - Error cases
- [ ] Add benchmarks in benches/storage_benchmark.rs
- [ ] Test edge cases (empty keys, empty values, Unicode)
- [ ] Verify async behavior is correct

**Deliverables**: Complete test coverage for storage layer

---

### Task 2.3: Segment-based Storage Preparation
- [ ] Create src/storage/segment.rs with Segment struct:
  - timestamp: u64
  - data: HashMap<Key, Value>
  - size: usize
  - segment_id: SegmentId
- [ ] Implement segment serialization/deserialization
- [ ] Add PendingSegment struct for buffering writes
- [ ] Create segment manager for tracking active/flushed segments
- [ ] Add segment size threshold logic

**Deliverables**: Segment data structures ready for future S3 integration

---

## Phase 3: OpenRaft Consensus Layer (5-6 tasks)

**Goal**: Implement distributed consensus using OpenRaft for cluster coordination.

### Task 3.1: OpenRaft State Machine
- [ ] Create src/consensus/state_machine.rs implementing openraft::RaftStateMachine
- [ ] Define AppData type for log entries (put/delete operations)
- [ ] Define AppDataResponse type for operation results
- [ ] Implement apply() method to apply committed entries to storage
- [ ] Implement snapshot() for state machine snapshots
- [ ] Add restore_snapshot() for recovering from snapshots

**Deliverables**: OpenRaft state machine implementation

---

### Task 3.2: OpenRaft Storage Backend
- [ ] Create src/consensus/storage.rs implementing openraft::RaftStorage
- [ ] Implement log storage (append, get, delete entries)
- [ ] Implement hard state storage (term, vote)
- [ ] Implement snapshot storage
- [ ] Use Sled for persistent raft storage
- [ ] Add proper error handling and conversions

**Deliverables**: Persistent storage for Raft log and metadata

---

### Task 3.3: OpenRaft Network Layer
- [ ] Create src/consensus/network.rs implementing openraft::RaftNetwork
- [ ] Implement send_append_entries RPC
- [ ] Implement send_vote RPC
- [ ] Implement send_install_snapshot RPC
- [ ] Use tokio TcpStream for network communication
- [ ] Add retry logic and timeout handling
- [ ] Implement connection pooling

**Deliverables**: Network layer for Raft RPCs

---

### Task 3.4: Consensus Node Integration
- [ ] Create src/consensus/mod.rs with ConsensusNode struct
- [ ] Initialize OpenRaft instance with state machine, storage, network
- [ ] Implement cluster membership management (add_learner, change_membership)
- [ ] Add leader/follower role tracking
- [ ] Implement graceful shutdown
- [ ] Add health check methods

**Deliverables**: Fully integrated OpenRaft node

---

### Task 3.5: Consensus Tests
- [ ] Create tests/consensus_tests.rs:
  - Single node startup
  - Leader election in 3-node cluster
  - Log replication
  - Follower failure and recovery
  - Leader failure and re-election
- [ ] Add test utilities for multi-node setup
- [ ] Test membership changes
- [ ] Verify state machine consistency

**Deliverables**: Comprehensive consensus layer tests

---

## Phase 4: Manifest Management (2-3 tasks)

**Goal**: Implement distributed metadata management using consensus.

### Task 4.1: Manifest Data Structures
- [ ] Create src/manifest/mod.rs with:
  - ManifestEntry struct (segment_id, timestamp, merkle_root, size)
  - ClusterManifest struct (version, entries: Vec<ManifestEntry>)
  - ClusterNode struct (id, address, state, last_heartbeat)
- [ ] Implement serialization/deserialization
- [ ] Add manifest versioning logic
- [ ] Create manifest diff/merge utilities

**Deliverables**: Manifest data structures and utilities

---

### Task 4.2: Manifest Manager
- [ ] Create ManifestManager struct
- [ ] Implement manifest updates through consensus (propose to Raft)
- [ ] Add manifest query methods (get_latest, get_segments)
- [ ] Implement manifest synchronization across nodes
- [ ] Add conflict resolution logic
- [ ] Cache manifest locally for performance

**Deliverables**: Manifest management with consensus backing

---

### Task 4.3: Manifest Tests
- [ ] Test manifest updates in single node
- [ ] Test manifest replication across cluster
- [ ] Test manifest consistency after node failure
- [ ] Test concurrent manifest updates
- [ ] Verify manifest versioning

**Deliverables**: Complete manifest tests

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

**Notes**: Current implementation provides stub endpoints that work in standalone mode. When full distributed consensus is integrated (Tasks 6.x and 7.x), these endpoints will be connected to the actual OpenRaft consensus layer.

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

## Phase 6: Node Discovery & Cluster Formation (2-3 tasks)

**Goal**: Implement automatic cluster discovery and dynamic membership.

### Task 6.1: Discovery Service ✅
- [x] Create src/discovery.rs with DiscoveryService
- [x] Implement UDP broadcast for node discovery
- [x] Add peer list management
- [x] Implement heartbeat protocol
- [x] Add failure detection logic
- [x] Support configurable discovery endpoints

**Deliverables**: Node discovery service

---

### Task 6.2: Cluster Initialization ✅
- [x] Implement bootstrap logic for first node
- [x] Add automatic cluster joining for new nodes
- [x] Support manual cluster seeding via config
- [x] Implement leader discovery
- [x] Add join request/response handling
- [x] Handle network partitions gracefully

**Deliverables**: Automatic cluster formation

---

### Task 6.3: Discovery Tests ✅
- [x] Test single node bootstrap
- [x] Test 3-node cluster auto-discovery
- [x] Test node joining running cluster
- [x] Test failure detection
- [x] Test network partition scenarios

**Deliverables**: Discovery and cluster formation tests

---

## Phase 7: Write Path & Data Replication (3-4 tasks)

**Goal**: Implement distributed write path with consensus.

### Task 7.1: Write Request Handling
- [ ] Create write request flow:
  - Client sends PUT request to any node
  - Node forwards to leader if not leader
  - Leader proposes write to Raft
  - Wait for consensus
  - Apply to local storage
  - Return success to client
- [ ] Implement request forwarding logic
- [ ] Add timeout handling
- [ ] Support batching of writes

**Deliverables**: Distributed write path

---

### Task 7.2: Read Request Handling
- [ ] Implement read flow:
  - Check local storage first
  - Support linearizable reads (query leader)
  - Support stale reads from followers (optional)
  - Cache frequently accessed data
- [ ] Add read consistency options
- [ ] Implement read-through caching

**Deliverables**: Distributed read path

---

### Task 7.3: Data Consistency Tests
- [ ] Test write-then-read consistency
- [ ] Test replication across all nodes
- [ ] Test read-your-writes consistency
- [ ] Test network partition scenarios
- [ ] Verify data durability after crashes

**Deliverables**: Consistency and replication tests

---

## Phase 8: Binary & Node Implementation (2-3 tasks)

**Goal**: Create runnable node binary and deployment scripts.

### Task 8.1: Node Binary
- [ ] Create src/bin/scribe-node.rs
- [ ] Implement CLI argument parsing with clap:
  - --config <path> - Config file path
  - --node-id <id> - Override node ID
  - --bootstrap - Bootstrap new cluster
- [ ] Add graceful shutdown handling (SIGTERM, SIGINT)
- [ ] Implement logging with tracing/tracing-subscriber
- [ ] Add startup banner and version info

**Deliverables**: Runnable node binary

---

### Task 8.2: Multi-Node Testing Scripts
- [ ] Create scripts/start-cluster.sh for starting 3-node cluster
- [ ] Create scripts/stop-cluster.sh for clean shutdown
- [ ] Add scripts/test-cluster.sh for basic cluster testing
- [ ] Create example systemd service files
- [ ] Add Docker support (Dockerfile)

**Deliverables**: Deployment and testing scripts

---

### Task 8.3: End-to-End Tests
- [ ] Create tests/e2e/ directory
- [ ] Write Python E2E test script:
  - Start 3-node cluster
  - Test data replication
  - Test leader election
  - Test node failure recovery
  - Test concurrent operations
- [ ] Add performance benchmarks
- [ ] Create stress tests

**Deliverables**: Complete E2E test suite

---

## Phase 9: Cryptographic Verification (2-3 tasks)

**Goal**: Add Merkle tree support for data verification.

### Task 9.1: Merkle Tree Implementation
- [ ] Create src/crypto/mod.rs with MerkleTree struct
- [ ] Implement tree construction from key-value pairs
- [ ] Add proof generation (get_proof for specific key)
- [ ] Implement proof verification
- [ ] Use SHA-256 for hashing
- [ ] Handle edge cases (empty tree, single element)

**Deliverables**: Merkle tree implementation

---

### Task 9.2: Manifest Merkle Root Integration
- [ ] Add merkle_root field to ManifestEntry
- [ ] Compute Merkle root during segment creation
- [ ] Store Merkle root in manifest
- [ ] Implement verification API endpoint
- [ ] Add GET /verify/:key endpoint

**Deliverables**: Merkle root in manifest

---

### Task 9.3: Crypto Tests
- [ ] Test Merkle tree construction
- [ ] Test proof generation and verification
- [ ] Test with various data sizes
- [ ] Test edge cases
- [ ] Benchmark performance

**Deliverables**: Cryptographic verification tests

---

## Phase 10: Advanced Features & Optimization (4-5 tasks)

**Goal**: Add production-ready features and optimizations.

### Task 10.1: Monitoring & Metrics
- [ ] Add Prometheus metrics collection
- [ ] Track key metrics:
  - Request latency (p50, p95, p99)
  - Throughput (ops/sec)
  - Storage size
  - Raft metrics (term, commit index)
  - Node health
- [ ] Add /metrics endpoint
- [ ] Create Grafana dashboard templates

**Deliverables**: Monitoring and metrics

---

### Task 10.2: Advanced Logging
- [ ] Implement structured logging with tracing
- [ ] Add log levels (debug, info, warn, error)
- [ ] Support log rotation
- [ ] Add request tracing with correlation IDs
- [ ] Configure log output (console, file, JSON)

**Deliverables**: Production-ready logging

---

### Task 10.3: Performance Optimization
- [ ] Implement batching for Raft proposals
- [ ] Add connection pooling optimization
- [ ] Optimize serialization (use bincode for internal)
- [ ] Add caching layer for hot data
- [ ] Tune Raft parameters (batch size, heartbeat)
- [ ] Profile and optimize hot paths

**Deliverables**: Performance improvements

---

### Task 10.4: Security Hardening
- [ ] Add TLS support for node-to-node communication
- [ ] Implement basic authentication for HTTP API
- [ ] Add request rate limiting
- [ ] Implement access control (read/write permissions)
- [ ] Add audit logging

**Deliverables**: Security features

---

### Task 10.5: Documentation
- [ ] Update README.md with new architecture
- [ ] Add API documentation
- [ ] Create deployment guide
- [ ] Write operational runbook
- [ ] Add architecture diagrams
- [ ] Document configuration options
- [ ] Create troubleshooting guide

**Deliverables**: Comprehensive documentation

---

## Phase 11: Future Enhancements (Optional)

**Goal**: Advanced features for production deployments.

### Task 11.1: S3 Cold Storage (Future)
- [ ] Integrate S3 storage backend
- [ ] Implement segment flushing to S3
- [ ] Add read-through from S3
- [ ] Support MinIO for local development

### Task 11.2: Snapshot & Compaction (Future)
- [ ] Implement log compaction
- [ ] Add snapshot creation
- [ ] Optimize snapshot transfer
- [ ] Add automatic compaction triggers

### Task 11.3: Multi-Region Support (Future)
- [ ] Add cross-region replication
- [ ] Implement geo-aware routing
- [ ] Support read replicas

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
