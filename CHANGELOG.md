# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- HTTP API server with PUT/GET endpoints for key-value operations
- Comprehensive test suite with unit and integration tests
- Configuration system with TOML support and environment variable overrides
- Development documentation with API usage examples
- Performance testing for large payloads (10MB+)
- Unicode key and value support
- Error handling with proper HTTP status code mapping

### Changed
- Updated README.md with current implementation status and HTTP API documentation
- Enhanced DEVELOPMENT.md with testing strategies and troubleshooting guide
- Refactored project structure to focus on server-side functionality

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