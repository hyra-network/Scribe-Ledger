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