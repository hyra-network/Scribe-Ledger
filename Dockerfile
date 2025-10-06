# Multi-stage build for Simple Scribe Ledger
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY benches ./benches
COPY examples ./examples
COPY tests ./tests

# Build release binary
RUN cargo build --release --bin scribe-node

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 scribe

# Create directories
RUN mkdir -p /var/lib/scribe-ledger && \
    chown -R scribe:scribe /var/lib/scribe-ledger

# Copy binary from builder
COPY --from=builder /app/target/release/scribe-node /usr/local/bin/

# Copy default configs
COPY config-node1.toml /etc/scribe-ledger/config.toml

# Set user
USER scribe

# Set working directory
WORKDIR /var/lib/scribe-ledger

# Expose ports (HTTP and Raft)
EXPOSE 8001 9001

# Health check
HEALTHCHECK --interval=10s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8001/health || exit 1

# Run the binary
ENTRYPOINT ["/usr/local/bin/scribe-node"]
CMD ["--config", "/etc/scribe-ledger/config.toml", "--log-level", "info"]
