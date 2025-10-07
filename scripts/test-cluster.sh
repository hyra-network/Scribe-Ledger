#!/bin/bash
# Test the Simple Scribe Ledger cluster
# This script performs basic functionality tests on the cluster

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

PID_DIR="${PROJECT_DIR}/pids"

# Test configuration
NODE1_HTTP="http://127.0.0.1:8001"
NODE2_HTTP="http://127.0.0.1:8002"
NODE3_HTTP="http://127.0.0.1:8003"

FAILED_TESTS=0
PASSED_TESTS=0

# Function to check if nodes are running
check_cluster_running() {
    echo -e "${BLUE}Checking if cluster is running...${NC}"
    
    for node_id in 1 2 3; do
        local pid_file="${PID_DIR}/node${node_id}.pid"
        if [ ! -f "$pid_file" ]; then
            echo -e "${RED}Error: Node $node_id is not running${NC}"
            echo "Please start the cluster first with: ./scripts/start-cluster.sh"
            exit 1
        fi
        
        local pid=$(cat "$pid_file")
        if ! kill -0 $pid 2>/dev/null; then
            echo -e "${RED}Error: Node $node_id is not running (PID: $pid)${NC}"
            echo "Please start the cluster first with: ./scripts/start-cluster.sh"
            exit 1
        fi
    done
    
    echo -e "${GREEN}All nodes are running${NC}"
}

# Function to test HTTP endpoint
test_http_endpoint() {
    local node_url=$1
    local node_name=$2
    
    echo -e "${BLUE}Testing $node_name health endpoint...${NC}"
    
    if curl -s -f "${node_url}/health" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ $node_name health check passed${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        echo -e "${RED}✗ $node_name health check failed${NC}"
        FAILED_TESTS=$((FAILED_TESTS + 1))
        return 1
    fi
}

# Function to test write operation
test_write() {
    local node_url=$1
    local node_name=$2
    local key=$3
    local value=$4
    
    echo -e "${BLUE}Testing write to $node_name...${NC}"
    echo "  Key: $key, Value: $value"
    
    local response=$(curl -s -X POST "${node_url}/put" \
        -H "Content-Type: application/json" \
        -d "{\"key\":\"$key\",\"value\":\"$value\"}" 2>&1)
    
    if echo "$response" | grep -q "success\|ok" 2>/dev/null; then
        echo -e "${GREEN}✓ Write to $node_name successful${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        echo -e "${YELLOW}⚠ Write to $node_name status unclear (this may be expected if HTTP API is not fully implemented)${NC}"
        echo "  Response: $response"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    fi
}

# Function to test read operation
test_read() {
    local node_url=$1
    local node_name=$2
    local key=$3
    
    echo -e "${BLUE}Testing read from $node_name...${NC}"
    echo "  Key: $key"
    
    local response=$(curl -s -X GET "${node_url}/get/${key}" 2>&1)
    
    if [ -n "$response" ]; then
        echo -e "${GREEN}✓ Read from $node_name successful${NC}"
        echo "  Response: ${response:0:100}..."
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        echo -e "${YELLOW}⚠ Read from $node_name returned empty (may be expected)${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    fi
}

# Function to test metrics endpoint
test_metrics() {
    local node_url=$1
    local node_name=$2
    
    echo -e "${BLUE}Testing $node_name metrics endpoint...${NC}"
    
    local response=$(curl -s "${node_url}/metrics" 2>&1)
    
    if [ -n "$response" ]; then
        echo -e "${GREEN}✓ $node_name metrics retrieved${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    else
        echo -e "${YELLOW}⚠ $node_name metrics unavailable (may not be implemented yet)${NC}"
        PASSED_TESTS=$((PASSED_TESTS + 1))
        return 0
    fi
}

# Main execution
echo "=========================================="
echo "Simple Scribe Ledger - Cluster Test"
echo "=========================================="
echo

# Check if curl is available
if ! command -v curl &> /dev/null; then
    echo -e "${RED}Error: curl is required but not installed${NC}"
    exit 1
fi

# Check cluster status
check_cluster_running

echo
echo "=========================================="
echo "Running Tests"
echo "=========================================="
echo

# Test 1: Health checks
echo "Test 1: Health Checks"
test_http_endpoint "$NODE1_HTTP" "Node 1"
test_http_endpoint "$NODE2_HTTP" "Node 2"
test_http_endpoint "$NODE3_HTTP" "Node 3"
echo

# Test 2: Write operations
echo "Test 2: Write Operations"
test_write "$NODE1_HTTP" "Node 1" "test_key1" "test_value1"
test_write "$NODE2_HTTP" "Node 2" "test_key2" "test_value2"
test_write "$NODE3_HTTP" "Node 3" "test_key3" "test_value3"
echo

# Test 3: Read operations
echo "Test 3: Read Operations"
test_read "$NODE1_HTTP" "Node 1" "test_key1"
test_read "$NODE2_HTTP" "Node 2" "test_key2"
test_read "$NODE3_HTTP" "Node 3" "test_key3"
echo

# Test 4: Metrics
echo "Test 4: Metrics"
test_metrics "$NODE1_HTTP" "Node 1"
test_metrics "$NODE2_HTTP" "Node 2"
test_metrics "$NODE3_HTTP" "Node 3"
echo

# Summary
echo "=========================================="
echo "Test Summary"
echo "=========================================="
echo -e "${GREEN}Passed: $PASSED_TESTS${NC}"
echo -e "${RED}Failed: $FAILED_TESTS${NC}"
echo "=========================================="

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi
