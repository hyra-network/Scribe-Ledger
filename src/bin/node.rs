use anyhow::Result;
use clap::Parser;
use scribe_ledger::{Config, ScribeLedger};
use tracing::warn;
use colored::*;

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
    
    // Display enhanced ASCII art banner with colors
    println!("{}", r#"
╦ ╦╦ ╦╦═╗╔═╗  ╔═╗╔═╗╦═╗╦╔╗ ╔═╗  ╦  ╔═╗╔╦╗╔═╗╔═╗╦═╗
╠═╣╚╦╝╠╦╝╠═╣  ╚═╗║  ╠╦╝║╠╩╗║╣   ║  ║╣  ║║║ ╦║╣ ╠╦╝
╩ ╩ ╩ ╩╚═╩ ╩  ╚═╝╚═╝╩╚═╩╚═╝╚═╝  ╩═╝╚═╝═╩╝╚═╝╚═╝╩╚═
"#.bright_cyan());
    
    println!("{} {}", "🔗".bright_yellow(), "Verifiable, Durable Off-Chain Storage for AI Ecosystem".bright_white().bold());
    println!("{} {}\n", "📡".bright_blue(), "Distributed Consensus & Real-time Monitoring".bright_white());
    
    // Load configuration first
    let config = Config::from_file(&cli.config).unwrap_or_else(|e| {
        warn!("Failed to load config file '{}': {}", cli.config, e);
        warn!("Using default configuration");
        Config::default()
    });
    
    // Display comprehensive startup information
    print_startup_info(&config, &cli.config);
    
    println!("{}", "🚀 INITIALIZING NODE...".bright_green().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_green());
    
    // Create and start the ledger
    let ledger = match ScribeLedger::new(config).await {
        Ok(ledger) => {
            println!("{} {}", "✅".bright_green(), "Scribe Ledger initialized successfully".bright_white());
            ledger
        },
        Err(e) => {
            println!("{} {}: {}", "❌".bright_red(), "Failed to initialize ledger".bright_red().bold(), e);
            std::process::exit(1);
        }
    };
    
    println!("{}", "🌟 STARTING SERVICES...".bright_yellow().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_yellow());
    
    if let Err(e) = ledger.start().await {
        println!("{} {}: {}", "❌".bright_red(), "Failed to start services".bright_red().bold(), e);
        std::process::exit(1);
    }
    
    Ok(())
}

fn print_startup_info(config: &Config, config_file: &str) {
    println!("{}", "📋 CONFIGURATION OVERVIEW".bright_magenta().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_magenta());
    
    // Node Information
    println!("{} {}", "🏷️  Node ID:".bright_cyan().bold(), config.node.id.bright_white().bold());
    println!("{} {}", "📁 Data Directory:".bright_cyan().bold(), config.node.data_dir.bright_white());
    println!("{} {}", "⚙️  Config File:".bright_cyan().bold(), config_file.bright_white());
    
    println!();
    
    // Network Configuration
    println!("{}", "🌐 NETWORK CONFIGURATION".bright_blue().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue());
    
    let http_url = format!("http://{}:{}", 
        if config.network.listen_addr == "0.0.0.0" { "localhost" } else { &config.network.listen_addr },
        config.network.client_port
    );
    
    println!("{} {}:{}", "📡 Listen Address:".bright_blue().bold(), 
        config.network.listen_addr.bright_white(), 
        config.network.client_port.to_string().bright_yellow().bold()
    );
    println!("{} {}", "🌍 HTTP API URL:".bright_blue().bold(), http_url.bright_green().underline());
    println!("{} {}", "🔗 Raft TCP Port:".bright_blue().bold(), config.network.raft_tcp_port.to_string().bright_yellow().bold());
    
    println!();
    
    // API Endpoints
    println!("{}", "🔌 API ENDPOINTS".bright_green().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_green());
    
    let base_url = format!("http://{}:{}", 
        if config.network.listen_addr == "0.0.0.0" { "localhost" } else { &config.network.listen_addr },
        config.network.client_port
    );
    
    println!("   {} PUT/GET {}/{{key}}  - Data operations", "💾".bright_yellow(), base_url.bright_white());
    println!("   {} GET {}{}  - Raft status", "📊".bright_cyan(), base_url.bright_white(), "/raft/status".bright_green());
    println!("   {} GET {}{}  - Performance metrics", "📈".bright_magenta(), base_url.bright_white(), "/raft/metrics".bright_green());
    println!("   {} GET {}{}   - Recent events", "📋".bright_blue(), base_url.bright_white(), "/raft/events".bright_green());
    
    let ws_url = format!("ws://{}:{}/raft/live", 
        if config.network.listen_addr == "0.0.0.0" { "localhost" } else { &config.network.listen_addr },
        config.network.client_port
    );
    println!("   {} WS  {}   - Live monitoring", "🔄".bright_red(), ws_url.bright_yellow().underline());
    
    println!();
    
    // Storage Configuration
    println!("{}", "💾 STORAGE CONFIGURATION".bright_purple().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_purple());
    
    println!("{} {}", "📦 Buffer Size:".bright_purple().bold(), 
        format!("{} MB", config.storage.buffer_size / 1024 / 1024).bright_white());
    println!("{} {}", "📐 Segment Limit:".bright_purple().bold(), 
        format!("{} GB", config.storage.segment_size_limit / 1024 / 1024 / 1024).bright_white());
    
    // Sled Database Info
    let db_path = std::path::Path::new(&config.node.data_dir).join("scribe.db");
    println!("{} {}", "🗄️  Sled Database:".bright_purple().bold(), db_path.display().to_string().bright_white());
    
    // Check if database exists and get size
    if db_path.exists() {
        if let Ok(size) = get_directory_size(&db_path) {
            println!("{} {}", "📊 Database Size:".bright_purple().bold(), 
                format_bytes(size).bright_white());
        }
    } else {
        println!("{} {}", "📊 Database Status:".bright_purple().bold(), "New database (will be created)".bright_yellow());
    }
    
    println!();
    
    // S3 Configuration
    println!("{}", "☁️  S3 STORAGE CONFIGURATION".bright_cyan().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_cyan());
    
    if let Some(endpoint) = &config.storage.s3.endpoint {
        println!("{} {}", "🌐 Endpoint:".bright_cyan().bold(), endpoint.bright_white());
    } else {
        println!("{} {}", "🌐 Endpoint:".bright_cyan().bold(), "AWS S3 (default)".bright_white());
    }
    
    println!("{} {}", "🪣 Bucket:".bright_cyan().bold(), config.storage.s3.bucket.bright_white());
    println!("{} {}", "🌍 Region:".bright_cyan().bold(), config.storage.s3.region.bright_white());
    
    if let Some(access_key) = &config.storage.s3.access_key {
        println!("{} {}", "🔑 Access Key:".bright_cyan().bold(), 
            format!("{}***", &access_key[..access_key.len().min(4)]).bright_white());
    } else {
        println!("{} {}", "🔑 Access Key:".bright_cyan().bold(), "Using AWS credentials".bright_yellow());
    }
    
    println!("{} {}", "🔧 Path Style:".bright_cyan().bold(), 
        if config.storage.s3.path_style { "Enabled (MinIO compatible)".bright_green() } else { "Disabled".bright_white() });
    
    println!();
    
    // Consensus Configuration
    println!("{}", "🤝 CONSENSUS CONFIGURATION".bright_red().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_red());
    
    println!("{} {}", "⏱️  Election Timeout:".bright_red().bold(), 
        format!("{} ms", config.consensus.election_timeout_ms).bright_white());
    println!("{} {}", "💓 Heartbeat Interval:".bright_red().bold(), 
        format!("{} ms", config.consensus.heartbeat_interval_ms).bright_white());
    
    if config.consensus.peers.is_empty() {
        println!("{} {}", "👥 Cluster Peers:".bright_red().bold(), "Single node (development mode)".bright_yellow());
    } else {
        println!("{} {}", "👥 Cluster Peers:".bright_red().bold(), config.consensus.peers.len().to_string().bright_white());
        for (i, peer) in config.consensus.peers.iter().enumerate() {
            println!("   {}. {}", i + 1, peer.bright_white());
        }
    }
    
    println!();
    
    // System Information
    println!("{}", "�️  SYSTEM INFORMATION".bright_yellow().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_yellow());
    
    // Get system info
    let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string());
    let user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    
    println!("{} {}", "🏠 Hostname:".bright_yellow().bold(), hostname.bright_white());
    println!("{} {}", "👤 User:".bright_yellow().bold(), user.bright_white());
    println!("{} {}", "📊 Log Level:".bright_yellow().bold(), 
        std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()).to_uppercase().bright_white());
    
    println!();
}

fn get_directory_size(path: &std::path::Path) -> std::io::Result<u64> {
    let mut size = 0;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                size += get_directory_size(&path)?;
            } else {
                size += std::fs::metadata(&path)?.len();
            }
        }
    } else {
        size = std::fs::metadata(path)?.len();
    }
    Ok(size)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}