# Troubleshooting Guide

Common issues and solutions for Hyra Scribe Ledger.

## Table of Contents

- [Connection Issues](#connection-issues)
- [Authentication Problems](#authentication-problems)
- [Performance Issues](#performance-issues)
- [Cluster Problems](#cluster-problems)
- [Storage Issues](#storage-issues)
- [TLS/SSL Issues](#tlsssl-issues)
- [Debug Logging](#debug-logging)

## Connection Issues

### Cannot Connect to Node

**Symptoms:**
- `Connection refused` errors
- `No route to host` errors
- Timeouts when connecting

**Diagnostics:**

```bash
# Check if service is running
sudo systemctl status scribe-node

# Check if port is listening
sudo netstat -tlnp | grep 8080

# Test local connectivity
curl http://localhost:8080/health

# Test network connectivity
telnet node1 8080
```

**Solutions:**

1. **Service not running:**
   ```bash
   sudo systemctl start scribe-node
   sudo systemctl status scribe-node
   ```

2. **Firewall blocking:**
   ```bash
   # Check firewall rules
   sudo iptables -L -n
   
   # Allow traffic on required ports
   sudo firewall-cmd --permanent --add-port=8080/tcp
   sudo firewall-cmd --permanent --add-port=8081/tcp
   sudo firewall-cmd --reload
   ```

3. **Wrong bind address:**
   ```toml
   # config.toml - bind to all interfaces
   [network]
   listen_addr = "0.0.0.0"  # not "127.0.0.1"
   ```

4. **SELinux blocking:**
   ```bash
   # Temporarily set to permissive
   sudo setenforce 0
   
   # Check for denials
   sudo ausearch -m avc -ts recent
   ```

### Intermittent Connection Drops

**Symptoms:**
- Connections drop randomly
- Request timeouts

**Diagnostics:**

```bash
# Check network interface
sudo ethtool eth0

# Check for packet loss
ping -c 100 node1

# Monitor connections
sudo tcpdump -i eth0 port 8080
```

**Solutions:**

1. **Network instability:**
   - Check network hardware
   - Increase TCP timeouts
   - Enable TCP keepalive

2. **Resource exhaustion:**
   ```bash
   # Check open file descriptors
   sudo lsof -p $(pgrep scribe-node) | wc -l
   
   # Increase limits if needed
   ulimit -n 65536
   ```

## Authentication Problems

### "Authentication required" Error

**Symptoms:**
```json
{"error": "Authentication required. Provide API key via Authorization: Bearer <token> or X-API-Key: <key>"}
```

**Solutions:**

1. **Missing API key:**
   ```bash
   # Add API key header
   curl -H "X-API-Key: your-key-here" http://localhost:8080/data
   
   # Or use Bearer token
   curl -H "Authorization: Bearer your-key-here" http://localhost:8080/data
   ```

2. **Authentication disabled but required:**
   ```toml
   # config.toml - enable auth
   [security.auth]
   enabled = true
   ```

### "Invalid API key" Error

**Symptoms:**
```json
{"error": "Invalid API key"}
```

**Solutions:**

1. **Check API key is correct:**
   ```bash
   # Verify key in configuration
   grep -A 5 'ADMIN_API_KEY' /etc/scribe-ledger/api-keys.env
   ```

2. **Key not loaded:**
   ```bash
   # Restart service to reload configuration
   sudo systemctl restart scribe-node
   ```

3. **Typo in API key:**
   - Copy-paste the key to avoid typos
   - Check for extra whitespace

### "Insufficient permissions" Error

**Symptoms:**
```json
{"error": "Insufficient permissions. Required: Write"}
```

**Solutions:**

1. **Using read-only key for write operations:**
   ```bash
   # Use write or admin key
   curl -H "X-API-Key: write-or-admin-key" \
     -X PUT http://localhost:8080/data \
     -d '{"value": "test"}'
   ```

2. **Check role permissions:**
   - Read-only: GET only
   - Read-write: GET, PUT
   - Admin: GET, PUT, DELETE, cluster/metrics access

## Performance Issues

### High Latency

**Symptoms:**
- P99 latency > 1000ms
- Slow response times

**Diagnostics:**

```bash
# Check system load
uptime
top -b -n 1

# Check I/O wait
iostat -x 1 10

# Check disk latency
sudo iotop -o

# Profile with perf
sudo perf record -g -p $(pgrep scribe-node)
sudo perf report
```

**Solutions:**

1. **High disk I/O:**
   ```toml
   # Increase cache size
   [storage]
   max_cache_size = 536870912  # 512MB
   ```

2. **CPU bottleneck:**
   - Increase worker threads
   - Distribute load across nodes
   - Scale horizontally

3. **Network latency:**
   ```bash
   # Check network latency between nodes
   ping -c 100 node2
   
   # Reduce timeouts if needed
   ```

4. **Large batch sizes:**
   ```toml
   # Reduce batch size
   [performance]
   batch_size = 50  # Smaller batches
   ```

### Rate Limiting Triggered

**Symptoms:**
```json
{"error": "Rate limit exceeded. Try again later."}
```

**Solutions:**

1. **Legitimate traffic spike:**
   ```toml
   # Increase rate limits
   [security.rate_limit]
   max_requests = 2000
   burst_size = 200
   ```

2. **Multiple clients with same API key:**
   - Use separate API keys per client
   - Rate limits are per client ID

3. **Implement backoff and retry:**
   ```bash
   # Exponential backoff example
   for i in 1 2 4 8 16; do
     if curl -H "X-API-Key: key" http://localhost:8080/data; then
       break
     fi
     sleep $i
   done
   ```

### Memory Issues

**Symptoms:**
- OOM killer events
- Swap usage high
- Process crashes

**Diagnostics:**

```bash
# Check memory usage
free -h

# Monitor process memory
sudo pmap -x $(pgrep scribe-node)

# Check for memory leaks
valgrind --leak-check=full ./scribe-node
```

**Solutions:**

1. **Reduce cache size:**
   ```toml
   [storage]
   max_cache_size = 134217728  # 128MB
   ```

2. **Add swap space:**
   ```bash
   sudo dd if=/dev/zero of=/swapfile bs=1G count=4
   sudo chmod 600 /swapfile
   sudo mkswap /swapfile
   sudo swapon /swapfile
   ```

3. **Increase system memory:**
   - Add more RAM
   - Use larger instance type

## Cluster Problems

### Cannot Form Cluster

**Symptoms:**
- Nodes cannot see each other
- Cluster initialization fails

**Diagnostics:**

```bash
# Check cluster members
curl http://node1:8080/cluster/nodes

# Check cluster status
curl http://node1:8080/cluster/info

# Check network connectivity
for node in node{1..3}; do
  ping -c 3 $node
done
```

**Solutions:**

1. **Network isolation:**
   ```bash
   # Verify nodes can reach each other
   ssh node1 'curl http://node2:8090/health'
   ssh node2 'curl http://node3:8100/health'
   ```

2. **Wrong node addresses:**
   ```toml
   # Verify addresses in config.toml
   [node]
   address = "10.0.1.10:8001"  # Must be reachable by other nodes
   ```

3. **Raft ports blocked:**
   ```bash
   # Open Raft TCP ports
   sudo firewall-cmd --permanent --add-port=8081-8083/tcp
   sudo firewall-cmd --reload
   ```

### Split Brain Scenario

**Symptoms:**
- Multiple leaders
- Inconsistent data
- Cluster cannot reach consensus

**Diagnostics:**

```bash
# Check leader on each node
for node in node{1..3}; do
  echo "$node:"
  curl -s http://$node:8080/cluster/leader/info
done
```

**Solutions:**

1. **Stop minority partition:**
   ```bash
   # If you have 2 partitions, stop the smaller one
   ssh isolated-node 'sudo systemctl stop scribe-node'
   ```

2. **Restore network connectivity:**
   - Fix network issues
   - Wait for automatic recovery

3. **Restart cluster if needed:**
   ```bash
   # Stop all nodes
   for node in node{1..3}; do
     ssh $node 'sudo systemctl stop scribe-node'
   done
   
   # Start from backup or reinitialize
   ```

### Node Cannot Rejoin Cluster

**Symptoms:**
- Node starts but doesn't join cluster
- Logs show connection errors

**Diagnostics:**

```bash
# Check node logs
sudo journalctl -u scribe-node -f

# Verify node is reachable
curl http://rejoining-node:8080/health

# Check cluster membership
curl http://leader:8080/cluster/nodes
```

**Solutions:**

1. **Stale node data:**
   ```bash
   # Clear data and let it resync
   sudo systemctl stop scribe-node
   sudo rm -rf /var/lib/scribe-ledger/*
   sudo systemctl start scribe-node
   ```

2. **Re-add to cluster:**
   ```bash
   curl -X POST http://leader:8080/cluster/nodes/add \
     -H 'X-API-Key: admin-key' \
     -d '{"node_id": 2, "address": "10.0.1.11:8002"}'
   ```

## Storage Issues

### Disk Full

**Symptoms:**
```
Error: No space left on device
```

**Solutions:**

1. **Free up space:**
   ```bash
   # Check disk usage
   df -h
   
   # Find large files
   sudo du -h /var/lib/scribe-ledger | sort -h | tail -20
   
   # Clean up old logs
   sudo journalctl --vacuum-time=7d
   ```

2. **Archive old data to S3:**
   ```bash
   # Trigger archival (if configured)
   curl -X POST http://localhost:8080/admin/archive \
     -H 'X-API-Key: admin-key'
   ```

3. **Expand storage:**
   - Add new volume
   - Resize existing volume
   - Migrate to larger instance

### Data Corruption

**Symptoms:**
- Merkle verification fails
- Storage errors in logs
- Cannot read data

**Diagnostics:**

```bash
# Check Merkle root
curl http://localhost:8080/verify/key

# Check storage integrity
sudo -u scribe-ledger /usr/local/bin/scribe-node --check-db
```

**Solutions:**

1. **Restore from backup:**
   ```bash
   sudo systemctl stop scribe-node
   sudo rm -rf /var/lib/scribe-ledger/*
   sudo tar -xzf /backup/latest-backup.tar.gz -C /
   sudo systemctl start scribe-node
   ```

2. **Resync from cluster:**
   ```bash
   # Clear corrupt data and let cluster replicate
   sudo systemctl stop scribe-node
   sudo rm -rf /var/lib/scribe-ledger/*
   sudo systemctl start scribe-node
   ```

## TLS/SSL Issues

### Certificate Errors

**Symptoms:**
```
SSL certificate problem: self signed certificate
```

**Solutions:**

1. **Accept self-signed cert (development only):**
   ```bash
   curl -k https://localhost:8080/health  # -k to ignore cert errors
   ```

2. **Use proper CA certificate:**
   ```bash
   curl --cacert /path/to/ca.crt https://localhost:8080/health
   ```

3. **Add cert to trust store:**
   ```bash
   sudo cp ca.crt /usr/local/share/ca-certificates/
   sudo update-ca-certificates
   ```

### Certificate Expired

**Symptoms:**
```
certificate has expired or is not yet valid
```

**Solutions:**

```bash
# Check certificate expiry
openssl x509 -in /etc/scribe-ledger/certs/server.crt -noout -dates

# Renew certificate
sudo certbot renew  # If using Let's Encrypt

# Or generate new certificate
openssl req -x509 -newkey rsa:4096 -keyout server.key \
  -out server.crt -days 365 -nodes

# Restart service
sudo systemctl restart scribe-node
```

### Mutual TLS Authentication Fails

**Symptoms:**
```
client certificate required
```

**Solutions:**

1. **Provide client certificate:**
   ```bash
   curl --cert client.crt --key client.key \
     https://localhost:8080/health
   ```

2. **Disable mutual TLS (if not needed):**
   ```toml
   [security.tls]
   require_client_cert = false
   ```

## Debug Logging

### Enable Debug Logging

**Via Environment Variable:**
```bash
export RUST_LOG=debug
sudo -E systemctl restart scribe-node
```

**Via Configuration:**
```toml
[logging]
level = "debug"
```

### View Logs

```bash
# Follow logs in real-time
sudo journalctl -u scribe-node -f

# Last 100 lines
sudo journalctl -u scribe-node -n 100

# Filter by time
sudo journalctl -u scribe-node --since "1 hour ago"

# Search for errors
sudo journalctl -u scribe-node | grep ERROR

# View file logs (if enabled)
tail -f /var/log/scribe-ledger/scribe.log
```

### Collect Diagnostic Bundle

```bash
#!/bin/bash
# collect-diagnostics.sh

mkdir -p /tmp/scribe-diagnostics
cd /tmp/scribe-diagnostics

# System info
uname -a > system-info.txt
free -h >> system-info.txt
df -h >> system-info.txt

# Service status
sudo systemctl status scribe-node > service-status.txt

# Logs
sudo journalctl -u scribe-node -n 1000 > service-logs.txt

# Configuration
sudo cp /etc/scribe-ledger/config.toml config.toml

# Metrics
curl http://localhost:8080/metrics > metrics.txt

# Cluster info
curl http://localhost:8080/cluster/info > cluster-info.txt

# Create tarball
tar -czf ../scribe-diagnostics-$(date +%Y%m%d-%H%M%S).tar.gz .
```

## Getting Help

If you cannot resolve an issue:

1. **Check GitHub Issues:** https://github.com/amogusdrip285/Scribe-Ledger/issues
2. **Create New Issue:** Include diagnostic bundle and reproduction steps
3. **Community Support:** Join the discussion forum
4. **Commercial Support:** Contact support@hyra-network.com

## Additional Resources

- [Deployment Guide](DEPLOYMENT.md)
- [Operations Runbook](OPERATIONS.md)
- [Configuration Reference](CONFIGURATION.md)
- [Performance Tuning Guide](docs/PERFORMANCE_OPTIMIZATIONS.md)
