use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use simple_scribe_ledger::SimpleScribeLedger;
use std::sync::{atomic::AtomicU64, Arc, OnceLock};
use tokio;
use tower_http::cors::CorsLayer;

// Pre-allocated static responses for common cases
static OK_RESPONSE: OnceLock<String> = OnceLock::new();
static HEALTH_RESPONSE: OnceLock<String> = OnceLock::new();

fn get_ok_response() -> &'static str {
    OK_RESPONSE
        .get_or_init(|| r#"{"status":"ok","message":"Value stored successfully"}"#.to_string())
}

fn get_health_response() -> &'static str {
    HEALTH_RESPONSE.get_or_init(|| {
        r#"{"status":"healthy","service":"simple-scribe-ledger-server"}"#.to_string()
    })
}

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

// Application state with metrics
struct AppState {
    ledger: Arc<SimpleScribeLedger>,
    gets: Arc<AtomicU64>,
    puts: Arc<AtomicU64>,
    deletes: Arc<AtomicU64>,
}

impl AppState {
    fn new(ledger: SimpleScribeLedger) -> Self {
        Self {
            ledger: Arc::new(ledger),
            gets: Arc::new(AtomicU64::new(0)),
            puts: Arc::new(AtomicU64::new(0)),
            deletes: Arc::new(AtomicU64::new(0)),
        }
    }
}

// PUT endpoint handler - optimized for performance
async fn put_handler(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    state
        .puts
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Check content type to determine if we're handling binary or JSON
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let result = if content_type.contains("application/octet-stream") {
        // Handle binary data directly - zero-copy
        state.ledger.put(key.as_bytes(), body.as_ref())
    } else {
        // Handle JSON data - optimized parsing
        match serde_json::from_slice::<PutRequest>(&body) {
            Ok(payload) => state.ledger.put(key.as_bytes(), payload.value.as_bytes()),
            Err(e) => {
                // Fast path for error response - pre-formatted string
                let error_json = format!(r#"{{"error":"Invalid JSON payload: {}"}}"#, e);
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "application/json")],
                    error_json,
                )
                    .into_response();
            }
        }
    };

    match result {
        Ok(()) => {
            // Use pre-allocated response string
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "application/json")],
                get_ok_response(),
            )
                .into_response()
        }
        Err(e) => {
            let error_json = format!(r#"{{"error":"Failed to store value: {}"}}"#, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                error_json,
            )
                .into_response()
        }
    }
}

// GET endpoint handler - optimized for performance
async fn get_handler(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    headers: HeaderMap,
) -> Response {
    state
        .gets
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    match state.ledger.get(key.as_bytes()) {
        Ok(Some(value_bytes)) => {
            if accept.contains("application/octet-stream") {
                // Return binary data directly - zero-copy
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/octet-stream")],
                    value_bytes,
                )
                    .into_response()
            } else {
                // Return JSON with string value - optimized path
                match String::from_utf8(value_bytes) {
                    Ok(value_str) => {
                        // Fast path: directly construct JSON string
                        let json_response =
                            format!(r#"{{"value":"{}"}}"#, value_str.replace('"', "\\\""));
                        (
                            StatusCode::OK,
                            [(header::CONTENT_TYPE, "application/json")],
                            json_response,
                        )
                            .into_response()
                    }
                    Err(_) => {
                        // If not valid UTF-8, return error
                        (
                            StatusCode::BAD_REQUEST,
                            [(header::CONTENT_TYPE, "application/json")],
                            r#"{"error":"Value is binary data. Use Accept: application/octet-stream header"}"#,
                        )
                            .into_response()
                    }
                }
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"value":null}"#,
        )
            .into_response(),
        Err(e) => {
            let error_json = format!(r#"{{"error":"Failed to retrieve value: {}"}}"#, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                error_json,
            )
                .into_response()
        }
    }
}

// DELETE endpoint handler - optimized
async fn delete_handler(State(state): State<Arc<AppState>>, Path(key): Path<String>) -> Response {
    state
        .deletes
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Check if key exists first
    match state.ledger.get(key.as_bytes()) {
        Ok(Some(_)) => {
            // Key exists, perform deletion
            let mut batch = SimpleScribeLedger::new_batch();
            batch.remove(key.as_bytes());
            match state.ledger.apply_batch(batch) {
                Ok(()) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/json")],
                    r#"{"status":"ok","message":"Key deleted successfully"}"#,
                )
                    .into_response(),
                Err(e) => {
                    let error_json = format!(r#"{{"error":"Failed to delete key: {}"}}"#, e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        [(header::CONTENT_TYPE, "application/json")],
                        error_json,
                    )
                        .into_response()
                }
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"error":"Key not found"}"#,
        )
            .into_response(),
        Err(e) => {
            let error_json = format!(r#"{{"error":"Failed to check key: {}"}}"#, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::CONTENT_TYPE, "application/json")],
                error_json,
            )
                .into_response()
        }
    }
}

// Health check endpoint - optimized with pre-allocated response
async fn health_handler() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        get_health_response(),
    )
        .into_response()
}

// Metrics endpoint - optimized JSON building
async fn metrics_handler(State(state): State<Arc<AppState>>) -> Response {
    let total_keys = state.ledger.len();
    let is_empty = state.ledger.is_empty();
    let total_gets = state.gets.load(std::sync::atomic::Ordering::Relaxed);
    let total_puts = state.puts.load(std::sync::atomic::Ordering::Relaxed);
    let total_deletes = state.deletes.load(std::sync::atomic::Ordering::Relaxed);

    // Build JSON string directly for better performance
    let json_response = format!(
        r#"{{"total_keys":{},"is_empty":{},"total_gets":{},"total_puts":{},"total_deletes":{}}}"#,
        total_keys, is_empty, total_gets, total_puts, total_deletes
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        json_response,
    )
        .into_response()
}

// Cluster join endpoint - stub implementation for now
async fn cluster_join_handler(Json(payload): Json<ClusterJoinRequest>) -> Response {
    // For now, this is a stub implementation
    // In a full distributed setup, this would:
    // 1. Add the node as a learner
    // 2. Wait for log replication to catch up
    // 3. Promote to voting member
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "message": format!("Node {} joining at {}", payload.node_id, payload.address),
            "note": "Cluster management is not yet fully implemented in standalone mode"
        })),
    )
        .into_response()
}

// Cluster leave endpoint - stub implementation
async fn cluster_leave_handler(Json(payload): Json<ClusterLeaveRequest>) -> Response {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "message": format!("Node {} leaving cluster", payload.node_id),
            "note": "Cluster management is not yet fully implemented in standalone mode"
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting Simple Scribe Ledger HTTP Server...");

    // Initialize the ledger with optimized configuration
    let ledger = SimpleScribeLedger::temp()?;
    let app_state = Arc::new(AppState::new(ledger));

    // Build the router with all endpoints - optimized routing order
    // Put most frequently used endpoints first for faster matching
    let app = Router::new()
        // Most frequently used endpoints first
        .route("/:key", get(get_handler))
        .route("/:key", put(put_handler))
        .route("/:key", delete(delete_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        // Cluster endpoints (less frequent)
        .route("/cluster/status", get(cluster_status_handler))
        .route("/cluster/members", get(cluster_members_handler))
        .route("/cluster/leader", get(cluster_leader_handler))
        .route("/cluster/join", axum::routing::post(cluster_join_handler))
        .route("/cluster/leave", axum::routing::post(cluster_leave_handler))
        .with_state(app_state)
        .layer(CorsLayer::permissive());

    println!("Server starting on http://0.0.0.0:3000");
    println!("Available endpoints:");
    println!("  GET    /health             - Health check");
    println!("  GET    /metrics            - Get server metrics");
    println!("  PUT    /:key               - Store a value (JSON or binary)");
    println!("  GET    /:key               - Retrieve a value (JSON or binary)");
    println!("  DELETE /:key               - Delete a key");
    println!();
    println!("Cluster management endpoints:");
    println!("  POST   /cluster/join       - Join a node to the cluster");
    println!("  POST   /cluster/leave      - Remove a node from the cluster");
    println!("  GET    /cluster/status     - Get cluster status");
    println!("  GET    /cluster/members    - List cluster members");
    println!("  GET    /cluster/leader     - Get current leader");
    println!();
    println!("Example usage:");
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
    println!("  # Metrics:");
    println!("  curl http://localhost:3000/metrics");
    println!();
    println!("  # Cluster status:");
    println!("  curl http://localhost:3000/cluster/status");

    // Run the server with optimized settings
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// Graceful shutdown signal handler
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
    println!("\nShutting down gracefully...");
}
