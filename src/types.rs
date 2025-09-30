//! Common types used throughout the Simple Scribe Ledger system
//!
//! This module defines type aliases and common data structures for the distributed ledger.

use serde::{Deserialize, Serialize};

/// Node identifier in the distributed system
pub type NodeId = u64;

/// Segment identifier for data partitioning
pub type SegmentId = u64;

/// Manifest identifier for tracking data organization
pub type ManifestId = u64;

/// Key type for storage operations
pub type Key = Vec<u8>;

/// Value type for storage operations
pub type Value = Vec<u8>;

/// Request types for client-server communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    /// Put a key-value pair
    Put { key: Key, value: Value },
    /// Get a value by key
    Get { key: Key },
    /// Delete a key
    Delete { key: Key },
}

/// Response types for client-server communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    /// Successful put operation
    PutOk,
    /// Successful get operation with optional value
    GetOk { value: Option<Value> },
    /// Successful delete operation
    DeleteOk,
    /// Error response
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let request = Request::Put {
            key: b"test_key".to_vec(),
            value: b"test_value".to_vec(),
        };

        // Test JSON serialization
        let json = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&json).unwrap();

        match deserialized {
            Request::Put { key, value } => {
                assert_eq!(key, b"test_key".to_vec());
                assert_eq!(value, b"test_value".to_vec());
            }
            _ => panic!("Expected Put request"),
        }
    }

    #[test]
    fn test_response_serialization() {
        let response = Response::GetOk {
            value: Some(b"test_value".to_vec()),
        };

        // Test JSON serialization
        let json = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&json).unwrap();

        match deserialized {
            Response::GetOk { value } => {
                assert_eq!(value, Some(b"test_value".to_vec()));
            }
            _ => panic!("Expected GetOk response"),
        }
    }

    #[test]
    fn test_get_request() {
        let request = Request::Get {
            key: b"key".to_vec(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&json).unwrap();

        match deserialized {
            Request::Get { key } => {
                assert_eq!(key, b"key".to_vec());
            }
            _ => panic!("Expected Get request"),
        }
    }

    #[test]
    fn test_delete_request() {
        let request = Request::Delete {
            key: b"key".to_vec(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&json).unwrap();

        match deserialized {
            Request::Delete { key } => {
                assert_eq!(key, b"key".to_vec());
            }
            _ => panic!("Expected Delete request"),
        }
    }

    #[test]
    fn test_error_response() {
        let response = Response::Error {
            message: "Test error".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: Response = serde_json::from_str(&json).unwrap();

        match deserialized {
            Response::Error { message } => {
                assert_eq!(message, "Test error");
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[test]
    fn test_type_aliases() {
        // Just verify the type aliases can be used
        let _node_id: NodeId = 1;
        let _segment_id: SegmentId = 100;
        let _manifest_id: ManifestId = 1000;
        let _key: Key = vec![1, 2, 3];
        let _value: Value = vec![4, 5, 6];
    }
}
