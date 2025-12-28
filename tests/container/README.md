# Container Deployment Test

Tests restic-manager in a **fully isolated Ubuntu container** with:
- Complete deployment simulation
- Cron job setup and execution
- Docker-in-Docker support
- Real PostgreSQL database
- Full backup workflow

This test validates the actual deployment scenario on a Linux server.

## Quick Start

```bash
cd tests/container
chmod +x setup.sh cleanup.sh entrypoint.sh
./setup.sh
```

## What It Tests

- ✅ Ubuntu container deployment
- ✅ Restic binary installation
- ✅ Cron job configuration
- ✅ Scheduled backup execution
- ✅ Docker socket access (for backing up other containers)
- ✅ Log file management
- ✅ Real-world deployment scenario

## Manual Testing

### 1. Enter the Container

```bash
docker exec -it restic-manager-test bash
```

### 2. Setup Restic

```bash
/app/restic-manager setup-restic
```

### 3. Create Backup Directory

```bash
mkdir -p /backup-data/dumps
```

### 4. Run Manual Backup

```bash
/app/restic-manager --config /app/config.toml run --service postgres
```

Expected output:
```
 INFO Starting backup for service: postgres
 INFO Running 2 pre-backup hooks
 INFO Running pre-backup hook: Dump PostgreSQL database
 INFO Hook completed successfully
 INFO Running pre-backup hook: Create backup metadata
 INFO Hook completed successfully
 INFO Initializing restic repository...
 INFO Backup completed successfully
 INFO Retention policy applied
 INFO Running 2 post-backup hooks
✓ Backup completed successfully
```

### 5. Setup Cron Job

```bash
# Create cron job for every 5 minutes
echo '*/5 * * * * /app/restic-manager --config /app/config.toml run --service postgres >> /var/log/restic-manager/cron.log 2>&1' | crontab -

# Verify cron job
crontab -l
```

### 6. Monitor Cron Execution

```bash
# Inside container
tail -f /var/log/restic-manager/cron.log

# Or from host
tail -f tests/container/logs/cron.log
```

### 7. Verify Backups

```bash
# Inside container
/app/restic-manager --config /app/config.toml snapshots --service postgres

# Or use restic directly
restic -r /backup-repo --password-file /root/restic_password snapshots
```

## Test Scenarios

### Scenario 1: Manual Backup
Tests immediate backup execution with all hooks.

### Scenario 2: Cron Scheduled Backup
Tests automated execution every 5 minutes:
1. Wait 5-10 minutes
2. Check logs: `tail -f /var/log/restic-manager/cron.log`
3. Verify snapshots created

### Scenario 3: Database Changes
Test incremental backups:
```bash
# Add more data
docker exec restic-test-db psql -U testuser -d testdb -c "
INSERT INTO users (name, email) VALUES ('David', 'david@example.com');
"

# Trigger backup
/app/restic-manager --config /app/config.toml run --service postgres

# Verify snapshot shows changes
restic -r /backup-repo --password-file /root/restic_password snapshots
```

### Scenario 4: Recovery Test
Test restoration:
```bash
# Restore latest snapshot
restic -r /backup-repo --password-file /root/restic_password restore latest --target /restore-test

# Verify restored files
ls -lh /restore-test/backup-data/dumps/
cat /restore-test/backup-data/dumps/testdb.sql
```

## Cleanup

```bash
./cleanup.sh
```

This stops and removes all containers, volumes, and logs.

## Architecture

```
┌─────────────────────────────────────┐
│ restic-manager-test (Ubuntu)        │
│                                     │
│  ┌─────────────────────────────┐   │
│  │ restic-manager binary       │   │
│  │ (mounted from host)         │   │
│  └─────────────────────────────┘   │
│                                     │
│  ┌─────────────────────────────┐   │
│  │ cron daemon                 │   │
│  │ Runs backup every 5 min     │   │
│  └─────────────────────────────┘   │
│                                     │
│  Docker socket (mounted)            │
│  /var/run/docker.sock               │
│  └───> Can control other containers │
└─────────────────────────────────────┘
         │
         ├──> Backups to: /backup-repo (volume)
         │
         └──> Connects to ─────┐
                               │
                         ┌─────▼────────────────────┐
                         │ restic-test-db           │
                         │ (PostgreSQL container)   │
                         │                          │
                         │ Data: testdb database    │
                         └──────────────────────────┘
```

## Success Criteria

- ✅ Restic binary downloads successfully
- ✅ Manual backup completes without errors
- ✅ Cron job is created and listed
- ✅ Scheduled backups execute automatically
- ✅ Database dumps are created
- ✅ Restic snapshots are created
- ✅ Logs are written correctly
- ✅ Backups can be restored

## Troubleshooting

### Cron not executing

```bash
# Check cron status
service cron status

# Check cron logs
tail -f /var/log/cron.log

# Manually test the command
/app/restic-manager --config /app/config.toml run --service postgres
```

### Docker socket permission denied

The container needs access to Docker socket. Verify:
```bash
ls -l /var/run/docker.sock
```

### Database connection fails

```bash
# Test database connectivity
docker exec restic-test-db psql -U testuser -d testdb -c "SELECT 1;"
```

## Notes

- Cron schedule is set to every 5 minutes for quick testing
- In production, use appropriate schedules (daily, weekly, etc.)
- Logs are mounted to host for easy access
- Binary is mounted read-only for security
- All data persists in Docker volumes
