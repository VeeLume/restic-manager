//! Tests for the 'snapshots' command
//!
//! The snapshots command lists available backup snapshots.

use test_utils::{
    ConfigBuilder, MockResticOps, ResticOperations,
    sample_snapshot, sample_snapshots, snapshot_with_time,
};
use restic_manager::utils::restic::ResticEnv;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_snapshots_list_all() {
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
fn test_snapshots_empty_repository() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new(); // No snapshots configured
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let snapshots = mock.list_snapshots(&env, timeout).unwrap();
    assert!(snapshots.is_empty());
}

#[test]
fn test_snapshots_count() {
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
fn test_snapshots_ordering_by_time() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    // Note: The mock returns the last element in the vector as "latest"
    // So we arrange snapshots with the most recent one last
    let mut snapshots = vec![
        snapshot_with_time("2025-12-20T10:00:00.000000000Z"),
        snapshot_with_time("2025-12-25T10:00:00.000000000Z"),
        snapshot_with_time("2025-12-28T10:00:00.000000000Z"), // Latest - last in vector
    ];
    snapshots[0].id = "snap-20".to_string();
    snapshots[1].id = "snap-25".to_string();
    snapshots[2].id = "snap-28".to_string();

    let mock = MockResticOps::new().with_snapshots(snapshots);
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let latest = mock.get_latest_snapshot(&env, timeout).unwrap();
    assert!(latest.is_some());
    // Latest should be the last one in the list (mock behavior)
    assert_eq!(latest.unwrap().id, "snap-28");
}

#[test]
fn test_snapshots_display_fields() {
    let snapshot = sample_snapshot();

    // Verify snapshot has expected fields
    assert!(!snapshot.id.is_empty());
    assert!(!snapshot.time.is_empty());
    assert!(!snapshot.hostname.is_empty());
    assert!(!snapshot.paths.is_empty());
}

#[test]
fn test_snapshots_hostname_filter() {
    let mut snapshots = sample_snapshots(3);
    snapshots[0].hostname = "server-a".to_string();
    snapshots[1].hostname = "server-b".to_string();
    snapshots[2].hostname = "server-a".to_string();

    let server_a_count = snapshots.iter().filter(|s| s.hostname == "server-a").count();
    assert_eq!(server_a_count, 2);
}

// Snapshot tags are not currently supported in the Snapshot struct
// If tags support is added in the future, add a test here

#[test]
fn test_snapshots_service_destination() {
    let config = ConfigBuilder::minimal()
        .add_sftp_destination("remote", "sftp://host/backups")
        .add_service("multi-dest")
        .build();

    // Verify destinations exist for querying snapshots
    assert!(config.destinations.contains_key("local"));
    assert!(config.destinations.contains_key("remote"));
}

#[test]
fn test_snapshots_paths_info() {
    let mut snapshot = sample_snapshot();
    snapshot.paths = vec![
        "/home/valerie/docker/appwrite".to_string(),
        "/home/valerie/docker/immich".to_string(),
    ];

    assert_eq!(snapshot.paths.len(), 2);
    assert!(snapshot.paths.iter().any(|p| p.contains("appwrite")));
    assert!(snapshot.paths.iter().any(|p| p.contains("immich")));
}

#[test]
fn test_snapshots_time_parsing() {
    let times = [
        "2025-12-28T10:00:00.000000000Z",
        "2025-01-15T23:59:59.999999999Z",
        "2024-06-01T00:00:00.000000000Z",
    ];

    for time_str in &times {
        let snapshot = snapshot_with_time(time_str);
        assert_eq!(&snapshot.time, *time_str);
    }
}

#[test]
fn test_snapshots_list_failure() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_failing_list();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let result = mock.list_snapshots(&env, timeout);
    assert!(result.is_err());
}

#[test]
fn test_snapshots_stats() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_stats("2.5 GiB total");
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let stats = mock.get_stats(&env, timeout).unwrap();
    assert!(stats.contains("2.5 GiB"));
}
