# Systemd Service Files

This directory contains systemd service files for running Hyra Scribe Ledger nodes as system services.

## Installation

1. **Create a dedicated user:**
   ```bash
   sudo useradd -r -s /bin/false scribe
   ```

2. **Create necessary directories:**
   ```bash
   sudo mkdir -p /opt/simple-scribe-ledger
   sudo mkdir -p /etc/simple-scribe-ledger
   sudo mkdir -p /var/lib/simple-scribe-ledger/node-{1,2,3}
   sudo chown -R scribe:scribe /var/lib/simple-scribe-ledger
   ```

3. **Copy the binary and configuration files:**
   ```bash
   sudo cp target/release/scribe-node /opt/simple-scribe-ledger/
   sudo cp config-node1.toml /etc/simple-scribe-ledger/node1.toml
   sudo cp config-node2.toml /etc/simple-scribe-ledger/node2.toml
   sudo cp config-node3.toml /etc/simple-scribe-ledger/node3.toml
   ```

4. **Update configuration files** to use the correct data directories:
   ```bash
   # Edit /etc/simple-scribe-ledger/node1.toml and set:
   data_dir = "/var/lib/simple-scribe-ledger/node-1"
   
   # Repeat for node2.toml and node3.toml
   ```

5. **Copy service files:**
   ```bash
   sudo cp scripts/systemd/*.service /etc/systemd/system/
   ```

6. **Reload systemd:**
   ```bash
   sudo systemctl daemon-reload
   ```

## Usage

### Start a single node:
```bash
sudo systemctl start scribe-node-1
```

### Start all nodes:
```bash
sudo systemctl start scribe-node-1
sudo systemctl start scribe-node-2
sudo systemctl start scribe-node-3
```

### Enable nodes to start on boot:
```bash
sudo systemctl enable scribe-node-1
sudo systemctl enable scribe-node-2
sudo systemctl enable scribe-node-3
```

### Check status:
```bash
sudo systemctl status scribe-node-1
```

### View logs:
```bash
sudo journalctl -u scribe-node-1 -f
```

### Stop a node:
```bash
sudo systemctl stop scribe-node-1
```

## Troubleshooting

### Check if service is running:
```bash
sudo systemctl is-active scribe-node-1
```

### View recent logs:
```bash
sudo journalctl -u scribe-node-1 -n 100 --no-pager
```

### Restart a failed service:
```bash
sudo systemctl restart scribe-node-1
```
