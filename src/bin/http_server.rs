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
use std::sync::{atomic::AtomicU64, Arc};
use tokio;
use tower_http::cors::CorsLayer;

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

// PUT endpoint handler - supports both JSON and binary data
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
        // Handle binary data directly
        state.ledger.put(&key, body.as_ref())
    } else {
        // Handle JSON data
        match serde_json::from_slice::<PutRequest>(&body) {
            Ok(payload) => state.ledger.put(&key, &payload.value),
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse {
                        error: format!("Invalid JSON payload: {}", e),
                    }),
                )
                    .into_response()
            }
        }
    };

    match result {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "message": "Value stored successfully"})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to store value: {}", e),
            }),
        )
            .into_response(),
    }
}

// GET endpoint handler - returns binary or JSON based on Accept header
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

    match state.ledger.get(&key) {
        Ok(Some(value_bytes)) => {
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
        Ok(None) => (StatusCode::NOT_FOUND, Json(GetResponse { value: None })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to retrieve value: {}", e),
            }),
        )
            .into_response(),
    }
}

// DELETE endpoint handler
async fn delete_handler(State(state): State<Arc<AppState>>, Path(key): Path<String>) -> Response {
    state
        .deletes
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    // Check if key exists first
    match state.ledger.get(&key) {
        Ok(Some(_)) => {
            // Key exists, perform deletion by setting to empty (sled doesn't have direct delete)
            // We'll use remove via batch operation
            let mut batch = SimpleScribeLedger::new_batch();
            batch.remove(key.as_bytes());
            match state.ledger.apply_batch(batch) {
                Ok(()) => (
                    StatusCode::OK,
                    Json(
                        serde_json::json!({"status": "ok", "message": "Key deleted successfully"}),
                    ),
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to delete key: {}", e),
                    }),
                )
                    .into_response(),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Key not found".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to check key: {}", e),
            }),
        )
            .into_response(),
    }
}

// Health check endpoint
async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "simple-scribe-ledger-server"
    }))
}

// Metrics endpoint
async fn metrics_handler(State(state): State<Arc<AppState>>) -> Response {
    let total_keys = state.ledger.len();
    let is_empty = state.ledger.is_empty();
    let total_gets = state.gets.load(std::sync::atomic::Ordering::Relaxed);
    let total_puts = state.puts.load(std::sync::atomic::Ordering::Relaxed);
    let total_deletes = state.deletes.load(std::sync::atomic::Ordering::Relaxed);

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

    // Build the router with all endpoints
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .route("/cluster/join", axum::routing::post(cluster_join_handler))
        .route("/cluster/leave", axum::routing::post(cluster_leave_handler))
        .route("/cluster/status", get(cluster_status_handler))
        .route("/cluster/members", get(cluster_members_handler))
        .route("/cluster/leader", get(cluster_leader_handler))
        .route("/:key", put(put_handler))
        .route("/:key", get(get_handler))
        .route("/:key", delete(delete_handler))
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

    // Run the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
