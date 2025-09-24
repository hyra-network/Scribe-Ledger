use serde::{Deserialize, Serialize};
use std::fs;
use crate::error::{Result, ScribeError};

/// Configuration for Scribe Ledger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Node configuration
    pub node: NodeConfig,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// Consensus configuration
    pub consensus: ConsensusConfig,
    
    /// Network configuration
    pub network: NetworkConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Unique node identifier
    pub id: String,
    
    /// Data directory for local storage
    pub data_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// S3/MinIO configuration
    pub s3: S3StorageConfig,
    
    /// Local buffer size before flushing to S3 (in bytes)
    pub buffer_size: usize,
    
    /// Segment size limit (in bytes)
    pub segment_size_limit: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct S3StorageConfig {
    /// S3 bucket name for durable storage
    pub bucket: String,
    
    /// S3 region
    pub region: String,
    
    /// S3 endpoint URL (optional, for MinIO compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    
    /// S3 access key (optional, falls back to IAM or env vars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key: Option<String>,
    
    /// S3 secret key (optional, falls back to IAM or env vars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_key: Option<String>,
    
    /// Use path-style addressing (required for MinIO)
    #[serde(default)]
    pub path_style: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Raft cluster peers
    pub peers: Vec<String>,
    
    /// Election timeout in milliseconds
    pub election_timeout_ms: u64,
    
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen address for the node
    pub listen_addr: String,
    
    /// Client API port
    pub client_port: u16,
    
    /// Consensus port for Raft communication
    pub consensus_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            node: NodeConfig {
                id: uuid::Uuid::new_v4().to_string(),
                data_dir: "./data".to_string(),
            },
            storage: StorageConfig {
                s3: S3StorageConfig {
                    bucket: "scribe-ledger".to_string(),
                    region: "us-east-1".to_string(),
                    endpoint: None,
                    access_key: None,
                    secret_key: None,
                    path_style: false,
                },
                buffer_size: 64 * 1024 * 1024, // 64MB
                segment_size_limit: 1024 * 1024 * 1024, // 1GB
            },
            consensus: ConsensusConfig {
                peers: vec![],
                election_timeout_ms: 5000,
                heartbeat_interval_ms: 1000,
            },
            network: NetworkConfig {
                listen_addr: "0.0.0.0".to_string(),
                client_port: 8080,
                consensus_port: 8081,
            },
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .map_err(|e| ScribeError::Config(format!("Failed to read config file: {}", e)))?;
        
        let mut config: Config = toml::from_str(&contents)
            .map_err(|e| ScribeError::Config(format!("Failed to parse config: {}", e)))?;
        
        // Override with environment variables (highest priority)
        config.apply_env_overrides();
        
        Ok(config)
    }
    
    /// Load configuration with environment variable overrides
    pub fn load_with_env() -> Self {
        let mut config = Self::default();
        config.apply_env_overrides();
        config
    }
    
    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // S3 Configuration overrides
        if let Ok(bucket) = std::env::var("SCRIBE_S3_BUCKET") {
            self.storage.s3.bucket = bucket;
        }
        if let Ok(region) = std::env::var("SCRIBE_S3_REGION") {
            self.storage.s3.region = region;
        }
        if let Ok(endpoint) = std::env::var("SCRIBE_S3_ENDPOINT") {
            self.storage.s3.endpoint = Some(endpoint);
        }
        if let Ok(access_key) = std::env::var("SCRIBE_S3_ACCESS_KEY") {
            self.storage.s3.access_key = Some(access_key);
        }
        if let Ok(secret_key) = std::env::var("SCRIBE_S3_SECRET_KEY") {
            self.storage.s3.secret_key = Some(secret_key);
        }
        if let Ok(path_style) = std::env::var("SCRIBE_S3_PATH_STYLE") {
            self.storage.s3.path_style = path_style.parse().unwrap_or(false);
        }
        
        // Node configuration overrides
        if let Ok(node_id) = std::env::var("SCRIBE_NODE_ID") {
            self.node.id = node_id;
        }
        if let Ok(data_dir) = std::env::var("SCRIBE_DATA_DIR") {
            self.node.data_dir = data_dir;
        }
        
        // Network configuration overrides
        if let Ok(listen_addr) = std::env::var("SCRIBE_LISTEN_ADDR") {
            self.network.listen_addr = listen_addr;
        }
        if let Ok(client_port) = std::env::var("SCRIBE_CLIENT_PORT") {
            if let Ok(port) = client_port.parse::<u16>() {
                self.network.client_port = port;
            }
        }
        if let Ok(consensus_port) = std::env::var("SCRIBE_CONSENSUS_PORT") {
            if let Ok(port) = consensus_port.parse::<u16>() {
                self.network.consensus_port = port;
            }
        }
    }
    
    /// Save configuration to a TOML file
    pub fn to_file(&self, path: &str) -> Result<()> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| ScribeError::Config(format!("Failed to serialize config: {}", e)))?;
        
        fs::write(path, contents)
            .map_err(|e| ScribeError::Config(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }
}