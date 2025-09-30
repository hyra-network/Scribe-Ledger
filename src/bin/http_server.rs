use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use simple_scribe_ledger::SimpleScribeLedger;
use std::sync::Arc;
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

// Application state
type AppState = Arc<SimpleScribeLedger>;

// PUT endpoint handler
async fn put_handler(
    State(ledger): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<PutRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<ErrorResponse>)> {
    match ledger.put(&key, &payload.value) {
        Ok(()) => Ok((
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "message": "Value stored successfully"})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to store value: {}", e),
            }),
        )),
    }
}

// GET endpoint handler
async fn get_handler(
    State(ledger): State<AppState>,
    Path(key): Path<String>,
) -> Result<(StatusCode, Json<GetResponse>), (StatusCode, Json<ErrorResponse>)> {
    match ledger.get(&key) {
        Ok(Some(value_bytes)) => match String::from_utf8(value_bytes) {
            Ok(value_str) => Ok((
                StatusCode::OK,
                Json(GetResponse {
                    value: Some(value_str),
                }),
            )),
            Err(e) => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to decode value as UTF-8: {}", e),
                }),
            )),
        },
        Ok(None) => Ok((StatusCode::OK, Json(GetResponse { value: None }))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to retrieve value: {}", e),
            }),
        )),
    }
}

// Health check endpoint
async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "simple-scribe-ledger-server"
    }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Starting Simple Scribe Ledger HTTP Server...");

    // Initialize the ledger with optimized configuration
    let ledger = SimpleScribeLedger::temp()?;
    let app_state = Arc::new(ledger);

    // Build the router
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/kv/:key", put(put_handler))
        .route("/kv/:key", get(get_handler))
        .with_state(app_state)
        .layer(CorsLayer::permissive());

    println!("Server starting on http://0.0.0.0:3000");
    println!("Available endpoints:");
    println!("  GET  /health       - Health check");
    println!("  PUT  /kv/:key      - Store a value (JSON: {{\"value\": \"...\"}})");
    println!("  GET  /kv/:key      - Retrieve a value");
    println!();
    println!("Example usage:");
    println!("  curl -X PUT http://localhost:3000/kv/test -H 'Content-Type: application/json' -d '{{\"value\": \"hello world\"}}'");
    println!("  curl http://localhost:3000/kv/test");

    // Run the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
