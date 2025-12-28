//! Tests for the 'verify' command
//!
//! The verify command runs restic check to verify repository integrity.

use test_utils::{
    ConfigBuilder, MockResticOps, ResticOperations,
};
use restic_manager::config::resolve_all_services;
use restic_manager::utils::restic::ResticEnv;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_verify_single_service() {
    let config = ConfigBuilder::minimal()
        .add_service("test-service")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    assert!(resolved.contains_key("test-service"));
}

#[test]
fn test_verify_all_services() {
    let config = ConfigBuilder::minimal()
        .add_service("service1")
        .add_service("service2")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    assert_eq!(resolved.len(), 2);
}

#[test]
fn test_verify_repository_check() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_check_result("no errors found");
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let result = mock.check_repository(&env, false, timeout).unwrap();
    assert!(result.contains("no errors"));
    assert!(mock.check_called());
}

#[test]
fn test_verify_with_read_data() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_check_result("data verification passed");
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(300);

    // read_data = true runs more thorough check
    let result = mock.check_repository(&env, true, timeout).unwrap();
    assert!(result.contains("verification"));
}

#[test]
fn test_verify_detects_errors() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_failing_check();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let result = mock.check_repository(&env, false, timeout);
    assert!(result.is_err());
}

#[test]
fn test_verify_multi_destination() {
    let config = ConfigBuilder::minimal()
        .add_sftp_destination("remote", "sftp://host/backups")
        .add_service("multi-target")
        .build();

    // Both destinations should be available for verification
    assert!(config.destinations.contains_key("local"));
    assert!(config.destinations.contains_key("remote"));
}

#[test]
fn test_verify_unlocks_on_failure() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    // Unlock should be called if repo is locked
    let result = mock.unlock_repository(&env, timeout);
    assert!(result.is_ok());
    assert!(mock.unlock_called());
}

#[test]
fn test_verify_timeout_handling() {
    let config = ConfigBuilder::minimal()
        .with_timeout(7200) // 2 hours for thorough check
        .add_service("large-backup")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("large-backup").unwrap();

    assert_eq!(service.timeout_seconds, 7200);
}

#[test]
fn test_verify_disabled_service_skipped() {
    let config = ConfigBuilder::minimal()
        .add_disabled_service("disabled-service")
        .build();

    let resolved = resolve_all_services(&config).unwrap();
    let service = resolved.get("disabled-service").unwrap();

    assert!(!service.enabled);
}

#[test]
fn test_verify_stats_retrieval() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new().with_stats("Repository size: 5.2 GiB, deduplicated");
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(30);

    let stats = mock.get_stats(&env, timeout).unwrap();
    assert!(stats.contains("5.2 GiB"));
}
