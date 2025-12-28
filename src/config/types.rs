use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Root configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub global: GlobalConfig,
    pub destinations: HashMap<String, Destination>,
    #[serde(default)]
    pub notifications: NotificationConfig,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
    pub services: HashMap<String, ServiceConfig>,
}

/// Global configuration settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    /// Path to restic password file
    pub restic_password_file: PathBuf,

    /// Base directory for Docker services
    pub docker_base: PathBuf,

    /// Default retention policy
    #[serde(default = "default_retention_daily")]
    pub retention_daily: u32,
    #[serde(default = "default_retention_weekly")]
    pub retention_weekly: u32,
    #[serde(default = "default_retention_monthly")]
    pub retention_monthly: u32,
    #[serde(default)]
    pub retention_yearly: u32,

    /// Timeout settings
    #[serde(default = "default_timeout")]
    pub default_timeout_seconds: u64,
    #[serde(default = "default_long_running_threshold")]
    pub long_running_threshold_minutes: u64,

    /// Logging configuration
    #[serde(default = "default_log_directory")]
    pub log_directory: PathBuf,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_log_max_files")]
    pub log_max_files: u32,
    #[serde(default = "default_log_max_size_mb")]
    pub log_max_size_mb: u64,

    /// Default exclusion patterns
    #[serde(default)]
    pub default_excludes: Vec<String>,

    /// Use system restic from PATH instead of managed binary
    #[serde(default)]
    pub use_system_restic: bool,
}

/// Backup destination configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Destination {
    #[serde(rename = "type")]
    pub dest_type: DestinationType,
    pub url: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DestinationType {
    Sftp,
    Local,
    S3,
    B2,
}

/// Notification configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NotificationConfig {
    #[serde(default)]
    pub discord_webhook_url: String,

    #[serde(default = "default_notify_on")]
    pub notify_on: Vec<NotifyEvent>,

    #[serde(default = "default_rate_limit")]
    pub rate_limit_minutes: u64,

    #[serde(default = "default_cache_file")]
    pub cache_file: PathBuf,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            discord_webhook_url: String::new(),
            notify_on: default_notify_on(),
            rate_limit_minutes: default_rate_limit(),
            cache_file: default_cache_file(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NotifyEvent {
    Failure,
    Warning,
    LongRunning,
    Success,
}

/// Profile for grouping common service settings
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Profile {
    #[serde(default)]
    pub targets: Vec<String>,

    #[serde(default)]
    pub retention_daily: Option<u32>,
    #[serde(default)]
    pub retention_weekly: Option<u32>,
    #[serde(default)]
    pub retention_monthly: Option<u32>,
    #[serde(default)]
    pub retention_yearly: Option<u32>,

    #[serde(default)]
    pub timeout_seconds: Option<u64>,

    #[serde(default)]
    pub notify_on: Vec<NotifyEvent>,
}

/// Service configuration (raw, before profile merging)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServiceConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Profile to inherit from (optional)
    #[serde(default)]
    pub profile: Option<String>,

    #[serde(default)]
    pub description: String,

    /// Cron schedule
    pub schedule: String,

    /// Backup targets (destination names)
    #[serde(default)]
    pub targets: Vec<String>,

    /// Timeout override
    #[serde(default)]
    pub timeout_seconds: Option<u64>,

    /// Retention overrides
    #[serde(default)]
    pub retention_daily: Option<u32>,
    #[serde(default)]
    pub retention_weekly: Option<u32>,
    #[serde(default)]
    pub retention_monthly: Option<u32>,
    #[serde(default)]
    pub retention_yearly: Option<u32>,

    /// Notification overrides
    #[serde(default)]
    pub notify_on: Vec<NotifyEvent>,

    /// Backup configuration (paths, volumes, hooks)
    #[serde(default)]
    pub config: Option<BackupConfig>,
}

/// Resolved service configuration (after profile merging)
#[derive(Debug, Clone)]
pub struct ResolvedServiceConfig {
    pub name: String,
    pub enabled: bool,
    pub description: String,
    pub schedule: String,
    pub targets: Vec<String>,
    pub timeout_seconds: u64,
    pub retention: RetentionPolicy,
    #[allow(dead_code)]
    pub notify_on: Vec<NotifyEvent>,
    pub config: Option<BackupConfig>,
}

#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub daily: u32,
    pub weekly: u32,
    pub monthly: u32,
    pub yearly: u32,
}

/// Hook to run before or after backup
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Hook {
    /// Name/description of the hook
    #[serde(default)]
    pub name: String,

    /// Command to execute
    pub command: String,

    /// Optional working directory
    #[serde(default)]
    pub working_dir: Option<PathBuf>,

    /// Timeout in seconds (optional)
    #[serde(default)]
    pub timeout_seconds: Option<u64>,

    /// Whether to continue on failure
    #[serde(default = "default_continue_on_error")]
    pub continue_on_error: bool,
}

fn default_continue_on_error() -> bool {
    false
}

/// Backup configuration (paths, volumes, hooks)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackupConfig {
    /// File/directory paths to backup (relative to docker_base or absolute)
    #[serde(default)]
    pub paths: Vec<String>,

    /// Docker volumes to backup
    #[serde(default)]
    pub volumes: Vec<String>,

    /// Exclusion patterns
    #[serde(default)]
    pub excludes: Vec<String>,

    /// Hooks to run before backup
    #[serde(default)]
    pub pre_backup_hooks: Vec<Hook>,

    /// Hooks to run after backup
    #[serde(default)]
    pub post_backup_hooks: Vec<Hook>,
}

// Default value functions

fn default_retention_daily() -> u32 { 7 }
fn default_retention_weekly() -> u32 { 4 }
fn default_retention_monthly() -> u32 { 6 }
fn default_timeout() -> u64 { 3600 }
fn default_long_running_threshold() -> u64 { 120 }
fn default_log_directory() -> PathBuf { PathBuf::from("~/logs") }
fn default_log_level() -> String { "info".to_string() }
fn default_log_max_files() -> u32 { 10 }
fn default_log_max_size_mb() -> u64 { 10 }
fn default_enabled() -> bool { true }
fn default_notify_on() -> Vec<NotifyEvent> {
    vec![NotifyEvent::Failure, NotifyEvent::Warning]
}
fn default_rate_limit() -> u64 { 60 }
fn default_cache_file() -> PathBuf {
    PathBuf::from("~/.cache/restic-manager-notifications.json")
}
