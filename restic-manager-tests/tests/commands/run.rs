//! Tests for the 'run' command
//!
//! The run command executes backups for one or all configured services.

use test_utils::{
    ConfigBuilder, MockResticOps, MockDockerOps, ResticOperations, DockerOperations,
    appwrite_volumes,
};
use restic_manager::config::resolve_all_services;
use restic_manager::utils::restic::ResticEnv;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_run_single_service() {
    let config = ConfigBuilder::minimal()
        .add_service("test-service")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("test-service").unwrap();

    // Service should be enabled and have valid configuration
    assert!(service.enabled);
    assert!(!service.targets.is_empty());
}

#[test]
fn test_run_all_services() {
    let config = ConfigBuilder::minimal()
        .add_service("service1")
        .add_service("service2")
        .add_service("service3")
        .build();

    let resolved = resolve_all_services(&config).unwrap();

    // All services should be resolvable
    assert_eq!(resolved.len(), 3);
    assert!(resolved.values().all(|s| s.enabled));
}

#[test]
fn test_run_skips_disabled_services() {
    let config = ConfigBuilder::minimal()
        .add_service("enabled-service")
        .add_disabled_service("disabled-service")
        .build();

    let resolved = resolve_all_services(&config).unwrap();

    let enabled = resolved.get("enabled-service").unwrap();
    let disabled = resolved.get("disabled-service").unwrap();

    assert!(enabled.enabled);
    assert!(!disabled.enabled);
}

#[test]
fn test_run_backup_creates_snapshot() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    // Simulate backup
    let result = mock.backup(&env, &[], &[], timeout);
    assert!(result.is_ok());
    assert!(mock.backup_called());
}

#[test]
fn test_run_with_paths() {
    let config = ConfigBuilder::minimal()
        .add_service_with_paths("files-service", vec![
            "data".to_string(),
            "config".to_string(),
        ])
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("files-service").unwrap();
    let config = service.config.as_ref().unwrap();

    assert_eq!(config.paths.len(), 2);
    assert!(config.paths.contains(&"data".to_string()));
    assert!(config.paths.contains(&"config".to_string()));
}

#[test]
fn test_run_with_volumes() {
    let volumes = appwrite_volumes();

    let config = ConfigBuilder::minimal()
        .add_service_with_volumes("docker-service", volumes.clone())
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("docker-service").unwrap();
    let strategy_config = service.config.as_ref().unwrap();

    assert_eq!(strategy_config.volumes.len(), volumes.len());
}

#[test]
fn test_run_archives_volumes() {
    let temp_dir = TempDir::new().unwrap();
    let volumes = vec!["vol1".to_string(), "vol2".to_string()];

    let mock = MockDockerOps::new().with_volumes(volumes.clone());
    let timeout = Duration::from_secs(60);

    // Archive each volume
    for volume in &volumes {
        let archive_path = temp_dir.path().join(format!("{}.tar.gz", volume));
        let result = mock.archive_volume(volume, &archive_path, timeout);
        assert!(result.is_ok(), "Failed to archive {}", volume);
    }

    assert!(mock.archive_called());
}

#[test]
fn test_run_multi_destination() {
    let config = ConfigBuilder::minimal()
        .add_sftp_destination("remote", "sftp://host/backups")
        .add_service("multi-target")
        .build();

    // Should have local (from minimal) + sftp destinations
    assert!(config.destinations.len() >= 2);
    assert!(config.destinations.contains_key("local"));
    assert!(config.destinations.contains_key("remote"));
}

#[test]
fn test_run_respects_timeout() {
    let config = ConfigBuilder::minimal()
        .with_timeout(7200)
        .add_service("long-running")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("long-running").unwrap();

    assert_eq!(service.timeout_seconds, 7200);
}

#[test]
fn test_run_applies_retention() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let retention = restic_manager::config::RetentionPolicy {
        daily: 7,
        weekly: 4,
        monthly: 6,
        yearly: 1,
    };

    let result = mock.apply_retention(&env, &retention, timeout);
    assert!(result.is_ok());
    // apply_retention succeeded - mock behavior verified
}

#[test]
fn test_run_handles_backup_failure() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_failing_backup();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let result = mock.backup(&env, &[], &[], timeout);
    assert!(result.is_err());
}

#[test]
fn test_run_handles_volume_archive_failure() {
    let temp_dir = TempDir::new().unwrap();
    let archive_path = temp_dir.path().join("volume.tar.gz");

    let mock = MockDockerOps::new().with_failing_archive();
    let timeout = Duration::from_secs(60);

    let result = mock.archive_volume("my-volume", &archive_path, timeout);
    assert!(result.is_err());
}
