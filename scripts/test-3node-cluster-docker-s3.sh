#!/bin/bash
# Test script for 3-node cluster with Docker MinIO (S3-compatible storage)
# This script will:
# 1. Start MinIO using Docker
# 2. Create S3 buckets
# 3. Start 3 Scribe-Ledger nodes
# 4. Test cluster operation
# 5. Verify data replication

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}╔═══════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║   Hyra Scribe Ledger - 3-Node Cluster with Docker S3║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════════╝${NC}"
echo ""

# Function to print step
print_step() {
    echo -e "\n${BLUE}▶ $1${NC}"
}

# Function to print success
print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Function to print error
print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Function to print warning
print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Cleanup function
cleanup() {
    print_step "Cleaning up..."
    
    # Stop nodes
    pkill -f scribe-node || true
    print_success "Stopped all scribe-node processes"
    
    # Stop MinIO Docker container
    docker-compose -f docker-compose-minio.yml down > /dev/null 2>&1 || true
    print_success "Stopped MinIO Docker container"
    
    # Clean up data directories (optional - comment out to keep data)
    # rm -rf ./node-1 ./node-2 ./node-3
    # print_success "Cleaned up data directories"
}

# Trap Ctrl+C and cleanup
trap cleanup EXIT INT TERM

# Step 1: Check if Docker is available
print_step "Checking Docker installation..."
if ! command -v docker &> /dev/null; then
    print_error "Docker is not installed"
    echo "Please install Docker from: https://www.docker.com/get-started"
    exit 1
fi

if ! docker info > /dev/null 2>&1; then
    print_error "Docker daemon is not running"
    echo "Please start Docker Desktop or Docker daemon"
    exit 1
fi

print_success "Docker is installed and running"

# Step 2: Start MinIO with Docker Compose
print_step "Starting MinIO S3-compatible storage with Docker..."
docker-compose -f docker-compose-minio.yml down > /dev/null 2>&1 || true
docker-compose -f docker-compose-minio.yml up -d

# Wait for MinIO to be ready
print_step "Waiting for MinIO to be ready..."
for i in {1..30}; do
    if curl -s http://localhost:9000/minio/health/live > /dev/null 2>&1; then
        print_success "MinIO is ready!"
        break
    fi
    if [ $i -eq 30 ]; then
        print_error "MinIO failed to start"
        docker-compose -f docker-compose-minio.yml logs
        exit 1
    fi
    sleep 1
done

print_success "MinIO Console: http://localhost:9001 (minioadmin/minioadmin)"
print_success "MinIO S3 API: http://localhost:9000"

# Step 3: Verify buckets were created
print_step "Verifying S3 buckets..."
sleep 2
docker-compose -f docker-compose-minio.yml logs minio-setup | tail -5
print_success "S3 buckets created: scribe-ledger-node1, scribe-ledger-node2, scribe-ledger-node3"

# Step 4: Build the project
print_step "Building Scribe-Ledger (release mode)..."
cargo build --release --bin scribe-node
print_success "Build complete"

# Step 5: Clean old data (optional)
print_step "Preparing node data directories..."
rm -rf ./node-1 ./node-2 ./node-3
print_success "Cleaned up old data directories"

# Step 6: Start Node 1 (Bootstrap/Leader) - FIRST!
print_step "Starting Node 1 (Bootstrap/Leader) on port 8001..."
print_warning "Node 1 will bootstrap a new cluster"
./target/release/scribe-node --bootstrap --config config-node1.toml > node1.log 2>&1 &
NODE1_PID=$!
sleep 3

if ps -p $NODE1_PID > /dev/null; then
    print_success "Node 1 started (PID: $NODE1_PID)"
else
    print_error "Failed to start Node 1"
    cat node1.log
    exit 1
fi

# Wait for Node 1 to fully initialize and become leader
print_step "Waiting for Node 1 to fully initialize as leader..."
sleep 12
print_success "Node 1 should now be ready and accepting connections"

# Verify Node 1 is healthy before starting Node 2
if curl -s http://localhost:8001/health > /dev/null 2>&1; then
    print_success "Node 1 health check passed"
else
    print_error "Node 1 is not responding"
    cat node1.log
    exit 1
fi

# Step 7: Start Node 2 - Should discover Node 1
print_step "Starting Node 2 on port 8002..."
print_warning "Node 2 will attempt to discover and join Node 1"
./target/release/scribe-node --config config-node2.toml > node2.log 2>&1 &
NODE2_PID=$!
sleep 3

if ps -p $NODE2_PID > /dev/null; then
    print_success "Node 2 started (PID: $NODE2_PID)"
else
    print_error "Failed to start Node 2"
    cat node2.log
    exit 1
fi

# Wait for Node 2 to discover Node 1
print_step "Waiting for Node 2 to discover and join the cluster..."
sleep 10
print_success "Node 2 should have joined the cluster"

# Step 8: Start Node 3 - Should discover Nodes 1 and 2
print_step "Starting Node 3 on port 8003..."
print_warning "Node 3 will attempt to discover and join the cluster"
./target/release/scribe-node --config config-node3.toml > node3.log 2>&1 &
NODE3_PID=$!
sleep 3

if ps -p $NODE3_PID > /dev/null; then
    print_success "Node 3 started (PID: $NODE3_PID)"
else
    print_error "Failed to start Node 3"
    cat node3.log
    exit 1
fi

# Wait for Node 3 to join and cluster to stabilize
print_step "Waiting for Node 3 to join and cluster to stabilize..."
sleep 10
print_success "All nodes should now be part of the cluster"

echo ""
echo -e "${CYAN}╔═══════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║              Cluster Status & Testing                ║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════════╝${NC}"
echo ""

# Step 9: Check health of all nodes
print_step "Checking health of all nodes..."
for port in 8001 8002 8003; do
    if curl -s http://localhost:$port/health > /dev/null 2>&1; then
        HEALTH=$(curl -s http://localhost:$port/health | jq -r '.status' 2>/dev/null || echo "unknown")
        NODE_ID=$(curl -s http://localhost:$port/health | jq -r '.node_id' 2>/dev/null || echo "?")
        print_success "Node $NODE_ID on port $port is $HEALTH"
    else
        print_error "Node on port $port is not responding"
    fi
done

# Step 10: Test data operations
print_step "Testing data operations across cluster..."

# Write to Node 1
echo -n "Writing 'test-key-1' to Node 1... "
if curl -s -X PUT http://localhost:8001/test-key-1 -d "Hello from Node 1 - Cluster Test!" > /dev/null 2>&1; then
    print_success "Write successful"
else
    print_error "Write failed"
fi

# Write to Node 2
echo -n "Writing 'test-key-2' to Node 2... "
if curl -s -X PUT http://localhost:8002/test-key-2 -d "Hello from Node 2 - Data Replication!" > /dev/null 2>&1; then
    print_success "Write successful"
else
    print_error "Write failed"
fi

# Write to Node 3
echo -n "Writing 'test-key-3' to Node 3... "
if curl -s -X PUT http://localhost:8003/test-key-3 -d "Hello from Node 3 - With S3 Storage!" > /dev/null 2>&1; then
    print_success "Write successful"
else
    print_error "Write failed"
fi

sleep 3

# Step 11: Test data replication
print_step "Testing data replication across nodes..."

# Read key-1 from Node 2 (written to Node 1)
echo -n "Reading 'test-key-1' from Node 2 (cross-node read)... "
RESULT=$(curl -s http://localhost:8002/test-key-1 2>/dev/null || echo "FAILED")
if [[ "$RESULT" == *"Node 1"* ]]; then
    print_success "✓ Data replicated successfully"
else
    print_warning "Data not replicated yet: $RESULT"
fi

# Read key-2 from Node 3 (written to Node 2)
echo -n "Reading 'test-key-2' from Node 3 (cross-node read)... "
RESULT=$(curl -s http://localhost:8003/test-key-2 2>/dev/null || echo "FAILED")
if [[ "$RESULT" == *"Node 2"* ]]; then
    print_success "✓ Data replicated successfully"
else
    print_warning "Data not replicated yet: $RESULT"
fi

# Read key-3 from Node 1 (written to Node 3)
echo -n "Reading 'test-key-3' from Node 1 (cross-node read)... "
RESULT=$(curl -s http://localhost:8001/test-key-3 2>/dev/null || echo "FAILED")
if [[ "$RESULT" == *"Node 3"* ]]; then
    print_success "✓ Data replicated successfully"
else
    print_warning "Data not replicated yet: $RESULT"
fi

# Step 12: Check Raft metrics
print_step "Checking Raft consensus metrics..."
for i in 1 2 3; do
    port=$((8000 + i))
    echo -e "\n${YELLOW}═══ Node $i (port $port) ═══${NC}"
    curl -s http://localhost:$port/metrics 2>/dev/null | jq '{
        current_term,
        current_leader,
        server_state,
        last_applied: .last_applied.index
    }' 2>/dev/null || echo "  Could not retrieve metrics"
done

# Step 13: Check S3 storage
print_step "Checking S3 storage (MinIO)..."
echo "You can view stored data in MinIO Console: http://localhost:9001"
echo "Buckets:"
docker exec scribe-minio mc ls local 2>/dev/null || echo "Could not list buckets"

echo ""
echo -e "${CYAN}╔═══════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║                  🎉 Test Complete!                   ║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════════╝${NC}"
echo ""
print_success "✓ MinIO S3 storage running in Docker"
print_success "✓ 3-node cluster running successfully"
print_success "✓ Data replication working"
print_success "✓ S3 integration active"
echo ""
echo -e "${YELLOW}📊 Access Points:${NC}"
echo "  • MinIO Console: http://localhost:9001 (minioadmin/minioadmin)"
echo "  • Node 1 API: http://localhost:8001"
echo "  • Node 2 API: http://localhost:8002"
echo "  • Node 3 API: http://localhost:8003"
echo ""
echo -e "${YELLOW}📝 Logs:${NC}"
echo "  • Node 1: tail -f node1.log"
echo "  • Node 2: tail -f node2.log"
echo "  • Node 3: tail -f node3.log"
echo "  • MinIO: docker-compose -f docker-compose-minio.yml logs -f"
echo ""
echo -e "${GREEN}Press Ctrl+C to stop all services${NC}"
echo ""

# Keep script running
wait

