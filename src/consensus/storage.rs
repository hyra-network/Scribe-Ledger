//! OpenRaft storage backend implementation
//!
//! This module implements RaftLogStorage and RaftLogReader traits for persistent
//! storage of Raft log entries, hard state (vote), and metadata using Sled.

// Allow large error types from OpenRaft - this is a library design choice
#![allow(clippy::result_large_err)]

use openraft::storage::{LogFlushed, RaftLogStorage};
use openraft::{
    LogId, LogState, RaftLogReader, StorageError, StorageIOError, Vote,
};
use std::fmt::Debug;
use std::ops::RangeBounds;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::consensus::state_machine::StateMachineStore;
use crate::consensus::type_config::TypeConfig;
use crate::types::NodeId;

/// Storage for Raft log and hard state
pub struct RaftStorage {
    /// Sled database for persistent storage
    db: sled::Db,
    /// In-memory state machine
    state_machine: Arc<RwLock<StateMachineStore>>,
}

impl RaftStorage {
    /// Create a new RaftStorage instance
    pub fn new(db: sled::Db) -> Self {
        Self {
            db,
            state_machine: Arc::new(RwLock::new(StateMachineStore::new())),
        }
    }

    /// Get the state machine
    pub fn state_machine(&self) -> Arc<RwLock<StateMachineStore>> {
        Arc::clone(&self.state_machine)
    }

    /// Tree names for different types of data
    const TREE_LOGS: &'static str = "logs";
    const TREE_VOTE: &'static str = "vote";
    const TREE_STATE: &'static str = "state";

    /// Keys for metadata
    const KEY_LAST_PURGED: &'static [u8] = b"last_purged";
    const KEY_VOTE: &'static [u8] = b"vote";
    const KEY_COMMITTED: &'static [u8] = b"committed";

    /// Get the logs tree
    fn logs(&self) -> Result<sled::Tree, StorageError<NodeId>> {
        self.db
            .open_tree(Self::TREE_LOGS)
            .map_err(|e| StorageError::from(StorageIOError::read(&e)))
    }

    /// Get the vote tree
    fn vote_tree(&self) -> Result<sled::Tree, StorageError<NodeId>> {
        self.db
            .open_tree(Self::TREE_VOTE)
            .map_err(|e| StorageError::from(StorageIOError::read(&e)))
    }

    /// Get the state tree
    fn state_tree(&self) -> Result<sled::Tree, StorageError<NodeId>> {
        self.db
            .open_tree(Self::TREE_STATE)
            .map_err(|e| StorageError::from(StorageIOError::read(&e)))
    }

    /// Convert log index to key
    fn log_key(index: u64) -> Vec<u8> {
        index.to_be_bytes().to_vec()
    }

}

/// Log reader for reading log entries
#[derive(Clone)]
pub struct LogReader {
    db: sled::Db,
}

impl LogReader {
    fn new(db: sled::Db) -> Self {
        Self { db }
    }

    fn logs(&self) -> Result<sled::Tree, StorageError<NodeId>> {
        self.db
            .open_tree(RaftStorage::TREE_LOGS)
            .map_err(|e| StorageError::from(StorageIOError::read(&e)))
    }
}

impl RaftLogReader<TypeConfig> for LogReader {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug + Send>(
        &mut self,
        range: RB,
    ) -> Result<Vec<openraft::Entry<TypeConfig>>, StorageError<NodeId>> {
        let logs = self.logs()?;

        let start = match range.start_bound() {
            std::ops::Bound::Included(&n) => n,
            std::ops::Bound::Excluded(&n) => n + 1,
            std::ops::Bound::Unbounded => 0,
        };

        let end = match range.end_bound() {
            std::ops::Bound::Included(&n) => n + 1,
            std::ops::Bound::Excluded(&n) => n,
            std::ops::Bound::Unbounded => u64::MAX,
        };

        let mut entries = Vec::new();
        for index in start..end {
            let key = RaftStorage::log_key(index);
            if let Some(value) = logs.get(&key).map_err(|e| StorageError::from(StorageIOError::read_logs(&e)))? {
                let entry: openraft::Entry<TypeConfig> =
                    bincode::deserialize(&value).map_err(|e| StorageError::from(StorageIOError::read_logs(&e)))?;
                entries.push(entry);
            } else {
                break;
            }
        }

        Ok(entries)
    }
}

impl RaftLogReader<TypeConfig> for RaftStorage {
    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug + Send>(
        &mut self,
        range: RB,
    ) -> Result<Vec<openraft::Entry<TypeConfig>>, StorageError<NodeId>> {
        let mut reader = LogReader::new(self.db.clone());
        reader.try_get_log_entries(range).await
    }
}

impl RaftLogStorage<TypeConfig> for RaftStorage {
    type LogReader = LogReader;

    async fn get_log_state(&mut self) -> Result<LogState<TypeConfig>, StorageError<NodeId>> {
        let logs = self.logs()?;
        let state = self.state_tree()?;

        // Get last purged log id
        let last_purged = state
            .get(Self::KEY_LAST_PURGED)
            .map_err(|e| StorageError::from(StorageIOError::read_logs(&e)))?
            .map(|v| {
                bincode::deserialize::<LogId<NodeId>>(&v).map_err(|e| StorageError::from(StorageIOError::read_logs(&e)))
            })
            .transpose()?;

        // Get last log entry
        let last_log_id = if let Some((_key, value)) = logs.last().map_err(|e| StorageError::from(StorageIOError::read_logs(&e)))? {
            let entry: openraft::Entry<TypeConfig> =
                bincode::deserialize(&value).map_err(|e| StorageError::from(StorageIOError::read_logs(&e)))?;
            Some(entry.log_id)
        } else {
            last_purged
        };

        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id,
        })
    }

    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> Result<(), StorageError<NodeId>> {
        let vote_tree = self.vote_tree()?;
        let encoded = bincode::serialize(vote).map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        vote_tree
            .insert(Self::KEY_VOTE, encoded)
            .map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        vote_tree.flush().map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<NodeId>>, StorageError<NodeId>> {
        let vote_tree = self.vote_tree()?;
        vote_tree
            .get(Self::KEY_VOTE)
            .map_err(|e| StorageError::from(StorageIOError::read_logs(&e)))?
            .map(|v| bincode::deserialize(&v).map_err(|e| StorageError::from(StorageIOError::read_logs(&e))))
            .transpose()
    }

    async fn save_committed(
        &mut self,
        committed: Option<LogId<NodeId>>,
    ) -> Result<(), StorageError<NodeId>> {
        let state = self.state_tree()?;
        if let Some(log_id) = committed {
            let encoded = bincode::serialize(&log_id).map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
            state
                .insert(Self::KEY_COMMITTED, encoded)
                .map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        } else {
            state
                .remove(Self::KEY_COMMITTED)
                .map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        }
        state.flush().map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        Ok(())
    }

    async fn read_committed(&mut self) -> Result<Option<LogId<NodeId>>, StorageError<NodeId>> {
        let state = self.state_tree()?;
        state
            .get(Self::KEY_COMMITTED)
            .map_err(|e| StorageError::from(StorageIOError::read_logs(&e)))?
            .map(|v| bincode::deserialize(&v).map_err(|e| StorageError::from(StorageIOError::read_logs(&e))))
            .transpose()
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        LogReader::new(self.db.clone())
    }

    async fn append<I>(
        &mut self,
        entries: I,
        callback: LogFlushed<TypeConfig>,
    ) -> Result<(), StorageError<NodeId>>
    where
        I: IntoIterator<Item = openraft::Entry<TypeConfig>> + Send,
        I::IntoIter: Send,
    {
        let logs = self.logs()?;

        for entry in entries {
            let key = Self::log_key(entry.log_id.index);
            let value = bincode::serialize(&entry).map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
            logs.insert(key, value).map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        }

        // Flush to disk
        logs.flush().map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;

        // Call the callback to signal that entries are persisted
        callback.log_io_completed(Ok(()));

        Ok(())
    }

    async fn truncate(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let logs = self.logs()?;

        // Remove all logs from log_id.index onwards
        let keys_to_remove: Vec<_> = logs
            .range(Self::log_key(log_id.index)..)
            .filter_map(|r| r.ok().map(|(k, _)| k))
            .collect();

        for key in keys_to_remove {
            logs.remove(key).map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        }

        logs.flush().map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        Ok(())
    }

    async fn purge(&mut self, log_id: LogId<NodeId>) -> Result<(), StorageError<NodeId>> {
        let logs = self.logs()?;
        let state = self.state_tree()?;

        // Remove all logs up to and including log_id.index
        let keys_to_remove: Vec<_> = logs
            .range(..=Self::log_key(log_id.index))
            .filter_map(|r| r.ok().map(|(k, _)| k))
            .collect();

        for key in keys_to_remove {
            logs.remove(key).map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        }

        // Update last purged log id
        let encoded = bincode::serialize(&log_id).map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        state
            .insert(Self::KEY_LAST_PURGED, encoded)
            .map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;

        logs.flush().map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        state.flush().map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openraft::{EntryPayload, LeaderId};
    use crate::consensus::type_config::AppRequest;

    fn create_test_storage() -> RaftStorage {
        let db = sled::Config::new().temporary(true).open().unwrap();
        RaftStorage::new(db)
    }

    // Helper to manually insert log entries for testing without using append callback
    async fn test_insert_log(storage: &RaftStorage, entry: openraft::Entry<TypeConfig>) -> Result<(), StorageError<NodeId>> {
        let logs = storage.logs()?;
        let key = RaftStorage::log_key(entry.log_id.index);
        let value = bincode::serialize(&entry).map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        logs.insert(key, value).map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        logs.flush().map_err(|e| StorageError::from(StorageIOError::write_logs(&e)))?;
        Ok(())
    }

    #[tokio::test]
    async fn test_save_and_read_vote() {
        let mut storage = create_test_storage();

        let vote = Vote::new(1, 1u64);
        storage.save_vote(&vote).await.unwrap();

        let read_vote = storage.read_vote().await.unwrap();
        assert_eq!(read_vote, Some(vote));
    }

    #[tokio::test]
    async fn test_append_and_read_logs() {
        let mut storage = create_test_storage();

        let log_id1 = LogId::new(LeaderId::new(1, 1), 1);
        let entry1 = openraft::Entry {
            log_id: log_id1,
            payload: EntryPayload::Normal(AppRequest::Put {
                key: b"key1".to_vec(),
                value: b"value1".to_vec(),
            }),
        };

        let log_id2 = LogId::new(LeaderId::new(1, 1), 2);
        let entry2 = openraft::Entry {
            log_id: log_id2,
            payload: EntryPayload::Normal(AppRequest::Put {
                key: b"key2".to_vec(),
                value: b"value2".to_vec(),
            }),
        };

        test_insert_log(&storage, entry1).await.unwrap();
        test_insert_log(&storage, entry2).await.unwrap();

        let mut reader = storage.get_log_reader().await;
        let entries = reader.try_get_log_entries(1..3).await.unwrap();
        
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].log_id, log_id1);
        assert_eq!(entries[1].log_id, log_id2);
    }

    #[tokio::test]
    async fn test_get_log_state() {
        let mut storage = create_test_storage();

        let state = storage.get_log_state().await.unwrap();
        assert_eq!(state.last_log_id, None);
        assert_eq!(state.last_purged_log_id, None);

        // Insert a log
        let log_id = LogId::new(LeaderId::new(1, 1), 1);
        let entry = openraft::Entry {
            log_id,
            payload: EntryPayload::Blank,
        };

        test_insert_log(&storage, entry).await.unwrap();

        let state = storage.get_log_state().await.unwrap();
        assert_eq!(state.last_log_id, Some(log_id));
    }

    #[tokio::test]
    async fn test_truncate() {
        let mut storage = create_test_storage();

        // Insert logs 1, 2, 3
        for i in 1..=3 {
            let log_id = LogId::new(LeaderId::new(1, 1), i);
            let entry = openraft::Entry {
                log_id,
                payload: EntryPayload::Blank,
            };
            test_insert_log(&storage, entry).await.unwrap();
        }

        // Truncate from index 2
        let log_id = LogId::new(LeaderId::new(1, 1), 2);
        storage.truncate(log_id).await.unwrap();

        // Should only have log 1 now
        let mut reader = storage.get_log_reader().await;
        let entries = reader.try_get_log_entries(1..10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].log_id.index, 1);
    }

    #[tokio::test]
    async fn test_purge() {
        let mut storage = create_test_storage();

        // Insert logs 1, 2, 3
        for i in 1..=3 {
            let log_id = LogId::new(LeaderId::new(1, 1), i);
            let entry = openraft::Entry {
                log_id,
                payload: EntryPayload::Blank,
            };
            test_insert_log(&storage, entry).await.unwrap();
        }

        // Purge up to index 2
        let log_id = LogId::new(LeaderId::new(1, 1), 2);
        storage.purge(log_id).await.unwrap();

        // Should only have log 3 now
        let mut reader = storage.get_log_reader().await;
        let entries = reader.try_get_log_entries(3..10).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].log_id.index, 3);

        // Check that last_purged is set
        let state = storage.get_log_state().await.unwrap();
        assert_eq!(state.last_purged_log_id, Some(log_id));
    }

    #[tokio::test]
    async fn test_save_and_read_committed() {
        let mut storage = create_test_storage();

        let log_id = LogId::new(LeaderId::new(1, 1), 5);
        storage.save_committed(Some(log_id)).await.unwrap();

        let committed = storage.read_committed().await.unwrap();
        assert_eq!(committed, Some(log_id));
    }
}
