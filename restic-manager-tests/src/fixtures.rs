//! Test fixtures and sample data
//!
//! Provides pre-built test data and templates for testing.

use restic_manager::utils::restic::Snapshot;

/// Create a sample snapshot for testing
pub fn sample_snapshot() -> Snapshot {
    Snapshot {
        id: "abc123def456789012345678901234567890abcd".to_string(),
        short_id: "abc123de".to_string(),
        time: "2025-12-28T10:30:00.000000000Z".to_string(),
        hostname: "test-host".to_string(),
        paths: vec!["/data".to_string()],
    }
}

/// Create multiple sample snapshots for testing
pub fn sample_snapshots(count: usize) -> Vec<Snapshot> {
    (0..count)
        .map(|i| Snapshot {
            id: format!("snapshot{:040}", i),
            short_id: format!("snap{:04}", i),
            time: format!("2025-12-{:02}T10:30:00.000000000Z", 28 - (i % 28)),
            hostname: "test-host".to_string(),
            paths: vec!["/data".to_string()],
        })
        .collect()
}

/// Create a snapshot with specific time (for age testing)
pub fn snapshot_with_time(time: &str) -> Snapshot {
    Snapshot {
        id: "time-specific-snapshot-id-12345678901234567890".to_string(),
        short_id: "time-spe".to_string(),
        time: time.to_string(),
        hostname: "test-host".to_string(),
        paths: vec!["/data".to_string()],
    }
}

/// Minimal valid config TOML template
pub fn minimal_config_toml() -> &'static str {
    r#"
[global]
restic_password_file = "{password_file}"
docker_base = "{docker_base}"
log_dir = "{log_dir}"

[destinations.local]
type = "local"
url = "{backup_path}"
description = "Local test destination"

[services.test]
enabled = true
description = "Test service"
schedule = "0 2 * * *"
targets = ["local"]
strategy = "generic"
"#
}

/// Config with multiple services
pub fn multi_service_config_toml() -> &'static str {
    r#"
[global]
restic_password_file = "{password_file}"
docker_base = "{docker_base}"
log_dir = "{log_dir}"

[destinations.local]
type = "local"
url = "{backup_path}"
description = "Local test destination"

[services.appwrite]
enabled = true
description = "Appwrite service"
schedule = "0 2 * * *"
targets = ["local"]
strategy = "appwrite"

[services.immich]
enabled = true
description = "Immich service"
schedule = "0 3 * * *"
targets = ["local"]
strategy = "immich"

[services.generic]
enabled = true
description = "Generic service"
schedule = "0 4 * * *"
targets = ["local"]
strategy = "generic"

[services.generic.strategy_config]
paths = ["data"]
volumes = ["app_data"]
"#
}

/// Config with profiles
pub fn config_with_profiles_toml() -> &'static str {
    r#"
[global]
restic_password_file = "{password_file}"
docker_base = "{docker_base}"
log_dir = "{log_dir}"

[destinations.local]
type = "local"
url = "{backup_path}"
description = "Local test destination"

[profiles.default]
retention = { daily = 7, weekly = 4, monthly = 6, yearly = 1 }
timeout_seconds = 3600

[profiles.production]
retention = { daily = 14, weekly = 8, monthly = 12, yearly = 2 }
timeout_seconds = 7200

[services.test]
enabled = true
description = "Test service with profile"
schedule = "0 2 * * *"
targets = ["local"]
strategy = "generic"
profile = "production"
"#
}

/// Config with multi-destination service
pub fn multi_destination_config_toml() -> &'static str {
    r#"
[global]
restic_password_file = "{password_file}"
docker_base = "{docker_base}"
log_dir = "{log_dir}"

[destinations.local]
type = "local"
url = "{backup_path}"
description = "Local backup"

[destinations.remote]
type = "sftp"
url = "sftp://user@host/backups"
description = "Remote backup"

[services.important]
enabled = true
description = "Important service backed up to multiple destinations"
schedule = "0 2 * * *"
targets = ["local", "remote"]
strategy = "generic"
"#
}

/// Config with hooks
pub fn config_with_hooks_toml() -> &'static str {
    r#"
[global]
restic_password_file = "{password_file}"
docker_base = "{docker_base}"
log_dir = "{log_dir}"

[destinations.local]
type = "local"
url = "{backup_path}"
description = "Local test destination"

[services.hooked]
enabled = true
description = "Service with hooks"
schedule = "0 2 * * *"
targets = ["local"]
strategy = "generic"

[services.hooked.strategy_config]
paths = ["data"]
pre_hooks = ["echo 'pre-backup'"]
post_hooks = ["echo 'post-backup'"]
"#
}

/// Sample volume names for Appwrite
pub fn appwrite_volumes() -> Vec<String> {
    vec![
        "appwrite_appwrite-cache".to_string(),
        "appwrite_appwrite-certificates".to_string(),
        "appwrite_appwrite-config".to_string(),
        "appwrite_appwrite-functions".to_string(),
        "appwrite_appwrite-mariadb".to_string(),
        "appwrite_appwrite-redis".to_string(),
        "appwrite_appwrite-uploads".to_string(),
    ]
}

/// Sample volume names for Immich
pub fn immich_volumes() -> Vec<String> {
    vec![
        "immich_pgdata".to_string(),
        "immich_model-cache".to_string(),
    ]
}

/// Create test data in a directory
pub fn create_test_data(dir: &std::path::Path) -> std::io::Result<()> {
    use std::fs;

    // Create some directories
    fs::create_dir_all(dir.join("data"))?;
    fs::create_dir_all(dir.join("config"))?;

    // Create some files
    fs::write(dir.join("data/file1.txt"), "Test data file 1")?;
    fs::write(dir.join("data/file2.txt"), "Test data file 2")?;
    fs::write(dir.join("config/settings.json"), r#"{"key": "value"}"#)?;

    Ok(())
}

/// Verify test data exists in a directory
pub fn verify_test_data(dir: &std::path::Path) -> bool {
    dir.join("data/file1.txt").exists()
        && dir.join("data/file2.txt").exists()
        && dir.join("config/settings.json").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_snapshot() {
        let snapshot = sample_snapshot();
        assert!(!snapshot.id.is_empty());
        assert!(!snapshot.short_id.is_empty());
        assert!(!snapshot.time.is_empty());
    }

    #[test]
    fn test_sample_snapshots() {
        let snapshots = sample_snapshots(5);
        assert_eq!(snapshots.len(), 5);

        // Check they're all unique
        let ids: Vec<_> = snapshots.iter().map(|s| &s.id).collect();
        for (i, id) in ids.iter().enumerate() {
            for (j, other_id) in ids.iter().enumerate() {
                if i != j {
                    assert_ne!(id, other_id);
                }
            }
        }
    }

    #[test]
    fn test_create_test_data() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        create_test_data(temp_dir.path()).unwrap();
        assert!(verify_test_data(temp_dir.path()));
    }

    #[test]
    fn test_appwrite_volumes() {
        let volumes = appwrite_volumes();
        assert!(!volumes.is_empty());
        // All should start with appwrite_appwrite-
        for v in &volumes {
            assert!(v.starts_with("appwrite_appwrite-"));
        }
    }
}
