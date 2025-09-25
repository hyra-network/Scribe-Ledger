use thiserror::Error;

/// Main error type for Scribe Ledger
#[derive(Error, Debug)]
pub enum ScribeError {
    #[error("Storage error: {0}")]
    Storage(#[from] sled::Error),
    
    #[error("Consensus error: {0}")]
    Consensus(String),
    
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("AWS error: {0}")]
    Aws(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Invalid data: {0}")]
    InvalidData(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for Scribe Ledger operations
pub type Result<T> = std::result::Result<T, ScribeError>;