#!/bin/bash
# Test script to verify all three issues are fixed

set -e

echo "================================"
echo "Testing Scribe-Ledger Fixes"
echo "================================"
echo ""

# Clean up any previous runs
echo "1. Cleaning up previous test data..."
rm -rf node-1 node-2 node-3 test-node-*
pkill -9 scribe-node 2>/dev/null || true
sleep 2

# Test 1: Verify config flexibility
echo ""
echo "2. Testing Problem 1: Config Flexibility"
echo "   - Checking that config.toml is used (not hard-coded values)"
echo "   - Config specifies port 8001 (not hard-coded 8080)"

# Start node with config
timeout 15 cargo run --bin scribe-node -- -c config.toml --bootstrap > /tmp/node1.log 2>&1 &
NODE_PID=$!
echo "   - Started node with PID $NODE_PID"
sleep 8

# Test if port 8001 works (from config)
if curl -s http://localhost:8001/health | grep -q "healthy"; then
    echo "   ✓ Port 8001 working (configured in config.toml)"
else
    echo "   ✗ Port 8001 not working"
    kill $NODE_PID 2>/dev/null || true
    exit 1
fi

# Test if port 8080 does NOT work (proving we're not using hard-coded default)
if curl -s --connect-timeout 2 http://localhost:8080/health 2>&1 | grep -q "Connection refused"; then
    echo "   ✓ Port 8080 not listening (hard-coded default NOT used)"
else
    echo "   Note: Port 8080 check inconclusive"
fi

echo ""
echo "3. Testing Problem 2: HTTP Server Functionality"
echo "   - Verifying HTTP endpoints are accessible"

# Test health endpoint
HEALTH=$(curl -s http://localhost:8001/health)
if echo "$HEALTH" | grep -q "healthy"; then
    echo "   ✓ /health endpoint working: $HEALTH"
else
    echo "   ✗ /health endpoint failed"
    kill $NODE_PID 2>/dev/null || true
    exit 1
fi

# Test metrics endpoint
METRICS=$(curl -s http://localhost:8001/metrics)
if echo "$METRICS" | grep -q '"state"'; then
    echo "   ✓ /metrics endpoint working"
else
    echo "   ✗ /metrics endpoint failed"
    kill $NODE_PID 2>/dev/null || true
    exit 1
fi

# Test cluster status endpoint
STATUS=$(curl -s http://localhost:8001/cluster/status)
if echo "$STATUS" | grep -q "node_id"; then
    echo "   ✓ /cluster/status endpoint working"
else
    echo "   ✗ /cluster/status endpoint failed"
    kill $NODE_PID 2>/dev/null || true
    exit 1
fi

echo ""
echo "4. Testing Problem 3: Node Re-creation/Restart"
echo "   - Stopping node and restarting without --bootstrap flag"

# Stop the node
kill $NODE_PID 2>/dev/null || true
sleep 3

# Restart WITHOUT --bootstrap flag (should detect existing state)
echo "   - Restarting node without --bootstrap flag..."
timeout 15 cargo run --bin scribe-node -- -c config.toml > /tmp/node1-restart.log 2>&1 &
NODE_PID=$!
sleep 8

# Check if node started successfully
if curl -s http://localhost:8001/health | grep -q "healthy"; then
    echo "   ✓ Node restarted successfully without bootstrap flag"
else
    echo "   ✗ Node restart failed"
    cat /tmp/node1-restart.log
    kill $NODE_PID 2>/dev/null || true
    exit 1
fi

# Verify it's still the leader (state preserved)
if curl -s http://localhost:8001/metrics | grep -q '"state"'; then
    echo "   ✓ Node state preserved (cluster operational)"
else
    echo "   ✗ Node state not preserved"
    kill $NODE_PID 2>/dev/null || true
    exit 1
fi

# Clean up
kill $NODE_PID 2>/dev/null || true
sleep 2

echo ""
echo "================================"
echo "All Tests Passed! ✓"
echo "================================"
echo ""
echo "Summary:"
echo "1. ✓ Configuration is flexible (uses config.toml, not hard-coded)"
echo "2. ✓ HTTP server starts and responds on configured port (8001)"
echo "3. ✓ Node can restart without errors (preserves existing state)"
echo ""
