use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, put},
    Json, Router,
};
use hyra_scribe_ledger::{logging, metrics, HyraScribeLedger};
use serde::{Deserialize, Serialize};
use std::sync::{atomic::AtomicU64, Arc};
use std::time::Instant;
use tokio;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info, warn};

#[derive(Debug, Serialize, Deserialize)]
struct PutRequest {
    value: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GetResponse {
    value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetricsResponse {
    total_keys: usize,
    is_empty: bool,
    total_gets: u64,
    total_puts: u64,
    total_deletes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClusterJoinRequest {
    node_id: u64,
    address: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClusterLeaveRequest {
    node_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClusterStatusResponse {
    node_id: u64,
    is_leader: bool,
    current_leader: Option<u64>,
    state: String,
    last_log_index: Option<u64>,
    last_applied: Option<String>,
    current_term: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClusterMembersResponse {
    members: Vec<ClusterMemberInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClusterMemberInfo {
    node_id: u64,
    address: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClusterLeaderResponse {
    leader_id: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VerifyResponse {
    key: String,
    verified: bool,
    proof: Option<VerifyProof>,
    error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VerifyProof {
    root_hash: String,
    siblings: Vec<String>,
}

// Application state with metrics
struct AppState {
    ledger: Arc<HyraScribeLedger>,
    gets: Arc<AtomicU64>,
    puts: Arc<AtomicU64>,
    deletes: Arc<AtomicU64>,
}

impl AppState {
    fn new(ledger: HyraScribeLedger) -> Self {
        Self {
            ledger: Arc::new(ledger),
            gets: Arc::new(AtomicU64::new(0)),
            puts: Arc::new(AtomicU64::new(0)),
            deletes: Arc::new(AtomicU64::new(0)),
        }
    }
}

// PUT endpoint handler - supports both JSON and binary data
async fn put_handler(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let start = Instant::now();
    let correlation_id = logging::generate_correlation_id();

    debug!(correlation_id = %correlation_id, key = %key, "PUT request received");

    state
        .puts
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Track metrics
    metrics::PUT_REQUESTS.inc();
    metrics::OPS_TOTAL.inc();

    // Check content type to determine if we're handling binary or JSON
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let result = if content_type.contains("application/octet-stream") {
        // Handle binary data directly
        state.ledger.put(&key, body.as_ref())
    } else {
        // Handle JSON data - use simd-json for faster parsing if available
        match serde_json::from_slice::<PutRequest>(&body) {
            Ok(payload) => {
                // Use payload.value as bytes directly to avoid allocation
                state.ledger.put(&key, payload.value.as_bytes())
            }
            Err(e) => {
                warn!(correlation_id = %correlation_id, error = %e, "Invalid JSON payload");
                metrics::ERRORS_TOTAL.inc();
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Invalid JSON payload: {}", e),
                    }),
                )
                    .into_response();
            }
        }
    };

    let duration = start.elapsed();
    metrics::PUT_LATENCY.observe(duration.as_secs_f64());

    match result {
        Ok(()) => {
            info!(correlation_id = %correlation_id, key = %key, latency_ms = %duration.as_millis(), "PUT request successful");
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "ok", "message": "Value stored successfully"})),
            )
                .into_response()
        }
        Err(e) => {
            error!(correlation_id = %correlation_id, key = %key, error = %e, "PUT request failed");
            metrics::ERRORS_TOTAL.inc();
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to store value: {}", e),
                }),
            )
                .into_response()
        }
    }
}

// GET endpoint handler - returns binary or JSON based on Accept header
async fn get_handler(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    headers: HeaderMap,
) -> Response {
    let start = Instant::now();
    let correlation_id = logging::generate_correlation_id();

    debug!(correlation_id = %correlation_id, key = %key, "GET request received");

    state
        .gets
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Track metrics
    metrics::GET_REQUESTS.inc();
    metrics::OPS_TOTAL.inc();

    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let result = match state.ledger.get(&key) {
        Ok(Some(value_bytes)) => {
            let duration = start.elapsed();
            metrics::GET_LATENCY.observe(duration.as_secs_f64());
            info!(correlation_id = %correlation_id, key = %key, latency_ms = %duration.as_millis(), "GET request successful");

            if accept.contains("application/octet-stream") {
                // Return binary data directly
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/octet-stream")],
                    value_bytes,
                )
                    .into_response()
            } else {
                // Return JSON with string value
                match String::from_utf8(value_bytes) {
                    Ok(value_str) => (
                        StatusCode::OK,
                        Json(GetResponse {
                            value: Some(value_str),
                        }),
                    )
                        .into_response(),
                    Err(_) => {
                        warn!(correlation_id = %correlation_id, key = %key, "Value is binary data");
                        // If not valid UTF-8, return error
                        (
                            StatusCode::BAD_REQUEST,
                            Json(ErrorResponse {
                                error: "Value is binary data. Use Accept: application/octet-stream header".to_string(),
                            }),
                        )
                            .into_response()
                    }
                }
            }
        }
        Ok(None) => {
            let duration = start.elapsed();
            metrics::GET_LATENCY.observe(duration.as_secs_f64());
            debug!(correlation_id = %correlation_id, key = %key, "GET request - key not found");
            (StatusCode::NOT_FOUND, Json(GetResponse { value: None })).into_response()
        }
        Err(e) => {
            let duration = start.elapsed();
            metrics::GET_LATENCY.observe(duration.as_secs_f64());
            error!(correlation_id = %correlation_id, key = %key, error = %e, "GET request failed");
            metrics::ERRORS_TOTAL.inc();
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to retrieve value: {}", e),
                }),
            )
                .into_response()
        }
    };

    result
}

// DELETE endpoint handler
// Note: In a production distributed ledger, data should be immutable and permanent.
// This endpoint is provided for development/testing purposes. In a true distributed
// setup with consensus, deletions would be handled as append-only log entries that
// mark data as deleted without actually removing it.
async fn delete_handler(State(state): State<Arc<AppState>>, Path(key): Path<String>) -> Response {
    let start = Instant::now();
    let correlation_id = logging::generate_correlation_id();

    debug!(correlation_id = %correlation_id, key = %key, "DELETE request received");

    state
        .deletes
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Track metrics
    metrics::DELETE_REQUESTS.inc();
    metrics::OPS_TOTAL.inc();

    // Check if key exists first
    let result = match state.ledger.get(&key) {
        Ok(Some(_)) => {
            // Key exists, perform deletion by setting to empty (sled doesn't have direct delete)
            // We'll use remove via batch operation
            let mut batch = HyraScribeLedger::new_batch();
            batch.remove(key.as_bytes());
            match state.ledger.apply_batch(batch) {
                Ok(()) => {
                    let duration = start.elapsed();
                    metrics::DELETE_LATENCY.observe(duration.as_secs_f64());
                    info!(correlation_id = %correlation_id, key = %key, latency_ms = %duration.as_millis(), "DELETE request successful");
                    (
                        StatusCode::OK,
                        Json(
                            serde_json::json!({"status": "ok", "message": "Key deleted successfully"}),
                        ),
                    )
                        .into_response()
                }
                Err(e) => {
                    let duration = start.elapsed();
                    metrics::DELETE_LATENCY.observe(duration.as_secs_f64());
                    error!(correlation_id = %correlation_id, key = %key, error = %e, "DELETE request failed");
                    metrics::ERRORS_TOTAL.inc();
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: format!("Failed to delete key: {}", e),
                        }),
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => {
            let duration = start.elapsed();
            metrics::DELETE_LATENCY.observe(duration.as_secs_f64());
            debug!(correlation_id = %correlation_id, key = %key, "DELETE request - key not found");
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "Key not found".to_string(),
                }),
            )
                .into_response()
        }
        Err(e) => {
            let duration = start.elapsed();
            metrics::DELETE_LATENCY.observe(duration.as_secs_f64());
            error!(correlation_id = %correlation_id, key = %key, error = %e, "DELETE request failed");
            metrics::ERRORS_TOTAL.inc();
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to check key: {}", e),
                }),
            )
                .into_response()
        }
    };

    result
}

// Health check endpoint
async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "hyra-scribe-ledger-server"
    }))
}

// Legacy metrics endpoint (JSON format for backward compatibility)
async fn metrics_handler(State(state): State<Arc<AppState>>) -> Response {
    let total_keys = state.ledger.len();
    let is_empty = state.ledger.is_empty();
    let total_gets = state.gets.load(std::sync::atomic::Ordering::Relaxed);
    let total_puts = state.puts.load(std::sync::atomic::Ordering::Relaxed);
    let total_deletes = state.deletes.load(std::sync::atomic::Ordering::Relaxed);

    // Update storage metrics for Prometheus
    metrics::update_storage_metrics(total_keys, 0); // Size calculation would require more work

    (
        StatusCode::OK,
        Json(MetricsResponse {
            total_keys,
            is_empty,
            total_gets,
            total_puts,
            total_deletes,
        }),
    )
        .into_response()
}

// Prometheus metrics endpoint
async fn prometheus_metrics_handler() -> Response {
    let metrics_text = metrics::get_metrics();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        metrics_text,
    )
        .into_response()
}

// Cluster join endpoint - Not implemented in standalone mode
//
// This HTTP server is designed for standalone/single-node testing of the storage layer.
// For distributed cluster operations, use the scribe-node binary which integrates
// with ConsensusNode and provides full Raft-based cluster management.
//
// To add a node to a cluster in production:
// 1. Start the new node with scribe-node binary
// 2. From the leader node, call: consensus.add_learner(node_id, BasicNode { addr })
// 3. Wait for log replication to catch up
// 4. From the leader node, call: consensus.change_membership(members)
async fn cluster_join_handler(Json(payload): Json<ClusterJoinRequest>) -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "Cluster management not available in standalone HTTP server",
            "message": format!("Node {} joining at {}", payload.node_id, payload.address),
            "note": "Use scribe-node binary with ConsensusNode for distributed cluster operations"
        })),
    )
        .into_response()
}

// Cluster leave endpoint - Not implemented in standalone mode
//
// This HTTP server is designed for standalone/single-node testing of the storage layer.
// For distributed cluster operations, use the scribe-node binary.
//
// To remove a node from a cluster in production:
// 1. From the leader node, call: consensus.change_membership() without the departing node
// 2. Stop the departing node
async fn cluster_leave_handler(Json(payload): Json<ClusterLeaveRequest>) -> Response {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({
            "error": "Cluster management not available in standalone HTTP server",
            "message": format!("Node {} leaving cluster", payload.node_id),
            "note": "Use scribe-node binary with ConsensusNode for distributed cluster operations"
        })),
    )
        .into_response()
}

// Cluster status endpoint
async fn cluster_status_handler() -> Response {
    // In standalone mode, we're always the leader
    (
        StatusCode::OK,
        Json(ClusterStatusResponse {
            node_id: 1,
            is_leader: true,
            current_leader: Some(1),
            state: "Leader".to_string(),
            last_log_index: Some(0),
            last_applied: Some("0:0".to_string()),
            current_term: 1,
        }),
    )
        .into_response()
}

// Cluster members endpoint
async fn cluster_members_handler() -> Response {
    // In standalone mode, only one member
    let members = vec![ClusterMemberInfo {
        node_id: 1,
        address: "127.0.0.1:3000".to_string(),
    }];

    (StatusCode::OK, Json(ClusterMembersResponse { members })).into_response()
}

// Cluster leader endpoint
async fn cluster_leader_handler() -> Response {
    // In standalone mode, we're always the leader
    (
        StatusCode::OK,
        Json(ClusterLeaderResponse { leader_id: Some(1) }),
    )
        .into_response()
}

// Verification endpoint - generates and verifies Merkle proof for a key
async fn verify_handler(State(state): State<Arc<AppState>>, Path(key): Path<String>) -> Response {
    // Check if key exists
    match state.ledger.get(&key) {
        Ok(Some(_)) => {
            // Generate Merkle proof for the key
            match state.ledger.generate_merkle_proof(&key) {
                Ok(Some(proof)) => {
                    // Get the current Merkle root
                    match state.ledger.compute_merkle_root() {
                        Ok(Some(root_hash)) => {
                            // Verify the proof
                            let verified = hyra_scribe_ledger::crypto::MerkleTree::verify_proof(
                                &proof, &root_hash,
                            );

                            // Convert proof to hex strings for JSON response
                            let siblings_hex: Vec<String> =
                                proof.siblings.iter().map(|s| hex::encode(s)).collect();

                            (
                                StatusCode::OK,
                                Json(VerifyResponse {
                                    key: key.clone(),
                                    verified,
                                    proof: Some(VerifyProof {
                                        root_hash: hex::encode(root_hash),
                                        siblings: siblings_hex,
                                    }),
                                    error: None,
                                }),
                            )
                                .into_response()
                        }
                        Ok(None) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(VerifyResponse {
                                key,
                                verified: false,
                                proof: None,
                                error: Some("Failed to compute Merkle root".to_string()),
                            }),
                        )
                            .into_response(),
                        Err(e) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(VerifyResponse {
                                key,
                                verified: false,
                                proof: None,
                                error: Some(format!("Error computing Merkle root: {}", e)),
                            }),
                        )
                            .into_response(),
                    }
                }
                Ok(None) => (
                    StatusCode::NOT_FOUND,
                    Json(VerifyResponse {
                        key,
                        verified: false,
                        proof: None,
                        error: Some("Key not found in Merkle tree".to_string()),
                    }),
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(VerifyResponse {
                        key,
                        verified: false,
                        proof: None,
                        error: Some(format!("Failed to generate proof: {}", e)),
                    }),
                )
                    .into_response(),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(VerifyResponse {
                key,
                verified: false,
                proof: None,
                error: Some("Key not found".to_string()),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(VerifyResponse {
                key,
                verified: false,
                proof: None,
                error: Some(format!("Failed to retrieve key: {}", e)),
            }),
        )
            .into_response(),
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> anyhow::Result<()> {
    // Initialize logging with default configuration
    let log_config = logging::LogConfig::default();
    let _guard = logging::init_logging(log_config);

    info!("Starting Hyra Scribe Ledger HTTP Server...");

    // Initialize Prometheus metrics
    metrics::init_metrics();
    info!("Metrics system initialized");

    // Initialize the ledger with optimized configuration
    let ledger = HyraScribeLedger::temp()?;
    let app_state = Arc::new(AppState::new(ledger));

    info!("Ledger initialized");

    // Build the router with all endpoints - optimized order
    // Place most frequently accessed endpoints first for faster routing
    let app = Router::new()
        .route("/:key", get(get_handler))
        .route("/:key", put(put_handler))
        .route("/:key", delete(delete_handler))
        .route("/verify/:key", get(verify_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/metrics/prometheus", get(prometheus_metrics_handler))
        .route("/cluster/info", get(cluster_status_handler))
        .route("/cluster/nodes", get(cluster_members_handler))
        .route("/cluster/leader/info", get(cluster_leader_handler))
        .route(
            "/cluster/nodes/add",
            axum::routing::post(cluster_join_handler),
        )
        .route(
            "/cluster/nodes/remove",
            axum::routing::post(cluster_leave_handler),
        )
        .with_state(app_state)
        .layer(CorsLayer::permissive());

    info!("Server starting on http://0.0.0.0:3000");
    info!("Available endpoints:");
    info!("  GET    /health                  - Health check");
    info!("  GET    /metrics                 - Get server metrics (JSON)");
    info!("  GET    /metrics/prometheus      - Prometheus metrics endpoint");
    info!("  PUT    /:key                    - Store a value (JSON or binary)");
    info!("  GET    /:key                    - Retrieve a value (JSON or binary)");
    info!("  DELETE /:key                    - Delete a key");
    info!("  GET    /verify/:key             - Verify a key with Merkle proof");
    info!("");
    info!("Cluster management endpoints:");
    info!("  POST   /cluster/nodes/add       - Add a node to the cluster");
    info!("  POST   /cluster/nodes/remove    - Remove a node from the cluster");
    info!("  GET    /cluster/info            - Get cluster status information");
    info!("  GET    /cluster/nodes           - List all cluster nodes");
    info!("  GET    /cluster/leader/info     - Get current cluster leader information");
    info!("");
    info!("Example usage:");
    println!("  # JSON data:");
    println!("  curl -X PUT http://localhost:3000/test -H 'Content-Type: application/json' -d '{{\"value\": \"hello world\"}}'");
    println!("  curl http://localhost:3000/test");
    println!();
    println!("  # Binary data:");
    println!("  curl -X PUT http://localhost:3000/binary -H 'Content-Type: application/octet-stream' --data-binary @file.bin");
    println!("  curl -H 'Accept: application/octet-stream' http://localhost:3000/binary");
    println!();
    println!("  # Delete:");
    println!("  curl -X DELETE http://localhost:3000/test");
    println!();
    println!("  # Verify:");
    println!("  curl http://localhost:3000/verify/test");
    println!();
    println!("  # Metrics:");
    println!("  curl http://localhost:3000/metrics");
    println!();
    println!("  # Cluster information:");
    println!("  curl http://localhost:3000/cluster/info");

    // Run the server with optimized TCP configuration
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
