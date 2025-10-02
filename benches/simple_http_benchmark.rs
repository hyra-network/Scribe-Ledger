use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde::{Deserialize, Serialize};
use simple_scribe_ledger::SimpleScribeLedger;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, put},
    Router,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PutRequest {
    value: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GetResponse {
    value: Option<String>,
}

type AppState = Arc<SimpleScribeLedger>;

// PUT endpoint handler (same as http_server.rs)
async fn put_handler(
    State(ledger): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<PutRequest>,
) -> Result<StatusCode, StatusCode> {
    ledger
        .put(&key, &payload.value)
        .map(|_| StatusCode::OK)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// GET endpoint handler (same as http_server.rs)
async fn get_handler(
    State(ledger): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<GetResponse>, StatusCode> {
    match ledger.get(&key) {
        Ok(Some(value_bytes)) => match String::from_utf8(value_bytes) {
            Ok(value_str) => Ok(Json(GetResponse {
                value: Some(value_str),
            })),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        },
        Ok(None) => Ok(Json(GetResponse { value: None })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Helper function to create and start an HTTP server
async fn start_test_server(port: u16) -> Arc<SimpleScribeLedger> {
    let ledger = SimpleScribeLedger::temp().unwrap();
    let app_state = Arc::new(ledger);

    let app = Router::new()
        .route("/kv/:key", put(put_handler))
        .route("/kv/:key", get(get_handler))
        .with_state(app_state.clone());

    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    app_state
}

fn benchmark_http_put_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_put_operations");
    group.measurement_time(Duration::from_secs(10));

    for ops in [10, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(ops), ops, |b, &ops| {
            let rt = Runtime::new().unwrap();

            // Start server once for this benchmark
            let port = 13000 + ops as u16; // Different port for each test
            let _ledger = rt.block_on(async { start_test_server(port).await });

            let client = reqwest::Client::new();
            let base_url = format!("http://127.0.0.1:{}/kv", port);

            b.iter(|| {
                rt.block_on(async {
                    for i in 0..ops {
                        let key = format!("key{}", i);
                        let url = format!("{}/{}", base_url, key);
                        let payload = PutRequest {
                            value: format!("value{}", i),
                        };

                        let response = client.put(&url).json(&payload).send().await.unwrap();

                        black_box(response.status());
                    }
                });
            });
        });
    }

    group.finish();
}

fn benchmark_http_get_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_get_operations");
    group.measurement_time(Duration::from_secs(10));

    for ops in [10, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(ops), ops, |b, &ops| {
            let rt = Runtime::new().unwrap();

            // Start server and pre-populate data
            let port = 14000 + ops as u16; // Different port for each test
            let _ledger = rt.block_on(async {
                let ledger = start_test_server(port).await;

                // Pre-populate data directly
                for i in 0..ops {
                    let key = format!("key{}", i);
                    let value = format!("value{}", i);
                    ledger.put(&key, &value).unwrap();
                }

                ledger
            });

            let client = reqwest::Client::new();
            let base_url = format!("http://127.0.0.1:{}/kv", port);

            b.iter(|| {
                rt.block_on(async {
                    for i in 0..ops {
                        let key = format!("key{}", i);
                        let url = format!("{}/{}", base_url, key);

                        let response = client.get(&url).send().await.unwrap();

                        let _data: GetResponse = response.json().await.unwrap();
                        black_box(_data);
                    }
                });
            });
        });
    }

    group.finish();
}

fn benchmark_http_mixed_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_mixed_operations");
    group.measurement_time(Duration::from_secs(10));

    for ops in [10, 100, 500, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(ops), ops, |b, &ops| {
            let rt = Runtime::new().unwrap();

            // Start server
            let port = 15000 + ops as u16; // Different port for each test
            let _ledger = rt.block_on(async { start_test_server(port).await });

            let client = reqwest::Client::new();
            let base_url = format!("http://127.0.0.1:{}/kv", port);

            b.iter(|| {
                rt.block_on(async {
                    for i in 0..ops {
                        let key = format!("key{}", i % 100);
                        let url = format!("{}/{}", base_url, key);

                        if i % 2 == 0 {
                            // PUT operation
                            let payload = PutRequest {
                                value: format!("value{}", i),
                            };
                            let response = client.put(&url).json(&payload).send().await.unwrap();
                            black_box(response.status());
                        } else {
                            // GET operation
                            let response = client.get(&url).send().await.unwrap();
                            black_box(response.status());
                        }
                    }
                });
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    benchmark_http_put_operations,
    benchmark_http_get_operations,
    benchmark_http_mixed_operations
);
criterion_main!(benches);
