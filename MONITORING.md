# 📊 Scribe Ledger Real-time Raft Monitoring System

A comprehensive real-time monitoring system for Raft consensus operations in Scribe Ledger, providing visibility into distributed coordination, performance metrics, and live event streaming.

## 🚀 Features

### Real-time Event Tracking
- **Leader Elections**: Track leadership changes and election timing
- **Log Operations**: Monitor log commits and applications with performance metrics
- **Node Management**: Track node joins, leaves, and cluster membership changes
- **Heartbeat Monitoring**: Monitor consensus heartbeat success rates
- **Configuration Changes**: Track cluster configuration modifications

### HTTP REST API
- `GET /raft/status` - Current Raft node status and leadership
- `GET /raft/metrics` - Performance metrics and statistics
- `GET /raft/events` - Recent Raft events history
- `WebSocket /raft/live` - Real-time event streaming

### Performance Metrics
- Commit latency tracking
- Apply operation timing
- Leader election duration
- Heartbeat success rates
- Message throughput statistics

## 📋 Event Types

The monitoring system tracks the following Raft events:

```rust
pub enum RaftEvent {
    // Node lifecycle events
    NodeJoined { node_id: u64, address: String, cluster_size: usize },
    NodeLeft { node_id: u64, reason: String, cluster_size: usize },
    
    // Leader election events
    LeaderElectionStarted { candidate_id: u64, term: u64, previous_leader: Option<u64> },
    LeaderElectionCompleted { leader_id: u64, term: u64, election_duration_ms: u64, votes_received: usize, total_voters: usize },
    
    // Log operation events
    LogCommitted { index: u64, term: u64, entry_size: usize, node_id: u64 },
    LogApplied { index: u64, term: u64, apply_duration_us: u64, node_id: u64 },
    
    // Communication events
    Heartbeat { from: u64, to: u64, sequence: u64, success: bool, response_time_us: Option<u64> },
}
```

## 🔧 Usage

### Starting the Node with Monitoring

```bash
# Start the Scribe Ledger node
cargo run --bin scribe-node

# The monitoring endpoints will be available at:
# http://localhost:8080/raft/status
# http://localhost:8080/raft/metrics  
# http://localhost:8080/raft/events
# ws://localhost:8080/raft/live
```

### Using the REST API

```bash
# Get current Raft status
curl http://localhost:8080/raft/status

# Get performance metrics
curl http://localhost:8080/raft/metrics

# Get recent events
curl http://localhost:8080/raft/events
```

### WebSocket Real-time Streaming

```javascript
const ws = new WebSocket('ws://localhost:8080/raft/live');

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    
    switch(data.type) {
        case 'event':
            console.log('Raft Event:', data.data);
            break;
        case 'heartbeat':
            console.log('Heartbeat:', data.timestamp);
            break;
        case 'status':
            console.log('Status:', data.message);
            break;
    }
};
```

### Python Demo Script

```bash
# Install dependencies
pip install websockets requests

# Run the monitoring demo
python3 demo_monitoring.py
```

## 📊 Example Responses

### Status Endpoint Response
```json
{
  "node_id": 1,
  "address": "127.0.0.1:8081",
  "is_leader": true,
  "status": "active"
}
```

### Metrics Endpoint Response  
```json
{
  "node_id": 1,
  "current_term": 5,
  "leader_id": 1,
  "is_leader": true,
  "commit_index": 42,
  "last_applied": 42,
  "avg_apply_latency_us": 150,
  "heartbeat_success_rate": 98.5,
  "messages_sent_per_sec": 25.3,
  "messages_received_per_sec": 23.1,
  "timestamp": 1695648542
}
```

### Events Endpoint Response
```json
{
  "events": [
    {
      "id": 123,
      "timestamp": 1695648542,
      "node_id": 1,
      "event": {
        "NodeJoined": {
          "node_id": 2,
          "address": "127.0.0.1:8082",
          "cluster_size": 2
        }
      },
      "severity": "Info",
      "context": {}
    }
  ],
  "count": 1
}
```

### WebSocket Event Stream
```json
{
  "type": "event",
  "data": {
    "id": 124,
    "timestamp": 1695648545,
    "node_id": 1,
    "event": {
      "LeaderElectionCompleted": {
        "leader_id": 1,
        "term": 6,
        "election_duration_ms": 250,
        "votes_received": 2,
        "total_voters": 3
      }
    },
    "severity": "Info"
  }
}
```

## 🧪 Testing

The monitoring system includes comprehensive tests:

```bash
# Run monitoring tests
cargo test monitoring -- --nocapture

# Run integration tests
cargo test test_raft_monitoring_integration -- --nocapture
cargo test test_monitoring_event_broadcasting -- --nocapture
cargo test test_monitoring_api_endpoints -- --nocapture
```

## 📈 Performance Considerations

- **Event History**: Limited to configurable maximum (default: 1000 events)
- **Broadcasting**: Uses tokio broadcast channels for efficient real-time distribution
- **WebSocket Connections**: Handles multiple concurrent connections with heartbeat
- **Memory Usage**: Events are stored in memory with automatic cleanup

## 🔒 Security Notes

- Monitoring endpoints are currently unauthenticated
- WebSocket connections have automatic heartbeat and cleanup
- Production deployments should add authentication and rate limiting

## 🛠️ Architecture

The monitoring system consists of:

1. **RaftMonitor**: Core monitoring component with event tracking
2. **RaftEvent**: Structured event types for all Raft operations  
3. **PerformanceTracker**: Statistical tracking of operation latencies
4. **HTTP Handlers**: REST API endpoints for status and metrics
5. **WebSocket Handler**: Real-time event streaming with proper connection management

## 📚 Integration

The monitoring system is fully integrated with the ConsensusNode:

```rust
// Access the monitor from a consensus node
let consensus_node = ConsensusNode::new(node_id, address, port, transport)?;
let monitor = consensus_node.monitor();

// Subscribe to events
let mut receiver = monitor.subscribe();

// Publish custom events
monitor.publish_event(RaftEvent::NodeJoined { ... }, EventSeverity::Info).await;
```

This monitoring system provides comprehensive visibility into Raft consensus operations, enabling debugging, performance analysis, and operational monitoring of distributed Scribe Ledger deployments.