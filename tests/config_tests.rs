// Integration tests for configuration loading and validation

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_config_validation_missing_password_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Create a config with a non-existent password file
    let config_content = format!(
        r#"
[global]
restic_password_file = "{}/nonexistent.txt"
docker_base = "{}"
log_dir = "{}"

[[destinations]]
name = "local"
url = "/tmp/backup"
"#,
        temp_dir.path().display(),
        temp_dir.path().display(),
        temp_dir.path().display()
    );

    fs::write(&config_path, config_content).unwrap();

    // This should fail because password file doesn't exist
    let result = restic_manager::config::load_config(&config_path);
    assert!(result.is_err());
}

#[test]
fn test_config_validation_no_destinations() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let password_file = temp_dir.path().join("password.txt");

    // Create password file
    fs::write(&password_file, "test-password").unwrap();

    // Create a config with no destinations
    let config_content = format!(
        r#"
[global]
restic_password_file = "{}"
docker_base = "{}"
log_dir = "{}"
"#,
        password_file.display(),
        temp_dir.path().display(),
        temp_dir.path().display()
    );

    fs::write(&config_path, config_content).unwrap();

    // This should fail because no destinations are defined
    let result = restic_manager::config::load_config(&config_path);
    assert!(result.is_err());
}

#[test]
fn test_config_validation_invalid_cron() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let password_file = temp_dir.path().join("password.txt");

    // Create password file
    fs::write(&password_file, "test-password").unwrap();

    // Create a config with invalid cron schedule
    let config_content = format!(
        r#"
[global]
restic_password_file = "{}"
docker_base = "{}"
log_dir = "{}"

[[destinations]]
name = "local"
url = "/tmp/backup"

[services.test]
description = "Test service"
schedule = "invalid cron"
strategy = "Generic"
targets = ["local"]
"#,
        password_file.display(),
        temp_dir.path().display(),
        temp_dir.path().display()
    );

    fs::write(&config_path, config_content).unwrap();

    // This should fail because cron schedule is invalid
    let result = restic_manager::config::load_config(&config_path);
    assert!(result.is_err());
}

#[test]
fn test_valid_config_loads() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let password_file = temp_dir.path().join("password.txt");

    // Create password file
    fs::write(&password_file, "test-password").unwrap();

    // Create a valid config
    let config_content = format!(
        r#"
[global]
restic_password_file = "{}"
docker_base = "{}"
log_dir = "{}"

[[destinations]]
name = "local"
url = "/tmp/backup"

[services.test]
description = "Test service"
schedule = "0 2 * * *"
strategy = "Generic"
targets = ["local"]
enabled = true
"#,
        password_file.display(),
        temp_dir.path().display(),
        temp_dir.path().display()
    );

    fs::write(&config_path, config_content).unwrap();

    // This should succeed
    let config = restic_manager::config::load_config(&config_path).unwrap();
    assert_eq!(config.services.len(), 1);
    assert_eq!(config.destinations.len(), 1);
}

#[test]
fn test_profile_inheritance() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");
    let password_file = temp_dir.path().join("password.txt");

    // Create password file
    fs::write(&password_file, "test-password").unwrap();

    // Create a config with profile inheritance
    let config_content = format!(
        r#"
[global]
restic_password_file = "{}"
docker_base = "{}"
log_dir = "{}"

[[destinations]]
name = "local"
url = "/tmp/backup"

[[destinations]]
name = "remote"
url = "/tmp/remote"

[profiles.production]
targets = ["local", "remote"]
retention_daily = 14

[services.test]
description = "Test service"
schedule = "0 2 * * *"
strategy = "Generic"
profile = "production"
enabled = true
"#,
        password_file.display(),
        temp_dir.path().display(),
        temp_dir.path().display()
    );

    fs::write(&config_path, config_content).unwrap();

    // Load and resolve config
    let config = restic_manager::config::load_config(&config_path).unwrap();
    let resolved = restic_manager::config::resolve_all_services(&config).unwrap();

    // Service should inherit targets from profile
    let service = resolved.get("test").unwrap();
    assert_eq!(service.targets.len(), 2);
    assert!(service.targets.contains(&"local".to_string()));
    assert!(service.targets.contains(&"remote".to_string()));
}
