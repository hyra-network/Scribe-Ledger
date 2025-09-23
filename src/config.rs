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
    /// S3 bucket name for durable storage
    pub s3_bucket: String,
    
    /// S3 region
    pub s3_region: String,
    
    /// Local buffer size before flushing to S3 (in bytes)
    pub buffer_size: usize,
    
    /// Segment size limit (in bytes)
    pub segment_size_limit: usize,
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
                s3_bucket: "scribe-ledger".to_string(),
                s3_region: "us-east-1".to_string(),
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
        
        let config: Config = toml::from_str(&contents)
            .map_err(|e| ScribeError::Config(format!("Failed to parse config: {}", e)))?;
        
        Ok(config)
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