//! Unit tests for Docker utilities
//!
//! These tests verify Docker volume operations using mock implementations.

use test_utils::{appwrite_volumes, MockDockerOps, DockerOperations};
use restic_manager::utils::docker_ops::mock::DockerCall;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_mock_docker_ops_list_volumes() {
    let mock = MockDockerOps::new().with_volumes(vec![
        "volume1".to_string(),
        "volume2".to_string(),
        "volume3".to_string(),
    ]);

    let timeout = Duration::from_secs(10);
    let volumes = mock.list_volumes(timeout).unwrap();

    assert!(mock.list_volumes_called());
    assert_eq!(volumes.len(), 3);
    assert!(volumes.contains(&"volume1".to_string()));
    assert!(volumes.contains(&"volume2".to_string()));
    assert!(volumes.contains(&"volume3".to_string()));
}

#[test]
fn test_mock_docker_ops_volume_exists_exact_match() {
    let mock = MockDockerOps::new().with_volumes(vec![
        "appwrite_appwrite-data".to_string(),
        "appwrite_appwrite-cache".to_string(),
        "other-volume".to_string(),
    ]);

    let timeout = Duration::from_secs(10);

    // Exact match should work
    assert!(mock.volume_exists("appwrite_appwrite-data", timeout).unwrap());
    assert!(mock.volume_exists("appwrite_appwrite-cache", timeout).unwrap());
    assert!(mock.volume_exists("other-volume", timeout).unwrap());

    // Substring should NOT match (critical for Appwrite!)
    assert!(!mock.volume_exists("appwrite", timeout).unwrap());
    assert!(!mock.volume_exists("data", timeout).unwrap());
    assert!(!mock.volume_exists("appwrite_appwrite", timeout).unwrap());
    assert!(!mock.volume_exists("cache", timeout).unwrap());
}

#[test]
fn test_mock_docker_ops_archive_volume() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("volume.tar.gz");

    let mock = MockDockerOps::new().with_volumes(vec!["my-volume".to_string()]);

    let timeout = Duration::from_secs(60);
    let result = mock.archive_volume("my-volume", &archive_path, timeout);

    assert!(result.is_ok());
    assert!(mock.archive_called());

    let calls = mock.archive_calls_for("my-volume");
    assert_eq!(calls.len(), 1);
}

#[test]
fn test_mock_docker_ops_archive_multiple_volumes() {
    let temp_dir = TempDir::new().unwrap();
    let volumes = appwrite_volumes();

    let mock = MockDockerOps::new().with_volumes(volumes.clone());
    let timeout = Duration::from_secs(60);

    // Archive each volume
    for volume in &volumes {
        let archive_path = temp_dir.path().join(format!("{}.tar.gz", volume));
        let result = mock.archive_volume(volume, &archive_path, timeout);
        assert!(result.is_ok(), "Failed to archive {}", volume);
    }

    // Verify all archives were called
    let calls = mock.get_calls();
    let archive_count = calls
        .iter()
        .filter(|c| matches!(c, DockerCall::ArchiveVolume { .. }))
        .count();

    assert_eq!(archive_count, volumes.len());
}

#[test]
fn test_mock_docker_ops_archive_failure() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("volume.tar.gz");

    let mock = MockDockerOps::new().with_failing_archive();

    let timeout = Duration::from_secs(60);
    let result = mock.archive_volume("my-volume", &archive_path, timeout);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Mock archive failure"));
}

#[test]
fn test_mock_docker_ops_restore_volume() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("volume.tar.gz");
    // Create a dummy archive file
    std::fs::write(&archive_path, "fake archive data").unwrap();

    let mock = MockDockerOps::new();

    let timeout = Duration::from_secs(60);
    let result = mock.restore_volume("restore-volume", &archive_path, timeout);

    assert!(result.is_ok());
    assert!(mock.restore_called());
}

#[test]
fn test_mock_docker_ops_restore_failure() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("volume.tar.gz");
    std::fs::write(&archive_path, "fake archive").unwrap();

    let mock = MockDockerOps::new().with_failing_restore();

    let timeout = Duration::from_secs(60);
    let result = mock.restore_volume("my-volume", &archive_path, timeout);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Mock restore failure"));
}

#[test]
fn test_mock_docker_ops_get_volume_size() {
    let mock = MockDockerOps::new()
        .with_volumes(vec!["sized-volume".to_string()])
        .with_volume_size("sized-volume", 1024 * 1024 * 100); // 100 MiB

    let timeout = Duration::from_secs(10);
    let size = mock.get_volume_size("sized-volume", timeout).unwrap();

    assert_eq!(size, 104857600); // 100 MiB in bytes
}

#[test]
fn test_mock_docker_ops_volume_size_default() {
    let mock = MockDockerOps::new().with_volumes(vec!["default-sized".to_string()]);

    let timeout = Duration::from_secs(10);
    let size = mock.get_volume_size("default-sized", timeout).unwrap();

    // Default size is 1024 bytes
    assert_eq!(size, 1024);
}

#[test]
fn test_mock_docker_ops_list_failure() {
    let mock = MockDockerOps::new().with_failing_list();

    let timeout = Duration::from_secs(10);
    let result = mock.list_volumes(timeout);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Mock list_volumes failure"));
}

#[test]
fn test_mock_docker_ops_volume_exists_failure() {
    let mock = MockDockerOps::new().with_failing_list();

    let timeout = Duration::from_secs(10);
    let result = mock.volume_exists("any-volume", timeout);

    assert!(result.is_err());
}

#[test]
fn test_appwrite_volume_names() {
    let volumes = appwrite_volumes();

    // All Appwrite volumes should have the doubled prefix
    for volume in &volumes {
        assert!(
            volume.starts_with("appwrite_appwrite-"),
            "Volume {} doesn't have expected prefix",
            volume
        );
    }

    // Should include key volumes
    assert!(volumes.iter().any(|v| v.contains("mariadb")));
    assert!(volumes.iter().any(|v| v.contains("redis")));
    assert!(volumes.iter().any(|v| v.contains("uploads")));
}

#[test]
fn test_mock_docker_ops_call_tracking() {
    let mock = MockDockerOps::new().with_volumes(vec!["vol1".to_string(), "vol2".to_string()]);

    let timeout = Duration::from_secs(10);

    // Perform various operations
    mock.list_volumes(timeout).unwrap();
    mock.volume_exists("vol1", timeout).unwrap();
    mock.volume_exists("vol2", timeout).unwrap();

    let calls = mock.get_calls();

    // Should have 3 calls
    assert_eq!(calls.len(), 3);
}
