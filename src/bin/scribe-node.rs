//! Scribe Node - Distributed ledger node binary
//!
//! This is the main executable for running a distributed Hyra Scribe Ledger node.
//! It provides CLI interface for node configuration, cluster initialization, and
//! graceful shutdown handling.

use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, put},
    Router,
};
use bytes::Bytes;
use clap::Parser;
use hyra_scribe_ledger::api::{DistributedApi, ReadConsistency};
use hyra_scribe_ledger::cluster::{ClusterConfig, ClusterInitializer, InitMode};
use hyra_scribe_ledger::config::Config;
use hyra_scribe_ledger::consensus::ConsensusNode;
use hyra_scribe_ledger::discovery::{DiscoveryConfig, DiscoveryService};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Hyra Scribe Ledger - Distributed Node
#[derive(Parser, Debug)]
#[command(name = "scribe-node")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Distributed ledger node with Raft consensus", long_about = None)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Node ID (overrides config file)
    #[arg(short, long, value_name = "ID")]
    node_id: Option<u64>,

    /// Bootstrap a new cluster (first node)
    #[arg(short, long)]
    bootstrap: bool,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing/logging
    setup_logging(&cli.log_level)?;

    // Print startup banner
    print_banner();

    // Load configuration
    let mut config = load_config(&cli)?;

    // Override node ID if provided via CLI
    if let Some(node_id) = cli.node_id {
        info!("Overriding node ID from CLI: {}", node_id);
        config.node.id = node_id;
    }

    // Validate configuration (skip validation check for now)
    // config.validate()?;

    info!(
        "Starting node {} at {}",
        config.node.id, config.node.address
    );
    info!("Data directory: {:?}", config.node.data_dir);
    info!("Client port: {}", config.network.client_port);
    info!("Raft port: {}", config.network.raft_port);

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(&config.node.data_dir)?;

    // Initialize storage
    let db_path = config.node.data_dir.join("db");
    let db = sled::open(&db_path)?;
    info!("Storage initialized at {:?}", db_path);

    // Create consensus node
    let consensus = Arc::new(
        ConsensusNode::new(config.node.id, db)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create consensus node: {}", e))?,
    );
    info!("Consensus node created with ID {}", config.node.id);

    // Create discovery service
    let discovery_config = DiscoveryConfig {
        node_id: config.node.id,
        raft_addr: format!("{}:{}", config.node.address, config.network.raft_port)
            .parse()
            .unwrap(),
        client_addr: format!("{}:{}", config.node.address, config.network.client_port)
            .parse()
            .unwrap(),
        discovery_port: config.network.raft_port,
        broadcast_addr: config.node.address.clone(),
        seed_addrs: config.network.seed_peers.clone(),
        heartbeat_interval_ms: 500,
        failure_timeout_ms: 1500,
    };

    let discovery = Arc::new(DiscoveryService::new(discovery_config)?);
    info!("Discovery service created");

    // Start discovery service
    discovery.start().await?;
    info!("Discovery service started");

    // Create cluster initializer
    let cluster_config = ClusterConfig {
        mode: if cli.bootstrap {
            InitMode::Bootstrap
        } else {
            InitMode::Join
        },
        seed_addrs: Vec::new(),
        discovery_timeout_ms: 5000,
        min_peers_for_join: 1,
    };

    let initializer = ClusterInitializer::new(discovery.clone(), consensus.clone(), cluster_config);

    // Initialize cluster
    info!(
        "Initializing cluster in {} mode",
        if cli.bootstrap { "Bootstrap" } else { "Join" }
    );
    if let Err(e) = initializer.initialize().await {
        error!("Failed to initialize cluster: {}", e);
        return Err(e.into());
    }

    // Create distributed API
    let api = Arc::new(DistributedApi::new(consensus.clone()));

    // Create app state
    let app_state = AppState {
        api,
        node_id: config.node.id,
    };

    // Start HTTP server
    let http_addr = format!("0.0.0.0:{}", config.network.client_port);
    info!("Starting HTTP API server on {}", http_addr);
    
    let http_addr_clone = http_addr.clone();
    let http_server = tokio::spawn(async move {
        if let Err(e) = start_http_server(&http_addr_clone, app_state).await {
            error!("HTTP server error: {}", e);
        }
    });

    info!("Node {} is ready", config.node.id);
    info!("HTTP API available at http://{}", http_addr);
    info!("Press Ctrl+C to shutdown gracefully");

    // Wait for shutdown signal
    wait_for_shutdown_signal().await;
    
    // Abort HTTP server
    http_server.abort();

    // Graceful shutdown
    info!("Shutdown signal received, stopping node...");

    // Stop discovery service
    discovery.stop();
    info!("Discovery service stopped");

    // Shutdown consensus node
    if let Err(e) = consensus.shutdown().await {
        error!("Error shutting down consensus: {}", e);
    } else {
        info!("Consensus node stopped");
    }

    info!("Node {} shutdown complete", config.node.id);
    Ok(())
}

/// Setup logging with tracing-subscriber
fn setup_logging(log_level: &str) -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "scribe_node={},hyra_scribe_ledger={}",
            log_level, log_level
        ))
    });

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .with(filter)
        .init();

    Ok(())
}

/// Print startup banner
fn print_banner() {
    println!(
        r#"
╔═══════════════════════════════════════════════════════╗
║                                                       ║
║           Hyra Scribe Ledger Node                    ║
║           Distributed Key-Value Store                ║
║                                                       ║
║           Version: {}                         ║
╚═══════════════════════════════════════════════════════╝
"#,
        env!("CARGO_PKG_VERSION")
    );
}

/// Load configuration from file or use defaults
fn load_config(cli: &Cli) -> Result<Config> {
    if let Some(config_path) = &cli.config {
        info!("Loading configuration from {:?}", config_path);
        Ok(Config::from_file(config_path.to_str().unwrap())?)
    } else {
        warn!("No config file specified, using default configuration");
        // Use default config for node 1
        let node_id = cli.node_id.unwrap_or(1);
        Ok(Config::default_for_node(node_id))
    }
}

// HTTP API types
#[derive(Clone)]
struct AppState {
    api: Arc<DistributedApi>,
    node_id: u64,
}

#[derive(Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    node_id: u64,
}

// HTTP API handlers
async fn health_handler(State(state): State<AppState>) -> impl IntoResponse {
    axum::Json(HealthResponse {
        status: "ok".to_string(),
        node_id: state.node_id,
    })
}

async fn put_handler(
    State(state): State<AppState>,
    Path(key): Path<String>,
    body: Bytes,
) -> impl IntoResponse {
    let value = body.to_vec();
    match state.api.put(key.into_bytes(), value).await {
        Ok(_) => (StatusCode::OK, "OK".to_string()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {}", e),
        ),
    }
}

async fn get_handler(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    match state.api.get(key.into_bytes(), ReadConsistency::Stale).await {
        Ok(Some(value)) => (
            StatusCode::OK,
            String::from_utf8_lossy(&value).to_string(),
        ),
        Ok(None) => (StatusCode::NOT_FOUND, "Not found".to_string()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {}", e),
        ),
    }
}

async fn delete_handler(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    match state.api.delete(key.into_bytes()).await {
        Ok(_) => (StatusCode::OK, "OK".to_string()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error: {}", e),
        ),
    }
}

async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    let metrics = state.api.metrics().await;
    axum::Json(metrics)
}

/// Start HTTP API server
async fn start_http_server(addr: &str, state: AppState) -> Result<()> {
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/:key", put(put_handler))
        .route("/:key", get(get_handler))
        .route("/:key", delete(delete_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("HTTP server listening on {}", addr);
    
    axum::serve(listener, app).await?;
    Ok(())
}

/// Wait for SIGTERM or SIGINT signal
async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm =
            signal(SignalKind::terminate()).expect("Failed to create SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to create SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM signal");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT signal");
            }
        }
    }

    #[cfg(not(unix))]
    {
        signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        info!("Received Ctrl+C signal");
    }
}
