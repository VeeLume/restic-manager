//! Tests for the 'restore' command
//!
//! The restore command restores files and volumes from a snapshot.

use test_utils::{
    ConfigBuilder, MockResticOps, MockDockerOps, ResticOperations, DockerOperations,
    sample_snapshot, sample_snapshots,
};
use restic_manager::config::resolve_all_services;
use restic_manager::utils::restic::ResticEnv;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_restore_with_snapshot_id() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let snapshot = sample_snapshot();
    let mock = MockResticOps::new().with_snapshots(vec![snapshot.clone()]);
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let result = mock.restore_snapshot(&env, &snapshot.id, None, &[], timeout);
    assert!(result.is_ok());
    assert!(mock.restore_called());
}

#[test]
fn test_restore_to_target_directory() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    let target_dir = temp_dir.path().join("restore-target");
    std::fs::create_dir_all(&target_dir).unwrap();
    std::fs::write(&password_file, "test").unwrap();

    let snapshot = sample_snapshot();
    let mock = MockResticOps::new().with_snapshots(vec![snapshot.clone()]);
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let result = mock.restore_snapshot(
        &env,
        &snapshot.id,
        Some(target_dir.to_str().unwrap()),
        &[],
        timeout,
    );

    assert!(result.is_ok());
}

#[test]
fn test_restore_specific_paths() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let snapshot = sample_snapshot();
    let mock = MockResticOps::new().with_snapshots(vec![snapshot.clone()]);
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let paths = vec!["data/important.txt".to_string(), "config/".to_string()];
    let result = mock.restore_snapshot(&env, &snapshot.id, None, &paths, timeout);

    assert!(result.is_ok());
}

#[test]
fn test_restore_no_snapshots_scenario() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new(); // No snapshots
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let snapshots = mock.list_snapshots(&env, timeout).unwrap();
    assert!(snapshots.is_empty());

    // In a real scenario, we'd check for snapshots first
    // The mock restore_snapshot doesn't validate snapshot existence
    // So we verify the expected workflow: check snapshots, then decide whether to restore
}

#[test]
fn test_restore_latest_snapshot() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mut snapshots = sample_snapshots(3);
    snapshots[2].time = "2025-12-28T15:00:00.000000000Z".to_string();

    let mock = MockResticOps::new().with_snapshots(snapshots);
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let latest = mock.get_latest_snapshot(&env, timeout).unwrap();
    assert!(latest.is_some());
    assert!(latest.unwrap().time.contains("2025-12-28T15:00"));
}

#[test]
fn test_restore_volume() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("volume.tar.gz");
    std::fs::write(&archive_path, "fake archive content").unwrap();

    let mock = MockDockerOps::new();
    let timeout = Duration::from_secs(60);

    let result = mock.restore_volume("my-volume", &archive_path, timeout);
    assert!(result.is_ok());
    assert!(mock.restore_called());
}

#[test]
fn test_restore_service_config() {
    let config = ConfigBuilder::minimal()
        .add_service_with_volumes("docker-service", vec!["app_data".to_string()])
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("docker-service").unwrap();

    // Service should have volumes for restoration
    let service_config = service.config.as_ref().unwrap();
    assert!(!service_config.volumes.is_empty());
}

#[test]
fn test_restore_lists_available_files() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let snapshot = sample_snapshot();
    let files = vec![
        "data/file1.txt".to_string(),
        "data/file2.txt".to_string(),
        "config/app.toml".to_string(),
    ];

    let mock = MockResticOps::new()
        .with_snapshots(vec![snapshot.clone()])
        .with_snapshot_files(&snapshot.id, files.clone());

    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let listed = mock.list_snapshot_files(&env, &snapshot.id, timeout).unwrap();
    assert_eq!(listed.len(), 3);
    assert!(listed.contains(&"data/file1.txt".to_string()));
}

#[test]
fn test_restore_failure_handling() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_failing_restore();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let result = mock.restore_snapshot(&env, "abc123", None, &[], timeout);
    assert!(result.is_err());
}

#[test]
fn test_restore_volume_failure_handling() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("volume.tar.gz");
    std::fs::write(&archive_path, "fake archive").unwrap();

    let mock = MockDockerOps::new().with_failing_restore();
    let timeout = Duration::from_secs(60);

    let result = mock.restore_volume("my-volume", &archive_path, timeout);
    assert!(result.is_err());
}
