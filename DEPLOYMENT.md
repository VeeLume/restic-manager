# Deployment Guide for Restic Manager

Complete step-by-step guide for deploying restic-manager to production servers.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Deployment Methods](#deployment-methods)
- [Step-by-Step Deployment](#step-by-step-deployment)
- [Configuration Setup](#configuration-setup)
- [Testing Deployment](#testing-deployment)
- [Monitoring and Maintenance](#monitoring-and-maintenance)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Build Server Requirements

- Rust toolchain (1.70+)
- Git
- Build essentials

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install build essentials
sudo apt-get install build-essential  # Debian/Ubuntu
sudo yum groupinstall "Development Tools"  # RHEL/CentOS
```

### Target Server Requirements

- Linux (Ubuntu 20.04+, Debian 11+, RHEL 8+, or similar)
- Docker (for backing up Docker volumes)
- Network access to backup destinations (SFTP, S3, etc.)
- Sufficient disk space for:
  - Restic binary (~30 MB)
  - Temporary backup files
  - Log files

### Optional but Recommended

- Cron or systemd (for scheduling)
- SSH access with key authentication
- Sudo privileges for initial setup

## Deployment Methods

### Method 1: Binary Distribution (Recommended)

**Best for:**
- Production servers without Rust toolchain
- Multiple servers with same architecture
- Fast, consistent deployments

**Steps:**
1. Build on build server
2. Copy binary to target servers
3. Configure and setup

### Method 2: Docker Container

**Best for:**
- Isolated environments
- Easy rollback and updates
- Kubernetes/container orchestration

### Method 3: Direct Compilation

**Best for:**
- Single server deployment
- Custom builds with specific features
- Development/testing environments

## Step-by-Step Deployment

### Step 1: Build the Binary

On your build server:

```bash
# Clone repository
git clone <your-repo-url>
cd restic-manager

# Build release binary
cargo build --release

# Verify binary
./target/release/restic-manager --version

# Strip debug symbols to reduce size (Unix only)
strip target/release/restic-manager

# Check size
ls -lh target/release/restic-manager
# Should be ~10-15 MB after stripping
```

### Step 2: Prepare Configuration

Create your production configuration file:

```bash
# Copy example config
cp config.example.toml production-config.toml

# Edit with your settings
nano production-config.toml
```

**Minimal production config:**

```toml
[global]
# Create password file first: echo "your-strong-password" > /root/.restic_password
restic_password_file = "/root/.restic_password"
log_directory = "/var/log/restic-manager"
docker_base = "/var/lib/docker/volumes"

# Retention
retention_daily = 14
retention_weekly = 8
retention_monthly = 12
retention_yearly = 3

# Destinations
[destinations.local]
type = "local"
path = "/backup/repos"

[destinations.offsite]
type = "sftp"
url = "sftp://backup@remote.example.com/backups"

# Services
[[services]]
name = "postgres"
enabled = true
targets = ["local", "offsite"]
schedule = "0 2 * * *"  # 2 AM daily
strategy = "generic"

[services.postgres.config]
# Dump database before backup
[[services.postgres.config.pre_backup_hooks]]
name = "Dump PostgreSQL"
command = "docker exec postgres pg_dump -U postgres mydb > /tmp/postgres.sql"
timeout_seconds = 600

paths = ["/tmp/postgres.sql"]
volumes = ["postgres_data"]

# Cleanup after backup
[[services.postgres.config.post_backup_hooks]]
name = "Remove dump"
command = "rm -f /tmp/postgres.sql"
continue_on_error = true
```

### Step 3: Transfer Files to Server

```bash
# Copy binary
scp target/release/restic-manager user@server.example.com:/tmp/

# Copy config
scp production-config.toml user@server.example.com:/tmp/config.toml

# SSH to server
ssh user@server.example.com
```

### Step 4: Install on Server

```bash
# Move binary to system location
sudo mv /tmp/restic-manager /usr/local/bin/
sudo chmod +x /usr/local/bin/restic-manager

# Create config directory
sudo mkdir -p /etc/restic-manager
sudo mv /tmp/config.toml /etc/restic-manager/

# Verify installation
restic-manager --version
```

### Step 5: Create Password File

```bash
# Create password file with strong password
sudo bash -c 'echo "your-very-strong-password-here" > /root/.restic_password'

# Secure the password file (CRITICAL!)
sudo chmod 600 /root/.restic_password

# Verify permissions
ls -l /root/.restic_password
# Should show: -rw------- (only root can read/write)
```

**Password Security Best Practices:**
- Use a strong random password (32+ characters)
- Never commit password to git
- Keep backup of password in secure location (password manager, vault)
- Different password per environment (dev/staging/prod)

### Step 6: Download Restic Binary

```bash
# Download and setup restic
sudo restic-manager setup-restic

# Verify restic installation
sudo restic-manager restic-version
```

### Step 7: Initialize Setup

```bash
# Preview what will be created (dry-run)
sudo restic-manager --config /etc/restic-manager/config.toml setup --dry-run

# Run full setup
sudo restic-manager --config /etc/restic-manager/config.toml setup

# This will:
# - Create /var/log/restic-manager directory
# - Initialize restic repositories on all destinations
# - Install cron jobs for scheduled backups
```

Expected output:
```
=== Setting up restic-manager ===

[1/4] Creating directories...
  ✓ Created /var/log/restic-manager

[2/4] Initializing restic repositories...
  Service: postgres
    ✓ Initialized postgres at local (/backup/repos)
    ✓ Initialized postgres at offsite (sftp://...)

[3/4] Installing cron jobs...
  ✓ Added job for 'postgres' (0 2 * * *)

[4/4] Verifying setup...
  ✓ 1 cron job(s) installed
  ✓ Log directory accessible

Setup complete! Backups will run according to schedule.
```

### Step 8: Test Manual Backup

Before relying on cron, test a manual backup:

```bash
# Run backup for one service
sudo restic-manager --config /etc/restic-manager/config.toml run --service postgres

# Check if backup succeeded
sudo restic-manager --config /etc/restic-manager/config.toml snapshots --service postgres

# Verify repository integrity
sudo restic-manager --config /etc/restic-manager/config.toml verify --service postgres
```

### Step 9: Verify Cron Installation

```bash
# Check cron jobs were installed
sudo crontab -l | grep "Restic Manager"

# Should show:
# # Restic Manager - Service: postgres
# 0 2 * * * /usr/local/bin/restic-manager --config /etc/restic-manager/config.toml run --service postgres >> /var/log/restic-manager/postgres.log 2>&1
```

### Step 10: Monitor First Scheduled Backup

Wait for the first scheduled backup to run, or manually trigger at the scheduled time:

```bash
# Watch logs in real-time
sudo tail -f /var/log/restic-manager/postgres.log

# Check cron execution (Debian/Ubuntu)
sudo tail -f /var/log/syslog | grep restic-manager

# After backup runs, verify
sudo restic-manager --config /etc/restic-manager/config.toml status
```

## Configuration Setup

### SFTP Destinations

For SFTP backup destinations, set up SSH key authentication:

```bash
# Generate SSH key (if not exists)
sudo ssh-keygen -t ed25519 -f /root/.ssh/id_ed25519 -N ""

# Copy public key to backup server
sudo ssh-copy-id -i /root/.ssh/id_ed25519.pub backup@remote.example.com

# Test connection
sudo ssh backup@remote.example.com "echo Connection successful"

# Add to config.toml
[destinations.offsite]
type = "sftp"
url = "sftp://backup@remote.example.com/backups"
```

### S3 Destinations

For S3/B2/Wasabi destinations:

```bash
# Set environment variables for restic
# Add to /etc/environment or cron environment
export AWS_ACCESS_KEY_ID="your-access-key"
export AWS_SECRET_ACCESS_KEY="your-secret-key"

# Add to config.toml
[destinations.s3]
type = "s3"
url = "s3:s3.amazonaws.com/my-backup-bucket"
```

### Multiple Services

Add all your services to config.toml:

```toml
# PostgreSQL
[[services]]
name = "postgres"
enabled = true
targets = ["local", "offsite"]
schedule = "0 2 * * *"
strategy = "generic"
# ... config ...

# Appwrite
[[services]]
name = "appwrite"
enabled = true
targets = ["local", "offsite"]
schedule = "0 3 * * *"  # Stagger times
strategy = "generic"
# ... config ...

# Docker volumes only
[[services]]
name = "volumes"
enabled = true
targets = ["local"]
schedule = "0 4 * * *"
strategy = "generic"

[services.volumes.config]
volumes = ["volume1", "volume2", "volume3"]
```

## Testing Deployment

### Comprehensive Test Checklist

```bash
# 1. Binary installed and accessible
which restic-manager
restic-manager --version

# 2. Configuration valid
restic-manager --config /etc/restic-manager/config.toml validate

# 3. Restic binary available
restic-manager restic-version

# 4. Password file accessible
test -f /root/.restic_password && echo "Password file exists"

# 5. Directories created
test -d /var/log/restic-manager && echo "Log directory exists"

# 6. Repositories initialized
restic-manager --config /etc/restic-manager/config.toml status

# 7. Manual backup works
restic-manager --config /etc/restic-manager/config.toml run --service postgres

# 8. Snapshots created
restic-manager --config /etc/restic-manager/config.toml snapshots --service postgres

# 9. Verify works
restic-manager --config /etc/restic-manager/config.toml verify --service postgres

# 10. Restore works (to temp location)
restic-manager --config /etc/restic-manager/config.toml restore --service postgres --target /tmp/test-restore

# 11. Cron jobs installed
crontab -l | grep "Restic Manager"

# 12. Cron service running
systemctl status cron
```

### Test Restore Procedure

Always test restores regularly:

```bash
# Monthly restore test
restic-manager --config /etc/restic-manager/config.toml restore \
  --service postgres \
  --target /tmp/restore-test-$(date +%Y%m%d) \
  --snapshot latest

# Verify restored data
ls -lah /tmp/restore-test-*/

# Cleanup
rm -rf /tmp/restore-test-*
```

## Monitoring and Maintenance

### Log Management

```bash
# View recent backup logs
sudo tail -100 /var/log/restic-manager/postgres.log

# Watch live
sudo tail -f /var/log/restic-manager/*.log

# Find errors
sudo grep -i error /var/log/restic-manager/*.log

# Check disk usage
du -sh /var/log/restic-manager
```

### Log Rotation

Create `/etc/logrotate.d/restic-manager`:

```
/var/log/restic-manager/*.log {
    daily
    rotate 30
    compress
    delaycompress
    notifempty
    create 0640 root root
    sharedscripts
}
```

### Regular Maintenance Tasks

**Daily:**
- Check cron execution logs
- Verify latest backup succeeded

**Weekly:**
- Review backup status: `restic-manager status`
- Check repository sizes
- Verify retention is working

**Monthly:**
- Run verification: `restic-manager verify --read-data`
- Test restore to temporary location
- Review and update retention policies
- Check disk space on backup destinations

**Quarterly:**
- Update restic binary: `restic-manager update-restic`
- Update restic-manager binary (rebuild and redeploy)
- Review and update configuration
- Disaster recovery drill (full restore test)

### Monitoring Scripts

Create `/usr/local/bin/check-backups.sh`:

```bash
#!/bin/bash
set -e

CONFIG="/etc/restic-manager/config.toml"
ALERT_EMAIL="admin@example.com"

# Run status check
if ! restic-manager --config "$CONFIG" status > /tmp/backup-status.txt 2>&1; then
    mail -s "Backup Status Check Failed" "$ALERT_EMAIL" < /tmp/backup-status.txt
    exit 1
fi

# Check for critical/warning in status
if grep -q "Critical\|Warning" /tmp/backup-status.txt; then
    mail -s "Backup Health Warning" "$ALERT_EMAIL" < /tmp/backup-status.txt
fi

rm /tmp/backup-status.txt
```

Schedule daily:
```bash
0 8 * * * /usr/local/bin/check-backups.sh
```

## Troubleshooting

### Backup Fails Immediately

```bash
# Check configuration
restic-manager --config /etc/restic-manager/config.toml validate

# Run with debug logging
RUST_LOG=debug restic-manager --config /etc/restic-manager/config.toml run --service postgres 2>&1 | tee /tmp/debug.log

# Common issues:
# - Password file not readable: chmod 600 /root/.restic_password
# - Repository not initialized: restic-manager setup --dirs-only
# - Docker not accessible: sudo usermod -aG docker $USER
```

### Cron Jobs Not Running

```bash
# Check cron service
sudo systemctl status cron

# Check crontab
sudo crontab -l

# Check syslog for cron execution
sudo grep CRON /var/log/syslog | grep restic-manager

# Common issues:
# - Cron service not running: sudo systemctl start cron
# - Environment variables not set in cron
# - Permissions issue (run as root)
```

### SFTP Connection Fails

```bash
# Test SSH connection manually
sudo ssh backup@remote.example.com

# Check SSH key permissions
ls -l /root/.ssh/id_ed25519
# Should be: -rw------- (600)

# Check known_hosts
sudo ssh-keyscan remote.example.com >> /root/.ssh/known_hosts

# Test restic connection manually
sudo -E restic -r sftp:backup@remote.example.com/backups --password-file /root/.restic_password snapshots
```

### Repository Locked

```bash
# Check for stale locks
sudo -E restic -r /backup/repos/postgres --password-file /root/.restic_password unlock

# If lock is truly stale (no backup running)
sudo -E restic -r /backup/repos/postgres --password-file /root/.restic_password unlock --remove-all
```

### Disk Space Issues

```bash
# Check available space
df -h /backup

# Check repository size
restic-manager --config /etc/restic-manager/config.toml status

# Manual prune to reclaim space
sudo -E restic -r /backup/repos/postgres --password-file /root/.restic_password forget --prune --keep-daily 7
```

## Security Best Practices

1. **Password File**
   - Use strong random passwords
   - Secure with `chmod 600`
   - Backup password securely
   - Rotate periodically

2. **SSH Keys**
   - Use ed25519 keys
   - Secure with `chmod 600`
   - Use separate keys per environment
   - Add passphrase for extra security

3. **File Permissions**
   - Config: `chmod 640 /etc/restic-manager/config.toml`
   - Binary: `chmod 755 /usr/local/bin/restic-manager`
   - Logs: `chmod 640 /var/log/restic-manager/*.log`

4. **Network**
   - Use SFTP/SSH, not FTP
   - Use TLS for S3 connections
   - Restrict access with firewall rules
   - Use VPN for remote backups

5. **Access Control**
   - Run backups as dedicated user (not root if possible)
   - Use sudo only when necessary
   - Limit who can read logs
   - Audit access regularly

## Backup Strategy Recommendations

### 3-2-1 Rule

- **3** copies of data
- **2** different storage types
- **1** offsite copy

Example:
```toml
[destinations.local]
type = "local"
path = "/backup/repos"

[destinations.nas]
type = "sftp"
url = "sftp://nas.local/backups"

[destinations.cloud]
type = "s3"
url = "s3:s3.amazonaws.com/backups"

[[services]]
name = "critical-data"
targets = ["local", "nas", "cloud"]  # 3 copies
```

### Retention Policy

Conservative approach:
```toml
retention_daily = 14    # 2 weeks
retention_weekly = 8    # 2 months
retention_monthly = 12  # 1 year
retention_yearly = 5    # 5 years
```

Aggressive (save space):
```toml
retention_daily = 7     # 1 week
retention_weekly = 4    # 1 month
retention_monthly = 6   # 6 months
retention_yearly = 2    # 2 years
```

Critical data:
```toml
retention_daily = 30    # 1 month
retention_weekly = 12   # 3 months
retention_monthly = 24  # 2 years
retention_yearly = 10   # 10 years
```

## Production Deployment Checklist

- [ ] Build server set up with Rust toolchain
- [ ] Binary compiled and tested
- [ ] Configuration created and validated
- [ ] Password file created with secure permissions
- [ ] Target servers accessible via SSH
- [ ] Binary deployed to `/usr/local/bin/`
- [ ] Configuration deployed to `/etc/restic-manager/`
- [ ] Restic binary installed
- [ ] Repositories initialized
- [ ] Manual backup tested successfully
- [ ] Snapshots visible and verified
- [ ] Repository verification passed
- [ ] Test restore completed
- [ ] Cron jobs installed and verified
- [ ] Log rotation configured
- [ ] Monitoring scripts set up
- [ ] Documentation updated with server details
- [ ] Team trained on restore procedures
- [ ] Disaster recovery plan documented
- [ ] Backup alerts/notifications configured

## Next Steps

After successful deployment:

1. **Document** - Record all configuration, passwords (in secure vault), server details
2. **Train** - Ensure team knows how to check status, run manual backups, and restore
3. **Monitor** - Set up alerts for failed backups
4. **Test** - Schedule monthly restore tests
5. **Review** - Quarterly review of backup strategy and retention policies
