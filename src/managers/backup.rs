//! Backup manager - orchestrates backup execution

use crate::config::{Config, Destination, Hook, ResolvedServiceConfig};
use crate::managers::notification::NotificationManager;
use crate::utils::locker::BackupLock;
use crate::utils::{docker, restic};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

pub struct BackupManager {
    config: Config,
    resolved_services: HashMap<String, ResolvedServiceConfig>,
    notification_manager: Option<NotificationManager>,
}

impl BackupManager {
    /// Create new backup manager
    pub fn new(
        config: Config,
        resolved_services: HashMap<String, ResolvedServiceConfig>,
    ) -> Self {
        // Create notification manager if webhook URL is configured
        let notification_manager = if !config.notifications.discord_webhook_url.is_empty() {
            Some(NotificationManager::new(config.notifications.clone()))
        } else {
            None
        };

        Self {
            config,
            resolved_services,
            notification_manager,
        }
    }

    /// Create backup manager with a specific notification manager
    #[allow(dead_code)]
    pub fn with_notification_manager(
        config: Config,
        resolved_services: HashMap<String, ResolvedServiceConfig>,
        notification_manager: NotificationManager,
    ) -> Self {
        Self {
            config,
            resolved_services,
            notification_manager: Some(notification_manager),
        }
    }

    /// Send a notification (if manager is configured)
    fn notify_failure(&self, service: &str, destination: Option<&str>, error: &str, duration_secs: u64) {
        if let Some(ref manager) = self.notification_manager {
            if let Err(e) = manager.send_failure(service, destination, error, Some(duration_secs)) {
                warn!("Failed to send failure notification: {}", e);
            }
        }
    }

    /// Send a success notification (if manager is configured)
    fn notify_success(&self, service: &str, destination: Option<&str>, duration_secs: u64) {
        if let Some(ref manager) = self.notification_manager {
            if let Err(e) = manager.send_success(service, destination, duration_secs) {
                warn!("Failed to send success notification: {}", e);
            }
        }
    }

    /// Send a long-running notification (if manager is configured)
    fn notify_long_running(&self, service: &str, destination: Option<&str>, duration_secs: u64) {
        if let Some(ref manager) = self.notification_manager {
            let threshold = self.config.global.long_running_threshold_minutes;
            if let Err(e) = manager.send_long_running(service, destination, duration_secs, threshold) {
                warn!("Failed to send long-running notification: {}", e);
            }
        }
    }

    /// Run backup for a specific service
    pub fn backup_service(&self, service_name: &str) -> Result<()> {
        let service = self
            .resolved_services
            .get(service_name)
            .context(format!("Service not found: {}", service_name))?;

        if !service.enabled {
            info!("Service '{}' is disabled, skipping", service_name);
            return Ok(());
        }

        // Acquire lock to prevent concurrent backups
        let _lock = BackupLock::acquire(service_name)
            .context(format!("Failed to acquire lock for service '{}'", service_name))?;

        let start_time = Instant::now();
        let long_running_threshold_secs = self.config.global.long_running_threshold_minutes * 60;
        let mut long_running_notified = false;

        info!("Starting backup for service: {}", service_name);

        // Backup to each target
        let mut errors = Vec::new();
        let mut success_count = 0;

        for target_name in &service.targets {
            let destination = self
                .config
                .destinations
                .get(target_name)
                .context(format!("Destination not found: {}", target_name))?;

            info!(
                "Backing up '{}' to destination: {} ({})",
                service_name, target_name, destination.description
            );

            // Check for long-running and notify once
            let elapsed = start_time.elapsed().as_secs();
            if !long_running_notified && elapsed > long_running_threshold_secs {
                self.notify_long_running(service_name, Some(target_name), elapsed);
                long_running_notified = true;
            }

            match self.backup_to_destination(service, destination) {
                Ok(_) => {
                    info!(
                        "Successfully backed up '{}' to '{}'",
                        service_name, target_name
                    );
                    success_count += 1;
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    error!(
                        "Failed to backup '{}' to '{}': {}",
                        service_name, target_name, error_msg
                    );
                    errors.push(format!("{}: {}", target_name, e));

                    // Send failure notification for this destination
                    self.notify_failure(
                        service_name,
                        Some(target_name),
                        &error_msg,
                        start_time.elapsed().as_secs(),
                    );

                    // Try to unlock repository on failure
                    let repo_url = restic::build_repository_url(destination, service_name, None);
                    let env = restic::ResticEnv::new(&self.config.global.restic_password_file, &repo_url);
                    if let Err(unlock_err) = restic::unlock_repository(&env, Duration::from_secs(30)) {
                        warn!("Failed to unlock repository after error: {}", unlock_err);
                    }
                }
            }
        }

        let duration = start_time.elapsed();
        let duration_secs = duration.as_secs();

        info!(
            "Backup for service '{}' completed in {:.2}s",
            service_name,
            duration.as_secs_f64()
        );

        // Send success notification if all destinations succeeded
        if errors.is_empty() && success_count > 0 {
            self.notify_success(service_name, None, duration_secs);
        }

        if !errors.is_empty() {
            anyhow::bail!(
                "Backup failed for {} destination(s): {}",
                errors.len(),
                errors.join(", ")
            );
        }

        Ok(())
    }

    /// Perform backup to a specific destination
    fn backup_to_destination(
        &self,
        service: &ResolvedServiceConfig,
        destination: &Destination,
    ) -> Result<()> {
        info!(
            "Starting backup for service '{}' to '{}'",
            service.name, destination.url
        );

        // Run pre-backup hooks
        self.run_pre_hooks(service)
            .context("Pre-backup hooks failed")?;

        // Create temporary directory for volume archives
        let temp_dir = std::env::temp_dir()
            .join("restic-manager")
            .join(&service.name);
        fs::create_dir_all(&temp_dir)
            .context("Failed to create temporary directory")?;

        // Backup Docker volumes to temp directory
        let volume_archives = self.backup_volumes(service, &temp_dir)
            .context("Failed to backup Docker volumes")?;

        // Collect file paths
        let mut paths_to_backup = self.collect_paths(service)?;

        // Add volume archives to backup
        paths_to_backup.extend(volume_archives);

        if paths_to_backup.is_empty() {
            warn!("No paths to backup for service '{}'", service.name);
            return Ok(());
        }

        // Setup restic environment
        let repo_url = restic::build_repository_url(destination, &service.name, None);
        let env = restic::ResticEnv::new(&self.config.global.restic_password_file, &repo_url);

        let timeout = Duration::from_secs(service.timeout_seconds);

        // Initialize repository if needed
        restic::init_repository(&env, timeout)
            .context("Failed to initialize repository")?;

        // Get excludes
        let excludes = crate::config::get_effective_excludes(service, &self.config.global);

        // Perform backup
        restic::backup(&env, &paths_to_backup, &excludes, timeout)
            .context("Failed to backup to restic")?;

        // Apply retention policy
        restic::apply_retention(&env, &service.retention, timeout)
            .context("Failed to apply retention policy")?;

        // Cleanup temporary directory
        if let Err(e) = fs::remove_dir_all(&temp_dir) {
            warn!("Failed to cleanup temporary directory: {}", e);
        }

        // Run post-backup hooks
        self.run_post_hooks(service)
            .context("Post-backup hooks failed")?;

        info!(
            "Successfully completed backup for service '{}' to '{}'",
            service.name, destination.url
        );

        Ok(())
    }

    /// Run pre-backup hooks
    fn run_pre_hooks(&self, service: &ResolvedServiceConfig) -> Result<()> {
        let empty_hooks = vec![];
        let hooks = service
            .config
            .as_ref()
            .map(|c| &c.pre_backup_hooks)
            .unwrap_or(&empty_hooks);

        if hooks.is_empty() {
            return Ok(());
        }

        info!("Running {} pre-backup hooks", hooks.len());

        for hook in hooks {
            self.run_hook(hook, service, "pre-backup")?;
        }

        Ok(())
    }

    /// Run post-backup hooks
    fn run_post_hooks(&self, service: &ResolvedServiceConfig) -> Result<()> {
        let empty_hooks = vec![];
        let hooks = service
            .config
            .as_ref()
            .map(|c| &c.post_backup_hooks)
            .unwrap_or(&empty_hooks);

        if hooks.is_empty() {
            return Ok(());
        }

        info!("Running {} post-backup hooks", hooks.len());

        for hook in hooks {
            self.run_hook(hook, service, "post-backup")?;
        }

        Ok(())
    }

    /// Execute a single hook
    fn run_hook(&self, hook: &Hook, service: &ResolvedServiceConfig, hook_type: &str) -> Result<()> {
        let hook_name = if hook.name.is_empty() {
            &hook.command
        } else {
            &hook.name
        };

        info!("Running {} hook: {}", hook_type, hook_name);

        let timeout = hook
            .timeout_seconds
            .map(Duration::from_secs)
            .or(Some(Duration::from_secs(service.timeout_seconds)));

        let working_dir = hook.working_dir.as_deref();

        let result = crate::utils::command::run_shell_command(
            &hook.command,
            working_dir,
            timeout,
        );

        match result {
            Ok(_) => {
                info!("Hook completed successfully: {}", hook_name);
                Ok(())
            }
            Err(e) => {
                if hook.continue_on_error {
                    warn!("Hook failed but continue_on_error=true: {} - {}", hook_name, e);
                    Ok(())
                } else {
                    error!("Hook failed: {} - {}", hook_name, e);
                    Err(e).context(format!("Failed to execute hook: {}", hook_name))
                }
            }
        }
    }

    /// Backup Docker volumes
    fn backup_volumes(
        &self,
        service: &ResolvedServiceConfig,
        temp_dir: &PathBuf,
    ) -> Result<Vec<PathBuf>> {
        let empty_volumes = vec![];
        let volumes = service
            .config
            .as_ref()
            .map(|c| &c.volumes)
            .unwrap_or(&empty_volumes);

        if volumes.is_empty() {
            return Ok(vec![]);
        }

        info!("Backing up {} Docker volumes", volumes.len());

        let timeout = Duration::from_secs(service.timeout_seconds);
        let mut archived_paths = Vec::new();

        // First, verify all volumes exist
        for volume_name in volumes {
            if !docker::volume_exists(volume_name, Duration::from_secs(30))? {
                anyhow::bail!("Docker volume does not exist: {}", volume_name);
            }
        }

        // Archive each volume
        for volume_name in volumes {
            let archive_path = temp_dir.join(format!("{}.tar.gz", volume_name));
            docker::archive_volume(volume_name, &archive_path, timeout)
                .context(format!("Failed to archive volume: {}", volume_name))?;

            archived_paths.push(archive_path);
        }

        Ok(archived_paths)
    }

    /// Collect file paths to backup
    fn collect_paths(&self, service: &ResolvedServiceConfig) -> Result<Vec<PathBuf>> {
        let empty_paths = vec![];
        let paths = service
            .config
            .as_ref()
            .map(|c| &c.paths)
            .unwrap_or(&empty_paths);

        let mut full_paths = Vec::new();

        for path in paths {
            let full_path = if PathBuf::from(path).is_absolute() {
                PathBuf::from(path)
            } else {
                self.config.global.docker_base.join(path)
            };

            if !full_path.exists() {
                warn!("Path does not exist: {:?}", full_path);
                continue;
            }

            full_paths.push(full_path);
        }

        Ok(full_paths)
    }

    /// Run backups for all enabled services
    pub fn backup_all(&self) -> Result<()> {
        info!("Starting backup for all enabled services");

        let enabled_services: Vec<_> = self
            .resolved_services
            .iter()
            .filter(|(_, service)| service.enabled)
            .collect();

        if enabled_services.is_empty() {
            warn!("No enabled services to backup");
            return Ok(());
        }

        info!("Found {} enabled services", enabled_services.len());

        let mut success_count = 0;
        let mut failure_count = 0;
        let mut errors = Vec::new();

        for (name, _) in enabled_services {
            match self.backup_service(name) {
                Ok(_) => {
                    success_count += 1;
                }
                Err(e) => {
                    failure_count += 1;
                    errors.push(format!("{}: {}", name, e));
                    error!("Failed to backup service '{}': {}", name, e);
                }
            }
        }

        info!(
            "Backup summary: {} succeeded, {} failed",
            success_count, failure_count
        );

        if failure_count > 0 {
            anyhow::bail!(
                "{} service(s) failed to backup:\n{}",
                failure_count,
                errors.join("\n")
            );
        }

        Ok(())
    }

    /// Get list of all service names
    #[allow(dead_code)]
    pub fn list_services(&self) -> Vec<String> {
        self.resolved_services.keys().cloned().collect()
    }

    /// Get service configuration
    #[allow(dead_code)]
    pub fn get_service(&self, name: &str) -> Option<&ResolvedServiceConfig> {
        self.resolved_services.get(name)
    }
}
