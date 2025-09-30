//! Type configuration for OpenRaft
//!
//! This module defines the concrete types used by OpenRaft in this application.

use openraft::raft::responder::OneshotResponder;
use openraft::{BasicNode, Entry, TokioRuntime};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

use crate::types::{Key, NodeId, Value};

/// Client request type for log entries
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AppRequest {
    /// Put a key-value pair
    Put { key: Key, value: Value },
    /// Delete a key
    Delete { key: Key },
}

/// Client response type for operations
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AppResponse {
    /// Successful put operation
    PutOk,
    /// Successful delete operation
    DeleteOk,
    /// Error response
    Error { message: String },
}

/// Type configuration for OpenRaft
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeConfig {}

impl openraft::RaftTypeConfig for TypeConfig {
    /// Application data type (log entry payload)
    type D = AppRequest;

    /// Application response type
    type R = AppResponse;

    /// Node identifier type
    type NodeId = NodeId;

    /// Node information type
    type Node = BasicNode;

    /// Log entry type
    type Entry = Entry<TypeConfig>;

    /// Snapshot data type (serialized as bytes)
    type SnapshotData = Cursor<Vec<u8>>;

    /// Async runtime (Tokio)
    type AsyncRuntime = TokioRuntime;

    /// Responder type for client write responses
    type Responder = OneshotResponder<Self>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_request_serialization() {
        let request = AppRequest::Put {
            key: b"test_key".to_vec(),
            value: b"test_value".to_vec(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: AppRequest = serde_json::from_str(&json).unwrap();

        match deserialized {
            AppRequest::Put { key, value } => {
                assert_eq!(key, b"test_key".to_vec());
                assert_eq!(value, b"test_value".to_vec());
            }
            _ => panic!("Expected Put request"),
        }
    }

    #[test]
    fn test_app_request_delete() {
        let request = AppRequest::Delete {
            key: b"key".to_vec(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: AppRequest = serde_json::from_str(&json).unwrap();

        match deserialized {
            AppRequest::Delete { key } => {
                assert_eq!(key, b"key".to_vec());
            }
            _ => panic!("Expected Delete request"),
        }
    }

    #[test]
    fn test_app_response_serialization() {
        let response = AppResponse::PutOk;

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: AppResponse = serde_json::from_str(&json).unwrap();

        match deserialized {
            AppResponse::PutOk => {}
            _ => panic!("Expected PutOk response"),
        }
    }

    #[test]
    fn test_app_response_error() {
        let response = AppResponse::Error {
            message: "Test error".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: AppResponse = serde_json::from_str(&json).unwrap();

        match deserialized {
            AppResponse::Error { message } => {
                assert_eq!(message, "Test error");
            }
            _ => panic!("Expected Error response"),
        }
    }
}
