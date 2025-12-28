// Automated integration test - tests the full backup workflow with Docker

use std::process::Command;
use std::path::PathBuf;
use std::env;

#[test]
#[ignore] // Run with: cargo test --test integration_automated -- --ignored
fn test_full_integration_workflow() {
    // Only run on Unix systems where the integration test is set up
    if cfg!(windows) {
        println!("Skipping integration test on Windows (use manual test)");
        return;
    }

    let project_root = env::current_dir().expect("Failed to get current directory");
    let integration_dir = project_root.join("tests/integration");

    // Step 1: Setup
    println!("Setting up integration test environment...");
    let setup_result = Command::new("bash")
        .current_dir(&integration_dir)
        .arg("setup.sh")
        .status()
        .expect("Failed to run setup.sh");

    assert!(setup_result.success(), "Setup script failed");

    // Step 2: Build the project
    println!("Building restic-manager...");
    let build_result = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .status()
        .expect("Failed to build project");

    assert!(build_result.success(), "Build failed");

    // Step 3: Run backup
    println!("Running backup...");
    let config_path = integration_dir.join("config.toml");
    let backup_result = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--")
        .arg("--config")
        .arg(&config_path)
        .arg("run")
        .arg("--service")
        .arg("postgres-backup")
        .status()
        .expect("Failed to run backup");

    assert!(backup_result.success(), "Backup failed");

    // Step 4: Verify backup artifacts
    println!("Verifying backup artifacts...");
    let test_data_dir = integration_dir.join("test-data");
    assert!(test_data_dir.exists(), "Test data directory not created");

    let backup_repo_dir = integration_dir.join("test-backup-repo");
    assert!(backup_repo_dir.exists(), "Backup repository not created");

    // Step 5: Cleanup
    println!("Cleaning up...");
    let cleanup_result = Command::new("bash")
        .current_dir(&integration_dir)
        .arg("cleanup.sh")
        .status()
        .expect("Failed to run cleanup.sh");

    assert!(cleanup_result.success(), "Cleanup script failed");

    println!("Integration test completed successfully!");
}

#[test]
#[ignore] // Run with: cargo test --test integration_automated -- --ignored
fn test_container_deployment() {
    // Only run on Unix systems with Docker
    if cfg!(windows) {
        println!("Skipping container test on Windows");
        return;
    }

    let project_root = env::current_dir().expect("Failed to get current directory");
    let container_dir = project_root.join("tests/container");

    // Step 1: Build the binary first
    println!("Building restic-manager binary...");
    let build_result = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .status()
        .expect("Failed to build project");

    assert!(build_result.success(), "Build failed");

    // Step 2: Setup container environment
    println!("Setting up container test environment...");
    let setup_result = Command::new("bash")
        .current_dir(&container_dir)
        .arg("setup.sh")
        .status()
        .expect("Failed to run setup.sh");

    assert!(setup_result.success(), "Container setup failed");

    // Step 3: Setup restic in container
    println!("Setting up restic in container...");
    let restic_setup = Command::new("docker")
        .arg("exec")
        .arg("restic-manager-test")
        .arg("/app/restic-manager")
        .arg("setup-restic")
        .status()
        .expect("Failed to setup restic");

    assert!(restic_setup.success(), "Restic setup in container failed");

    // Step 4: Create backup directory
    println!("Creating backup directory in container...");
    let mkdir_result = Command::new("docker")
        .arg("exec")
        .arg("restic-manager-test")
        .arg("mkdir")
        .arg("-p")
        .arg("/backup-data/dumps")
        .status()
        .expect("Failed to create directory");

    assert!(mkdir_result.success(), "Failed to create backup directory");

    // Step 5: Run backup in container
    println!("Running backup in container...");
    let backup_result = Command::new("docker")
        .arg("exec")
        .arg("restic-manager-test")
        .arg("/app/restic-manager")
        .arg("--config")
        .arg("/app/config.toml")
        .arg("run")
        .arg("--service")
        .arg("postgres")
        .status()
        .expect("Failed to run backup");

    assert!(backup_result.success(), "Container backup failed");

    // Step 6: Cleanup
    println!("Cleaning up container test...");
    let cleanup_result = Command::new("bash")
        .current_dir(&container_dir)
        .arg("cleanup.sh")
        .status()
        .expect("Failed to run cleanup.sh");

    assert!(cleanup_result.success(), "Cleanup failed");

    println!("Container test completed successfully!");
}
