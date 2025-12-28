//! Test utilities for restic-manager
//!
//! This crate provides shared test utilities, mock implementations,
//! and helper functions for testing the restic-manager application.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use test_utils::{ConfigBuilder, TestContext, MockResticOps};
//!
//! #[test]
//! fn my_test() {
//!     let ctx = TestContext::new();
//!     let config = ConfigBuilder::minimal()
//!         .add_service("test-service")
//!         .build();
//!     // ... test code
//! }
//! ```

pub mod config_builder;
pub mod fixtures;
pub mod test_context;

// Re-export commonly used items
pub use config_builder::ConfigBuilder;
pub use fixtures::*;
pub use test_context::TestContext;

// Re-export types from the main crate for convenience
pub use restic_manager::config::{
    Config, Destination, DestinationType, GlobalConfig,
    NotificationConfig, Profile, ResolvedServiceConfig, RetentionPolicy, ServiceConfig,
    BackupConfig,
};
pub use restic_manager::utils::restic::{ResticEnv, Snapshot};

// Re-export mock implementations from the main crate
pub use restic_manager::utils::docker_ops::mock::MockDockerOps;
pub use restic_manager::utils::docker_ops::DockerOperations;
pub use restic_manager::utils::executor::mock::MockExecutor;
pub use restic_manager::utils::executor::CommandExecutor;
pub use restic_manager::utils::restic_ops::mock::MockResticOps;
pub use restic_manager::utils::restic_ops::ResticOperations;

/// Common test result type
pub type TestResult<T = ()> = anyhow::Result<T>;
