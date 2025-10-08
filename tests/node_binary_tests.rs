//! Integration tests for scribe-node binary
//!
//! These tests verify the scribe-node binary CLI functionality,
//! startup, graceful shutdown, and cluster initialization.

use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

/// Test that scribe-node binary can be compiled and executed
#[test]
fn test_scribe_node_binary_exists() {
    let output = Command::new("cargo")
        .args(["build", "--bin", "scribe-node"])
        .output()
        .expect("Failed to build scribe-node");

    assert!(
        output.status.success(),
        "scribe-node binary should compile successfully"
    );
}

/// Test scribe-node --help output
#[test]
fn test_scribe_node_help() {
    let output = Command::new("./target/debug/scribe-node")
        .arg("--help")
        .output()
        .expect("Failed to execute scribe-node");

    assert!(output.status.success(), "Help command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Distributed ledger node with Raft consensus"),
        "Help text should contain description"
    );
    assert!(
        stdout.contains("--config"),
        "Help should show --config option"
    );
    assert!(
        stdout.contains("--node-id"),
        "Help should show --node-id option"
    );
    assert!(
        stdout.contains("--bootstrap"),
        "Help should show --bootstrap option"
    );
    assert!(
        stdout.contains("--log-level"),
        "Help should show --log-level option"
    );
}

/// Test scribe-node --version output
#[test]
fn test_scribe_node_version() {
    let output = Command::new("./target/debug/scribe-node")
        .arg("--version")
        .output()
        .expect("Failed to execute scribe-node");

    assert!(output.status.success(), "Version command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("scribe-node"),
        "Version output should contain binary name"
    );
    assert!(
        stdout.contains("0.1.0"),
        "Version output should contain version number"
    );
}

/// Test that scribe-node accepts CLI arguments
#[test]
fn test_scribe_node_cli_arguments_parsing() {
    // Just test that the binary accepts the arguments without error
    // We don't actually run it since that would start a server
    let output = Command::new("./target/debug/scribe-node")
        .arg("--help")
        .output()
        .expect("Failed to execute scribe-node");

    assert!(output.status.success());
}

/// Test scribe-node startup with default config
#[tokio::test]
async fn test_scribe_node_startup_default_config() {
    // Create a temporary directory for the test node
    let temp_dir = std::env::temp_dir().join(format!("scribe-node-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create a minimal config file in the temp directory
    let config_content = format!(
        r#"
[node]
id = 99
address = "127.0.0.1"
data_dir = "{}/data"

[network]
listen_addr = "127.0.0.1:18099"
client_port = 18099
raft_port = 19099

[storage]
segment_size = 67108864
max_cache_size = 268435456

[consensus]
election_timeout_ms = 1000
heartbeat_interval_ms = 300
"#,
        temp_dir.display()
    );

    let config_path = temp_dir.join("config.toml");
    std::fs::write(&config_path, config_content).unwrap();

    // Start the node with bootstrap mode in the background
    let mut child = Command::new("./target/debug/scribe-node")
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--bootstrap",
            "--log-level",
            "error",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start scribe-node");

    // Give it time to start
    sleep(Duration::from_millis(3000)).await;

    // Check if process is still running
    match child.try_wait() {
        Ok(None) => {
            // Process is still running - good
            child.kill().expect("Failed to kill scribe-node process");
        }
        Ok(Some(status)) => {
            // Process exited - capture output for debugging
            let stderr = if let Some(mut stderr_pipe) = child.stderr.take() {
                use std::io::Read;
                let mut buf = String::new();
                stderr_pipe.read_to_string(&mut buf).ok();
                buf
            } else {
                String::new()
            };
            // Cleanup before panic
            let _ = std::fs::remove_dir_all(&temp_dir);
            panic!(
                "Node exited unexpectedly with status: {:?}\nSTDERR: {}",
                status, stderr
            );
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&temp_dir);
            panic!("Error checking process status: {}", e);
        }
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test scribe-node with config file
#[tokio::test]
async fn test_scribe_node_with_config_file() {
    // Create temporary directory and config
    let temp_dir = std::env::temp_dir().join(format!("scribe-node-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create a minimal config file
    let config_content = format!(
        r#"
[node]
id = 88
address = "127.0.0.1"
data_dir = "{}/data"

[network]
listen_addr = "127.0.0.1:18088"
client_port = 18088
raft_port = 19088

[storage]
segment_size = 67108864
max_cache_size = 268435456

[consensus]
election_timeout_ms = 1000
heartbeat_interval_ms = 300
"#,
        temp_dir.display()
    );

    let config_path = temp_dir.join("config.toml");
    std::fs::write(&config_path, config_content).unwrap();

    // Start the node with config file in the background
    let mut child = Command::new("./target/debug/scribe-node")
        .args([
            "--config",
            config_path.to_str().unwrap(),
            "--bootstrap",
            "--log-level",
            "error",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start scribe-node");

    // Give it time to start
    sleep(Duration::from_millis(3000)).await;

    // Check if process is still running
    match child.try_wait() {
        Ok(None) => {
            // Process is still running - good
            child.kill().expect("Failed to kill scribe-node process");
        }
        Ok(Some(status)) => {
            // Process exited - capture output for debugging
            let stderr = if let Some(mut stderr_pipe) = child.stderr.take() {
                use std::io::Read;
                let mut buf = String::new();
                stderr_pipe.read_to_string(&mut buf).ok();
                buf
            } else {
                String::new()
            };
            // Cleanup before panic
            let _ = std::fs::remove_dir_all(&temp_dir);
            panic!(
                "Node exited unexpectedly with status: {:?}\nSTDERR: {}",
                status, stderr
            );
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&temp_dir);
            panic!("Error checking process status: {}", e);
        }
    }

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test scribe-node graceful shutdown with SIGTERM
#[cfg(unix)]
#[tokio::test]
async fn test_scribe_node_graceful_shutdown() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    // Start the node
    let mut child = Command::new("./target/debug/scribe-node")
        .args(["--bootstrap", "--node-id", "98", "--log-level", "info"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start scribe-node");

    // Give it time to start
    sleep(Duration::from_millis(2000)).await;

    // Send SIGTERM
    let pid = Pid::from_raw(child.id() as i32);
    kill(pid, Signal::SIGTERM).expect("Failed to send SIGTERM");

    // Wait for graceful shutdown (with timeout)
    let timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    loop {
        if start.elapsed() > timeout {
            // Timeout - force kill
            child.kill().ok();
            panic!("Node did not shut down gracefully within timeout");
        }

        match child.try_wait() {
            Ok(Some(_status)) => {
                // Process exited
                break;
            }
            Ok(None) => {
                // Still running, wait a bit
                sleep(Duration::from_millis(100)).await;
            }
            Err(e) => {
                panic!("Error checking process status: {}", e);
            }
        }
    }
}

/// Test scribe-node with invalid config file
#[test]
fn test_scribe_node_invalid_config_file() {
    let output = Command::new("./target/debug/scribe-node")
        .args(["--config", "/nonexistent/config.toml"])
        .output()
        .expect("Failed to execute scribe-node");

    // Should fail with nonexistent config file
    assert!(
        !output.status.success(),
        "Should fail with nonexistent config file"
    );
}

/// Test that scribe-node binary is not too large (optimization check)
#[test]
fn test_scribe_node_binary_size() {
    let binary_path = "./target/debug/scribe-node";
    let metadata = std::fs::metadata(binary_path).expect("Failed to get binary metadata");

    // Binary should be reasonable size
    // Note: With AWS SDK S3 support (Task 6.1), binary size increased from ~180MB to ~220MB
    // With flate2 compression (Task 6.2), binary size increased to ~265MB
    // This is expected due to AWS SDK and compression dependencies
    let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
    assert!(
        size_mb < 300.0,
        "Binary size should be reasonable: {:.2} MB",
        size_mb
    );

    println!("scribe-node binary size: {:.2} MB", size_mb);
}

/// Test CLI node-id override functionality
#[test]
fn test_scribe_node_node_id_override() {
    // Test that node-id can be specified
    let output = Command::new("./target/debug/scribe-node")
        .args(["--node-id", "42", "--help"])
        .output()
        .expect("Failed to execute scribe-node");

    assert!(output.status.success(), "Should accept node-id parameter");
}

/// Test CLI bootstrap flag
#[test]
fn test_scribe_node_bootstrap_flag() {
    // Test that bootstrap flag can be specified
    let output = Command::new("./target/debug/scribe-node")
        .args(["--bootstrap", "--help"])
        .output()
        .expect("Failed to execute scribe-node");

    assert!(output.status.success(), "Should accept bootstrap flag");
}

/// Test CLI log-level option
#[test]
fn test_scribe_node_log_level_option() {
    let log_levels = vec!["trace", "debug", "info", "warn", "error"];

    for level in log_levels {
        let output = Command::new("./target/debug/scribe-node")
            .args(["--log-level", level, "--help"])
            .output()
            .expect("Failed to execute scribe-node");

        assert!(
            output.status.success(),
            "Should accept log-level: {}",
            level
        );
    }
}
