//! Tests for the 'validate' command
//!
//! The validate command checks configuration file syntax and validity.

use test_utils::{ConfigBuilder, TestContext};
use restic_manager::config::load_config;
use std::fs;

#[test]
fn test_validate_valid_config() {
    let builder = ConfigBuilder::minimal().add_service("test-service");
    let (config, temp_dir) = builder.persist();

    let config_path = temp_dir.path().join("config.toml");
    let toml_str = toml::to_string_pretty(&config).unwrap();
    fs::write(&config_path, toml_str).unwrap();

    let result = load_config(&config_path);
    assert!(result.is_ok(), "Valid config should pass validation");

    let loaded = result.unwrap();
    assert_eq!(loaded.services.len(), 1);
    assert_eq!(loaded.destinations.len(), 1);
}

#[test]
fn test_validate_multiple_services() {
    let builder = ConfigBuilder::minimal()
        .add_service("service1")
        .add_service("service2")
        .add_service("service3");

    let (config, temp_dir) = builder.persist();

    let config_path = temp_dir.path().join("config.toml");
    let toml_str = toml::to_string_pretty(&config).unwrap();
    fs::write(&config_path, toml_str).unwrap();

    let result = load_config(&config_path);
    assert!(result.is_ok());

    let loaded = result.unwrap();
    assert_eq!(loaded.services.len(), 3);
}

#[test]
fn test_validate_invalid_toml() {
    let ctx = TestContext::new();
    let config_path = ctx.create_file("config.toml", "invalid { toml content");

    let result = load_config(&config_path);
    assert!(result.is_err(), "Invalid TOML should fail");
}

#[test]
fn test_validate_missing_required_field() {
    let ctx = TestContext::new();

    // Config missing required global.restic_password_file
    let config_content = r#"
[global]
docker_base = "."
log_directory = "."

[destinations.local]
type = "local"
url = "."
description = "Test"
"#;

    let config_path = ctx.create_file("config.toml", config_content);
    let result = load_config(&config_path);

    // Should fail due to missing required field
    assert!(result.is_err());
}

#[test]
fn test_validate_empty_config() {
    let ctx = TestContext::new();
    let config_path = ctx.create_file("config.toml", "");

    let result = load_config(&config_path);
    assert!(result.is_err(), "Empty config should fail");
}

#[test]
fn test_validate_nonexistent_file() {
    let result = load_config(std::path::Path::new("/nonexistent/config.toml"));
    assert!(result.is_err(), "Nonexistent file should fail");
}

#[test]
fn test_validate_config_with_all_destination_types() {
    let builder = ConfigBuilder::minimal()
        .add_sftp_destination("sftp-dest", "sftp://user@host/backups")
        .add_service("test");

    let (config, temp_dir) = builder.persist();

    let config_path = temp_dir.path().join("config.toml");
    let toml_str = toml::to_string_pretty(&config).unwrap();
    fs::write(&config_path, toml_str).unwrap();

    let result = load_config(&config_path);
    assert!(result.is_ok());

    let loaded = result.unwrap();
    assert_eq!(loaded.destinations.len(), 2); // local + sftp
}

#[test]
fn test_validate_service_with_invalid_target() {
    let builder = ConfigBuilder::minimal();
    // Convert Windows backslashes to forward slashes for TOML compatibility
    let password_file = builder.password_file().to_path_buf()
        .to_string_lossy().replace('\\', "/");
    let docker_base = builder.temp_dir().join("docker")
        .to_string_lossy().replace('\\', "/");
    let log_dir = builder.temp_dir().join("logs")
        .to_string_lossy().replace('\\', "/");
    let backup_path = builder.temp_dir().join("backups")
        .to_string_lossy().replace('\\', "/");

    let (_, temp_dir) = builder.persist();

    // Create config with service targeting non-existent destination
    let config_content = format!(r#"
[global]
restic_password_file = "{}"
docker_base = "{}"
log_directory = "{}"

[destinations.local]
type = "local"
url = "{}"
description = "Local"

[services.test]
enabled = true
description = "Test"
schedule = "0 2 * * *"
targets = ["nonexistent-destination"]
strategy = "generic"
"#,
        password_file,
        docker_base,
        log_dir,
        backup_path
    );

    let config_path = temp_dir.path().join("config.toml");
    fs::write(&config_path, config_content).unwrap();

    let result = load_config(&config_path);
    assert!(result.is_err(), "Invalid target should fail validation");
}

#[test]
fn test_validate_cron_schedule_formats() {
    let valid_schedules = [
        "0 2 * * *",      // Daily at 2 AM
        "0 0 * * 0",      // Weekly on Sunday
        "0 0 1 * *",      // Monthly on 1st
        "*/15 * * * *",   // Every 15 minutes
        "0 0 * * 1-5",    // Weekdays at midnight
    ];

    for schedule in &valid_schedules {
        let builder = ConfigBuilder::minimal();
        // Convert Windows backslashes to forward slashes for TOML compatibility
        let password_file = builder.password_file().to_path_buf()
            .to_string_lossy().replace('\\', "/");
        let docker_base = builder.temp_dir().join("docker")
            .to_string_lossy().replace('\\', "/");
        let log_dir = builder.temp_dir().join("logs")
            .to_string_lossy().replace('\\', "/");
        let backup_path = builder.temp_dir().join("backups")
            .to_string_lossy().replace('\\', "/");

        let (_, temp_dir) = builder.persist();

        let config_content = format!(r#"
[global]
restic_password_file = "{}"
docker_base = "{}"
log_directory = "{}"

[destinations.local]
type = "local"
url = "{}"
description = "Local"

[services.test]
enabled = true
description = "Test"
schedule = "{}"
targets = ["local"]
strategy = "generic"
"#,
            password_file,
            docker_base,
            log_dir,
            backup_path,
            schedule
        );

        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, config_content).unwrap();

        let result = load_config(&config_path);
        assert!(result.is_ok(), "Schedule '{}' should be valid: {:?}", schedule, result.err());
    }
}
