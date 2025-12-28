pub mod generic;

use crate::config::{Destination, GlobalConfig, ResolvedServiceConfig};
use anyhow::Result;

/// Trait for backup strategies
pub trait BackupStrategy {
    /// Perform backup for a service to a destination
    fn backup(
        &self,
        service: &ResolvedServiceConfig,
        destination: &Destination,
        global: &GlobalConfig,
    ) -> Result<()>;

    /// Get strategy name (for logging)
    fn name(&self) -> &'static str;
}
