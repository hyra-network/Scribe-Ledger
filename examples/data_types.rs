use anyhow::Result;
use hyra_scribe_ledger::SimpleScribeLedger;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct User {
    name: String,
    age: u32,
    email: String,
}

fn main() -> Result<()> {
    println!("=== Working with Different Data Types ===");

    let ledger = SimpleScribeLedger::new("./tutorial_data")?;

    // Store strings
    ledger.put("greeting", "Hello, World!")?;
    println!("Stored greeting");

    // Store numbers (as strings)
    ledger.put("balance", "1250.75")?;
    println!("Stored balance");

    // Store JSON (for complex data structures)
    let user = User {
        name: "Alice".to_string(),
        age: 30,
        email: "alice@example.com".to_string(),
    };
    let user_json = serde_json::to_string(&user)?;
    ledger.put("user:alice", user_json)?;
    println!("Stored user data as JSON");

    // Store binary data
    let binary_data = vec![0u8, 1, 2, 3, 255, 128, 64];
    ledger.put("binary_data", &binary_data)?;
    println!("Stored binary data");

    println!("\n--- Retrieving Data ---");

    // Retrieve and display string data
    if let Some(greeting) = ledger.get("greeting")? {
        println!("Greeting: {}", String::from_utf8_lossy(&greeting));
    }

    // Retrieve and display number
    if let Some(balance) = ledger.get("balance")? {
        let balance_str = String::from_utf8_lossy(&balance);
        let balance_num: f64 = balance_str.parse().unwrap_or(0.0);
        println!("Balance: ${:.2}", balance_num);
    }

    // Retrieve and parse JSON data
    if let Some(user_data) = ledger.get("user:alice")? {
        let user_str = String::from_utf8_lossy(&user_data);
        let user: User = serde_json::from_str(&user_str)?;
        println!("User: {:?}", user);
        println!("  - Name: {}", user.name);
        println!("  - Age: {}", user.age);
        println!("  - Email: {}", user.email);
    }

    // Retrieve and display binary data
    if let Some(binary) = ledger.get("binary_data")? {
        println!("Binary data: {:?}", binary);
    }

    ledger.flush()?;
    println!("\nTotal items stored: {}", ledger.len());
    println!("Data types example completed successfully!");

    Ok(())
}
