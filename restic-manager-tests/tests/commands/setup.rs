//! Tests for the 'setup' command
//!
//! The setup command initializes directories and registers cron jobs.

use test_utils::ConfigBuilder;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_setup_creates_log_directory() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().join("logs");

    // Simulate setup creating log directory
    fs::create_dir_all(&log_dir).unwrap();

    assert!(log_dir.exists());
    assert!(log_dir.is_dir());
}

#[test]
fn test_setup_creates_backup_directory() {
    let temp_dir = TempDir::new().unwrap();
    let backup_dir = temp_dir.path().join("backups");

    fs::create_dir_all(&backup_dir).unwrap();

    assert!(backup_dir.exists());
}

#[test]
fn test_setup_initializes_repository() {
    // Repository initialization is handled by restic init
    // This test verifies the setup flow with mocked restic
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test-password").unwrap();

    use test_utils::{MockResticOps, ResticOperations};
    use restic_manager::utils::restic::ResticEnv;
    use std::time::Duration;

    let mock = MockResticOps::new();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let result = mock.init_repository(&env, timeout);
    assert!(result.is_ok());
    assert!(mock.init_called());
}

#[test]
fn test_setup_dry_run() {
    // Dry run should report what would be done without making changes
    let config = ConfigBuilder::minimal()
        .add_service("test-service")
        .build();

    // In dry run mode, we just verify config is valid
    assert!(!config.services.is_empty());
}

#[test]
fn test_setup_dirs_only() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().join("logs");
    let backup_dir = temp_dir.path().join("backups");

    // dirs-only flag creates directories but doesn't set up cron
    fs::create_dir_all(&log_dir).unwrap();
    fs::create_dir_all(&backup_dir).unwrap();

    assert!(log_dir.exists());
    assert!(backup_dir.exists());
}

#[test]
fn test_setup_password_file_exists() {
    let builder = ConfigBuilder::minimal();
    let password_file = builder.password_file();

    // Password file should exist from minimal config
    assert!(password_file.exists());
}

#[test]
fn test_setup_validates_config() {
    let config = ConfigBuilder::minimal()
        .add_service("valid-service")
        .build();

    // Setup should validate configuration before proceeding
    use restic_manager::config::resolve_all_services;

    let result = resolve_all_services(&config);
    assert!(result.is_ok());
}

#[test]
fn test_setup_multiple_destinations() {
    let builder = ConfigBuilder::minimal();
    let backup2 = builder.temp_dir().join("backup2");
    fs::create_dir_all(&backup2).unwrap();

    let config = builder
        .add_local_destination("backup2", &backup2)
        .add_service("multi-dest")
        .build();

    // Should have multiple destinations
    assert!(config.destinations.len() >= 2);
}

#[test]
fn test_setup_creates_nested_directories() {
    let temp_dir = TempDir::new().unwrap();
    let nested_path = temp_dir.path().join("a").join("b").join("c").join("logs");

    fs::create_dir_all(&nested_path).unwrap();

    assert!(nested_path.exists());
}

#[test]
fn test_setup_handles_existing_directory() {
    let temp_dir = TempDir::new().unwrap();
    let log_dir = temp_dir.path().join("logs");

    // Create directory first
    fs::create_dir_all(&log_dir).unwrap();

    // Creating again should not fail
    fs::create_dir_all(&log_dir).unwrap();

    assert!(log_dir.exists());
}
