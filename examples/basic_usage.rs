use anyhow::Result;
use hyra_scribe_ledger::SimpleScribeLedger;

fn main() -> Result<()> {
    println!("=== Basic Usage Example ===");

    // Create a new storage instance (data will be stored in "./example_data" directory)
    let ledger = SimpleScribeLedger::new("./example_data")?;

    // Store some data
    ledger.put("user:alice", "Alice Smith")?;
    ledger.put("user:bob", "Bob Johnson")?;
    ledger.put("counter", "42")?;

    // Retrieve and display the data
    if let Some(alice) = ledger.get("user:alice")? {
        println!("Found: {}", String::from_utf8_lossy(&alice));
    }

    if let Some(bob) = ledger.get("user:bob")? {
        println!("Found: {}", String::from_utf8_lossy(&bob));
    }

    if let Some(counter) = ledger.get("counter")? {
        println!("Counter: {}", String::from_utf8_lossy(&counter));
    }

    // Try to get a non-existent key
    match ledger.get("user:charlie")? {
        Some(value) => println!("Found Charlie: {}", String::from_utf8_lossy(&value)),
        None => println!("User 'charlie' not found"),
    }

    // Ensure data is written to disk
    ledger.flush()?;

    println!("Storage contains {} keys", ledger.len());
    println!("Example completed successfully!");

    Ok(())
}
