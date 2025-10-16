//! Node discovery service for automatic cluster formation
//!
//! This module provides UDP broadcast-based node discovery with heartbeat
//! and failure detection mechanisms.

use crate::error::{Result, ScribeError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

/// Default discovery port for UDP broadcasts
pub const DEFAULT_DISCOVERY_PORT: u16 = 7946;

/// Default heartbeat interval
pub const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 1000;

/// Default failure detection timeout (3x heartbeat interval)
pub const DEFAULT_FAILURE_TIMEOUT_MS: u64 = 3000;

/// Maximum UDP packet size for discovery messages
const MAX_UDP_PACKET_SIZE: usize = 1024;

/// Discovery message types exchanged between nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiscoveryMessage {
    /// Announce node presence
    Announce {
        node_id: u64,
        raft_addr: SocketAddr,
        client_addr: SocketAddr,
        /// Cluster secret for authentication (optional)
        cluster_secret: Option<String>,
    },
    /// Heartbeat to indicate node is alive
    Heartbeat {
        node_id: u64,
        /// Cluster secret for authentication (optional)
        cluster_secret: Option<String>,
    },
    /// Request peer list from other nodes
    PeerListRequest {
        node_id: u64,
        /// Cluster secret for authentication (optional)
        cluster_secret: Option<String>,
    },
    /// Response with known peers
    PeerListResponse { peers: Vec<PeerInfo> },
}

/// Information about a discovered peer node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerInfo {
    pub node_id: u64,
    pub raft_addr: SocketAddr,
    pub client_addr: SocketAddr,
}

/// Internal peer state tracking
#[derive(Debug, Clone)]
struct PeerState {
    pub info: PeerInfo,
    pub last_seen: Instant,
}

/// Configuration for the discovery service
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Node ID of this instance
    pub node_id: u64,
    /// Raft address of this node
    pub raft_addr: SocketAddr,
    /// Client API address of this node
    pub client_addr: SocketAddr,
    /// UDP port for discovery broadcasts
    pub discovery_port: u16,
    /// Broadcast address for discovery
    pub broadcast_addr: String,
    /// Seed addresses for initial discovery (optional, for testing/local networks)
    pub seed_addrs: Vec<String>,
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval_ms: u64,
    /// Failure detection timeout in milliseconds
    pub failure_timeout_ms: u64,
    /// Cluster secret for cross-network authentication (optional)
    pub cluster_secret: Option<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            node_id: 1,
            raft_addr: "127.0.0.1:9001".parse().unwrap(),
            client_addr: "127.0.0.1:8001".parse().unwrap(),
            discovery_port: DEFAULT_DISCOVERY_PORT,
            broadcast_addr: "255.255.255.255".to_string(),
            seed_addrs: Vec::new(),
            heartbeat_interval_ms: DEFAULT_HEARTBEAT_INTERVAL_MS,
            failure_timeout_ms: DEFAULT_FAILURE_TIMEOUT_MS,
            cluster_secret: None,
        }
    }
}

/// Node discovery service
pub struct DiscoveryService {
    config: DiscoveryConfig,
    peers: Arc<RwLock<HashMap<u64, PeerState>>>,
    socket: Arc<UdpSocket>,
    running: Arc<RwLock<bool>>,
}

impl DiscoveryService {
    /// Create a new discovery service
    pub fn new(config: DiscoveryConfig) -> Result<Self> {
        // Bind UDP socket for discovery
        let bind_addr = format!("0.0.0.0:{}", config.discovery_port);

        // Create socket with SO_REUSEADDR and SO_REUSEPORT for testing
        use std::net::ToSocketAddrs;
        let addr = bind_addr
            .to_socket_addrs()
            .map_err(|e| ScribeError::Network(format!("Invalid bind address: {}", e)))?
            .next()
            .ok_or_else(|| ScribeError::Network("Failed to resolve bind address".to_string()))?;

        let socket = socket2::Socket::new(
            if addr.is_ipv4() {
                socket2::Domain::IPV4
            } else {
                socket2::Domain::IPV6
            },
            socket2::Type::DGRAM,
            Some(socket2::Protocol::UDP),
        )
        .map_err(|e| ScribeError::Network(format!("Failed to create socket: {}", e)))?;

        // Enable address reuse for testing multiple instances
        socket
            .set_reuse_address(true)
            .map_err(|e| ScribeError::Network(format!("Failed to set reuse address: {}", e)))?;

        // Note: SO_REUSEPORT would be ideal for parallel tests but requires additional
        // platform-specific handling. SO_REUSEADDR is sufficient for most cases.

        socket
            .bind(&addr.into())
            .map_err(|e| ScribeError::Network(format!("Failed to bind discovery socket: {}", e)))?;

        let socket: UdpSocket = socket.into();

        // Enable broadcast
        socket
            .set_broadcast(true)
            .map_err(|e| ScribeError::Network(format!("Failed to enable broadcast: {}", e)))?;

        // Set non-blocking mode
        socket
            .set_nonblocking(true)
            .map_err(|e| ScribeError::Network(format!("Failed to set non-blocking mode: {}", e)))?;

        info!(
            "Discovery service initialized on port {} for node {}",
            config.discovery_port, config.node_id
        );

        Ok(Self {
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            socket: Arc::new(socket),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the discovery service
    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.write().unwrap();
            if *running {
                return Err(ScribeError::Discovery(
                    "Discovery service already running".to_string(),
                ));
            }
            *running = true;
        }

        info!(
            "Starting discovery service for node {}",
            self.config.node_id
        );

        // Send initial announce
        self.send_announce()?;

        // Spawn background tasks
        let peers_clone = Arc::clone(&self.peers);
        let config_clone = self.config.clone();
        let socket_clone = Arc::clone(&self.socket);
        let running_clone = Arc::clone(&self.running);

        // Receiver task
        tokio::spawn(async move {
            Self::receiver_loop(peers_clone, config_clone, socket_clone, running_clone).await;
        });

        // Heartbeat task
        let self_clone = self.clone_for_task();
        tokio::spawn(async move {
            self_clone.heartbeat_loop().await;
        });

        // Failure detection task
        let self_clone2 = self.clone_for_task();
        tokio::spawn(async move {
            self_clone2.failure_detection_loop().await;
        });

        Ok(())
    }

    /// Stop the discovery service
    pub fn stop(&self) {
        let mut running = self.running.write().unwrap();
        *running = false;
        info!("Discovery service stopped for node {}", self.config.node_id);
    }

    /// Get list of currently known peers
    pub fn get_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().unwrap();
        peers.values().map(|state| state.info.clone()).collect()
    }

    /// Get a specific peer by node ID
    pub fn get_peer(&self, node_id: u64) -> Option<PeerInfo> {
        let peers = self.peers.read().unwrap();
        peers.get(&node_id).map(|state| state.info.clone())
    }

    /// Check if a peer is alive
    pub fn is_peer_alive(&self, node_id: u64) -> bool {
        let peers = self.peers.read().unwrap();
        if let Some(state) = peers.get(&node_id) {
            let elapsed = state.last_seen.elapsed();
            elapsed.as_millis() < self.config.failure_timeout_ms as u128
        } else {
            false
        }
    }

    /// Send announce message to discover peers
    fn send_announce(&self) -> Result<()> {
        let msg = DiscoveryMessage::Announce {
            node_id: self.config.node_id,
            raft_addr: self.config.raft_addr,
            client_addr: self.config.client_addr,
            cluster_secret: self.config.cluster_secret.clone(),
        };

        self.broadcast_message(&msg)?;
        debug!("Sent announce for node {}", self.config.node_id);
        Ok(())
    }

    /// Send heartbeat message
    fn send_heartbeat(&self) -> Result<()> {
        let msg = DiscoveryMessage::Heartbeat {
            node_id: self.config.node_id,
            cluster_secret: self.config.cluster_secret.clone(),
        };

        self.broadcast_message(&msg)?;
        debug!("Sent heartbeat for node {}", self.config.node_id);
        Ok(())
    }

    /// Broadcast a discovery message
    fn broadcast_message(&self, msg: &DiscoveryMessage) -> Result<()> {
        let data = bincode::serialize(msg).map_err(|e| {
            ScribeError::Serialization(format!("Failed to serialize message: {}", e))
        })?;

        if data.len() > MAX_UDP_PACKET_SIZE {
            return Err(ScribeError::Discovery(format!(
                "Message too large: {} bytes",
                data.len()
            )));
        }

        // Send to broadcast address
        let broadcast_addr = format!(
            "{}:{}",
            self.config.broadcast_addr, self.config.discovery_port
        );
        let _ = self.socket.send_to(&data, &broadcast_addr);

        // Also send to seed addresses if configured (useful for testing and local networks)
        if !self.config.seed_addrs.is_empty() {
            for seed in &self.config.seed_addrs {
                let addr = if seed.contains(':') {
                    seed.clone()
                } else {
                    format!("{}:{}", seed, self.config.discovery_port)
                };
                if let Err(e) = self.socket.send_to(&data, &addr) {
                    debug!("Failed to send to seed {}: {}", addr, e);
                }
            }
        }

        debug!(
            "Sent message to broadcast and {} seed addresses",
            self.config.seed_addrs.len()
        );
        Ok(())
    }

    /// Receiver loop for processing incoming messages
    async fn receiver_loop(
        peers: Arc<RwLock<HashMap<u64, PeerState>>>,
        config: DiscoveryConfig,
        socket: Arc<UdpSocket>,
        running: Arc<RwLock<bool>>,
    ) {
        let mut buf = vec![0u8; MAX_UDP_PACKET_SIZE];

        loop {
            // Check if still running
            {
                let is_running = *running.read().unwrap();
                if !is_running {
                    break;
                }
            }

            // Try to receive message
            match socket.recv_from(&mut buf) {
                Ok((size, from_addr)) => {
                    if let Ok(msg) = bincode::deserialize::<DiscoveryMessage>(&buf[..size]) {
                        Self::handle_message(&peers, &config, &msg, &socket, from_addr);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No data available, sleep briefly
                    sleep(Duration::from_millis(10)).await;
                }
                Err(e) => {
                    error!("Error receiving discovery message: {}", e);
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }

        info!("Discovery receiver loop stopped");
    }

    /// Handle incoming discovery message
    fn handle_message(
        peers: &Arc<RwLock<HashMap<u64, PeerState>>>,
        config: &DiscoveryConfig,
        msg: &DiscoveryMessage,
        socket: &Arc<UdpSocket>,
        from_addr: SocketAddr,
    ) {
        match msg {
            DiscoveryMessage::Announce {
                node_id,
                raft_addr,
                client_addr,
                cluster_secret,
            } => {
                // Ignore our own announces
                if *node_id == config.node_id {
                    return;
                }

                // Validate cluster secret if configured
                if !Self::validate_cluster_secret(config, cluster_secret) {
                    warn!(
                        "Rejected announce from node {} - invalid cluster secret",
                        node_id
                    );
                    return;
                }

                let mut peers_map = peers.write().unwrap();
                let peer_info = PeerInfo {
                    node_id: *node_id,
                    raft_addr: *raft_addr,
                    client_addr: *client_addr,
                };

                let is_new_peer = !peers_map.contains_key(node_id);

                peers_map.insert(
                    *node_id,
                    PeerState {
                        info: peer_info.clone(),
                        last_seen: Instant::now(),
                    },
                );

                info!("Discovered peer node {} at {}", node_id, raft_addr);

                // If this is a new peer, respond with our own announce directly to sender
                if is_new_peer {
                    drop(peers_map); // Release lock before sending
                    let response = DiscoveryMessage::Announce {
                        node_id: config.node_id,
                        raft_addr: config.raft_addr,
                        client_addr: config.client_addr,
                        cluster_secret: config.cluster_secret.clone(),
                    };

                    if let Ok(data) = bincode::serialize(&response) {
                        // Send directly back to the sender's address
                        let _ = socket.send_to(&data, from_addr);
                        debug!(
                            "Sent announce response to new peer {} at {}",
                            node_id, from_addr
                        );
                    }
                }
            }
            DiscoveryMessage::Heartbeat {
                node_id,
                cluster_secret,
            } => {
                // Ignore our own heartbeats
                if *node_id == config.node_id {
                    return;
                }

                // Validate cluster secret if configured
                if !Self::validate_cluster_secret(config, cluster_secret) {
                    warn!(
                        "Rejected heartbeat from node {} - invalid cluster secret",
                        node_id
                    );
                    return;
                }

                let mut peers_map = peers.write().unwrap();
                if let Some(state) = peers_map.get_mut(node_id) {
                    state.last_seen = Instant::now();
                    debug!("Received heartbeat from node {}", node_id);
                } else {
                    debug!("Received heartbeat from unknown node {}, ignoring", node_id);
                }
            }
            DiscoveryMessage::PeerListRequest {
                node_id,
                cluster_secret,
            } => {
                // Validate cluster secret if configured
                if !Self::validate_cluster_secret(config, cluster_secret) {
                    warn!(
                        "Rejected peer list request from node {} - invalid cluster secret",
                        node_id
                    );
                    return;
                }

                debug!("Received peer list request from node {}", node_id);
                // Implementation for peer list exchange would go here
                // For now, receiving an announce is sufficient for discovery
            }
            DiscoveryMessage::PeerListResponse { peers: _peer_list } => {
                debug!("Received peer list response");
                // Implementation for peer list exchange would go here
                // For now, direct announces are sufficient
            }
        }
    }

    /// Heartbeat loop to periodically send heartbeats
    async fn heartbeat_loop(&self) {
        let interval = Duration::from_millis(self.config.heartbeat_interval_ms);

        loop {
            // Check if still running
            {
                let is_running = *self.running.read().unwrap();
                if !is_running {
                    break;
                }
            }

            sleep(interval).await;

            if let Err(e) = self.send_heartbeat() {
                warn!("Failed to send heartbeat: {}", e);
            }
        }

        info!("Heartbeat loop stopped");
    }

    /// Failure detection loop to remove dead peers
    async fn failure_detection_loop(&self) {
        let check_interval = Duration::from_millis(self.config.heartbeat_interval_ms);
        let failure_timeout = Duration::from_millis(self.config.failure_timeout_ms);

        loop {
            // Check if still running
            {
                let is_running = *self.running.read().unwrap();
                if !is_running {
                    break;
                }
            }

            sleep(check_interval).await;

            // Remove dead peers
            let mut peers = self.peers.write().unwrap();
            let mut dead_peers = Vec::new();

            for (node_id, state) in peers.iter() {
                if state.last_seen.elapsed() > failure_timeout {
                    dead_peers.push(*node_id);
                }
            }

            for node_id in dead_peers {
                peers.remove(&node_id);
                warn!("Removed dead peer node {}", node_id);
            }
        }

        info!("Failure detection loop stopped");
    }

    /// Validate cluster secret from incoming message
    fn validate_cluster_secret(config: &DiscoveryConfig, msg_secret: &Option<String>) -> bool {
        match (&config.cluster_secret, msg_secret) {
            // Both have secrets - they must match
            (Some(our_secret), Some(their_secret)) => our_secret == their_secret,
            // We have a secret but they don't - reject
            (Some(_), None) => false,
            // We don't have a secret but they do - reject (for security)
            (None, Some(_)) => false,
            // Neither has secret - allow (open network)
            (None, None) => true,
        }
    }

    /// Clone for background tasks
    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            peers: Arc::clone(&self.peers),
            socket: Arc::clone(&self.socket),
            running: Arc::clone(&self.running),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test constants to avoid hardcoded values
    const TEST_NODE_ID: u64 = 1;
    const TEST_NODE_ID_2: u64 = 2;
    const TEST_HEARTBEAT_NODE_ID: u64 = 42;
    const TEST_NONEXISTENT_NODE_ID: u64 = 999;
    const TEST_RAFT_PORT: u16 = 9001;
    const TEST_RAFT_PORT_2: u16 = 9002;
    const TEST_CLIENT_PORT: u16 = 8001;
    const TEST_CLIENT_PORT_2: u16 = 8002;
    const TEST_IP: &str = "127.0.0.1";

    fn test_raft_addr(port: u16) -> std::net::SocketAddr {
        format!("{}:{}", TEST_IP, port).parse().unwrap()
    }

    fn test_client_addr(port: u16) -> std::net::SocketAddr {
        format!("{}:{}", TEST_IP, port).parse().unwrap()
    }

    #[test]
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.node_id, TEST_NODE_ID);
        assert_eq!(config.discovery_port, DEFAULT_DISCOVERY_PORT);
        assert_eq!(config.heartbeat_interval_ms, DEFAULT_HEARTBEAT_INTERVAL_MS);
        assert_eq!(config.failure_timeout_ms, DEFAULT_FAILURE_TIMEOUT_MS);
    }

    #[test]
    fn test_peer_info_serialization() {
        let peer = PeerInfo {
            node_id: TEST_NODE_ID,
            raft_addr: test_raft_addr(TEST_RAFT_PORT),
            client_addr: test_client_addr(TEST_CLIENT_PORT),
        };

        let serialized = bincode::serialize(&peer).unwrap();
        let deserialized: PeerInfo = bincode::deserialize(&serialized).unwrap();

        assert_eq!(peer, deserialized);
    }

    #[test]
    fn test_discovery_message_serialization() {
        let msg = DiscoveryMessage::Announce {
            node_id: TEST_NODE_ID,
            raft_addr: test_raft_addr(TEST_RAFT_PORT),
            client_addr: test_client_addr(TEST_CLIENT_PORT),
            cluster_secret: None,
        };

        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: DiscoveryMessage = bincode::deserialize(&serialized).unwrap();

        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_heartbeat_message_serialization() {
        let msg = DiscoveryMessage::Heartbeat {
            node_id: TEST_HEARTBEAT_NODE_ID,
            cluster_secret: None,
        };

        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: DiscoveryMessage = bincode::deserialize(&serialized).unwrap();

        assert_eq!(msg, deserialized);
    }

    #[tokio::test]
    async fn test_discovery_service_creation() {
        let config = DiscoveryConfig {
            node_id: TEST_NODE_ID,
            raft_addr: test_raft_addr(TEST_RAFT_PORT),
            client_addr: test_client_addr(TEST_CLIENT_PORT),
            discovery_port: 17946, // Use non-standard port for testing
            broadcast_addr: TEST_IP.to_string(),
            seed_addrs: Vec::new(),
            heartbeat_interval_ms: 500,
            cluster_secret: None,
            failure_timeout_ms: 1500,
        };

        let service = DiscoveryService::new(config);
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_get_peers_empty() {
        let config = DiscoveryConfig {
            node_id: TEST_NODE_ID,
            raft_addr: test_raft_addr(TEST_RAFT_PORT),
            client_addr: test_client_addr(TEST_CLIENT_PORT),
            discovery_port: 17947,
            broadcast_addr: TEST_IP.to_string(),
            seed_addrs: Vec::new(),
            heartbeat_interval_ms: 500,
            failure_timeout_ms: 1500,
            cluster_secret: None,
        };

        let service = DiscoveryService::new(config).unwrap();
        let peers = service.get_peers();
        assert_eq!(peers.len(), 0);
    }

    #[tokio::test]
    async fn test_peer_tracking() {
        let config = DiscoveryConfig {
            node_id: TEST_NODE_ID,
            raft_addr: test_raft_addr(TEST_RAFT_PORT),
            client_addr: test_client_addr(TEST_CLIENT_PORT),
            discovery_port: 17948,
            broadcast_addr: TEST_IP.to_string(),
            seed_addrs: Vec::new(),
            heartbeat_interval_ms: 500,
            failure_timeout_ms: 1500,
            cluster_secret: None,
        };

        let service = DiscoveryService::new(config).unwrap();

        // Manually add a peer
        let peer_info = PeerInfo {
            node_id: TEST_NODE_ID_2,
            raft_addr: test_raft_addr(TEST_RAFT_PORT_2),
            client_addr: test_client_addr(TEST_CLIENT_PORT_2),
        };

        {
            let mut peers = service.peers.write().unwrap();
            peers.insert(
                TEST_NODE_ID_2,
                PeerState {
                    info: peer_info.clone(),
                    last_seen: Instant::now(),
                },
            );
        }

        let peers = service.get_peers();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, TEST_NODE_ID_2);

        let peer = service.get_peer(TEST_NODE_ID_2);
        assert!(peer.is_some());
        assert_eq!(peer.unwrap().node_id, TEST_NODE_ID_2);

        assert!(service.is_peer_alive(TEST_NODE_ID_2));
        assert!(!service.is_peer_alive(TEST_NONEXISTENT_NODE_ID));
    }

    #[tokio::test]
    async fn test_peer_expiration() {
        let config = DiscoveryConfig {
            node_id: TEST_NODE_ID,
            raft_addr: test_raft_addr(TEST_RAFT_PORT),
            client_addr: test_client_addr(TEST_CLIENT_PORT),
            discovery_port: 17949,
            broadcast_addr: TEST_IP.to_string(),
            seed_addrs: Vec::new(),
            heartbeat_interval_ms: 100,
            cluster_secret: None,
            failure_timeout_ms: 200, // Very short timeout for testing
        };

        let service = DiscoveryService::new(config).unwrap();

        // Manually add a peer with old timestamp
        let peer_info = PeerInfo {
            node_id: TEST_NODE_ID_2,
            raft_addr: test_raft_addr(TEST_RAFT_PORT_2),
            client_addr: test_client_addr(TEST_CLIENT_PORT_2),
        };

        {
            let mut peers = service.peers.write().unwrap();
            peers.insert(
                TEST_NODE_ID_2,
                PeerState {
                    info: peer_info,
                    last_seen: Instant::now() - Duration::from_millis(300),
                },
            );
        }

        // Peer should be considered dead
        assert!(!service.is_peer_alive(TEST_NODE_ID_2));
    }

    #[tokio::test]
    async fn test_start_stop() {
        let config = DiscoveryConfig {
            node_id: TEST_NODE_ID,
            raft_addr: test_raft_addr(TEST_RAFT_PORT),
            client_addr: test_client_addr(TEST_CLIENT_PORT),
            discovery_port: 17950,
            broadcast_addr: TEST_IP.to_string(),
            seed_addrs: Vec::new(),
            heartbeat_interval_ms: 500,
            failure_timeout_ms: 1500,
            cluster_secret: None,
        };

        let service = DiscoveryService::new(config).unwrap();

        // Start service
        let result = service.start().await;
        assert!(result.is_ok());

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Stop service
        service.stop();

        // Give it a moment to stop
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify it stopped
        let running = *service.running.read().unwrap();
        assert!(!running);
    }

    #[test]
    fn test_message_size_limit() {
        let msg = DiscoveryMessage::Announce {
            node_id: TEST_NODE_ID,
            raft_addr: test_raft_addr(TEST_RAFT_PORT),
            client_addr: test_client_addr(TEST_CLIENT_PORT),
            cluster_secret: None,
        };

        let serialized = bincode::serialize(&msg).unwrap();
        assert!(serialized.len() <= MAX_UDP_PACKET_SIZE);
    }
}
