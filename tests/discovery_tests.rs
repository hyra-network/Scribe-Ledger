//! Integration tests for the discovery service
//!
//! These tests verify node discovery, heartbeat, and failure detection functionality.

use hyra_scribe_ledger::discovery::{
    DiscoveryConfig, DiscoveryService, DEFAULT_HEARTBEAT_INTERVAL_MS,
};
use std::time::Duration;
use tokio::time::sleep;

/// Helper to create a test discovery config for a node
/// Uses unique ports per node and configures seed addresses for discovery
fn create_test_config(node_id: u64, base_port: u16, num_nodes: u64) -> DiscoveryConfig {
    let my_port = base_port + node_id as u16;

    // Create seed addresses for all other nodes in the cluster
    let mut seed_addrs = Vec::new();
    for other_id in 1..=num_nodes {
        if other_id != node_id {
            let other_port = base_port + other_id as u16;
            seed_addrs.push(format!("127.0.0.1:{}", other_port));
        }
    }

    DiscoveryConfig {
        node_id,
        raft_addr: format!("127.0.0.1:{}", 9000 + node_id).parse().unwrap(),
        client_addr: format!("127.0.0.1:{}", 8000 + node_id).parse().unwrap(),
        discovery_port: my_port,
        broadcast_addr: "127.0.0.1".to_string(),
        seed_addrs,                 // Seed addresses for discovery
        heartbeat_interval_ms: 200, // Fast heartbeat for testing
        failure_timeout_ms: 600,    // 3x heartbeat
        cluster_secret: None,       // No secret for tests
    }
}

#[tokio::test]
async fn test_single_node_bootstrap() {
    // Test that a single node can start discovery service
    let config = create_test_config(1, 18001, 1);
    let service = DiscoveryService::new(config).unwrap();

    assert!(service.start().await.is_ok());

    // Wait a bit
    sleep(Duration::from_millis(300)).await;

    // Should have no peers
    let peers = service.get_peers();
    assert_eq!(peers.len(), 0);

    service.stop();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_two_node_discovery() {
    // Test two nodes discovering each other
    let config1 = create_test_config(1, 18010, 2);
    let config2 = create_test_config(2, 18010, 2);

    let service1 = DiscoveryService::new(config1).unwrap();
    let service2 = DiscoveryService::new(config2).unwrap();

    // Start both services
    service1.start().await.unwrap();
    service2.start().await.unwrap();

    // Wait for discovery to happen (multiple heartbeat intervals)
    sleep(Duration::from_millis(800)).await;

    // Each node should discover the other
    let peers1 = service1.get_peers();
    let peers2 = service2.get_peers();

    assert_eq!(
        peers1.len(),
        1,
        "Node 1 should discover node 2, found {} peers",
        peers1.len()
    );
    assert_eq!(
        peers2.len(),
        1,
        "Node 2 should discover node 1, found {} peers",
        peers2.len()
    );

    // Verify peer details
    if !peers1.is_empty() {
        assert_eq!(peers1[0].node_id, 2);
    }
    if !peers2.is_empty() {
        assert_eq!(peers2[0].node_id, 1);
    }

    service1.stop();
    service2.stop();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_three_node_cluster_discovery() {
    // Test three-node cluster auto-discovery
    let config1 = create_test_config(1, 18020, 3);
    let config2 = create_test_config(2, 18020, 3);
    let config3 = create_test_config(3, 18020, 3);

    let service1 = DiscoveryService::new(config1).unwrap();
    let service2 = DiscoveryService::new(config2).unwrap();
    let service3 = DiscoveryService::new(config3).unwrap();

    // Start all services
    service1.start().await.unwrap();
    service2.start().await.unwrap();
    service3.start().await.unwrap();

    // Wait for full discovery
    sleep(Duration::from_millis(1000)).await;

    // Each node should discover the other two
    let peers1 = service1.get_peers();
    let peers2 = service2.get_peers();
    let peers3 = service3.get_peers();

    assert_eq!(
        peers1.len(),
        2,
        "Node 1 should discover 2 peers, found {}",
        peers1.len()
    );
    assert_eq!(
        peers2.len(),
        2,
        "Node 2 should discover 2 peers, found {}",
        peers2.len()
    );
    assert_eq!(
        peers3.len(),
        2,
        "Node 3 should discover 2 peers, found {}",
        peers3.len()
    );

    service1.stop();
    service2.stop();
    service3.stop();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_node_joining_running_cluster() {
    // Start initial cluster with 2 nodes (will add 3rd later)
    let config1 = create_test_config(1, 18030, 3);
    let config2 = create_test_config(2, 18030, 3);

    let service1 = DiscoveryService::new(config1).unwrap();
    let service2 = DiscoveryService::new(config2).unwrap();

    service1.start().await.unwrap();
    service2.start().await.unwrap();

    // Wait for initial discovery
    sleep(Duration::from_millis(800)).await;

    // Verify initial cluster
    assert_eq!(service1.get_peers().len(), 1);
    assert_eq!(service2.get_peers().len(), 1);

    // Add a third node to running cluster
    let config3 = create_test_config(3, 18030, 3);
    let service3 = DiscoveryService::new(config3).unwrap();
    service3.start().await.unwrap();

    // Wait longer for all nodes to exchange heartbeats and discover each other
    // Need to wait for at least one heartbeat cycle (200ms) plus buffer
    sleep(Duration::from_millis(1500)).await;

    // All nodes should now see each other
    let peers1 = service1.get_peers();
    let peers2 = service2.get_peers();
    let peers3 = service3.get_peers();

    assert_eq!(
        peers1.len(),
        2,
        "Node 1 should see 2 peers after join, found {}",
        peers1.len()
    );
    assert_eq!(
        peers2.len(),
        2,
        "Node 2 should see 2 peers after join, found {}",
        peers2.len()
    );
    assert_eq!(
        peers3.len(),
        2,
        "Node 3 should see 2 peers after join, found {}",
        peers3.len()
    );

    service1.stop();
    service2.stop();
    service3.stop();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_failure_detection() {
    // Test that dead nodes are detected and removed
    let config1 = create_test_config(1, 18040, 2);
    let config2 = create_test_config(2, 18040, 2);

    let service1 = DiscoveryService::new(config1).unwrap();
    let service2 = DiscoveryService::new(config2).unwrap();

    // Start both services
    service1.start().await.unwrap();
    service2.start().await.unwrap();

    // Wait for discovery
    sleep(Duration::from_millis(800)).await;

    // Verify both nodes see each other
    assert_eq!(service1.get_peers().len(), 1);
    assert_eq!(service2.get_peers().len(), 1);

    // Stop node 2 (simulating failure)
    service2.stop();

    // Wait for failure detection (> failure_timeout_ms + check_interval)
    // Need to wait for heartbeat interval + failure timeout + some buffer
    sleep(Duration::from_millis(1200)).await;

    // Node 1 should no longer see node 2
    let peers1 = service1.get_peers();
    assert_eq!(
        peers1.len(),
        0,
        "Node 1 should detect node 2 failure, found {} peers",
        peers1.len()
    );

    service1.stop();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_peer_alive_check() {
    // Test the is_peer_alive functionality
    let config1 = create_test_config(1, 18050, 2);
    let config2 = create_test_config(2, 18050, 2);

    let service1 = DiscoveryService::new(config1).unwrap();
    let service2 = DiscoveryService::new(config2).unwrap();

    service1.start().await.unwrap();
    service2.start().await.unwrap();

    // Wait for discovery
    sleep(Duration::from_millis(800)).await;

    // Check that peer is alive
    assert!(
        service1.is_peer_alive(2),
        "Node 2 should be alive from node 1's perspective"
    );
    assert!(
        service2.is_peer_alive(1),
        "Node 1 should be alive from node 2's perspective"
    );

    // Stop node 2
    service2.stop();

    // Wait for timeout
    sleep(Duration::from_millis(800)).await;

    // Node 2 should no longer be alive
    assert!(
        !service1.is_peer_alive(2),
        "Node 2 should not be alive after stopping"
    );

    service1.stop();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_get_specific_peer() {
    // Test retrieving specific peer information
    let config1 = create_test_config(1, 18060, 3);
    let config2 = create_test_config(2, 18060, 3);
    let config3 = create_test_config(3, 18060, 3);

    let service1 = DiscoveryService::new(config1).unwrap();
    let service2 = DiscoveryService::new(config2).unwrap();
    let service3 = DiscoveryService::new(config3).unwrap();

    service1.start().await.unwrap();
    service2.start().await.unwrap();
    service3.start().await.unwrap();

    // Wait for discovery
    sleep(Duration::from_millis(1000)).await;

    // Get specific peer from node 1
    let peer2 = service1.get_peer(2);
    let peer3 = service1.get_peer(3);
    let peer_nonexistent = service1.get_peer(999);

    assert!(peer2.is_some(), "Should find peer 2");
    assert!(peer3.is_some(), "Should find peer 3");
    assert!(
        peer_nonexistent.is_none(),
        "Should not find non-existent peer"
    );

    if let Some(p2) = peer2 {
        assert_eq!(p2.node_id, 2);
        assert_eq!(p2.raft_addr.port(), 9002);
    }

    if let Some(p3) = peer3 {
        assert_eq!(p3.node_id, 3);
        assert_eq!(p3.raft_addr.port(), 9003);
    }

    service1.stop();
    service2.stop();
    service3.stop();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_heartbeat_maintains_peer() {
    // Test that continuous heartbeats keep peer alive
    let config1 = create_test_config(1, 18070, 2);
    let config2 = create_test_config(2, 18070, 2);

    let service1 = DiscoveryService::new(config1).unwrap();
    let service2 = DiscoveryService::new(config2).unwrap();

    service1.start().await.unwrap();
    service2.start().await.unwrap();

    // Wait for initial discovery
    sleep(Duration::from_millis(800)).await;

    assert_eq!(service1.get_peers().len(), 1);

    // Wait for several heartbeat cycles (should maintain peer)
    sleep(Duration::from_millis(1500)).await;

    // Peer should still be there
    assert_eq!(
        service1.get_peers().len(),
        1,
        "Peer should remain after heartbeats"
    );
    assert!(service1.is_peer_alive(2), "Peer 2 should still be alive");

    service1.stop();
    service2.stop();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_multiple_start_prevention() {
    // Test that service cannot be started twice
    let config = create_test_config(1, 18080, 1);
    let service = DiscoveryService::new(config).unwrap();

    // First start should succeed
    assert!(service.start().await.is_ok());

    // Wait a moment
    sleep(Duration::from_millis(100)).await;

    // Second start should fail
    let result = service.start().await;
    assert!(
        result.is_err(),
        "Second start should fail, but got: {:?}",
        result
    );

    service.stop();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_network_partition_simulation() {
    // Simulate a simple network partition scenario
    // Node 1 and 2 discover each other, then node 2 stops (partition),
    // then node 2 restarts (partition heals)

    let config1 = create_test_config(1, 18090, 2);
    let config2 = create_test_config(2, 18090, 2);

    let service1 = DiscoveryService::new(config1.clone()).unwrap();
    let service2 = DiscoveryService::new(config2.clone()).unwrap();

    // Initial discovery
    service1.start().await.unwrap();
    service2.start().await.unwrap();
    sleep(Duration::from_millis(800)).await;

    assert_eq!(service1.get_peers().len(), 1);

    // Simulate partition - stop node 2
    service2.stop();
    sleep(Duration::from_millis(1200)).await;

    // Node 1 should detect failure
    assert_eq!(
        service1.get_peers().len(),
        0,
        "Should detect partition/failure"
    );

    // Heal partition - restart node 2
    let config2_new = create_test_config(2, 18090, 2);
    let service2_new = DiscoveryService::new(config2_new).unwrap();
    service2_new.start().await.unwrap();
    sleep(Duration::from_millis(800)).await;

    // Nodes should rediscover each other
    assert_eq!(
        service1.get_peers().len(),
        1,
        "Should rediscover after partition heals"
    );

    service1.stop();
    service2_new.stop();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_discovery_config_values() {
    // Test constants to avoid hardcoded values
    const TEST_NODE_ID: u64 = 1;
    const TEST_RAFT_PORT: u16 = 9001;
    const TEST_CLIENT_PORT: u16 = 8001;
    const TEST_DISCOVERY_PORT: u16 = 18011;
    const TEST_IP: &str = "127.0.0.1";

    // Test that custom configuration values are respected
    let custom_heartbeat = 100;
    let custom_timeout = 300;

    let config = DiscoveryConfig {
        node_id: TEST_NODE_ID,
        raft_addr: format!("{}:{}", TEST_IP, TEST_RAFT_PORT).parse().unwrap(),
        client_addr: format!("{}:{}", TEST_IP, TEST_CLIENT_PORT).parse().unwrap(),
        discovery_port: TEST_DISCOVERY_PORT,
        broadcast_addr: TEST_IP.to_string(),
        seed_addrs: vec![TEST_IP.to_string()],
        heartbeat_interval_ms: custom_heartbeat,
        failure_timeout_ms: custom_timeout,
        cluster_secret: None,
    };

    assert_eq!(config.heartbeat_interval_ms, custom_heartbeat);
    assert_eq!(config.failure_timeout_ms, custom_timeout);

    let service = DiscoveryService::new(config).unwrap();
    service.start().await.unwrap();

    // Service should be running with custom config
    sleep(Duration::from_millis(200)).await;

    service.stop();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_default_discovery_config() {
    // Test default configuration
    let config = DiscoveryConfig::default();

    assert_eq!(config.node_id, 1);
    assert_eq!(config.discovery_port, 7946);
    assert_eq!(config.heartbeat_interval_ms, DEFAULT_HEARTBEAT_INTERVAL_MS);
    assert_eq!(config.broadcast_addr, "255.255.255.255");
}
