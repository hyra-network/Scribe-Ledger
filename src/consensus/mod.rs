/// Raft consensus implementation for Scribe Ledger
use raft::{prelude::*, storage::MemStorage, Config as RaftConfig, RawNode};
use slog::{Drain, Logger};
use crate::error::{Result, ScribeError};
use crate::types::Manifest;

/// Consensus node managing the global manifest
pub struct ConsensusNode {
    raft_node: RawNode<MemStorage>,
    manifest: Manifest,
    #[allow(dead_code)]
    node_id: u64,
}

impl ConsensusNode {
    /// Create a new consensus node
    pub fn new(node_id: u64, _peers: Vec<u64>) -> Result<Self> {
        let config = RaftConfig {
            id: node_id,
            election_tick: 10,
            heartbeat_tick: 3,
            ..Default::default()
        };
        
        let storage = MemStorage::new();
        
        // Create a simple logger for raft
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        let logger = Logger::root(drain, slog::o!());
        
        let raft_node = RawNode::new(&config, storage, &logger)
            .map_err(|e| ScribeError::Consensus(format!("Failed to create Raft node: {}", e)))?;
        
        Ok(Self {
            raft_node,
            manifest: Manifest::new(),
            node_id,
        })
    }
    
    /// Process a Raft tick
    pub fn tick(&mut self) {
        self.raft_node.tick();
    }
    
    /// Propose a manifest update
    pub fn propose_manifest_update(&mut self, manifest: Manifest) -> Result<()> {
        let data = serde_json::to_vec(&manifest)?;
        self.raft_node.propose(vec![], data)
            .map_err(|e| ScribeError::Consensus(format!("Failed to propose: {}", e)))?;
        Ok(())
    }
    
    /// Get the current manifest
    pub fn get_manifest(&self) -> &Manifest {
        &self.manifest
    }
    
    /// Handle received messages
    pub fn handle_message(&mut self, msg: Message) -> Result<()> {
        self.raft_node.step(msg)
            .map_err(|e| ScribeError::Consensus(format!("Failed to step: {}", e)))?;
        Ok(())
    }
    
    /// Process ready state
    pub fn process_ready(&mut self) -> Result<()> {
        if !self.raft_node.has_ready() {
            return Ok(());
        }
        
        let ready = self.raft_node.ready();
        
        // Handle committed entries
        for entry in ready.committed_entries() {
            if !entry.data.is_empty() {
                // Apply manifest update
                if let Ok(manifest) = serde_json::from_slice::<Manifest>(&entry.data) {
                    self.manifest = manifest;
                    tracing::info!("Applied manifest update, version: {}", self.manifest.version);
                }
            }
        }
        
        self.raft_node.advance(ready);
        Ok(())
    }
}