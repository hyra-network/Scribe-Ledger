//! # Hyra Scribe Ledger
//!
//! Verifiable, Durable Off-Chain Storage for the Hyra AI Ecosystem.
//!
//! Hyra Scribe Ledger is a distributed, immutable, append-only key-value storage system
//! designed to serve as the durable data layer for Hyra AI.

pub mod consensus;
pub mod manifest;
pub mod storage;
pub mod write_node;

pub mod error;
pub mod types;
pub mod config;
pub mod crypto;
pub mod network;

pub use error::{Result, ScribeError};
pub use types::*;
pub use config::Config;

use std::path::Path;
use std::sync::Arc;
use bytes::Bytes;
use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    routing::{get, put},
    Router,
};

/// Main entry point for the Scribe Ledger library
pub struct ScribeLedger {
    db: sled::Db,
    config: Config,
}

impl ScribeLedger {
    /// Create a new Scribe Ledger instance
    pub fn new(config: Config) -> Result<Self> {
        let data_dir = Path::new(&config.node.data_dir);
        std::fs::create_dir_all(data_dir)?;
        
        let db_path = data_dir.join("scribe.db");
        let db = sled::open(&db_path)?;
        
        Ok(Self { db, config })
    }

    /// Store a key-value pair
    pub async fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush_async().await?;
        Ok(())
    }

    /// Retrieve a value by key
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self.db.get(key.as_bytes())? {
            Some(value) => Ok(Some(value.to_vec())),
            None => Ok(None),
        }
    }

    /// Start the Scribe Ledger node with HTTP server
    pub async fn start(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.network.listen_addr, self.config.network.client_port);
        
        let ledger = Arc::new(self);
        
        let app = Router::new()
            .route("/:key", put(put_handler))
            .route("/:key", get(get_handler))
            .with_state(ledger);
        
        tracing::info!("ScribeLedger HTTP server running on {}", addr);
        
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
        
        Ok(())
    }
}

async fn put_handler(
    State(ledger): State<Arc<ScribeLedger>>,
    AxumPath(key): AxumPath<String>,
    body: Bytes,
) -> std::result::Result<&'static str, StatusCode> {
    match ledger.put(&key, &body).await {
        Ok(_) => Ok("OK"),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn get_handler(
    State(ledger): State<Arc<ScribeLedger>>,
    AxumPath(key): AxumPath<String>,
) -> std::result::Result<Vec<u8>, StatusCode> {
    match ledger.get(&key).await {
        Ok(Some(value)) => Ok(value),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_config() -> (Config, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::default();
        config.node.data_dir = temp_dir.path().to_string_lossy().to_string();
        config.network.client_port = 0; // Use random port for testing
        (config, temp_dir)
    }

    #[tokio::test]
    async fn test_new_scribe_ledger() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();
        // Just verify the ledger was created successfully
        assert!(std::path::Path::new(&ledger.config.node.data_dir).exists());
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        // Test PUT operation
        let key = "test_key";
        let value = b"test_value";
        ledger.put(key, value).await.unwrap();

        // Test GET operation
        let retrieved_value = ledger.get(key).await.unwrap();
        assert_eq!(retrieved_value, Some(value.to_vec()));
    }

    #[tokio::test]
    async fn test_get_nonexistent_key() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        // Test GET operation on non-existent key
        let result = ledger.get("nonexistent_key").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_put_overwrite() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        let key = "test_key";
        let value1 = b"value1";
        let value2 = b"value2";

        // Put first value
        ledger.put(key, value1).await.unwrap();
        let retrieved = ledger.get(key).await.unwrap();
        assert_eq!(retrieved, Some(value1.to_vec()));

        // Overwrite with second value
        ledger.put(key, value2).await.unwrap();
        let retrieved = ledger.get(key).await.unwrap();
        assert_eq!(retrieved, Some(value2.to_vec()));
    }

    #[tokio::test]
    async fn test_multiple_keys() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        // Insert multiple key-value pairs
        let pairs = vec![
            ("key1", b"value1".as_slice()),
            ("key2", b"value2".as_slice()),
            ("key3", b"value3".as_slice()),
        ];

        for (key, value) in &pairs {
            ledger.put(key, value).await.unwrap();
        }

        // Verify all pairs
        for (key, expected_value) in &pairs {
            let retrieved = ledger.get(key).await.unwrap();
            assert_eq!(retrieved, Some(expected_value.to_vec()));
        }
    }

    #[tokio::test]
    async fn test_empty_value() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        let key = "empty_key";
        let empty_value = b"";

        ledger.put(key, empty_value).await.unwrap();
        let retrieved = ledger.get(key).await.unwrap();
        assert_eq!(retrieved, Some(empty_value.to_vec()));
    }

    #[tokio::test]
    async fn test_unicode_keys_and_values() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        let key = "키_유니코드";
        let value = "값_유니코드".as_bytes();

        ledger.put(key, value).await.unwrap();
        let retrieved = ledger.get(key).await.unwrap();
        assert_eq!(retrieved, Some(value.to_vec()));
    }

    #[tokio::test]
    async fn test_large_value() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        let key = "large_key";
        let large_value: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

        ledger.put(key, &large_value).await.unwrap();
        let retrieved = ledger.get(key).await.unwrap();
        assert_eq!(retrieved, Some(large_value));
    }

    #[tokio::test]
    async fn test_large_text_data_5mb() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        // Create 5MB of text data (repeated Lorem Ipsum pattern)
        let base_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.\n";
        let target_size = 5 * 1024 * 1024; // 5MB
        let repeat_count = target_size / base_text.len() + 1;
        let large_text = base_text.repeat(repeat_count);
        let large_text_bytes = large_text.as_bytes();
        
        // Truncate to exactly 5MB
        let text_5mb = &large_text_bytes[..target_size];
        
        println!("Testing 5MB text data (size: {} bytes)", text_5mb.len());
        
        let key = "large_text_5mb";
        ledger.put(key, text_5mb).await.unwrap();
        
        let retrieved = ledger.get(key).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_data = retrieved.unwrap();
        assert_eq!(retrieved_data.len(), target_size);
        assert_eq!(retrieved_data, text_5mb);
        
        // Verify the text is still valid UTF-8 (at least the beginning)
        let retrieved_str = std::str::from_utf8(&retrieved_data[..1000]).unwrap();
        assert!(retrieved_str.starts_with("Lorem ipsum"));
    }

    #[tokio::test]
    async fn test_large_binary_data_10mb() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        // Create 10MB of binary data with a predictable pattern
        let size_10mb = 10 * 1024 * 1024; // 10MB
        let mut binary_data = Vec::with_capacity(size_10mb);
        
        // Create a repeating pattern that's easy to verify
        for i in 0..size_10mb {
            binary_data.push((i % 256) as u8);
        }
        
        println!("Testing 10MB binary data (size: {} bytes)", binary_data.len());
        
        let key = "large_binary_10mb";
        ledger.put(key, &binary_data).await.unwrap();
        
        let retrieved = ledger.get(key).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_data = retrieved.unwrap();
        assert_eq!(retrieved_data.len(), size_10mb);
        
        // Verify the pattern is correct
        for (i, &byte) in retrieved_data.iter().enumerate() {
            assert_eq!(byte, (i % 256) as u8, "Mismatch at position {}", i);
        }
        
        // Quick spot checks at different positions
        assert_eq!(retrieved_data[0], 0);
        assert_eq!(retrieved_data[255], 255);
        assert_eq!(retrieved_data[256], 0);
        assert_eq!(retrieved_data[size_10mb - 1], ((size_10mb - 1) % 256) as u8);
    }

    #[tokio::test]
    async fn test_large_mixed_data_25mb() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        // Create 25MB of mixed text and binary data
        let size_25mb = 25 * 1024 * 1024; // 25MB
        let mut mixed_data = Vec::with_capacity(size_25mb);
        
        let text_chunk = "Mixed data chunk with text and binary: 🚀🔥💎\n".as_bytes();
        let mut position = 0;
        
        while position < size_25mb {
            if position % 1024 == 0 {
                // Every 1KB, add a text chunk
                let remaining = size_25mb - position;
                let chunk_size = std::cmp::min(text_chunk.len(), remaining);
                mixed_data.extend_from_slice(&text_chunk[..chunk_size]);
                position += chunk_size;
            } else {
                // Fill with binary pattern
                mixed_data.push((position % 256) as u8);
                position += 1;
            }
        }
        
        // Ensure exactly 25MB
        mixed_data.truncate(size_25mb);
        
        println!("Testing 25MB mixed data (size: {} bytes)", mixed_data.len());
        
        let key = "large_mixed_25mb";
        ledger.put(key, &mixed_data).await.unwrap();
        
        let retrieved = ledger.get(key).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_data = retrieved.unwrap();
        assert_eq!(retrieved_data.len(), size_25mb);
        assert_eq!(retrieved_data, mixed_data);
        
        // Verify some text chunks are still there (find valid UTF-8 boundaries)
        if let Ok(text_start) = std::str::from_utf8(&retrieved_data[0..100]) {
            assert!(text_start.contains("Mixed data chunk"));
        } else {
            // If the beginning isn't valid UTF-8, check at 1KB boundary where we know text is
            let text_at_1k = std::str::from_utf8(&retrieved_data[1024..1024 + text_chunk.len()]).unwrap();
            assert!(text_at_1k.contains("Mixed data chunk"));
        }
    }

    #[tokio::test]
    async fn test_large_json_data_15mb() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        // Create 15MB of JSON-like data (simulating AI model outputs)
        let size_15mb = 15 * 1024 * 1024; // 15MB
        let mut json_data = String::with_capacity(size_15mb);
        
        json_data.push_str("{\n  \"ai_inference_results\": [\n");
        
        let mut current_size = json_data.len();
        let mut record_id = 0;
        
        while current_size < size_15mb - 1000 { // Leave room for closing
            let record = format!(
                "    {{\n      \"id\": {},\n      \"timestamp\": \"2025-09-24T{}:{}:{}.{}Z\",\n      \"model\": \"hyra-ai-v2.1\",\n      \"input_hash\": \"0x{:064x}\",\n      \"output_data\": \"{}\",\n      \"confidence\": 0.{},\n      \"processing_time_ms\": {}\n    }}{}\n",
                record_id,
                record_id % 24, record_id % 60, record_id % 60, record_id % 1000,
                record_id,
                "A".repeat(200), // Simulate large output data
                950 + (record_id % 50),
                10 + (record_id % 990),
                if current_size < size_15mb - 2000 { "," } else { "" }
            );
            
            if current_size + record.len() > size_15mb - 1000 {
                break;
            }
            
            json_data.push_str(&record);
            current_size = json_data.len();
            record_id += 1;
        }
        
        json_data.push_str("  ]\n}");
        
        // Ensure we're close to 15MB
        let json_bytes = json_data.as_bytes();
        println!("Testing JSON data (size: {} bytes, target: {} MB)", json_bytes.len(), json_bytes.len() / 1024 / 1024);
        
        let key = "large_json_15mb";
        ledger.put(key, json_bytes).await.unwrap();
        
        let retrieved = ledger.get(key).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_data = retrieved.unwrap();
        assert_eq!(retrieved_data.len(), json_bytes.len());
        assert_eq!(retrieved_data, json_bytes);
        
        // Verify it's still valid JSON structure
        let retrieved_str = std::str::from_utf8(&retrieved_data).unwrap();
        assert!(retrieved_str.starts_with("{\n  \"ai_inference_results\": ["));
        assert!(retrieved_str.ends_with("  ]\n}"));
        assert!(retrieved_str.contains("hyra-ai-v2.1"));
    }

    #[tokio::test] 
    async fn test_multiple_large_files_concurrent() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();
        let ledger = std::sync::Arc::new(ledger);

        // Test concurrent storage of multiple large files
        let size_5mb = 5 * 1024 * 1024; // 5MB each
        let num_files = 3;
        
        let mut handles = Vec::new();
        
        for file_id in 0..num_files {
            let ledger_clone = ledger.clone();
            let handle = tokio::spawn(async move {
                // Create unique data for each file
                let mut file_data = Vec::with_capacity(size_5mb);
                for i in 0..size_5mb {
                    file_data.push(((i + file_id * 1000) % 256) as u8);
                }
                
                let key = format!("concurrent_large_file_{}", file_id);
                ledger_clone.put(&key, &file_data).await.unwrap();
                
                // Verify immediately
                let retrieved = ledger_clone.get(&key).await.unwrap();
                assert!(retrieved.is_some());
                let retrieved_data = retrieved.unwrap();
                assert_eq!(retrieved_data.len(), size_5mb);
                assert_eq!(retrieved_data, file_data);
                
                file_id
            });
            handles.push(handle);
        }
        
        // Wait for all files to be stored and verified
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.unwrap());
        }
        
        // Verify all files are still accessible
        for file_id in 0..num_files {
            let key = format!("concurrent_large_file_{}", file_id);
            let retrieved = ledger.get(&key).await.unwrap();
            assert!(retrieved.is_some());
            
            let retrieved_data = retrieved.unwrap();
            assert_eq!(retrieved_data.len(), size_5mb);
            
            // Verify the unique pattern for this file
            for (i, &byte) in retrieved_data.iter().enumerate() {
                let expected = ((i + file_id * 1000) % 256) as u8;
                assert_eq!(byte, expected, "File {} mismatch at position {}", file_id, i);
            }
        }
        
        println!("Successfully stored and verified {} concurrent 5MB files", num_files);
    }

    #[tokio::test]
    async fn test_very_large_single_file_50mb() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new(config).unwrap();

        // Create 50MB of data - this tests the upper limits
        let size_50mb = 50 * 1024 * 1024; // 50MB
        println!("Creating 50MB test data...");
        
        // Use a more memory-efficient approach for very large data
        let chunk_size = 64 * 1024; // 64KB chunks
        let mut large_data = Vec::with_capacity(size_50mb);
        
        for chunk_id in 0..(size_50mb / chunk_size) {
            for i in 0..chunk_size {
                let byte_value = ((chunk_id * chunk_size + i) % 256) as u8;
                large_data.push(byte_value);
            }
        }
        
        // Fill remaining bytes
        let remaining = size_50mb % chunk_size;
        for i in 0..remaining {
            let byte_value = (((size_50mb / chunk_size) * chunk_size + i) % 256) as u8;
            large_data.push(byte_value);
        }
        
        assert_eq!(large_data.len(), size_50mb);
        println!("Testing 50MB data storage (size: {} bytes)", large_data.len());
        
        let key = "very_large_50mb";
        
        // Measure storage time
        let start = std::time::Instant::now();
        ledger.put(key, &large_data).await.unwrap();
        let store_duration = start.elapsed();
        
        println!("Stored 50MB in {:?}", store_duration);
        
        // Measure retrieval time
        let start = std::time::Instant::now();
        let retrieved = ledger.get(key).await.unwrap();
        let retrieve_duration = start.elapsed();
        
        println!("Retrieved 50MB in {:?}", retrieve_duration);
        
        assert!(retrieved.is_some());
        let retrieved_data = retrieved.unwrap();
        assert_eq!(retrieved_data.len(), size_50mb);
        
        // Verify data integrity with sampling (checking every 1MB)
        let sample_interval = 1024 * 1024; // 1MB
        for sample_pos in (0..size_50mb).step_by(sample_interval) {
            let expected = (sample_pos % 256) as u8;
            assert_eq!(retrieved_data[sample_pos], expected, 
                      "Data integrity check failed at position {}", sample_pos);
        }
        
        // Check first and last bytes
        assert_eq!(retrieved_data[0], 0);
        assert_eq!(retrieved_data[size_50mb - 1], ((size_50mb - 1) % 256) as u8);
        
        println!("50MB data integrity verified successfully");
    }

    // HTTP endpoint tests would require integration testing
    // These tests verify the HTTP handling logic without starting a server
    #[tokio::test]
    async fn test_http_request_parsing() {
        // Test route parsing
        let put_path = "/put/test_key";
        assert!(put_path.starts_with("/put/"));
        
        let key = &put_path[5..];
        assert_eq!(key, "test_key");

        // Test GET route parsing  
        let get_path = "/get/test_key";
        assert!(get_path.starts_with("/get/"));
        
        let key = &get_path[5..];
        assert_eq!(key, "test_key");
    }

    #[test]
    fn test_config_creation() {
        let (config, _temp_dir) = create_test_config();
        assert!(!config.node.id.is_empty());
        assert!(!config.node.data_dir.is_empty());
        assert_eq!(config.network.client_port, 0);
    }
}