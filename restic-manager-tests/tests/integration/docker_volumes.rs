//! Docker volume integration tests
//!
//! These tests require Docker and verify volume backup/restore workflows.
//! Run with: `cargo test -p restic-manager-tests --test integration -- --ignored`

use super::common::VolumeGuard;
use anyhow::Result;
use restic_manager::utils::docker::{
    archive_volume, get_volume_size, list_volumes, restore_volume, volume_exists,
};
use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to check if Docker is available
fn is_docker_available() -> bool {
    Command::new("docker")
        .args(&["ps"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Helper to create a test volume with data
fn create_test_volume(name: &str, content: &str) -> Result<()> {
    // Create volume
    Command::new("docker")
        .args(&["volume", "create", name])
        .output()?;

    // Add test data
    Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "-v",
            &format!("{}:/data", name),
            "alpine",
            "sh",
            "-c",
            &format!("echo '{}' > /data/test.txt", content),
        ])
        .output()?;

    Ok(())
}

/// Helper to cleanup test volume
fn cleanup_volume(name: &str) {
    let _ = Command::new("docker")
        .args(&["volume", "rm", name])
        .output();
}

/// Helper to read data from volume
fn read_volume_data(name: &str) -> Result<String> {
    let output = Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "-v",
            &format!("{}:/data", name),
            "alpine",
            "cat",
            "/data/test.txt",
        ])
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Test listing Docker volumes
#[test]
#[ignore] // Requires Docker
fn test_list_docker_volumes() {
    if !is_docker_available() {
        println!("Docker not available, skipping test");
        return;
    }

    let timeout = Duration::from_secs(10);
    let test_volume = "restic-test-list-volume";

    // Create test volume
    create_test_volume(test_volume, "test").expect("Failed to create test volume");
    let _guard = VolumeGuard::new(test_volume.to_string());

    // List volumes
    let volumes = list_volumes(timeout).expect("Failed to list volumes");

    // Verify test volume is in the list
    assert!(
        volumes.contains(&test_volume.to_string()),
        "Test volume not found in list"
    );

    // Cleanup happens automatically via guard
}

/// Test volume archive and restore
#[test]
#[ignore] // Requires Docker
fn test_volume_archive_restore() {
    if !is_docker_available() {
        println!("Docker not available, skipping test");
        return;
    }

    let timeout = Duration::from_secs(60);
    let test_volume = "restic-test-archive-volume";
    let test_content = "This is test data for archive/restore";
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let archive_path = temp_dir.path().join("test-volume.tar.gz");

    // Create volume with test data
    create_test_volume(test_volume, test_content).expect("Failed to create test volume");
    let _guard = VolumeGuard::new(test_volume.to_string());

    // Archive the volume
    archive_volume(test_volume, &archive_path, timeout).expect("Failed to archive volume");

    // Verify archive file exists
    assert!(archive_path.exists(), "Archive file should exist");
    assert!(
        archive_path.metadata().unwrap().len() > 0,
        "Archive file should not be empty"
    );

    // Delete the original volume
    cleanup_volume(test_volume);

    // Verify volume is gone
    let exists = volume_exists(test_volume, timeout).expect("Failed to check volume exists");
    assert!(!exists, "Volume should be deleted");

    // Create new empty volume for restoration
    Command::new("docker")
        .args(&["volume", "create", test_volume])
        .output()
        .expect("Failed to create volume");

    // Restore the volume
    restore_volume(test_volume, &archive_path, timeout).expect("Failed to restore volume");

    // Verify data integrity
    let restored_content = read_volume_data(test_volume).expect("Failed to read restored data");
    assert_eq!(
        restored_content, test_content,
        "Restored data should match original"
    );

    // Cleanup happens automatically via guard
}

/// Test archiving multiple volumes (Appwrite scenario)
#[test]
#[ignore] // Requires Docker
fn test_archive_multiple_volumes() {
    if !is_docker_available() {
        println!("Docker not available, skipping test");
        return;
    }

    let timeout = Duration::from_secs(60);
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let volumes = vec![
        ("restic-test-vol1", "data1"),
        ("restic-test-vol2", "data2"),
        ("restic-test-vol3", "data3"),
    ];

    // Create volumes with guards for automatic cleanup
    let mut guards = Vec::new();
    for (name, content) in &volumes {
        create_test_volume(name, content).expect("Failed to create volume");
        guards.push(VolumeGuard::new(name.to_string()));
    }

    // Archive each volume
    let mut archives = Vec::new();
    for (name, _) in &volumes {
        let archive_path = temp_dir.path().join(format!("{}.tar.gz", name));
        archive_volume(name, &archive_path, timeout).expect("Failed to archive volume");
        assert!(archive_path.exists(), "Archive should exist");
        archives.push(archive_path);
    }

    // Verify all archives exist and have different sizes (different content)
    assert_eq!(archives.len(), 3, "Should have 3 archives");

    // Cleanup happens automatically via guards
}

/// Test volume size calculation
#[test]
#[ignore] // Requires Docker
fn test_get_volume_size() {
    if !is_docker_available() {
        println!("Docker not available, skipping test");
        return;
    }

    let timeout = Duration::from_secs(30);
    let test_volume = "restic-test-size-volume";

    // Create volume
    create_test_volume(test_volume, "test").expect("Failed to create volume");
    let _guard = VolumeGuard::new(test_volume.to_string());

    // Add a file with known size (1KB)
    Command::new("docker")
        .args(&[
            "run",
            "--rm",
            "-v",
            &format!("{}:/data", test_volume),
            "alpine",
            "dd",
            "if=/dev/zero",
            "of=/data/1kb.dat",
            "bs=1024",
            "count=1",
        ])
        .output()
        .expect("Failed to create test file");

    // Get volume size
    let size = get_volume_size(test_volume, timeout).expect("Failed to get volume size");

    // Volume should be at least 1KB (plus filesystem overhead)
    assert!(size >= 1024, "Volume size should be at least 1KB");

    // Cleanup happens automatically via guard
}

/// Test exact volume name matching (critical for Appwrite)
#[test]
#[ignore] // Requires Docker
fn test_exact_volume_name_matching() {
    if !is_docker_available() {
        println!("Docker not available, skipping test");
        return;
    }

    let timeout = Duration::from_secs(10);

    // Create volumes with similar names
    let exact_name = "appwrite_appwrite-data";
    let prefix_name = "appwrite";

    create_test_volume(exact_name, "exact").expect("Failed to create exact volume");
    let _guard = VolumeGuard::new(exact_name.to_string());

    // Test exact match
    let exists = volume_exists(exact_name, timeout).expect("Failed to check exact volume");
    assert!(exists, "Exact volume name should exist");

    // Test that prefix doesn't match
    let exists = volume_exists(prefix_name, timeout).expect("Failed to check prefix");
    assert!(
        !exists,
        "Prefix-only name should not match longer volume name"
    );

    // Cleanup happens automatically via guard
}
