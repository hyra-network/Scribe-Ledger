# End-to-End Testing

This directory contains comprehensive end-to-end tests for Scribe Ledger.

## Test Files

- `e2e_test.py` - Core E2E test suite for multi-node cluster functionality
- `benchmark.py` - Performance benchmarking and load testing framework

## Running Tests

### E2E Functionality Tests
```bash
cd tests/e2e
python3 e2e_test.py
```

### Performance Benchmarks
```bash
cd tests/e2e
python3 benchmark.py
```

## Requirements

Install Python dependencies:
```bash
pip3 install requests asyncio tabulate
```