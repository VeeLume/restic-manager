//! Restic Manager Library
//!
//! This library provides backup orchestration functionality wrapping restic.

pub mod config;
pub mod managers;
pub mod utils;

// Re-export commonly used types
pub use config::{load_config, resolve_all_services, Config, ResolvedServiceConfig};
pub use managers::backup::BackupManager;
pub use managers::logging::{init_logging, init_console_logging, LoggingConfig, LogGuard};
pub use managers::notification::NotificationManager;
