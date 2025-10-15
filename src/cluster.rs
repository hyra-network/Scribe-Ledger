//! Cluster initialization and management
//!
//! This module provides automatic cluster formation by coordinating node discovery
//! with Raft consensus cluster initialization.

use crate::consensus::ConsensusNode;
use crate::discovery::{DiscoveryService, PeerInfo};
use crate::error::{Result, ScribeError};
use crate::types::NodeId;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info, warn};

/// Cluster initialization mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitMode {
    /// Bootstrap a new cluster (first node)
    Bootstrap,
    /// Join an existing cluster
    Join,
}

/// Configuration for cluster initialization
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Initialization mode
    pub mode: InitMode,
    /// Seed addresses for discovery (optional, for manual seeding)
    pub seed_addrs: Vec<String>,
    /// Timeout for waiting for peers to be discovered (milliseconds)
    pub discovery_timeout_ms: u64,
    /// Minimum number of peers required before joining
    pub min_peers_for_join: usize,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            mode: InitMode::Join,
            seed_addrs: Vec::new(),
            discovery_timeout_ms: 5000,
            min_peers_for_join: 1,
        }
    }
}

/// Cluster initializer that coordinates discovery and consensus
pub struct ClusterInitializer {
    /// Discovery service
    discovery: Arc<DiscoveryService>,
    /// Consensus node
    consensus: Arc<ConsensusNode>,
    /// Configuration
    config: ClusterConfig,
    /// Node ID
    node_id: NodeId,
}

impl ClusterInitializer {
    /// Create a new cluster initializer
    pub fn new(
        discovery: Arc<DiscoveryService>,
        consensus: Arc<ConsensusNode>,
        config: ClusterConfig,
    ) -> Self {
        let node_id = consensus.node_id();
        Self {
            discovery,
            consensus,
            config,
            node_id,
        }
    }

    /// Initialize the cluster based on configuration
    pub async fn initialize(&self) -> Result<()> {
        match self.config.mode {
            InitMode::Bootstrap => self.bootstrap().await,
            InitMode::Join => self.join_cluster().await,
        }
    }

    /// Bootstrap a new cluster (single-node initialization)
    async fn bootstrap(&self) -> Result<()> {
        info!("Bootstrapping new cluster with node {}", self.node_id);

        // Initialize consensus as single-node cluster
        self.consensus
            .initialize()
            .await
            .map_err(|e| ScribeError::Consensus(format!("Failed to bootstrap cluster: {}", e)))?;

        info!(
            "Successfully bootstrapped cluster with node {}",
            self.node_id
        );
        Ok(())
    }

    /// Join an existing cluster
    async fn join_cluster(&self) -> Result<()> {
        info!(
            "Attempting to join existing cluster for node {}",
            self.node_id
        );

        // Wait for peer discovery
        let peers = self.wait_for_peers().await?;

        if peers.is_empty() {
            warn!("No peers discovered");
            // Don't try to bootstrap if we already have state, just continue as standalone
            info!("Node {} will continue as standalone (existing state preserved)", self.node_id);
            return Ok(());
        }

        // Find the leader
        let leader_info = self.discover_leader(&peers).await?;

        info!(
            "Discovered leader node {} at {}",
            leader_info.node_id, leader_info.raft_addr
        );

        // Register peers with consensus layer
        for peer in &peers {
            info!(
                "Registering peer node {} at {}",
                peer.node_id, peer.raft_addr
            );
            self.consensus
                .register_peer(peer.node_id, peer.raft_addr.to_string())
                .await;
        }

        // If we're not the leader, request to join the cluster
        if leader_info.node_id != self.node_id {
            self.request_join(&leader_info).await?;
        }

        Ok(())
    }

    /// Wait for peers to be discovered
    async fn wait_for_peers(&self) -> Result<Vec<PeerInfo>> {
        let timeout = Duration::from_millis(self.config.discovery_timeout_ms);
        let start = std::time::Instant::now();
        let check_interval = Duration::from_millis(500);

        info!(
            "Waiting for at least {} peer(s) to be discovered (timeout: {}ms)...",
            self.config.min_peers_for_join, self.config.discovery_timeout_ms
        );

        loop {
            let peers = self.discovery.get_peers();

            if peers.len() >= self.config.min_peers_for_join {
                info!("Discovered {} peer(s)", peers.len());
                return Ok(peers);
            }

            if start.elapsed() > timeout {
                debug!(
                    "Discovery timeout reached with {} peer(s) found",
                    peers.len()
                );
                return Ok(peers);
            }

            debug!("Currently have {} peer(s), waiting...", peers.len());
            sleep(check_interval).await;
        }
    }

    /// Discover the leader node from the list of peers
    ///
    /// This uses a simple heuristic of selecting the peer with the lowest node ID,
    /// which is typically the bootstrap node in a cluster. In a production system
    /// with dynamic leader election, you would query each peer's Raft state via RPC
    /// to find the current leader.
    ///
    /// For the current implementation, the bootstrap node (lowest ID) starts as leader
    /// and maintains leadership unless it fails, at which point Raft will elect a new
    /// leader automatically.
    async fn discover_leader(&self, peers: &[PeerInfo]) -> Result<PeerInfo> {
        if peers.is_empty() {
            return Err(ScribeError::Cluster(
                "No peers available for leader discovery".to_string(),
            ));
        }

        // Use heuristic: lowest node ID is typically the bootstrap node/initial leader
        // This works for initial cluster formation. For dynamic scenarios, implement
        // RPC queries to each peer's Raft state to find the current leader.
        let leader = peers
            .iter()
            .min_by_key(|p| p.node_id)
            .ok_or_else(|| ScribeError::Cluster("Failed to select leader".to_string()))?;

        info!(
            "Selected node {} as leader candidate for cluster join",
            leader.node_id
        );

        Ok(leader.clone())
    }

    /// Request to join the cluster through the leader
    ///
    /// This prepares the node to join the cluster. The actual join process requires
    /// the leader to call add_learner() and then change_membership() on its ConsensusNode.
    ///
    /// In a complete distributed system, this would send an RPC request to the leader
    /// to initiate the join process. For the current implementation, cluster membership
    /// is managed through the ConsensusNode API (add_learner and change_membership methods).
    ///
    /// Operators should use the cluster management endpoints or CLI tools to add nodes,
    /// which will call the appropriate ConsensusNode methods on the leader.
    async fn request_join(&self, leader: &PeerInfo) -> Result<()> {
        info!(
            "Requesting to join cluster via leader node {} at {}",
            leader.node_id, leader.raft_addr
        );

        // Log the Raft address that will be used for this node
        let my_raft_addr = self
            .discovery
            .get_peers()
            .first()
            .map(|p| p.raft_addr.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        info!(
            "Node {} ready to join cluster (Raft addr: {}). Leader must call add_learner and change_membership.",
            self.node_id, my_raft_addr
        );

        // The actual join is coordinated externally through the leader's ConsensusNode:
        // 1. Leader calls consensus.add_learner(node_id, BasicNode { addr: raft_addr })
        // 2. Leader waits for log replication to catch up
        // 3. Leader calls consensus.change_membership() to promote to voter

        Ok(())
    }

    /// Handle network partitions gracefully
    pub async fn handle_partition(&self) -> Result<()> {
        warn!(
            "Detected potential network partition for node {}",
            self.node_id
        );

        // Check if we can still see any peers
        let peers = self.discovery.get_peers();

        if peers.is_empty() {
            warn!("No peers visible - potential network partition");
            // In a real system, we might want to:
            // 1. Step down as leader if we are one
            // 2. Enter a read-only mode
            // 3. Wait for reconnection
        } else {
            info!("Still have {} peer(s) visible", peers.len());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::DiscoveryConfig;

    #[test]
    fn test_cluster_config_default() {
        let config = ClusterConfig::default();
        assert_eq!(config.mode, InitMode::Join);
        assert_eq!(config.min_peers_for_join, 1);
    }

    #[test]
    fn test_init_mode() {
        assert_eq!(InitMode::Bootstrap, InitMode::Bootstrap);
        assert_eq!(InitMode::Join, InitMode::Join);
        assert_ne!(InitMode::Bootstrap, InitMode::Join);
    }

    #[tokio::test]
    async fn test_cluster_initializer_bootstrap() {
        let discovery_config = DiscoveryConfig {
            node_id: 1,
            raft_addr: "127.0.0.1:9001".parse().unwrap(),
            client_addr: "127.0.0.1:8001".parse().unwrap(),
            discovery_port: 17001,
            broadcast_addr: "127.0.0.1".to_string(),
            seed_addrs: Vec::new(),
            heartbeat_interval_ms: 500,
            failure_timeout_ms: 1500,
        };

        let discovery = Arc::new(DiscoveryService::new(discovery_config).unwrap());
        let db = sled::Config::new().temporary(true).open().unwrap();
        let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

        let cluster_config = ClusterConfig {
            mode: InitMode::Bootstrap,
            seed_addrs: Vec::new(),
            discovery_timeout_ms: 1000,
            min_peers_for_join: 1,
        };

        let initializer = ClusterInitializer::new(discovery, consensus.clone(), cluster_config);

        // Bootstrap should succeed
        assert!(initializer.initialize().await.is_ok());

        // After bootstrap, node should eventually become leader
        tokio::time::sleep(Duration::from_millis(2000)).await;
        assert!(consensus.is_leader().await);
    }
}
