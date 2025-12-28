//! Generic backup strategy
//!
//! Handles:
//! - File/directory backups
//! - Docker volume backups
//! - Pre/post backup hooks
//! - Restic repository management

use super::BackupStrategy;
use crate::config::{Destination, GlobalConfig, Hook, ResolvedServiceConfig};
use crate::utils::{docker, restic};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info, warn};

pub struct GenericStrategy;

impl GenericStrategy {
    pub fn new() -> Self {
        Self
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
    fn collect_paths(
        &self,
        service: &ResolvedServiceConfig,
        global: &GlobalConfig,
    ) -> Result<Vec<PathBuf>> {
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
                global.docker_base.join(path)
            };

            if !full_path.exists() {
                warn!("Path does not exist: {:?}", full_path);
                continue;
            }

            full_paths.push(full_path);
        }

        Ok(full_paths)
    }
}

impl BackupStrategy for GenericStrategy {
    fn backup(
        &self,
        service: &ResolvedServiceConfig,
        destination: &Destination,
        global: &GlobalConfig,
    ) -> Result<()> {
        info!(
            "Starting generic backup for service '{}' to '{}'",
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
        let mut paths_to_backup = self.collect_paths(service, global)?;

        // Add volume archives to backup
        paths_to_backup.extend(volume_archives);

        if paths_to_backup.is_empty() {
            warn!("No paths to backup for service '{}'", service.name);
            return Ok(());
        }

        // Setup restic environment
        let repo_url = restic::build_repository_url(destination, &service.name, None);
        let env = restic::ResticEnv::new(&global.restic_password_file, &repo_url);

        let timeout = Duration::from_secs(service.timeout_seconds);

        // Initialize repository if needed
        restic::init_repository(&env, timeout)
            .context("Failed to initialize repository")?;

        // Get excludes
        let excludes = crate::config::get_effective_excludes(service, global);

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

    fn name(&self) -> &'static str {
        "generic"
    }
}
