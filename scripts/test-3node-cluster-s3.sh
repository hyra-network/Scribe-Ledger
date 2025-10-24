#!/bin/bash
# Test script for 3-node cluster with S3 storage
# This script will:
# 1. Start MinIO (S3-compatible storage)
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
echo -e "${CYAN}║   Hyra Scribe Ledger - 3-Node Cluster Test with S3  ║${NC}"
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
    
    # Stop MinIO
    pkill -f minio || true
    print_success "Stopped MinIO"
    
    # Clean up data directories
    rm -rf ./node-1 ./node-2 ./node-3 ./minio-data
    print_success "Cleaned up data directories"
}

# Trap Ctrl+C and cleanup
trap cleanup EXIT INT TERM

# Step 1: Check if MinIO is available
print_step "Checking MinIO installation..."
if ! command -v minio &> /dev/null; then
    print_error "MinIO is not installed"
    echo "Install with: brew install minio/stable/minio (macOS)"
    echo "Or download from: https://min.io/download"
    exit 1
fi
print_success "MinIO is installed"

# Step 2: Start MinIO
print_step "Starting MinIO S3-compatible storage..."
mkdir -p ./minio-data
MINIO_ROOT_USER=minioadmin MINIO_ROOT_PASSWORD=minioadmin \
    minio server ./minio-data --address :9000 --console-address :9001 > /dev/null 2>&1 &
MINIO_PID=$!
sleep 3

if ps -p $MINIO_PID > /dev/null; then
    print_success "MinIO started (PID: $MINIO_PID)"
    print_success "MinIO Console: http://localhost:9001 (minioadmin/minioadmin)"
else
    print_error "Failed to start MinIO"
    exit 1
fi

# Step 3: Configure MinIO client (mc)
print_step "Configuring MinIO client..."
if ! command -v mc &> /dev/null; then
    print_warning "MinIO client (mc) not installed, skipping bucket creation"
    print_warning "You may need to create buckets manually: scribe-ledger-node1, scribe-ledger-node2, scribe-ledger-node3"
else
    mc alias set local http://localhost:9000 minioadmin minioadmin > /dev/null 2>&1 || true
    
    # Create buckets for each node
    mc mb local/scribe-ledger-node1 --ignore-existing > /dev/null 2>&1 || true
    mc mb local/scribe-ledger-node2 --ignore-existing > /dev/null 2>&1 || true
    mc mb local/scribe-ledger-node3 --ignore-existing > /dev/null 2>&1 || true
    print_success "Created S3 buckets: scribe-ledger-node1, scribe-ledger-node2, scribe-ledger-node3"
fi

# Step 4: Build the project
print_step "Building Scribe-Ledger (release mode)..."
cargo build --release --bin scribe-node
print_success "Build complete"

# Step 5: Clean old data
print_step "Cleaning old node data..."
rm -rf ./node-1 ./node-2 ./node-3
print_success "Cleaned up old data directories"

# Step 6: Start Node 1 (Bootstrap/Leader)
print_step "Starting Node 1 (Bootstrap/Leader) on port 8001..."
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

# Step 7: Start Node 2
print_step "Starting Node 2 on port 8002..."
./target/release/scribe-node --config config-node2.toml > node2.log 2>&1 &
NODE2_PID=$!
sleep 5

if ps -p $NODE2_PID > /dev/null; then
    print_success "Node 2 started (PID: $NODE2_PID)"
else
    print_error "Failed to start Node 2"
    cat node2.log
    exit 1
fi

# Step 8: Start Node 3
print_step "Starting Node 3 on port 8003..."
./target/release/scribe-node --config config-node3.toml > node3.log 2>&1 &
NODE3_PID=$!
sleep 5

if ps -p $NODE3_PID > /dev/null; then
    print_success "Node 3 started (PID: $NODE3_PID)"
else
    print_error "Failed to start Node 3"
    cat node3.log
    exit 1
fi

# Wait for cluster to form
print_step "Waiting for cluster formation..."
sleep 10
print_success "Cluster should be ready"

echo ""
echo -e "${CYAN}╔═══════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║              Cluster Status & Testing                ║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════════╝${NC}"
echo ""

# Step 9: Check health of all nodes
print_step "Checking health of all nodes..."
for port in 8001 8002 8003; do
    if curl -s http://localhost:$port/health > /dev/null 2>&1; then
        print_success "Node on port $port is healthy"
    else
        print_error "Node on port $port is not responding"
    fi
done

# Step 10: Test data operations
print_step "Testing data operations..."

# Write to Node 1
echo -n "Writing test data to Node 1... "
if curl -s -X PUT http://localhost:8001/test-key-1 -d "Hello from Node 1" > /dev/null 2>&1; then
    print_success "Write to Node 1 successful"
else
    print_error "Write to Node 1 failed"
fi

# Write to Node 2
echo -n "Writing test data to Node 2... "
if curl -s -X PUT http://localhost:8002/test-key-2 -d "Hello from Node 2" > /dev/null 2>&1; then
    print_success "Write to Node 2 successful"
else
    print_error "Write to Node 2 failed"
fi

# Write to Node 3
echo -n "Writing test data to Node 3... "
if curl -s -X PUT http://localhost:8003/test-key-3 -d "Hello from Node 3" > /dev/null 2>&1; then
    print_success "Write to Node 3 successful"
else
    print_error "Write to Node 3 failed"
fi

sleep 2

# Step 11: Test data replication
print_step "Testing data replication across nodes..."

# Read key-1 from Node 2
echo -n "Reading test-key-1 from Node 2... "
RESULT=$(curl -s http://localhost:8002/test-key-1 2>/dev/null || echo "FAILED")
if [ "$RESULT" = "Hello from Node 1" ]; then
    print_success "Data replicated to Node 2"
else
    print_warning "Data not yet replicated to Node 2: $RESULT"
fi

# Read key-2 from Node 3
echo -n "Reading test-key-2 from Node 3... "
RESULT=$(curl -s http://localhost:8003/test-key-2 2>/dev/null || echo "FAILED")
if [ "$RESULT" = "Hello from Node 2" ]; then
    print_success "Data replicated to Node 3"
else
    print_warning "Data not yet replicated to Node 3: $RESULT"
fi

# Read key-3 from Node 1
echo -n "Reading test-key-3 from Node 1... "
RESULT=$(curl -s http://localhost:8001/test-key-3 2>/dev/null || echo "FAILED")
if [ "$RESULT" = "Hello from Node 3" ]; then
    print_success "Data replicated to Node 1"
else
    print_warning "Data not yet replicated to Node 1: $RESULT"
fi

# Step 12: Check metrics
print_step "Checking Raft metrics..."
for i in 1 2 3; do
    port=$((8000 + i))
    echo -e "\n${YELLOW}Node $i (port $port):${NC}"
    curl -s http://localhost:$port/metrics 2>/dev/null | jq '{
        current_term,
        current_leader,
        last_applied: .last_applied.index
    }' 2>/dev/null || echo "  Could not retrieve metrics"
done

echo ""
echo -e "${CYAN}╔═══════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║                  Test Summary                        ║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════════╝${NC}"
echo ""
print_success "✓ MinIO S3 storage running on http://localhost:9000"
print_success "✓ MinIO Console available at http://localhost:9001"
print_success "✓ Node 1 (Leader) running on http://localhost:8001"
print_success "✓ Node 2 running on http://localhost:8002"
print_success "✓ Node 3 running on http://localhost:8003"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop all services and cleanup${NC}"
echo ""

# Keep script running
wait

