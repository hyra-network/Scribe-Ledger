/// Raft consensus implementation for Scribe Ledger
use raft::{prelude::*, storage::MemStorage, Config as RaftConfig, RawNode, StateRole};
use slog::{Drain, Logger};
use crate::error::{Result, ScribeError};
use crate::manifest::{ManifestManager, ClusterManifest, ClusterNode};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use std::time::Duration;


/// Messages exchanged between cluster nodes
#[derive(Debug, Clone)]
pub enum ClusterMessage {
    RaftMessage(Message),
    ManifestUpdate(ClusterManifest),
    NodeJoin(ClusterNode),
    NodeLeave(u64),
    JoinRequest { node: ClusterNode, leader_hint: Option<u64> },
    JoinResponse { success: bool, leader_id: Option<u64>, members: Vec<ClusterNode> },
    LeaveRequest { node_id: u64 },
    LeaveResponse { success: bool },
    Heartbeat { from: u64, timestamp: u64 },
    Discovery { requesting_node: ClusterNode },
    DiscoveryResponse { leader_id: Option<u64>, members: Vec<ClusterNode> },
}

/// Serializable message wrapper for network transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    ManifestUpdate(ClusterManifest),
    NodeJoin(ClusterNode),
    NodeLeave(u64),
    JoinRequest { node: ClusterNode, leader_hint: Option<u64> },
    JoinResponse { success: bool, leader_id: Option<u64>, members: Vec<ClusterNode> },
    LeaveRequest { node_id: u64 },
    LeaveResponse { success: bool },
    Heartbeat { from: u64, timestamp: u64 },
    Discovery { requesting_node: ClusterNode },
    DiscoveryResponse { leader_id: Option<u64>, members: Vec<ClusterNode> },
}

/// Node state in the cluster lifecycle
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeState {
    Initializing,
    Joining,
    Active,
    Leaving,
    Left,
    Failed,
}

/// Enhanced cluster node with state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNodeState {
    pub node: ClusterNode,
    pub state: NodeState,
    pub last_heartbeat: u64,
    pub join_time: Option<u64>,
    pub leave_time: Option<u64>,
}

/// Network transport trait for cluster communication
#[async_trait::async_trait]
pub trait NetworkTransport: Send + Sync {
    async fn send_message(&self, address: &str, message: ClusterMessage) -> Result<()>;
    async fn broadcast_message(&self, addresses: &[String], message: ClusterMessage) -> Result<()>;
}

/// TCP-based transport implementation for Raft
pub struct TcpTransport {
    connections: Arc<Mutex<HashMap<u64, TcpStream>>>,
    connection_pool: Arc<Mutex<HashMap<String, TcpStream>>>,
    retry_count: u32,
    timeout: Duration,
}

impl TcpTransport {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            connection_pool: Arc::new(Mutex::new(HashMap::new())),
            retry_count: 3,
            timeout: Duration::from_secs(5),
        }
    }
    
    async fn get_or_create_connection(&self, address: &str) -> Result<TcpStream> {
        let mut pool = self.connection_pool.lock().await;
        
        // Try to reuse existing connection
        if let Some(stream) = pool.remove(address) {
            // Test if connection is still alive
            if self.test_connection(&stream).await {
                return Ok(stream);
            }
        }
        
        // Create new connection
        let stream = TcpStream::connect(address).await
            .map_err(|e| ScribeError::Network(format!("Failed to connect to {}: {}", address, e)))?;
            
        Ok(stream)
    }
    
    async fn test_connection(&self, _stream: &TcpStream) -> bool {
        // Simple connection test - could be improved with ping/pong
        true
    }
    
    async fn send_message_with_retry(&self, address: &str, message: &ClusterMessage) -> Result<()> {
        let mut attempts = 0;
        
        while attempts < self.retry_count {
            match self.send_message_once(address, message).await {
                Ok(_) => return Ok(()),
                Err(e) => {
                    attempts += 1;
                    if attempts >= self.retry_count {
                        return Err(e);
                    }
                    tracing::warn!("Failed to send message to {}, attempt {}/{}: {}", 
                                 address, attempts, self.retry_count, e);
                    tokio::time::sleep(Duration::from_millis(100 * attempts as u64)).await;
                }
            }
        }
        
        Err(ScribeError::Network(format!("Failed to send message to {} after {} attempts", address, self.retry_count)))
    }
    
    async fn send_message_once(&self, address: &str, message: &ClusterMessage) -> Result<()> {
        let mut stream = self.get_or_create_connection(address).await?;
        
        // Serialize message
        // Convert ClusterMessage to NetworkMessage for serialization
        let network_msg = match &message {
            ClusterMessage::RaftMessage(_) => {
                return Err(ScribeError::Consensus("Cannot serialize Raft messages over TCP".to_string()));
            },
            ClusterMessage::ManifestUpdate(manifest) => NetworkMessage::ManifestUpdate(manifest.clone()),
            ClusterMessage::NodeJoin(node) => NetworkMessage::NodeJoin(node.clone()),
            ClusterMessage::NodeLeave(node_id) => NetworkMessage::NodeLeave(*node_id),
            ClusterMessage::JoinRequest { node, leader_hint } => NetworkMessage::JoinRequest { node: node.clone(), leader_hint: *leader_hint },
            ClusterMessage::JoinResponse { success, leader_id, members } => NetworkMessage::JoinResponse { success: *success, leader_id: *leader_id, members: members.clone() },
            ClusterMessage::LeaveRequest { node_id } => NetworkMessage::LeaveRequest { node_id: *node_id },
            ClusterMessage::LeaveResponse { success } => NetworkMessage::LeaveResponse { success: *success },
            ClusterMessage::Heartbeat { from, timestamp } => NetworkMessage::Heartbeat { from: *from, timestamp: *timestamp },
            ClusterMessage::Discovery { requesting_node } => NetworkMessage::Discovery { requesting_node: requesting_node.clone() },
            ClusterMessage::DiscoveryResponse { leader_id, members } => NetworkMessage::DiscoveryResponse { leader_id: *leader_id, members: members.clone() },
        };
        
        let serialized = bincode::serialize(&network_msg)
            .map_err(|e| ScribeError::Consensus(format!("Serialization failed: {}", e)))?;
            
        // Send message length first
        let len = serialized.len() as u32;
        stream.write_all(&len.to_be_bytes()).await
            .map_err(|e| ScribeError::Network(format!("Failed to send length: {}", e)))?;
            
        // Send message
        stream.write_all(&serialized).await
            .map_err(|e| ScribeError::Network(format!("Failed to send message: {}", e)))?;
            
        stream.flush().await
            .map_err(|e| ScribeError::Network(format!("Failed to flush: {}", e)))?;
            
        Ok(())
    }
}

#[async_trait::async_trait]
impl NetworkTransport for TcpTransport {
    async fn send_message(&self, address: &str, message: ClusterMessage) -> Result<()> {
        self.send_message_with_retry(address, &message).await
    }
    
    async fn broadcast_message(&self, addresses: &[String], message: ClusterMessage) -> Result<()> {
        let mut tasks = Vec::new();
        
        for addr in addresses {
            let transport = self.clone();
            let addr = addr.clone();
            let msg = message.clone();
            
            let task = tokio::spawn(async move {
                transport.send_message(&addr, msg).await
            });
            tasks.push(task);
        }
        
        // Wait for all messages to be sent
        let mut errors = Vec::new();
        for task in tasks {
            if let Err(e) = task.await.unwrap_or_else(|e| Err(ScribeError::Network(format!("Task error: {}", e)))) {
                errors.push(e);
            }
        }
        
        if !errors.is_empty() {
            return Err(ScribeError::Network(format!("Broadcast failed for {} addresses", errors.len())));
        }
        
        Ok(())
    }
}

impl Clone for TcpTransport {
    fn clone(&self) -> Self {
        Self {
            connections: Arc::clone(&self.connections),
            connection_pool: Arc::clone(&self.connection_pool),
            retry_count: self.retry_count,
            timeout: self.timeout,
        }
    }
}

/// HTTP-based transport implementation (fallback)
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
    async fn send_message(&self, address: &str, message: ClusterMessage) -> Result<()> {
        // Convert ClusterMessage to NetworkMessage for serialization
        let network_msg = match &message {
            ClusterMessage::RaftMessage(_) => {
                return Err(ScribeError::Consensus("Cannot serialize Raft messages over HTTP".to_string()));
            },
            ClusterMessage::ManifestUpdate(manifest) => NetworkMessage::ManifestUpdate(manifest.clone()),
            ClusterMessage::NodeJoin(node) => NetworkMessage::NodeJoin(node.clone()),
            ClusterMessage::NodeLeave(node_id) => NetworkMessage::NodeLeave(*node_id),
            ClusterMessage::JoinRequest { node, leader_hint } => NetworkMessage::JoinRequest { node: node.clone(), leader_hint: *leader_hint },
            ClusterMessage::JoinResponse { success, leader_id, members } => NetworkMessage::JoinResponse { success: *success, leader_id: *leader_id, members: members.clone() },
            ClusterMessage::LeaveRequest { node_id } => NetworkMessage::LeaveRequest { node_id: *node_id },
            ClusterMessage::LeaveResponse { success } => NetworkMessage::LeaveResponse { success: *success },
            ClusterMessage::Heartbeat { from, timestamp } => NetworkMessage::Heartbeat { from: *from, timestamp: *timestamp },
            ClusterMessage::Discovery { requesting_node } => NetworkMessage::Discovery { requesting_node: requesting_node.clone() },
            ClusterMessage::DiscoveryResponse { leader_id, members } => NetworkMessage::DiscoveryResponse { leader_id: *leader_id, members: members.clone() },
        };
        
        let serialized = serde_json::to_vec(&network_msg)
            .map_err(|e| ScribeError::Serialization(format!("Failed to serialize: {}", e)))?;
            
        let response = self.client
            .post(&format!("http://{}/cluster/message", address))
            .header("Content-Type", "application/json")
            .body(serialized)
            .send()
            .await
            .map_err(|e| ScribeError::Network(format!("HTTP request failed: {}", e)))?;
            
        if !response.status().is_success() {
            return Err(ScribeError::Network(format!("HTTP error: {}", response.status())));
        }
        
        Ok(())
    }
    
    async fn broadcast_message(&self, addresses: &[String], message: ClusterMessage) -> Result<()> {
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
    tcp_port: u16,
    peer_addresses: Arc<RwLock<HashMap<u64, String>>>,
    cluster_state: Arc<RwLock<HashMap<u64, ClusterNodeState>>>,
    is_leader: Arc<RwLock<bool>>,
    tcp_server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ConsensusNode {
    /// Create a new consensus node
    pub fn new(
        node_id: u64,
        address: String,
        tcp_port: u16,
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
            tcp_port,
            peer_addresses: Arc::new(RwLock::new(HashMap::new())),
            cluster_state: Arc::new(RwLock::new(HashMap::new())),
            is_leader: Arc::new(RwLock::new(false)),
            tcp_server_handle: None,
        })
    }

    /// Start TCP server for Raft communication
    pub async fn start_tcp_server(&mut self) -> Result<()> {
        let listen_addr = format!("0.0.0.0:{}", self.tcp_port);
        let listener = TcpListener::bind(&listen_addr).await
            .map_err(|e| ScribeError::Network(format!("Failed to bind TCP server: {}", e)))?;
        
        tracing::info!("TCP server listening on {}", listen_addr);
        
        let cluster_state = Arc::clone(&self.cluster_state);
        let is_leader = Arc::clone(&self.is_leader);
        let node_id = self.node_id;
        
        let handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        tracing::debug!("New TCP connection from {}", addr);
                        let cluster_state = Arc::clone(&cluster_state);
                        let is_leader = Arc::clone(&is_leader);
                        
                        tokio::spawn(async move {
                            if let Err(e) = Self::handle_tcp_connection(stream, cluster_state, is_leader, node_id).await {
                                tracing::error!("Error handling TCP connection: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        tracing::error!("Failed to accept TCP connection: {}", e);
                        break;
                    }
                }
            }
        });
        
        self.tcp_server_handle = Some(handle);
        Ok(())
    }
    
    async fn handle_tcp_connection(
        mut stream: TcpStream,
        cluster_state: Arc<RwLock<HashMap<u64, ClusterNodeState>>>,
        is_leader: Arc<RwLock<bool>>,
        node_id: u64
    ) -> Result<()> {
        let mut buffer = [0u8; 4];
        
        // Read message length
        stream.read_exact(&mut buffer).await
            .map_err(|e| ScribeError::Network(format!("Failed to read length: {}", e)))?;
        let length = u32::from_be_bytes(buffer) as usize;
        
        if length > 10 * 1024 * 1024 { // 10MB limit
            return Err(ScribeError::Network("Message too large".to_string()));
        }
        
        // Read message data
        let mut message_data = vec![0u8; length];
        stream.read_exact(&mut message_data).await
            .map_err(|e| ScribeError::Network(format!("Failed to read message: {}", e)))?;
        
        // Deserialize message
        let network_msg: NetworkMessage = bincode::deserialize(&message_data)
            .map_err(|e| ScribeError::Serialization(format!("Failed to deserialize: {}", e)))?;
        
        // Convert NetworkMessage back to ClusterMessage
        let message = match network_msg {
            NetworkMessage::ManifestUpdate(manifest) => ClusterMessage::ManifestUpdate(manifest),
            NetworkMessage::NodeJoin(node) => ClusterMessage::NodeJoin(node),
            NetworkMessage::NodeLeave(node_id) => ClusterMessage::NodeLeave(node_id),
            NetworkMessage::JoinRequest { node, leader_hint } => ClusterMessage::JoinRequest { node, leader_hint },
            NetworkMessage::JoinResponse { success, leader_id, members } => ClusterMessage::JoinResponse { success, leader_id, members },
            NetworkMessage::LeaveRequest { node_id } => ClusterMessage::LeaveRequest { node_id },
            NetworkMessage::LeaveResponse { success } => ClusterMessage::LeaveResponse { success },
            NetworkMessage::Heartbeat { from, timestamp } => ClusterMessage::Heartbeat { from, timestamp },
            NetworkMessage::Discovery { requesting_node } => ClusterMessage::Discovery { requesting_node },
            NetworkMessage::DiscoveryResponse { leader_id, members } => ClusterMessage::DiscoveryResponse { leader_id, members },
        };
        
        // Process message
        Self::process_cluster_message(message, cluster_state, is_leader, node_id).await?;
        
        Ok(())
    }
    
    async fn process_cluster_message(
        message: ClusterMessage,
        cluster_state: Arc<RwLock<HashMap<u64, ClusterNodeState>>>,
        is_leader: Arc<RwLock<bool>>,
        _node_id: u64
    ) -> Result<()> {
        match message {
            ClusterMessage::RaftMessage(_) => {
                // Raft messages are handled separately through the raft library
                tracing::debug!("Received Raft message (handled by raft library)");
            }
            ClusterMessage::JoinRequest { node, leader_hint } => {
                if *is_leader.read().await {
                    tracing::info!("Processing join request from node {}", node.id);
                    // Add node to cluster state
                    let mut state = cluster_state.write().await;
                    state.insert(node.id, ClusterNodeState {
                        node: node.clone(),
                        state: NodeState::Joining,
                        last_heartbeat: Self::current_timestamp(),
                        join_time: Some(Self::current_timestamp()),
                        leave_time: None,
                    });
                    // Send join response (simplified)
                    tracing::info!("Node {} joined cluster", node.id);
                } else if let Some(leader_id) = leader_hint {
                    tracing::info!("Forwarding join request to leader {}", leader_id);
                    // Forward to leader
                }
            }
            ClusterMessage::JoinResponse { success, leader_id, members } => {
                tracing::info!("Received join response: success={}, leader_id={:?}", success, leader_id);
                if success {
                    // Update local cluster state with member list
                    let mut state = cluster_state.write().await;
                    for member in members {
                        state.insert(member.id, ClusterNodeState {
                            node: member,
                            state: NodeState::Active,
                            last_heartbeat: Self::current_timestamp(),
                            join_time: Some(Self::current_timestamp()),
                            leave_time: None,
                        });
                    }
                }
            }
            ClusterMessage::LeaveRequest { node_id: leaving_node_id } => {
                if *is_leader.read().await {
                    tracing::info!("Processing leave request from node {}", leaving_node_id);
                    let mut state = cluster_state.write().await;
                    if let Some(node_state) = state.get_mut(&leaving_node_id) {
                        node_state.state = NodeState::Leaving;
                        node_state.leave_time = Some(Self::current_timestamp());
                    }
                }
            }
            ClusterMessage::LeaveResponse { success } => {
                tracing::info!("Received leave response: success={}", success);
            }
            ClusterMessage::ManifestUpdate(manifest) => {
                tracing::info!("Received manifest update: {:?}", manifest);
                // Handle manifest update
            }
            ClusterMessage::NodeJoin(node) => {
                tracing::info!("Node {} joining cluster", node.id);
                let mut state = cluster_state.write().await;
                state.insert(node.id, ClusterNodeState {
                    node,
                    state: NodeState::Joining,
                    last_heartbeat: Self::current_timestamp(),
                    join_time: Some(Self::current_timestamp()),
                    leave_time: None,
                });
            }
            ClusterMessage::NodeLeave(node_id) => {
                tracing::info!("Node {} leaving cluster", node_id);
                let mut state = cluster_state.write().await;
                if let Some(node_state) = state.get_mut(&node_id) {
                    node_state.state = NodeState::Leaving;
                    node_state.leave_time = Some(Self::current_timestamp());
                }
            }
            ClusterMessage::Heartbeat { from, timestamp } => {
                let mut state = cluster_state.write().await;
                if let Some(node_state) = state.get_mut(&from) {
                    node_state.last_heartbeat = timestamp;
                    if node_state.state == NodeState::Joining {
                        node_state.state = NodeState::Active;
                    }
                }
            }
            ClusterMessage::Discovery { requesting_node } => {
                // Send discovery response with current cluster info
                tracing::debug!("Discovery request from node {}", requesting_node.id);
            }
            ClusterMessage::DiscoveryResponse { leader_id, members } => {
                tracing::info!("Received discovery response: leader_id={:?}, {} members", leader_id, members.len());
                // Update local cluster state with discovered members
                let mut state = cluster_state.write().await;
                for member in members {
                    state.insert(member.id, ClusterNodeState {
                        node: member,
                        state: NodeState::Active,
                        last_heartbeat: Self::current_timestamp(),
                        join_time: Some(Self::current_timestamp()),
                        leave_time: None,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
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
    
    /// Join an existing cluster
    pub async fn join_cluster(&mut self, seed_nodes: Vec<String>) -> Result<()> {
        tracing::info!("Attempting to join cluster with seed nodes: {:?}", seed_nodes);
        
        let join_request = ClusterMessage::JoinRequest {
            node: ClusterNode {
                id: self.node_id,
                address: format!("{}:{}", self.address, self.tcp_port),
                status: crate::manifest::NodeStatus::Active,
                last_seen: Self::current_timestamp(),
            },
            leader_hint: None,
        };
        
        // Try to contact seed nodes to find the leader
        for seed_addr in &seed_nodes {
            match self.transport.send_message(seed_addr, join_request.clone()).await {
                Ok(_) => {
                    tracing::info!("Sent join request to {}", seed_addr);
                    // Wait for response and process it
                    // In a real implementation, this would be handled by the TCP server
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!("Failed to contact seed node {}: {}", seed_addr, e);
                    continue;
                }
            }
        }
        
        Err(ScribeError::Consensus("Failed to contact any seed nodes".to_string()))
    }
    
    /// Leave the cluster gracefully
    pub async fn leave_cluster(&mut self) -> Result<()> {
        if !*self.is_leader.read().await {
            tracing::info!("Requesting to leave cluster");
            
            let leave_request = ClusterMessage::LeaveRequest {
                node_id: self.node_id,
            };
            
            // Find leader and send leave request
            let peer_addrs: Vec<String> = self.peer_addresses.read().await.values().cloned().collect();
            for addr in peer_addrs {
                if let Err(e) = self.transport.send_message(&addr, leave_request.clone()).await {
                    tracing::warn!("Failed to send leave request to {}: {}", addr, e);
                }
            }
        } else {
            tracing::info!("Leader leaving cluster - initiating leadership transfer");
            // Leader should transfer leadership before leaving
            self.transfer_leadership().await?;
        }
        
        // Update local state
        let mut state = self.cluster_state.write().await;
        if let Some(node_state) = state.get_mut(&self.node_id) {
            node_state.state = NodeState::Leaving;
            node_state.leave_time = Some(Self::current_timestamp());
        }
        
        // Stop TCP server
        if let Some(handle) = self.tcp_server_handle.take() {
            handle.abort();
        }
        
        Ok(())
    }
    
    /// Transfer leadership to another node
    async fn transfer_leadership(&mut self) -> Result<()> {
        tracing::info!("Transferring leadership...");
        
        // Find a suitable candidate (simplified selection)
        let cluster_state = self.cluster_state.read().await;
        let candidates: Vec<u64> = cluster_state
            .iter()
            .filter(|(_, state)| state.state == NodeState::Active && state.node.id != self.node_id)
            .map(|(id, _)| *id)
            .collect();
        
        if let Some(&candidate_id) = candidates.first() {
            tracing::info!("Transferring leadership to node {}", candidate_id);
            // In a real Raft implementation, this would use the transfer_leader API
            // For now, we'll simulate by stepping down
            *self.is_leader.write().await = false;
            Ok(())
        } else {
            Err(ScribeError::Consensus("No suitable leadership candidate found".to_string()))
        }
    }
    
    /// Add a peer to the cluster (leader operation)
    pub async fn add_peer(&mut self, node: ClusterNode) -> Result<()> {
        if !*self.is_leader.read().await {
            return Err(ScribeError::Consensus("Only leader can add peers".to_string()));
        }
        
        tracing::info!("Adding peer {} at {}", node.id, node.address);
        
        // Update peer addresses  
        self.peer_addresses.write().await.insert(node.id, node.address.clone());
        
        // Add to manifest
        self.manifest_manager.add_node(node.clone())?;
        
        // Add to cluster state
        let mut state = self.cluster_state.write().await;
        state.insert(node.id, ClusterNodeState {
            node: node.clone(),
            state: NodeState::Active,
            last_heartbeat: Self::current_timestamp(),
            join_time: Some(Self::current_timestamp()),
            leave_time: None,
        });
        
        // In a real implementation, this would propose a configuration change to Raft
        tracing::info!("Successfully added peer {}", node.id);
        Ok(())
    }
    
    /// Remove a peer from the cluster (leader operation)
    pub async fn remove_peer(&mut self, node_id: u64) -> Result<()> {
        if !*self.is_leader.read().await {
            return Err(ScribeError::Consensus("Only leader can remove peers".to_string()));
        }
        
        tracing::info!("Removing peer {}", node_id);
        
        // Remove from peer addresses
        self.peer_addresses.write().await.remove(&node_id);
        
        // Remove from manifest
        self.manifest_manager.remove_node(node_id)?;
        
        // Update cluster state
        let mut state = self.cluster_state.write().await;
        if let Some(node_state) = state.get_mut(&node_id) {
            node_state.state = NodeState::Left;
            node_state.leave_time = Some(Self::current_timestamp());
        }
        
        // In a real implementation, this would propose a configuration change to Raft
        tracing::info!("Successfully removed peer {}", node_id);
        Ok(())
    }
    
    /// Get current cluster members
    pub async fn get_cluster_members(&self) -> Vec<ClusterNodeState> {
        self.cluster_state.read().await.values().cloned().collect()
    }
    
    /// Check if this node is the leader
    pub async fn is_leader(&self) -> bool {
        *self.is_leader.read().await
    }
    
    /// Start heartbeat monitoring
    pub async fn start_heartbeat_monitor(&self) {
        let cluster_state = Arc::clone(&self.cluster_state);
        let node_id = self.node_id;
        let transport = Arc::clone(&self.transport);
        let peer_addresses = Arc::clone(&self.peer_addresses);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            
            loop {
                interval.tick().await;
                
                // Send heartbeats to all peers
                let heartbeat = ClusterMessage::Heartbeat {
                    from: node_id,
                    timestamp: Self::current_timestamp(),
                };
                
                let peers: Vec<String> = peer_addresses.read().await.values().cloned().collect();
                for peer_addr in peers {
                    if let Err(e) = transport.send_message(&peer_addr, heartbeat.clone()).await {
                        tracing::warn!("Failed to send heartbeat to {}: {}", peer_addr, e);
                    }
                }
                
                // Check for failed nodes
                let current_time = Self::current_timestamp();
                let mut state = cluster_state.write().await;
                for (node_id, node_state) in state.iter_mut() {
                    if current_time - node_state.last_heartbeat > 30 && node_state.state == NodeState::Active {
                        tracing::warn!("Node {} appears to have failed (no heartbeat for 30s)", node_id);
                        node_state.state = NodeState::Failed;
                    }
                }
            }
        });
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
            ClusterMessage::JoinRequest { node, leader_hint: _ } => {
                tracing::info!("Received join request from node {}", node.id);
                // Add peer through consensus
                self.add_peer(node).await?;
            },
            ClusterMessage::JoinResponse { success, leader_id: _, members: _ } => {
                tracing::info!("Received join response: success={}", success);
                // Handle join response locally
            },
            ClusterMessage::LeaveRequest { node_id } => {
                tracing::info!("Received leave request from node {}", node_id);
                self.remove_peer(node_id).await?;
            },
            ClusterMessage::LeaveResponse { success } => {
                tracing::info!("Received leave response: success={}", success);
                // Handle leave response locally
            },
            ClusterMessage::Heartbeat { from, timestamp: _ } => {
                tracing::debug!("Received heartbeat from node {}", from);
                // Heartbeats are handled by the transport layer
            },
            ClusterMessage::Discovery { requesting_node } => {
                tracing::debug!("Received discovery request from node {}", requesting_node.id);
                // Discovery is handled by the transport layer
            },
            ClusterMessage::DiscoveryResponse { leader_id: _, members: _ } => {
                tracing::debug!("Received discovery response");
                // Discovery responses are handled by the transport layer
            },
        }
        Ok(())
    }
    
    /// Process ready state and send messages
    pub async fn process_ready(&mut self) -> Result<()> {
        if !self.raft_node.has_ready() {
            return Ok(());
        }
        
        let ready = self.raft_node.ready();
        
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
            if let Ok(_manifest) = serde_json::from_slice::<ClusterManifest>(&entry.data) {
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
    pub fn is_raft_leader(&self) -> bool {
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
        let node = ConsensusNode::new(1, "127.0.0.1".to_string(), 8001, transport);
        assert!(node.is_ok());
        
        let node = node.unwrap();
        assert_eq!(node.node_id(), 1);
        assert_eq!(node.address(), "127.0.0.1");
        assert_eq!(node.tcp_port, 8001);
        assert!(!node.is_raft_leader()); // Initially not leader
    }
    
    #[tokio::test]
    async fn test_cluster_initialization() {
        let transport = Arc::new(MockTransport);
        let mut node = ConsensusNode::new(1, "127.0.0.1".to_string(), 8001, transport).unwrap();
        
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
        let mut node = ConsensusNode::new(1, "127.0.0.1".to_string(), 8001, transport).unwrap();
        
        // Make node a leader for testing
        *node.is_leader.write().await = true;
        
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