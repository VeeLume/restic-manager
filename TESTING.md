# Testing Guide

Platform-independent testing for restic-manager using a dedicated test crate.

## Quick Start

```bash
# Run all fast tests (no Docker required)
cargo test -p restic-manager-tests

# Run Docker integration tests (requires Docker)
cargo test -p restic-manager-tests -- --ignored

# Run everything
cargo test -p restic-manager-tests -- --include-ignored

# Also run main crate unit tests
cargo test
```

## Test Structure

Tests are organized in a dedicated workspace crate for better organization and separation:

```
restic-manager-tests/
├── Cargo.toml
├── src/lib.rs              # Test utilities as a library
│   ├── config_builder.rs   # Fluent config construction
│   ├── fixtures.rs         # Sample data (snapshots, volumes)
│   └── test_context.rs     # Test setup/teardown helpers
├── tests/
│   ├── unit/               # Unit tests
│   │   ├── config.rs       # Config loading/validation
│   │   ├── restic.rs       # Restic URL building, env vars
│   │   └── docker.rs       # Docker volume operations
│   ├── commands/           # Command tests (mocked)
│   │   ├── run.rs          # run command
│   │   ├── restore.rs      # restore command
│   │   ├── status.rs       # status command
│   │   ├── list.rs         # list command
│   │   ├── snapshots.rs    # snapshots command
│   │   ├── verify.rs       # verify command
│   │   ├── setup.rs        # setup command
│   │   ├── validate.rs     # validate command
│   │   └── restic_binary.rs # setup-restic, update-restic, restic-version
│   └── integration/        # Docker tests (ignored by default)
│       ├── postgres.rs     # PostgreSQL backup tests
│       └── docker_volumes.rs # Volume operations
└── README.md
```

### Fast Tests (Always Run)

No external dependencies required. Run on every platform (Windows/Linux/macOS).

```bash
# All fast tests
cargo test -p restic-manager-tests

# Specific test suites
cargo test -p restic-manager-tests --test unit      # Unit tests
cargo test -p restic-manager-tests --test commands  # Command tests
```

**What's tested:**
- Configuration loading and validation
- Service resolution
- Mock restic/docker operations
- All 11 CLI commands (run, restore, status, list, snapshots, verify, setup, validate, setup-restic, update-restic, restic-version)
- URL building
- Retention policies

**Runtime:** ~5-10 seconds

### Docker Tests (Optional)

Require Docker to be running. Marked with `#[ignore]` attribute.

```bash
# All Docker tests
cargo test -p restic-manager-tests -- --ignored

# Specific Docker test suites
cargo test -p restic-manager-tests --test integration -- --ignored
```

**What's tested:**
- PostgreSQL backup with testcontainers
- Docker volume archiving and restoration
- Volume size calculation

**Runtime:** ~2-5 minutes (containers need to start)

## Test Architecture

### Trait-Based Mocking

Production code uses traits for dependency injection:

```rust
// src/utils/restic_ops.rs
pub trait ResticOperations: Send + Sync {
    fn init_repository(&self, env: &ResticEnv, timeout: Duration) -> Result<()>;
    fn backup(&self, env: &ResticEnv, paths: &[PathBuf], ...) -> Result<()>;
    fn list_snapshots(&self, env: &ResticEnv, timeout: Duration) -> Result<Vec<Snapshot>>;
    // ... more operations
}

// Real implementation
pub struct RealResticOps;
impl ResticOperations for RealResticOps { /* delegates to restic binary */ }

// Mock implementation (in same file for convenience)
pub mod mock {
    pub struct MockResticOps { /* configurable mock behavior */ }
    impl ResticOperations for MockResticOps { /* records calls, returns configured responses */ }
}
```

### Test Utilities

The test crate provides reusable utilities:

```rust
use test_utils::{
    ConfigBuilder,           // Fluent config construction
    MockResticOps,          // Mock restic operations
    MockDockerOps,          // Mock docker operations
    ResticOperations,       // Trait for operations
    DockerOperations,       // Trait for operations
    sample_snapshot,        // Create sample snapshot
    sample_snapshots,       // Create multiple snapshots
    appwrite_volumes,       // Appwrite volume names
    TestContext,            // Setup/teardown helper
};
```

### ConfigBuilder Pattern

```rust
let config = ConfigBuilder::minimal()
    .add_service("my-service")
    .add_service_with_paths("files", vec!["data".to_string()])
    .add_service_with_volumes("docker", vec!["app_data".to_string()])
    .add_sftp_destination("remote", "sftp://host/backups")
    .with_timeout(7200)
    .build();
```

## Running Tests

### Development Workflow

```bash
# 1. Fast feedback loop (no Docker)
cargo test -p restic-manager-tests

# 2. Before commit - run Docker tests (if on Linux)
cargo test -p restic-manager-tests -- --ignored

# 3. Full validation
cargo test -p restic-manager-tests -- --include-ignored

# 4. Run main crate tests too
cargo test
```

### Specific Tests

```bash
# Run one test file
cargo test -p restic-manager-tests --test commands restore

# Run one test function
cargo test -p restic-manager-tests test_restore_with_snapshot_id

# Run with output
cargo test -p restic-manager-tests -- --nocapture

# Run serially (not parallel)
cargo test -p restic-manager-tests -- --test-threads=1
```

## Writing Tests

### Unit Test Example

```rust
// restic-manager-tests/tests/unit/config.rs

use test_utils::{ConfigBuilder, TestContext};
use restic_manager::config::{load_config, resolve_all_services};

#[test]
fn test_config_loading_valid() {
    let builder = ConfigBuilder::minimal().add_service("test-service");
    let (config, temp_dir) = builder.persist();

    let config_path = temp_dir.path().join("config.toml");
    let toml_str = toml::to_string_pretty(&config).unwrap();
    std::fs::write(&config_path, toml_str).unwrap();

    let loaded = load_config(&config_path);
    assert!(loaded.is_ok());
}
```

### Command Test Example

```rust
// restic-manager-tests/tests/commands/run.rs

use test_utils::{MockResticOps, ResticOperations};
use restic_manager::utils::restic::ResticEnv;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_run_backup_creates_snapshot() {
    let temp_dir = TempDir::new().unwrap();
    let password_file = temp_dir.path().join("password");
    std::fs::write(&password_file, "test").unwrap();

    let mock = MockResticOps::new();
    let env = ResticEnv::new(&password_file, "/tmp/repo");
    let timeout = Duration::from_secs(60);

    let result = mock.backup(&env, &[], &[], timeout);
    assert!(result.is_ok());
    assert!(mock.backup_called());
}
```

### Docker Integration Test Example

```rust
// restic-manager-tests/tests/integration/postgres.rs

#[test]
#[ignore]  // Requires Docker
fn test_postgres_backup_with_docker() {
    // This test requires a real Docker environment
    // It would use testcontainers to spin up a PostgreSQL container
}
```

## Test Coverage

| Component | Tests | Description |
|-----------|-------|-------------|
| Unit: Config | 8 | Config loading, validation, profiles |
| Unit: Restic | 18 | URL building, env vars, mock operations |
| Unit: Docker | 14 | Volume listing, archiving, mock operations |
| Commands: run | 11 | Backup execution, services, retention |
| Commands: restore | 10 | Snapshot restore, volumes, paths |
| Commands: status | 10 | Health checks, stats, snapshots |
| Commands: list | 7 | Service listing, details |
| Commands: snapshots | 12 | Snapshot listing, filtering |
| Commands: verify | 10 | Repository verification |
| Commands: setup | 10 | Directory creation, initialization |
| Commands: validate | 10 | Config validation |
| Commands: restic_binary | 9 | Binary management |
| Integration | 8 | Docker-based tests (ignored) |
| **Total** | **130+** | All tests |

## Troubleshooting

### Docker Not Available

```bash
# Check if Docker is running
docker ps

# Start Docker Desktop (Windows/macOS)
# Or: sudo systemctl start docker (Linux)
```

### Windows Path Issues

When writing TOML configs in tests, use forward slashes or escape backslashes:

```rust
let password_file = builder.password_file().to_path_buf()
    .to_string_lossy().replace('\\', "/");
```

### Test Timeout

```bash
# Increase timeout in cargo test
cargo test -p restic-manager-tests -- --test-threads=1
```

### Permission Denied (Linux)

```bash
# Add user to docker group
sudo usermod -aG docker $USER
newgrp docker
```

## Quick Reference

| Command | Purpose | Docker Required |
|---------|---------|----------------|
| `cargo test` | Main crate unit tests | No |
| `cargo test -p restic-manager-tests` | All test crate tests | No |
| `cargo test -p restic-manager-tests --test unit` | Unit tests only | No |
| `cargo test -p restic-manager-tests --test commands` | Command tests only | No |
| `cargo test -p restic-manager-tests -- --ignored` | Docker tests | Yes |
| `cargo test -p restic-manager-tests -- --include-ignored` | Everything | Yes |
| `cargo test -p restic-manager-tests -- --nocapture` | Show output | - |

## See Also

- [restic-manager-tests/README.md](restic-manager-tests/README.md) - Test crate docs
- [CLAUDE.md](CLAUDE.md) - Development guide
- [.github/workflows/ci.yml](.github/workflows/ci.yml) - CI/CD configuration

---

**Last Updated:** 2025-12-28
**Test Framework:** Rust built-in + trait-based mocking
**Total Tests:** 130+ tests (plus 37 main crate unit tests)
