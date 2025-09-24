use anyhow::Result;
use clap::Parser;
use scribe_ledger::{Config, ScribeLedger};
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "scribe-node")]
#[command(about = "Hyra Scribe Ledger Node")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config.toml")]
    config: String,
    
    /// Node ID
    #[arg(short, long)]
    node_id: Option<String>,
    
    /// Listen address
    #[arg(short, long, default_value = "0.0.0.0:8080")]
    listen: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    
    // Display ASCII art banner
    println!(r#"
╦ ╦╦ ╦╦═╗╔═╗  ╔═╗╔═╗╦═╗╦╔╗ ╔═╗  ╦  ╔═╗╔╦╗╔═╗╔═╗╦═╗
╠═╣╚╦╝╠╦╝╠═╣  ╚═╗║  ╠╦╝║╠╩╗║╣   ║  ║╣  ║║║ ╦║╣ ╠╦╝
╩ ╩ ╩ ╩╚═╩ ╩  ╚═╝╚═╝╩╚═╩╚═╝╚═╝  ╩═╝╚═╝═╩╝╚═╝╚═╝╩╚═
"#);
    println!("🔗 Verifiable, Durable Off-Chain Storage for AI Ecosystem");
    println!("📡 Node starting up...\n");
    
    info!("Starting Scribe Ledger Node");
    info!("Config file: {}", cli.config);
    println!("Listen address: {}", cli.listen);
    
    // Load configuration
    let config = Config::from_file(&cli.config).unwrap_or_else(|_| {
        info!("Using default configuration");
        Config::default()
    });
    
    // Create and start the ledger
    let ledger = match ScribeLedger::new(config).await {
        Ok(ledger) => ledger,
        Err(e) => {
            error!("Failed to create ledger: {}", e);
            std::process::exit(1);
        }
    };
    
    if let Err(e) = ledger.start().await {
        error!("Failed to start ledger: {}", e);
        std::process::exit(1);
    }
    
    Ok(())
}