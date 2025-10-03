//! Error types for Simple Scribe Ledger
//!
//! This module defines all error types that can occur in the distributed ledger system.

use thiserror::Error;

/// Main error type for Scribe Ledger operations
#[derive(Error, Debug)]
pub enum ScribeError {
    /// Storage-related errors (e.g., sled database errors)
    #[error("Storage error: {0}")]
    Storage(#[from] sled::Error),

    /// Consensus/Raft-related errors
    #[error("Consensus error: {0}")]
    Consensus(String),

    /// Network communication errors
    #[error("Network error: {0}")]
    Network(String),

    /// Discovery service errors
    #[error("Discovery error: {0}")]
    Discovery(String),

    /// Configuration errors (parsing, validation)
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Manifest-related errors
    #[error("Manifest error: {0}")]
    Manifest(String),

    /// Cluster initialization and management errors
    #[error("Cluster error: {0}")]
    Cluster(String),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error for other cases
    #[error("{0}")]
    Other(String),
}

/// Implement From for serde_json errors
impl From<serde_json::Error> for ScribeError {
    fn from(err: serde_json::Error) -> Self {
        ScribeError::Serialization(err.to_string())
    }
}

/// Implement From for bincode errors
impl From<bincode::Error> for ScribeError {
    fn from(err: bincode::Error) -> Self {
        ScribeError::Serialization(err.to_string())
    }
}

/// Implement From for toml deserialization errors
impl From<toml::de::Error> for ScribeError {
    fn from(err: toml::de::Error) -> Self {
        ScribeError::Configuration(err.to_string())
    }
}

/// Type alias for Results using ScribeError
pub type Result<T> = std::result::Result<T, ScribeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ScribeError::Storage(sled::Error::Unsupported("test".to_string()));
        assert!(err.to_string().contains("Storage error"));

        let err = ScribeError::Consensus("test consensus error".to_string());
        assert!(err.to_string().contains("Consensus error"));

        let err = ScribeError::Network("test network error".to_string());
        assert!(err.to_string().contains("Network error"));

        let err = ScribeError::Configuration("test config error".to_string());
        assert!(err.to_string().contains("Configuration error"));

        let err = ScribeError::Serialization("test serialization error".to_string());
        assert!(err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_error_from_sled() {
        let sled_err = sled::Error::Unsupported("test".to_string());
        let scribe_err: ScribeError = sled_err.into();
        assert!(matches!(scribe_err, ScribeError::Storage(_)));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let scribe_err: ScribeError = io_err.into();
        assert!(matches!(scribe_err, ScribeError::Io(_)));
    }

    #[test]
    fn test_error_from_serde_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let scribe_err: ScribeError = json_err.into();
        assert!(matches!(scribe_err, ScribeError::Serialization(_)));
    }

    #[test]
    fn test_error_from_toml() {
        let toml_err = toml::from_str::<toml::Value>("invalid toml [[[").unwrap_err();
        let scribe_err: ScribeError = toml_err.into();
        assert!(matches!(scribe_err, ScribeError::Configuration(_)));
    }

    #[test]
    fn test_manifest_error() {
        let err = ScribeError::Manifest("test manifest error".to_string());
        assert!(err.to_string().contains("Manifest error"));
        assert!(err.to_string().contains("test manifest error"));
    }

    #[test]
    fn test_cluster_error() {
        let err = ScribeError::Cluster("test cluster error".to_string());
        assert!(err.to_string().contains("Cluster error"));
        assert!(err.to_string().contains("test cluster error"));
    }
}
