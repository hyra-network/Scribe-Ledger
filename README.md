# Hyra Scribe Ledger

> **Verifiable, Durable Off-Chain Storage System**

---

Hyra Scribe Ledger is a distributed, immutable, append-only key-value storage system inspired by [Hyra Scribe Ledger](https://github.com/hyra-network/Scribe-Ledger). This implementation leverages **OpenRaft** for modern consensus with optimized performance for high-throughput distributed storage.

## 🏛️ Core Tenets

Hyra Scribe Ledger is built on foundational principles:

1. **Immutability:** Write-once, read-forever. Data is stored in append-only logs, creating a permanent and auditable history.
2. **Durability:** Data, once committed, is considered permanent with robust replication across the cluster.
3. **Verifiability:** Cryptographic integrity checks ensure data authenticity and detect tampering.
4. **Performance:** Optimized with OpenRaft for high-throughput distributed consensus operations.

## 🚀 Quick Start

**Clone the repository:**
```bash
git clone https://github.com/hyra-network/Scribe-Ledger.git
cd Scribe-Ledger
```

**Build the project:**
```bash
cargo build --release
```

**Run a single node:**
```bash
cargo run --bin scribe-node
```

**Run a 3-node cluster:**
```bash
# Terminal 1 - Node 1 (Leader)
cargo run --bin scribe-node -- --config config-node1.toml

# Terminal 2 - Node 2 (Follower)
cargo run --bin scribe-node -- --config config-node2.toml

# Terminal 3 - Node 3 (Follower) 
cargo run --bin scribe-node -- --config config-node3.toml
```

**Run E2E tests:**
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

The HTTP server will start on http://localhost:8080 by default.

## 📡 HTTP API Usage

Scribe Ledger provides a simple HTTP API for storing and retrieving data:

**Store Data (PUT)**
```bash
# Store a value
curl -X PUT http://localhost:8080/my-key \
  -H "Content-Type: application/octet-stream" \
  --data-binary "my value data"
```

**Retrieve Data (GET)**
```bash
# Get a value
curl http://localhost:8080/my-key
```

## ⚙️ Configuration

### Single Node Configuration

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

### Multi-Node Cluster Configuration

For production deployments, use the provided cluster configuration files:

- `config-node1.toml` - Primary leader node (HTTP: 8080, Raft TCP: 8081)
- `config-node2.toml` - Follower node (HTTP: 8090, Raft TCP: 8082)
- `config-node3.toml` - Follower node (HTTP: 8100, Raft TCP: 8083)

Each node configuration includes:

- **Separate ports**: HTTP API port for client communication and dedicated TCP port for Raft consensus
- **Cluster membership**: Peer discovery and automatic leader election
- **S3 integration**: Shared MinIO/S3 storage for distributed persistence
- **Health monitoring**: Heartbeat and failure detection mechanisms

## 🛠️ Development

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

## 🌐 3-Node Cluster Tutorial

This comprehensive tutorial shows you how to set up a 3-node distributed cluster of Hyra Scribe Ledger nodes and communicate between them.

### Prerequisites

Before starting, ensure you have:
- Rust 1.70 or later installed
- The project built in release mode: `cargo build --release`
- Three terminal windows/tabs ready

### Architecture Overview

The cluster consists of:
- **Node 1**: HTTP on port 8001, Raft on port 9001
- **Node 2**: HTTP on port 8002, Raft on port 9002
- **Node 3**: HTTP on port 8003, Raft on port 9003

All nodes will automatically discover each other, elect a leader, and replicate data through the Raft consensus protocol.

### Step 1: Start the Cluster

You can start the cluster in two ways:

#### Option A: Using the automated script (Recommended)

```bash
# Make sure the script is executable
chmod +x scripts/start-cluster.sh

# Start all 3 nodes
./scripts/start-cluster.sh
```

The script will:
- Start all three nodes in the background
- Wait for initialization
- Check node health
- Display cluster information

#### Option B: Start nodes manually

**Terminal 1 - Node 1:**
```bash
cargo run --release --bin scribe-node -- --config config-node1.toml --node-id 1
```

**Terminal 2 - Node 2:**
```bash
cargo run --release --bin scribe-node -- --config config-node2.toml --node-id 2
```

**Terminal 3 - Node 3:**
```bash
cargo run --release --bin scribe-node -- --config config-node3.toml --node-id 3
```

Wait 5-10 seconds for the nodes to discover each other and elect a leader.

### Step 2: Check Cluster Health

Verify all nodes are running:

```bash
# Check Node 1
curl http://127.0.0.1:8001/health

# Check Node 2
curl http://127.0.0.1:8002/health

# Check Node 3
curl http://127.0.0.1:8003/health
```

Expected response from each:
```json
{"status":"ok"}
```

Check cluster status:

```bash
# Check cluster members (from any node)
curl http://127.0.0.1:8001/cluster/members

# Check current leader
curl http://127.0.0.1:8001/cluster/leader
```

### Step 3: Write Data to the Cluster

You can write data to **any node** - it will automatically be replicated to all nodes:

```bash
# Write to Node 1
curl -X PUT http://127.0.0.1:8001/put \
  -H "Content-Type: application/json" \
  -d '{"key": "user:alice", "value": "Alice Smith"}'

# Write to Node 2
curl -X PUT http://127.0.0.1:8002/put \
  -H "Content-Type: application/json" \
  -d '{"key": "user:bob", "value": "Bob Johnson"}'

# Write to Node 3
curl -X PUT http://127.0.0.1:8003/put \
  -H "Content-Type: application/json" \
  -d '{"key": "balance", "value": "1000.50"}'
```

### Step 4: Read Data from Any Node

Data is automatically replicated, so you can read from **any node**:

```bash
# Read from Node 1 (data written to Node 2)
curl http://127.0.0.1:8001/get/user:bob

# Read from Node 2 (data written to Node 3)
curl http://127.0.0.1:8002/get/balance

# Read from Node 3 (data written to Node 1)
curl http://127.0.0.1:8003/get/user:alice
```

Expected responses:
```
Bob Johnson
1000.50
Alice Smith
```

### Step 5: Test Data Replication

Let's verify that data is truly replicated across all nodes:

```bash
# Write a unique key to Node 1
curl -X PUT http://127.0.0.1:8001/put \
  -H "Content-Type: application/json" \
  -d '{"key": "test:replication", "value": "distributed data"}'

# Read from all three nodes (should return same value)
echo "Node 1:" && curl http://127.0.0.1:8001/get/test:replication
echo "Node 2:" && curl http://127.0.0.1:8002/get/test:replication
echo "Node 3:" && curl http://127.0.0.1:8003/get/test:replication
```

All three nodes should return: `distributed data`

### Step 6: Test Concurrent Operations

Send multiple write operations simultaneously:

```bash
# Batch write test
for i in {1..10}; do
  curl -X PUT http://127.0.0.1:8001/put \
    -H "Content-Type: application/json" \
    -d "{\"key\": \"item:$i\", \"value\": \"value$i\"}" &
done
wait

# Verify all writes succeeded (read from different nodes)
curl http://127.0.0.1:8002/get/item:5
curl http://127.0.0.1:8003/get/item:10
```

### Step 7: Test Node Failure and Recovery

**Simulate a node failure:**

If using the automated script:
```bash
# Stop Node 2
kill $(cat pids/node2.pid)
```

If running manually, press `Ctrl+C` in Terminal 2.

**Verify cluster continues to work:**
```bash
# Write to Node 1 (cluster still has majority: 2/3 nodes)
curl -X PUT http://127.0.0.1:8001/put \
  -H "Content-Type: application/json" \
  -d '{"key": "after:failure", "value": "still working"}'

# Read from Node 3
curl http://127.0.0.1:8003/get/after:failure
```

**Restart Node 2:**
```bash
cargo run --release --bin scribe-node -- --config config-node2.toml --node-id 2
```

After a few seconds, Node 2 will rejoin and sync the data:
```bash
# Verify Node 2 has the data written while it was down
curl http://127.0.0.1:8002/get/after:failure
```

### Step 8: Performance Testing

Test cluster throughput:

```bash
# Run the cluster test script
python3 tests/e2e/cluster_e2e_test.py
```

Or benchmark manually:
```bash
# Write 100 items
for i in {1..100}; do
  curl -s -X PUT http://127.0.0.1:8001/put \
    -H "Content-Type: application/json" \
    -d "{\"key\": \"perf:$i\", \"value\": \"value$i\"}" > /dev/null
done

# Read them back
for i in {1..100}; do
  curl -s http://127.0.0.1:8002/get/perf:$i > /dev/null
done
```

### Step 9: Monitor Cluster Metrics

Check cluster metrics:

```bash
# Get metrics from each node
curl http://127.0.0.1:8001/metrics
curl http://127.0.0.1:8002/metrics
curl http://127.0.0.1:8003/metrics
```

### Step 10: Shutdown

**Using the automated script:**
```bash
./scripts/stop-cluster.sh
```

**Manual shutdown:**
Press `Ctrl+C` in each terminal running a node.

### Key Concepts Demonstrated

1. **Automatic Discovery**: Nodes discover each other via UDP broadcast
2. **Leader Election**: Cluster automatically elects a leader using Raft
3. **Data Replication**: All writes are replicated to all nodes
4. **Fault Tolerance**: Cluster continues operating with majority of nodes (2/3)
5. **Consistency**: All nodes serve the same data after replication
6. **Load Distribution**: Read from any node, writes forwarded to leader

### Troubleshooting

**Nodes won't start:**
- Check if ports 8001-8003 and 9001-9003 are available
- Verify config files exist: `config-node1.toml`, `config-node2.toml`, `config-node3.toml`

**Cluster won't form:**
- Wait 10-15 seconds for UDP discovery
- Check firewall allows UDP broadcast
- Verify all nodes are on the same network

**Data not replicating:**
- Ensure majority of nodes (2/3) are running
- Check leader exists: `curl http://127.0.0.1:8001/cluster/leader`
- Review node logs for errors

**Performance issues:**
- Use `--release` flag for production performance
- Ensure SSD storage for best I/O performance
- Consider network latency between nodes

---

## 📚 Get Started Tutorial

### Step 1: Basic Usage

Let's start with a simple example that demonstrates the core functionality:

```rust
use hyra_scribe_ledger::SimpleScribeLedger;
use anyhow::Result;

fn main() -> Result<()> {
    // Create a new storage instance (data will be stored in "./data" directory)
    let ledger = SimpleScribeLedger::new("./data")?;
    
    // Store some data
    ledger.put("user:alice", "Alice Smith")?;
    ledger.put("user:bob", "Bob Johnson")?;
    ledger.put("counter", "42")?;
    
    // Retrieve and display the data
    if let Some(alice) = ledger.get("user:alice")? {
        println!("Found: {}", String::from_utf8_lossy(&alice));
    }
    
    // Ensure data is written to disk
    ledger.flush()?;
    
    println!("Storage contains {} keys", ledger.len());
    
    Ok(())
}
```

**Try it yourself:**
```bash
cargo run --example basic_usage
```

### Step 2: Working with Different Data Types

The storage engine works with any data that can be converted to bytes:

```rust
use hyra_scribe_ledger::SimpleScribeLedger;
use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct User {
    name: String,
    age: u32,
    email: String,
}

fn main() -> Result<()> {
    let ledger = SimpleScribeLedger::new("./tutorial_data")?;
    
    // Store strings
    ledger.put("greeting", "Hello, World!")?;
    
    // Store numbers (as strings)
    ledger.put("balance", "1250.75")?;
    
    // Store JSON (for complex data structures)
    let user = User {
        name: "Alice".to_string(),
        age: 30,
        email: "alice@example.com".to_string(),
    };
    let user_json = serde_json::to_string(&user)?;
    ledger.put("user:alice", user_json)?;
    
    // Retrieve and parse the data
    if let Some(user_data) = ledger.get("user:alice")? {
        let user_str = String::from_utf8_lossy(&user_data);
        let user: User = serde_json::from_str(&user_str)?;
        println!("User: {:?}", user);
    }
    
    Ok(())
}
```

**Try it yourself:**
```bash
cargo run --example data_types
```

### Step 3: Building a Simple Key-Value Store Application

Let's build a simple command-line application that demonstrates practical usage:

```rust
use hyra_scribe_ledger::SimpleScribeLedger;
use anyhow::Result;
use std::io::{self, Write};

fn main() -> Result<()> {
    let ledger = SimpleScribeLedger::new("./my_store")?;
    
    loop {
        print!("Enter command (put <key> <value>, get <key>, list, quit): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        let parts: Vec<&str> = input.split_whitespace().collect();
        
        match parts.as_slice() {
            ["put", key, value] => {
                ledger.put(key, value)?;
                println!("Stored: {} = {}", key, value);
            }
            ["get", key] => {
                match ledger.get(key)? {
                    Some(value) => println!("Found: {} = {}", key, String::from_utf8_lossy(&value)),
                    None => println!("Key '{}' not found", key),
                }
            }
            ["list"] => {
                println!("Database contains {} keys", ledger.len());
            }
            ["quit"] => break,
            _ => println!("Invalid command"),
        }
        
        ledger.flush()?;
    }
    
    Ok(())
}
```

**Try it yourself:**
```bash
cargo run --example cli_store
```

This will start an interactive command-line application where you can:
- `put user alice` - Store data
- `get user` - Retrieve data  
- `list` - See how many items are stored
- `quit` - Exit the application

### Step 4: Performance Testing

Test the performance of your storage:

```bash
# Run the optimized performance test (recommended)
cargo run --release --bin optimized_performance_test

# Run the basic performance test
cargo run --release --bin performance_test

# Run comprehensive benchmarks
cargo bench
```

**Important**: Always use `--release` flag for accurate performance measurements. Debug builds are ~5-10x slower.

### Step 5: Advanced Usage

#### Working with Binary Data

```rust
use hyra_scribe_ledger::SimpleScribeLedger;
use anyhow::Result;

fn main() -> Result<()> {
    let ledger = SimpleScribeLedger::temp()?; // Temporary in-memory storage
    
    // Store binary data
    let binary_data = vec![0u8, 1, 2, 3, 255];
    ledger.put("binary_key", &binary_data)?;
    
    // Retrieve binary data
    if let Some(data) = ledger.get("binary_key")? {
        println!("Binary data: {:?}", data);
    }
    
    Ok(())
}
```

#### Database Management

```rust
use hyra_scribe_ledger::SimpleScribeLedger;
use anyhow::Result;

fn main() -> Result<()> {
    let ledger = SimpleScribeLedger::new("./management_demo")?;
    
    // Add some data
    for i in 0..1000 {
        ledger.put(format!("key{}", i), format!("value{}", i))?;
    }
    
    println!("Database size: {} keys", ledger.len());
    println!("Is empty: {}", ledger.is_empty());
    
    // Clear all data
    ledger.clear()?;
    println!("After clear - Database size: {} keys", ledger.len());
    
    Ok(())
}
```

## 🚀 Running the Examples

We provide several example applications you can run immediately:

```bash
# Basic usage example
cargo run --example basic_usage

# Data types example with JSON serialization  
cargo run --example data_types

# Interactive CLI key-value store
cargo run --example cli_store
```

## 🏗️ Features

- **Fast Storage**: Built on sled, a modern embedded database
- **Simple API**: Just two operations - put and get
- **High Performance**: Achieves tens of thousands of operations per second
- **Memory Safe**: Written in Rust with comprehensive error handling
- **Persistent Storage**: Data persists across application restarts
- **Comprehensive Testing**: Unit tests and performance benchmarks included
- **Flexible Data Types**: Works with any data convertible to bytes
- **Database Management**: Clear, flush, and size operations

## 📖 Usage Examples

### Running the Demo

```bash
cargo run
```

Expected output:
```
Simple Scribe Ledger Demo
=========================
Putting key-value pairs...
Getting values...
name: Simple Scribe Ledger
version: 0.1.0
language: Rust
Key 'nonexistent' not found
Total keys: 3
```

### Running Tests

```bash
cargo test
```

### Running Performance Tests

Run the optimized performance benchmark:

```bash
cargo run --release --bin optimized_performance_test
```

Run the custom performance benchmark:

```bash
cargo run --release --bin performance_test
```

Expected output (release mode):
```
Optimized Simple Scribe Ledger Performance Test
===============================================

Testing with 10000 operations:
  PUT operations (batched): 285,772 ops/sec (34.99 ms total)
  GET operations (optimized): 1,912,032 ops/sec (5.23 ms total)
  MIXED operations (optimized): 509,253 ops/sec (19.64 ms total)
```

**⚠️ Performance Note**: Always use `--release` flag for benchmarking. Debug builds are significantly slower (~5-10x).

Run the comprehensive Criterion benchmarks:

```bash
cargo bench
```

## 📊 Performance

The implementation achieves excellent performance with our optimizations:

**Release mode performance (recommended)**:
- **PUT operations**: ~200,000-300,000 ops/sec
- **GET operations**: ~1,500,000-2,500,000 ops/sec  
- **Mixed operations**: ~400,000-600,000 ops/sec

**Debug mode performance**:
- **PUT operations**: ~50,000-65,000 ops/sec
- **GET operations**: ~200,000-280,000 ops/sec
- **Mixed operations**: ~70,000-110,000 ops/sec

Performance varies based on operation size, batching, and system characteristics.

### Performance Tips

1. **Use release builds**: Always compile with `cargo run --release` for production performance
2. **Batch operations**: Use `apply_batch()` for bulk operations to improve write throughput
3. **Reduce flush frequency**: Let sled handle automatic flushing for temporary data
4. **Pre-allocate data**: Pre-generate keys/values to avoid allocation overhead in hot paths

### Benchmarks

**Optimized Performance Test** (cargo run --release --bin optimized_performance_test)
```
╔══════════╦══════════════════════════════╦══════════════════════════════╦══════════════════════════════╗
║ Ops      ║ PUT (ops/sec, total ms)      ║ GET (ops/sec, total ms)      ║ MIXED (ops/sec, total ms)    ║
╠══════════╬══════════════════════════════╬══════════════════════════════╬══════════════════════════════╣
║     100  ║ 108,852   (0.92 ms)          ║ 2,484,163   (0.04 ms)        ║ 434,556   (0.23 ms)          ║
║   1,000  ║ 282,111   (3.54 ms)          ║ 2,179,186   (0.46 ms)        ║ 596,053   (1.68 ms)          ║
║   5,000  ║ 288,302  (17.34 ms)          ║ 1,763,890   (2.83 ms)        ║ 527,662   (9.48 ms)          ║
║  10,000  ║ 285,772  (34.99 ms)          ║ 1,912,032   (5.23 ms)        ║ 509,253  (19.64 ms)          ║
╚══════════╩══════════════════════════════╩══════════════════════════════╩══════════════════════════════╝
```

**Note**: These are release mode results. Debug mode performance will be significantly lower but still meets production targets for most use cases.
Sustained (10,000 ops) → MIXED: 30,171 ops/sec (364.59 ms total)

Benchmark Results (cargo bench)
PUT Operations
```
╔══════╦══════════════════════════╗
║ Ops  ║ Time [ms]                ║
╠══════╬══════════════════════════╣
║    1 ║ 1.08 – 1.11 ms           ║
║   10 ║ 1.10 – 1.13 ms           ║
║  100 ║ 1.67 – 1.70 ms           ║
║ 1000 ║ 5.66 – 5.85 ms           ║
║ 5000 ║ 26.11 – 27.38 ms         ║
║10000 ║ 52.98 – 55.52 ms         ║
╚══════╩══════════════════════════╝
```

GET Operations
```
╔══════╦══════════════════════════╗
║ Ops  ║ Time [ns / µs / ms]      ║
╠══════╬══════════════════════════╣
║    1 ║ 375 – 398 ns             ║
║   10 ║ 4.32 – 4.56 µs           ║
║  100 ║ 47.19 – 50.05 µs         ║
║ 1000 ║ 557.7 – 585.9 µs         ║
║ 5000 ║ 3.48 – 3.67 ms           ║
║10000 ║ 7.62 – 8.01 ms           ║
╚══════╩══════════════════════════╝
```

MIXED Operations
```
╔══════╦══════════════════════════╗
║ Ops  ║ Time [ms]                ║
╠══════╬══════════════════════════╣
║    1 ║ 1.07 – 1.10 ms           ║
║   10 ║ 1.12 – 1.15 ms           ║
║  100 ║ 1.47 – 1.50 ms           ║
║ 1000 ║ 3.84 – 4.00 ms           ║
║ 5000 ║ 14.21 – 14.85 ms         ║
║10000 ║ 29.35 – 30.47 ms         ║
╚══════╩══════════════════════════╝
```

Throughput Benchmarks
```
╔═══════════════╦════════════════╦══════════════════════════╗
║ Operation     ║ Time [ms]      ║ Throughput               ║
╠═══════════════╬════════════════╬══════════════════════════╣
║ PUT 10,000    ║ 50.25 – 53.59  ║ 186.6K – 199.0K elem/s   ║
║ GET 10,000    ║  6.73 – 7.04   ║ 1.42M – 1.49M elem/s     ║
╚═══════════════╩════════════════╩══════════════════════════╝
```

## 🔧 API Reference

### Core Methods

#### `SimpleScribeLedger::new(path)`
Creates a new persistent storage instance at the specified path.

**Parameters:**
- `path`: Directory path where the database files will be stored

**Returns:** `Result<SimpleScribeLedger>`

**Example:**
```rust
let ledger = SimpleScribeLedger::new("./my_database")?;
```

#### `SimpleScribeLedger::temp()`
Creates a temporary in-memory instance for testing.

**Returns:** `Result<SimpleScribeLedger>`

**Example:**
```rust
let ledger = SimpleScribeLedger::temp()?;
```

#### `put<K, V>(&self, key: K, value: V) -> Result<()>`
Stores a key-value pair where both key and value implement `AsRef<[u8]>`.

**Parameters:**
- `key`: The key (String, &str, Vec<u8>, &[u8], etc.)
- `value`: The value (String, &str, Vec<u8>, &[u8], etc.)

**Example:**
```rust
ledger.put("user:123", "John Doe")?;
ledger.put(b"binary_key", vec![1, 2, 3, 4])?;
```

#### `get<K>(&self, key: K) -> Result<Option<Vec<u8>>>`
Retrieves a value by key, returning `None` if the key doesn't exist.

**Parameters:**
- `key`: The key to look up

**Returns:** `Result<Option<Vec<u8>>>`

**Example:**
```rust
if let Some(value) = ledger.get("user:123")? {
    let name = String::from_utf8_lossy(&value);
    println!("User name: {}", name);
}
```

### Utility Methods

#### `len(&self) -> usize`
Returns the number of key-value pairs in the storage.

#### `is_empty(&self) -> bool`
Returns true if the storage contains no key-value pairs.

#### `clear(&self) -> Result<()>`
Removes all key-value pairs from the storage.

#### `flush(&self) -> Result<()>`
Ensures all pending writes are persisted to disk.

## 🧪 Testing

The project includes comprehensive tests that cover both the basic functionality and the underlying sled engine:

### Unit Tests (13 tests)
```bash
cargo test --lib
```

Our unit tests cover:
- Basic put/get operations
- Multiple operations and batch processing
- Value overwriting and updates
- Database clearing
- Persistence across restarts
- Binary data handling
- Unicode support
- Large keys and values
- Concurrent operations
- Stress testing with thousands of operations
- Empty keys and values handling
- Flush behavior and durability
- Error handling scenarios

### Integration Tests (11 tests)
```bash
cargo test --test integration_tests
cargo test --test sled_engine_tests
```

Integration tests verify:
- **Database lifecycle**: Create, populate, reopen, verify persistence
- **High load operations**: Batch processing and performance under load
- **Data consistency**: Verify data integrity across operations
- **Memory management**: Large datasets and cleanup behavior
- **Edge cases**: Maximum sizes, special characters, error conditions
- **Concurrent read/write**: Multi-threaded access patterns
- **Performance characteristics**: Sequential vs random access
- **Durability guarantees**: Data survives process restarts
- **Data patterns**: Hierarchical keys, timestamps, sequences
- **Memory efficiency**: Varying data sizes and memory usage

### Performance Benchmarks
```bash
cargo bench
```

Benchmark categories:
1. **PUT operations** at various scales (1 to 10,000 operations)
2. **GET operations** at various scales (1 to 10,000 operations) 
3. **Mixed operations** combining puts and gets
4. **Throughput measurements** for sustained performance
5. **Sustained performance** under realistic workloads

All tests can be run together with:
```bash
cargo test
```

## 🏛️ Architecture

The implementation is built on top of:

- **[sled](https://github.com/spacejam/sled)**: A modern embedded database optimized for concurrent workloads
- **[anyhow](https://github.com/dtolnay/anyhow)**: For comprehensive error handling
- **[criterion](https://github.com/bheisler/criterion.rs)**: For detailed performance benchmarking

### Directory Structure

```
simple-scribe-ledger/
├── src/
│   ├── lib.rs                  # Core SimpleScribeLedger implementation
│   ├── main.rs                 # Demo application
│   └── bin/
│       └── performance_test.rs # Performance benchmark tool
├── benches/
│   └── storage_benchmark.rs    # Criterion benchmarks
├── tests/                      # Integration tests
├── Cargo.toml                  # Project configuration
└── README.md                   # This file
```

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Run `cargo test` and `cargo fmt`
6. Submit a pull request

## 📝 License

MIT License - see LICENSE file for details.

## 🔗 Related Projects

- [Scribe Ledger](https://github.com/hyra-network/Scribe-Ledger) - The original inspiration
- [sled](https://github.com/spacejam/sled) - The underlying embedded database
- [Rust](https://www.rust-lang.org/) - The programming language
