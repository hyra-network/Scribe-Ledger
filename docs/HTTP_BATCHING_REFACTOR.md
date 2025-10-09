# HTTP Batching Logic Refactoring

## Summary

Successfully moved HTTP request batching logic from benchmark files to source code as requested in the issue.

## Changes Made

### 1. Created `src/http_client.rs` (new file, 156 lines)
   - Implemented `batched_put_operations()` - Handles batched PUT requests with MAX_CONCURRENCY=20
   - Implemented `batched_get_operations()` - Handles batched GET requests with MAX_CONCURRENCY=20
   - Implemented `batched_mixed_operations()` - Handles batched mixed PUT/GET requests
   - Defined `PutRequest` and `GetResponse` structs (moved from benchmark file)
   - All functions use controlled concurrency to ensure linear scaling

### 2. Updated `benches/simple_http_benchmark.rs` (reduced from 300 to 202 lines)
   - Removed duplicate batching logic from all three benchmark functions
   - Removed `MAX_CONCURRENCY` constant (now in http_client module)
   - Removed duplicate struct definitions (now imported from http_client)
   - Benchmarks now simply call `batched_*_operations()` with full dataset
   - **Result: 98 lines removed, much cleaner code**

### 3. Updated `src/lib.rs`
   - Added `pub mod http_client;` to expose the new module

### 4. Updated `Cargo.toml`
   - Moved `reqwest` from dev-dependencies to main dependencies
   - Required for the http_client module to compile

### 5. Added Test in `tests/http_tests.rs`
   - Created `test_batched_http_operations()` to verify the new module works correctly
   - Tests both PUT and GET batched operations with 50 requests each
   - All 20 HTTP tests passing

## Benefits

1. **Separation of Concerns**: Batching logic is now in source code, not test code
2. **Reusability**: The batching functions can be used by any code that needs them
3. **Cleaner Benchmarks**: Benchmarks are now focused on measurement, not implementation
4. **Maintainability**: Single source of truth for batching logic
5. **Testability**: Batching logic can now be unit tested independently

## Verification

- ✅ All 160 library tests pass
- ✅ All 20 HTTP integration tests pass (including new test for batched operations)
- ✅ Benchmark file compiles without warnings
- ✅ Code successfully builds in both dev and release modes

## Performance Impact

No performance impact expected - the batching logic is identical to what was in the benchmark,
just moved to a different location. The benchmark still uses:
- MAX_CONCURRENCY = 20
- Same batching algorithm
- Same async/await patterns

## Files Changed

```
 Cargo.toml                       |   1 +
 benches/simple_http_benchmark.rs | 117 +++-----------------------------------
 src/http_client.rs               | 155 +++++++++++++++++++++++++++++++++++++++++++
 src/lib.rs                       |   1 +
 tests/http_tests.rs              |  29 +++++++++
 5 files changed, 196 insertions(+), 107 deletions(-)
```

## Implementation Details

### Before (in benchmark):
```rust
// Inside benchmark function
const MAX_CONCURRENCY: usize = 20;
let mut i = 0;
while i < ops {
    let batch_size = std::cmp::min(MAX_CONCURRENCY, ops - i);
    // ... complex batching logic ...
    i += batch_size;
}
```

### After (in source code):
```rust
// In benchmark - clean and simple
let result = batched_put_operations(&client, &base_url, &keys, &payloads).await;

// In src/http_client.rs - reusable implementation
pub async fn batched_put_operations(...) -> usize {
    const MAX_CONCURRENCY: usize = 20;
    // ... batching implementation ...
}
```

## Conclusion

The refactoring successfully achieves the goal stated in the issue: "instead of splitting the 
http queries inside benchmark, it should do that inside the source code, and we only call the 
whole function with 1000 requests in benchmark".

The benchmarks now simply call helper functions with the full dataset (10, 100, 500, or 1000 
requests), and the batching happens transparently inside those functions.
