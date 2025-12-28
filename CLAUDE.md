# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`restic-manager` is a Rust-based backup orchestration tool that wraps restic to manage backups across multiple services and destinations. It replaces a previous Python implementation that suffered from configuration duplication and limited flexibility.

### Core Requirements

- **Dual Destinations**: Backup to both home Raspberry Pi (SFTP: `valerie@home.veelume.icu//media/usbdrive`) and Hetzner Storage Box (SFTP: `u486657@u486657.your-storagebox.de:23/backups`)
- **Service Types**: Support both "generic" (simple file/volume backups) and "complex" (pre-backup hooks like database dumps)
- **Critical Services**: Appwrite (MariaDB + volumes), Immich (PostgreSQL + photo library)
- **Notifications**: Discord webhooks for failures, warnings, and long-running operations
- **Logging**: Structured logs with rotation for automation/debugging
- **Scheduling**: Cron-based execution with different frequencies per service criticality

### Configuration File

The system uses `backup-config.toml` (not in this repo, lives on server at `/home/valerie/backup-config.toml`). Key structure:

```toml
[global]
restic_password_file = "/home/valerie/restic_password"
docker_base = "/home/valerie/docker"
retention_daily = 6
retention_weekly = 3
retention_monthly = 1
default_timeout_seconds = 3600
log_directory = "/home/valerie/logs"

[destinations.home]
type = "sftp"
url = "sftp://valerie@home.veelume.icu//media/usbdrive"

[destinations.hetzner]
type = "sftp"
url = "sftp://u486657@u486657.your-storagebox.de:23/backups"

[notifications]
discord_webhook_url = "..."
notify_on = ["failure", "warning", "long_running"]

[services.appwrite]
enabled = true
criticality = "production"
type = "complex"
schedule = "0 2 * * *"
targets = ["home", "hetzner"]
timeout_seconds = 7200
# Either:
backup_script = "/path/to/script.sh"  # For legacy scripts
# Or:
strategy = "appwrite"  # Built-in strategy
```

## Architecture

### Module Structure

The codebase follows a clean separation of concerns:

```
src/
├── main.rs              # CLI entry point, argument parsing
├── config/              # Configuration loading and validation
│   ├── mod.rs
│   └── types.rs         # Config struct definitions
├── managers/            # High-level orchestration
│   ├── backup.rs        # Backup execution coordination
│   ├── restore.rs       # Restoration coordination
│   ├── status.rs        # Health checks and reporting
│   ├── notification.rs  # Discord webhook integration
│   └── logging.rs       # Structured logging with rotation
├── strategies/          # Service-specific backup implementations
│   ├── base.rs          # Strategy trait definition
│   ├── generic.rs       # Files + Docker volumes
│   ├── appwrite.rs      # MariaDB dump + volumes
│   └── immich.rs        # PostgreSQL dump + photo library (dual repos)
├── utils/               # Shared utilities
│   ├── restic.rs        # Restic subprocess wrappers
│   ├── docker.rs        # Docker command helpers
│   └── locker.rs        # File-based locking (prevent concurrent runs)
└── cli/                 # Command implementations
    └── commands.rs      # Subcommand handlers
```

### Strategy Pattern

Each service type implements the `BackupStrategy` trait:

```rust
trait BackupStrategy {
    fn backup(&self, config: &ServiceConfig, dest: &Destination) -> Result<()>;
    fn restore(&self, config: &ServiceConfig, snapshot_id: &str) -> Result<()>;
}
```

**Generic Strategy**: Backs up paths relative to `docker_base`, archives Docker volumes using `docker run --rm -v volume:/data -v /tmp:/backup alpine tar czf ...`, then pushes to restic.

**Appwrite Strategy**: Not yet planned, can be a placeholer

**Immich Strategy**: refer to immich docs

### CLI Commands

```bash
restic-manager                           # Status overview (default)
restic-manager run [--service NAME]      # Execute backups
restic-manager restore --service NAME    # Interactive restoration
restic-manager status [--service NAME]   # Health metrics
restic-manager setup                     # Initialize directories and register cron jobs
restic-manager verify [--service NAME]   # Run restic check
restic-manager list                      # List configured services
restic-manager snapshots --service NAME  # Show available snapshots
```

### Critical Implementation Details

**Timeouts**: All subprocess calls MUST have timeouts. Use per-service `timeout_seconds` from config, fallback to global `default_timeout_seconds`.

**Locking**: Use file-based locks (`/tmp/restic-manager-<service>.lock`) to prevent concurrent backups of the same service. Acquire lock before backup, release after completion/failure.

**Error Handling**:
- Log errors with context (service name, destination, timestamp)
- Send Discord notification on failure
- Unlock restic repository if backup fails: `restic unlock` (but check exit code, log warnings if fails)
- Never mark backup as successful if subprocess failed

**Logging**:
- File logs: `~/logs/restic-manager-<service>-YYYYMMDD.log` (DEBUG level)
- Console: INFO level
- Format: `[2025-12-27 14:30:45] [INFO] [backup.service] Message`
- Rotation: 10 files max, 10MB each

**Docker Volume Handling**:
- List volumes: `docker volume ls --format '{{.Name}}'`
- Check volume exists: exact line match (not substring)
- Archive: `docker run --rm -v <volume>:/data -v <tmp_dir>:/backup alpine tar czf /backup/<volume>.tar.gz -C /data .`
- Restore: Similar but with `tar xzf`

**Discord Notifications**:
- Rate limit: cache in `~/.cache/restic-manager-notifications.json`
- Colors: red (failure), orange (warning), yellow (long-running)
- Include: service name, error message, duration
- Threshold for "long-running": configurable per-service or global default

**Restic Integration**:
- Password: always from file (`RESTIC_PASSWORD_FILE` env var)
- Repository: `RESTIC_REPOSITORY` env var
- Show backup output (stdout) for visibility
- Retention: `restic forget --keep-daily N --keep-weekly N --keep-monthly N --prune`

## Development Commands

```bash
# Build
cargo build
cargo build --release

# Run
cargo run -- <subcommand>
cargo run -- run --service appwrite

# Test
cargo test
cargo test --lib          # Library tests only
cargo test --test <name>  # Specific integration test

# Check without building
cargo check

# Lint
cargo clippy
cargo clippy -- -D warnings  # Treat warnings as errors

# Format
cargo fmt
cargo fmt -- --check  # Verify formatting without modifying

# Documentation
cargo doc --open  # Build and open docs
```

## Key Decisions from Previous Python Implementation

**What went wrong with Python version**:
- Config duplication for similar services
- Silent failures (subprocess timeouts missing)
- Unclear backup progress (hidden stdout)
- Docker volume name mismatches (substring vs exact match)
- No concurrent run protection
- Inadequate logging for cron automation

**Rust advantages for this use case**:
- Strong typing prevents config errors
- Explicit error handling (Result types)
- Better subprocess management (tokio or std with proper timeouts)
- Trait system for strategy pattern (cleaner than Python ABC)
- Single compiled binary (no dependency management on server)
- Performance for large file operations

## Dependencies to Consider

- **clap**: CLI argument parsing
- **serde** + **toml**: Config file deserialization
- **tokio** or **async-std**: Async runtime (if using async for concurrent backups)
- **reqwest**: Discord webhook HTTP requests
- **tracing** or **env_logger**: Structured logging
- **chrono**: Timestamp formatting
- **anyhow** or **thiserror**: Error handling
- **nix** or **file-lock**: File-based locking

## Testing Strategy

- **Unit tests**: Config parsing, strategy logic, utility functions
- **Integration tests**: End-to-end backup of mock service (without actual restic/Docker)
- **Mock restic**: Use test fixtures for restic output parsing
- **Mock Docker**: Test volume listing/archiving logic without real Docker
- **Config validation**: Test all config combinations (generic vs complex, single vs dual destination)

## Deployment Context

- Target: Linux server (Ubuntu/Debian)
- Execution: Cron jobs (per-service schedules)
- User: `valerie` with Docker socket access
- Config location: `/home/valerie/backup-config.toml`
- Binary location: `/home/valerie/.local/bin/restic-manager` (or similar in PATH)
- Logs: `/home/valerie/logs/`
- Lock files: `/tmp/restic-manager-*.lock`

Critical details from bash scripts:
- **Appwrite volume names**: Use `appwrite_appwrite-*` prefix (doubled)
- **Immich dual repos**: Database and photo library are separate restic repositories
- **Retention application**: After backup, not before
- **Unlock on failure**: Always attempt, but don't fail if unlock fails
