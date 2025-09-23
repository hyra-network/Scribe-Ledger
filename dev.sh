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
    echo "  test         Run tests"
    echo "  fmt          Format code"
    echo "  clippy       Run clippy lints"
    echo "  clean        Clean build artifacts"
    echo "  setup        Setup development environment"
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
    "help"|*)
        show_help
        ;;
esac