use simple_scribe_ledger::SimpleScribeLedger;
use anyhow::Result;

fn main() -> Result<()> {
    println!("Simple Scribe Ledger Demo");
    println!("=========================");
    
    // Create a temporary storage instance
    let ledger = SimpleScribeLedger::temp()?;
    
    // Demonstrate put operation
    println!("Putting key-value pairs...");
    ledger.put("name", "Simple Scribe Ledger")?;
    ledger.put("version", "0.1.0")?;
    ledger.put("language", "Rust")?;
    
    // Demonstrate get operations
    println!("Getting values...");
    if let Some(value) = ledger.get("name")? {
        println!("name: {}", String::from_utf8_lossy(&value));
    }
    
    if let Some(value) = ledger.get("version")? {
        println!("version: {}", String::from_utf8_lossy(&value));
    }
    
    if let Some(value) = ledger.get("language")? {
        println!("language: {}", String::from_utf8_lossy(&value));
    }
    
    // Try getting a non-existent key
    match ledger.get("nonexistent")? {
        Some(value) => println!("nonexistent: {}", String::from_utf8_lossy(&value)),
        None => println!("Key 'nonexistent' not found"),
    }
    
    println!("Total keys: {}", ledger.len());
    
    Ok(())
}
