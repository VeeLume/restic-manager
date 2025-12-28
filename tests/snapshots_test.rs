// Test for snapshots command functionality

use std::process::Command;

#[test]
#[ignore] // Run manually: cargo test --test snapshots_test -- --ignored
fn test_snapshots_command_help() {
    // Test that snapshots command has proper help text
    let output = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--")
        .arg("snapshots")
        .arg("--help")
        .output()
        .expect("Failed to run snapshots --help");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify help text contains expected information
    assert!(stdout.contains("Show available snapshots"));
    assert!(stdout.contains("--service"));
    assert!(stdout.contains("--destination"));
}

#[test]
#[ignore] // Requires valid configuration
fn test_snapshots_nonexistent_service() {
    // Test error handling for non-existent service
    let output = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--")
        .arg("--config")
        .arg("config.example.toml")
        .arg("snapshots")
        .arg("--service")
        .arg("nonexistent-service")
        .output()
        .expect("Failed to run command");

    // Should fail with error about service not found
    assert!(!output.status.success());
}
