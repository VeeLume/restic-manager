//! Logging manager with file rotation
//!
//! Provides dual-output logging:
//! - Console: INFO level with concise format
//! - File: DEBUG level with rotation (daily + size-based)

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer};

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Directory for log files
    pub log_directory: PathBuf,
    /// Log level for file output (console always uses INFO)
    pub log_level: Level,
    /// Maximum number of log files to keep
    pub max_files: u32,
    /// Maximum size per log file in MB (reserved for future size-based rotation)
    #[allow(dead_code)]
    pub max_size_mb: u64,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_directory: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("logs"),
            log_level: Level::DEBUG,
            max_files: 10,
            max_size_mb: 10,
        }
    }
}

impl LoggingConfig {
    /// Create from global config values
    pub fn from_config(
        log_directory: &Path,
        log_level: &str,
        max_files: u32,
        max_size_mb: u64,
    ) -> Self {
        let level = match log_level.to_lowercase().as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" | "warning" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        };

        Self {
            log_directory: log_directory.to_path_buf(),
            log_level: level,
            max_files,
            max_size_mb,
        }
    }
}

/// Initialize logging with console and file outputs
///
/// Returns a guard that must be kept alive for the duration of the program.
/// When the guard is dropped, any remaining logs are flushed to disk.
pub fn init_logging(config: &LoggingConfig) -> Result<LogGuard> {
    // Ensure log directory exists
    let log_dir = expand_tilde(&config.log_directory);
    fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;

    // Create rolling file appender (daily rotation)
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        &log_dir,
        "restic-manager.log",
    );

    // Create non-blocking writer for file output
    let (non_blocking, file_guard) = tracing_appender::non_blocking(file_appender);

    // File layer: DEBUG level, detailed format
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false) // No colors in file
        .with_target(true)
        .with_level(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_span_events(FmtSpan::NONE)
        .with_filter(level_filter(config.log_level));

    // Console layer: INFO level, concise format
    let console_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true) // Colors on console
        .with_target(false)
        .with_level(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_span_events(FmtSpan::NONE)
        .with_filter(level_filter(Level::INFO));

    // Combine layers with base subscriber
    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .init();

    // Cleanup old log files
    cleanup_old_logs(&log_dir, config.max_files)?;

    Ok(LogGuard {
        _file_guard: file_guard,
    })
}

/// Initialize simple console-only logging (for when config isn't available)
pub fn init_console_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .init();
}

/// Create a level filter for tracing layers
fn level_filter(level: Level) -> EnvFilter {
    EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new(format!("restic_manager={}", level))
                .add_directive(format!("{}", level).parse().unwrap())
        })
}

/// Expand tilde (~) in path to home directory
fn expand_tilde(path: &Path) -> PathBuf {
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    path.to_path_buf()
}

/// Cleanup old log files, keeping only the most recent N files
fn cleanup_old_logs(log_dir: &Path, max_files: u32) -> Result<()> {
    let mut log_files: Vec<_> = fs::read_dir(log_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_name()
                .to_string_lossy()
                .starts_with("restic-manager")
                && entry.file_name()
                    .to_string_lossy()
                    .ends_with(".log")
        })
        .collect();

    // Sort by modification time (newest first)
    log_files.sort_by(|a, b| {
        let a_time = a.metadata().and_then(|m| m.modified()).ok();
        let b_time = b.metadata().and_then(|m| m.modified()).ok();
        b_time.cmp(&a_time)
    });

    // Remove files beyond the limit
    for file in log_files.into_iter().skip(max_files as usize) {
        if let Err(e) = fs::remove_file(file.path()) {
            tracing::warn!("Failed to remove old log file {:?}: {}", file.path(), e);
        } else {
            tracing::debug!("Removed old log file: {:?}", file.path());
        }
    }

    Ok(())
}

/// Guard that keeps the logging system alive
///
/// When dropped, flushes any remaining logs to disk.
pub struct LogGuard {
    _file_guard: WorkerGuard,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.log_level, Level::DEBUG);
        assert_eq!(config.max_files, 10);
        assert_eq!(config.max_size_mb, 10);
    }

    #[test]
    fn test_logging_config_from_config() {
        let config = LoggingConfig::from_config(
            Path::new("/tmp/logs"),
            "warn",
            5,
            20,
        );
        assert_eq!(config.log_level, Level::WARN);
        assert_eq!(config.max_files, 5);
        assert_eq!(config.max_size_mb, 20);
    }

    #[test]
    fn test_expand_tilde() {
        let path = Path::new("~/logs");
        let expanded = expand_tilde(path);
        assert!(!expanded.starts_with("~"));
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        let path = Path::new("/var/log");
        let expanded = expand_tilde(path);
        assert_eq!(expanded, PathBuf::from("/var/log"));
    }

    #[test]
    fn test_cleanup_old_logs() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test log files
        for i in 0..5 {
            let path = temp_dir.path().join(format!("restic-manager.{}.log", i));
            fs::write(&path, format!("log content {}", i)).unwrap();
            // Add small delay to ensure different modification times
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Keep only 3 files
        cleanup_old_logs(temp_dir.path(), 3).unwrap();

        // Count remaining files
        let remaining: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();

        assert_eq!(remaining.len(), 3);
    }
}
