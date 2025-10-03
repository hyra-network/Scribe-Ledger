//! Cluster initialization and management
//!
//! This module provides automatic cluster formation by coordinating node discovery
//! with Raft consensus cluster initialization.

use crate::consensus::ConsensusNode;
use crate::discovery::{DiscoveryService, PeerInfo};
use crate::error::{Result, ScribeError};
use crate::types::NodeId;
use openraft::BasicNode;
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
            warn!("No peers discovered, falling back to bootstrap mode");
            return self.bootstrap().await;
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
    async fn discover_leader(&self, peers: &[PeerInfo]) -> Result<PeerInfo> {
        // Strategy: Try to contact each peer to find the leader
        // For simplicity, we'll use the first available peer as a starting point
        // In a real implementation, we would query each peer's Raft state

        if peers.is_empty() {
            return Err(ScribeError::Cluster(
                "No peers available for leader discovery".to_string(),
            ));
        }

        // For now, use a simple heuristic: the lowest node ID is likely the bootstrap node
        // In production, we would actually query the Raft state of peers
        let leader = peers
            .iter()
            .min_by_key(|p| p.node_id)
            .ok_or_else(|| ScribeError::Cluster("Failed to select leader".to_string()))?;

        Ok(leader.clone())
    }

    /// Request to join the cluster through the leader
    async fn request_join(&self, leader: &PeerInfo) -> Result<()> {
        info!(
            "Requesting to join cluster via leader node {} at {}",
            leader.node_id, leader.raft_addr
        );

        // Create a BasicNode for this node
        let _my_node = BasicNode {
            addr: self.get_my_raft_addr()?,
        };

        // First, add ourselves as a learner
        debug!("Adding node {} as learner", self.node_id);

        // Note: In a real distributed system, we would send an RPC to the leader
        // to have it add us as a learner. For now, we're assuming the leader
        // will handle this through another mechanism (e.g., automatic discovery)

        // For the purpose of this implementation, we'll document that the leader
        // needs to call add_learner on the joining node

        info!(
            "Node {} prepared to join cluster (waiting for leader to add as learner)",
            self.node_id
        );

        Ok(())
    }

    /// Get this node's Raft address from discovery config
    fn get_my_raft_addr(&self) -> Result<String> {
        // We need to extract the Raft address from somewhere
        // This would typically come from the node's configuration
        // For now, we'll return a placeholder that should be overridden
        Err(ScribeError::Configuration(
            "Raft address not configured - should be set in node configuration".to_string(),
        ))
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
