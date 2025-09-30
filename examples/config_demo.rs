use simple_scribe_ledger::config::Config;
use simple_scribe_ledger::error::ScribeError;
use simple_scribe_ledger::types::{Request, Response};

fn main() -> Result<(), ScribeError> {
    println!("=== Simple Scribe Ledger - Tasks 1.2 & 1.3 Demo ===\n");

    // Task 1.2: Configuration System
    println!("1. Configuration System Demo");
    println!("   Loading configuration from config.toml...");
    
    let config = Config::from_file("config.toml")?;
    println!("   ✓ Configuration loaded successfully!");
    println!("     - Node ID: {}", config.node.id);
    println!("     - Client Port: {}", config.network.client_port);
    println!("     - Raft Port: {}", config.network.raft_port);
    println!("     - Data Directory: {}", config.node.data_dir.display());
    println!("     - Election Timeout: {:?}", config.election_timeout());
    println!("     - Heartbeat Interval: {:?}", config.heartbeat_interval());

    // Task 1.3: Error Handling
    println!("\n2. Error Handling Demo");
    println!("   Testing error conversions...");
    
    // Test configuration error
    match Config::from_file("nonexistent.toml") {
        Err(e) => println!("   ✓ Config error handled: {}", e),
        Ok(_) => println!("   Unexpected success"),
    }

    // Task 1.3: Type System
    println!("\n3. Type System Demo");
    println!("   Creating and serializing requests...");
    
    let put_request = Request::Put {
        key: b"example_key".to_vec(),
        value: b"example_value".to_vec(),
    };
    println!("   ✓ Put Request: {:?}", put_request);
    
    let get_request = Request::Get {
        key: b"example_key".to_vec(),
    };
    println!("   ✓ Get Request: {:?}", get_request);
    
    let response = Response::GetOk {
        value: Some(b"example_value".to_vec()),
    };
    println!("   ✓ Response: {:?}", response);

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&put_request)?;
    println!("\n   JSON Serialization:");
    println!("{}", json);

    println!("\n=== Demo Complete ===");
    println!("\nAll features implemented:");
    println!("✓ Configuration system with TOML support");
    println!("✓ Environment variable overrides");
    println!("✓ Configuration validation");
    println!("✓ Comprehensive error handling");
    println!("✓ Type system with Request/Response types");
    println!("✓ Serialization support");
    
    Ok(())
}
