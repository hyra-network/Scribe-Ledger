//! Consensus module for distributed consensus using OpenRaft
//!
//! This module contains the Raft consensus implementation for the distributed ledger.

// Allow io_other_error clippy lint as this is a standard pattern
#![allow(clippy::io_other_error)]

pub mod network;
pub mod state_machine;
pub mod storage;
pub mod type_config;

pub use network::{Network, NetworkFactory};
pub use state_machine::{SnapshotBuilder, StateMachine, StateMachineStore};
pub use storage::{LogReader, RaftStorage};
pub use type_config::{AppRequest, AppResponse, TypeConfig};

use openraft::{BasicNode, Config, Raft};
use std::collections::BTreeSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::NodeId;

/// Type alias for the Raft instance
pub type RaftInstance = Raft<TypeConfig>;

/// Consensus node that integrates OpenRaft with storage, state machine, and network
pub struct ConsensusNode {
    /// The Raft instance
    raft: Arc<RaftInstance>,
    /// Network factory for creating network clients
    network_factory: Arc<RwLock<NetworkFactory>>,
    /// Node ID
    node_id: NodeId,
}

impl ConsensusNode {
    /// Create a new consensus node
    pub async fn new(
        node_id: NodeId,
        db: sled::Db,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create storage
        let storage = RaftStorage::new(db);
        
        // Create separate state machine instance (not from storage)
        let state_machine = StateMachineStore::new();

        // Create network factory
        let network_factory = NetworkFactory::new(node_id);

        // Create Raft configuration
        let config = Config {
            heartbeat_interval: 500,
            election_timeout_min: 1500,
            election_timeout_max: 3000,
            enable_tick: true,
            enable_heartbeat: true,
            ..Default::default()
        };

        // Create Raft instance with separate log store and state machine
        let raft = Raft::new(
            node_id,
            Arc::new(config),
            network_factory.clone(),
            storage,
            state_machine,
        )
        .await
        .map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create Raft instance: {:?}", e),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

        Ok(Self {
            raft: Arc::new(raft),
            network_factory: Arc::new(RwLock::new(network_factory)),
            node_id,
        })
    }

    /// Get the Raft instance
    pub fn raft(&self) -> Arc<RaftInstance> {
        Arc::clone(&self.raft)
    }

    /// Get the node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Register a peer node with its network address
    pub async fn register_peer(&self, node_id: NodeId, address: String) {
        let network_factory = self.network_factory.write().await;
        network_factory.register_node(node_id, address).await;
    }

    /// Initialize the cluster (single-node cluster)
    pub async fn initialize(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut nodes = BTreeSet::new();
        nodes.insert(self.node_id);

        self.raft
            .initialize(nodes)
            .await
            .map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to initialize cluster: {:?}", e),
                )) as Box<dyn std::error::Error + Send + Sync>
            })?;

        Ok(())
    }

    /// Add a learner to the cluster
    pub async fn add_learner(
        &self,
        node_id: NodeId,
        node: BasicNode,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.raft
            .add_learner(node_id, node, true)
            .await
            .map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to add learner: {:?}", e),
                )) as Box<dyn std::error::Error + Send + Sync>
            })?;

        Ok(())
    }

    /// Change membership of the cluster
    pub async fn change_membership(
        &self,
        members: BTreeSet<NodeId>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.raft
            .change_membership(members, false)
            .await
            .map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to change membership: {:?}", e),
                )) as Box<dyn std::error::Error + Send + Sync>
            })?;

        Ok(())
    }

    /// Check if this node is the leader
    pub async fn is_leader(&self) -> bool {
        self.raft
            .with_raft_state(|state| state.server_state.is_leader())
            .await
            .unwrap_or(false)
    }

    /// Get current leader ID
    pub async fn current_leader(&self) -> Option<NodeId> {
        self.raft.current_leader().await
    }

    /// Perform a health check
    pub async fn health_check(&self) -> HealthStatus {
        let is_leader = self.is_leader().await;
        let current_leader = self.current_leader().await;
        
        let metrics = self.raft.metrics().borrow().clone();
        
        HealthStatus {
            node_id: self.node_id,
            is_leader,
            current_leader,
            state: format!("{:?}", metrics.state),
            last_log_index: metrics.last_log_index,
            last_applied: metrics.last_applied,
            current_term: metrics.current_term,
        }
    }

    /// Graceful shutdown of the consensus node
    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.raft.shutdown().await.map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to shutdown Raft: {:?}", e),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

        Ok(())
    }

    /// Client write operation
    pub async fn client_write(
        &self,
        request: AppRequest,
    ) -> Result<AppResponse, Box<dyn std::error::Error + Send + Sync>> {
        self.raft
            .client_write(request)
            .await
            .map(|r| r.data)
            .map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Client write error: {:?}", e),
                )) as Box<dyn std::error::Error + Send + Sync>
            })
    }

    /// Get metrics from the Raft instance
    pub async fn metrics(&self) -> openraft::RaftMetrics<NodeId, BasicNode> {
        self.raft.metrics().borrow().clone()
    }
}

/// Health status information for a consensus node
#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Node ID
    pub node_id: NodeId,
    /// Whether this node is the leader
    pub is_leader: bool,
    /// Current leader ID (if known)
    pub current_leader: Option<NodeId>,
    /// Current Raft state
    pub state: String,
    /// Last log index
    pub last_log_index: Option<u64>,
    /// Last applied log index
    pub last_applied: Option<openraft::LogId<NodeId>>,
    /// Current term
    pub current_term: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consensus_node_creation() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let node = ConsensusNode::new(1, db).await.unwrap();
        assert_eq!(node.node_id(), 1);
    }

    #[tokio::test]
    async fn test_register_peer() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let node = ConsensusNode::new(1, db).await.unwrap();
        
        node.register_peer(2, "127.0.0.1:5002".to_string()).await;
        
        // Can't directly access node_addresses from outside, so just verify it doesn't error
    }

    #[tokio::test]
    async fn test_initialize_single_node_cluster() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let node = ConsensusNode::new(1, db).await.unwrap();
        
        // Initialize as single-node cluster
        node.initialize().await.unwrap();
        
        // Wait a bit for election
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
        
        // Should be leader
        assert!(node.is_leader().await);
    }

    #[tokio::test]
    async fn test_health_check() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let node = ConsensusNode::new(1, db).await.unwrap();
        
        let health = node.health_check().await;
        assert_eq!(health.node_id, 1);
    }

    #[tokio::test]
    async fn test_metrics() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let node = ConsensusNode::new(1, db).await.unwrap();
        
        let metrics = node.metrics().await;
        assert_eq!(metrics.id, 1);
    }

    #[tokio::test]
    async fn test_client_write_before_init() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let node = ConsensusNode::new(1, db).await.unwrap();
        
        let request = AppRequest::Put {
            key: b"test_key".to_vec(),
            value: b"test_value".to_vec(),
        };
        
        // Writing before initialization should fail
        let result = node.client_write(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_current_leader() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let node = ConsensusNode::new(1, db).await.unwrap();
        
        // Before initialization, there should be no leader
        let leader = node.current_leader().await;
        assert_eq!(leader, None);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let db = sled::Config::new().temporary(true).open().unwrap();
        let node = ConsensusNode::new(1, db).await.unwrap();
        
        // Shutdown should succeed
        node.shutdown().await.unwrap();
    }
}
