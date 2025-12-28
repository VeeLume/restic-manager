pub mod restic;
pub mod docker;
pub mod locker;
pub mod command;
pub mod cron;
pub mod restic_installer;

// Trait-based abstractions for testability
pub mod executor;
pub mod restic_ops;
pub mod docker_ops;

// Re-export commonly used types and traits (used by test crate)
#[allow(unused_imports)]
pub use executor::{CommandExecutor, RealExecutor};
#[allow(unused_imports)]
pub use restic_ops::{ResticOperations, RealResticOps};
#[allow(unused_imports)]
pub use docker_ops::{DockerOperations, RealDockerOps};
