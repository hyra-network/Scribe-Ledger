#!/bin/bash

# Hyra Scribe Ledger Development Script

set -e

echo "🚀 Hyra Scribe Ledger Development Environment"
echo "============================================="

# Function to display help
show_help() {
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  build        Build the project"
    echo "  run-node     Run the Scribe node"
    echo "  run-dev      Run the Scribe node with MinIO configuration"
    echo "  test         Run tests"
    echo "  fmt          Format code"
    echo "  clippy       Run clippy lints"
    echo "  clean        Clean build artifacts"
    echo "  setup        Setup development environment"
    echo ""
    echo "MinIO/Storage Commands:"
    echo "  start-minio  Start MinIO using Docker Compose"
    echo "  stop-minio   Stop MinIO containers"
    echo "  minio-status Check MinIO status"
    echo "  minio-logs   Show MinIO logs"
    echo "  minio-test   Test MinIO connectivity and functionality"
    echo "  minio-reset  Reset MinIO data (destructive)"
    echo ""
    echo "  help         Show this help message"
    echo ""
}

# Setup development environment
setup_dev() {
    echo "📦 Setting up development environment..."
    
    # Copy environment file if it doesn't exist
    if [ ! -f .env ]; then
        cp .env.example .env
        echo "Created .env file from .env.example"
    fi
    
    # Create data directory
    mkdir -p data
    echo "Created data directory"
    
    # Install rust tools if not present
    if ! command -v rustfmt &> /dev/null; then
        rustup component add rustfmt
    fi
    
    if ! command -v cargo-clippy &> /dev/null; then
        rustup component add clippy
    fi
    
    echo "✅ Development environment setup complete!"
}

# Build the project
build_project() {
    echo "🔨 Building Scribe Ledger..."
    cargo build --release
    echo "✅ Build complete!"
}

# Run node
run_node() {
    echo "🚀 Starting Scribe Node..."
    RUST_LOG=info cargo run -- "$@"
}

# Run node with development configuration
run_dev_node() {
    echo "🚀 Starting Scribe Node (Development Mode with MinIO)..."
    if [ ! -f config-dev.toml ]; then
        echo "❌ config-dev.toml not found. Run './dev.sh setup' first."
        exit 1
    fi
    
    # Check if MinIO is running
    if ! docker-compose ps minio | grep -q "Up"; then
        echo "⚠️  MinIO is not running. Starting MinIO first..."
        start_minio
        sleep 5
    fi
    
    echo "🔧 Using development configuration with MinIO backend"
    RUST_LOG=debug SCRIBE_CONFIG=config-dev.toml cargo run -- "$@"
}

# MinIO management functions
start_minio() {
    echo "🗄️ Starting MinIO with Docker Compose..."
    docker-compose up -d minio
    echo "⏳ Waiting for MinIO to be ready..."
    sleep 10
    docker-compose up minio-init
    echo "✅ MinIO is running!"
    echo "📊 MinIO Console: http://localhost:9001"
    echo "🔐 Username: scribe-admin"
    echo "🔐 Password: scribe-password-123"
}

stop_minio() {
    echo "🛑 Stopping MinIO..."
    docker-compose down
    echo "✅ MinIO stopped!"
}

minio_status() {
    echo "📊 MinIO Status:"
    docker-compose ps
    echo ""
    if docker-compose ps minio | grep -q "Up"; then
        echo "✅ MinIO is running"
        echo "📊 Console: http://localhost:9001"
        echo "🚀 S3 API: http://localhost:9000"
    else
        echo "❌ MinIO is not running"
    fi
}

minio_logs() {
    echo "📋 MinIO Logs:"
    docker-compose logs -f minio
}

minio_test() {
    echo "🧪 Running MinIO connectivity tests..."
    if [ -f "./test-minio.sh" ]; then
        ./test-minio.sh
    else
        echo "❌ test-minio.sh not found"
        exit 1
    fi
}

minio_reset() {
    echo "⚠️  This will destroy all MinIO data!"
    read -p "Are you sure? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "🗑️ Resetting MinIO data..."
        docker-compose down -v
        docker volume rm scribe-ledger_minio_data 2>/dev/null || true
        echo "✅ MinIO data reset complete!"
        echo "Run './dev.sh start-minio' to start fresh"
    else
        echo "❌ Reset cancelled"
    fi
}

# Run tests
run_tests() {
    echo "🧪 Running tests..."
    cargo test
}

# Format code
format_code() {
    echo "✨ Formatting code..."
    cargo fmt
    echo "✅ Code formatted!"
}

# Run clippy
run_clippy() {
    echo "📎 Running clippy..."
    cargo clippy -- -D warnings
}

# Clean build artifacts
clean_build() {
    echo "🧹 Cleaning build artifacts..."
    cargo clean
    echo "✅ Clean complete!"
}

# Main script logic
case "${1:-help}" in
    "build")
        build_project
        ;;
    "run-node")
        shift
        run_node "$@"
        ;;
    "run-dev")
        shift
        run_dev_node "$@"
        ;;
    "test")
        run_tests
        ;;
    "fmt")
        format_code
        ;;
    "clippy")
        run_clippy
        ;;
    "clean")
        clean_build
        ;;
    "setup")
        setup_dev
        ;;
    "start-minio")
        start_minio
        ;;
    "stop-minio")
        stop_minio
        ;;
    "minio-status")
        minio_status
        ;;
    "minio-logs")
        minio_logs
        ;;
    "minio-test")
        minio_test
        ;;
    "minio-reset")
        minio_reset
        ;;
    "help"|*)
        show_help
        ;;
esac