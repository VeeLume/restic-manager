# Integration Test Guide

This guide walks you through running a full end-to-end integration test of restic-manager with a real Docker container.

## Overview

The test sets up:
- PostgreSQL container with test data
- Pre-backup hook that dumps the database
- Docker volume backup
- Post-backup hooks for verification
- Full restic backup to local repository

## Prerequisites

- Docker Desktop installed and running
- Rust toolchain installed

**Note**: Restic will be automatically downloaded by restic-manager, so you don't need to install it manually!

## Running the Test

### Step 1: Setup Restic Binary

```cmd
cargo run --release -- setup-restic
```

This downloads the latest restic binary from GitHub.

### Step 2: Setup Test Environment

```cmd
test-setup.bat
```

This will:
1. Create test directories (`test-data`, `test-backup-repo`)
2. Start PostgreSQL container
3. Wait for database to be ready
4. Create test tables (users, posts)
5. Insert sample data

### Step 3: Run Backup

```cmd
cargo run --release -- --config test-docker-config.toml run --service postgres-backup
```

Expected output:
```
 INFO Starting backup for service: postgres-backup
 INFO Running 2 pre-backup hooks
 INFO Running pre-backup hook: Dump PostgreSQL database
 INFO Hook completed successfully: Dump PostgreSQL database
 INFO Running pre-backup hook: Create backup metadata
 INFO Hook completed successfully: Create backup metadata
 INFO Backing up 1 Docker volumes
 INFO Archiving Docker volume: restic-test-postgres-data
 INFO Successfully archived volume: restic-test-postgres-data
 INFO Starting restic backup for 2 paths
 INFO Initializing restic repository...
 INFO Repository initialized successfully
 INFO Backup completed successfully
 INFO Applying retention policy...
 INFO Retention policy applied successfully
 INFO Running 2 post-backup hooks
 INFO Running post-backup hook: Verify dump file
 INFO Hook completed successfully: Verify dump file
 INFO Running post-backup hook: Show backup statistics
 INFO Hook completed successfully: Show backup statistics
 INFO Successfully completed backup for service 'postgres-backup'
✓ Backup completed successfully
```

### Step 4: Verify Backup

#### Check backup repository:
```cmd
restic -r E:/vscode/rust/restic-manager/test-backup-repo snapshots --password-file test_password
```

#### Check what was backed up:
```cmd
restic -r E:/vscode/rust/restic-manager/test-backup-repo ls latest --password-file test_password
```

You should see:
- `test-data/dumps/testdb-backup.sql` (database dump)
- `test-data/dumps/backup-metadata.txt` (metadata)
- `restic-test-postgres-data.tar.gz` (Docker volume archive)

### Step 5: Inspect Database Dump

```cmd
type test-data\dumps\testdb-backup.sql
```

You should see SQL statements creating the tables and inserting data.

### Step 6: Test Recovery (Optional)

#### Restore from backup:
```cmd
restic -r E:/vscode/rust/restic-manager/test-backup-repo restore latest --target E:/vscode/rust/restic-manager/test-restore --password-file test_password
```

#### Verify restored files:
```cmd
dir test-restore
type test-restore\test-data\dumps\testdb-backup.sql
```

### Step 7: Cleanup

```cmd
test-cleanup.bat
```

This removes:
- Docker container and volumes
- Test data directories
- Backup repository

## What Gets Tested

### ✅ Pre-backup Hooks
- Database dump via `docker exec`
- Metadata file creation
- Command execution with timeouts
- Error handling

### ✅ Docker Volume Backup
- Volume existence verification
- Volume archiving to tar.gz
- Archive creation in temp directory

### ✅ File Backup
- Path resolution (relative to docker_base)
- Include/exclude patterns
- Multiple paths in single backup

### ✅ Restic Integration
- Repository initialization
- Backup execution with environment variables
- Retention policy application
- Repository locking/unlocking

### ✅ Post-backup Hooks
- Verification commands
- Continue on error behavior
- Hook sequencing

### ✅ Error Handling
- Timeout enforcement
- Lock file management
- Failed hook handling
- Repository unlock on failure

## Troubleshooting

### Docker Container Won't Start

```cmd
# Check Docker is running
docker ps

# View container logs
docker logs restic-test-postgres

# Restart container
docker-compose -f docker-compose.test.yml restart
```

### Database Connection Issues

```cmd
# Check container health
docker-compose -f docker-compose.test.yml ps

# Test connection manually
docker exec restic-test-postgres psql -U testuser -d testdb -c "SELECT 1;"
```

### Backup Fails

```cmd
# Enable debug logging
set RUST_LOG=debug
cargo run -- --config test-docker-config.toml run --service postgres-backup

# Check restic is in PATH
restic version

# Verify password file exists
type test_password
```

### Hook Execution Fails

Hooks run in Windows cmd shell. Test them manually:
```cmd
docker exec restic-test-postgres pg_dump -U testuser testdb > test-data\dumps\manual-test.sql
type test-data\dumps\manual-test.sql
```

## Advanced Testing

### Multiple Backups

Run the backup multiple times to test:
- Incremental backups
- Retention policy
- Deduplication

```cmd
cargo run --release -- --config test-docker-config.toml run --service postgres-backup
# Wait a bit, modify data
docker exec restic-test-postgres psql -U testuser -d testdb -c "INSERT INTO users (name, email) VALUES ('David', 'david@example.com');"
# Run backup again
cargo run --release -- --config test-docker-config.toml run --service postgres-backup
```

### Verify Retention

```cmd
# List all snapshots
restic -r E:/vscode/rust/restic-manager/test-backup-repo snapshots --password-file test_password

# Check retention is applied (should only keep configured number)
```

### Concurrent Backup Test

Try running two backups simultaneously:
```cmd
# Terminal 1
cargo run -- --config test-docker-config.toml run --service postgres-backup

# Terminal 2 (while first is running)
cargo run -- --config test-docker-config.toml run --service postgres-backup
```

Second should fail with lock error.

### Large Data Test

```cmd
# Add more data
docker exec restic-test-postgres psql -U testuser -d testdb -c "INSERT INTO posts (user_id, title, content) SELECT user_id, 'Test Post ' || generate_series(1,1000), 'Large content test' FROM users LIMIT 1000;"

# Backup should handle larger dump
cargo run --release -- --config test-docker-config.toml run --service postgres-backup
```

## Success Criteria

The test is successful if:

1. ✅ All pre-backup hooks execute successfully
2. ✅ Docker volume is archived without errors
3. ✅ Database dump file is created with valid SQL
4. ✅ Restic backup completes and reports success
5. ✅ Retention policy is applied
6. ✅ All post-backup hooks execute
7. ✅ Backup can be restored and verified
8. ✅ No lock files remain after completion
9. ✅ Logs show proper sequencing of operations

## Next Steps

After successful test:
1. Adapt configuration for your real services
2. Create hooks for your actual databases (MariaDB, PostgreSQL, etc.)
3. Configure real backup destinations (SFTP, S3, etc.)
4. Set up scheduling with cron/Task Scheduler
5. Configure Discord notifications
