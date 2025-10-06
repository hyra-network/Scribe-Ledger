#!/bin/bash
# Start a 3-node Simple Scribe Ledger cluster
# This script starts three nodes with different configurations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
NODE_BINARY="${PROJECT_DIR}/target/release/scribe-node"
if [ ! -f "$NODE_BINARY" ]; then
    NODE_BINARY="${PROJECT_DIR}/target/debug/scribe-node"
fi

LOG_DIR="${PROJECT_DIR}/logs"
PID_DIR="${PROJECT_DIR}/pids"

# Create necessary directories
mkdir -p "$LOG_DIR"
mkdir -p "$PID_DIR"

# Check if binary exists
if [ ! -f "$NODE_BINARY" ]; then
    echo -e "${RED}Error: scribe-node binary not found${NC}"
    echo "Please build the project first with: cargo build --bin scribe-node"
    exit 1
fi

# Check if nodes are already running
if [ -f "$PID_DIR/node1.pid" ] || [ -f "$PID_DIR/node2.pid" ] || [ -f "$PID_DIR/node3.pid" ]; then
    echo -e "${YELLOW}Warning: Some nodes may already be running${NC}"
    echo "Run './scripts/stop-cluster.sh' first to stop existing nodes"
    read -p "Continue anyway? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Function to start a node
start_node() {
    local node_id=$1
    local config_file="${PROJECT_DIR}/config-node${node_id}.toml"
    local log_file="${LOG_DIR}/node${node_id}.log"
    local pid_file="${PID_DIR}/node${node_id}.pid"
    
    if [ ! -f "$config_file" ]; then
        echo -e "${RED}Error: Config file $config_file not found${NC}"
        return 1
    fi
    
    echo -e "${GREEN}Starting node $node_id...${NC}"
    
    # Start the node in the background
    nohup "$NODE_BINARY" --config "$config_file" --log-level info > "$log_file" 2>&1 &
    
    local pid=$!
    echo $pid > "$pid_file"
    
    echo "  Node $node_id started with PID $pid"
    echo "  Log: $log_file"
    echo "  Config: $config_file"
}

# Function to check if a node is running
check_node() {
    local node_id=$1
    local pid_file="${PID_DIR}/node${node_id}.pid"
    
    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            echo -e "${GREEN}  Node $node_id is running (PID: $pid)${NC}"
            return 0
        else
            echo -e "${RED}  Node $node_id is not running (stale PID file)${NC}"
            rm -f "$pid_file"
            return 1
        fi
    else
        echo -e "${RED}  Node $node_id is not running${NC}"
        return 1
    fi
}

# Main execution
echo "=========================================="
echo "Simple Scribe Ledger - Cluster Startup"
echo "=========================================="
echo

# Start all nodes
start_node 1
sleep 1
start_node 2
sleep 1
start_node 3

echo
echo "=========================================="
echo "Waiting for nodes to initialize..."
sleep 3

# Check node status
echo "Checking node status..."
check_node 1
check_node 2
check_node 3

echo
echo "=========================================="
echo -e "${GREEN}Cluster startup complete!${NC}"
echo
echo "Cluster Information:"
echo "  Node 1: http://127.0.0.1:8001 (Raft: 9001)"
echo "  Node 2: http://127.0.0.1:8002 (Raft: 9002)"
echo "  Node 3: http://127.0.0.1:8003 (Raft: 9003)"
echo
echo "Logs: $LOG_DIR"
echo "PIDs: $PID_DIR"
echo
echo "To stop the cluster: ./scripts/stop-cluster.sh"
echo "To test the cluster: ./scripts/test-cluster.sh"
echo "=========================================="
