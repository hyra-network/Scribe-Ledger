/// Cluster discovery and membership management
use crate::error::Result;
use crate::consensus::{ClusterNodeState, NodeState};
use crate::manifest::ClusterNode;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

/// Service discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub discovery_interval: Duration,
    pub health_check_interval: Duration,
    pub node_timeout: Duration,
    pub seed_nodes: Vec<String>,
    pub enable_auto_discovery: bool,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            discovery_interval: Duration::from_secs(30),
            health_check_interval: Duration::from_secs(10),
            node_timeout: Duration::from_secs(60),
            seed_nodes: Vec::new(),
            enable_auto_discovery: true,
        }
    }
}

/// Cluster discovery service
pub struct ClusterDiscovery {
    config: DiscoveryConfig,
    known_nodes: Arc<RwLock<HashMap<u64, ClusterNodeState>>>,
    leader_id: Arc<RwLock<Option<u64>>>,
    local_node_id: u64,
    last_discovery: Arc<RwLock<Instant>>,
}

impl ClusterDiscovery {
    pub fn new(local_node_id: u64, config: DiscoveryConfig) -> Self {
        Self {
            config,
            known_nodes: Arc::new(RwLock::new(HashMap::new())),
            leader_id: Arc::new(RwLock::new(None)),
            local_node_id,
            last_discovery: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    /// Start the discovery service
    pub async fn start(&self) -> Result<()> {
        if self.config.enable_auto_discovery {
            self.start_discovery_loop().await;
        }
        self.start_health_monitoring().await;
        Ok(())
    }
    
    /// Discover cluster members
    pub async fn discover_cluster(&self) -> Result<Vec<ClusterNodeState>> {
        tracing::info!("Starting cluster discovery...");
        
        let mut discovered_nodes = HashMap::new();
        
        // Try seed nodes first
        for seed_addr in &self.config.seed_nodes {
            match self.contact_node(seed_addr).await {
                Ok(nodes) => {
                    for node in nodes {
                        discovered_nodes.insert(node.node.id, node);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to contact seed node {}: {}", seed_addr, e);
                }
            }
        }
        
        // Update known nodes
        let mut known = self.known_nodes.write().await;
        for (id, node_state) in discovered_nodes {
            known.insert(id, node_state);
        }
        
        *self.last_discovery.write().await = Instant::now();
        
        Ok(known.values().cloned().collect())
    }
    
    /// Contact a node for discovery
    async fn contact_node(&self, address: &str) -> Result<Vec<ClusterNodeState>> {
        // Simulate contacting node for discovery
        // In real implementation, this would send a discovery message
        tracing::debug!("Contacting node at {} for discovery", address);
        
        // For now, return empty list
        Ok(Vec::new())
    }
    
    /// Find the current leader
    pub async fn find_leader(&self) -> Option<u64> {
        *self.leader_id.read().await
    }
    
    /// Update leader information
    pub async fn update_leader(&self, leader_id: Option<u64>) {
        *self.leader_id.write().await = leader_id;
        if let Some(id) = leader_id {
            tracing::info!("Leader updated to node {}", id);
        } else {
            tracing::warn!("No leader currently known");
        }
    }
    
    /// Get all known nodes
    pub async fn get_known_nodes(&self) -> Vec<ClusterNodeState> {
        self.known_nodes.read().await.values().cloned().collect()
    }
    
    /// Add a newly discovered node
    pub async fn add_discovered_node(&self, node_state: ClusterNodeState) {
        let mut known = self.known_nodes.write().await;
        known.insert(node_state.node.id, node_state.clone());
        tracing::info!("Discovered new node: {}", node_state.node.id);
    }
    
    /// Remove a node that has left or failed
    pub async fn remove_node(&self, node_id: u64) {
        let mut known = self.known_nodes.write().await;
        if known.remove(&node_id).is_some() {
            tracing::info!("Removed node {} from known nodes", node_id);
        }
        
        // Update leader if it was the removed node
        if let Some(leader) = *self.leader_id.read().await {
            if leader == node_id {
                *self.leader_id.write().await = None;
                tracing::warn!("Leader node {} removed, leader unknown", node_id);
            }
        }
    }
    
    /// Start periodic discovery loop
    async fn start_discovery_loop(&self) {
        let discovery = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(discovery.config.discovery_interval);
            
            loop {
                interval.tick().await;
                
                if let Err(e) = discovery.discover_cluster().await {
                    tracing::error!("Discovery failed: {}", e);
                }
                
                // Clean up old/failed nodes
                discovery.cleanup_failed_nodes().await;
            }
        });
    }
    
    /// Start health monitoring
    async fn start_health_monitoring(&self) {
        let discovery = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(discovery.config.health_check_interval);
            
            loop {
                interval.tick().await;
                discovery.check_node_health().await;
            }
        });
    }
    
    /// Check health of known nodes
    async fn check_node_health(&self) {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let timeout_seconds = self.config.node_timeout.as_secs();
        let mut nodes_to_remove = Vec::new();
        
        {
            let mut known = self.known_nodes.write().await;
            for (node_id, node_state) in known.iter_mut() {
                if current_time - node_state.last_heartbeat > timeout_seconds {
                    if node_state.state != NodeState::Failed {
                        tracing::warn!("Node {} marked as failed (no heartbeat)", node_id);
                        node_state.state = NodeState::Failed;
                    }
                    
                    // Mark for removal if failed for too long
                    if current_time - node_state.last_heartbeat > timeout_seconds * 2 {
                        nodes_to_remove.push(*node_id);
                    }
                }
            }
        }
        
        // Remove nodes that have been failed for too long
        for node_id in nodes_to_remove {
            self.remove_node(node_id).await;
        }
    }
    
    /// Clean up failed nodes
    async fn cleanup_failed_nodes(&self) {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let cleanup_threshold = self.config.node_timeout.as_secs() * 3;
        let mut nodes_to_remove = Vec::new();
        
        {
            let known = self.known_nodes.read().await;
            for (node_id, node_state) in known.iter() {
                if matches!(node_state.state, NodeState::Failed | NodeState::Left) {
                    if current_time - node_state.last_heartbeat > cleanup_threshold {
                        nodes_to_remove.push(*node_id);
                    }
                }
            }
        }
        
        for node_id in nodes_to_remove {
            self.remove_node(node_id).await;
        }
    }
}

impl Clone for ClusterDiscovery {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            known_nodes: Arc::clone(&self.known_nodes),
            leader_id: Arc::clone(&self.leader_id),
            local_node_id: self.local_node_id,
            last_discovery: Arc::clone(&self.last_discovery),
        }
    }
}

/// Cluster membership manager
pub struct MembershipManager {
    discovery: ClusterDiscovery,
    pending_joins: Arc<RwLock<HashMap<u64, ClusterNode>>>,
    pending_leaves: Arc<RwLock<HashMap<u64, Instant>>>,
}

impl MembershipManager {
    pub fn new(local_node_id: u64, config: DiscoveryConfig) -> Self {
        Self {
            discovery: ClusterDiscovery::new(local_node_id, config),
            pending_joins: Arc::new(RwLock::new(HashMap::new())),
            pending_leaves: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Start membership management
    pub async fn start(&self) -> Result<()> {
        self.discovery.start().await?;
        self.start_membership_processing().await;
        Ok(())
    }
    
    /// Request to join cluster
    pub async fn request_join(&self, node: ClusterNode) -> Result<()> {
        tracing::info!("Processing join request for node {}", node.id);
        
        let mut pending = self.pending_joins.write().await;
        pending.insert(node.id, node.clone());
        
        // Add to discovery as well
        let node_state = ClusterNodeState {
            node,
            state: NodeState::Joining,
            last_heartbeat: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            join_time: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()),
            leave_time: None,
        };
        
        self.discovery.add_discovered_node(node_state).await;
        Ok(())
    }
    
    /// Request to leave cluster
    pub async fn request_leave(&self, node_id: u64) -> Result<()> {
        tracing::info!("Processing leave request for node {}", node_id);
        
        let mut pending = self.pending_leaves.write().await;
        pending.insert(node_id, Instant::now());
        
        Ok(())
    }
    
    /// Start processing membership changes
    async fn start_membership_processing(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                manager.process_pending_changes().await;
            }
        });
    }
    
    /// Process pending membership changes
    async fn process_pending_changes(&self) {
        // Process pending joins
        let pending_joins: Vec<ClusterNode> = {
            let pending = self.pending_joins.read().await;
            pending.values().cloned().collect()
        };
        
        for node in pending_joins {
            // Simulate approval process
            tracing::info!("Approving join for node {}", node.id);
            let mut pending = self.pending_joins.write().await;
            pending.remove(&node.id);
        }
        
        // Process pending leaves
        let pending_leaves: Vec<u64> = {
            let pending = self.pending_leaves.read().await;
            pending.keys().cloned().collect()
        };
        
        for node_id in pending_leaves {
            tracing::info!("Approving leave for node {}", node_id);
            self.discovery.remove_node(node_id).await;
            let mut pending = self.pending_leaves.write().await;
            pending.remove(&node_id);
        }
    }
    
    /// Get cluster discovery service
    pub fn discovery(&self) -> &ClusterDiscovery {
        &self.discovery
    }
}

impl Clone for MembershipManager {
    fn clone(&self) -> Self {
        Self {
            discovery: self.discovery.clone(),
            pending_joins: Arc::clone(&self.pending_joins),
            pending_leaves: Arc::clone(&self.pending_leaves),
        }
    }
}