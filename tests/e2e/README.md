# End-to-End Tests

This directory contains end-to-end tests for the Simple Scribe Ledger cluster.

## Prerequisites

- Python 3.7 or later
- Simple Scribe Ledger binary built (`cargo build --bin scribe-node`)
- Node configuration files (config-node1.toml, config-node2.toml, config-node3.toml)

## Installation

Install Python dependencies:

```bash
pip install -r requirements.txt
```

Or using a virtual environment:

```bash
python3 -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
pip install -r requirements.txt
```

## Running E2E Tests

### Basic Usage

Run all E2E tests:

```bash
python3 tests/e2e/cluster_e2e_test.py
```

Or make it executable and run:

```bash
chmod +x tests/e2e/cluster_e2e_test.py
./tests/e2e/cluster_e2e_test.py
```

### Test Coverage

The E2E test suite includes:

1. **Health Checks** - Verify all nodes respond to health endpoints
2. **Node Connectivity** - Ensure nodes are accessible
3. **Data Replication** - Test data propagation across nodes
4. **Metrics Endpoints** - Verify metrics collection
5. **Concurrent Operations** - Test parallel write operations
6. **Performance Benchmark** - Measure operation latency
7. **Stress Test** - Test system under load (100+ operations)

### Manual Cluster Testing

You can also manually start and test the cluster:

```bash
# Start the cluster
./scripts/start-cluster.sh

# Run tests (in another terminal)
./scripts/test-cluster.sh

# Stop the cluster
./scripts/stop-cluster.sh
```

## CI Integration

The E2E tests are integrated into the GitHub Actions workflow and run on every push/PR.

## Troubleshooting

### Port Conflicts

If you see "Address already in use" errors:

1. Stop any existing cluster: `./scripts/stop-cluster.sh`
2. Check for processes using the ports: `lsof -i :8001,8002,8003,9001,9002,9003`
3. Kill any lingering processes

### Build Errors

Make sure the binary is built:

```bash
cargo build --bin scribe-node
```

For release builds:

```bash
cargo build --release --bin scribe-node
```

### Test Failures

If tests fail:

1. Check the logs in `logs/` directory
2. Verify configurations in `config-node*.toml` files
3. Ensure all dependencies are installed
4. Make sure no firewall is blocking localhost connections
