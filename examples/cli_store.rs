use simple_scribe_ledger::SimpleScribeLedger;
use anyhow::Result;
use std::io::{self, Write};

fn main() -> Result<()> {
    println!("=== Simple Key-Value Store Application ===");
    println!("Commands:");
    println!("  put <key> <value> - Store a key-value pair");
    println!("  get <key>         - Retrieve a value by key");
    println!("  list              - Show number of stored keys");
    println!("  clear             - Remove all data");
    println!("  quit              - Exit the application");
    println!();
    
    let ledger = SimpleScribeLedger::new("./my_store")?;
    
    loop {
        print!("store> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = input.splitn(3, ' ').collect();
        
        match parts.as_slice() {
            ["put", key, value] => {
                ledger.put(key, value)?;
                println!("✓ Stored: {} = {}", key, value);
            }
            ["get", key] => {
                match ledger.get(key)? {
                    Some(value) => println!("✓ Found: {} = {}", key, String::from_utf8_lossy(&value)),
                    None => println!("✗ Key '{}' not found", key),
                }
            }
            ["list"] => {
                let count = ledger.len();
                if count == 0 {
                    println!("Database is empty");
                } else {
                    println!("Database contains {} key(s)", count);
                }
            }
            ["clear"] => {
                ledger.clear()?;
                println!("✓ Database cleared");
            }
            ["quit"] | ["exit"] => {
                println!("Saving data and exiting...");
                break;
            }
            ["help"] => {
                println!("Available commands:");
                println!("  put <key> <value> - Store a key-value pair");
                println!("  get <key>         - Retrieve a value by key");
                println!("  list              - Show number of stored keys");
                println!("  clear             - Remove all data");
                println!("  help              - Show this help message");
                println!("  quit              - Exit the application");
            }
            _ => {
                if !input.trim().is_empty() {
                    println!("✗ Invalid command. Type 'help' for available commands.");
                }
            }
        }
        
        // Ensure data is persisted after each operation
        ledger.flush()?;
    }
    
    println!("Goodbye!");
    Ok(())
}