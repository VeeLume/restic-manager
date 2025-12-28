//! Fluent API for building test configurations
//!
//! Provides a builder pattern for creating test configurations with sensible defaults.

use restic_manager::config::{
    Config, Destination, DestinationType, GlobalConfig,
    NotificationConfig, Profile, RetentionPolicy, ServiceConfig, BackupConfig,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Builder for creating test configurations
pub struct ConfigBuilder {
    temp_dir: TempDir,
    global: GlobalConfig,
    destinations: HashMap<String, Destination>,
    services: HashMap<String, ServiceConfig>,
    profiles: HashMap<String, Profile>,
    notifications: NotificationConfig,
}

impl ConfigBuilder {
    /// Create a new ConfigBuilder with minimal defaults
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create default password file
        let password_file = temp_dir.path().join("restic-password");
        fs::write(&password_file, "test-password-123").expect("Failed to write password file");

        // Create docker_base directory
        let docker_base = temp_dir.path().join("docker");
        fs::create_dir_all(&docker_base).expect("Failed to create docker_base");

        // Create log directory
        let log_directory = temp_dir.path().join("logs");
        fs::create_dir_all(&log_directory).expect("Failed to create log_directory");

        let global = GlobalConfig {
            restic_password_file: password_file,
            docker_base,
            retention_daily: 7,
            retention_weekly: 4,
            retention_monthly: 6,
            retention_yearly: 1,
            default_timeout_seconds: 300,
            long_running_threshold_minutes: 30,
            log_directory,
            log_level: "info".to_string(),
            log_max_files: 5,
            log_max_size_mb: 10,
            default_excludes: vec![],
            use_system_restic: false,
        };

        Self {
            temp_dir,
            global,
            destinations: HashMap::new(),
            services: HashMap::new(),
            profiles: HashMap::new(),
            notifications: NotificationConfig::default(),
        }
    }

    /// Create a minimal config with a local destination and one service
    pub fn minimal() -> Self {
        let mut builder = Self::new();

        // Add a local destination
        let backup_path = builder.temp_dir.path().join("backups");
        fs::create_dir_all(&backup_path).expect("Failed to create backup dir");

        builder.destinations.insert(
            "local".to_string(),
            Destination {
                dest_type: DestinationType::Local,
                url: backup_path.display().to_string(),
                description: "Local test destination".to_string(),
            },
        );

        builder
    }

    /// Set the password file path
    pub fn with_password_file(mut self, path: &Path) -> Self {
        self.global.restic_password_file = path.to_path_buf();
        self
    }

    /// Set the docker base directory
    pub fn with_docker_base(mut self, path: &Path) -> Self {
        self.global.docker_base = path.to_path_buf();
        self
    }

    /// Set the log directory
    pub fn with_log_dir(mut self, path: &Path) -> Self {
        self.global.log_directory = path.to_path_buf();
        self
    }

    /// Set the default timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.global.default_timeout_seconds = seconds;
        self
    }

    /// Set the retention policy
    pub fn with_retention(mut self, retention: RetentionPolicy) -> Self {
        self.global.retention_daily = retention.daily;
        self.global.retention_weekly = retention.weekly;
        self.global.retention_monthly = retention.monthly;
        self.global.retention_yearly = retention.yearly;
        self
    }

    /// Add a local destination
    pub fn add_local_destination(mut self, name: &str, path: &Path) -> Self {
        self.destinations.insert(
            name.to_string(),
            Destination {
                dest_type: DestinationType::Local,
                url: path.display().to_string(),
                description: format!("Local destination: {}", name),
            },
        );
        self
    }

    /// Add an SFTP destination
    pub fn add_sftp_destination(mut self, name: &str, url: &str) -> Self {
        self.destinations.insert(
            name.to_string(),
            Destination {
                dest_type: DestinationType::Sftp,
                url: url.to_string(),
                description: format!("SFTP destination: {}", name),
            },
        );
        self
    }

    /// Add a destination with custom settings
    pub fn add_destination(mut self, name: &str, dest: Destination) -> Self {
        self.destinations.insert(name.to_string(), dest);
        self
    }

    /// Add a simple service
    pub fn add_service(mut self, name: &str) -> Self {
        self.services.insert(
            name.to_string(),
            ServiceConfig {
                enabled: true,
                profile: None,
                description: format!("Test service: {}", name),
                schedule: "0 2 * * *".to_string(),
                targets: vec!["local".to_string()],
                timeout_seconds: None,
                retention_daily: None,
                retention_weekly: None,
                retention_monthly: None,
                retention_yearly: None,
                notify_on: vec![],
                config: None,
            },
        );
        self
    }

    /// Add a service with full configuration
    pub fn add_service_config(mut self, name: &str, config: ServiceConfig) -> Self {
        self.services.insert(name.to_string(), config);
        self
    }

    /// Add a disabled service
    pub fn add_disabled_service(mut self, name: &str) -> Self {
        self.services.insert(
            name.to_string(),
            ServiceConfig {
                enabled: false,
                profile: None,
                description: format!("Disabled service: {}", name),
                schedule: "0 2 * * *".to_string(),
                targets: vec!["local".to_string()],
                timeout_seconds: None,
                retention_daily: None,
                retention_weekly: None,
                retention_monthly: None,
                retention_yearly: None,
                notify_on: vec![],
                config: None,
            },
        );
        self
    }

    /// Add a service with paths to backup
    pub fn add_service_with_paths(mut self, name: &str, paths: Vec<String>) -> Self {
        self.services.insert(
            name.to_string(),
            ServiceConfig {
                enabled: true,
                profile: None,
                description: format!("Service with paths: {}", name),
                schedule: "0 2 * * *".to_string(),
                targets: vec!["local".to_string()],
                timeout_seconds: None,
                retention_daily: None,
                retention_weekly: None,
                retention_monthly: None,
                retention_yearly: None,
                notify_on: vec![],
                config: Some(BackupConfig {
                    paths,
                    volumes: vec![],
                    excludes: vec![],
                    pre_backup_hooks: vec![],
                    post_backup_hooks: vec![],
                }),
            },
        );
        self
    }

    /// Add a service with volumes to backup
    pub fn add_service_with_volumes(mut self, name: &str, volumes: Vec<String>) -> Self {
        self.services.insert(
            name.to_string(),
            ServiceConfig {
                enabled: true,
                profile: None,
                description: format!("Service with volumes: {}", name),
                schedule: "0 2 * * *".to_string(),
                targets: vec!["local".to_string()],
                timeout_seconds: None,
                retention_daily: None,
                retention_weekly: None,
                retention_monthly: None,
                retention_yearly: None,
                notify_on: vec![],
                config: Some(BackupConfig {
                    paths: vec![],
                    volumes,
                    excludes: vec![],
                    pre_backup_hooks: vec![],
                    post_backup_hooks: vec![],
                }),
            },
        );
        self
    }

    /// Add a profile
    pub fn add_profile(mut self, name: &str, profile: Profile) -> Self {
        self.profiles.insert(name.to_string(), profile);
        self
    }

    /// Set notification configuration
    pub fn with_notifications(mut self, config: NotificationConfig) -> Self {
        self.notifications = config;
        self
    }

    /// Get the temp directory path
    pub fn temp_dir(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Get the password file path
    pub fn password_file(&self) -> &Path {
        &self.global.restic_password_file
    }

    /// Get a destination backup path
    pub fn destination_path(&self, name: &str) -> Option<PathBuf> {
        self.destinations.get(name).map(|d| PathBuf::from(&d.url))
    }

    /// Build the Config
    pub fn build(self) -> Config {
        Config {
            global: self.global,
            destinations: self.destinations,
            services: self.services,
            profiles: self.profiles,
            notifications: self.notifications,
        }
    }

    /// Keep the temp directory (don't delete on drop)
    pub fn persist(self) -> (Config, TempDir) {
        let config = Config {
            global: self.global,
            destinations: self.destinations,
            services: self.services,
            profiles: self.profiles,
            notifications: self.notifications,
        };
        (config, self.temp_dir)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_config() {
        let config = ConfigBuilder::minimal().add_service("test").build();

        assert!(config.destinations.contains_key("local"));
        assert!(config.services.contains_key("test"));
        assert!(config.services.get("test").unwrap().enabled);
    }

    #[test]
    fn test_add_multiple_services() {
        let config = ConfigBuilder::minimal()
            .add_service("service1")
            .add_service("service2")
            .add_disabled_service("service3")
            .build();

        assert_eq!(config.services.len(), 3);
        assert!(config.services.get("service1").unwrap().enabled);
        assert!(config.services.get("service2").unwrap().enabled);
        assert!(!config.services.get("service3").unwrap().enabled);
    }

    #[test]
    fn test_service_with_paths() {
        let config = ConfigBuilder::minimal()
            .add_service_with_paths("test", vec!["data".to_string(), "config".to_string()])
            .build();

        let service = config.services.get("test").unwrap();
        let backup_config = service.config.as_ref().unwrap();
        assert_eq!(backup_config.paths.len(), 2);
    }

    #[test]
    fn test_service_with_volumes() {
        let config = ConfigBuilder::minimal()
            .add_service_with_volumes("test", vec!["app_data".to_string()])
            .build();

        let service = config.services.get("test").unwrap();
        let backup_config = service.config.as_ref().unwrap();
        assert_eq!(backup_config.volumes.len(), 1);
        assert_eq!(backup_config.volumes[0], "app_data");
    }

    #[test]
    fn test_multiple_destinations() {
        let builder = ConfigBuilder::minimal();
        let temp_path = builder.temp_dir().join("backup2");
        std::fs::create_dir_all(&temp_path).unwrap();

        let config = builder
            .add_local_destination("backup2", &temp_path)
            .add_sftp_destination("remote", "sftp://user@host/backups")
            .build();

        assert_eq!(config.destinations.len(), 3); // local + backup2 + remote
    }
}
