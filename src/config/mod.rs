//! Configuration module for restic-manager
//!
//! This module handles loading, validating, and resolving configuration from TOML files.
//!
//! ## Configuration Inheritance
//!
//! Settings are applied in this order (later overrides earlier):
//! 1. Global defaults
//! 2. Profile settings (if profile is specified)
//! 3. Service-level settings
//!
//! ## Example Usage
//!
//! ```no_run
//! use restic_manager::config;
//!
//! let config = config::load_config("backup-config.toml")?;
//! let resolved_services = config::resolve_all_services(&config)?;
//!
//! for (name, service) in resolved_services {
//!     println!("Service: {}, Targets: {:?}", name, service.targets);
//! }
//! ```

mod loader;
mod types;

pub use loader::{load_config, resolve_all_services, resolve_service, ConfigError, Result};
pub use types::*;

/// Get the merged exclude patterns for a service
/// This combines global default_excludes with service-specific excludes
pub fn get_effective_excludes(service: &ResolvedServiceConfig, global: &GlobalConfig) -> Vec<String> {
    let mut excludes = global.default_excludes.clone();

    if let Some(ref config) = service.config {
        excludes.extend(config.excludes.clone());
    }

    excludes
}

/// Expand tilde (~) in path
pub fn expand_tilde(path: &std::path::Path) -> std::path::PathBuf {
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_effective_excludes() {
        // Create a mock global config with default excludes
        let global = GlobalConfig {
            restic_password_file: PathBuf::from("/tmp/password"),
            docker_base: PathBuf::from("/docker"),
            log_directory: PathBuf::from("/logs"),
            log_level: "info".to_string(),
            log_max_files: 10,
            log_max_size_mb: 10,
            retention_daily: 7,
            retention_weekly: 4,
            retention_monthly: 6,
            retention_yearly: 1,
            default_timeout_seconds: 3600,
            long_running_threshold_minutes: 120,
            default_excludes: vec!["*.log".to_string(), "*.tmp".to_string()],
            use_system_restic: false,
        };

        // Create a resolved service with additional excludes
        let service = ResolvedServiceConfig {
            name: "test".to_string(),
            description: "Test service".to_string(),
            enabled: true,
            schedule: "0 2 * * *".to_string(),
            targets: vec!["local".to_string()],
            strategy: BackupStrategy::Generic,
            timeout_seconds: 3600,
            retention: RetentionPolicy {
                daily: 7,
                weekly: 4,
                monthly: 6,
                yearly: 1,
            },
            notify_on: vec![],
            config: Some(ServiceStrategyConfig {
                paths: vec![],
                volumes: vec![],
                pre_backup_hooks: vec![],
                post_backup_hooks: vec![],
                excludes: vec!["*.cache".to_string()],
                mariadb_container: None,
                mariadb_database: None,
                mariadb_user: None,
                postgres_container: None,
                postgres_database: None,
                postgres_user: None,
                library_path: None,
                database_repo_suffix: None,
                library_repo_suffix: None,
            }),
        };

        // Get effective excludes
        let excludes = get_effective_excludes(&service, &global);

        // Should contain both global and service excludes
        assert_eq!(excludes.len(), 3);
        assert!(excludes.contains(&"*.log".to_string()));
        assert!(excludes.contains(&"*.tmp".to_string()));
        assert!(excludes.contains(&"*.cache".to_string()));
    }

    #[test]
    fn test_expand_tilde() {
        // Test tilde expansion
        let path = PathBuf::from("~/test");
        let expanded = expand_tilde(&path);
        assert!(!expanded.starts_with("~"));

        // Test non-tilde path (should be unchanged)
        let path = PathBuf::from("/absolute/path");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, path);
    }
}
