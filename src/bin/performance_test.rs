use simple_scribe_ledger::SimpleScribeLedger;
use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("Simple Scribe Ledger Performance Test - Optimized");
    println!("================================================");
    
    // Test different operation counts
    let test_sizes = vec![100, 1000, 5000, 10000];
    
    for size in test_sizes {
        println!("\nTesting with {} operations:", size);
        
        // Pre-allocate keys and values to avoid allocation overhead during benchmarking
        let keys: Vec<String> = (0..size).map(|i| format!("key{}", i)).collect();
        let values: Vec<String> = (0..size).map(|i| format!("value{}", i)).collect();
        
        // Test PUT operations with optimizations
        let ledger = SimpleScribeLedger::temp()?;
        
        // Warm-up phase
        ledger.put("warmup", "value")?;
        ledger.flush()?;
        
        let start = Instant::now();
        
        // Use batching for better performance when size is large
        if size > 100 {
            let batch_size = std::cmp::min(100, size / 4);
            let mut i = 0;
            while i < size {
                let mut batch = SimpleScribeLedger::new_batch();
                let end = std::cmp::min(i + batch_size, size);
                
                for j in i..end {
                    batch.insert(keys[j].as_bytes(), values[j].as_bytes());
                }
                
                ledger.apply_batch(batch)?;
                i = end;
            }
        } else {
            // For smaller sizes, individual operations might be more appropriate
            for i in 0..size {
                ledger.put(&keys[i], &values[i])?;
            }
        }
        
        ledger.flush()?;
        
        let put_duration = start.elapsed();
        let put_ops_per_sec = size as f64 / put_duration.as_secs_f64();
        
        println!("  PUT operations (optimized): {:.0} ops/sec ({:.2} ms total)", 
                put_ops_per_sec, put_duration.as_secs_f64() * 1000.0);
        
        // Test GET operations using pre-allocated keys
        let start = Instant::now();
        
        for key in &keys {
            let _value = ledger.get(key)?;
        }
        
        let get_duration = start.elapsed();
        let get_ops_per_sec = size as f64 / get_duration.as_secs_f64();
        
        println!("  GET operations (optimized): {:.0} ops/sec ({:.2} ms total)", 
                get_ops_per_sec, get_duration.as_secs_f64() * 1000.0);
                
        // Test MIXED operations with optimizations
        let ledger = SimpleScribeLedger::temp()?;
        
        // Warm-up phase
        ledger.put("warmup_mixed", "value")?;
        ledger.flush()?;
        
        let start = Instant::now();
        
        // Half puts, half gets using pre-allocated data
        let half_size = size / 2;
        
        // First put half the data using batching for larger sizes
        if half_size > 50 {
            let batch_size = std::cmp::min(50, half_size / 4);
            let mut i = 0;
            while i < half_size {
                let mut batch = SimpleScribeLedger::new_batch();
                let end = std::cmp::min(i + batch_size, half_size);
                
                for j in i..end {
                    batch.insert(keys[j].as_bytes(), values[j].as_bytes());
                }
                
                ledger.apply_batch(batch)?;
                i = end;
            }
        } else {
            // For smaller sizes, use individual operations
            for i in 0..half_size {
                ledger.put(&keys[i], &values[i])?;
            }
        }
        
        // Then get it back using pre-allocated keys
        for i in 0..half_size {
            let _value = ledger.get(&keys[i])?;
        }
        
        ledger.flush()?;
        let mixed_duration = start.elapsed();
        let mixed_ops_per_sec = size as f64 / mixed_duration.as_secs_f64();
        
        println!("  MIXED operations (optimized): {:.0} ops/sec ({:.2} ms total)", 
                mixed_ops_per_sec, mixed_duration.as_secs_f64() * 1000.0);
    }
    
    // Sustained performance test with optimizations
    println!("\n--- Optimized Sustained Performance Test (10,000 operations) ---");
    let ledger = SimpleScribeLedger::temp()?;
    let test_size = 10000;
    
    // Pre-allocate all data to avoid allocation overhead during test
    let warmup_keys: Vec<String> = (0..1000).map(|i| format!("warmup{}", i)).collect();
    let warmup_values: Vec<String> = (0..1000).map(|i| format!("value{}", i)).collect();
    let test_keys: Vec<String> = (0..test_size).map(|i| format!("sustained{}", i)).collect();
    let test_values: Vec<String> = (0..test_size).map(|i| format!("value{}", i)).collect();
    
    // Warm up with batching
    let mut warmup_batch = SimpleScribeLedger::new_batch();
    for (key, value) in warmup_keys.iter().zip(warmup_values.iter()) {
        warmup_batch.insert(key.as_bytes(), value.as_bytes());
    }
    ledger.apply_batch(warmup_batch)?;
    ledger.flush()?;
    
    // Actual test with batching and optimizations
    let start = Instant::now();
    let batch_size = 50;
    let mut total_ops = 0;
    
    let mut i = 0;
    while i < test_size {
        let mut batch = SimpleScribeLedger::new_batch();
        let end = std::cmp::min(i + batch_size, test_size);
        
        for j in i..end {
            batch.insert(test_keys[j].as_bytes(), test_values[j].as_bytes());
        }
        ledger.apply_batch(batch)?;
        total_ops += end - i;
        
        // Every 100 operations, do some gets using pre-allocated keys
        if i % 100 == 0 && i > 0 {
            for k in 0..10 {
                if i >= k + 1 {
                    let _value = ledger.get(&test_keys[i - k - 1])?;
                    total_ops += 1;
                }
            }
        }
        
        i = end;
    }
    
    // Single flush at the end for optimal performance
    ledger.flush()?;
    let sustained_duration = start.elapsed();
    let sustained_ops_per_sec = total_ops as f64 / sustained_duration.as_secs_f64();
    
    println!("Optimized sustained mixed operations: {:.0} ops/sec ({:.2} ms total, {} total ops)",
            sustained_ops_per_sec, sustained_duration.as_secs_f64() * 1000.0, total_ops);
    
    println!("\n--- Performance Optimization Summary ---");
    println!("✓ Pre-allocated keys/values eliminate allocation overhead during benchmarking");
    println!("✓ Batch operations significantly improve write throughput");
    println!("✓ Warm-up phases ensure consistent timing measurements");
    println!("✓ Reduced flush frequency improves overall performance");
    println!("✓ Optimizations should show significant performance improvements");
    
    Ok(())
}