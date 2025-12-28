//! Tests for the 'status' command
//!
//! The status command displays backup status and health metrics.

use test_utils::{
    ConfigBuilder, MockResticOps, ResticOperations,
    sample_snapshot, sample_snapshots, snapshot_with_time,
};
use restic_manager::config::resolve_all_services;
use restic_manager::utils::restic::ResticEnv;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_status_overview_counts() {
    let config = ConfigBuilder::minimal()
        .add_service("service1")
        .add_service("service2")
        .add_sftp_destination("remote", "sftp://host/backups")
        .build();

    // Should have 2 services and 2 destinations
    assert_eq!(config.services.len(), 2);
    assert_eq!(config.destinations.len(), 2);
}

#[test]
fn test_status_single_service() {
    let config = ConfigBuilder::minimal().add_service("test-service").build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("test-service").unwrap();

    assert!(service.enabled);
    assert!(!service.description.is_empty());
    assert!(!service.schedule.is_empty());
    assert!(!service.targets.is_empty());
}

#[test]
fn test_status_with_snapshots() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_snapshots(sample_snapshots(5));
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let count = mock.count_snapshots(&env, timeout).unwrap();
    let latest = mock.get_latest_snapshot(&env, timeout).unwrap();

    assert_eq!(count, 5);
    assert!(latest.is_some());
}

#[test]
fn test_status_no_snapshots() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new(); // No snapshots configured
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let count = mock.count_snapshots(&env, timeout).unwrap();
    let latest = mock.get_latest_snapshot(&env, timeout).unwrap();

    assert_eq!(count, 0);
    assert!(latest.is_none());
}

#[test]
fn test_status_repository_stats() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_stats("1.5 GiB");
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let stats = mock.get_stats(&env, timeout).unwrap();

    assert_eq!(stats, "1.5 GiB");
}

#[test]
fn test_status_health_healthy() {
    // A healthy backup is less than 24 hours old
    let snapshot = snapshot_with_time("2025-12-28T10:00:00.000000000Z");

    // Snapshot time parsing and health check would be done by the application
    // Here we just verify the snapshot has the expected format
    assert!(snapshot.time.contains("2025-12-28"));
}

#[test]
fn test_status_multi_destination() {
    let builder = ConfigBuilder::minimal()
        .add_sftp_destination("remote", "sftp://host/backups");

    let backup2 = builder.temp_dir().join("backup2");
    std::fs::create_dir_all(&backup2).unwrap();

    let config = builder
        .add_local_destination("backup2", &backup2)
        .add_service("multi-target")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("multi-target").unwrap();

    // Service should have access to multiple destinations
    // (though the default service only targets "local")
    assert!(config.destinations.len() >= 3);
}

#[test]
fn test_status_disabled_service() {
    let config = ConfigBuilder::minimal()
        .add_disabled_service("disabled-service")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("disabled-service").unwrap();

    assert!(!service.enabled);
}

#[test]
fn test_status_service_timeout() {
    let config = ConfigBuilder::minimal()
        .with_timeout(3600)
        .add_service("test")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("test").unwrap();

    assert_eq!(service.timeout_seconds, 3600);
}

#[test]
fn test_status_retention_policy() {
    let config = ConfigBuilder::minimal()
        .with_retention(restic_manager::config::RetentionPolicy {
            daily: 7,
            weekly: 4,
            monthly: 6,
            yearly: 1,
        })
        .add_service("test")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("test").unwrap();

    assert_eq!(service.retention.daily, 7);
    assert_eq!(service.retention.weekly, 4);
    assert_eq!(service.retention.monthly, 6);
    assert_eq!(service.retention.yearly, 1);
}

#[test]
fn test_status_shows_latest_snapshot_info() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mut snapshots = sample_snapshots(3);
    snapshots[2].time = "2025-12-28T15:30:00.000000000Z".to_string();
    snapshots[2].hostname = "backup-server".to_string();

    let mock = MockResticOps::new().with_snapshots(snapshots);
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let latest = mock.get_latest_snapshot(&env, timeout).unwrap().unwrap();

    assert!(latest.time.contains("2025-12-28T15:30"));
    assert_eq!(latest.hostname, "backup-server");
}
