#!/bin/bash
# Test runner script for Scribe Ledger

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if server is running
check_server() {
    local server_url=${1:-"http://localhost:8080"}
    print_status "Checking server at $server_url..."
    
    if curl -s -f "$server_url/health_check" > /dev/null 2>&1; then
        print_success "Server is running"
        return 0
    else
        print_warning "Server not responding at $server_url"
        return 1
    fi
}

# Run unit tests
run_unit_tests() {
    print_status "Running Rust unit tests..."
    cargo test --lib
    print_success "Unit tests completed"
}

# Run E2E tests
run_e2e_tests() {
    print_status "Running E2E functional tests..."
    cd tests/e2e
    python3 e2e_test.py
    cd ../..
    print_success "E2E tests completed"
}

# Run performance tests
run_performance_tests() {
    local server_url=${1:-"http://localhost:8080"}
    
    if ! check_server "$server_url"; then
        print_error "Server must be running for performance tests"
        return 1
    fi
    
    print_status "Running performance tests..."
    cd tests/e2e
    
    print_status "Quick performance test..."
    python3 quick_perf.py "$server_url"
    
    echo ""
    print_status "Comprehensive benchmark..."
    python3 benchmark.py
    
    cd ../..
    print_success "Performance tests completed"
}

# Run stress tests
run_stress_tests() {
    local server_url=${1:-"http://localhost:8080"}
    local duration=${2:-30}
    
    if ! check_server "$server_url"; then
        print_error "Server must be running for stress tests"
        return 1
    fi
    
    print_status "Running stress tests for ${duration}s..."
    cd tests/e2e
    python3 stress_test.py "$server_url" "$duration"
    cd ../..
    print_success "Stress tests completed"
}

# Start single node for testing
start_test_server() {
    print_status "Starting test server..."
    cargo build --release
    cargo run --release --bin scribe-node &
    SERVER_PID=$!
    
    # Wait for server to start
    sleep 3
    
    if check_server; then
        print_success "Test server started (PID: $SERVER_PID)"
        return 0
    else
        print_error "Failed to start test server"
        kill $SERVER_PID 2>/dev/null || true
        return 1
    fi
}

# Stop test server
stop_test_server() {
    if [ ! -z "$SERVER_PID" ]; then
        print_status "Stopping test server (PID: $SERVER_PID)..."
        kill $SERVER_PID 2>/dev/null || true
        wait $SERVER_PID 2>/dev/null || true
        print_success "Test server stopped"
    fi
}

# Show usage
show_usage() {
    echo "Scribe Ledger Test Runner"
    echo ""
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo ""
    echo "Commands:"
    echo "  unit                    Run Rust unit tests"
    echo "  e2e                     Run E2E functional tests"
    echo "  performance [URL]       Run performance tests (requires running server)"
    echo "  stress [URL] [DURATION] Run stress tests (requires running server)"
    echo "  all [URL]               Run all tests (requires running server)"
    echo "  server                  Start test server and run performance tests"
    echo ""
    echo "Options:"
    echo "  URL                     Server URL (default: http://localhost:8080)"
    echo "  DURATION               Stress test duration in seconds (default: 30)"
    echo ""
    echo "Examples:"
    echo "  $0 unit                                    # Run unit tests only"
    echo "  $0 performance                             # Run performance tests"
    echo "  $0 performance http://localhost:8081       # Run performance tests on custom port"
    echo "  $0 stress http://localhost:8080 60         # Run 60-second stress test"
    echo "  $0 server                                  # Start server and run tests"
    echo "  $0 all                                     # Run all tests (server must be running)"
}

# Main script logic
main() {
    case "${1:-}" in
        "unit")
            run_unit_tests
            ;;
        "e2e")
            run_e2e_tests
            ;;
        "performance")
            run_performance_tests "${2:-http://localhost:8080}"
            ;;
        "stress")
            run_stress_tests "${2:-http://localhost:8080}" "${3:-30}"
            ;;
        "all")
            local server_url="${2:-http://localhost:8080}"
            run_unit_tests
            echo ""
            run_performance_tests "$server_url"
            echo ""
            run_stress_tests "$server_url" 30
            ;;
        "server")
            # Start server, run tests, then stop server
            if start_test_server; then
                echo ""
                run_performance_tests "http://localhost:8080"
                echo ""
                print_status "Running quick stress test..."
                run_stress_tests "http://localhost:8080" 20
                stop_test_server
            fi
            ;;
        "help"|"-h"|"--help"|"")
            show_usage
            ;;
        *)
            print_error "Unknown command: $1"
            echo ""
            show_usage
            exit 1
            ;;
    esac
}

# Cleanup on exit
cleanup() {
    stop_test_server
}
trap cleanup EXIT

# Run main function
main "$@"