//! Unit tests for restic utilities
//!
//! These tests verify restic URL building, environment handling, and snapshot parsing.

use test_utils::{sample_snapshot, sample_snapshots, MockResticOps, ResticOperations};
use restic_manager::config::{Destination, DestinationType, RetentionPolicy};
use restic_manager::utils::restic::{build_repository_url, ResticEnv};
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_build_repository_url_with_trailing_slash() {
    let destination = Destination {
        dest_type: DestinationType::Sftp,
        url: "sftp://user@host/backups/".to_string(),
        description: "Test".to_string(),
    };

    let url = build_repository_url(&destination, "postgres", None);
    assert_eq!(url, "sftp://user@host/backups/postgres");
}

#[test]
fn test_build_repository_url_without_trailing_slash() {
    let destination = Destination {
        dest_type: DestinationType::Sftp,
        url: "sftp://user@host/backups".to_string(),
        description: "Test".to_string(),
    };

    let url = build_repository_url(&destination, "postgres", None);
    assert_eq!(url, "sftp://user@host/backups/postgres");
}

#[test]
fn test_build_repository_url_with_suffix() {
    let destination = Destination {
        dest_type: DestinationType::Local,
        url: "/tmp/backups".to_string(),
        description: "Test".to_string(),
    };

    let url = build_repository_url(&destination, "postgres", Some("-prod"));
    assert_eq!(url, "/tmp/backups/postgres-prod");
}

#[test]
fn test_build_repository_url_local() {
    let destination = Destination {
        dest_type: DestinationType::Local,
        url: "/var/backups".to_string(),
        description: "Test".to_string(),
    };

    let url = build_repository_url(&destination, "myservice", None);
    assert_eq!(url, "/var/backups/myservice");
}

#[test]
fn test_restic_env_creation() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password.txt");
    std::fs::write(&password_file, "test-password").unwrap();

    let env = ResticEnv::new(&password_file, "sftp://user@host/backups");

    let vars = env.vars();
    assert_eq!(vars.len(), 2);
    assert!(vars.contains_key("RESTIC_PASSWORD_FILE"));
    assert!(vars.contains_key("RESTIC_REPOSITORY"));
    assert_eq!(
        vars.get("RESTIC_REPOSITORY").unwrap(),
        "sftp://user@host/backups"
    );
}

#[test]
fn test_mock_restic_ops_init() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let result = mock.init_repository(&env, timeout);
    assert!(result.is_ok());
    assert!(mock.init_called());
}

#[test]
fn test_mock_restic_ops_backup() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let paths = vec![PathBuf::from("/data"), PathBuf::from("/config")];
    let result = mock.backup(&env, &paths, &[], timeout);

    assert!(result.is_ok());
    assert!(mock.backup_called());
}

#[test]
fn test_mock_restic_ops_backup_failure() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_failing_backup();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let result = mock.backup(&env, &[PathBuf::from("/data")], &[], timeout);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Mock backup failure"));
}

#[test]
fn test_mock_restic_ops_list_snapshots() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_snapshots(sample_snapshots(5));
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let snapshots = mock.list_snapshots(&env, timeout).unwrap();

    assert_eq!(snapshots.len(), 5);
}

#[test]
fn test_mock_restic_ops_get_latest_snapshot() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mut snapshots = sample_snapshots(3);
    // Make the last one distinguishable
    snapshots[2].id = "latest-snapshot-id".to_string();

    let mock = MockResticOps::new().with_snapshots(snapshots);
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let latest = mock.get_latest_snapshot(&env, timeout).unwrap();

    assert!(latest.is_some());
    assert_eq!(latest.unwrap().id, "latest-snapshot-id");
}

#[test]
fn test_mock_restic_ops_count_snapshots() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_snapshots(sample_snapshots(10));
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let count = mock.count_snapshots(&env, timeout).unwrap();

    assert_eq!(count, 10);
}

#[test]
fn test_mock_restic_ops_restore() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_snapshots(vec![sample_snapshot()]);
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let result = mock.restore_snapshot(&env, "abc123", Some("/tmp/restore"), &[], timeout);

    assert!(result.is_ok());
    assert!(mock.restore_called());
}

#[test]
fn test_mock_restic_ops_restore_failure() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_failing_restore();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let result = mock.restore_snapshot(&env, "abc123", None, &[], timeout);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Mock restore failure"));
}

#[test]
fn test_mock_restic_ops_check() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_check_result("no errors found");
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(300);

    let result = mock.check_repository(&env, false, timeout).unwrap();

    assert_eq!(result, "no errors found");
}

#[test]
fn test_mock_restic_ops_stats() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_stats("2.5 GiB");
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let stats = mock.get_stats(&env, timeout).unwrap();

    assert_eq!(stats, "2.5 GiB");
}

#[test]
fn test_mock_restic_ops_apply_retention() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(300);
    let retention = RetentionPolicy {
        daily: 7,
        weekly: 4,
        monthly: 6,
        yearly: 1,
    };

    let result = mock.apply_retention(&env, &retention, timeout);

    assert!(result.is_ok());
}

#[test]
fn test_mock_restic_ops_unlock() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let result = mock.unlock_repository(&env, timeout);

    assert!(result.is_ok());
}

#[test]
fn test_snapshot_fixture() {
    let snapshot = sample_snapshot();

    assert!(!snapshot.id.is_empty());
    assert!(!snapshot.short_id.is_empty());
    assert!(snapshot.time.contains("2025"));
    assert_eq!(snapshot.hostname, "test-host");
}
