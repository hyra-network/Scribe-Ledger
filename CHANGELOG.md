# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2025-09-24

### Added - Phase 3: Distributed Consensus
- **Raft Consensus Implementation**: Complete Raft-based distributed consensus using raft-rs 0.7
  - ConsensusNode with leader election and log replication
  - NetworkTransport trait for cluster communication
  - ClusterMessage system for inter-node messaging
  - Peer management with add/remove node operations
- **Enhanced Manifest Management**: Distributed cluster state management
  - ClusterManifest with cluster-wide metadata tracking
  - ClusterNode representation with status tracking (Active/Inactive/Joining/Leaving)
  - ManifestManager with membership management capabilities
- **Multi-Node Cluster Support**: Production-ready 3-node cluster configuration
  - Individual node configuration files (config-node1.toml, config-node2.toml, config-node3.toml)
  - Cluster membership management with automatic discovery
  - Fault tolerance with automatic failover and recovery
- **E2E Testing Framework**: Comprehensive Python-based testing infrastructure
  - Multi-node cluster test scenarios
  - Data replication validation across nodes
  - Leader election and failover testing
  - Node failure and recovery simulation
  - Concurrent write testing with multiple clients
- **Network Transport Layer**: HTTP-based inter-node communication foundation
  - HttpTransport implementation for cluster messaging
  - Retry logic and failure handling capabilities
  - Message serialization and routing infrastructure

### Enhanced
- **S3 Integration**: Complete immutable storage with MinIO compatibility
  - Automatic background flush to S3-compatible storage
  - Data recovery from S3 on node startup
  - Read-through cache for seamless data access
  - Immutable segment design ensuring data durability
- **Testing Infrastructure**: Expanded test coverage (34 unit tests, 3 consensus tests)
  - Comprehensive consensus behavior validation
  - Multi-node integration testing capabilities
  - Performance testing for concurrent operations
  - S3 storage integration testing (7 S3-specific tests)
- **Configuration System**: Enhanced multi-node configuration support
  - Consensus timing parameters (election_timeout, heartbeat_timeout)
  - Cluster membership configuration with initial peer discovery
  - Network configuration for inter-node communication
  - Storage configuration for distributed S3 access

### Technical Improvements
- **Dependencies**: Added production-grade distributed systems libraries
  - `raft = "0.7"` - Battle-tested Raft consensus implementation
  - `slog` ecosystem - Structured logging for distributed systems
  - `async-trait = "0.1"` - Async trait support for network interfaces
  - `reqwest = "0.11"` - HTTP client for inter-node communication
- **Architecture**: Distributed system design patterns
  - Leader-follower cluster topology
  - State machine replication for consistent metadata
  - Asynchronous message passing between nodes
  - Graceful degradation under node failures

### Fixed
- Resolved compilation issues with Raft message serialization
- Fixed ClusterNode field naming consistency (id vs node_id)
- Corrected timestamp handling in cluster node metadata
- Improved error handling in consensus operations

## [0.2.1] - 2025-09-23

### Added - Phase 2: S3 Integration  
- **Complete S3 Storage Backend**: Full S3-compatible storage implementation
  - S3Storage struct with comprehensive CRUD operations
  - MinIO development environment support
  - Immutable segment-based data organization
  - Automatic retry logic and error handling
- **Hybrid Storage Architecture**: Multi-tier storage system  
  - Hot tier: Sled embedded database for low-latency access
  - Cold tier: S3-compatible storage for durability
  - Background flush mechanism for seamless data migration
  - Read-through cache for transparent data access
- **Data Recovery System**: Robust data recovery from S3
  - Automatic recovery of data on node restart
  - Segment-based recovery with manifest validation
  - Comprehensive error handling and logging
- **Configuration Enhancements**: Advanced configuration options
  - S3 endpoint configuration for MinIO compatibility
  - Environment variable support with precedence over TOML
  - Storage-specific settings (segment size, flush intervals)
  - AWS credentials and region configuration

### Added - Phase 1 Continued
- HTTP API server with PUT/GET endpoints for key-value operations
- Comprehensive test suite with unit and integration tests
- Configuration system with TOML support and environment variable overrides
- Development documentation with API usage examples
- Performance testing for large payloads (10MB+)
- Unicode key and value support
- Error handling with proper HTTP status code mapping

### Changed
- Updated README.md with distributed consensus architecture
- Enhanced DEVELOPMENT.md with cluster setup and E2E testing instructions
- Refactored project structure for distributed systems support

## [0.2.0] - 2025-09-23

### Added
- Full HTTP server implementation using Axum framework
- Sled embedded database integration for persistent storage
- Async/await support throughout the codebase
- ScribeLedger core library with put/get operations
- Comprehensive error handling with custom error types
- Configuration management with default values
- HTTP handlers for PUT and GET operations
- State management using Arc for shared access

### Changed
- Migrated from conceptual client-server model to working HTTP API
- Updated dependencies to include Axum, Tokio, and Sled
- Restructured codebase around HTTP server architecture

### Removed
- Client-side code and binaries (focusing on server-only implementation)
- Unused dependencies and build targets

## [0.1.0] - 2025-09-23

### Added
- Initial project structure and Cargo configuration
- Core module definitions (consensus, manifest, storage, write_node)
- Type definitions and error handling framework
- Configuration system foundation
- Cryptographic utilities for Merkle trees
- Network module for future distributed features
- Storage traits and S3 integration stubs
- Raft consensus module structure
- Basic documentation and README

### Infrastructure
- Rust project setup with proper module organization
- Development tooling configuration (rustfmt, clippy)
- Git repository initialization
- License and contributing guidelines

---

## Version History Summary

- **v0.1.0**: Initial project structure and foundation
- **v0.2.0**: HTTP server implementation with local storage
- **Unreleased**: Enhanced documentation and testing

---

## Development Milestones

### Phase 1: Foundation ✅
- [x] Project structure and build system
- [x] Core types and error handling
- [x] HTTP server with Axum
- [x] Local storage with Sled
- [x] Configuration system
- [x] Comprehensive testing

### Phase 2: S3 Integration 🚧
- [ ] S3 storage backend implementation
- [ ] Segment-based storage architecture
- [ ] Background flush operations
- [ ] Cold storage tier integration

### Phase 3: Distributed Consensus 📋
- [ ] Raft consensus implementation
- [ ] Manifest management system
- [ ] Multi-node coordination
- [ ] Cluster membership

### Phase 4: Cryptographic Verification 📋
- [ ] Merkle tree implementation
- [ ] Proof generation and verification
- [ ] On-chain integration support
- [ ] Cryptographic guarantees

### Phase 5: Production Readiness 📋
- [ ] Monitoring and metrics
- [ ] Performance optimization
- [ ] Security hardening
- [ ] Production deployment

---

## Breaking Changes

### v0.2.0
- **API Changes**: Introduced HTTP-based API, removed conceptual client library
- **Dependencies**: Added Axum, Tokio, Sled as core dependencies
- **Architecture**: Shifted from distributed-first to local-first implementation

---

## Migration Guide

### From v0.1.0 to v0.2.0

The project has evolved from a conceptual framework to a working HTTP server. If you were using the previous version:

1. **New HTTP API**: Replace any conceptual client usage with HTTP requests:
   ```bash
   # Store data
   curl -X PUT http://localhost:8080/my-key --data-binary "my-data"
   
   # Retrieve data
   curl http://localhost:8080/my-key
   ```

2. **Configuration**: Update any configuration to use the new TOML format:
   ```toml
   [node]
   data_dir = "./data"
   
   [network]
   client_port = 8080
   ```

3. **Running**: Use the simplified run command:
   ```bash
   cargo run  # No need for --bin flags
   ```

---

## Contributors

- Development Team - Initial implementation and HTTP server
- Community Contributors - Testing and feedback

---

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.