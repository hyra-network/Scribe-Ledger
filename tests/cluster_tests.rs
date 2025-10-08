//! Integration tests for cluster initialization
//!
//! These tests verify cluster bootstrapping, auto-joining, and coordination
//! between discovery and consensus layers.

use hyra_scribe_ledger::cluster::{ClusterConfig, ClusterInitializer, InitMode};
use hyra_scribe_ledger::consensus::ConsensusNode;
use hyra_scribe_ledger::discovery::{DiscoveryConfig, DiscoveryService};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Helper to create discovery config for testing
fn create_discovery_config(node_id: u64, base_port: u16, num_nodes: u64) -> DiscoveryConfig {
    let my_port = base_port + node_id as u16;
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
        seed_addrs,
        heartbeat_interval_ms: 200,
        failure_timeout_ms: 600,
    }
}

#[tokio::test]
async fn test_bootstrap_single_node() {
    // Test bootstrapping a single-node cluster
    let discovery_config = create_discovery_config(1, 19001, 1);
    let discovery = Arc::new(DiscoveryService::new(discovery_config).unwrap());

    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    let cluster_config = ClusterConfig {
        mode: InitMode::Bootstrap,
        seed_addrs: Vec::new(),
        discovery_timeout_ms: 1000,
        min_peers_for_join: 1,
    };

    let initializer = ClusterInitializer::new(discovery.clone(), consensus.clone(), cluster_config);

    // Start discovery
    discovery.start().await.unwrap();

    // Bootstrap cluster
    let result = initializer.initialize().await;
    assert!(result.is_ok(), "Bootstrap should succeed: {:?}", result);

    // Wait for leader election
    sleep(Duration::from_millis(2000)).await;

    // Node should be leader
    assert!(
        consensus.is_leader().await,
        "Bootstrap node should become leader"
    );

    // Cleanup
    discovery.stop();
    consensus.shutdown().await.unwrap();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_join_mode_fallback_to_bootstrap() {
    // Test that join mode falls back to bootstrap when no peers are found
    let discovery_config = create_discovery_config(1, 19010, 1);
    let discovery = Arc::new(DiscoveryService::new(discovery_config).unwrap());

    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    let cluster_config = ClusterConfig {
        mode: InitMode::Join,
        seed_addrs: Vec::new(),
        discovery_timeout_ms: 1000, // Short timeout
        min_peers_for_join: 1,
    };

    let initializer = ClusterInitializer::new(discovery.clone(), consensus.clone(), cluster_config);

    // Start discovery
    discovery.start().await.unwrap();

    // Initialize - should fall back to bootstrap since no peers
    let result = initializer.initialize().await;
    assert!(
        result.is_ok(),
        "Join should fall back to bootstrap: {:?}",
        result
    );

    // Wait for leader election
    sleep(Duration::from_millis(2000)).await;

    // Node should be leader (since it bootstrapped)
    assert!(
        consensus.is_leader().await,
        "Node should become leader after fallback bootstrap"
    );

    // Cleanup
    discovery.stop();
    consensus.shutdown().await.unwrap();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_discover_peers_before_join() {
    // Test that node waits for peer discovery before attempting to join
    let discovery_config1 = create_discovery_config(1, 19020, 2);
    let discovery_config2 = create_discovery_config(2, 19020, 2);

    let discovery1 = Arc::new(DiscoveryService::new(discovery_config1).unwrap());
    let discovery2 = Arc::new(DiscoveryService::new(discovery_config2).unwrap());

    let db1 = sled::Config::new().temporary(true).open().unwrap();
    let db2 = sled::Config::new().temporary(true).open().unwrap();

    let consensus1 = Arc::new(ConsensusNode::new(1, db1).await.unwrap());
    let consensus2 = Arc::new(ConsensusNode::new(2, db2).await.unwrap());

    // Start both discovery services
    discovery1.start().await.unwrap();
    discovery2.start().await.unwrap();

    // Wait for discovery
    sleep(Duration::from_millis(1000)).await;

    // Both should have discovered each other
    let peers1 = discovery1.get_peers();
    let peers2 = discovery2.get_peers();

    assert_eq!(peers1.len(), 1, "Node 1 should discover node 2");
    assert_eq!(peers2.len(), 1, "Node 2 should discover node 1");

    // Cleanup
    discovery1.stop();
    discovery2.stop();
    consensus1.shutdown().await.unwrap();
    consensus2.shutdown().await.unwrap();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_cluster_config_default() {
    let config = ClusterConfig::default();
    assert_eq!(config.mode, InitMode::Join);
    assert_eq!(config.min_peers_for_join, 1);
    assert!(config.seed_addrs.is_empty());
}

#[tokio::test]
async fn test_bootstrap_mode_configuration() {
    let config = ClusterConfig {
        mode: InitMode::Bootstrap,
        seed_addrs: vec!["127.0.0.1:9001".to_string()],
        discovery_timeout_ms: 2000,
        min_peers_for_join: 0,
    };

    assert_eq!(config.mode, InitMode::Bootstrap);
    assert_eq!(config.discovery_timeout_ms, 2000);
    assert_eq!(config.seed_addrs.len(), 1);
}

#[tokio::test]
async fn test_join_mode_configuration() {
    let config = ClusterConfig {
        mode: InitMode::Join,
        seed_addrs: vec!["127.0.0.1:9001".to_string(), "127.0.0.1:9002".to_string()],
        discovery_timeout_ms: 5000,
        min_peers_for_join: 2,
    };

    assert_eq!(config.mode, InitMode::Join);
    assert_eq!(config.min_peers_for_join, 2);
    assert_eq!(config.seed_addrs.len(), 2);
}

#[tokio::test]
async fn test_manual_seed_addresses() {
    // Test that manual seed addresses are properly configured
    let seed_addrs = vec![
        "127.0.0.1:9001".to_string(),
        "127.0.0.1:9002".to_string(),
        "127.0.0.1:9003".to_string(),
    ];

    let cluster_config = ClusterConfig {
        mode: InitMode::Join,
        seed_addrs: seed_addrs.clone(),
        discovery_timeout_ms: 1000,
        min_peers_for_join: 2,
    };

    assert_eq!(cluster_config.seed_addrs, seed_addrs);
}

#[tokio::test]
async fn test_initialization_with_timeout() {
    // Test that initialization respects timeout settings
    let discovery_config = create_discovery_config(1, 19030, 1);
    let discovery = Arc::new(DiscoveryService::new(discovery_config).unwrap());

    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    let cluster_config = ClusterConfig {
        mode: InitMode::Join,
        seed_addrs: Vec::new(),
        discovery_timeout_ms: 500, // Very short timeout
        min_peers_for_join: 5,     // Impossible to reach
    };

    let initializer = ClusterInitializer::new(discovery.clone(), consensus.clone(), cluster_config);

    discovery.start().await.unwrap();

    let start = std::time::Instant::now();
    let result = initializer.initialize().await;
    let elapsed = start.elapsed();

    // Should complete (fall back to bootstrap) within reasonable time
    assert!(
        elapsed < Duration::from_millis(2000),
        "Should timeout quickly and fall back"
    );
    assert!(result.is_ok(), "Should succeed after fallback");

    // Cleanup
    discovery.stop();
    consensus.shutdown().await.unwrap();
    sleep(Duration::from_millis(100)).await;
}

#[tokio::test]
async fn test_handle_partition() {
    // Test graceful handling of network partitions
    let discovery_config = create_discovery_config(1, 19040, 1);
    let discovery = Arc::new(DiscoveryService::new(discovery_config).unwrap());

    let db = sled::Config::new().temporary(true).open().unwrap();
    let consensus = Arc::new(ConsensusNode::new(1, db).await.unwrap());

    let cluster_config = ClusterConfig {
        mode: InitMode::Bootstrap,
        seed_addrs: Vec::new(),
        discovery_timeout_ms: 1000,
        min_peers_for_join: 1,
    };

    let initializer = ClusterInitializer::new(discovery.clone(), consensus.clone(), cluster_config);

    discovery.start().await.unwrap();
    initializer.initialize().await.unwrap();

    // Simulate partition handling
    let result = initializer.handle_partition().await;
    assert!(result.is_ok(), "Partition handling should not fail");

    // Cleanup
    discovery.stop();
    consensus.shutdown().await.unwrap();
    sleep(Duration::from_millis(100)).await;
}
