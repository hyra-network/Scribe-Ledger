# Deployment Guide

This guide covers deploying Hyra Scribe Ledger in production environments.

## Table of Contents

- [Prerequisites](#prerequisites)
- [TLS Certificate Setup](#tls-certificate-setup)
- [Authentication Configuration](#authentication-configuration)
- [Single Node Deployment](#single-node-deployment)
- [Multi-Node Cluster Deployment](#multi-node-cluster-deployment)
- [Docker Deployment](#docker-deployment)
- [SystemD Service Setup](#systemd-service-setup)
- [Production Checklist](#production-checklist)

## Prerequisites

- Rust 1.70 or later
- OpenSSL (for TLS certificates)
- Linux/Unix operating system
- Network connectivity between cluster nodes

## TLS Certificate Setup

### Generate Self-Signed Certificates (Development)

For development and testing:

```bash
# Create certificates directory
mkdir -p /etc/scribe-ledger/certs
cd /etc/scribe-ledger/certs

# Generate private key
openssl genrsa -out server.key 4096

# Generate certificate signing request
openssl req -new -key server.key -out server.csr \
  -subj "/C=US/ST=State/L=City/O=Organization/CN=localhost"

# Generate self-signed certificate (valid for 365 days)
openssl x509 -req -days 365 -in server.csr \
  -signkey server.key -out server.crt

# Set permissions
chmod 600 server.key
chmod 644 server.crt
```

### Use Certificates from Certificate Authority (Production)

For production deployments, obtain certificates from a trusted CA:

1. **Let's Encrypt (Free)**
   ```bash
   # Install certbot
   sudo apt-get install certbot
   
   # Generate certificate
   sudo certbot certonly --standalone \
     -d your-domain.com \
     -d node1.your-domain.com
   
   # Certificates will be in /etc/letsencrypt/live/your-domain.com/
   ```

2. **Commercial CA**
   - Purchase certificate from trusted CA
   - Follow CA-specific instructions
   - Install certificate and key on server

### Mutual TLS Setup

For node-to-node authentication:

```bash
# Generate CA certificate
openssl req -x509 -new -nodes \
  -keyout ca.key -out ca.crt \
  -days 3650 -subj "/CN=Scribe-Ledger-CA"

# Generate node certificates signed by CA
for node in node1 node2 node3; do
  openssl genrsa -out ${node}.key 4096
  openssl req -new -key ${node}.key -out ${node}.csr \
    -subj "/CN=${node}.your-domain.com"
  openssl x509 -req -in ${node}.csr \
    -CA ca.crt -CAkey ca.key -CAcreateserial \
    -out ${node}.crt -days 365
done
```

## Authentication Configuration

### Generate API Keys

```bash
# Generate secure random API keys
openssl rand -hex 32  # Generates 64-character hex string
```

### Configuration File

Create `config.toml`:

```toml
[node]
id = 1
address = "10.0.1.10:8001"
data_dir = "/var/lib/scribe-ledger"

[network]
listen_addr = "0.0.0.0"
client_port = 8080
raft_tcp_port = 8081

[storage]
segment_size = 1048576      # 1MB
max_cache_size = 268435456  # 256MB

[consensus]
election_timeout = 10
heartbeat_timeout = 3

[security]
# TLS configuration
[security.tls]
enabled = true
cert_path = "/etc/scribe-ledger/certs/server.crt"
key_path = "/etc/scribe-ledger/certs/server.key"
ca_cert_path = "/etc/scribe-ledger/certs/ca.crt"
require_client_cert = true

# Authentication configuration
[security.auth]
enabled = true

# Rate limiting
[security.rate_limit]
enabled = true
max_requests = 1000
window_secs = 60
burst_size = 100

# Logging configuration
[logging]
level = "info"
format = "json"
enable_file = true
log_dir = "/var/log/scribe-ledger"
log_file_prefix = "scribe"
```

### API Key Management

Store API keys securely:

```bash
# Create API keys file
cat > /etc/scribe-ledger/api-keys.env <<EOF
ADMIN_API_KEY=<generated-admin-key>
WRITE_API_KEY=<generated-write-key>
READ_API_KEY=<generated-read-key>
EOF

# Secure the file
chmod 600 /etc/scribe-ledger/api-keys.env
chown scribe-ledger:scribe-ledger /etc/scribe-ledger/api-keys.env
```

## Single Node Deployment

### Manual Deployment

1. **Build the binary:**
   ```bash
   cargo build --release --bin scribe-node
   sudo cp target/release/scribe-node /usr/local/bin/
   ```

2. **Create directories:**
   ```bash
   sudo mkdir -p /var/lib/scribe-ledger
   sudo mkdir -p /var/log/scribe-ledger
   sudo mkdir -p /etc/scribe-ledger
   ```

3. **Create service user:**
   ```bash
   sudo useradd -r -s /bin/false -d /var/lib/scribe-ledger scribe-ledger
   sudo chown -R scribe-ledger:scribe-ledger /var/lib/scribe-ledger
   sudo chown -R scribe-ledger:scribe-ledger /var/log/scribe-ledger
   ```

4. **Start the node:**
   ```bash
   sudo -u scribe-ledger /usr/local/bin/scribe-node \
     --config /etc/scribe-ledger/config.toml
   ```

## Multi-Node Cluster Deployment

### Node Configuration

**Node 1 (10.0.1.10):**
```toml
[node]
id = 1
address = "10.0.1.10:8001"

[network]
client_port = 8080
raft_tcp_port = 8081
```

**Node 2 (10.0.1.11):**
```toml
[node]
id = 2
address = "10.0.1.11:8002"

[network]
client_port = 8090
raft_tcp_port = 8082
```

**Node 3 (10.0.1.12):**
```toml
[node]
id = 3
address = "10.0.1.12:8003"

[network]
client_port = 8100
raft_tcp_port = 8083
```

### Initialize Cluster

```bash
# Start all nodes
ssh node1 'sudo systemctl start scribe-node-1'
ssh node2 'sudo systemctl start scribe-node-2'
ssh node3 'sudo systemctl start scribe-node-3'

# Bootstrap cluster (on node1)
curl -X POST http://node1:8080/cluster/init

# Add nodes to cluster
curl -X POST http://node1:8080/cluster/nodes/add \
  -H 'Content-Type: application/json' \
  -d '{"node_id": 2, "address": "10.0.1.11:8002"}'

curl -X POST http://node1:8080/cluster/nodes/add \
  -H 'Content-Type: application/json' \
  -d '{"node_id": 3, "address": "10.0.1.12:8003"}'

# Verify cluster status
curl http://node1:8080/cluster/info
```

## Docker Deployment

### Docker Compose Setup

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  scribe-node-1:
    build: .
    container_name: scribe-node-1
    ports:
      - "8080:8080"
      - "8081:8081"
    environment:
      - NODE_ID=1
      - NODE_ADDRESS=scribe-node-1:8001
      - CLIENT_PORT=8080
      - RAFT_PORT=8081
    volumes:
      - ./data/node1:/var/lib/scribe-ledger
      - ./certs:/etc/scribe-ledger/certs:ro
    networks:
      - scribe-network

  scribe-node-2:
    build: .
    container_name: scribe-node-2
    ports:
      - "8090:8090"
      - "8082:8082"
    environment:
      - NODE_ID=2
      - NODE_ADDRESS=scribe-node-2:8002
      - CLIENT_PORT=8090
      - RAFT_PORT=8082
    volumes:
      - ./data/node2:/var/lib/scribe-ledger
      - ./certs:/etc/scribe-ledger/certs:ro
    networks:
      - scribe-network

  scribe-node-3:
    build: .
    container_name: scribe-node-3
    ports:
      - "8100:8100"
      - "8083:8083"
    environment:
      - NODE_ID=3
      - NODE_ADDRESS=scribe-node-3:8003
      - CLIENT_PORT=8100
      - RAFT_PORT=8083
    volumes:
      - ./data/node3:/var/lib/scribe-ledger
      - ./certs:/etc/scribe-ledger/certs:ro
    networks:
      - scribe-network

networks:
  scribe-network:
    driver: bridge
```

### Deploy with Docker

```bash
# Build images
docker-compose build

# Start cluster
docker-compose up -d

# View logs
docker-compose logs -f

# Scale horizontally (if needed)
docker-compose up -d --scale scribe-node=5

# Stop cluster
docker-compose down
```

## SystemD Service Setup

Create `/etc/systemd/system/scribe-node.service`:

```ini
[Unit]
Description=Scribe Ledger Node
After=network.target

[Service]
Type=simple
User=scribe-ledger
Group=scribe-ledger
WorkingDirectory=/var/lib/scribe-ledger
ExecStart=/usr/local/bin/scribe-node --config /etc/scribe-ledger/config.toml
Restart=on-failure
RestartSec=5s
StandardOutput=journal
StandardError=journal

# Security hardening
PrivateTmp=true
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/scribe-ledger /var/log/scribe-ledger

# Resource limits
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

### Manage Service

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable on boot
sudo systemctl enable scribe-node

# Start service
sudo systemctl start scribe-node

# Check status
sudo systemctl status scribe-node

# View logs
sudo journalctl -u scribe-node -f

# Restart service
sudo systemctl restart scribe-node

# Stop service
sudo systemctl stop scribe-node
```

## Production Checklist

### Security

- [ ] TLS enabled for all communication
- [ ] Strong API keys generated and distributed
- [ ] Certificate files secured (chmod 600 for keys)
- [ ] API keys rotated regularly
- [ ] Rate limiting configured appropriately
- [ ] Audit logging enabled
- [ ] Firewall rules configured
- [ ] SELinux/AppArmor policies applied

### High Availability

- [ ] Minimum 3 nodes for quorum
- [ ] Nodes deployed across availability zones
- [ ] Load balancer configured for client requests
- [ ] Health checks configured
- [ ] Automatic failover tested
- [ ] Backup and restore procedures documented

### Monitoring

- [ ] Prometheus metrics endpoint exposed
- [ ] Grafana dashboards configured
- [ ] Alerting rules defined
- [ ] Log aggregation set up (ELK, Splunk, etc.)
- [ ] Uptime monitoring configured
- [ ] Performance baselines established

### Performance

- [ ] Storage capacity planned and provisioned
- [ ] Network bandwidth adequate
- [ ] Cache size configured appropriately
- [ ] Batch sizes tuned for workload
- [ ] S3 archival configured (if using cold storage)
- [ ] Load testing completed

### Operations

- [ ] Backup procedures documented and tested
- [ ] Restore procedures documented and tested
- [ ] Runbook created for common operations
- [ ] Incident response procedures defined
- [ ] On-call rotation established
- [ ] Documentation up to date

### Compliance

- [ ] Data retention policies configured
- [ ] Audit logs retained per compliance requirements
- [ ] Access control policies enforced
- [ ] Encryption at rest configured (if required)
- [ ] Compliance certifications obtained

## Troubleshooting

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for common issues and solutions.

## Operations Guide

See [OPERATIONS.md](OPERATIONS.md) for day-to-day operational procedures.

## Configuration Reference

See [CONFIGURATION.md](CONFIGURATION.md) for complete configuration documentation.
