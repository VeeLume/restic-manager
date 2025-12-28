use super::types::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse config file: {0}")]
    ParseError(#[from] toml::de::Error),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Profile '{0}' not found")]
    ProfileNotFound(String),

    #[error("Destination '{0}' not found")]
    DestinationNotFound(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

/// Load and validate configuration from a TOML file
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config> {
    let contents = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    validate_config(&config)?;
    Ok(config)
}

/// Validate the configuration
fn validate_config(config: &Config) -> Result<()> {
    // Validate global settings
    if !config.global.restic_password_file.exists() {
        return Err(ConfigError::ValidationError(format!(
            "Restic password file does not exist: {:?}",
            config.global.restic_password_file
        )));
    }

    if !config.global.docker_base.exists() {
        return Err(ConfigError::ValidationError(format!(
            "Docker base directory does not exist: {:?}",
            config.global.docker_base
        )));
    }

    // Validate destinations exist
    if config.destinations.is_empty() {
        return Err(ConfigError::ValidationError(
            "No destinations defined".to_string(),
        ));
    }

    // Validate services
    for (name, service) in &config.services {
        validate_service(name, service, config)?;
    }

    Ok(())
}

fn validate_service(name: &str, service: &ServiceConfig, config: &Config) -> Result<()> {
    // Check that profile exists if specified
    if let Some(ref profile_name) = service.profile {
        if !config.profiles.contains_key(profile_name) {
            return Err(ConfigError::ProfileNotFound(profile_name.clone()));
        }
    }

    // Validate that targets exist (either from service or will be inherited from profile)
    let targets = get_effective_targets(service, config);
    for target in targets {
        if !config.destinations.contains_key(&target) {
            return Err(ConfigError::DestinationNotFound(target));
        }
    }

    // Validate cron schedule format (basic check)
    if service.schedule.split_whitespace().count() != 5 {
        return Err(ConfigError::ValidationError(format!(
            "Service '{}': invalid cron schedule format (expected 5 fields): {}",
            name, service.schedule
        )));
    }

    Ok(())
}

/// Get effective targets for a service (considering profile inheritance)
fn get_effective_targets(service: &ServiceConfig, config: &Config) -> Vec<String> {
    if !service.targets.is_empty() {
        return service.targets.clone();
    }

    if let Some(ref profile_name) = service.profile {
        if let Some(profile) = config.profiles.get(profile_name) {
            return profile.targets.clone();
        }
    }

    Vec::new()
}

/// Resolve a service configuration by merging with profile and global defaults
pub fn resolve_service(
    name: &str,
    service: &ServiceConfig,
    config: &Config,
) -> Result<ResolvedServiceConfig> {
    // Get profile if specified
    let profile = service
        .profile
        .as_ref()
        .and_then(|p| config.profiles.get(p));

    // Resolve targets (service > profile > error)
    let targets = if !service.targets.is_empty() {
        service.targets.clone()
    } else if let Some(p) = profile {
        p.targets.clone()
    } else {
        return Err(ConfigError::ValidationError(format!(
            "Service '{}' has no targets defined and no profile",
            name
        )));
    };

    // Resolve timeout (service > profile > global)
    let timeout_seconds = service
        .timeout_seconds
        .or_else(|| profile.and_then(|p| p.timeout_seconds))
        .unwrap_or(config.global.default_timeout_seconds);

    // Resolve retention (service > profile > global)
    let retention = RetentionPolicy {
        daily: service
            .retention_daily
            .or_else(|| profile.and_then(|p| p.retention_daily))
            .unwrap_or(config.global.retention_daily),
        weekly: service
            .retention_weekly
            .or_else(|| profile.and_then(|p| p.retention_weekly))
            .unwrap_or(config.global.retention_weekly),
        monthly: service
            .retention_monthly
            .or_else(|| profile.and_then(|p| p.retention_monthly))
            .unwrap_or(config.global.retention_monthly),
        yearly: service
            .retention_yearly
            .or_else(|| profile.and_then(|p| p.retention_yearly))
            .unwrap_or(config.global.retention_yearly),
    };

    // Resolve notify_on (service > profile > global)
    let notify_on = if !service.notify_on.is_empty() {
        service.notify_on.clone()
    } else if let Some(p) = profile {
        if !p.notify_on.is_empty() {
            p.notify_on.clone()
        } else {
            config.notifications.notify_on.clone()
        }
    } else {
        config.notifications.notify_on.clone()
    };

    Ok(ResolvedServiceConfig {
        name: name.to_string(),
        enabled: service.enabled,
        description: service.description.clone(),
        schedule: service.schedule.clone(),
        targets,
        timeout_seconds,
        retention,
        notify_on,
        config: service.config.clone(),
    })
}

/// Resolve all services in the configuration
pub fn resolve_all_services(config: &Config) -> Result<HashMap<String, ResolvedServiceConfig>> {
    let mut resolved = HashMap::new();

    for (name, service) in &config.services {
        let resolved_service = resolve_service(name, service, config)?;
        resolved.insert(name.clone(), resolved_service);
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_profile_inheritance() {
        // This would test that profile inheritance works correctly
        // We'll implement tests after setting up the basic structure
    }

    #[test]
    fn test_config_validation() {
        // Test validation logic
    }
}
