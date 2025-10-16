# Windows Setup Guide

This guide provides step-by-step instructions for setting up and running Hyra Scribe Ledger on Windows.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Running a Single Node](#running-a-single-node)
- [Running a Multi-Node Cluster](#running-a-multi-node-cluster)
- [Testing the Setup](#testing-the-setup)
- [Windows-Specific Considerations](#windows-specific-considerations)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Software

1. **Rust and Cargo**
   - Download and install from [rustup.rs](https://rustup.rs/)
   - Open PowerShell as Administrator and run:
     ```powershell
     # Download and run rustup-init.exe
     Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile "$env:TEMP\rustup-init.exe"
     & "$env:TEMP\rustup-init.exe"
     ```
   - Follow the on-screen instructions (default installation is recommended)
   - Restart your terminal or PowerShell window after installation
   - Verify installation:
     ```powershell
     rustc --version
     cargo --version
     ```

2. **Git for Windows**
   - Download from [git-scm.com](https://git-scm.com/download/win)
   - Install with default settings
   - Verify installation:
     ```powershell
     git --version
     ```

3. **Visual Studio Build Tools** (Required for Rust compilation)
   - Download [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
   - During installation, select "Desktop development with C++"
   - Or use the Visual Studio Installer to add the C++ build tools

4. **curl** (For testing HTTP endpoints)
   - Windows 10/11 includes curl by default
   - Verify with:
     ```powershell
     curl --version
     ```
   - Alternative: Use PowerShell's `Invoke-WebRequest` or `Invoke-RestMethod`

## Installation

### 1. Clone the Repository

Open PowerShell or Command Prompt and run:

```powershell
# Navigate to your desired directory
cd C:\Users\YourUsername\Projects

# Clone the repository
git clone https://github.com/hyra-network/Scribe-Ledger.git
cd Scribe-Ledger
```

### 2. Build the Project

```powershell
# Build in release mode (recommended for performance)
cargo build --release

# Or build in debug mode (faster compilation, slower execution)
cargo build
```

The build process may take several minutes on the first run as it downloads and compiles dependencies.

### 3. Verify the Build

```powershell
# Check that the binary was created
dir target\release\scribe-node.exe

# Or for debug build
dir target\debug\scribe-node.exe
```

## Running a Single Node

### Using Default Configuration

The simplest way to start a single node is with the default configuration:

```powershell
# Run with the default config.toml
cargo run --release --bin scribe-node -- --config config.toml
```

The node will start and the HTTP API will be available at `http://localhost:8001`.

### Creating a Custom Configuration

1. Copy the default configuration:
   ```powershell
   Copy-Item config.toml my-config.toml
   ```

2. Edit `my-config.toml` in your favorite text editor (Notepad, VS Code, etc.):
   ```toml
   [node]
   id = 1
   address = "127.0.0.1"
   data_dir = "./my-data"

   [network]
   listen_addr = "127.0.0.1:8001"
   client_port = 8001
   raft_port = 9001
   ```

3. Run with your custom configuration:
   ```powershell
   cargo run --release --bin scribe-node -- --config my-config.toml
   ```

### Testing the Single Node

Open a new PowerShell window and test the API:

```powershell
# Health check
curl http://localhost:8001/health

# Store some data (using PowerShell-native command)
Invoke-RestMethod -Uri http://localhost:8001/test-key -Method Put -Body "Hello from Windows!" -ContentType "application/octet-stream"

# Retrieve the data
curl http://localhost:8001/test-key

# Check metrics
curl http://localhost:8001/metrics
```

**Note:** PowerShell's `curl` is an alias for `Invoke-WebRequest`. For PUT requests with headers, use `Invoke-RestMethod` as shown above.

## Running a Multi-Node Cluster

Running a cluster on Windows requires starting multiple nodes in separate terminal windows.

### Step 1: Prepare Configuration Files

The repository includes pre-configured files for a 3-node cluster:
- `config-node1.toml` - Node 1 (Port 8001, Raft 9001)
- `config-node2.toml` - Node 2 (Port 8002, Raft 9002)
- `config-node3.toml` - Node 3 (Port 8003, Raft 9003)

These are ready to use without modification.

### Step 2: Start the Nodes

Open **three separate PowerShell windows** (or use Windows Terminal with multiple tabs):

**Terminal 1 - Start Node 1:**
```powershell
cd C:\Users\YourUsername\Projects\Scribe-Ledger
cargo run --release --bin scribe-node -- --config config-node1.toml --bootstrap
```

Wait for Node 1 to start (you'll see log messages indicating it's running).

**Terminal 2 - Start Node 2:**
```powershell
cd C:\Users\YourUsername\Projects\Scribe-Ledger
cargo run --release --bin scribe-node -- --config config-node2.toml
```

**Terminal 3 - Start Node 3:**
```powershell
cd C:\Users\YourUsername\Projects\Scribe-Ledger
cargo run --release --bin scribe-node -- --config config-node3.toml
```

### Step 3: Verify Cluster Formation

Open a fourth PowerShell window and check the cluster:

```powershell
# Check Node 1
curl http://localhost:8001/cluster/info

# Check Node 2
curl http://localhost:8002/cluster/info

# Check Node 3
curl http://localhost:8003/cluster/info

# List cluster members from any node
curl http://localhost:8001/cluster/nodes
```

## Testing the Setup

### Basic Operations

```powershell
# Write data to the cluster (any node) - using PowerShell-native command
Invoke-RestMethod -Uri http://localhost:8001/user:alice -Method Put -Body "Alice Smith" -ContentType "application/octet-stream"

# Read from any node (data is replicated)
curl http://localhost:8002/user:alice

# Delete data
Invoke-RestMethod -Uri http://localhost:8003/user:alice -Method Delete
```

**Note:** For PUT and DELETE operations with specific content types, use `Invoke-RestMethod` instead of the `curl` alias.

### Cluster Health Monitoring

```powershell
# Check health of each node
curl http://localhost:8001/health
curl http://localhost:8002/health
curl http://localhost:8003/health

# Get detailed metrics
curl http://localhost:8001/metrics/prometheus
```

### Testing Replication

1. Write data to Node 1:
   ```powershell
   Invoke-RestMethod -Uri http://localhost:8001/test-replication -Method Put -Body "This should replicate!" -ContentType "application/octet-stream"
   ```

2. Read from Node 2 and Node 3:
   ```powershell
   curl http://localhost:8002/test-replication
   curl http://localhost:8003/test-replication
   ```

Both should return the same data.

### Testing Fault Tolerance

1. Stop Node 2 (press Ctrl+C in Terminal 2)

2. Write data using Node 1:
   ```powershell
   Invoke-RestMethod -Uri http://localhost:8001/fault-test -Method Put -Body "Node 2 is down" -ContentType "application/octet-stream"
   ```

3. Read from Node 3:
   ```powershell
   curl http://localhost:8003/fault-test
   ```

4. Restart Node 2 and verify it catches up:
   ```powershell
   # In Terminal 2
   cargo run --release --bin scribe-node -- --config config-node2.toml
   
   # After it starts, read the data
   curl http://localhost:8002/fault-test
   ```

## Windows-Specific Considerations

### File Paths

- Windows uses backslashes (`\`) in paths, but TOML configuration files use forward slashes (`/`)
- In configuration files, you can use either:
  ```toml
  data_dir = "./node-1"      # Portable (recommended)
  data_dir = ".\\node-1"     # Windows-style
  data_dir = "C:/data/node-1" # Absolute path
  ```

### Firewall

If you encounter connection issues:

1. Open Windows Defender Firewall
2. Click "Allow an app through firewall"
3. Add `scribe-node.exe` from your `target\release` directory
4. Allow both Private and Public networks (for development)

### Background Execution

To run nodes in the background:

**Using PowerShell Jobs:**
```powershell
# Start as background job
Start-Job -ScriptBlock { 
    Set-Location "C:\Users\YourUsername\Projects\Scribe-Ledger"
    cargo run --release --bin scribe-node -- --config config-node1.toml --bootstrap
}

# List jobs
Get-Job

# Stop all jobs
Get-Job | Stop-Job
Get-Job | Remove-Job
```

**Using Windows Task Scheduler:**
- Create a scheduled task to run `scribe-node.exe` with appropriate arguments
- Set it to run at system startup for automatic node launching

### Data Directories

By default, nodes create data directories in the project folder:
- `.\node-1\` for Node 1
- `.\node-2\` for Node 2
- `.\node-3\` for Node 3

To use a different location, update the `data_dir` in your configuration files.

### Port Availability

Check if ports are in use:

```powershell
# Check if port 8001 is in use
netstat -ano | findstr :8001

# Find process using port 8001
Get-NetTCPConnection -LocalPort 8001 | Select-Object -Property LocalAddress, LocalPort, State, OwningProcess

# Kill process by PID
Stop-Process -Id <PID> -Force
```

## Troubleshooting

### Build Errors

**"link.exe not found" or "LINK : fatal error LNK1181"**
- Install Visual Studio Build Tools with C++ support
- Restart your terminal after installation

**"cargo: command not found"**
- Ensure Rust was installed correctly
- Add `%USERPROFILE%\.cargo\bin` to your PATH environment variable
- Restart your terminal

### Runtime Errors

**"Port already in use"**
```powershell
# Find and kill process using the port
netstat -ano | findstr :8001
Stop-Process -Id <PID> -Force
```

**"Access is denied" when creating data directories**
- Run PowerShell as Administrator, or
- Choose a data directory where you have write permissions
- Update `data_dir` in your configuration

**Nodes can't discover each other**
- Ensure Windows Firewall isn't blocking UDP port 17946
- All nodes should be on the same network
- Check that `broadcast_addr` in `config.toml` is set correctly for your network

**"Failed to bind to address"**
- Check if another application is using the port
- Try using a different port in your configuration
- Ensure you're not running multiple instances of the same node

### Performance Issues

**Slow compilation**
- First builds are always slower (downloading dependencies)
- Use `--release` flag for better runtime performance
- Consider using `cargo build` (debug mode) during development for faster compilation

**High CPU usage**
- This is normal during consensus operations
- Reduce heartbeat frequency in configuration if needed
- Ensure you're using the release build (`--release`)

### Testing with PowerShell

If `curl` doesn't work as expected, use PowerShell alternatives:

```powershell
# GET request
Invoke-RestMethod -Uri http://localhost:8001/health

# PUT request
Invoke-RestMethod -Uri http://localhost:8001/my-key -Method Put -Body "my value" -ContentType "application/octet-stream"

# DELETE request
Invoke-RestMethod -Uri http://localhost:8001/my-key -Method Delete

# With headers
Invoke-RestMethod -Uri http://localhost:8001/metrics -Headers @{"Accept"="application/json"}
```

### Getting Help

If you encounter issues not covered here:

1. Check the main [README.md](../README.md) for general documentation
2. Review [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for common issues
3. Check the [GitHub Issues](https://github.com/hyra-network/Scribe-Ledger/issues) page
4. Open a new issue with:
   - Windows version
   - Rust version (`rustc --version`)
   - Error messages
   - Steps to reproduce

## Next Steps

- **Configuration**: See [CONFIGURATION.md](CONFIGURATION.md) for detailed configuration options
- **Deployment**: See [DEPLOYMENT.md](DEPLOYMENT.md) for production deployment guidance
- **Operations**: See [OPERATIONS.md](OPERATIONS.md) for cluster management
- **Development**: See [DEVELOPMENT.md](../DEVELOPMENT.md) for contributing to the project

## Additional Resources

- [Rust on Windows](https://www.rust-lang.org/tools/install) - Official Rust installation guide
- [Windows Terminal](https://github.com/microsoft/terminal) - Modern terminal for Windows
- [Visual Studio Code](https://code.visualstudio.com/) - Recommended editor with Rust support
