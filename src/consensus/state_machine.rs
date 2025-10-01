//! OpenRaft state machine implementation
//!
//! This module implements the RaftStateMachine trait for applying log entries
//! to the key-value store state machine.

// Allow large error types from OpenRaft - this is a library design choice
#![allow(clippy::result_large_err)]

use openraft::entry::RaftPayload;
use openraft::storage::RaftStateMachine;
use openraft::{
    LogId, RaftSnapshotBuilder, SnapshotMeta, StorageError, StorageIOError, StoredMembership,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::consensus::type_config::{AppRequest, AppResponse, TypeConfig};
use crate::types::{Key, NodeId, Value};

/// Snapshot data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    /// Last applied log id
    pub last_applied: Option<LogId<NodeId>>,
    /// Last membership configuration
    pub last_membership: StoredMembership<NodeId, openraft::BasicNode>,
    /// State machine data (key-value pairs)
    pub data: HashMap<Key, Value>,
}

/// State machine for the key-value store
pub struct StateMachine {
    /// Last applied log id
    last_applied: Option<LogId<NodeId>>,
    /// Last membership configuration
    last_membership: StoredMembership<NodeId, openraft::BasicNode>,
    /// In-memory key-value store
    data: HashMap<Key, Value>,
}

impl StateMachine {
    /// Create a new state machine
    pub fn new() -> Self {
        Self {
            last_applied: None,
            last_membership: StoredMembership::default(),
            data: HashMap::new(),
        }
    }

    /// Get a value from the state machine
    pub fn get(&self, key: &Key) -> Option<Value> {
        self.data.get(key).cloned()
    }

    /// Get all data from the state machine
    pub fn get_all(&self) -> HashMap<Key, Value> {
        self.data.clone()
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot builder for creating snapshots
pub struct SnapshotBuilder {
    snapshot_data: SnapshotData,
}

impl SnapshotBuilder {
    /// Create a new snapshot builder
    pub fn new(
        last_applied: Option<LogId<NodeId>>,
        last_membership: StoredMembership<NodeId, openraft::BasicNode>,
        data: HashMap<Key, Value>,
    ) -> Self {
        Self {
            snapshot_data: SnapshotData {
                last_applied,
                last_membership,
                data,
            },
        }
    }
}

impl RaftSnapshotBuilder<TypeConfig> for SnapshotBuilder {
    async fn build_snapshot(
        &mut self,
    ) -> Result<openraft::Snapshot<TypeConfig>, StorageError<NodeId>> {
        let snapshot_id = format!(
            "{:?}",
            self.snapshot_data
                .last_applied
                .as_ref()
                .map(|id| format!("{}-{}", id.leader_id, id.index))
                .unwrap_or_else(|| "none".to_string())
        );

        // Serialize snapshot data to bytes
        let data = bincode::serialize(&self.snapshot_data)
            .map_err(|e| StorageError::from(StorageIOError::write_snapshot(None, &e)))?;

        let snapshot_meta = SnapshotMeta {
            last_log_id: self.snapshot_data.last_applied,
            last_membership: self.snapshot_data.last_membership.clone(),
            snapshot_id: snapshot_id.clone(),
        };

        let cursor = Cursor::new(data);

        Ok(openraft::Snapshot {
            meta: snapshot_meta,
            snapshot: Box::new(cursor),
        })
    }
}

/// Thread-safe wrapper for state machine
pub struct StateMachineStore {
    inner: Arc<RwLock<StateMachine>>,
}

impl StateMachineStore {
    /// Create a new state machine store
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(StateMachine::new())),
        }
    }

    /// Get a value from the state machine
    pub async fn get(&self, key: &Key) -> Option<Value> {
        let sm = self.inner.read().await;
        sm.get(key)
    }

    /// Get all data from the state machine
    pub async fn get_all(&self) -> HashMap<Key, Value> {
        let sm = self.inner.read().await;
        sm.get_all()
    }
}

impl Default for StateMachineStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RaftStateMachine<TypeConfig> for StateMachineStore {
    type SnapshotBuilder = SnapshotBuilder;

    async fn applied_state(
        &mut self,
    ) -> Result<
        (
            Option<LogId<NodeId>>,
            StoredMembership<NodeId, openraft::BasicNode>,
        ),
        StorageError<NodeId>,
    > {
        let sm = self.inner.read().await;
        Ok((sm.last_applied, sm.last_membership.clone()))
    }

    async fn apply<I>(&mut self, entries: I) -> Result<Vec<AppResponse>, StorageError<NodeId>>
    where
        I: IntoIterator<Item = openraft::Entry<TypeConfig>> + Send,
        I::IntoIter: Send,
    {
        let mut sm = self.inner.write().await;
        let mut responses = Vec::new();

        for entry in entries {
            // Update last applied log id
            sm.last_applied = Some(entry.log_id);

            // Handle membership changes
            if let Some(membership) = entry.get_membership() {
                sm.last_membership = StoredMembership::new(Some(entry.log_id), membership.clone());
            }

            // Apply the log entry to state machine
            let response = match entry.payload {
                openraft::EntryPayload::Blank => AppResponse::PutOk,
                openraft::EntryPayload::Normal(ref req) => match req {
                    AppRequest::Put { key, value } => {
                        sm.data.insert(key.clone(), value.clone());
                        AppResponse::PutOk
                    }
                    AppRequest::Delete { key } => {
                        sm.data.remove(key);
                        AppResponse::DeleteOk
                    }
                },
                openraft::EntryPayload::Membership(_) => AppResponse::PutOk,
            };

            responses.push(response);
        }

        Ok(responses)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        let sm = self.inner.read().await;
        SnapshotBuilder::new(sm.last_applied, sm.last_membership.clone(), sm.data.clone())
    }

    async fn begin_receiving_snapshot(
        &mut self,
    ) -> Result<Box<Cursor<Vec<u8>>>, StorageError<NodeId>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<NodeId, openraft::BasicNode>,
        snapshot: Box<Cursor<Vec<u8>>>,
    ) -> Result<(), StorageError<NodeId>> {
        let data = snapshot.into_inner();
        let snapshot_data: SnapshotData = bincode::deserialize(&data).map_err(|e| {
            StorageError::from(StorageIOError::read_snapshot(Some(meta.signature()), &e))
        })?;

        let mut sm = self.inner.write().await;
        sm.last_applied = snapshot_data.last_applied;
        sm.last_membership = snapshot_data.last_membership;
        sm.data = snapshot_data.data;

        Ok(())
    }

    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<openraft::Snapshot<TypeConfig>>, StorageError<NodeId>> {
        // For now, we don't keep a persistent snapshot
        // This will be built on-demand by get_snapshot_builder
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openraft::{EntryPayload, LeaderId};

    #[tokio::test]
    async fn test_state_machine_apply_put() {
        let mut sm = StateMachineStore::new();

        let log_id = LogId::new(LeaderId::new(1, 1), 1);
        let entry = openraft::Entry {
            log_id,
            payload: EntryPayload::Normal(AppRequest::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec(),
            }),
        };

        let responses = sm.apply(vec![entry]).await.unwrap();
        assert_eq!(responses.len(), 1);
        assert!(matches!(responses[0], AppResponse::PutOk));

        let value = sm.get(&b"key1".to_vec()).await;
        assert_eq!(value, Some(b"value1".to_vec()));
    }

    #[tokio::test]
    async fn test_state_machine_apply_delete() {
        let mut sm = StateMachineStore::new();

        // First put a value
        let log_id1 = LogId::new(LeaderId::new(1, 1), 1);
        let entry1 = openraft::Entry {
            log_id: log_id1,
            payload: EntryPayload::Normal(AppRequest::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec(),
            }),
        };
        sm.apply(vec![entry1]).await.unwrap();

        // Then delete it
        let log_id2 = LogId::new(LeaderId::new(1, 1), 2);
        let entry2 = openraft::Entry {
            log_id: log_id2,
            payload: EntryPayload::Normal(AppRequest::Delete {
                key: b"key1".to_vec(),
            }),
        };

        let responses = sm.apply(vec![entry2]).await.unwrap();
        assert_eq!(responses.len(), 1);
        assert!(matches!(responses[0], AppResponse::DeleteOk));

        let value = sm.get(&b"key1".to_vec()).await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_state_machine_applied_state() {
        let mut sm = StateMachineStore::new();

        let (last_applied, _) = sm.applied_state().await.unwrap();
        assert_eq!(last_applied, None);

        let log_id = LogId::new(LeaderId::new(1, 1), 1);
        let entry = openraft::Entry {
            log_id,
            payload: EntryPayload::Normal(AppRequest::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec(),
            }),
        };

        sm.apply(vec![entry]).await.unwrap();

        let (last_applied, _) = sm.applied_state().await.unwrap();
        assert_eq!(last_applied, Some(log_id));
    }

    #[tokio::test]
    async fn test_snapshot_builder() {
        let mut sm = StateMachineStore::new();

        // Apply some entries
        let log_id = LogId::new(LeaderId::new(1, 1), 1);
        let entry = openraft::Entry {
            log_id,
            payload: EntryPayload::Normal(AppRequest::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec(),
            }),
        };
        sm.apply(vec![entry]).await.unwrap();

        // Build snapshot
        let mut builder = sm.get_snapshot_builder().await;
        let snapshot = builder.build_snapshot().await.unwrap();

        assert_eq!(snapshot.meta.last_log_id, Some(log_id));
    }

    #[tokio::test]
    async fn test_install_snapshot() {
        let mut sm = StateMachineStore::new();

        // Create snapshot data
        let log_id = LogId::new(LeaderId::new(1, 1), 5);
        let mut data = HashMap::new();
        data.insert(b"key1".to_vec(), b"value1".to_vec());
        data.insert(b"key2".to_vec(), b"value2".to_vec());

        let snapshot_data = SnapshotData {
            last_applied: Some(log_id),
            last_membership: StoredMembership::default(),
            data,
        };

        let bytes = bincode::serialize(&snapshot_data).unwrap();
        let cursor = Box::new(Cursor::new(bytes));

        let meta = SnapshotMeta {
            last_log_id: Some(log_id),
            last_membership: StoredMembership::default(),
            snapshot_id: "test-snapshot".to_string(),
        };

        sm.install_snapshot(&meta, cursor).await.unwrap();

        let value1 = sm.get(&b"key1".to_vec()).await;
        assert_eq!(value1, Some(b"value1".to_vec()));

        let value2 = sm.get(&b"key2".to_vec()).await;
        assert_eq!(value2, Some(b"value2".to_vec()));

        let (last_applied, _) = sm.applied_state().await.unwrap();
        assert_eq!(last_applied, Some(log_id));
    }
}
