# Operations Runbook

This guide provides operational procedures for managing Hyra Scribe Ledger in production.

## Table of Contents

- [Monitoring and Alerting](#monitoring-and-alerting)
- [Common Operational Tasks](#common-operational-tasks)
- [Incident Response](#incident-response)
- [Maintenance Procedures](#maintenance-procedures)
- [Performance Tuning](#performance-tuning)

## Monitoring and Alerting

### Metrics Collection

**Prometheus Configuration:**

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'scribe-ledger'
    static_configs:
      - targets:
          - 'node1:8001'
          - 'node2:8002'
          - 'node3:8003'
    metrics_path: '/metrics/prometheus'
    scrape_interval: 15s
```

**Key Metrics to Monitor:**

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `scribe_ops_total` | Total operations | Rate decrease > 50% |
| `scribe_request_latency_p99` | P99 latency | > 1000ms |
| `scribe_errors_total` | Error count | Rate > 10/min |
| `scribe_storage_keys_total` | Storage size | > 90% capacity |
| `scribe_raft_term` | Current Raft term | Frequent changes |
| `scribe_node_health` | Node health status | < 1 (unhealthy) |

### Grafana Dashboards

**Import Dashboard:**

```bash
# Download dashboard JSON
curl -o scribe-dashboard.json \
  https://raw.githubusercontent.com/hyra-network/simple-scribe-ledger/main/dashboards/scribe-ledger.json

# Import to Grafana
curl -X POST http://grafana:3000/api/dashboards/db \
  -H 'Content-Type: application/json' \
  -d @scribe-dashboard.json
```

**Key Panels:**
- Request rate (ops/sec)
- Latency percentiles (P50, P95, P99)
- Error rate
- Storage utilization
- Cluster health
- Raft metrics (term, commit index)

### Alert Rules

**Example Prometheus Alert Rules:**

```yaml
# alerts.yml
groups:
  - name: scribe-ledger
    interval: 30s
    rules:
      - alert: HighLatency
        expr: scribe_request_latency_p99 > 1000
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High latency detected"
          description: "P99 latency is {{ $value }}ms"

      - alert: HighErrorRate
        expr: rate(scribe_errors_total[5m]) > 0.1
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "High error rate"
          description: "Error rate: {{ $value }}/sec"

      - alert: NodeDown
        expr: up{job="scribe-ledger"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Node is down"
          description: "{{ $labels.instance }} is not responding"

      - alert: ClusterQuorumLost
        expr: count(up{job="scribe-ledger"} == 1) < 2
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Cluster quorum lost"
          description: "Less than 2 nodes available"

      - alert: StorageNearFull
        expr: scribe_storage_keys_total > 900000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Storage approaching capacity"
          description: "{{ $value }} keys stored"
```

### Log Aggregation

**Filebeat Configuration:**

```yaml
# filebeat.yml
filebeat.inputs:
  - type: log
    enabled: true
    paths:
      - /var/log/scribe-ledger/*.log
    json.keys_under_root: true
    json.add_error_key: true

output.elasticsearch:
  hosts: ["elasticsearch:9200"]
  index: "scribe-ledger-%{+yyyy.MM.dd}"

processors:
  - add_host_metadata: ~
  - add_cloud_metadata: ~
```

## Common Operational Tasks

### Check Cluster Status

```bash
# Check cluster health
curl http://localhost:8001/health

# View cluster info
curl http://localhost:8001/cluster/info

# List cluster members
curl http://localhost:8001/cluster/nodes

# Get current leader
curl http://localhost:8001/cluster/leader/info
```

### Add Node to Cluster

```bash
# 1. Deploy new node with unique ID and address
# 2. Start the node
sudo systemctl start scribe-node-4

# 3. Add to cluster (from leader)
curl -X POST http://leader:8001/cluster/nodes/add \
  -H 'Content-Type: application/json' \
  -H 'X-API-Key: admin-key' \
  -d '{
    "node_id": 4,
    "address": "10.0.1.13:8004"
  }'

# 4. Verify node joined
curl http://leader:8001/cluster/nodes
```

### Remove Node from Cluster

```bash
# 1. Remove from cluster (graceful)
curl -X POST http://leader:8001/cluster/nodes/remove \
  -H 'Content-Type: application/json' \
  -H 'X-API-Key: admin-key' \
  -d '{"node_id": 4}'

# 2. Stop the node
ssh node4 'sudo systemctl stop scribe-node-4'

# 3. Verify removal
curl http://leader:8001/cluster/nodes
```

### Rotate API Keys

```bash
# 1. Generate new API keys
NEW_ADMIN_KEY=$(openssl rand -hex 32)
NEW_WRITE_KEY=$(openssl rand -hex 32)
NEW_READ_KEY=$(openssl rand -hex 32)

# 2. Add new keys to configuration
# (Update config file or environment variables)

# 3. Restart nodes with new configuration
for node in node{1..3}; do
  ssh $node 'sudo systemctl restart scribe-node'
done

# 4. Update client applications with new keys

# 5. Remove old keys from configuration (after grace period)
```

### Backup and Restore

**Backup Procedure:**

```bash
# 1. Stop writes (if possible) or create consistent snapshot
curl -X POST http://leader:8001/admin/pause-writes \
  -H 'X-API-Key: admin-key'

# 2. Backup data directory
sudo tar -czf /backup/scribe-ledger-$(date +%Y%m%d).tar.gz \
  /var/lib/scribe-ledger

# 3. Backup configuration
sudo cp /etc/scribe-ledger/config.toml \
  /backup/config-$(date +%Y%m%d).toml

# 4. Resume writes
curl -X POST http://leader:8001/admin/resume-writes \
  -H 'X-API-Key: admin-key'

# 5. Verify backup integrity
tar -tzf /backup/scribe-ledger-$(date +%Y%m%d).tar.gz > /dev/null
```

**Restore Procedure:**

```bash
# 1. Stop the node
sudo systemctl stop scribe-node

# 2. Clear existing data
sudo rm -rf /var/lib/scribe-ledger/*

# 3. Extract backup
sudo tar -xzf /backup/scribe-ledger-20240115.tar.gz \
  -C /

# 4. Restore configuration
sudo cp /backup/config-20240115.toml \
  /etc/scribe-ledger/config.toml

# 5. Fix permissions
sudo chown -R scribe-ledger:scribe-ledger /var/lib/scribe-ledger

# 6. Start the node
sudo systemctl start scribe-node

# 7. Verify data
curl http://localhost:8001/metrics
```

### Certificate Renewal

```bash
# 1. Generate new certificates (or renew with Let's Encrypt)
sudo certbot renew

# 2. Update certificate paths in config (if changed)

# 3. Restart nodes one at a time
for node in node{1..3}; do
  ssh $node 'sudo systemctl restart scribe-node'
  sleep 30  # Wait for node to rejoin cluster
done

# 4. Verify TLS is working
openssl s_client -connect node1:8001
```

## Incident Response

### Node Failure

**Symptoms:**
- Node not responding to health checks
- Cluster shows node as unavailable
- Alerts for node down

**Response:**

```bash
# 1. Check node status
ssh node2 'sudo systemctl status scribe-node'

# 2. Check logs for errors
ssh node2 'sudo journalctl -u scribe-node -n 100'

# 3. Attempt restart
ssh node2 'sudo systemctl restart scribe-node'

# 4. If restart fails, check:
#    - Disk space
#    - Memory availability
#    - Network connectivity
#    - Configuration errors

# 5. If cannot recover, remove node and redeploy
curl -X POST http://leader:8001/cluster/nodes/remove \
  -H 'X-API-Key: admin-key' \
  -d '{"node_id": 2}'
```

### Cluster Quorum Lost

**Symptoms:**
- Writes are failing
- Cluster cannot elect leader
- Less than majority of nodes available

**Response:**

```bash
# 1. Check how many nodes are available
curl http://any-node:8001/cluster/nodes

# 2. Bring up missing nodes
for node in node{1..3}; do
  ssh $node 'sudo systemctl start scribe-node'
done

# 3. If nodes cannot recover, restore from backup

# 4. If disaster recovery needed, initialize new cluster
#    from most recent backup
```

### High Latency

**Symptoms:**
- P99 latency > 1000ms
- Slow response times
- Client timeouts

**Investigation:**

```bash
# 1. Check system resources
ssh node1 'top -b -n 1'
ssh node1 'df -h'
ssh node1 'free -h'

# 2. Check network latency
ping -c 10 node1
ping -c 10 node2

# 3. Check database metrics
curl http://node1:8001/metrics/prometheus | grep latency

# 4. Check for slow queries in logs
ssh node1 'sudo journalctl -u scribe-node | grep "slow"'

# 5. Tune performance (see Performance Tuning section)
```

### Data Corruption

**Symptoms:**
- Merkle proof verification failures
- Inconsistent data across nodes
- Storage errors in logs

**Response:**

```bash
# 1. Identify affected node
curl http://each-node:8001/verify/key

# 2. Stop affected node
ssh affected-node 'sudo systemctl stop scribe-node'

# 3. Clear corrupted data
ssh affected-node 'sudo rm -rf /var/lib/scribe-ledger/*'

# 4. Restore from backup or let it sync from cluster
#    (cluster will replicate data automatically)

# 5. Restart node
ssh affected-node 'sudo systemctl start scribe-node'

# 6. Verify data integrity
curl http://affected-node:8001/verify/key
```

## Maintenance Procedures

### Rolling Upgrade

```bash
# 1. Build new version
cargo build --release --bin scribe-node

# 2. Upgrade one node at a time
for node in node{1..3}; do
  echo "Upgrading $node..."
  
  # Copy new binary
  scp target/release/scribe-node $node:/tmp/
  
  # Install new binary
  ssh $node 'sudo systemctl stop scribe-node && \
             sudo cp /tmp/scribe-node /usr/local/bin/ && \
             sudo systemctl start scribe-node'
  
  # Wait for node to rejoin
  sleep 60
  
  # Verify node is healthy
  curl http://$node:8001/health
  
  echo "$node upgraded successfully"
done
```

### Database Compaction

```bash
# If database grows too large, compact on each node

# 1. Trigger compaction (if supported)
curl -X POST http://node1:8001/admin/compact \
  -H 'X-API-Key: admin-key'

# 2. Monitor progress
watch -n 5 'curl -s http://node1:8001/metrics | grep storage'

# 3. Repeat for other nodes
```

### Log Rotation

Configure logrotate:

```bash
# /etc/logrotate.d/scribe-ledger
/var/log/scribe-ledger/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 0644 scribe-ledger scribe-ledger
    postrotate
        systemctl reload scribe-node
    endscript
}
```

## Performance Tuning

### Adjust Cache Size

```toml
# config.toml
[storage]
max_cache_size = 536870912  # 512MB (increase if you have memory)
```

### Tune Batch Sizes

```toml
[performance]
batch_size = 200  # Increase for better write throughput
max_concurrency = 50  # Adjust based on workload
```

### Optimize Consensus Parameters

```toml
[consensus]
election_timeout = 5    # Reduce for faster failover
heartbeat_timeout = 1   # More frequent heartbeats
```

### Rate Limiting Adjustments

```toml
[security.rate_limit]
max_requests = 2000  # Increase for high-traffic scenarios
window_secs = 60
burst_size = 200
```

### Kernel Tuning

```bash
# /etc/sysctl.conf

# Increase file descriptor limits
fs.file-max = 1000000

# TCP tuning
net.core.somaxconn = 4096
net.ipv4.tcp_max_syn_backlog = 8192
net.ipv4.tcp_fin_timeout = 15

# Apply changes
sudo sysctl -p
```

## Health Checks

### Automated Health Monitoring

```bash
#!/bin/bash
# /usr/local/bin/check-scribe-health.sh

NODES=("node1:8001" "node2:8002" "node3:8003")

for node in "${NODES[@]}"; do
  if ! curl -sf "http://$node/health" > /dev/null; then
    echo "ALERT: $node is unhealthy"
    # Send notification (PagerDuty, email, etc.)
  fi
done
```

Add to crontab:
```bash
*/5 * * * * /usr/local/bin/check-scribe-health.sh
```

## Capacity Planning

Monitor these metrics to plan capacity:

- Storage growth rate (GB/day)
- Request rate trends (ops/sec)
- Latency trends (P99)
- Resource utilization (CPU, memory, disk)

**Formula for node capacity:**
- Minimum 3 nodes for quorum
- Add 1 node per 100k ops/sec sustained load
- Plan for 2x peak capacity
- Keep storage utilization < 80%
