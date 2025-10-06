use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Maximum concurrent requests to prevent resource exhaustion and ensure linear scaling
const MAX_CONCURRENCY: usize = 20;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PutRequest {
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetResponse {
    pub value: Option<String>,
}

/// Perform batched HTTP PUT operations with controlled concurrency
///
/// # Arguments
/// * `client` - The HTTP client to use
/// * `base_url` - Base URL for the requests (e.g., "http://127.0.0.1:3000/kv")
/// * `keys` - Vector of keys to PUT
/// * `payloads` - Vector of payloads corresponding to keys
///
/// # Returns
/// Number of operations performed
pub async fn batched_put_operations(
    client: &Client,
    base_url: &str,
    keys: &[String],
    payloads: &[PutRequest],
) -> usize {
    let ops = keys.len();

    for chunk_start in (0..ops).step_by(MAX_CONCURRENCY) {
        let chunk_end = std::cmp::min(chunk_start + MAX_CONCURRENCY, ops);
        let mut handles = Vec::with_capacity(chunk_end - chunk_start);

        for j in chunk_start..chunk_end {
            let client = client.clone();
            let url = format!("{}/{}", base_url, &keys[j]);
            let payload = payloads[j].clone();

            handles.push(tokio::spawn(async move {
                let _response = client.put(&url).json(&payload).send().await.unwrap();
            }));
        }

        // Wait for current batch to complete before starting next batch
        for handle in handles {
            handle.await.unwrap();
        }
    }

    ops
}

/// Perform batched HTTP GET operations with controlled concurrency
///
/// # Arguments
/// * `client` - The HTTP client to use
/// * `urls` - Vector of URLs to GET
///
/// # Returns
/// Number of operations performed
pub async fn batched_get_operations(client: &Client, urls: &[String]) -> usize {
    let ops = urls.len();

    for chunk_start in (0..ops).step_by(MAX_CONCURRENCY) {
        let chunk_end = std::cmp::min(chunk_start + MAX_CONCURRENCY, ops);
        let mut handles = Vec::with_capacity(chunk_end - chunk_start);

        for url in urls.iter().skip(chunk_start).take(chunk_end - chunk_start) {
            let client = client.clone();
            let url = url.clone();

            handles.push(tokio::spawn(async move {
                let response = client.get(&url).send().await.unwrap();
                let _data: GetResponse = response.json().await.unwrap();
            }));
        }

        // Wait for current batch to complete before starting next batch
        for handle in handles {
            handle.await.unwrap();
        }
    }

    ops
}

/// Perform batched HTTP mixed operations (alternating PUT/GET) with controlled concurrency
///
/// # Arguments
/// * `client` - The HTTP client to use
/// * `base_url` - Base URL for the requests
/// * `keys` - Vector of keys for operations
/// * `payloads` - Vector of payloads for PUT operations
///
/// # Returns
/// Number of operations performed
pub async fn batched_mixed_operations(
    client: &Client,
    base_url: &str,
    keys: &[String],
    payloads: &[PutRequest],
) -> usize {
    let ops = keys.len();

    for chunk_start in (0..ops).step_by(MAX_CONCURRENCY) {
        let chunk_end = std::cmp::min(chunk_start + MAX_CONCURRENCY, ops);
        let mut handles = Vec::with_capacity(chunk_end - chunk_start);

        for j in chunk_start..chunk_end {
            let client = client.clone();
            let url = format!("{}/{}", base_url, &keys[j]);

            let handle = if j % 2 == 0 {
                let payload = payloads[j].clone();
                tokio::spawn(async move {
                    let _response = client.put(&url).json(&payload).send().await.unwrap();
                })
            } else {
                tokio::spawn(async move {
                    let _response = client.get(&url).send().await.unwrap();
                })
            };

            handles.push(handle);
        }

        // Wait for current batch to complete before starting next batch
        for handle in handles {
            handle.await.unwrap();
        }
    }

    ops
}
