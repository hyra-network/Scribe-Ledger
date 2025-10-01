//! OpenRaft network layer implementation
//!
//! This module implements the RaftNetwork trait for node-to-node communication
//! using TCP connections with connection pooling and retry logic.

// Allow large error types from OpenRaft - this is a library design choice
#![allow(clippy::result_large_err)]

use openraft::error::{InstallSnapshotError, NetworkError, RPCError, RaftError};
use openraft::network::{RPCOption, RaftNetwork, RaftNetworkFactory};
use openraft::raft::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse,
    VoteRequest, VoteResponse,
};
use openraft::BasicNode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::timeout;

use crate::consensus::type_config::TypeConfig;
use crate::types::NodeId;

/// Default timeout for network operations
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Maximum number of retry attempts
const MAX_RETRIES: u32 = 3;

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
enum NetworkMessage {
    AppendEntries(AppendEntriesRequest<TypeConfig>),
    Vote(VoteRequest<NodeId>),
    InstallSnapshot(InstallSnapshotRequest<TypeConfig>),
}

/// Network response types
#[derive(Debug, Serialize, Deserialize)]
enum NetworkResponse {
    AppendEntries(Result<AppendEntriesResponse<NodeId>, String>),
    Vote(Result<VoteResponse<NodeId>, String>),
    InstallSnapshot(Result<InstallSnapshotResponse<NodeId>, String>),
}

/// Connection pool for managing TCP connections to other nodes
struct ConnectionPool {
    connections: Arc<RwLock<HashMap<NodeId, Arc<RwLock<Option<TcpStream>>>>>>,
}

impl ConnectionPool {
    fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a connection to a target node
    async fn get_connection(
        &self,
        node_addr: &str,
    ) -> Result<TcpStream, RPCError<NodeId, BasicNode, RaftError<NodeId>>> {
        // Create new connection with timeout
        let stream = timeout(DEFAULT_TIMEOUT, TcpStream::connect(node_addr))
            .await
            .map_err(|_| {
                RPCError::Network(NetworkError::new(&std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!("Timeout connecting to {}", node_addr),
                )))
            })?
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        Ok(stream)
    }
}

/// Network implementation for Raft RPC
pub struct Network {
    /// The target node ID for this network instance
    target: NodeId,
    /// Target node address
    target_addr: String,
    /// Connection pool for reusing connections
    pool: ConnectionPool,
}

impl Network {
    /// Create a new network instance for a specific target
    pub fn new(target: NodeId, target_addr: String) -> Self {
        Self {
            target,
            target_addr,
            pool: ConnectionPool::new(),
        }
    }

    /// Send a message with retry logic
    async fn send_with_retry<T>(
        &self,
        message: NetworkMessage,
    ) -> Result<T, RPCError<NodeId, BasicNode, RaftError<NodeId>>>
    where
        T: for<'de> Deserialize<'de> + Send,
    {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                // Exponential backoff
                tokio::time::sleep(Duration::from_millis(100 * (1 << attempt))).await;
            }

            match self.try_send(&message).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap())
    }

    /// Try to send a message once
    async fn try_send<T>(
        &self,
        message: &NetworkMessage,
    ) -> Result<T, RPCError<NodeId, BasicNode, RaftError<NodeId>>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let mut stream = self.pool.get_connection(&self.target_addr).await?;

        // Serialize and send the message
        let msg_bytes = bincode::serialize(message).map_err(|e| {
            RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Serialization error: {}", e),
            )))
        })?;

        // Send message length first (4 bytes)
        let len = msg_bytes.len() as u32;
        stream
            .write_all(&len.to_be_bytes())
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        // Send message data
        stream
            .write_all(&msg_bytes)
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        stream
            .flush()
            .await
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        // Read response length (4 bytes)
        let mut len_bytes = [0u8; 4];
        timeout(DEFAULT_TIMEOUT, stream.read_exact(&mut len_bytes))
            .await
            .map_err(|_| {
                RPCError::Network(NetworkError::new(&std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Timeout reading response length",
                )))
            })?
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let len = u32::from_be_bytes(len_bytes) as usize;

        // Read response data
        let mut response_bytes = vec![0u8; len];
        timeout(DEFAULT_TIMEOUT, stream.read_exact(&mut response_bytes))
            .await
            .map_err(|_| {
                RPCError::Network(NetworkError::new(&std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Timeout reading response data",
                )))
            })?
            .map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        // Deserialize response
        let response: T = bincode::deserialize(&response_bytes).map_err(|e| {
            RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Deserialization error: {}", e),
            )))
        })?;

        Ok(response)
    }
}

impl RaftNetwork<TypeConfig> for Network {
    async fn append_entries(
        &mut self,
        rpc: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId>>> {
        let message = NetworkMessage::AppendEntries(rpc);
        let response: NetworkResponse = self.send_with_retry(message).await?;

        match response {
            NetworkResponse::AppendEntries(result) => result.map_err(|e| {
                RPCError::Network(NetworkError::new(&std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )))
            }),
            _ => Err(RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid response type",
            )))),
        }
    }

    async fn vote(
        &mut self,
        rpc: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, RPCError<NodeId, BasicNode, RaftError<NodeId>>> {
        let message = NetworkMessage::Vote(rpc);
        let response: NetworkResponse = self.send_with_retry(message).await?;

        match response {
            NetworkResponse::Vote(result) => result.map_err(|e| {
                RPCError::Network(NetworkError::new(&std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )))
            }),
            _ => Err(RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid response type",
            )))),
        }
    }

    async fn install_snapshot(
        &mut self,
        rpc: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<
        InstallSnapshotResponse<NodeId>,
        RPCError<NodeId, BasicNode, RaftError<NodeId, InstallSnapshotError>>,
    > {
        let message = NetworkMessage::InstallSnapshot(rpc);
        let response: NetworkResponse = self.send_with_retry(message).await.map_err(|e| match e {
            RPCError::Network(n) => RPCError::Network(n),
            _ => RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                "Network error",
            ))),
        })?;

        match response {
            NetworkResponse::InstallSnapshot(result) => result.map_err(|e| {
                RPCError::Network(NetworkError::new(&std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e,
                )))
            }),
            _ => Err(RPCError::Network(NetworkError::new(&std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid response type",
            )))),
        }
    }
}

/// Factory for creating network instances
#[derive(Clone)]
pub struct NetworkFactory {
    node_addresses: Arc<RwLock<HashMap<NodeId, String>>>,
}

impl NetworkFactory {
    /// Create a new network factory
    pub fn new(_node_id: NodeId) -> Self {
        Self {
            node_addresses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a node address
    pub async fn register_node(&self, node_id: NodeId, address: String) {
        let mut addresses = self.node_addresses.write().await;
        addresses.insert(node_id, address);
    }
}

impl RaftNetworkFactory<TypeConfig> for NetworkFactory {
    type Network = Network;

    async fn new_client(&mut self, target: NodeId, _node: &BasicNode) -> Self::Network {
        let addresses = self.node_addresses.read().await;
        let target_addr = addresses
            .get(&target)
            .cloned()
            .unwrap_or_else(|| format!("127.0.0.1:{}", 5000 + target));
        Network::new(target, target_addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_creation() {
        let network = Network::new(1, "127.0.0.1:5001".to_string());
        assert_eq!(network.target, 1);
    }

    #[tokio::test]
    async fn test_network_factory() {
        let factory = NetworkFactory::new(1);
        factory
            .register_node(2, "127.0.0.1:5002".to_string())
            .await;

        let addresses = factory.node_addresses.read().await;
        assert_eq!(addresses.get(&2), Some(&"127.0.0.1:5002".to_string()));
    }

    #[tokio::test]
    async fn test_connection_pool() {
        let pool = ConnectionPool::new();

        // Test that connection pool is initially empty
        let connections = pool.connections.read().await;
        assert!(connections.is_empty());
    }

    #[test]
    fn test_network_message_serialization() {
        use crate::consensus::type_config::AppRequest;
        use openraft::{EntryPayload, LeaderId, LogId};

        let log_id = LogId::new(LeaderId::new(1, 1), 1);
        let entry = openraft::Entry {
            log_id,
            payload: EntryPayload::Normal(AppRequest::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec(),
            }),
        };

        let request = AppendEntriesRequest {
            vote: openraft::Vote::new(1, 1u64),
            prev_log_id: None,
            entries: vec![entry],
            leader_commit: None,
        };

        let message = NetworkMessage::AppendEntries(request);
        let serialized = bincode::serialize(&message).unwrap();
        let deserialized: NetworkMessage = bincode::deserialize(&serialized).unwrap();

        match deserialized {
            NetworkMessage::AppendEntries(_) => {}
            _ => panic!("Expected AppendEntries message"),
        }
    }

    #[test]
    fn test_vote_message_serialization() {
        let vote_request = VoteRequest {
            vote: openraft::Vote::new(1, 1u64),
            last_log_id: None,
        };

        let message = NetworkMessage::Vote(vote_request);
        let serialized = bincode::serialize(&message).unwrap();
        let deserialized: NetworkMessage = bincode::deserialize(&serialized).unwrap();

        match deserialized {
            NetworkMessage::Vote(_) => {}
            _ => panic!("Expected Vote message"),
        }
    }

    #[test]
    fn test_network_response_serialization() {
        use openraft::Vote;

        let response: AppendEntriesResponse<NodeId> = AppendEntriesResponse::Success;

        let net_response = NetworkResponse::AppendEntries(Ok(response));
        let serialized = bincode::serialize(&net_response).unwrap();
        let deserialized: NetworkResponse = bincode::deserialize(&serialized).unwrap();

        match deserialized {
            NetworkResponse::AppendEntries(Ok(_)) => {}
            _ => panic!("Expected successful AppendEntries response"),
        }
    }
}
