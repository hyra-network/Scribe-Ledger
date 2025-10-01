//! Consensus module for distributed consensus using OpenRaft
//!
//! This module contains the Raft consensus implementation for the distributed ledger.

pub mod state_machine;
pub mod storage;
pub mod type_config;

pub use state_machine::{SnapshotBuilder, StateMachine, StateMachineStore};
pub use storage::{LogReader, RaftStorage};
pub use type_config::{AppRequest, AppResponse, TypeConfig};
