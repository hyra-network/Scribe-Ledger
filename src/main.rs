use anyhow::Result;
use hyra_scribe_ledger::SimpleScribeLedger;
use std::sync::Arc;

fn main() -> Result<()> {
    println!("Optimized Hyra Scribe Ledger Demo");
    println!("==================================");

    // Keep database handle alive and reuse it - don't reopen for each operation
    let ledger = Arc::new(SimpleScribeLedger::new("./optimized_demo_db")?);

    // Simulate multiple operations that would normally reopen the database
    // This is the problematic pattern mentioned in the problem statement:
    //   let ledger = SimpleScribeLedger::new(path)?; // <- Don't do this every time!
    //   ledger.operation()?;
    // Instead, we keep one instance and reuse it:

    println!("Demonstrating batch operations with persistent handle...");

    // Batch 1: User data
    println!("Storing user data...");
    perform_user_operations(Arc::clone(&ledger))?;

    // Batch 2: System data
    println!("Storing system data...");
    perform_system_operations(Arc::clone(&ledger))?;

    // Batch 3: Application data
    println!("Storing application data...");
    perform_app_operations(Arc::clone(&ledger))?;

    // Query all data
    println!("\nRetrieving all stored data:");
    query_all_data(&ledger)?;

    println!("\nDatabase statistics:");
    println!("Total keys stored: {}", ledger.len());
    println!("Is empty: {}", ledger.is_empty());

    // Only flush once at the end, not after each operation
    ledger.flush()?;
    println!("✓ All data persisted to disk");

    Ok(())
}

// Simulate operations that would typically reopen the database each time
// BAD: fn perform_user_operations(path: &str) -> Result<()> {
//          let ledger = SimpleScribeLedger::new(path)?; // <- Expensive!
// GOOD: Pass the existing database handle
fn perform_user_operations(ledger: Arc<SimpleScribeLedger>) -> Result<()> {
    // Use batch operations for better performance
    let mut batch = SimpleScribeLedger::new_batch();

    batch.insert(b"user:1:name", b"Alice Johnson");
    batch.insert(b"user:1:email", b"alice@example.com");
    batch.insert(b"user:1:role", b"admin");

    batch.insert(b"user:2:name", b"Bob Smith");
    batch.insert(b"user:2:email", b"bob@example.com");
    batch.insert(b"user:2:role", b"user");

    ledger.apply_batch(batch)?;
    println!("  ✓ Stored 6 user records in one batch");

    Ok(())
}

fn perform_system_operations(ledger: Arc<SimpleScribeLedger>) -> Result<()> {
    // Another batch operation
    let mut batch = SimpleScribeLedger::new_batch();

    batch.insert(b"system:version", b"1.0.0");
    batch.insert(b"system:startup_time", b"2024-01-15T10:30:00Z");
    batch.insert(b"system:config:max_users", b"1000");

    ledger.apply_batch(batch)?;
    println!("  ✓ Stored 3 system records in one batch");

    Ok(())
}

fn perform_app_operations(ledger: Arc<SimpleScribeLedger>) -> Result<()> {
    // Mix of individual and batch operations
    ledger.put("app:name", "Hyra Scribe Ledger")?;
    ledger.put("app:language", "Rust")?;

    let mut batch = SimpleScribeLedger::new_batch();
    batch.insert(b"app:features:persistence", b"true");
    batch.insert(b"app:features:performance", b"optimized");
    batch.insert(b"app:features:safety", b"memory_safe");

    ledger.apply_batch(batch)?;
    println!("  ✓ Stored 5 application records (2 individual + 3 batch)");

    Ok(())
}

fn query_all_data(ledger: &SimpleScribeLedger) -> Result<()> {
    // Sample some of the stored data
    let keys_to_check = ["user:1:name", "user:2:role", "system:version", "app:name"];

    for key in &keys_to_check {
        if let Some(value) = ledger.get(key)? {
            println!("  {} = {}", key, String::from_utf8_lossy(&value));
        }
    }

    Ok(())
}
