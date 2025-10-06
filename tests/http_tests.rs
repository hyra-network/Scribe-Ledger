use anyhow::Result;
use reqwest;
use serde_json::json;
use simple_scribe_ledger::SimpleScribeLedger;
use std::sync::Arc;
use std::time::Duration;
use tokio;

// Import the HTTP server types and handlers (we'll replicate them for testing)
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode as AxumStatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;
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

    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/json");

    let result = if content_type.contains("application/octet-stream") {
        state.ledger.put(&key, body.as_ref())
    } else {
        match serde_json::from_slice::<PutRequest>(&body) {
            Ok(payload) => state.ledger.put(&key, &payload.value),
            Err(e) => {
                return (
                    AxumStatusCode::BAD_REQUEST,
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
            AxumStatusCode::OK,
            Json(json!({"status": "ok", "message": "Value stored successfully"})),
        )
            .into_response(),
        Err(e) => (
            AxumStatusCode::INTERNAL_SERVER_ERROR,
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
                (
                    AxumStatusCode::OK,
                    [(header::CONTENT_TYPE, "application/octet-stream")],
                    value_bytes,
                )
                    .into_response()
            } else {
                match String::from_utf8(value_bytes) {
                    Ok(value_str) => (
                        AxumStatusCode::OK,
                        Json(GetResponse {
                            value: Some(value_str),
                        }),
                    )
                        .into_response(),
                    Err(_) => (
                        AxumStatusCode::BAD_REQUEST,
                        Json(ErrorResponse {
                            error:
                                "Value is binary data. Use Accept: application/octet-stream header"
                                    .to_string(),
                        }),
                    )
                        .into_response(),
                }
            }
        }
        Ok(None) => (AxumStatusCode::NOT_FOUND, Json(GetResponse { value: None })).into_response(),
        Err(e) => (
            AxumStatusCode::INTERNAL_SERVER_ERROR,
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

    match state.ledger.get(&key) {
        Ok(Some(_)) => {
            let mut batch = SimpleScribeLedger::new_batch();
            batch.remove(key.as_bytes());
            match state.ledger.apply_batch(batch) {
                Ok(()) => (
                    AxumStatusCode::OK,
                    Json(json!({"status": "ok", "message": "Key deleted successfully"})),
                )
                    .into_response(),
                Err(e) => (
                    AxumStatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to delete key: {}", e),
                    }),
                )
                    .into_response(),
            }
        }
        Ok(None) => (
            AxumStatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "Key not found".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            AxumStatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to check key: {}", e),
            }),
        )
            .into_response(),
    }
}

// Health check endpoint
async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({
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
        AxumStatusCode::OK,
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

// Cluster join endpoint - stub implementation for tests
async fn cluster_join_handler(Json(payload): Json<ClusterJoinRequest>) -> Response {
    (
        AxumStatusCode::OK,
        Json(json!({
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
        AxumStatusCode::OK,
        Json(json!({
            "status": "ok",
            "message": format!("Node {} leaving cluster", payload.node_id),
            "note": "Cluster management is not yet fully implemented in standalone mode"
        })),
    )
        .into_response()
}

// Cluster status endpoint
async fn cluster_status_handler() -> Response {
    (
        AxumStatusCode::OK,
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
    let members = vec![ClusterMemberInfo {
        node_id: 1,
        address: "127.0.0.1:3000".to_string(),
    }];

    (AxumStatusCode::OK, Json(ClusterMembersResponse { members })).into_response()
}

// Cluster leader endpoint
async fn cluster_leader_handler() -> Response {
    (
        AxumStatusCode::OK,
        Json(ClusterLeaderResponse { leader_id: Some(1) }),
    )
        .into_response()
}

// Helper function to create test server
async fn create_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let ledger = SimpleScribeLedger::temp().expect("Failed to create temp ledger");
    let app_state = Arc::new(AppState::new(ledger));

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

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let addr = listener.local_addr().expect("Failed to get local addr");
    let base_url = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("Server failed");
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    (base_url, handle)
}

#[tokio::test]
async fn test_health_endpoint() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();
    let response = client.get(format!("{}/health", base_url)).send().await?;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "simple-scribe-ledger-server");

    Ok(())
}

#[tokio::test]
async fn test_put_and_get_json() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();

    // PUT a value
    let put_response = client
        .put(format!("{}/test_key", base_url))
        .json(&json!({"value": "test_value"}))
        .send()
        .await?;

    assert_eq!(put_response.status().as_u16(), 200);

    // GET the value
    let get_response = client.get(format!("{}/test_key", base_url)).send().await?;

    assert_eq!(get_response.status().as_u16(), 200);

    let body: GetResponse = get_response.json().await?;
    assert_eq!(body.value, Some("test_value".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_get_nonexistent_key() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/nonexistent", base_url))
        .send()
        .await?;

    assert_eq!(response.status().as_u16(), 404);

    let body: GetResponse = response.json().await?;
    assert_eq!(body.value, None);

    Ok(())
}

#[tokio::test]
async fn test_delete_endpoint() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();

    // PUT a value
    client
        .put(format!("{}/delete_test", base_url))
        .json(&json!({"value": "to_be_deleted"}))
        .send()
        .await?;

    // Verify it exists
    let get_response = client
        .get(format!("{}/delete_test", base_url))
        .send()
        .await?;
    assert_eq!(get_response.status().as_u16(), 200);

    // DELETE it
    let delete_response = client
        .delete(format!("{}/delete_test", base_url))
        .send()
        .await?;
    assert_eq!(delete_response.status().as_u16(), 200);

    // Verify it's gone
    let get_response = client
        .get(format!("{}/delete_test", base_url))
        .send()
        .await?;
    assert_eq!(get_response.status().as_u16(), 404);

    Ok(())
}

#[tokio::test]
async fn test_delete_nonexistent_key() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();
    let response = client
        .delete(format!("{}/nonexistent", base_url))
        .send()
        .await?;

    assert_eq!(response.status().as_u16(), 404);

    Ok(())
}

#[tokio::test]
async fn test_binary_data_support() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();
    let binary_data = vec![0u8, 1, 2, 3, 255, 254, 253];

    // PUT binary data
    let put_response = client
        .put(format!("{}/binary_key", base_url))
        .header("Content-Type", "application/octet-stream")
        .body(binary_data.clone())
        .send()
        .await?;

    assert_eq!(put_response.status().as_u16(), 200);

    // GET binary data
    let get_response = client
        .get(format!("{}/binary_key", base_url))
        .header("Accept", "application/octet-stream")
        .send()
        .await?;

    assert_eq!(get_response.status().as_u16(), 200);
    assert_eq!(
        get_response
            .headers()
            .get("content-type")
            .unwrap()
            .to_str()?,
        "application/octet-stream"
    );

    let body_bytes = get_response.bytes().await?;
    assert_eq!(body_bytes.to_vec(), binary_data);

    Ok(())
}

#[tokio::test]
async fn test_metrics_endpoint() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();

    // Check initial metrics
    let response = client.get(format!("{}/metrics", base_url)).send().await?;
    assert_eq!(response.status().as_u16(), 200);

    let metrics: MetricsResponse = response.json().await?;
    assert_eq!(metrics.is_empty, true);
    assert_eq!(metrics.total_keys, 0);

    // Perform some operations
    client
        .put(format!("{}/key1", base_url))
        .json(&json!({"value": "value1"}))
        .send()
        .await?;

    client
        .put(format!("{}/key2", base_url))
        .json(&json!({"value": "value2"}))
        .send()
        .await?;

    client.get(format!("{}/key1", base_url)).send().await?;

    // Check metrics again
    let response = client.get(format!("{}/metrics", base_url)).send().await?;
    let metrics: MetricsResponse = response.json().await?;

    assert_eq!(metrics.is_empty, false);
    assert_eq!(metrics.total_keys, 2);
    assert_eq!(metrics.total_puts, 2);
    assert_eq!(metrics.total_gets, 1);

    Ok(())
}

#[tokio::test]
async fn test_concurrent_requests() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = Arc::new(reqwest::Client::new());
    let mut handles = vec![];

    // Spawn 10 concurrent PUT operations
    for i in 0..10 {
        let client = Arc::clone(&client);
        let base_url = base_url.clone();
        let handle = tokio::spawn(async move {
            client
                .put(format!("{}/concurrent_{}", base_url, i))
                .json(&json!({"value": format!("value_{}", i)}))
                .send()
                .await
                .unwrap()
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let response = handle.await?;
        assert_eq!(response.status().as_u16(), 200);
    }

    // Verify all keys exist
    for i in 0..10 {
        let response = client
            .get(format!("{}/concurrent_{}", base_url, i))
            .send()
            .await?;
        assert_eq!(response.status().as_u16(), 200);

        let body: GetResponse = response.json().await?;
        assert_eq!(body.value, Some(format!("value_{}", i)));
    }

    Ok(())
}

#[tokio::test]
async fn test_large_payload() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();

    // Create a large payload (1MB)
    let large_value = "x".repeat(1024 * 1024);

    // PUT large payload
    let put_response = client
        .put(format!("{}/large_key", base_url))
        .json(&json!({"value": large_value}))
        .send()
        .await?;

    assert_eq!(put_response.status().as_u16(), 200);

    // GET large payload
    let get_response = client.get(format!("{}/large_key", base_url)).send().await?;

    assert_eq!(get_response.status().as_u16(), 200);

    let body: GetResponse = get_response.json().await?;
    assert_eq!(body.value, Some(large_value));

    Ok(())
}

#[tokio::test]
async fn test_invalid_json() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();

    // Send invalid JSON
    let response = client
        .put(format!("{}/test", base_url))
        .header("Content-Type", "application/json")
        .body("{invalid json}")
        .send()
        .await?;

    assert_eq!(response.status().as_u16(), 400);

    let body: ErrorResponse = response.json().await?;
    assert!(body.error.contains("Invalid JSON payload"));

    Ok(())
}

#[tokio::test]
async fn test_error_responses() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();

    // Test GET on non-existent key
    let response = client
        .get(format!("{}/nonexistent", base_url))
        .send()
        .await?;
    assert_eq!(response.status().as_u16(), 404);

    // Test DELETE on non-existent key
    let response = client
        .delete(format!("{}/nonexistent", base_url))
        .send()
        .await?;
    assert_eq!(response.status().as_u16(), 404);

    Ok(())
}

#[tokio::test]
async fn test_multiple_put_overwrites() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();

    // PUT initial value
    client
        .put(format!("{}/overwrite_test", base_url))
        .json(&json!({"value": "value1"}))
        .send()
        .await?;

    // PUT second value (overwrite)
    client
        .put(format!("{}/overwrite_test", base_url))
        .json(&json!({"value": "value2"}))
        .send()
        .await?;

    // PUT third value (overwrite)
    client
        .put(format!("{}/overwrite_test", base_url))
        .json(&json!({"value": "value3"}))
        .send()
        .await?;

    // GET and verify only the last value exists
    let response = client
        .get(format!("{}/overwrite_test", base_url))
        .send()
        .await?;

    let body: GetResponse = response.json().await?;
    assert_eq!(body.value, Some("value3".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_special_characters_in_keys() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();

    // Test with URL-encoded special characters
    let special_key = "test-key_with.special:chars";
    let encoded_key = urlencoding::encode(special_key);

    client
        .put(format!("{}/{}", base_url, encoded_key))
        .json(&json!({"value": "special_value"}))
        .send()
        .await?;

    let response = client
        .get(format!("{}/{}", base_url, encoded_key))
        .send()
        .await?;

    assert_eq!(response.status().as_u16(), 200);

    let body: GetResponse = response.json().await?;
    assert_eq!(body.value, Some("special_value".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_cluster_status_endpoint() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/cluster/status", base_url))
        .send()
        .await?;

    assert_eq!(response.status().as_u16(), 200);

    let body: ClusterStatusResponse = response.json().await?;
    assert_eq!(body.node_id, 1);
    assert_eq!(body.is_leader, true);
    assert_eq!(body.current_leader, Some(1));
    assert_eq!(body.state, "Leader");

    Ok(())
}

#[tokio::test]
async fn test_cluster_members_endpoint() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/cluster/members", base_url))
        .send()
        .await?;

    assert_eq!(response.status().as_u16(), 200);

    let body: ClusterMembersResponse = response.json().await?;
    assert_eq!(body.members.len(), 1);
    assert_eq!(body.members[0].node_id, 1);

    Ok(())
}

#[tokio::test]
async fn test_cluster_leader_endpoint() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/cluster/leader", base_url))
        .send()
        .await?;

    assert_eq!(response.status().as_u16(), 200);

    let body: ClusterLeaderResponse = response.json().await?;
    assert_eq!(body.leader_id, Some(1));

    Ok(())
}

#[tokio::test]
async fn test_cluster_join_endpoint() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();
    let request = ClusterJoinRequest {
        node_id: 2,
        address: "127.0.0.1:3001".to_string(),
    };

    let response = client
        .post(format!("{}/cluster/join", base_url))
        .json(&request)
        .send()
        .await?;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["status"], "ok");
    assert!(body["message"].as_str().unwrap().contains("Node 2"));

    Ok(())
}

#[tokio::test]
async fn test_cluster_leave_endpoint() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();
    let request = ClusterLeaveRequest { node_id: 2 };

    let response = client
        .post(format!("{}/cluster/leave", base_url))
        .json(&request)
        .send()
        .await?;

    assert_eq!(response.status().as_u16(), 200);

    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["status"], "ok");
    assert!(body["message"].as_str().unwrap().contains("Node 2"));

    Ok(())
}

#[tokio::test]
async fn test_cluster_endpoints_integration() -> Result<()> {
    let (base_url, _handle) = create_test_server().await;

    let client = reqwest::Client::new();

    // Check initial status
    let status_response = client
        .get(format!("{}/cluster/status", base_url))
        .send()
        .await?;
    assert_eq!(status_response.status().as_u16(), 200);

    // Check members
    let members_response = client
        .get(format!("{}/cluster/members", base_url))
        .send()
        .await?;
    assert_eq!(members_response.status().as_u16(), 200);

    // Check leader
    let leader_response = client
        .get(format!("{}/cluster/leader", base_url))
        .send()
        .await?;
    assert_eq!(leader_response.status().as_u16(), 200);

    Ok(())
}

#[tokio::test]
async fn test_batched_http_operations() -> Result<()> {
    use simple_scribe_ledger::http_client::{batched_put_operations, batched_get_operations, PutRequest};
    
    let (base_url, _handle) = create_test_server().await;
    let client = reqwest::Client::new();

    // Test batched PUT operations
    let keys: Vec<String> = (0..50).map(|i| format!("batch_key_{}", i)).collect();
    let payloads: Vec<PutRequest> = (0..50)
        .map(|i| PutRequest {
            value: format!("batch_value_{}", i),
        })
        .collect();

    let ops_count = batched_put_operations(&client, &base_url, &keys, &payloads).await;
    assert_eq!(ops_count, 50);

    // Test batched GET operations
    let urls: Vec<String> = (0..50)
        .map(|i| format!("{}/batch_key_{}", base_url, i))
        .collect();

    let get_count = batched_get_operations(&client, &urls).await;
    assert_eq!(get_count, 50);

    Ok(())
}
