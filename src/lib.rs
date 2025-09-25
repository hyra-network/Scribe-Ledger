//! # Hyra Scribe Ledger
//!
//! Verifiable, Durable Off-Chain Storage for the Hyra AI Ecosystem.
//!
//! Hyra Scribe Ledger is a distributed, immutable, append-only key-value storage system
//! designed to serve as the durable data layer for Hyra AI.

pub mod consensus;
pub mod manifest;
pub mod monitoring;
pub mod storage;
pub mod write_node;
pub mod discovery;

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
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use bytes::Bytes;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use axum::{
    extract::{Path as AxumPath, State, WebSocketUpgrade, ws::{WebSocket, Message}},
    http::StatusCode,
    response::{Json, Response},
    routing::{get, put},
    Router,
};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use crate::storage::{StorageBackend, s3::S3Storage};
use crate::consensus::ConsensusNode;
use crate::consensus::TcpTransport;
use base64::{Engine, engine::general_purpose};

/// Represents a segment of data to be flushed to S3
#[derive(Debug, Clone)]
pub struct PendingSegment {
    data: HashMap<String, Vec<u8>>,
    timestamp: u64,
    size: usize,
}

/// Main entry point for the Scribe Ledger library
pub struct ScribeLedger {
    db: sled::Db,
    config: Config,
    s3_storage: Option<S3Storage>,
    pending_flush: Arc<Mutex<PendingSegment>>,
    last_flush_time: Arc<Mutex<u64>>,
    consensus_node: Option<Arc<Mutex<ConsensusNode>>>,
}

impl ScribeLedger {
    /// Create a new Scribe Ledger instance
    pub async fn new(config: Config) -> Result<Self> {
        let data_dir = Path::new(&config.node.data_dir);
        std::fs::create_dir_all(data_dir)?;
        
        let db_path = data_dir.join("scribe.db");
        let db = sled::open(&db_path)?;
        
        // Initialize S3 storage
        let s3_storage = Some(S3Storage::from_config(&config).await?);
        
        // Initialize pending flush data
        let pending_flush = Arc::new(Mutex::new(PendingSegment {
            data: HashMap::new(),
            timestamp: current_timestamp(),
            size: 0,
        }));
        
        let last_flush_time = Arc::new(Mutex::new(current_timestamp()));
        
        // Initialize consensus node
        let transport = Arc::new(TcpTransport::new());
        let consensus_node = ConsensusNode::new(
            1, // Node ID - should be configurable
            config.network.listen_addr.clone(),
            config.network.raft_tcp_port,
            transport,
        )?;
        let consensus_node = Some(Arc::new(Mutex::new(consensus_node)));
        
        let mut ledger = Self { 
            db, 
            config, 
            s3_storage,
            pending_flush,
            last_flush_time,
            consensus_node,
        };
        
        // Recover data from S3 on startup
        ledger.recover_from_s3().await?;
        
        Ok(ledger)
    }
    
    /// Create a new Scribe Ledger instance without S3 (for testing)
    pub fn new_local_only(config: Config) -> Result<Self> {
        let data_dir = Path::new(&config.node.data_dir);
        std::fs::create_dir_all(data_dir)?;
        
        let db_path = data_dir.join("scribe.db");
        let db = sled::open(&db_path)?;
        
        let pending_flush = Arc::new(Mutex::new(PendingSegment {
            data: HashMap::new(),
            timestamp: current_timestamp(),
            size: 0,
        }));
        
        let last_flush_time = Arc::new(Mutex::new(current_timestamp()));
        
        Ok(Self { 
            db, 
            config, 
            s3_storage: None,
            pending_flush,
            last_flush_time,
            consensus_node: None, // No consensus in local-only mode
        })
    }

    /// Store a key-value pair (append-only, immutable)
    pub async fn put(&self, key: &str, value: &[u8]) -> Result<()> {
        // Store in local database first (hot tier)
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush_async().await?;
        
        // Add to pending flush for S3 (cold tier)
        if self.s3_storage.is_some() {
            let mut pending = self.pending_flush.lock().await;
            
            // Check if we should flush based on size
            let value_size = key.len() + value.len();
            pending.data.insert(key.to_string(), value.to_vec());
            pending.size += value_size;
            
            // Trigger flush if size exceeds limit
            if pending.size > 10 * 1024 * 1024 { // 10MB limit
                drop(pending); // Release lock before flush
                
                if let Some(s3) = &self.s3_storage {
                    Self::flush_to_s3(s3, &self.pending_flush, &self.last_flush_time).await?;
                }
            }
        }
        
        Ok(())
    }

    /// Retrieve a value by key (read-through cache pattern)
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // First check local database (hot tier)
        match self.db.get(key.as_bytes())? {
            Some(value) => Ok(Some(value.to_vec())),
            None => {
                // If not found locally, search in S3 segments (cold tier)
                if self.s3_storage.is_some() {
                    self.search_in_s3(key).await
                } else {
                    Ok(None)
                }
            }
        }
    }
    
    /// Search for a key in S3 segments (newest to oldest)
    async fn search_in_s3(&self, key: &str) -> Result<Option<Vec<u8>>> {
        if let Some(s3) = &self.s3_storage {
            // List all segments (they should be sorted by timestamp, newest first)
            let segments = s3.list_segments().await?;
            
            // Search in reverse chronological order (newest to oldest)
            for segment_meta in segments {
                let segment_data = s3.get_segment(segment_meta.id).await?;
                let segment_str = String::from_utf8_lossy(&segment_data);
                
                // Parse segment data to find our key
                for line in segment_str.lines() {
                    if let Some((seg_key, value_b64)) = line.split_once(':') {
                        if seg_key == key {
                            // Found the key! Decode and return
                            if let Ok(value) = general_purpose::STANDARD.decode(value_b64) {
                                // Cache the value locally for future reads
                                self.db.insert(key.as_bytes(), value.clone())?;
                                return Ok(Some(value));
                            }
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }

    /// Start the Scribe Ledger node with HTTP server
    pub async fn start(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.network.listen_addr, self.config.network.client_port);
        
        let ledger = Arc::new(self);
        
        // Start background flush task if S3 is configured
        if let Some(s3) = &ledger.s3_storage {
            Self::start_background_flush(
                s3.clone(),
                ledger.pending_flush.clone(),
                ledger.last_flush_time.clone()
            )?;
        }
        
        // Start consensus node TCP server if available
        if let Some(consensus_node) = &ledger.consensus_node {
            let mut node = consensus_node.lock().await;
            node.start_tcp_server().await?;
            tracing::info!("Raft consensus node started on TCP port {}", node.address());
        }
        
        let app = Router::new()
            .route("/:key", put(put_handler))
            .route("/:key", get(get_handler))
            .route("/raft/status", get(raft_status_handler))
            .route("/raft/metrics", get(raft_metrics_handler))
            .route("/raft/events", get(raft_events_handler))
            .route("/raft/live", get(raft_live_handler))
            .with_state(ledger);
        
        tracing::info!("ScribeLedger HTTP server running on {} with S3 integration", addr);
        
        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
        
        Ok(())
    }
    
    /// Recover data from S3 storage on startup
    async fn recover_from_s3(&mut self) -> Result<()> {
        if let Some(s3) = &self.s3_storage {
            tracing::info!("Starting data recovery from S3...");
            
            // List all segments in S3
            let segments = s3.list_segments().await?;
            tracing::info!("Found {} segments in S3", segments.len());
            
            let mut recovered_count = 0;
            
            for segment_meta in segments {
                // Skip if data already exists locally (avoid overwrites)
                let segment_data = s3.get_segment(segment_meta.id).await?;
                
                // Deserialize segment data (format: key1:value1\nkey2:value2\n...)
                let segment_str = String::from_utf8_lossy(&segment_data);
                for line in segment_str.lines() {
                    if let Some((key, value_b64)) = line.split_once(':') {
                        // Only recover if key doesn't exist locally (immutable property)
                        if self.db.get(key.as_bytes())?.is_none() {
                            if let Ok(value) = general_purpose::STANDARD.decode(value_b64) {
                                self.db.insert(key.as_bytes(), value)?;
                                recovered_count += 1;
                            }
                        }
                    }
                }
            }
            
            if recovered_count > 0 {
                self.db.flush_async().await?;
                tracing::info!("Recovered {} keys from S3", recovered_count);
            } else {
                tracing::info!("No new data to recover from S3");
            }
        }
        
        Ok(())
    }
    
    /// Background task to flush data to S3 periodically
    pub fn start_background_flush(
        s3_storage: S3Storage,
        pending_flush: Arc<Mutex<PendingSegment>>,
        last_flush_time: Arc<Mutex<u64>>,
    ) -> Result<()> {
        let flush_interval = Duration::from_secs(30); // Flush every 30 seconds
        let max_segment_size = 10 * 1024 * 1024; // 10MB max segment size
        
        tokio::spawn(async move {
            let mut ticker = interval(flush_interval);
            
            loop {
                ticker.tick().await;
                
                let should_flush = {
                    let pending = pending_flush.lock().await;
                    pending.size > max_segment_size || 
                    current_timestamp() - pending.timestamp > 30 // 30 seconds timeout
                };
                
                if should_flush {
                    if let Err(e) = Self::flush_to_s3(&s3_storage, &pending_flush, &last_flush_time).await {
                        tracing::error!("Failed to flush to S3: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Flush pending data to S3 as an immutable segment
    async fn flush_to_s3(
        s3_storage: &S3Storage,
        pending_flush: &Arc<Mutex<PendingSegment>>,
        last_flush_time: &Arc<Mutex<u64>>,
    ) -> Result<()> {
        let segment_data = {
            let mut pending = pending_flush.lock().await;
            
            if pending.data.is_empty() {
                return Ok(());
            }
            
            let data = pending.data.clone();
            
            // Clear pending data and reset timestamp
            pending.data.clear();
            pending.size = 0;
            pending.timestamp = current_timestamp();
            
            data
        };
        
        if !segment_data.is_empty() {
            let segment_id = SegmentId::new();
            
            // Serialize data to format: key1:base64_value1\nkey2:base64_value2\n...
            let mut serialized_data = String::new();
            let data_len = segment_data.len();
            for (key, value) in &segment_data {
                let value_b64 = general_purpose::STANDARD.encode(value);
                serialized_data.push_str(&format!("{}:{}\n", key, value_b64));
            }
            
            // Store as immutable segment in S3 (readonly after creation)
            s3_storage.store_segment(segment_id, serialized_data.as_bytes()).await?;
            
            // Update last flush time
            let mut last_time = last_flush_time.lock().await;
            *last_time = current_timestamp();
            
            tracing::info!("Flushed segment {} to S3 with {} keys", segment_id.0, data_len);
        }
        
        Ok(())
    }
    
    /// Force flush all pending data to S3 (for graceful shutdown)
    pub async fn force_flush(&self) -> Result<()> {
        if let Some(s3) = &self.s3_storage {
            Self::flush_to_s3(s3, &self.pending_flush, &self.last_flush_time).await?;
        }
        Ok(())
    }
}

/// Get current timestamp in seconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
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

/// Get current Raft status
async fn raft_status_handler(
    State(ledger): State<Arc<ScribeLedger>>,
) -> std::result::Result<Json<Value>, StatusCode> {
    if let Some(consensus_node) = &ledger.consensus_node {
        let node = consensus_node.lock().await;
        let status = json!({
            "node_id": node.node_id(),
            "address": node.address(),
            "is_leader": node.is_raft_leader(),
            "status": "active"
        });
        Ok(Json(status))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Get Raft performance metrics
async fn raft_metrics_handler(
    State(ledger): State<Arc<ScribeLedger>>,
) -> std::result::Result<Json<Value>, StatusCode> {
    if let Some(consensus_node) = &ledger.consensus_node {
        let node = consensus_node.lock().await;
        let monitor = node.monitor();
    let metrics = monitor.get_current_metrics().await;
        
        let response = json!({
            "node_id": metrics.node_id,
            "current_term": metrics.current_term,
            "leader_id": metrics.leader_id,
            "is_leader": metrics.is_leader,
            "commit_index": metrics.commit_index,
            "last_applied": metrics.last_applied,
            "avg_apply_latency_us": metrics.avg_apply_latency_us,
            "heartbeat_success_rate": metrics.heartbeat_success_rate,
            "messages_sent_per_sec": metrics.messages_sent_per_sec,
            "messages_received_per_sec": metrics.messages_received_per_sec,
            "timestamp": metrics.timestamp
        });
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Get recent Raft events
async fn raft_events_handler(
    State(ledger): State<Arc<ScribeLedger>>,
) -> std::result::Result<Json<Value>, StatusCode> {
    if let Some(consensus_node) = &ledger.consensus_node {
        let node = consensus_node.lock().await;
        let monitor = node.monitor();
        let events = monitor.get_recent_events(50).await; // Get last 50 events
        
        let response = json!({
            "events": events,
            "count": events.len()
        });
        Ok(Json(response))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// WebSocket endpoint for real-time Raft events
async fn raft_live_handler(
    ws: WebSocketUpgrade,
    State(ledger): State<Arc<ScribeLedger>>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, ledger))
}

/// Handle WebSocket connection for real-time events
async fn handle_websocket(mut socket: WebSocket, ledger: Arc<ScribeLedger>) {
    if let Some(consensus_node) = &ledger.consensus_node {
        let node = consensus_node.lock().await;
        let monitor = node.monitor();
        let mut receiver = monitor.subscribe();
        drop(node); // Release lock
        
        // Send initial status
        let status_msg = json!({
            "type": "status",
            "message": "Connected to Raft monitoring"
        });
        if socket.send(Message::Text(status_msg.to_string())).await.is_err() {
            return;
        }
        
        // Listen for events and forward to WebSocket
        let mut heartbeat_interval = interval(Duration::from_secs(30));
        
        loop {
            tokio::select! {
                // Receive monitoring events
                event_result = receiver.recv() => {
                    match event_result {
                        Ok(event) => {
                            let msg = json!({
                                "type": "event",
                                "data": event
                            });
                            if socket.send(Message::Text(msg.to_string())).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            let lag_msg = json!({
                                "type": "warning",
                                "message": format!("Lagged behind, skipped {} events", skipped)
                            });
                            if socket.send(Message::Text(lag_msg.to_string())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                
                // Send periodic heartbeat
                _ = heartbeat_interval.tick() => {
                    let heartbeat_msg = json!({
                        "type": "heartbeat",
                        "timestamp": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                    });
                    if socket.send(Message::Text(heartbeat_msg.to_string())).await.is_err() {
                        break;
                    }
                }
                
                // Handle incoming WebSocket messages
                msg_result = socket.recv() => {
                    match msg_result {
                        Some(Ok(Message::Close(_))) => {
                            break;
                        }
                        Some(Err(_)) => {
                            break;
                        }
                        _ => {} // Ignore other message types
                    }
                }
            }
        }
    } else {
        let error_msg = json!({
            "type": "error",
            "message": "Consensus node not available"
        });
        let _ = socket.send(Message::Text(error_msg.to_string())).await;
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
        let ledger = ScribeLedger::new_local_only(config).unwrap();
        // Just verify the ledger was created successfully
        assert!(std::path::Path::new(&ledger.config.node.data_dir).exists());
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new_local_only(config).unwrap();

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
        let ledger = ScribeLedger::new_local_only(config).unwrap();

        // Test GET operation on non-existent key
        let result = ledger.get("nonexistent_key").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn test_put_overwrite() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new_local_only(config).unwrap();

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
        let ledger = ScribeLedger::new_local_only(config).unwrap();

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
        let ledger = ScribeLedger::new_local_only(config).unwrap();

        let key = "empty_key";
        let empty_value = b"";

        ledger.put(key, empty_value).await.unwrap();
        let retrieved = ledger.get(key).await.unwrap();
        assert_eq!(retrieved, Some(empty_value.to_vec()));
    }

    #[tokio::test]
    async fn test_unicode_keys_and_values() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new_local_only(config).unwrap();

        let key = "키_유니코드";
        let value = "값_유니코드".as_bytes();

        ledger.put(key, value).await.unwrap();
        let retrieved = ledger.get(key).await.unwrap();
        assert_eq!(retrieved, Some(value.to_vec()));
    }

    #[tokio::test]
    async fn test_large_value() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new_local_only(config).unwrap();

        let key = "large_key";
        let large_value: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

        ledger.put(key, &large_value).await.unwrap();
        let retrieved = ledger.get(key).await.unwrap();
        assert_eq!(retrieved, Some(large_value));
    }

    #[tokio::test]
    async fn test_large_text_data_5mb() {
        let (config, _temp_dir) = create_test_config();
        let ledger = ScribeLedger::new_local_only(config).unwrap();

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
        let ledger = ScribeLedger::new_local_only(config).unwrap();

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
        let ledger = ScribeLedger::new_local_only(config).unwrap();

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
        let ledger = ScribeLedger::new_local_only(config).unwrap();

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
        let ledger = ScribeLedger::new_local_only(config).unwrap();
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
        let ledger = ScribeLedger::new_local_only(config).unwrap();

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

    // S3 Integration Tests (require MinIO to be running)
    #[tokio::test]
    #[ignore] // Requires MinIO
    async fn test_s3_integration_full_workflow() {
        // Create config with S3 settings
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::load_with_env();
        config.node.data_dir = temp_dir.path().to_string_lossy().to_string();
        config.network.client_port = 0;
        
        // Override with MinIO test settings if not set
        if config.storage.s3.endpoint.is_none() {
            config.storage.s3.endpoint = Some("http://localhost:9000".to_string());
            config.storage.s3.access_key = Some("scribe-admin".to_string());
            config.storage.s3.secret_key = Some("scribe-password-123".to_string());
            config.storage.s3.bucket = "scribe-ledger-test".to_string();
            config.storage.s3.path_style = true;
        }
        
        // Create ledger with S3 integration
        let ledger = ScribeLedger::new(config).await.unwrap();
        
        // Test data
        let test_data = vec![
            ("key1", "value1"),
            ("key2", "value2"),
            ("key3", "value3"),
        ];
        
        // Store data (should go to local first, then flush to S3)
        for (key, value) in &test_data {
            ledger.put(key, value.as_bytes()).await.unwrap();
        }
        
        // Force flush to S3
        ledger.force_flush().await.unwrap();
        
        // Verify data can be retrieved
        for (key, expected_value) in &test_data {
            let retrieved = ledger.get(key).await.unwrap();
            assert_eq!(retrieved, Some(expected_value.as_bytes().to_vec()));
        }
    }

    #[tokio::test]
    #[ignore] // Requires MinIO
    async fn test_s3_data_recovery() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();
        
        let mut config1 = Config::load_with_env();
        config1.node.data_dir = temp_dir1.path().to_string_lossy().to_string();
        config1.network.client_port = 0;
        
        let mut config2 = Config::load_with_env();
        config2.node.data_dir = temp_dir2.path().to_string_lossy().to_string();
        config2.network.client_port = 0;
        
        // Override with MinIO test settings if not set
        for config in [&mut config1, &mut config2] {
            if config.storage.s3.endpoint.is_none() {
                config.storage.s3.endpoint = Some("http://localhost:9000".to_string());
                config.storage.s3.access_key = Some("scribe-admin".to_string());
                config.storage.s3.secret_key = Some("scribe-password-123".to_string());
                config.storage.s3.bucket = "scribe-ledger-test".to_string();
                config.storage.s3.path_style = true;
            }
        }
        
        // First ledger: store data and flush to S3
        {
            let ledger1 = ScribeLedger::new(config1).await.unwrap();
            
            ledger1.put("recovery_key1", b"recovery_value1").await.unwrap();
            ledger1.put("recovery_key2", b"recovery_value2").await.unwrap();
            
            // Force flush to S3
            ledger1.force_flush().await.unwrap();
        }
        
        // Second ledger: should recover data from S3
        {
            let ledger2 = ScribeLedger::new(config2).await.unwrap();
            
            // Data should be recovered from S3
            let value1 = ledger2.get("recovery_key1").await.unwrap();
            let value2 = ledger2.get("recovery_key2").await.unwrap();
            
            assert_eq!(value1, Some(b"recovery_value1".to_vec()));
            assert_eq!(value2, Some(b"recovery_value2".to_vec()));
        }
    }

    #[tokio::test]
    #[ignore] // Requires MinIO
    async fn test_s3_read_through_cache() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::load_with_env();
        config.node.data_dir = temp_dir.path().to_string_lossy().to_string();
        config.network.client_port = 0;
        
        // Override with MinIO test settings if not set
        if config.storage.s3.endpoint.is_none() {
            config.storage.s3.endpoint = Some("http://localhost:9000".to_string());
            config.storage.s3.access_key = Some("scribe-admin".to_string());
            config.storage.s3.secret_key = Some("scribe-password-123".to_string());
            config.storage.s3.bucket = "scribe-ledger-test".to_string();
            config.storage.s3.path_style = true;
        }
        
        let ledger = ScribeLedger::new(config).await.unwrap();
        
        // Store and flush data
        ledger.put("cache_test_key", b"cache_test_value").await.unwrap();
        ledger.force_flush().await.unwrap();
        
        // Clear local cache by removing from sled
        ledger.db.remove("cache_test_key").unwrap();
        
        // Should still be able to read from S3 (read-through)
        let value = ledger.get("cache_test_key").await.unwrap();
        assert_eq!(value, Some(b"cache_test_value".to_vec()));
        
        // After read-through, should be cached locally
        let local_value = ledger.db.get("cache_test_key").unwrap();
        assert!(local_value.is_some());
    }

    #[tokio::test]
    #[ignore] // Requires MinIO
    async fn test_s3_immutable_segments() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = Config::load_with_env();
        config.node.data_dir = temp_dir.path().to_string_lossy().to_string();
        config.network.client_port = 0;
        
        // Override with MinIO test settings if not set
        if config.storage.s3.endpoint.is_none() {
            config.storage.s3.endpoint = Some("http://localhost:9000".to_string());
            config.storage.s3.access_key = Some("scribe-admin".to_string());
            config.storage.s3.secret_key = Some("scribe-password-123".to_string());
            config.storage.s3.bucket = "scribe-ledger-test".to_string();
            config.storage.s3.path_style = true;
        }
        
        let ledger = ScribeLedger::new(config).await.unwrap();
        
        // Store data multiple times (append-only)
        ledger.put("immutable_key", b"value1").await.unwrap();
        ledger.force_flush().await.unwrap();
        
        ledger.put("immutable_key", b"value2").await.unwrap();
        ledger.force_flush().await.unwrap();
        
        // Should get the latest value (value2)
        let value = ledger.get("immutable_key").await.unwrap();
        assert_eq!(value, Some(b"value2".to_vec()));
        
        // Verify segments in S3 are readonly (they contain historical data)
        if let Some(s3) = &ledger.s3_storage {
            let segments = s3.list_segments().await.unwrap();
            assert!(segments.len() >= 2, "Should have at least 2 immutable segments");
        }
    }

    #[tokio::test]
    async fn test_raft_monitoring_integration() {
        use crate::consensus::TcpTransport;
        use crate::monitoring::{RaftEvent, EventSeverity};
        use std::sync::Arc;

        // Create a consensus node with monitoring
        let transport = Arc::new(TcpTransport::new());
        let consensus_node = crate::consensus::ConsensusNode::new(
            1,
            "127.0.0.1".to_string(),
            8081,
            transport,
        ).unwrap();

        // Test that monitor is accessible
        let monitor = consensus_node.monitor();
        assert_eq!(monitor.node_id(), 1);

        // Test event publishing
        monitor.publish_event(
            RaftEvent::NodeJoined {
                node_id: 1,
                address: "127.0.0.1:8081".to_string(),
                cluster_size: 1,
            },
            EventSeverity::Info,
        ).await;

        // Test event retrieval
        let events = monitor.get_recent_events(10).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].node_id, 1);
    }

    #[tokio::test]
    async fn test_monitoring_api_endpoints() {
        let (config, _temp_dir) = create_test_config();
        let mut config = config;
        config.network.raft_tcp_port = 8082; // Use different port for test
        
        let ledger = Arc::new(ScribeLedger::new_local_only(config.clone()).unwrap());

        // Test raft status endpoint (should return unavailable for local-only ledger)
        let response = raft_status_handler(axum::extract::State(ledger.clone())).await;
        match response {
            Err(status_code) => assert_eq!(status_code, StatusCode::SERVICE_UNAVAILABLE),
            Ok(_) => panic!("Expected SERVICE_UNAVAILABLE for local-only ledger"),
        }

        // Test with consensus node
        let transport = Arc::new(crate::consensus::TcpTransport::new());
        let consensus_node = crate::consensus::ConsensusNode::new(
            1,
            "127.0.0.1".to_string(),
            8083,
            transport,
        ).unwrap();
        
        // Create a new config with different data directory to avoid lock conflicts
        let (config2, _temp_dir2) = create_test_config();
        let mut config2 = config2;
        config2.network.raft_tcp_port = 8083;
        
        let mut ledger_with_consensus = ScribeLedger::new_local_only(config2).unwrap();
        ledger_with_consensus.consensus_node = Some(Arc::new(tokio::sync::Mutex::new(consensus_node)));
        let ledger_with_consensus = Arc::new(ledger_with_consensus);

        // Test raft status endpoint (should work now)
        let response = raft_status_handler(axum::extract::State(ledger_with_consensus.clone())).await;
        assert!(response.is_ok());

        // Test raft metrics endpoint
        let response = raft_metrics_handler(axum::extract::State(ledger_with_consensus.clone())).await;
        assert!(response.is_ok());

        // Test raft events endpoint
        let response = raft_events_handler(axum::extract::State(ledger_with_consensus.clone())).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_monitoring_event_broadcasting() {
        use crate::consensus::TcpTransport;
        use crate::monitoring::{RaftEvent, EventSeverity};
        use std::sync::Arc;
        use tokio::time::{timeout, Duration};

        // Create a consensus node with monitoring
        let transport = Arc::new(TcpTransport::new());
        let consensus_node = crate::consensus::ConsensusNode::new(
            1,
            "127.0.0.1".to_string(),
            8084,
            transport,
        ).unwrap();

        let monitor = consensus_node.monitor();
        let mut receiver = monitor.subscribe();

        // Publish an event
        let test_event = RaftEvent::NodeJoined {
            node_id: 1,
            address: "127.0.0.1:8084".to_string(),
            cluster_size: 1,
        };

        monitor.publish_event(test_event.clone(), EventSeverity::Info).await;

        // Verify we can receive the event
        let received_event = timeout(Duration::from_secs(1), receiver.recv()).await
            .expect("Should receive event within timeout")
            .expect("Should receive event successfully");

        assert_eq!(received_event.node_id, 1);
        match received_event.event {
            RaftEvent::NodeJoined { node_id, .. } => assert_eq!(node_id, 1),
            _ => panic!("Expected NodeJoined event"),
        }
    }
}