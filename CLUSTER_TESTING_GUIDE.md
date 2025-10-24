# 🧪 3-Node Cluster Testing Guide with S3

This guide explains how to test the 3-node Hyra Scribe Ledger cluster with S3 storage.

## 📋 Prerequisites

1. **MinIO** - S3-compatible storage
   ```bash
   # macOS
   brew install minio/stable/minio minio/stable/mc
   
   # Linux
   wget https://dl.min.io/server/minio/release/linux-amd64/minio
   chmod +x minio
   sudo mv minio /usr/local/bin/
   ```

2. **Rust & Cargo** - Already installed ✓

## 🚀 Quick Start - Automated Test

The easiest way to test is using our automated script:

```bash
./scripts/test-3node-cluster-s3.sh
```

This script will:
- ✅ Start MinIO S3 storage
- ✅ Create S3 buckets for each node
- ✅ Build and start 3 nodes
- ✅ Test data replication
- ✅ Show cluster metrics
- ✅ Cleanup on exit (Ctrl+C)

## 🔧 Manual Testing (Step by Step)

### Step 1: Start MinIO

```bash
# Start MinIO server
mkdir -p ./minio-data
MINIO_ROOT_USER=minioadmin MINIO_ROOT_PASSWORD=minioadmin \
    minio server ./minio-data --address :9000 --console-address :9001
```

**MinIO Console:** http://localhost:9001 (minioadmin/minioadmin)

### Step 2: Create S3 Buckets

```bash
# Configure MinIO client
mc alias set local http://localhost:9000 minioadmin minioadmin

# Create buckets
mc mb local/scribe-ledger-node1
mc mb local/scribe-ledger-node2
mc mb local/scribe-ledger-node3

# Verify buckets
mc ls local
```

### Step 3: Build the Project

```bash
cargo build --release --bin scribe-node
```

### Step 4: Start Nodes

**Terminal 1 - Node 1 (Bootstrap/Leader):**
```bash
./target/release/scribe-node --bootstrap --config config-node1.toml
```

**Terminal 2 - Node 2:**
```bash
./target/release/scribe-node --config config-node2.toml
```

**Terminal 3 - Node 3:**
```bash
./target/release/scribe-node --config config-node3.toml
```

### Step 5: Test the Cluster

**Check Health:**
```bash
curl http://localhost:8001/health  # Node 1
curl http://localhost:8002/health  # Node 2
curl http://localhost:8003/health  # Node 3
```

**Write Data to Different Nodes:**
```bash
# Write to Node 1
curl -X PUT http://localhost:8001/user:alice -d "Alice from Node 1"

# Write to Node 2
curl -X PUT http://localhost:8002/user:bob -d "Bob from Node 2"

# Write to Node 3
curl -X PUT http://localhost:8003/user:charlie -d "Charlie from Node 3"
```

**Test Data Replication:**
```bash
# Read from different nodes (should work due to replication)
curl http://localhost:8002/user:alice    # Read Node 1's data from Node 2
curl http://localhost:8003/user:bob      # Read Node 2's data from Node 3
curl http://localhost:8001/user:charlie  # Read Node 3's data from Node 1
```

**Check Metrics:**
```bash
curl -s http://localhost:8001/metrics | jq '{current_term, current_leader, last_applied}'
curl -s http://localhost:8002/metrics | jq '{current_term, current_leader, last_applied}'
curl -s http://localhost:8003/metrics | jq '{current_term, current_leader, last_applied}'
```

### Step 6: Verify S3 Storage

**Check S3 Buckets:**
```bash
# List objects in each bucket
mc ls local/scribe-ledger-node1
mc ls local/scribe-ledger-node2
mc ls local/scribe-ledger-node3
```

**Download and Inspect:**
```bash
# Download a segment from S3
mc cp local/scribe-ledger-node1/segment_0001 ./test-segment

# Check size
ls -lh ./test-segment
```

## 🧹 Cleanup

```bash
# Stop all nodes
pkill -f scribe-node

# Stop MinIO
pkill -f minio

# Clean data directories
rm -rf ./node-1 ./node-2 ./node-3 ./minio-data
```

## ✅ What to Test & Verify

### 1. Cluster Formation ✓
- [ ] All 3 nodes start successfully
- [ ] Nodes discover each other via UDP broadcast
- [ ] Raft consensus elects a leader (usually Node 1)

### 2. Data Replication ✓
- [ ] Write to one node, read from another
- [ ] Data consistency across all nodes
- [ ] Eventual consistency model works

### 3. S3 Integration ✓
- [ ] Segments archived to S3 buckets
- [ ] Each node uses separate bucket
- [ ] Data retrievable from S3

### 4. Fault Tolerance ✓
- [ ] Kill one node, cluster still works
- [ ] Kill leader, new leader elected
- [ ] Restart node, rejoins cluster

### 5. Performance ✓
- [ ] Multiple concurrent writes
- [ ] Read throughput across nodes
- [ ] Metrics show expected values

## 📊 Expected Results

**Healthy Cluster:**
```json
{
  "current_term": 1,
  "current_leader": 1,
  "last_applied": {
    "index": 3
  }
}
```

**Node Ports:**
- Node 1: HTTP 8001, Raft 9001, Discovery 17946
- Node 2: HTTP 8002, Raft 9002, Discovery 17947
- Node 3: HTTP 8003, Raft 9003, Discovery 17948

**S3 Buckets:**
- scribe-ledger-node1
- scribe-ledger-node2
- scribe-ledger-node3

## 🐛 Troubleshooting

**Nodes not discovering each other:**
- Check firewall settings for UDP port 17946-17948
- Verify different discovery ports in configs
- Check logs: `tail -f node*.log`

**S3 connection failures:**
- Verify MinIO is running: `curl http://localhost:9000/minio/health/live`
- Check credentials in config files
- Verify buckets exist: `mc ls local`

**Port conflicts:**
- Check if ports are in use: `lsof -i :8001 -i :9001`
- Kill existing processes: `pkill -f scribe-node`

## 📝 Notes

- **Bootstrap Mode:** Only Node 1 needs `--bootstrap` flag
- **Discovery:** Nodes use UDP broadcast for discovery
- **S3 Path Style:** Required for MinIO compatibility
- **Data Directory:** Each node uses separate directory (node-1, node-2, node-3)

## 🎯 Success Criteria

✅ All 3 nodes running without errors  
✅ Data written to one node readable from others  
✅ S3 buckets contain archived segments  
✅ Leader election works  
✅ Metrics endpoints return valid data  
✅ Cluster survives single node failure  

If all criteria pass, the cluster is working correctly! 🎉

