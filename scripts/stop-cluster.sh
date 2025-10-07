#!/bin/bash
# Stop the Simple Scribe Ledger cluster
# This script gracefully shuts down all running nodes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PID_DIR="${PROJECT_DIR}/pids"

# Function to stop a node
stop_node() {
    local node_id=$1
    local pid_file="${PID_DIR}/node${node_id}.pid"
    
    if [ ! -f "$pid_file" ]; then
        echo -e "${YELLOW}  Node $node_id: PID file not found${NC}"
        return 0
    fi
    
    local pid=$(cat "$pid_file")
    
    if kill -0 $pid 2>/dev/null; then
        echo -e "${GREEN}Stopping node $node_id (PID: $pid)...${NC}"
        
        # Send SIGTERM for graceful shutdown
        kill -TERM $pid
        
        # Wait for the process to terminate (max 10 seconds)
        local count=0
        while kill -0 $pid 2>/dev/null && [ $count -lt 10 ]; do
            sleep 1
            count=$((count + 1))
        done
        
        # Force kill if still running
        if kill -0 $pid 2>/dev/null; then
            echo -e "${YELLOW}  Node $node_id did not stop gracefully, forcing shutdown...${NC}"
            kill -9 $pid 2>/dev/null || true
            sleep 1
        fi
        
        echo -e "${GREEN}  Node $node_id stopped${NC}"
    else
        echo -e "${YELLOW}  Node $node_id: Process not running (PID: $pid)${NC}"
    fi
    
    rm -f "$pid_file"
}

# Main execution
echo "=========================================="
echo "Simple Scribe Ledger - Cluster Shutdown"
echo "=========================================="
echo

# Stop all nodes
stop_node 1
stop_node 2
stop_node 3

echo
echo "=========================================="
echo -e "${GREEN}Cluster shutdown complete!${NC}"

# Optional: Clean up data directories
read -p "Do you want to clean up data directories? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Cleaning up data directories..."
    rm -rf "${PROJECT_DIR}/node-1" 2>/dev/null || true
    rm -rf "${PROJECT_DIR}/node-2" 2>/dev/null || true
    rm -rf "${PROJECT_DIR}/node-3" 2>/dev/null || true
    echo -e "${GREEN}Data directories cleaned${NC}"
fi

echo "=========================================="
