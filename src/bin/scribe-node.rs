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

    // Initialize S3 storage if configured
    if let Some(s3_config) = &config.storage.s3 {
        info!("S3 storage configuration detected");
        info!("  Bucket: {}", s3_config.bucket);
        info!("  Region: {}", s3_config.region);
        if let Some(endpoint) = &s3_config.endpoint {
            info!("  Endpoint: {}", endpoint);
        }
        info!("  Path style: {}", s3_config.path_style);
        info!("  Pool size: {}", s3_config.pool_size);
        info!("  Timeout: {}s", s3_config.timeout_secs);
        info!("  Max retries: {}", s3_config.max_retries);

        // Create S3 storage config from the TOML config
        let s3_storage_config = hyra_scribe_ledger::storage::s3::S3StorageConfig {
            bucket: s3_config.bucket.clone(),
            region: s3_config.region.clone(),
            endpoint: s3_config.endpoint.clone(),
            access_key_id: s3_config.access_key_id.clone(),
            secret_access_key: s3_config.secret_access_key.clone(),
            path_style: s3_config.path_style,
            timeout_secs: s3_config.timeout_secs,
            max_retries: s3_config.max_retries,
        };

        // Try to initialize S3 storage (this will validate configuration)
        match hyra_scribe_ledger::storage::s3::S3Storage::new(s3_storage_config).await {
            Ok(_s3_storage) => {
                info!("✓ S3 storage initialized successfully");
                // S3 storage is ready for use by archival tier when needed
            }
            Err(e) => {
                warn!("Failed to initialize S3 storage: {}", e);
                warn!("Node will continue without S3 archival support");
            }
        }
    } else {
        info!("S3 storage not configured (running with local storage only)");
    }

    // Create consensus node
    let consensus = Arc::new(
        ConsensusNode::new_with_scribe_config(config.node.id, db, &config.consensus)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create consensus node: {}", e))?,
    );
    info!("Consensus node created with ID {}", config.node.id);

    // Create discovery service
    let discovery_config = hyra_scribe_ledger::discovery::DiscoveryConfig {
        node_id: config.node.id,
        raft_addr: format!("{}:{}", config.node.address, config.network.raft_port)
            .parse()
            .unwrap(),
        client_addr: format!("{}:{}", config.node.address, config.network.client_port)
            .parse()
            .unwrap(),
        discovery_port: config.discovery.discovery_port,
        broadcast_addr: config.discovery.broadcast_addr.clone(),
        seed_addrs: config.network.seed_peers.clone(),
        heartbeat_interval_ms: config.discovery.heartbeat_interval_ms,
        failure_timeout_ms: config.discovery.failure_timeout_ms,
        cluster_secret: config.discovery.cluster_secret.clone(),
    };

    let discovery = Arc::new(DiscoveryService::new(discovery_config)?);
    info!("Discovery service created");

    // Start discovery service
    discovery.start().await?;
    info!("Discovery service started");

    // Determine initialization mode
    // Check if the database already has Raft state (previous initialization)
    let has_existing_state = check_existing_raft_state(&db_path)?;

    let init_mode = if cli.bootstrap {
        if has_existing_state {
            warn!("Bootstrap flag provided but Raft state already exists");
            warn!("Cluster will attempt to join existing state instead");
            warn!(
                "To force bootstrap, delete the data directory: {:?}",
                config.node.data_dir
            );
            InitMode::Join
        } else {
            info!("Bootstrapping new cluster");
            InitMode::Bootstrap
        }
    } else if has_existing_state {
        info!("Existing Raft state detected, rejoining cluster");
        InitMode::Join
    } else {
        warn!("No existing Raft state found");
        warn!("If this is the first node, use --bootstrap flag");
        info!("Attempting to join existing cluster");
        InitMode::Join
    };

    // Create cluster initializer
    let cluster_config = ClusterConfig {
        mode: init_mode.clone(),
        seed_addrs: Vec::new(),
        discovery_timeout_ms: 5000,
        min_peers_for_join: 1,
    };

    let initializer = ClusterInitializer::new(discovery.clone(), consensus.clone(), cluster_config);

    // Initialize cluster
    info!(
        "Initializing cluster in {} mode",
        if matches!(init_mode, InitMode::Bootstrap) {
            "Bootstrap"
        } else {
            "Join"
        }
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
    // ANSI color codes
    const CYAN: &str = "\x1b[36m";
    const BRIGHT_CYAN: &str = "\x1b[96m";
    const YELLOW: &str = "\x1b[33m";
    const RESET: &str = "\x1b[0m";
    const BOLD: &str = "\x1b[1m";

    let version = env!("CARGO_PKG_VERSION");

    println!(
        "\n{}{}╔═══════════════════════════════════════════════════════════╗",
        BOLD, CYAN
    );
    println!("║                                                           ║");
    println!(
        "║     {}{}🚀  Hyra Scribe Ledger Node  🚀{}{}                ║",
        BOLD, BRIGHT_CYAN, RESET, CYAN
    );
    println!(
        "║        {}Distributed Key-Value Store with Raft{}          ║",
        BRIGHT_CYAN, CYAN
    );
    println!("║                                                           ║");
    println!(
        "║           {}Version: {:<10}{}                        ║",
        YELLOW, version, CYAN
    );
    println!(
        "╚═══════════════════════════════════════════════════════════╝{}\n",
        RESET
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
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        info!("Received Ctrl+C signal");
    }
}
