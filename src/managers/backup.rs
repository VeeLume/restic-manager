//! Backup manager - orchestrates backup execution

use crate::config::{BackupStrategy as ConfigBackupStrategy, Config, ResolvedServiceConfig};
use crate::strategies::{generic::GenericStrategy, BackupStrategy};
use crate::utils::locker::BackupLock;
use crate::utils::restic;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

pub struct BackupManager {
    config: Config,
    resolved_services: HashMap<String, ResolvedServiceConfig>,
}

impl BackupManager {
    /// Create new backup manager
    pub fn new(
        config: Config,
        resolved_services: HashMap<String, ResolvedServiceConfig>,
    ) -> Self {
        Self {
            config,
            resolved_services,
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
        info!("Starting backup for service: {}", service_name);

        // Get strategy
        let strategy: Box<dyn BackupStrategy> = match &service.strategy {
            ConfigBackupStrategy::Generic => Box::new(GenericStrategy::new()),
            ConfigBackupStrategy::Appwrite => {
                anyhow::bail!("Appwrite strategy not yet implemented - use generic with hooks")
            }
            ConfigBackupStrategy::Immich => {
                anyhow::bail!("Immich strategy not yet implemented - use generic with hooks")
            }
            ConfigBackupStrategy::Script(script_path) => {
                anyhow::bail!(
                    "Script strategy not yet implemented: {:?}",
                    script_path
                )
            }
        };

        // Backup to each target
        let mut errors = Vec::new();

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

            match strategy.backup(service, destination, &self.config.global) {
                Ok(_) => {
                    info!(
                        "Successfully backed up '{}' to '{}'",
                        service_name, target_name
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to backup '{}' to '{}': {}",
                        service_name, target_name, e
                    );
                    errors.push(format!("{}: {}", target_name, e));

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
        info!(
            "Backup for service '{}' completed in {:.2}s",
            service_name,
            duration.as_secs_f64()
        );

        if !errors.is_empty() {
            anyhow::bail!(
                "Backup failed for {} destination(s): {}",
                errors.len(),
                errors.join(", ")
            );
        }

        Ok(())
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
    pub fn list_services(&self) -> Vec<String> {
        self.resolved_services.keys().cloned().collect()
    }

    /// Get service configuration
    pub fn get_service(&self, name: &str) -> Option<&ResolvedServiceConfig> {
        self.resolved_services.get(name)
    }
}
