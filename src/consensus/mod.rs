/// Raft consensus implementation for Scribe Ledger
use raft::{prelude::*, storage::MemStorage, Config as RaftConfig, RawNode, StateRole};
use slog::{Drain, Logger};
use crate::error::{Result, ScribeError};
use crate::manifest::{ManifestManager, ClusterManifest, ClusterNode};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;


/// Messages exchanged between cluster nodes
#[derive(Debug, Clone)]
pub enum ClusterMessage {
    RaftMessage(Message),
    ManifestUpdate(ClusterManifest),
    NodeJoin(ClusterNode),
    NodeLeave(u64),
}

/// Network transport trait for cluster communication
#[async_trait::async_trait]
pub trait NetworkTransport: Send + Sync {
    async fn send_message(&self, address: &str, message: ClusterMessage) -> Result<()>;
    async fn broadcast_message(&self, addresses: &[String], message: ClusterMessage) -> Result<()>;
}

/// HTTP-based transport implementation
pub struct HttpTransport {
    client: reqwest::Client,
}

impl HttpTransport {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl NetworkTransport for HttpTransport {
    async fn send_message(&self, address: &str, _message: ClusterMessage) -> Result<()> {
        // For now, just log the message send (actual HTTP transport would be implemented later)
        tracing::debug!("Would send message to {}", address);
        Ok(())
    }
    
    async fn broadcast_message(&self, addresses: &[String], message: ClusterMessage) -> Result<()> {
        // Broadcast to all addresses
        for addr in addresses {
            self.send_message(addr, message.clone()).await?;
        }
        Ok(())
    }
}

/// Consensus node managing distributed coordination
pub struct ConsensusNode {
    raft_node: RawNode<MemStorage>,
    manifest_manager: ManifestManager,
    node_id: u64,
    transport: Arc<dyn NetworkTransport>,
    address: String,
    peer_addresses: Arc<RwLock<HashMap<u64, String>>>,
}

impl ConsensusNode {
    /// Create a new consensus node
    pub fn new(
        node_id: u64,
        address: String,
        transport: Arc<dyn NetworkTransport>
    ) -> Result<Self> {
        let config = RaftConfig {
            id: node_id,
            election_tick: 10,
            heartbeat_tick: 3,
            max_size_per_msg: 1024 * 1024, // 1MB
            max_inflight_msgs: 256,
            ..Default::default()
        };
        
        let storage = MemStorage::new();
        
        // Create logger for raft
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = Logger::root(drain, slog::o!());
        
        let raft_node = RawNode::new(&config, storage, &logger)
            .map_err(|e| ScribeError::Consensus(format!("Failed to create Raft node: {}", e)))?;
        
        let manifest_manager = ManifestManager::new(node_id);
        
        Ok(Self {
            raft_node,
            manifest_manager,
            node_id,
            transport,
            address,
            peer_addresses: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Initialize cluster with initial nodes
    pub async fn initialize_cluster(&mut self, initial_nodes: Vec<ClusterNode>) -> Result<()> {
        // Add initial nodes to manifest and peer addresses
        let mut addresses = self.peer_addresses.write().await;
        for node in initial_nodes {
            addresses.insert(node.id, node.address.clone());
            self.manifest_manager.add_node(node)?;
        }
        drop(addresses);
        
        // Note: In a real deployment, configuration changes would be proposed 
        // after the cluster has established a leader. For testing, we just 
        // store the peer information locally.
        tracing::info!("Cluster initialized with {} nodes", self.peer_addresses.read().await.len());
        
        Ok(())
    }
    
    /// Add a peer to the cluster
    pub async fn add_peer(&mut self, node: ClusterNode) -> Result<()> {
        // Update peer addresses  
        self.peer_addresses.write().await.insert(node.id, node.address.clone());
        
        // Add to manifest
        self.manifest_manager.add_node(node.clone())?;
        
        tracing::info!("Added peer {} at {}", node.id, node.address);
        
        // Note: Configuration changes would be proposed when there's a leader
        // For now, just store locally for testing
        Ok(())
    }
    
    /// Remove a peer from the cluster
    pub async fn remove_peer(&mut self, node_id: u64) -> Result<()> {
        // Remove from peer addresses
        self.peer_addresses.write().await.remove(&node_id);
        
        // Remove from manifest
        self.manifest_manager.remove_node(node_id)?;
        
        tracing::info!("Removed peer {}", node_id);
        
        // Note: Configuration changes would be proposed when there's a leader
        Ok(())
    }
    
    /// Process a single Raft tick
    pub fn tick(&mut self) {
        self.raft_node.tick();
    }
    
    /// Handle incoming cluster message
    pub async fn handle_message(&mut self, message: ClusterMessage) -> Result<()> {
        match message {
            ClusterMessage::RaftMessage(msg) => {
                self.raft_node.step(msg)
                    .map_err(|e| ScribeError::Consensus(format!("Failed to process Raft message: {}", e)))?;
            },
            ClusterMessage::ManifestUpdate(_manifest) => {
                // Manifest updates will be handled via Raft consensus
                tracing::debug!("Received manifest update message");
            },
            ClusterMessage::NodeJoin(node) => {
                self.add_peer(node).await?;
            },
            ClusterMessage::NodeLeave(node_id) => {
                self.remove_peer(node_id).await?;
            },
        }
        Ok(())
    }
    
    /// Process ready state and send messages
    pub async fn process_ready(&mut self) -> Result<()> {
        if !self.raft_node.has_ready() {
            return Ok(());
        }
        
        let mut ready = self.raft_node.ready();
        
        // Send messages to peers
        let peer_addresses = self.peer_addresses.read().await;
        for msg in ready.messages() {
            if let Some(addr) = peer_addresses.get(&msg.to) {
                let cluster_msg = ClusterMessage::RaftMessage(msg.clone());
                if let Err(e) = self.transport.send_message(addr, cluster_msg).await {
                    tracing::warn!("Failed to send message to {}: {}", msg.to, e);
                }
            }
        }
        drop(peer_addresses);
        
        // Apply committed entries
        for entry in ready.committed_entries() {
            if entry.data.is_empty() {
                // Configuration change entry
                continue;
            }
            
            // Deserialize and apply manifest operations
            if let Ok(manifest) = serde_json::from_slice::<ClusterManifest>(&entry.data) {
                // For now, just log the manifest update
                // In a full implementation, we'd update our local manifest
                tracing::info!("Received manifest update from committed entry");
            }
        }
        
        // Advance Raft state
        let light_rd = self.raft_node.advance(ready);
        
        // Send any additional messages
        let peer_addresses = self.peer_addresses.read().await;
        for msg in light_rd.messages() {
            if let Some(addr) = peer_addresses.get(&msg.to) {
                let cluster_msg = ClusterMessage::RaftMessage(msg.clone());
                if let Err(e) = self.transport.send_message(addr, cluster_msg).await {
                    tracing::warn!("Failed to send light message to {}: {}", msg.to, e);
                }
            }
        }
        drop(peer_addresses);
        
        self.raft_node.advance_apply();
        
        Ok(())
    }
    
    /// Propose a manifest update to the cluster
    pub async fn propose_manifest_update(&mut self, manifest: ClusterManifest) -> Result<()> {
        let data = serde_json::to_vec(&manifest)
            .map_err(|e| ScribeError::Consensus(format!("Failed to serialize manifest: {}", e)))?;
            
        self.raft_node.propose(vec![], data)
            .map_err(|e| ScribeError::Consensus(format!("Failed to propose manifest update: {}", e)))?;
        
        Ok(())
    }
    
    /// Check if this node is the cluster leader
    pub fn is_leader(&self) -> bool {
        self.raft_node.raft.state == StateRole::Leader
    }
    
    /// Get current cluster manifest
    pub fn get_manifest_manager(&self) -> &ManifestManager {
        &self.manifest_manager
    }
    
    /// Get node ID
    pub fn node_id(&self) -> u64 {
        self.node_id
    }
    
    /// Get node address
    pub fn address(&self) -> &str {
        &self.address
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::NodeStatus;
    use std::sync::Arc;
    
    struct MockTransport;
    
    #[async_trait::async_trait]
    impl NetworkTransport for MockTransport {
        async fn send_message(&self, _address: &str, _message: ClusterMessage) -> Result<()> {
            Ok(())
        }
        
        async fn broadcast_message(&self, _addresses: &[String], _message: ClusterMessage) -> Result<()> {
            Ok(())
        }
    }
    
    #[tokio::test]
    async fn test_consensus_node_creation() {
        let transport = Arc::new(MockTransport);
        let node = ConsensusNode::new(1, "127.0.0.1:8001".to_string(), transport);
        assert!(node.is_ok());
        
        let node = node.unwrap();
        assert_eq!(node.node_id(), 1);
        assert_eq!(node.address(), "127.0.0.1:8001");
        assert!(!node.is_leader()); // Initially not leader
    }
    
    #[tokio::test]
    async fn test_cluster_initialization() {
        let transport = Arc::new(MockTransport);
        let mut node = ConsensusNode::new(1, "127.0.0.1:8001".to_string(), transport).unwrap();
        
        let initial_nodes = vec![
            ClusterNode {
                id: 1,
                address: "127.0.0.1:8001".to_string(),
                status: NodeStatus::Active,
                last_seen: 1234567890,
            },
            ClusterNode {
                id: 2,
                address: "127.0.0.1:8002".to_string(),
                status: NodeStatus::Active,
                last_seen: 1234567890,
            },
        ];
        
        let result = node.initialize_cluster(initial_nodes).await;
        if let Err(e) = &result {
            println!("Error initializing cluster: {:?}", e);
        }
        assert!(result.is_ok());
        
        // Check that peer addresses were stored
        let addresses = node.peer_addresses.read().await;
        assert_eq!(addresses.len(), 2);
    }
    
    #[tokio::test]
    async fn test_peer_management() {
        let transport = Arc::new(MockTransport);
        let mut node = ConsensusNode::new(1, "127.0.0.1:8001".to_string(), transport).unwrap();
        
        let new_peer = ClusterNode {
            id: 3,
            address: "127.0.0.1:8003".to_string(),
            status: NodeStatus::Active,
            last_seen: 1234567890,
        };
        
        // Add peer
        let result = node.add_peer(new_peer.clone()).await;
        assert!(result.is_ok());
        
        // Check peer is in addresses
        let addresses = node.peer_addresses.read().await;
        assert!(addresses.contains_key(&3));
        assert_eq!(addresses.get(&3).unwrap(), "127.0.0.1:8003");
        drop(addresses);
        
        // Remove peer
        let result = node.remove_peer(3).await;
        assert!(result.is_ok());
        
        let addresses = node.peer_addresses.read().await;
        assert!(!addresses.contains_key(&3));
    }
}