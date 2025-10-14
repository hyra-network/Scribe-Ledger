# Configuration Reference

Complete configuration documentation for Hyra Scribe Ledger.

## Table of Contents

- [Configuration File Format](#configuration-file-format)
- [Node Configuration](#node-configuration)
- [Network Configuration](#network-configuration)
- [Storage Configuration](#storage-configuration)
- [Consensus Configuration](#consensus-configuration)
- [Security Configuration](#security-configuration)
- [Logging Configuration](#logging-configuration)
- [Performance Configuration](#performance-configuration)
- [Environment Variables](#environment-variables)

## Configuration File Format

Configuration is specified in TOML format:

```toml
# config.toml
[node]
# Node settings

[network]
# Network settings

[storage]
# Storage settings

[consensus]
# Consensus settings

[security]
# Security settings

[logging]
# Logging settings
```

## Node Configuration

```toml
[node]
# Unique node identifier (required)
# Must be unique across all cluster nodes
id = 1

# Node address for cluster communication (required)
# Format: "host:port"
# Must be reachable by other cluster nodes
address = "10.0.1.10:8001"

# Data directory for persistent storage (required)
# Path to store database files
data_dir = "/var/lib/scribe-ledger"
```

**Defaults:**
- `id`: No default (required)
- `address`: No default (required)
- `data_dir`: `"./data"`

**Environment Variable Overrides:**
- `SCRIBE_NODE_ID`
- `SCRIBE_NODE_ADDRESS`
- `SCRIBE_NODE_DATA_DIR`

## Network Configuration

```toml
[network]
# IP address to bind HTTP server (default: "127.0.0.1")
# Use "0.0.0.0" to listen on all interfaces
listen_addr = "0.0.0.0"

# Port for HTTP API server (default: 8080)
client_port = 8080

# Port for Raft TCP communication (default: 8081)
raft_tcp_port = 8081

# Maximum concurrent connections (default: 1000)
max_connections = 1000

# Connection timeout in seconds (default: 30)
connection_timeout = 30

# Keep-alive timeout in seconds (default: 120)
keepalive_timeout = 120
```

**Defaults:**
- `listen_addr`: `"127.0.0.1"`
- `client_port`: `8080`
- `raft_tcp_port`: `8081`
- `max_connections`: `1000`
- `connection_timeout`: `30`
- `keepalive_timeout`: `120`

**Environment Variable Overrides:**
- `SCRIBE_NETWORK_LISTEN_ADDR`
- `SCRIBE_NETWORK_CLIENT_PORT`
- `SCRIBE_NETWORK_RAFT_TCP_PORT`

## Storage Configuration

```toml
[storage]
# Segment size in bytes (default: 1048576 = 1MB)
# Size threshold for creating new segments
segment_size = 1048576

# Maximum cache size in bytes (default: 268435456 = 256MB)
# Amount of memory to use for caching hot data
max_cache_size = 268435456

# Flush interval in milliseconds (default: 5000 = 5s)
# How often to flush writes to disk
# Set to 0 to flush on every write (slower but more durable)
flush_interval_ms = 5000

# Storage mode (default: "HighThroughput")
# Options: "HighThroughput", "LowSpace"
storage_mode = "HighThroughput"

# Enable S3 cold storage (default: false)
enable_s3 = false

# S3 bucket name (required if enable_s3 = true)
s3_bucket = "scribe-ledger-archive"

# S3 region (default: "us-east-1")
s3_region = "us-east-1"

# S3 endpoint (optional, for MinIO or custom S3)
s3_endpoint = "http://minio:9000"

# Archive age threshold in seconds (default: 86400 = 24h)
# Segments older than this are archived to S3
archive_age_threshold = 86400
```

**Defaults:**
- `segment_size`: `1048576` (1MB)
- `max_cache_size`: `268435456` (256MB)
- `flush_interval_ms`: `5000` (5 seconds)
- `storage_mode`: `"HighThroughput"`
- `enable_s3`: `false`

**Environment Variable Overrides:**
- `SCRIBE_STORAGE_SEGMENT_SIZE`
- `SCRIBE_STORAGE_MAX_CACHE_SIZE`
- `AWS_S3_BUCKET` (for s3_bucket)
- `AWS_REGION` (for s3_region)

## Consensus Configuration

```toml
[consensus]
# Election timeout in seconds (default: 10)
# Time to wait before starting new election
# Should be > heartbeat_timeout * 2
election_timeout = 10

# Heartbeat timeout in seconds (default: 3)
# Time between leader heartbeats
# Should be << election_timeout
heartbeat_timeout = 3

# Maximum log entries per batch (default: 100)
# Number of entries to batch in Raft proposals
raft_batch_size = 100

# Snapshot threshold (default: 10000)
# Create snapshot after this many log entries
snapshot_threshold = 10000

# Enable auto-compaction (default: true)
# Automatically compact logs after snapshot
auto_compact = true
```

**Defaults:**
- `election_timeout`: `10` seconds
- `heartbeat_timeout`: `3` seconds
- `raft_batch_size`: `100`
- `snapshot_threshold`: `10000`
- `auto_compact`: `true`

**Tuning Guidelines:**
- Lower `election_timeout` = faster failover, but more likely to trigger unnecessary elections
- Higher `election_timeout` = slower failover, but more stable
- `heartbeat_timeout` should be < `election_timeout / 2`
- Increase `raft_batch_size` for higher write throughput

## Security Configuration

### TLS Configuration

```toml
[security.tls]
# Enable TLS/SSL encryption (default: false)
enabled = true

# Path to TLS certificate file in PEM format (required if enabled)
cert_path = "/etc/scribe-ledger/certs/server.crt"

# Path to TLS private key file in PEM format (required if enabled)
key_path = "/etc/scribe-ledger/certs/server.key"

# Path to CA certificate for client verification (optional)
# Required if require_client_cert = true
ca_cert_path = "/etc/scribe-ledger/certs/ca.crt"

# Require client certificates for mutual TLS (default: false)
require_client_cert = false

# TLS minimum version (default: "1.2")
# Options: "1.0", "1.1", "1.2", "1.3"
min_tls_version = "1.2"

# Allowed cipher suites (default: secure defaults)
# See https://www.openssl.org/docs/man1.1.1/man1/ciphers.html
cipher_suites = "ECDHE-RSA-AES256-GCM-SHA384:ECDHE-RSA-AES128-GCM-SHA256"
```

**Defaults:**
- `enabled`: `false`
- `require_client_cert`: `false`
- `min_tls_version`: `"1.2"`

**Security Best Practices:**
- Always use TLS in production
- Use TLS 1.2 or higher
- Keep certificates and keys secure (chmod 600)
- Rotate certificates before expiry
- Use mutual TLS for node-to-node communication

### Authentication Configuration

```toml
[security.auth]
# Enable authentication (default: false)
enabled = true

# Authentication method (default: "api_key")
# Options: "api_key", "jwt", "oauth2"
method = "api_key"

# API key file path (required if enabled)
# File should contain key-role mappings
api_keys_file = "/etc/scribe-ledger/api-keys.json"

# Session timeout in seconds (default: 3600 = 1 hour)
session_timeout = 3600

# Enable API key rotation (default: false)
enable_key_rotation = false

# Key rotation interval in seconds (default: 2592000 = 30 days)
key_rotation_interval = 2592000
```

**API Keys File Format:**
```json
{
  "admin-key-here": "admin",
  "write-key-here": "read_write",
  "read-key-here": "read_only"
}
```

**Role Permissions:**
- `read_only`: GET operations only
- `read_write`: GET, PUT operations
- `admin`: All operations including DELETE, cluster management, metrics

**Defaults:**
- `enabled`: `false`
- `method`: `"api_key"`
- `session_timeout`: `3600`

**Security Best Practices:**
- Use strong, random API keys (32+ characters)
- Rotate keys regularly
- Use separate keys for different clients
- Store keys securely with restricted permissions

### Rate Limiting Configuration

```toml
[security.rate_limit]
# Enable rate limiting (default: false)
enabled = true

# Maximum requests per window (default: 100)
max_requests = 1000

# Time window in seconds (default: 60)
window_secs = 60

# Burst capacity (default: max_requests / 10)
# Allows temporary spikes above average rate
burst_size = 100

# Rate limit key (default: "ip")
# Options: "ip", "api_key", "user"
limit_by = "api_key"

# Enable distributed rate limiting (default: false)
# Share limits across cluster nodes
distributed = false
```

**Defaults:**
- `enabled`: `false`
- `max_requests`: `100`
- `window_secs`: `60`
- `burst_size`: `max_requests / 10`
- `limit_by`: `"ip"`

**Tuning Guidelines:**
- Start conservative and increase based on monitoring
- Typical values:
  - Low traffic: 100 req/min
  - Medium traffic: 1000 req/min
  - High traffic: 10000 req/min
- Set `burst_size` to handle temporary spikes (10-20% of max_requests)

## Logging Configuration

```toml
[logging]
# Log level (default: "info")
# Options: "trace", "debug", "info", "warn", "error"
level = "info"

# Log format (default: "console")
# Options: "console", "json"
format = "console"

# Enable file logging (default: false)
enable_file = true

# Log directory (default: "./logs")
log_dir = "/var/log/scribe-ledger"

# Log file prefix (default: "scribe-ledger")
log_file_prefix = "scribe"

# Enable console logging (default: true)
enable_console = true

# Enable audit logging (default: false)
enable_audit = true

# Audit log file (default: "audit.log")
audit_log_file = "audit.log"

# Log rotation (default: "daily")
# Options: "hourly", "daily", "weekly", "size"
rotation = "daily"

# Maximum log file size in MB (default: 100)
# Only used if rotation = "size"
max_log_size = 100

# Number of rotated files to keep (default: 30)
keep_logs = 30
```

**Defaults:**
- `level`: `"info"`
- `format`: `"console"`
- `enable_file`: `false`
- `enable_console`: `true`
- `rotation`: `"daily"`

**Log Levels:**
- `trace`: Very detailed debugging (not for production)
- `debug`: Debugging information
- `info`: General informational messages (recommended for production)
- `warn`: Warning messages
- `error`: Error messages only

## Performance Configuration

```toml
[performance]
# Worker thread count (default: number of CPU cores)
# Set to 0 for automatic detection
worker_threads = 0

# Batch size for write operations (default: 100)
batch_size = 100

# Maximum concurrent requests (default: 1000)
max_concurrency = 1000

# Enable request pipelining (default: true)
enable_pipelining = true

# Connection pool size (default: 100)
connection_pool_size = 100

# Idle connection timeout in seconds (default: 300)
idle_timeout = 300
```

**Defaults:**
- `worker_threads`: `0` (auto-detect)
- `batch_size`: `100`
- `max_concurrency`: `1000`

## Environment Variables

All configuration options can be overridden with environment variables using the `SCRIBE_` prefix:

```bash
# Node configuration
export SCRIBE_NODE_ID=1
export SCRIBE_NODE_ADDRESS="10.0.1.10:8001"
export SCRIBE_NODE_DATA_DIR="/var/lib/scribe-ledger"

# Network configuration
export SCRIBE_NETWORK_LISTEN_ADDR="0.0.0.0"
export SCRIBE_NETWORK_CLIENT_PORT=8080
export SCRIBE_NETWORK_RAFT_TCP_PORT=8081

# Storage configuration
export SCRIBE_STORAGE_SEGMENT_SIZE=1048576
export SCRIBE_STORAGE_MAX_CACHE_SIZE=268435456

# Consensus configuration
export SCRIBE_CONSENSUS_ELECTION_TIMEOUT=10
export SCRIBE_CONSENSUS_HEARTBEAT_TIMEOUT=3

# Security configuration
export SCRIBE_SECURITY_TLS_ENABLED=true
export SCRIBE_SECURITY_TLS_CERT_PATH="/path/to/cert.pem"
export SCRIBE_SECURITY_TLS_KEY_PATH="/path/to/key.pem"

export SCRIBE_SECURITY_AUTH_ENABLED=true
export SCRIBE_SECURITY_RATE_LIMIT_ENABLED=true
export SCRIBE_SECURITY_RATE_LIMIT_MAX_REQUESTS=1000

# Logging configuration
export SCRIBE_LOGGING_LEVEL="info"
export SCRIBE_LOGGING_FORMAT="json"
export SCRIBE_LOGGING_ENABLE_FILE=true
export SCRIBE_LOGGING_LOG_DIR="/var/log/scribe-ledger"
```

**Environment Variable Priority:**
1. Environment variables (highest priority)
2. Configuration file
3. Default values (lowest priority)

## Configuration Examples

### Development Configuration

```toml
[node]
id = 1
address = "127.0.0.1:8001"
data_dir = "./data"

[network]
listen_addr = "127.0.0.1"
client_port = 8080

[storage]
max_cache_size = 134217728  # 128MB

[logging]
level = "debug"
format = "console"
enable_console = true

[security.auth]
enabled = false  # Disabled for development

[security.rate_limit]
enabled = false  # Disabled for development
```

### Production Configuration

```toml
[node]
id = 1
address = "10.0.1.10:8001"
data_dir = "/var/lib/scribe-ledger"

[network]
listen_addr = "0.0.0.0"
client_port = 8080
raft_tcp_port = 8081
max_connections = 5000

[storage]
segment_size = 10485760  # 10MB
max_cache_size = 536870912  # 512MB
flush_interval_ms = 5000

[consensus]
election_timeout = 10
heartbeat_timeout = 3
raft_batch_size = 200

[security.tls]
enabled = true
cert_path = "/etc/scribe-ledger/certs/server.crt"
key_path = "/etc/scribe-ledger/certs/server.key"
ca_cert_path = "/etc/scribe-ledger/certs/ca.crt"
require_client_cert = true

[security.auth]
enabled = true
api_keys_file = "/etc/scribe-ledger/api-keys.json"

[security.rate_limit]
enabled = true
max_requests = 2000
window_secs = 60
burst_size = 200

[logging]
level = "info"
format = "json"
enable_file = true
log_dir = "/var/log/scribe-ledger"
enable_audit = true
```

## Validation

To validate your configuration:

```bash
# Test configuration
scribe-node --config config.toml --validate

# Check for syntax errors
cat config.toml | toml-lint
```

## Additional Resources

- [Deployment Guide](DEPLOYMENT.md)
- [Operations Runbook](OPERATIONS.md)
- [Troubleshooting Guide](TROUBLESHOOTING.md)
