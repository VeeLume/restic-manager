# Setup Command

Complete guide to initializing restic-manager for automated backups.

## Overview

The `setup` command prepares your system for automated backups by:
1. Creating necessary directories
2. Initializing restic repositories
3. Installing cron jobs for scheduled backups (Unix only)
4. Verifying the setup

## Quick Start

```bash
# Preview what will be done
restic-manager setup --dry-run

# Full setup
restic-manager setup

# Only create directories and initialize repositories
restic-manager setup --dirs-only

# Only install cron jobs
restic-manager setup --cron-only
```

## Usage

```bash
restic-manager setup [OPTIONS]
```

### Options

- `--dry-run` - Show what would be done without making changes
- `--cron-only` - Only install cron jobs, skip directory creation
- `--dirs-only` - Only create directories and initialize repositories, skip cron

## What Setup Does

### Step 1: Create Directories

Creates all necessary directories defined in your configuration:

- **Log directory** (`global.log_directory`) - Where backup logs are stored
- **Docker base directory** (`global.docker_base`) - Base path for Docker volume backups
- **Service-specific directories** - Any additional paths needed by services

Example:
```
[1/4] Creating directories...
  ✓ Created /home/user/logs
  ✓ Created /home/user/docker
```

### Step 2: Initialize Restic Repositories

Initializes restic repositories for all enabled services and destinations:

- Checks if repository already exists
- Initializes new repositories
- Uses password from configuration
- Handles multiple destinations per service

Example:
```
[2/4] Initializing restic repositories...
  Service: postgres
    ✓ Initialized repository at local (/backup/repos/postgres)
    ✓ Repository already exists at remote (sftp://backup.example.com/postgres)
  Service: appwrite
    ✓ Initialized repository at hetzner (sftp://...)
```

### Step 3: Install Cron Jobs (Unix only)

Creates cron entries for each service with a backup schedule:

- Reads current crontab
- Adds or updates entries for each service
- Uses unique markers to prevent duplicates
- Logs output to service-specific log files

**Cron Entry Format:**
```bash
# Restic Manager - Service: postgres
0 2 * * * /usr/local/bin/restic-manager --config /etc/restic-manager/config.toml run --service postgres >> /var/log/restic-manager/postgres.log 2>&1
```

Example:
```
[3/4] Installing cron jobs...
  ✓ Added job for 'postgres' (0 2 * * *)
  ✓ Added job for 'appwrite' (0 3 * * *)
```

**Windows Note:**
```
[3/4] Installing cron jobs...
  ⓘ Cron jobs are not supported on Windows.
  Please use Task Scheduler to schedule backups:
    schtasks /create /tn "Restic Manager - postgres" /tr "C:\path\to\restic-manager.exe --config C:\path\to\config.toml run --service postgres" /sc daily /st 02:00
```

### Step 4: Verify Setup

Validates that everything is configured correctly:

- Checks cron jobs are installed (Unix)
- Verifies directories exist and are writable
- Confirms repositories are initialized

Example:
```
[4/4] Verifying setup...
  ✓ All cron jobs installed (2)
  ✓ All directories accessible (2)
  ✓ All repositories initialized (3)

Setup complete! Backups will run according to schedule.

View scheduled jobs:
  crontab -l

Test a backup manually:
  restic-manager run --service postgres
```

## Dry Run Mode

Preview changes before applying them:

```bash
restic-manager setup --dry-run
```

Example output:
```
[DRY RUN] Setting up restic-manager...

[1/4] Creating directories...
  [DRY RUN] Would create: /home/user/logs
  [DRY RUN] Would create: /home/user/docker

[2/4] Initializing restic repositories...
  Service: postgres
    [DRY RUN] Would initialize: /backup/repos/postgres
    [DRY RUN] Would initialize: sftp://backup.example.com/postgres
  Service: appwrite
    [DRY RUN] Would initialize: sftp://...

[3/4] Installing cron jobs...
  [DRY RUN] Would add cron job:
    # Restic Manager - Service: postgres
    0 2 * * * /usr/local/bin/restic-manager --config /etc/restic-manager/config.toml run --service postgres >> /var/log/restic-manager/postgres.log 2>&1
  [DRY RUN] Would add cron job:
    # Restic Manager - Service: appwrite
    0 3 * * * /usr/local/bin/restic-manager --config /etc/restic-manager/config.toml run --service appwrite >> /var/log/restic-manager/appwrite.log 2>&1

[4/4] Verifying setup...
  [DRY RUN] Would verify cron jobs
  [DRY RUN] Would verify directories
  [DRY RUN] Would verify repositories

[DRY RUN] No changes made. Run without --dry-run to apply.
```

## Partial Setup

### Only Directories (`--dirs-only`)

Create directories and initialize repositories, skip cron:

```bash
restic-manager setup --dirs-only
```

Use when:
- You want to manually control backup scheduling
- Setting up on Windows
- Testing repository initialization

### Only Cron (`--cron-only`)

Install cron jobs, skip directory creation:

```bash
restic-manager setup --cron-only
```

Use when:
- Directories already exist
- Updating cron schedules after config changes
- Re-installing cron after system changes

## Configuration Requirements

Your `config.toml` must define:

```toml
[global]
log_directory = "/var/log/restic-manager"  # Required for cron output
docker_base = "/var/lib/docker/volumes"     # If using Docker backups

[destinations.local]
type = "local"
path = "/backup/repos"

[[services]]
name = "postgres"
schedule = "0 2 * * *"  # Required for cron job
enabled = true
# ... rest of service config
```

## Prerequisites

### All Platforms

- Configuration file with at least one enabled service
- Restic binary installed (run `restic-manager ensure-restic` if needed)
- Write permissions for:
  - Configured directories
  - Repository locations

### Unix (Linux/macOS)

- `crontab` command available
- User crontab accessible (not restricted by system policy)

### Windows

- Task Scheduler available
- Administrative privileges for creating scheduled tasks

## Examples

### First-Time Setup

Complete setup for a new installation:

```bash
# 1. Preview the setup
restic-manager setup --dry-run

# 2. Run the full setup
sudo restic-manager setup

# 3. Verify cron jobs
crontab -l | grep "Restic Manager"

# 4. Test a backup manually
restic-manager run --service postgres

# 5. Check status
restic-manager status
```

### Update Cron Schedules

After changing schedules in config.toml:

```bash
# Update cron jobs only
restic-manager setup --cron-only

# Verify changes
crontab -l
```

### Re-initialize Repositories

Initialize new repositories without changing cron:

```bash
# Only create directories and repositories
restic-manager setup --dirs-only
```

### Windows Setup

On Windows, setup creates directories but you must configure Task Scheduler:

```bash
# Create directories and initialize repositories
restic-manager.exe setup --dirs-only

# Create scheduled task
schtasks /create `
  /tn "Restic Manager - postgres" `
  /tr "C:\Program Files\restic-manager\restic-manager.exe --config C:\config\config.toml run --service postgres" `
  /sc daily `
  /st 02:00 `
  /ru SYSTEM
```

## Troubleshooting

### "Permission denied" creating directories

**Problem**: No write permission to create directories.

**Solution**: Run with appropriate permissions:
```bash
sudo restic-manager setup
```

Or create directories manually first:
```bash
sudo mkdir -p /var/log/restic-manager
sudo chown $USER:$USER /var/log/restic-manager
restic-manager setup
```

### "Failed to execute crontab -l"

**Problem**: User crontab doesn't exist or is restricted.

**Solution**: Initialize crontab first:
```bash
crontab -l 2>/dev/null || echo "" | crontab -
restic-manager setup
```

### "Repository already exists"

**Problem**: Trying to initialize an already-initialized repository.

**Solution**: This is not an error. Setup will skip already-initialized repositories:
```
✓ Repository already exists at local (/backup/repos/postgres)
```

If you need to re-initialize (destroys data):
```bash
restic --repo /backup/repos/postgres forget --prune
restic --repo /backup/repos/postgres init
```

### Cron Jobs Not Running

**Problem**: Cron jobs installed but backups don't run.

**Solution**: Check cron logs and permissions:
```bash
# Check system cron log
sudo tail -f /var/log/cron

# Check service-specific log
tail -f /var/log/restic-manager/postgres.log

# Verify cron service is running
sudo systemctl status cron    # Debian/Ubuntu
sudo systemctl status crond   # RHEL/CentOS

# Test manually
restic-manager run --service postgres
```

### Invalid Cron Schedule

**Problem**: Schedule in config.toml is invalid.

**Solution**: Cron schedules must have exactly 5 fields:
```toml
# CORRECT
schedule = "0 2 * * *"      # Daily at 2 AM
schedule = "*/15 * * * *"   # Every 15 minutes
schedule = "0 0 1 * *"      # First day of month

# INCORRECT
schedule = "0 2 * *"        # Only 4 fields
schedule = "0 2 * * * *"    # 6 fields (seconds not supported)
```

Validate at: https://crontab.guru/

### Duplicate Cron Entries

**Problem**: Multiple entries for the same service in crontab.

**Solution**: Setup is idempotent and should prevent duplicates. If duplicates exist:
```bash
# Remove all restic-manager jobs manually
crontab -l | grep -v "Restic Manager" | crontab -

# Re-run setup
restic-manager setup --cron-only
```

## Cron Job Management

### List All Restic Manager Cron Jobs

```bash
crontab -l | grep "Restic Manager"
```

### Manually Add a Cron Job

```bash
# Get current crontab
crontab -l > mycron

# Add entry
echo "# Restic Manager - Service: myservice" >> mycron
echo "0 3 * * * /usr/local/bin/restic-manager --config /etc/restic-manager/config.toml run --service myservice >> /var/log/restic-manager/myservice.log 2>&1" >> mycron

# Install new crontab
crontab mycron
rm mycron
```

### Remove a Specific Cron Job

```bash
# Remove jobs for specific service
crontab -l | grep -v "Service: postgres" | grep -v "run --service postgres" | crontab -

# Or re-run setup (will update all)
restic-manager setup --cron-only
```

### Remove All Restic Manager Cron Jobs

```bash
crontab -l | grep -v "Restic Manager" | grep -v "restic-manager" | crontab -
```

## Best Practices

### 1. Always Dry Run First

Preview changes before applying:
```bash
restic-manager setup --dry-run
restic-manager setup
```

### 2. Version Control Your Config

Keep config.toml in version control:
```bash
git add config.toml
git commit -m "Update backup schedules"
restic-manager setup --cron-only
```

### 3. Stagger Backup Times

Avoid running all backups simultaneously:
```toml
[[services]]
name = "postgres"
schedule = "0 2 * * *"   # 2 AM

[[services]]
name = "appwrite"
schedule = "0 3 * * *"   # 3 AM

[[services]]
name = "volumes"
schedule = "0 4 * * *"   # 4 AM
```

### 4. Use Absolute Paths

Cron has limited PATH, use absolute paths in config:
```toml
[global]
log_directory = "/var/log/restic-manager"  # Not ~/logs
docker_base = "/var/lib/docker/volumes"    # Not ./docker
```

### 5. Monitor Cron Output

Check logs regularly:
```bash
tail -f /var/log/restic-manager/*.log
```

### 6. Test Before Scheduling

Always test manually first:
```bash
# Test each service
restic-manager run --service postgres
restic-manager run --service appwrite

# Then setup cron
restic-manager setup --cron-only
```

### 7. Document Your Setup

Keep notes about your configuration:
```bash
# Create setup notes
cat > DEPLOYMENT.md << 'EOF'
# Deployment Notes

Setup date: 2025-12-28
Config location: /etc/restic-manager/config.toml
Log location: /var/log/restic-manager/

Cron schedule:
- postgres: Daily at 2 AM
- appwrite: Daily at 3 AM

Last setup command:
  restic-manager setup --dry-run
  restic-manager setup
EOF
```

## Integration with Other Commands

### Complete Deployment Workflow

```bash
# 1. Install restic
restic-manager ensure-restic

# 2. Verify config
restic-manager --config config.toml status

# 3. Preview setup
restic-manager setup --dry-run

# 4. Run setup
restic-manager setup

# 5. Test backups
restic-manager run --service postgres

# 6. Verify repositories
restic-manager verify

# 7. Check status
restic-manager status
```

### Update Workflow

After changing configuration:

```bash
# 1. Update cron schedules
restic-manager setup --cron-only

# 2. Verify changes
crontab -l | grep "Restic Manager"

# 3. Test manually
restic-manager run --service myservice
```

## Platform-Specific Notes

### Linux (systemd)

Alternative to cron using systemd timers:

```bash
# Create timer unit
sudo tee /etc/systemd/system/restic-manager-postgres.timer << 'EOF'
[Unit]
Description=Restic Manager Backup - postgres

[Timer]
OnCalendar=daily
OnCalendar=02:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

# Create service unit
sudo tee /etc/systemd/system/restic-manager-postgres.service << 'EOF'
[Unit]
Description=Restic Manager Backup - postgres

[Service]
Type=oneshot
ExecStart=/usr/local/bin/restic-manager --config /etc/restic-manager/config.toml run --service postgres
EOF

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable restic-manager-postgres.timer
sudo systemctl start restic-manager-postgres.timer
```

### macOS

Use launchd instead of cron:

```bash
# Create launchd plist
cat > ~/Library/LaunchAgents/com.restic-manager.postgres.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.restic-manager.postgres</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/restic-manager</string>
        <string>--config</string>
        <string>/etc/restic-manager/config.toml</string>
        <string>run</string>
        <string>--service</string>
        <string>postgres</string>
    </array>
    <key>StartCalendarInterval</key>
    <dict>
        <key>Hour</key>
        <integer>2</integer>
        <key>Minute</key>
        <integer>0</integer>
    </dict>
</dict>
</plist>
EOF

# Load the job
launchctl load ~/Library/LaunchAgents/com.restic-manager.postgres.plist
```

### Windows Task Scheduler

Create scheduled task via PowerShell:

```powershell
$Action = New-ScheduledTaskAction `
    -Execute "C:\Program Files\restic-manager\restic-manager.exe" `
    -Argument "--config C:\config\config.toml run --service postgres"

$Trigger = New-ScheduledTaskTrigger -Daily -At 2am

$Settings = New-ScheduledTaskSettingsSet `
    -ExecutionTimeLimit (New-TimeSpan -Hours 4) `
    -RestartCount 3 `
    -RestartInterval (New-TimeSpan -Minutes 1)

Register-ScheduledTask `
    -TaskName "Restic Manager - postgres" `
    -Action $Action `
    -Trigger $Trigger `
    -Settings $Settings `
    -User "SYSTEM"
```

## Related Commands

- `restic-manager ensure-restic` - Install restic binary
- `restic-manager status` - Check backup status
- `restic-manager run --service <name>` - Manually trigger backup
- `restic-manager verify` - Verify repository integrity

## See Also

- [Main README](README.md)
- [Configuration Guide](README.md#configuration)
- [Testing Documentation](TESTING.md)
- [Cron Expression Reference](https://crontab.guru/)
