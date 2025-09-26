# Simple Scribe Ledger

A simple version of [Scribe Ledger](https://github.com/hyra-network/Scribe-Ledger) implemented in Rust using the [sled](https://github.com/spacejam/sled) embedded database.

This implementation provides a minimal key-value storage engine with two fundamental operations:
- `put(key, value)` - Store a key-value pair
- `get(key)` - Retrieve a value by key

## 🚀 Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (1.70 or later)
- Git

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/amogusdrip285/simple-scribe-ledger
   cd simple-scribe-ledger
   ```

2. **Build the project:**
   ```bash
   cargo build
   ```

3. **Run the demo:**
   ```bash
   cargo run
   ```

4. **Run tests:**
   ```bash
   cargo test
   ```

## 📚 Get Started Tutorial

### Step 1: Basic Usage

Let's start with a simple example that demonstrates the core functionality:

```rust
use simple_scribe_ledger::SimpleScribeLedger;
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
use simple_scribe_ledger::SimpleScribeLedger;
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
use simple_scribe_ledger::SimpleScribeLedger;
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
# Run the built-in performance test
cargo run --bin performance_test

# Run comprehensive benchmarks
cargo bench
```

### Step 5: Advanced Usage

#### Working with Binary Data

```rust
use simple_scribe_ledger::SimpleScribeLedger;
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
use simple_scribe_ledger::SimpleScribeLedger;
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

Run the custom performance benchmark:

```bash
cargo run --bin performance_test
```

Expected output:
```
Simple Scribe Ledger Performance Test
====================================

Testing with 100 operations:
  PUT operations: 27759 ops/sec (3.60 ms total)
  GET operations: 255044 ops/sec (0.39 ms total)
  MIXED operations: 106598 ops/sec (0.94 ms total)
...
```

Run the comprehensive Criterion benchmarks:

```bash
cargo bench
```

## 📊 Performance

The implementation achieves excellent performance:

- **PUT operations**: ~48,000-60,000 ops/sec
- **GET operations**: ~200,000-280,000 ops/sec  
- **Mixed operations**: ~80,000-106,000 ops/sec

Performance varies based on operation size and system characteristics.

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
