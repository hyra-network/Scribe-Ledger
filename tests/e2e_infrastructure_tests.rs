//! E2E Test Infrastructure Tests (Task 8.3)
//!
//! Unit tests to verify E2E test infrastructure is correctly set up.
//! These tests verify scripts, configs, and test framework exist and are valid.

use std::fs;
use std::path::Path;
use std::process::Command;

/// Test 1: Verify start-cluster.sh script exists and is executable
#[test]
fn test_start_cluster_script_exists() {
    let script_path = Path::new("scripts/start-cluster.sh");
    assert!(script_path.exists(), "start-cluster.sh script should exist");

    // Check if script is executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(script_path).expect("Failed to get script metadata");
        let permissions = metadata.permissions();
        assert!(
            permissions.mode() & 0o111 != 0,
            "start-cluster.sh should be executable"
        );
    }
}

/// Test 2: Verify stop-cluster.sh script exists and is executable
#[test]
fn test_stop_cluster_script_exists() {
    let script_path = Path::new("scripts/stop-cluster.sh");
    assert!(script_path.exists(), "stop-cluster.sh script should exist");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(script_path).expect("Failed to get script metadata");
        let permissions = metadata.permissions();
        assert!(
            permissions.mode() & 0o111 != 0,
            "stop-cluster.sh should be executable"
        );
    }
}

/// Test 3: Verify test-cluster.sh script exists and is executable
#[test]
fn test_test_cluster_script_exists() {
    let script_path = Path::new("scripts/test-cluster.sh");
    assert!(script_path.exists(), "test-cluster.sh script should exist");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(script_path).expect("Failed to get script metadata");
        let permissions = metadata.permissions();
        assert!(
            permissions.mode() & 0o111 != 0,
            "test-cluster.sh should be executable"
        );
    }
}

/// Test 4: Verify all node configuration files exist
#[test]
fn test_node_config_files_exist() {
    let configs = [
        "config-node1.toml",
        "config-node2.toml",
        "config-node3.toml",
    ];

    for config in &configs {
        let config_path = Path::new(config);
        assert!(
            config_path.exists(),
            "Configuration file {} should exist",
            config
        );

        // Verify it's a valid TOML file by trying to read it
        let content = fs::read_to_string(config_path).expect(&format!("Failed to read {}", config));

        // Basic validation - should contain required sections
        assert!(
            content.contains("[node]"),
            "{} should have [node] section",
            config
        );
        assert!(
            content.contains("[network]"),
            "{} should have [network] section",
            config
        );
        assert!(
            content.contains("[storage]"),
            "{} should have [storage] section",
            config
        );
        assert!(
            content.contains("[consensus]"),
            "{} should have [consensus] section",
            config
        );
    }
}

/// Test 5: Verify systemd service files exist
#[test]
fn test_systemd_service_files_exist() {
    let services = [
        "scripts/systemd/scribe-node-1.service",
        "scripts/systemd/scribe-node-2.service",
        "scripts/systemd/scribe-node-3.service",
    ];

    for service in &services {
        let service_path = Path::new(service);
        assert!(
            service_path.exists(),
            "Systemd service file {} should exist",
            service
        );

        // Verify it's a valid systemd file
        let content =
            fs::read_to_string(service_path).expect(&format!("Failed to read {}", service));

        assert!(
            content.contains("[Unit]"),
            "{} should have [Unit] section",
            service
        );
        assert!(
            content.contains("[Service]"),
            "{} should have [Service] section",
            service
        );
        assert!(
            content.contains("[Install]"),
            "{} should have [Install] section",
            service
        );
        assert!(
            content.contains("ExecStart="),
            "{} should have ExecStart directive",
            service
        );
    }
}

/// Test 6: Verify systemd README exists
#[test]
fn test_systemd_readme_exists() {
    let readme_path = Path::new("scripts/systemd/README.md");
    assert!(readme_path.exists(), "systemd README.md should exist");

    let content = fs::read_to_string(readme_path).expect("Failed to read systemd README.md");

    // Verify it contains installation instructions
    assert!(
        content.contains("Installation"),
        "README should contain Installation section"
    );
    assert!(
        content.contains("Usage"),
        "README should contain Usage section"
    );
}

/// Test 7: Verify Dockerfile exists and is valid
#[test]
fn test_dockerfile_exists() {
    let dockerfile_path = Path::new("Dockerfile");
    assert!(dockerfile_path.exists(), "Dockerfile should exist");

    let content = fs::read_to_string(dockerfile_path).expect("Failed to read Dockerfile");

    // Basic validation
    assert!(
        content.contains("FROM"),
        "Dockerfile should have FROM instruction"
    );
    assert!(
        content.contains("rust"),
        "Dockerfile should use Rust base image"
    );
    assert!(
        content.contains("scribe-node"),
        "Dockerfile should reference scribe-node binary"
    );
    assert!(content.contains("EXPOSE"), "Dockerfile should expose ports");
}

/// Test 8: Verify docker-compose.yml exists and is valid
#[test]
fn test_docker_compose_exists() {
    let compose_path = Path::new("docker-compose.yml");
    assert!(compose_path.exists(), "docker-compose.yml should exist");

    let content = fs::read_to_string(compose_path).expect("Failed to read docker-compose.yml");

    // Verify it defines all 3 nodes
    assert!(
        content.contains("node1"),
        "docker-compose should define node1"
    );
    assert!(
        content.contains("node2"),
        "docker-compose should define node2"
    );
    assert!(
        content.contains("node3"),
        "docker-compose should define node3"
    );

    // Verify it has network configuration
    assert!(
        content.contains("networks"),
        "docker-compose should define networks"
    );
    assert!(
        content.contains("volumes"),
        "docker-compose should define volumes"
    );
}

/// Test 9: Verify .dockerignore exists
#[test]
fn test_dockerignore_exists() {
    let dockerignore_path = Path::new(".dockerignore");
    assert!(dockerignore_path.exists(), ".dockerignore should exist");

    let content = fs::read_to_string(dockerignore_path).expect("Failed to read .dockerignore");

    // Should ignore common development files
    assert!(
        content.contains("target"),
        ".dockerignore should ignore target/"
    );
    assert!(content.contains(".git"), ".dockerignore should ignore .git");
}

/// Test 10: Verify E2E test directory structure
#[test]
fn test_e2e_directory_structure() {
    let e2e_dir = Path::new("tests/e2e");
    assert!(e2e_dir.exists(), "tests/e2e directory should exist");
    assert!(e2e_dir.is_dir(), "tests/e2e should be a directory");
}

/// Test 11: Verify Python E2E test script exists and is executable
#[test]
fn test_python_e2e_script_exists() {
    let script_path = Path::new("tests/e2e/cluster_e2e_test.py");
    assert!(
        script_path.exists(),
        "cluster_e2e_test.py script should exist"
    );

    let content = fs::read_to_string(script_path).expect("Failed to read cluster_e2e_test.py");

    // Verify it's a Python script
    assert!(
        content.contains("#!/usr/bin/env python3"),
        "Should have Python shebang"
    );
    assert!(content.contains("def main()"), "Should have main function");
    assert!(
        content.contains("test_health_checks"),
        "Should have health check test"
    );
    assert!(
        content.contains("test_data_replication"),
        "Should have data replication test"
    );
    assert!(
        content.contains("test_concurrent_operations"),
        "Should have concurrent operations test"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(script_path).expect("Failed to get script metadata");
        let permissions = metadata.permissions();
        assert!(
            permissions.mode() & 0o111 != 0,
            "cluster_e2e_test.py should be executable"
        );
    }
}

/// Test 12: Verify E2E requirements.txt exists
#[test]
fn test_e2e_requirements_exists() {
    let requirements_path = Path::new("tests/e2e/requirements.txt");
    assert!(
        requirements_path.exists(),
        "requirements.txt should exist in tests/e2e/"
    );

    let content = fs::read_to_string(requirements_path).expect("Failed to read requirements.txt");

    // Should contain requests library
    assert!(
        content.contains("requests"),
        "requirements.txt should include requests library"
    );
}

/// Test 13: Verify E2E README exists
#[test]
fn test_e2e_readme_exists() {
    let readme_path = Path::new("tests/e2e/README.md");
    assert!(readme_path.exists(), "tests/e2e/README.md should exist");

    let content = fs::read_to_string(readme_path).expect("Failed to read e2e README.md");

    assert!(
        content.contains("Prerequisites"),
        "README should contain Prerequisites"
    );
    assert!(
        content.contains("Running E2E Tests"),
        "README should contain running instructions"
    );
}

/// Test 14: Verify scripts have valid bash syntax (basic check)
#[test]
fn test_scripts_have_bash_shebang() {
    let scripts = [
        "scripts/start-cluster.sh",
        "scripts/stop-cluster.sh",
        "scripts/test-cluster.sh",
    ];

    for script in &scripts {
        let content = fs::read_to_string(script).expect(&format!("Failed to read {}", script));

        assert!(
            content.starts_with("#!/bin/bash"),
            "{} should start with #!/bin/bash shebang",
            script
        );

        // Should have 'set -e' for safety
        assert!(
            content.contains("set -e"),
            "{} should have 'set -e' for error handling",
            script
        );
    }
}

/// Test 15: Verify bash scripts can be checked for syntax errors
#[test]
fn test_bash_scripts_syntax() {
    // This test only runs if bash is available
    let scripts = [
        "scripts/start-cluster.sh",
        "scripts/stop-cluster.sh",
        "scripts/test-cluster.sh",
    ];

    for script in &scripts {
        // Skip if bash is not available
        if Command::new("bash").arg("--version").output().is_err() {
            eprintln!("Bash not available, skipping syntax check for {}", script);
            continue;
        }

        let result = Command::new("bash")
            .arg("-n") // Check syntax only
            .arg(script)
            .output();

        match result {
            Ok(output) => {
                assert!(
                    output.status.success(),
                    "{} has syntax errors: {}",
                    script,
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            Err(e) => {
                eprintln!("Failed to check syntax for {}: {}", script, e);
            }
        }
    }
}

/// Test 16: Verify node configurations have unique IDs and ports
#[test]
fn test_node_configs_have_unique_ids_and_ports() {
    let configs = [
        ("config-node1.toml", 1, 8001, 9001),
        ("config-node2.toml", 2, 8002, 9002),
        ("config-node3.toml", 3, 8003, 9003),
    ];

    for (config_file, expected_id, expected_http, expected_raft) in &configs {
        let content =
            fs::read_to_string(config_file).expect(&format!("Failed to read {}", config_file));

        // Check node ID
        assert!(
            content.contains(&format!("id = {}", expected_id)),
            "{} should have id = {}",
            config_file,
            expected_id
        );

        // Check HTTP port
        assert!(
            content.contains(&expected_http.to_string()),
            "{} should reference port {}",
            config_file,
            expected_http
        );

        // Check Raft port
        assert!(
            content.contains(&expected_raft.to_string()),
            "{} should reference port {}",
            config_file,
            expected_raft
        );
    }
}
