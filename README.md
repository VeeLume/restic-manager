# Restic Manager

A robust backup orchestration tool written in Rust that wraps [restic](https://restic.net/) to manage backups across multiple services and destinations with automated scheduling, hooks, and comprehensive management commands.

## Table of Contents

- [Features](#features)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Configuration Guide](#configuration-guide)
- [Commands](#commands)
- [Testing](#testing)
- [Deployment](#deployment)
- [Documentation](#documentation)

## Features

### Core Capabilities

- **Unified Configuration**: Single TOML file with DRY principles and profile inheritance
- **Multiple Destinations**: Backup to multiple locations (SFTP, S3, B2, local) simultaneously
- **Automated Scheduling**: Cron-based scheduling (Unix) or Task Scheduler (Windows)
- **Docker Integration**: Automated Docker volume backups with tar.gz archives
- **Flexible Hooks**: Pre/post-backup hooks for database dumps and custom operations
- **Repository Management**: List snapshots, verify integrity, restore from backups
- **Restic Management**: Automatic download and updates of restic binary
- **Cross-Platform**: Windows, Linux, macOS (x64 + ARM)

### Management Commands

- **setup** - Initialize directories, repositories, and cron jobs
- **snapshots** - List available snapshots for any service
- **status** - Show backup health and last backup time
- **verify** - Check repository integrity (standard or deep)
- **restore** - Interactive restoration with preview and confirmation
- **run** - Execute backups manually or via cron

### Safety Features

- **File Locking**: Prevents concurrent backups
- **Timeouts**: Per-service and per-hook timeout controls
- **Error Handling**: Comprehensive error messages and recovery
- **Dry Run**: Preview setup changes before applying
- **Interactive Restore**: Preview and confirm before restoring

## Quick Start

### Installation

```bash
# Clone the repository
git clone <repo-url>
cd restic-manager

# Build the project
cargo build --release

# The binary will be at target/release/restic-manager
# Copy to your PATH for convenience
sudo cp target/release/restic-manager /usr/local/bin/  # Unix
# Or add to PATH on Windows
```

### First-Time Setup

```bash
# 1. Download restic binary
restic-manager setup-restic

# 2. Create configuration file
cp config.example.toml config.toml
# Edit config.toml with your settings

# 3. Validate configuration
restic-manager --config config.toml validate

# 4. Initialize directories and repositories
restic-manager --config config.toml setup

# 5. Test a manual backup
restic-manager --config config.toml run --service <service-name>

# 6. Check status
restic-manager --config config.toml status
```

### Basic Commands

```bash
# Show status overview
restic-manager status

# List configured services
restic-manager list

# Run backup for specific service
restic-manager run --service postgres

# List available snapshots
restic-manager snapshots --service postgres

# Show detailed status
restic-manager status --service postgres

# Verify repository integrity
restic-manager verify --service postgres

# Restore interactively
restic-manager restore --service postgres

# Setup cron jobs (Unix)
restic-manager setup
```

## Configuration Guide

### Configuration File Structure

The configuration uses TOML format with three main sections:

1. **Global settings** - Apply to all services
2. **Profiles** - Reusable templates for similar services
3. **Services** - Individual backup configurations

### Minimal Configuration

```toml
[global]
restic_password_file = "/path/to/password.txt"
log_directory = "/var/log/restic-manager"
docker_base = "/var/lib/docker/volumes"

[destinations.local]
type = "local"
path = "/backup/repos"

[[services]]
name = "myapp"
enabled = true
targets = ["local"]
schedule = "0 2 * * *"
strategy = "generic"

[services.myapp.config]
paths = ["myapp"]  # Relative to docker_base
```

### Full Configuration with Profiles

```toml
[global]
# Restic configuration
restic_password_file = "/home/user/.restic_password"
use_system_restic = false  # Use managed restic binary

# Paths
docker_base = "/var/lib/docker/volumes"
log_directory = "/var/log/restic-manager"

# Retention (defaults for all services)
retention_daily = 7
retention_weekly = 4
retention_monthly = 6
retention_yearly = 2

# Timeouts
default_timeout_seconds = 3600  # 1 hour

# Exclusions (applied to all services)
default_excludes = [".git", ".env", "node_modules", "*.tmp"]

# Notifications (optional)
[notifications]
discord_webhook_url = "https://discord.com/api/webhooks/..."
notify_on = ["failure", "warning"]
long_running_threshold_minutes = 120
rate_limit_minutes = 60

# Backup destinations
[destinations.local]
type = "local"
path = "/backup/repos"

[destinations.remote]
type = "sftp"
url = "sftp://user@backup.example.com/backups"

[destinations.s3]
type = "s3"
url = "s3:s3.amazonaws.com/my-bucket"

# Reusable profiles
[profiles.production]
targets = ["local", "remote"]
retention_daily = 14
retention_weekly = 8
retention_monthly = 12
notify_on = ["failure", "warning", "long_running"]

[profiles.critical]
targets = ["local", "remote", "s3"]
retention_daily = 30
retention_weekly = 12
retention_monthly = 24
retention_yearly = 5

# Services
[[services]]
name = "postgres"
enabled = true
profile = "production"
schedule = "0 2 * * *"  # Daily at 2 AM
strategy = "generic"
timeout_seconds = 7200  # 2 hours

[services.postgres.config]
# Pre-backup: dump database
[[services.postgres.config.pre_backup_hooks]]
name = "Dump PostgreSQL database"
command = "docker exec postgres pg_dump -U postgres mydb > /tmp/postgres-dump.sql"
timeout_seconds = 600

# Backup the dump and volumes
paths = ["/tmp/postgres-dump.sql"]
volumes = ["postgres_data"]

# Post-backup: cleanup
[[services.postgres.config.post_backup_hooks]]
name = "Remove database dump"
command = "rm -f /tmp/postgres-dump.sql"
continue_on_error = true

[[services]]
name = "appwrite"
enabled = true
profile = "production"
schedule = "0 3 * * *"  # Daily at 3 AM
strategy = "generic"

[services.appwrite.config]
# Pre-backup: dump MariaDB
[[services.appwrite.config.pre_backup_hooks]]
name = "Dump MariaDB"
command = "docker exec appwrite-mariadb mysqldump -u root -p$MYSQL_ROOT_PASSWORD appwrite > /tmp/appwrite-db.sql"
timeout_seconds = 600

# Backup database dump and Docker volumes
paths = ["/tmp/appwrite-db.sql"]
volumes = [
    "appwrite_appwrite-uploads",
    "appwrite_appwrite-functions",
    "appwrite_appwrite-certificates"
]

# Post-backup: cleanup
[[services.appwrite.config.post_backup_hooks]]
name = "Cleanup dump"
command = "rm -f /tmp/appwrite-db.sql"
continue_on_error = true
```

### Configuration Inheritance

Settings are resolved in this order (later overrides earlier):

1. **Global defaults** → Apply to all services
2. **Profile settings** → Inherited by services using the profile
3. **Service-level settings** → Override everything

Example:
```toml
[global]
retention_daily = 7  # Default: keep 7 daily

[profiles.production]
retention_daily = 14  # Production: keep 14 daily

[[services]]
name = "critical-db"
profile = "production"
retention_daily = 30  # This service: keep 30 daily (overrides profile and global)
```

### Backup Strategies

#### Generic Strategy (Recommended)

Handles files, Docker volumes, and custom operations via hooks:

```toml
strategy = "generic"

[services.myservice.config]
# File paths (relative to docker_base)
paths = ["myapp", "config"]

# Docker volumes
volumes = ["myapp_data", "myapp_uploads"]

# Exclude patterns
excludes = ["*.log", "cache/*"]

# Pre-backup hooks (e.g., database dumps)
[[services.myservice.config.pre_backup_hooks]]
name = "Dump database"
command = "docker exec db pg_dump mydb > /tmp/dump.sql"
timeout_seconds = 600

# Post-backup hooks (e.g., cleanup)
[[services.myservice.config.post_backup_hooks]]
name = "Remove dump"
command = "rm /tmp/dump.sql"
continue_on_error = true
```

### Scheduling

Use standard cron syntax:

```toml
# Daily at 2 AM
schedule = "0 2 * * *"

# Every 6 hours
schedule = "0 */6 * * *"

# Weekly on Sunday at 3 AM
schedule = "0 3 * * 0"

# Monthly on the 1st at midnight
schedule = "0 0 1 * *"
```

Validate at: https://crontab.guru/

## Commands

### Setup

Initialize directories, repositories, and cron jobs:

```bash
# Full setup (directories + repositories + cron)
restic-manager setup

# Preview changes without applying
restic-manager setup --dry-run

# Only create directories and initialize repositories
restic-manager setup --dirs-only

# Only install cron jobs
restic-manager setup --cron-only
```

**What it does:**
1. Creates log directories
2. Initializes restic repositories for all destinations
3. Installs cron jobs for scheduled backups (Unix)
4. Verifies setup

[Detailed documentation →](SETUP.md)

### Run Backups

Execute backups manually:

```bash
# Backup all enabled services
restic-manager run

# Backup specific service
restic-manager run --service postgres

# Backup with verbose logging
RUST_LOG=debug restic-manager run --service postgres
```

### Snapshots

List available snapshots:

```bash
# List snapshots for a service (all destinations)
restic-manager snapshots --service postgres

# List snapshots for specific destination
restic-manager snapshots --service postgres --destination remote
```

**Output:**
```
Snapshots for service 'postgres' at destination 'local':

ID        Date                 Hostname
abc12345  2025-12-28 10:30:15  server01
def67890  2025-12-27 10:30:15  server01
ghi24680  2025-12-26 10:30:15  server01

Total: 3 snapshots
Repository size: 2.3 GB (after deduplication)
```

[Detailed documentation →](SNAPSHOTS.md)

### Status

Show backup health and statistics:

```bash
# Show status for all services
restic-manager status

# Show detailed status for specific service
restic-manager status --service postgres
```

**Output:**
```
Service: postgres
  Description: PostgreSQL database backup
  Schedule: Daily at 2:00 AM (0 2 * * *)
  Strategy: generic

  Destination: local (/backup/repos)
    Snapshots: 14
    Last backup: 2 hours ago (2025-12-28 10:30:15)
    Age: ✓ Healthy
    Repository size: 2.3 GB
```

Health indicators:
- ✓ **Healthy**: Backup within 24 hours
- ⚠ **Warning**: Backup 24-48 hours old
- ✗ **Critical**: Backup over 48 hours old or no backups

[Detailed documentation →](STATUS-VERIFY.md)

### Verify

Check repository integrity:

```bash
# Verify all repositories
restic-manager verify

# Verify specific service
restic-manager verify --service postgres

# Deep verification (reads all data - slow)
restic-manager verify --service postgres --read-data
```

**Output:**
```
Service: postgres
  Destination: local (/backup/repos)
    ✓ Repository structure is OK
    ✓ No errors found

=== Verification Summary ===
Total checks: 2
Passed: 2
Failed: 0

✓ All checks passed!
```

[Detailed documentation →](STATUS-VERIFY.md)

### Restore

Interactive restoration with safety features:

```bash
# Interactive restore (recommended)
restic-manager restore --service postgres

# Restore to temporary location (safe)
restic-manager restore --service postgres --target /tmp/restore

# Restore specific snapshot
restic-manager restore --service postgres --snapshot abc12345

# Restore from specific destination
restic-manager restore --service postgres --destination remote

# Restore specific paths only
restic-manager restore --service postgres --path /etc/config
```

**Interactive workflow:**
1. Select destination (if multiple)
2. Select snapshot from list
3. Preview contents (first 10 files + total count)
4. Confirm restoration (default: No)
5. Execute with progress tracking

**Safety features:**
- Preview before restore
- Explicit confirmation required
- Warning for in-place restores
- Snapshot validation

[Detailed documentation →](RESTORE.md)

### Restic Management

Manage the restic binary:

```bash
# Download and install restic
restic-manager setup-restic

# Update restic to latest version
restic-manager update-restic

# Check restic version and location
restic-manager restic-version
```

**Binary locations:**
- **Unix**: `~/.restic-manager/bin/restic`
- **Windows**: `%LOCALAPPDATA%\restic-manager\bin\restic.exe`

[Detailed documentation →](RESTIC-MANAGEMENT.md)

### Other Commands

```bash
# Validate configuration
restic-manager validate

# List all configured services
restic-manager list
```

## Testing

Restic Manager includes comprehensive automated testing:

### Test Suites

1. **Unit Tests** - Fast, isolated tests for core functionality
2. **Integration Tests** - Real PostgreSQL container with hooks
3. **Container Tests** - Full Ubuntu deployment simulation

### Running Tests

```bash
# Unit tests only
cargo test

# All tests
./run-tests.sh --all

# Or use Makefile
make test          # Unit tests
make test-all      # All tests
make test-integration  # Integration tests
make test-container    # Container tests

# Simulate CI
make ci  # format + lint + all tests
```

### CI/CD

GitHub Actions automatically runs:
- Unit tests on Linux, macOS, Windows
- Integration tests with Docker
- Container deployment tests
- Security audit with cargo-audit

[Testing documentation →](TESTING.md)

## Deployment

### Recommended Deployment Methods

#### Method 1: Binary Distribution (Recommended)

**Build once, deploy everywhere:**

```bash
# On build server
cargo build --release
strip target/release/restic-manager  # Reduce size (Unix)

# Copy to server
scp target/release/restic-manager user@server:/usr/local/bin/

# On server
chmod +x /usr/local/bin/restic-manager
restic-manager setup-restic
restic-manager --config /etc/restic-manager/config.toml setup
```

**Benefits:**
- No compilation on production server
- Fast deployment
- Consistent binary across servers
- Works on servers without Rust toolchain

#### Method 2: Docker Deployment

**Run in container:**

```dockerfile
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    docker.io \
    cron \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY target/release/restic-manager /usr/local/bin/
COPY config.toml /etc/restic-manager/

# Setup
RUN /usr/local/bin/restic-manager setup-restic
RUN /usr/local/bin/restic-manager --config /etc/restic-manager/config.toml setup --dirs-only

# Cron setup
RUN service cron start

CMD ["cron", "-f"]
```

Build and run:
```bash
docker build -t restic-manager .
docker run -v /var/run/docker.sock:/var/run/docker.sock \
           -v /backup:/backup \
           restic-manager
```

#### Method 3: Direct Compilation

**Build on target server:**

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone <repo-url>
cd restic-manager
cargo build --release

# Install
sudo cp target/release/restic-manager /usr/local/bin/
sudo mkdir -p /etc/restic-manager
sudo cp config.toml /etc/restic-manager/

# Setup
restic-manager setup-restic
restic-manager --config /etc/restic-manager/config.toml setup
```

### Production Checklist

- [ ] Binary installed in `/usr/local/bin/restic-manager`
- [ ] Configuration at `/etc/restic-manager/config.toml`
- [ ] Password file created with secure permissions (`chmod 600`)
- [ ] Log directory created (`/var/log/restic-manager`)
- [ ] Restic binary installed (`restic-manager setup-restic`)
- [ ] Repositories initialized (`restic-manager setup --dirs-only`)
- [ ] Test manual backup (`restic-manager run --service <name>`)
- [ ] Cron jobs installed (`restic-manager setup --cron-only`)
- [ ] Verify cron jobs (`crontab -l | grep "Restic Manager"`)
- [ ] Test scheduled backup (wait for cron or run manually)
- [ ] Verify backups (`restic-manager snapshots --service <name>`)
- [ ] Set up monitoring (check logs, Discord webhooks)

### Systemd Alternative (Unix)

Instead of cron, use systemd timers:

```ini
# /etc/systemd/system/restic-manager@.service
[Unit]
Description=Restic Manager Backup - %i

[Service]
Type=oneshot
ExecStart=/usr/local/bin/restic-manager --config /etc/restic-manager/config.toml run --service %i
```

```ini
# /etc/systemd/system/restic-manager@.timer
[Unit]
Description=Restic Manager Backup Timer - %i

[Timer]
OnCalendar=daily
OnCalendar=02:00
Persistent=true

[Install]
WantedBy=timers.target
```

Enable:
```bash
sudo systemctl enable --now restic-manager@postgres.timer
sudo systemctl enable --now restic-manager@appwrite.timer
```

### Windows Task Scheduler

```powershell
# Create scheduled task
$Action = New-ScheduledTaskAction `
    -Execute "C:\Program Files\restic-manager\restic-manager.exe" `
    -Argument "--config C:\config\config.toml run --service postgres"

$Trigger = New-ScheduledTaskTrigger -Daily -At 2am

Register-ScheduledTask `
    -TaskName "Restic Manager - postgres" `
    -Action $Action `
    -Trigger $Trigger `
    -User "SYSTEM"
```

### Monitoring

**Check logs:**
```bash
tail -f /var/log/restic-manager/*.log
```

**Check cron execution:**
```bash
sudo grep "restic-manager" /var/log/syslog  # Debian/Ubuntu
sudo grep "restic-manager" /var/log/cron    # RHEL/CentOS
```

**Repository health:**
```bash
restic-manager status
restic-manager verify
```

## Documentation

### Command Guides

- [SETUP.md](SETUP.md) - Initialize directories and configure cron jobs
- [SNAPSHOTS.md](SNAPSHOTS.md) - List and manage snapshots
- [STATUS-VERIFY.md](STATUS-VERIFY.md) - Status reporting and verification
- [RESTORE.md](RESTORE.md) - Interactive restoration guide

### Reference Documentation

- [TESTING.md](TESTING.md) - Comprehensive testing guide
- [RESTIC-MANAGEMENT.md](RESTIC-MANAGEMENT.md) - Restic binary management
- [TODO.md](TODO.md) - Implementation roadmap

### Development

- [tests/README.md](tests/README.md) - Test suite overview
- [tests/integration/README.md](tests/integration/README.md) - Integration tests
- [tests/container/README.md](tests/container/README.md) - Container tests

## Architecture

### Project Structure

```
restic-manager/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── lib.rs               # Library API
│   ├── config/              # Configuration system
│   │   ├── mod.rs           # Public API
│   │   ├── types.rs         # Type definitions
│   │   └── loader.rs        # Loading and validation
│   ├── managers/            # High-level orchestration
│   │   └── backup.rs        # Backup manager with locking
│   ├── strategies/          # Backup strategies
│   │   ├── mod.rs           # Strategy trait
│   │   └── generic.rs       # Generic strategy with hooks
│   └── utils/               # Shared utilities
│       ├── command.rs       # Command execution
│       ├── restic.rs        # Restic operations
│       ├── docker.rs        # Docker volume operations
│       ├── locker.rs        # File-based locking
│       ├── cron.rs          # Cron job management
│       └── restic_installer.rs # Binary management
├── tests/
│   ├── config_tests.rs      # Configuration tests
│   ├── integration_automated.rs  # Automated test runners
│   ├── integration/         # Integration test suite
│   └── container/           # Container deployment tests
├── config.example.toml      # Example configuration
└── README.md                # This file
```

### Design Principles

1. **Type Safety**: Rust's type system ensures correctness
2. **Modular Design**: Clear separation of concerns
3. **Error Handling**: Comprehensive error messages and recovery
4. **Configuration**: TOML with inheritance and DRY principles
5. **Hooks Over Hardcoding**: Flexible pre/post hooks instead of specialized strategies
6. **Cross-Platform**: Works on Windows, Linux, macOS

## Troubleshooting

### Common Issues

**Restic binary not found:**
```bash
restic-manager setup-restic
```

**Permission denied:**
```bash
chmod +x /usr/local/bin/restic-manager
```

**Cron not executing:**
```bash
# Check cron is running
sudo systemctl status cron

# Check crontab
crontab -l | grep "Restic Manager"

# Check logs
tail -f /var/log/restic-manager/*.log
```

**Backup fails with timeout:**
```toml
# Increase timeout in config
timeout_seconds = 7200  # 2 hours
```

**Repository already exists:**
This is not an error. Setup skips already-initialized repositories.

## Contributing

Issues and pull requests welcome at: (add your repo URL)

## License

TBD

## Credits

Built to replace a previous Python implementation with improved:
- Type safety (Rust vs Python)
- Performance (compiled vs interpreted)
- Error handling (Result types vs exceptions)
- Maintainability (strong typing and borrow checker)
