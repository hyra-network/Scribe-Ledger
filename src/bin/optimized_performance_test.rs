use simple_scribe_ledger::SimpleScribeLedger;
use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("Optimized Simple Scribe Ledger Performance Test");
    println!("===============================================");
    
    // Test different operation counts
    let test_sizes = vec![100, 1000, 5000, 10000];
    
    for size in test_sizes {
        println!("\nTesting with {} operations:", size);
        
        // Pre-generate keys and values to eliminate allocation overhead
        let keys: Vec<Vec<u8>> = (0..size).map(|i| format!("key{}", i).into_bytes()).collect();
        let values: Vec<Vec<u8>> = (0..size).map(|i| format!("value{}", i).into_bytes()).collect();
        
        // Test PUT operations with batching - matching benchmark pattern
        let start = Instant::now();
        
        // Run multiple iterations like benchmark does to include setup overhead
        let iterations = std::cmp::max(1, 100 / size); // More iterations for smaller sizes
        for _ in 0..iterations {
            let ledger = SimpleScribeLedger::temp()?;
            
            // Use batching for better performance
            let batch_size = 100;
            let mut i = 0;
            while i < size {
                let mut batch = SimpleScribeLedger::new_batch();
                let end = std::cmp::min(i + batch_size, size);
                
                for j in i..end {
                    batch.insert(keys[j].as_slice(), values[j].as_slice());
                }
                
                ledger.apply_batch(batch)?;
                i = end;
            }
            // Only flush at the end, not during operations
            ledger.flush()?;
        }
        
        let put_duration = start.elapsed();
        let total_ops = size * iterations;
        let put_ops_per_sec = total_ops as f64 / put_duration.as_secs_f64();
        
        println!("  PUT operations (batched): {:.0} ops/sec ({:.2} ms total)", 
                put_ops_per_sec, put_duration.as_secs_f64() * 1000.0);
        
        // Test GET operations with pre-allocated keys - matching benchmark pattern
        let start = Instant::now();
        
        for _ in 0..iterations {
            let ledger = SimpleScribeLedger::temp()?;
            
            // Pre-populate using batching
            let batch_size = 100;
            let mut i = 0;
            while i < size {
                let mut batch = SimpleScribeLedger::new_batch();
                let end = std::cmp::min(i + batch_size, size);
                
                for j in i..end {
                    batch.insert(keys[j].as_slice(), values[j].as_slice());
                }
                
                ledger.apply_batch(batch)?;
                i = end;
            }
            ledger.flush()?;
            
            for key in &keys {
                let _value = ledger.get(key.as_slice())?;
            }
        }
        
        let get_duration = start.elapsed();
        let total_get_ops = size * iterations;
        let get_ops_per_sec = total_get_ops as f64 / get_duration.as_secs_f64();
        
        println!("  GET operations (optimized): {:.0} ops/sec ({:.2} ms total)", 
                get_ops_per_sec, get_duration.as_secs_f64() * 1000.0);
                
        // Test MIXED operations with pre-allocated data - matching benchmark pattern
        let start = Instant::now();
        
        for _ in 0..iterations {
            let ledger = SimpleScribeLedger::temp()?;
            
            // Half puts, half gets with batching
            let half_size = size / 2;
            
            // Batch the puts
            let mut batch = SimpleScribeLedger::new_batch();
            for i in 0..half_size {
                batch.insert(keys[i].as_slice(), values[i].as_slice());
            }
            ledger.apply_batch(batch)?;
            
            // Then get them back
            for i in 0..half_size {
                let _value = ledger.get(keys[i].as_slice())?;
            }
            
            // Only flush at the end
            ledger.flush()?;
        }
        let mixed_duration = start.elapsed();
        let total_mixed_ops = size * iterations;
        let mixed_ops_per_sec = total_mixed_ops as f64 / mixed_duration.as_secs_f64();
        
        println!("  MIXED operations (optimized): {:.0} ops/sec ({:.2} ms total)", 
                mixed_ops_per_sec, mixed_duration.as_secs_f64() * 1000.0);
    }
    
    // Sustained performance test with optimizations
    println!("\n--- Optimized Sustained Performance Test (10,000 operations) ---");
    let ledger = SimpleScribeLedger::temp()?;
    let test_size = 10000;
    
    // Pre-generate all keys and values
    let warmup_keys: Vec<Vec<u8>> = (0..1000).map(|i| format!("warmup{}", i).into_bytes()).collect();
    let warmup_values: Vec<Vec<u8>> = (0..1000).map(|i| format!("value{}", i).into_bytes()).collect();
    
    // Warm up with batch operations
    let mut batch = SimpleScribeLedger::new_batch();
    for (key, value) in warmup_keys.iter().zip(warmup_values.iter()) {
        batch.insert(key.as_slice(), value.as_slice());
    }
    ledger.apply_batch(batch)?;
    ledger.flush()?;
    
    // Pre-generate test data
    let test_keys: Vec<Vec<u8>> = (0..test_size).map(|i| format!("sustained{}", i).into_bytes()).collect();
    let test_values: Vec<Vec<u8>> = (0..test_size).map(|i| format!("value{}", i).into_bytes()).collect();
    
    // Actual test with batching
    let start = Instant::now();
    let batch_size = 50;
    let mut total_ops = 0;
    
    let mut i = 0;
    while i < test_size {
        let mut batch = SimpleScribeLedger::new_batch();
        let end = std::cmp::min(i + batch_size, test_size);
        
        for j in i..end {
            batch.insert(test_keys[j].as_slice(), test_values[j].as_slice());
        }
        ledger.apply_batch(batch)?;
        total_ops += end - i;
        
        // Every 100 operations, do some gets
        if i % 100 == 0 && i > 0 {
            for k in 0..10 {
                if i >= k + 1 {
                    let _value = ledger.get(test_keys[i - k - 1].as_slice())?;
                    total_ops += 1;
                }
            }
        }
        
        i = end;
    }
    
    // Single flush at the end
    ledger.flush()?;
    let sustained_duration = start.elapsed();
    let sustained_ops_per_sec = total_ops as f64 / sustained_duration.as_secs_f64();
    
    println!("Optimized sustained mixed operations: {:.0} ops/sec ({:.2} ms total, {} total ops)",
            sustained_ops_per_sec, sustained_duration.as_secs_f64() * 1000.0, total_ops);
    
    println!("\n--- Performance Comparison Summary ---");
    println!("✓ Optimized simple scribe ledger with tuned sled configuration");
    println!("✓ Pre-allocated keys/values eliminate runtime string allocation overhead");  
    println!("✓ Batch operations significantly improve write throughput");
    println!("✓ Reduced flush frequency improves overall performance");
    println!("✓ Performance should be significantly higher than baseline");
    
    Ok(())
}