//! Configuration system for Simple Scribe Ledger
//!
//! This module provides configuration management with TOML file parsing and
//! environment variable override support.

use crate::error::{Result, ScribeError};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration structure for the distributed ledger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Node-specific configuration
    pub node: NodeConfig,
    /// Network configuration
    pub network: NetworkConfig,
    /// Storage configuration
    pub storage: StorageConfig,
    /// Consensus/Raft configuration
    pub consensus: ConsensusConfig,
    /// API configuration
    #[serde(default)]
    pub api: ApiConfig,
    /// Discovery configuration
    #[serde(default)]
    pub discovery: DiscoveryConfig,
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Unique node identifier
    pub id: u64,
    /// Node address (hostname or IP)
    pub address: String,
    /// Data directory for this node
    pub data_dir: PathBuf,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Address to listen on for client connections
    pub listen_addr: SocketAddr,
    /// Port for client HTTP API
    pub client_port: u16,
    /// Port for Raft consensus communication
    pub raft_port: u16,
    /// Seed peer addresses for initial discovery (optional)
    /// Format: ["node_id@host:port", "node_id@host:port"]
    #[serde(default)]
    pub seed_peers: Vec<String>,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Maximum size of a data segment in bytes
    pub segment_size: usize,
    /// Maximum cache size in bytes
    pub max_cache_size: usize,
    /// S3 storage configuration (optional)
    #[serde(default)]
    pub s3: Option<S3Config>,
}

/// S3 storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    /// S3 bucket name
    pub bucket: String,
    /// S3 region
    pub region: String,
    /// S3 endpoint URL (for MinIO compatibility)
    pub endpoint: Option<String>,
    /// Access key ID
    pub access_key_id: Option<String>,
    /// Secret access key
    pub secret_access_key: Option<String>,
    /// Enable path-style addressing (required for MinIO)
    #[serde(default)]
    pub path_style: bool,
    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: usize,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

fn default_pool_size() -> usize {
    10
}

fn default_timeout() -> u64 {
    30
}

fn default_max_retries() -> u32 {
    3
}

/// Consensus configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Election timeout minimum in milliseconds
    #[serde(default = "default_election_timeout_min")]
    pub election_timeout_min: u64,
    /// Election timeout maximum in milliseconds (for randomization)
    #[serde(default = "default_election_timeout_max")]
    pub election_timeout_max: u64,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Maximum batch size for Raft proposals
    #[serde(default = "default_max_payload_entries")]
    pub max_payload_entries: u64,
    /// Snapshot policy: number of logs to keep in memory before triggering snapshot
    #[serde(default = "default_snapshot_policy")]
    pub snapshot_logs_since_last: u64,
    /// Maximum number of entries to send in a single append entries request
    #[serde(default = "default_max_in_snapshot_log_to_keep")]
    pub max_in_snapshot_log_to_keep: u64,
}

fn default_election_timeout_min() -> u64 {
    1500
}

fn default_election_timeout_max() -> u64 {
    3000
}

fn default_max_payload_entries() -> u64 {
    300
}

fn default_snapshot_policy() -> u64 {
    5000
}

fn default_max_in_snapshot_log_to_keep() -> u64 {
    1000
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Write timeout in seconds
    #[serde(default = "default_write_timeout_secs")]
    pub write_timeout_secs: u64,
    /// Read timeout in seconds
    #[serde(default = "default_read_timeout_secs")]
    pub read_timeout_secs: u64,
    /// Maximum batch size for write operations
    #[serde(default = "default_api_batch_size")]
    pub max_batch_size: usize,
    /// Cache capacity for hot data
    #[serde(default = "default_cache_capacity")]
    pub cache_capacity: usize,
}

fn default_write_timeout_secs() -> u64 {
    30
}

fn default_read_timeout_secs() -> u64 {
    10
}

fn default_api_batch_size() -> usize {
    100
}

fn default_cache_capacity() -> usize {
    1000
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            write_timeout_secs: default_write_timeout_secs(),
            read_timeout_secs: default_read_timeout_secs(),
            max_batch_size: default_api_batch_size(),
            cache_capacity: default_cache_capacity(),
        }
    }
}

/// Discovery service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Heartbeat interval in milliseconds
    #[serde(default = "default_discovery_heartbeat_ms")]
    pub heartbeat_interval_ms: u64,
    /// Failure detection timeout in milliseconds
    #[serde(default = "default_discovery_failure_timeout_ms")]
    pub failure_timeout_ms: u64,
    /// UDP port for discovery broadcasts (default: 17946)
    #[serde(default = "default_discovery_port")]
    pub discovery_port: u16,
    /// Broadcast address for local discovery (default: 255.255.255.255 for LAN)
    #[serde(default = "default_broadcast_addr")]
    pub broadcast_addr: String,
    /// Secret token for cross-network node authentication (optional)
    /// Nodes must have matching tokens to join the same cluster across networks
    #[serde(default)]
    pub cluster_secret: Option<String>,
}

fn default_discovery_heartbeat_ms() -> u64 {
    500
}

fn default_discovery_failure_timeout_ms() -> u64 {
    1500
}

fn default_discovery_port() -> u16 {
    17946
}

fn default_broadcast_addr() -> String {
    "255.255.255.255".to_string()
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: default_discovery_heartbeat_ms(),
            failure_timeout_ms: default_discovery_failure_timeout_ms(),
            discovery_port: default_discovery_port(),
            broadcast_addr: default_broadcast_addr(),
            cluster_secret: None,
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> Result<Self> {
        let contents = std::fs::read_to_string(path).map_err(|e| {
            ScribeError::Configuration(format!("Failed to read config file: {}", e))
        })?;

        let mut config: Config = toml::from_str(&contents)?;

        // Apply environment variable overrides
        config.apply_env_overrides();

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Create a default configuration for testing
    pub fn default_for_node(node_id: u64) -> Self {
        Self {
            node: NodeConfig {
                id: node_id,
                address: "127.0.0.1".to_string(),
                data_dir: PathBuf::from(format!("./node-{}", node_id)),
            },
            network: NetworkConfig {
                listen_addr: format!("127.0.0.1:{}", 8000 + node_id)
                    .parse()
                    .expect("Valid socket address"),
                client_port: (8000 + node_id) as u16,
                raft_port: (9000 + node_id) as u16,
                seed_peers: Vec::new(),
            },
            storage: StorageConfig {
                segment_size: 64 * 1024 * 1024,    // 64MB
                max_cache_size: 256 * 1024 * 1024, // 256MB
                s3: None,                          // No S3 by default
            },
            consensus: ConsensusConfig {
                election_timeout_min: 1500,
                election_timeout_max: 3000,
                heartbeat_interval_ms: 300,
                max_payload_entries: 300,
                snapshot_logs_since_last: 5000,
                max_in_snapshot_log_to_keep: 1000,
            },
            api: ApiConfig::default(),
            discovery: DiscoveryConfig::default(),
        }
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // Node config overrides
        if let Ok(id) = std::env::var("SCRIBE_NODE_ID") {
            if let Ok(parsed_id) = id.parse() {
                self.node.id = parsed_id;
            }
        }
        if let Ok(addr) = std::env::var("SCRIBE_NODE_ADDRESS") {
            self.node.address = addr;
        }
        if let Ok(dir) = std::env::var("SCRIBE_DATA_DIR") {
            self.node.data_dir = PathBuf::from(dir);
        }

        // Network config overrides
        if let Ok(addr) = std::env::var("SCRIBE_LISTEN_ADDR") {
            if let Ok(parsed_addr) = addr.parse() {
                self.network.listen_addr = parsed_addr;
            }
        }
        if let Ok(port) = std::env::var("SCRIBE_CLIENT_PORT") {
            if let Ok(parsed_port) = port.parse() {
                self.network.client_port = parsed_port;
            }
        }
        if let Ok(port) = std::env::var("SCRIBE_RAFT_PORT") {
            if let Ok(parsed_port) = port.parse() {
                self.network.raft_port = parsed_port;
            }
        }

        // Storage config overrides
        if let Ok(size) = std::env::var("SCRIBE_SEGMENT_SIZE") {
            if let Ok(parsed_size) = size.parse() {
                self.storage.segment_size = parsed_size;
            }
        }
        if let Ok(size) = std::env::var("SCRIBE_MAX_CACHE_SIZE") {
            if let Ok(parsed_size) = size.parse() {
                self.storage.max_cache_size = parsed_size;
            }
        }

        // Consensus config overrides
        if let Ok(timeout) = std::env::var("SCRIBE_ELECTION_TIMEOUT_MIN_MS") {
            if let Ok(parsed_timeout) = timeout.parse() {
                self.consensus.election_timeout_min = parsed_timeout;
            }
        }
        if let Ok(timeout) = std::env::var("SCRIBE_ELECTION_TIMEOUT_MAX_MS") {
            if let Ok(parsed_timeout) = timeout.parse() {
                self.consensus.election_timeout_max = parsed_timeout;
            }
        }
        if let Ok(interval) = std::env::var("SCRIBE_HEARTBEAT_INTERVAL_MS") {
            if let Ok(parsed_interval) = interval.parse() {
                self.consensus.heartbeat_interval_ms = parsed_interval;
            }
        }

        // Discovery config overrides
        if let Ok(port) = std::env::var("SCRIBE_DISCOVERY_PORT") {
            if let Ok(parsed_port) = port.parse() {
                self.discovery.discovery_port = parsed_port;
            }
        }
        if let Ok(addr) = std::env::var("SCRIBE_BROADCAST_ADDR") {
            self.discovery.broadcast_addr = addr;
        }
        if let Ok(secret) = std::env::var("SCRIBE_CLUSTER_SECRET") {
            self.discovery.cluster_secret = Some(secret);
        }
        if let Ok(interval) = std::env::var("SCRIBE_DISCOVERY_HEARTBEAT_MS") {
            if let Ok(parsed_interval) = interval.parse() {
                self.discovery.heartbeat_interval_ms = parsed_interval;
            }
        }
        if let Ok(timeout) = std::env::var("SCRIBE_DISCOVERY_FAILURE_TIMEOUT_MS") {
            if let Ok(parsed_timeout) = timeout.parse() {
                self.discovery.failure_timeout_ms = parsed_timeout;
            }
        }
    }

    /// Validate the configuration
    fn validate(&self) -> Result<()> {
        // Validate node config
        if self.node.id == 0 {
            return Err(ScribeError::Configuration(
                "Node ID must be greater than 0".to_string(),
            ));
        }
        if self.node.address.is_empty() {
            return Err(ScribeError::Configuration(
                "Node address cannot be empty".to_string(),
            ));
        }

        // Validate network config
        if self.network.client_port == 0 {
            return Err(ScribeError::Configuration(
                "Client port must be greater than 0".to_string(),
            ));
        }
        if self.network.raft_port == 0 {
            return Err(ScribeError::Configuration(
                "Raft port must be greater than 0".to_string(),
            ));
        }
        if self.network.client_port == self.network.raft_port {
            return Err(ScribeError::Configuration(
                "Client port and Raft port must be different".to_string(),
            ));
        }

        // Validate storage config
        if self.storage.segment_size == 0 {
            return Err(ScribeError::Configuration(
                "Segment size must be greater than 0".to_string(),
            ));
        }
        if self.storage.max_cache_size == 0 {
            return Err(ScribeError::Configuration(
                "Max cache size must be greater than 0".to_string(),
            ));
        }

        // Validate consensus config
        if self.consensus.election_timeout_min == 0 {
            return Err(ScribeError::Configuration(
                "Election timeout minimum must be greater than 0".to_string(),
            ));
        }
        if self.consensus.election_timeout_max == 0 {
            return Err(ScribeError::Configuration(
                "Election timeout maximum must be greater than 0".to_string(),
            ));
        }
        if self.consensus.election_timeout_min >= self.consensus.election_timeout_max {
            return Err(ScribeError::Configuration(
                "Election timeout minimum must be less than maximum".to_string(),
            ));
        }
        if self.consensus.heartbeat_interval_ms == 0 {
            return Err(ScribeError::Configuration(
                "Heartbeat interval must be greater than 0".to_string(),
            ));
        }
        if self.consensus.heartbeat_interval_ms >= self.consensus.election_timeout_min {
            return Err(ScribeError::Configuration(
                "Heartbeat interval must be less than election timeout minimum".to_string(),
            ));
        }

        Ok(())
    }

    /// Get election timeout minimum as Duration
    pub fn election_timeout_min(&self) -> Duration {
        Duration::from_millis(self.consensus.election_timeout_min)
    }

    /// Get election timeout maximum as Duration
    pub fn election_timeout_max(&self) -> Duration {
        Duration::from_millis(self.consensus.election_timeout_max)
    }

    /// Get heartbeat interval as Duration
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_millis(self.consensus.heartbeat_interval_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    // Test constants to avoid hardcoded values
    const TEST_NODE_ID: u64 = 1;
    const TEST_NODE_ID_2: u64 = 42;
    const TEST_CLIENT_PORT: u16 = 8001;
    const TEST_RAFT_PORT: u16 = 9001;
    const TEST_CLIENT_PORT_OVERRIDE: u16 = 7777;

    #[test]
    fn test_default_config() {
        let config = Config::default_for_node(TEST_NODE_ID);

        assert_eq!(config.node.id, TEST_NODE_ID);
        assert_eq!(config.network.client_port, TEST_CLIENT_PORT);
        assert_eq!(config.network.raft_port, TEST_RAFT_PORT);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_node_id() {
        let mut config = Config::default_for_node(TEST_NODE_ID);
        config.node.id = 0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_same_ports() {
        let mut config = Config::default_for_node(TEST_NODE_ID);
        config.network.raft_port = config.network.client_port;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_zero_segment_size() {
        let mut config = Config::default_for_node(TEST_NODE_ID);
        config.storage.segment_size = 0;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validation_heartbeat_timeout() {
        let mut config = Config::default_for_node(TEST_NODE_ID);
        config.consensus.heartbeat_interval_ms = 1000;
        config.consensus.election_timeout_min = 500;

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_duration_helpers() {
        let config = Config::default_for_node(TEST_NODE_ID);

        assert_eq!(config.election_timeout_min(), Duration::from_millis(1500));
        assert_eq!(config.election_timeout_max(), Duration::from_millis(3000));
        assert_eq!(config.heartbeat_interval(), Duration::from_millis(300));
    }

    #[test]
    fn test_env_override_node_id() {
        env::set_var("SCRIBE_NODE_ID", "42");

        let mut config = Config::default_for_node(TEST_NODE_ID);
        config.apply_env_overrides();

        assert_eq!(config.node.id, TEST_NODE_ID_2);

        env::remove_var("SCRIBE_NODE_ID");
    }

    #[test]
    fn test_env_override_client_port() {
        env::set_var("SCRIBE_CLIENT_PORT", "7777");

        let mut config = Config::default_for_node(TEST_NODE_ID);
        config.apply_env_overrides();

        assert_eq!(config.network.client_port, TEST_CLIENT_PORT_OVERRIDE);

        env::remove_var("SCRIBE_CLIENT_PORT");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default_for_node(TEST_NODE_ID);

        // Test TOML serialization
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(deserialized.node.id, config.node.id);
        assert_eq!(deserialized.network.client_port, config.network.client_port);
    }
}
