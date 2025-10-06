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
    let mut i = 0;
    
    while i < ops {
        let batch_size = std::cmp::min(MAX_CONCURRENCY, ops - i);
        let mut handles = Vec::with_capacity(batch_size);

        for j in i..(i + batch_size) {
            let client = client.clone();
            let url = format!("{}/{}", base_url, keys[j]);
            let payload = payloads[j].clone();

            let handle = tokio::spawn(async move {
                let _response = client.put(&url).json(&payload).send().await.unwrap();
            });

            handles.push(handle);
        }

        // Wait for current batch to complete before starting next batch
        for handle in handles {
            handle.await.unwrap();
        }

        i += batch_size;
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
pub async fn batched_get_operations(
    client: &Client,
    urls: &[String],
) -> usize {
    let ops = urls.len();
    let mut i = 0;
    
    while i < ops {
        let batch_size = std::cmp::min(MAX_CONCURRENCY, ops - i);
        let mut handles = Vec::with_capacity(batch_size);

        for j in i..(i + batch_size) {
            let client = client.clone();
            let url = urls[j].clone();

            let handle = tokio::spawn(async move {
                let response = client.get(&url).send().await.unwrap();
                let _data: GetResponse = response.json().await.unwrap();
            });

            handles.push(handle);
        }

        // Wait for current batch to complete before starting next batch
        for handle in handles {
            handle.await.unwrap();
        }

        i += batch_size;
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
    let mut i = 0;
    
    while i < ops {
        let batch_size = std::cmp::min(MAX_CONCURRENCY, ops - i);
        let mut handles = Vec::with_capacity(batch_size);

        for j in i..(i + batch_size) {
            let client = client.clone();
            let url = format!("{}/{}", base_url, keys[j]);

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

        i += batch_size;
    }
    
    ops
}
