# Simple Scribe Ledger

A simple version of [Scribe Ledger](https://github.com/hyra-network/Scribe-Ledger) implemented in Rust using the [sled](https://github.com/spacejam/sled) embedded database.

This implementation provides a minimal key-value storage engine with two fundamental operations:
- `put(key, value)` - Store a key-value pair
- `get(key)` - Retrieve a value by key

## Features

- **Fast Storage**: Built on sled, a modern embedded database
- **Simple API**: Just two operations - put and get
- **High Performance**: Achieves tens of thousands of operations per second
- **Memory Safe**: Written in Rust with comprehensive error handling
- **Persistent Storage**: Data persists across application restarts
- **Comprehensive Testing**: Unit tests and performance benchmarks included

## Installation

Make sure you have Rust installed, then clone the repository:

```bash
git clone https://github.com/amogusdrip285/simple-scribe-ledger
cd simple-scribe-ledger
```

## Usage

### Basic Example

```rust
use simple_scribe_ledger::SimpleScribeLedger;
use anyhow::Result;

fn main() -> Result<()> {
    // Create a new storage instance
    let ledger = SimpleScribeLedger::new("./data")?;
    
    // Put some data
    ledger.put("name", "Simple Scribe Ledger")?;
    ledger.put("version", "0.1.0")?;
    
    // Get the data back
    if let Some(name) = ledger.get("name")? {
        println!("Name: {}", String::from_utf8_lossy(&name));
    }
    
    // Flush to ensure data is written to disk
    ledger.flush()?;
    
    Ok(())
}
```

### Running the Demo

```bash
cargo run
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

Run the comprehensive Criterion benchmarks:

```bash
cargo bench
```

## Performance

The implementation achieves excellent performance:

- **PUT operations**: ~48,000-60,000 ops/sec
- **GET operations**: ~200,000-280,000 ops/sec  
- **Mixed operations**: ~80,000-106,000 ops/sec

Performance varies based on operation size and system characteristics.

## API Reference

### `SimpleScribeLedger::new(path)`
Creates a new persistent storage instance at the specified path.

### `SimpleScribeLedger::temp()`
Creates a temporary in-memory instance for testing.

### `put<K, V>(&self, key: K, value: V) -> Result<()>`
Stores a key-value pair where both key and value implement `AsRef<[u8]>`.

### `get<K>(&self, key: K) -> Result<Option<Vec<u8>>>`
Retrieves a value by key, returning `None` if the key doesn't exist.

### `len(&self) -> usize`
Returns the number of key-value pairs in the storage.

### `clear(&self) -> Result<()>`
Removes all key-value pairs from the storage.

### `flush(&self) -> Result<()>`
Ensures all pending writes are persisted to disk.

## Benchmarking

The project includes comprehensive benchmarks that test:

1. **PUT operations** at various scales (1 to 10,000 operations)
2. **GET operations** at various scales (1 to 10,000 operations)
3. **Mixed operations** combining puts and gets
4. **Throughput measurements** for sustained performance
5. **Sustained performance** under realistic workloads

Run benchmarks with:
```bash
cargo bench
```

## Architecture

The implementation is built on top of:
- **sled**: A modern embedded database optimized for concurrent workloads
- **anyhow**: For comprehensive error handling
- **criterion**: For detailed performance benchmarking

The storage engine provides a simple abstraction over sled while maintaining its performance characteristics.

## License

MIT License - see LICENSE file for details.
