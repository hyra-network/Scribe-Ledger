use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, RwLock};
use tracing::{error, info, debug};

/// Unique identifier for Raft events
pub type EventId = u64;

/// Timestamp in Unix epoch seconds
pub type Timestamp = u64;

/// Raft monitoring event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftEvent {
    /// Log entry committed to the Raft log
    LogCommitted {
        /// Raft log index
        index: u64,
        /// Raft term when committed
        term: u64,
        /// Size of the committed entry in bytes
        entry_size: usize,
        /// Node ID that committed the entry
        node_id: u64,
    },
    
    /// Log entry applied to state machine
    LogApplied {
        /// Raft log index
        index: u64,
        /// Raft term
        term: u64,
        /// Time taken to apply entry (microseconds)
        apply_duration_us: u64,
        /// Node ID that applied the entry
        node_id: u64,
    },
    
    /// Leader election started
    LeaderElectionStarted {
        /// Election term
        term: u64,
        /// Node ID starting election
        candidate_id: u64,
        /// Previous leader ID (if known)
        previous_leader: Option<u64>,
    },
    
    /// Leader election completed
    LeaderElectionCompleted {
        /// Election term
        term: u64,
        /// Elected leader ID
        leader_id: u64,
        /// Election duration in milliseconds
        election_duration_ms: u64,
        /// Number of votes received
        votes_received: u32,
        /// Total voters in cluster
        total_voters: u32,
    },
    
    /// Leader changed
    LeaderChanged {
        /// Previous leader ID
        previous_leader: Option<u64>,
        /// New leader ID
        new_leader: u64,
        /// Term of leadership change
        term: u64,
    },
    
    /// Node joined cluster
    NodeJoined {
        /// Joining node ID
        node_id: u64,
        /// Node address
        address: String,
        /// Current cluster size after join
        cluster_size: usize,
    },
    
    /// Node left cluster
    NodeLeft {
        /// Leaving node ID
        node_id: u64,
        /// Reason for leaving
        reason: String,
        /// Current cluster size after leave
        cluster_size: usize,
    },
    
    /// Heartbeat sent/received
    Heartbeat {
        /// From node ID
        from: u64,
        /// To node ID
        to: u64,
        /// Heartbeat sequence number
        sequence: u64,
        /// Success/failure
        success: bool,
        /// Response time in microseconds (if successful)
        response_time_us: Option<u64>,
    },
    
    /// Log compaction event
    LogCompaction {
        /// Entries removed from log
        entries_removed: u64,
        /// New log start index
        new_start_index: u64,
        /// Snapshot created at index
        snapshot_index: u64,
        /// Compaction duration in milliseconds
        duration_ms: u64,
    },
    
    /// Configuration change
    ConfigChange {
        /// Type of change
        change_type: ConfigChangeType,
        /// Configuration before change
        old_config: Vec<u64>,
        /// Configuration after change
        new_config: Vec<u64>,
        /// Index where change was applied
        index: u64,
    },
    
    /// Performance metrics snapshot
    MetricsSnapshot {
        /// Current Raft metrics
        metrics: RaftMetrics,
    },
}

/// Types of configuration changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigChangeType {
    AddNode,
    RemoveNode,
    ReplaceNode,
    JointConsensus,
}

/// Comprehensive Raft performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftMetrics {
    /// Current node ID
    pub node_id: u64,
    /// Current Raft term
    pub current_term: u64,
    /// Current leader ID
    pub leader_id: Option<u64>,
    /// Is this node the leader
    pub is_leader: bool,
    /// Log index of last committed entry
    pub commit_index: u64,
    /// Log index of last applied entry
    pub last_applied: u64,
    /// Total log size in entries
    pub log_size: u64,
    /// Log size in bytes
    pub log_size_bytes: u64,
    /// Cluster size
    pub cluster_size: usize,
    /// Number of active followers (leader only)
    pub active_followers: u32,
    /// Average commit latency in microseconds
    pub avg_commit_latency_us: u64,
    /// Average apply latency in microseconds
    pub avg_apply_latency_us: u64,
    /// Elections count since startup
    pub elections_count: u64,
    /// Leadership changes count
    pub leadership_changes: u64,
    /// Heartbeat success rate (0.0 - 1.0)
    pub heartbeat_success_rate: f64,
    /// Messages sent per second
    pub messages_sent_per_sec: f64,
    /// Messages received per second  
    pub messages_received_per_sec: f64,
    /// Last metrics update timestamp
    pub timestamp: Timestamp,
}

/// Complete monitoring event with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringEvent {
    /// Unique event ID
    pub id: EventId,
    /// Event timestamp
    pub timestamp: Timestamp,
    /// Node ID that generated the event
    pub node_id: u64,
    /// The actual Raft event
    pub event: RaftEvent,
    /// Event severity level
    pub severity: EventSeverity,
    /// Optional additional context
    pub context: HashMap<String, String>,
}

/// Event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

/// Real-time event broadcasting and monitoring system
pub struct RaftMonitor {
    /// Node ID for this monitor
    node_id: u64,
    /// Event counter for unique IDs
    event_counter: Arc<RwLock<u64>>,
    /// Broadcast channel for real-time events
    event_broadcaster: broadcast::Sender<MonitoringEvent>,
    /// Event history storage (limited size)
    event_history: Arc<RwLock<Vec<MonitoringEvent>>>,
    /// Maximum events to keep in history
    max_history_size: usize,
    /// Current metrics
    current_metrics: Arc<RwLock<RaftMetrics>>,
    /// Performance tracking
    performance_tracker: Arc<RwLock<PerformanceTracker>>,
}

/// Performance tracking data
#[derive(Debug, Clone)]
pub struct PerformanceTracker {
    /// Commit latency samples (microseconds)
    pub commit_latencies: Vec<u64>,
    /// Apply latency samples (microseconds)
    pub apply_latencies: Vec<u64>,
    /// Election durations (milliseconds)
    pub election_durations: Vec<u64>,
    /// Heartbeat response times (microseconds)
    pub heartbeat_times: Vec<u64>,
    /// Message counts
    pub messages_sent: u64,
    pub messages_received: u64,
    /// Tracking window start time
    pub window_start: Timestamp,
    /// Maximum samples to keep
    pub max_samples: usize,
}

impl RaftMonitor {
    /// Create a new Raft monitor
    pub fn new(node_id: u64, max_history_size: usize) -> Self {
        let (tx, _) = broadcast::channel(1000); // Buffer up to 1000 events
        
        Self {
            node_id,
            event_counter: Arc::new(RwLock::new(0)),
            event_broadcaster: tx,
            event_history: Arc::new(RwLock::new(Vec::new())),
            max_history_size,
            current_metrics: Arc::new(RwLock::new(RaftMetrics::default(node_id))),
            performance_tracker: Arc::new(RwLock::new(PerformanceTracker::new())),
        }
    }
    
    /// Publish a Raft event
    pub async fn publish_event(&self, event: RaftEvent, severity: EventSeverity) {
        let event_id = {
            let mut counter = self.event_counter.write().await;
            *counter += 1;
            *counter
        };
        
        let monitoring_event = MonitoringEvent {
            id: event_id,
            timestamp: current_timestamp(),
            node_id: self.node_id,
            event: event.clone(),
            severity: severity.clone(),
            context: HashMap::new(),
        };
        
        // Update performance metrics based on event
        self.update_metrics_from_event(&event).await;
        
        // Broadcast to real-time subscribers
        if let Err(_) = self.event_broadcaster.send(monitoring_event.clone()) {
            debug!("No active subscribers for event broadcast");
        }
        
        // Store in history
        self.store_event(monitoring_event).await;
        
        // Log event for debugging
        match severity {
            EventSeverity::Debug => debug!("Raft event: {:?}", event),
            EventSeverity::Info => info!("Raft event: {:?}", event),
            EventSeverity::Warning => tracing::warn!("Raft event: {:?}", event),
            EventSeverity::Error => error!("Raft event: {:?}", event),
            EventSeverity::Critical => error!("CRITICAL Raft event: {:?}", event),
        }
    }
    
    /// Get node ID
    pub fn node_id(&self) -> u64 {
        self.node_id
    }

    /// Subscribe to real-time events
    pub fn subscribe(&self) -> broadcast::Receiver<MonitoringEvent> {
        self.event_broadcaster.subscribe()
    }
    
    /// Get recent events (alias for get_event_history)
    pub async fn get_recent_events(&self, limit: usize) -> Vec<MonitoringEvent> {
        let history = self.event_history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }
    
    /// Get event history
    pub async fn get_event_history(&self, limit: Option<usize>) -> Vec<MonitoringEvent> {
        let history = self.event_history.read().await;
        match limit {
            Some(n) => history.iter().rev().take(n).cloned().collect(),
            None => history.clone(),
        }
    }
    
    /// Get current Raft metrics
    pub async fn get_current_metrics(&self) -> RaftMetrics {
        self.current_metrics.read().await.clone()
    }
    
    /// Update metrics manually
    pub async fn update_metrics(&self, metrics: RaftMetrics) {
        *self.current_metrics.write().await = metrics;
        
        // Publish metrics snapshot event
        self.publish_event(
            RaftEvent::MetricsSnapshot { 
                metrics: self.current_metrics.read().await.clone() 
            },
            EventSeverity::Debug
        ).await;
    }
    
    /// Get performance statistics
    pub async fn get_performance_stats(&self) -> PerformanceStats {
        let tracker = self.performance_tracker.read().await;
        PerformanceStats::from_tracker(&tracker)
    }
    
    /// Store event in history with size limit
    async fn store_event(&self, event: MonitoringEvent) {
        let mut history = self.event_history.write().await;
        history.push(event);
        
        // Keep only recent events
        let history_len = history.len();
        if history_len > self.max_history_size {
            history.drain(0..history_len - self.max_history_size);
        }
    }
    
    /// Update performance metrics based on event
    async fn update_metrics_from_event(&self, event: &RaftEvent) {
        let mut tracker = self.performance_tracker.write().await;
        
        match event {
            RaftEvent::LogCommitted { .. } => {
                // Track commit events
            }
            RaftEvent::LogApplied { apply_duration_us, .. } => {
                tracker.add_apply_latency(*apply_duration_us);
            }
            RaftEvent::LeaderElectionCompleted { election_duration_ms, .. } => {
                tracker.add_election_duration(*election_duration_ms);
            }
            RaftEvent::Heartbeat { response_time_us, success, .. } => {
                if *success {
                    if let Some(time) = response_time_us {
                        tracker.add_heartbeat_time(*time);
                    }
                }
            }
            _ => {}
        }
        
        // Update current metrics
        let mut metrics = self.current_metrics.write().await;
        metrics.timestamp = current_timestamp();
        
        // Calculate derived metrics
        metrics.avg_apply_latency_us = tracker.avg_apply_latency();
        metrics.heartbeat_success_rate = tracker.heartbeat_success_rate();
        metrics.messages_sent_per_sec = tracker.messages_per_second_sent();
        metrics.messages_received_per_sec = tracker.messages_per_second_received();
    }
}

/// Performance statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub avg_commit_latency_us: u64,
    pub p95_commit_latency_us: u64,
    pub avg_apply_latency_us: u64,
    pub p95_apply_latency_us: u64,
    pub avg_election_duration_ms: u64,
    pub avg_heartbeat_time_us: u64,
    pub heartbeat_success_rate: f64,
    pub messages_sent_per_sec: f64,
    pub messages_received_per_sec: f64,
    pub sample_count: usize,
    pub window_duration_sec: u64,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            commit_latencies: Vec::new(),
            apply_latencies: Vec::new(),
            election_durations: Vec::new(),
            heartbeat_times: Vec::new(),
            messages_sent: 0,
            messages_received: 0,
            window_start: current_timestamp(),
            max_samples: 1000,
        }
    }
    
    pub fn add_apply_latency(&mut self, latency_us: u64) {
        self.apply_latencies.push(latency_us);
        if self.apply_latencies.len() > self.max_samples {
            self.apply_latencies.remove(0);
        }
    }
    
    pub fn add_election_duration(&mut self, duration_ms: u64) {
        self.election_durations.push(duration_ms);
        if self.election_durations.len() > self.max_samples {
            self.election_durations.remove(0);
        }
    }
    
    pub fn add_heartbeat_time(&mut self, time_us: u64) {
        self.heartbeat_times.push(time_us);
        if self.heartbeat_times.len() > self.max_samples {
            self.heartbeat_times.remove(0);
        }
    }
    
    pub fn avg_apply_latency(&self) -> u64 {
        if self.apply_latencies.is_empty() {
            0
        } else {
            self.apply_latencies.iter().sum::<u64>() / self.apply_latencies.len() as u64
        }
    }
    
    pub fn heartbeat_success_rate(&self) -> f64 {
        // Simplified calculation - in practice would track failures too
        if self.heartbeat_times.is_empty() {
            0.0
        } else {
            1.0 // Assume successful if we have timing data
        }
    }
    
    pub fn messages_per_second_sent(&self) -> f64 {
        let window_duration = current_timestamp() - self.window_start;
        if window_duration == 0 {
            0.0
        } else {
            self.messages_sent as f64 / window_duration as f64
        }
    }
    
    pub fn messages_per_second_received(&self) -> f64 {
        let window_duration = current_timestamp() - self.window_start;
        if window_duration == 0 {
            0.0
        } else {
            self.messages_received as f64 / window_duration as f64
        }
    }
}

impl PerformanceStats {
    pub fn from_tracker(tracker: &PerformanceTracker) -> Self {
        let mut apply_latencies = tracker.apply_latencies.clone();
        apply_latencies.sort_unstable();
        
        let commit_latencies = vec![]; // Would be populated from actual commit tracking
        let mut heartbeat_times = tracker.heartbeat_times.clone();
        heartbeat_times.sort_unstable();
        
        Self {
            avg_commit_latency_us: 0, // Would calculate from actual data
            p95_commit_latency_us: percentile(&commit_latencies, 95),
            avg_apply_latency_us: tracker.avg_apply_latency(),
            p95_apply_latency_us: percentile(&apply_latencies, 95),
            avg_election_duration_ms: if tracker.election_durations.is_empty() {
                0
            } else {
                tracker.election_durations.iter().sum::<u64>() / tracker.election_durations.len() as u64
            },
            avg_heartbeat_time_us: if heartbeat_times.is_empty() {
                0
            } else {
                heartbeat_times.iter().sum::<u64>() / heartbeat_times.len() as u64
            },
            heartbeat_success_rate: tracker.heartbeat_success_rate(),
            messages_sent_per_sec: tracker.messages_per_second_sent(),
            messages_received_per_sec: tracker.messages_per_second_received(),
            sample_count: apply_latencies.len() + commit_latencies.len() + heartbeat_times.len(),
            window_duration_sec: current_timestamp() - tracker.window_start,
        }
    }
}

impl RaftMetrics {
    pub fn default(node_id: u64) -> Self {
        Self {
            node_id,
            current_term: 0,
            leader_id: None,
            is_leader: false,
            commit_index: 0,
            last_applied: 0,
            log_size: 0,
            log_size_bytes: 0,
            cluster_size: 1,
            active_followers: 0,
            avg_commit_latency_us: 0,
            avg_apply_latency_us: 0,
            elections_count: 0,
            leadership_changes: 0,
            heartbeat_success_rate: 0.0,
            messages_sent_per_sec: 0.0,
            messages_received_per_sec: 0.0,
            timestamp: current_timestamp(),
        }
    }
}

/// Calculate percentile from sorted vector
fn percentile(sorted_values: &[u64], p: u8) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }
    
    let index = ((p as f64 / 100.0) * (sorted_values.len() - 1) as f64).round() as usize;
    sorted_values.get(index).copied().unwrap_or(0)
}

/// Get current Unix timestamp
pub fn current_timestamp() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_monitor_creation() {
        let monitor = RaftMonitor::new(1, 100);
        assert_eq!(monitor.node_id, 1);
        assert_eq!(monitor.max_history_size, 100);
        
        let metrics = monitor.get_current_metrics().await;
        assert_eq!(metrics.node_id, 1);
    }

    #[tokio::test]
    async fn test_event_publishing() {
        let monitor = RaftMonitor::new(1, 100);
        
        let event = RaftEvent::LogCommitted {
            index: 1,
            term: 1,
            entry_size: 1024,
            node_id: 1,
        };
        
        monitor.publish_event(event, EventSeverity::Info).await;
        
        let history = monitor.get_event_history(None).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].node_id, 1);
    }

    #[tokio::test]
    async fn test_real_time_subscription() {
        let monitor = RaftMonitor::new(1, 100);
        let mut receiver = monitor.subscribe();
        
        // Publish event in background
        let monitor_clone = Arc::new(monitor);
        let publisher = monitor_clone.clone();
        tokio::spawn(async move {
            sleep(Duration::from_millis(100)).await;
            publisher.publish_event(
                RaftEvent::LeaderElectionStarted { 
                    term: 2, 
                    candidate_id: 1, 
                    previous_leader: None 
                },
                EventSeverity::Info
            ).await;
        });
        
        // Receive event
        let received_event = receiver.recv().await.unwrap();
        assert_eq!(received_event.node_id, 1);
        
        if let RaftEvent::LeaderElectionStarted { term, candidate_id, .. } = received_event.event {
            assert_eq!(term, 2);
            assert_eq!(candidate_id, 1);
        } else {
            panic!("Expected LeaderElectionStarted event");
        }
    }
}