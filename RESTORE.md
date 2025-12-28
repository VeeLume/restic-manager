# Restore Command

Complete guide to restoring backups with restic-manager.

## Overview

The `restore` command provides interactive and automated restoration from restic snapshots with safety features including preview, confirmation, and flexible targeting.

## Usage

```bash
# Interactive restore (recommended)
restic-manager restore --service <SERVICE_NAME>

# Restore specific snapshot
restic-manager restore --service <SERVICE_NAME> --snapshot <SNAPSHOT_ID>

# Restore to different location (safe)
restic-manager restore --service <SERVICE_NAME> --target /tmp/restore

# Restore from specific destination
restic-manager restore --service <SERVICE_NAME> --destination <DEST_NAME>

# Restore specific paths only
restic-manager restore --service <SERVICE_NAME> --path /path/to/file

# Combination (non-interactive with all options)
restic-manager restore --service postgres \
  --snapshot abc12345 \
  --destination local \
  --target /tmp/restore \
  --path /var/lib/postgresql
```

## Interactive Mode

When you don't specify all options, restic-manager guides you through the restore process interactively.

### Example Interactive Session

```bash
$ restic-manager restore --service postgres
```

**Step 1: Destination Selection** (if multiple destinations)
```
=== Restore Service: postgres ===

Multiple destinations available. Select one:
> local
  remote
```

**Step 2: Snapshot Selection**
```
Using destination: local (/backup/repos)

Available snapshots:
  abc12345 - 2025-12-28 10:30:15 (server01)
  def67890 - 2025-12-27 10:30:15 (server01)
> ghi24680 - 2025-12-26 10:30:15 (server01)

? Select snapshot to restore ›
```

**Step 3: Preview**
```
Selected snapshot: abc12345

Preview of snapshot contents:
  drwxr-xr-x     0 2025-12-28 10:30:15 /var/lib/postgresql
  drwxr-xr-x     0 2025-12-28 10:30:15 /var/lib/postgresql/data
  -rw-------  8192 2025-12-28 10:30:15 /var/lib/postgresql/data/base
  -rw-------  4096 2025-12-28 10:30:15 /var/lib/postgresql/data/global
  ... and 156 more files

Total: 160 items

No target directory specified.
Restore will overwrite original locations!

Restore target: Original locations (IN-PLACE)
```

**Step 4: Confirmation**
```
? Do you want to proceed with the restore? (y/N) ›
```

**Step 5: Execution**
```
Starting restore...

restoring <snapshot abc12345> to target /
 160 files restored in 2.4s
 Total restored: 2.3 GB

✓ Restore completed successfully!
Files restored to original locations
```

## Command Options

### Required

- `--service <SERVICE>` or `-s <SERVICE>` - Service to restore

### Optional

- `--snapshot <SNAPSHOT_ID>` - Specific snapshot to restore
  - Can be full ID or short ID (first 8 characters)
  - If not specified, interactive selection is shown
  - Defaults to most recent snapshot in interactive mode

- `--destination <DEST>` or `-d <DEST>` - Which backup destination to restore from
  - If service has multiple destinations and this is not specified, interactive selection is shown
  - If service has only one destination, it's used automatically

- `--target <PATH>` or `-t <PATH>` - Where to restore files
  - If not specified, files restore to their original locations (**DANGEROUS**)
  - **Recommended**: Always specify a target for safety
  - Example: `--target /tmp/restore`

- `--path <PATH>` - Restore only specific paths (can be used multiple times)
  - Example: `--path /var/lib/postgresql/data`
  - Example: `--path /etc/config --path /var/data`
  - Only files matching these paths will be restored

## Safety Features

### 1. Preview Before Restore

The command shows a preview of what will be restored:
- First 10 files from the snapshot
- Total number of items
- Restore target location

### 2. Confirmation Required

A confirmation prompt is shown before any restore operation:
```
? Do you want to proceed with the restore? (y/N) ›
```

Default is **No** - you must explicitly confirm.

### 3. Target Directory Warning

If no target is specified (in-place restore):
```
No target directory specified.
Restore will overwrite original locations!
```

### 4. Snapshot Validation

The command verifies the specified snapshot exists before attempting restore.

## Use Cases

### Safe Recovery Test

Restore to a temporary directory to verify backup contents:

```bash
restic-manager restore --service postgres --target /tmp/test-restore
# Inspect files in /tmp/test-restore
# Delete when done
```

### Selective File Recovery

Restore only specific files:

```bash
restic-manager restore --service postgres \
  --target /tmp/restore \
  --path /etc/postgresql/postgresql.conf
```

### Point-in-Time Recovery

Restore from a specific snapshot (not the latest):

```bash
# List snapshots first
restic-manager snapshots --service postgres

# Restore specific one
restic-manager restore --service postgres --snapshot abc12345
```

### Disaster Recovery

Full restore to original locations (**DANGEROUS** - use with caution):

```bash
# Stop the service first
systemctl stop postgresql

# Restore (will prompt for confirmation)
restic-manager restore --service postgres

# Start the service
systemctl start postgresql
```

### Cross-Server Recovery

Restore to a different server:

```bash
# On recovery server, with same config file
restic-manager restore --service postgres --target /var/lib/postgresql-recovered
```

## Non-Interactive Mode

For automation/scripting, specify all options to avoid interactive prompts:

```bash
#!/bin/bash
# Automated restore script

SERVICE="postgres"
SNAPSHOT="latest"  # or specific ID
DESTINATION="remote"
TARGET="/mnt/restore"

restic-manager restore \
  --service "$SERVICE" \
  --snapshot "$SNAPSHOT" \
  --destination "$DESTINATION" \
  --target "$TARGET"
```

## Restore Workflow

1. **Preparation**
   - Service configuration loaded
   - Destination selected (interactive or specified)
   - Repository accessed

2. **Snapshot Selection**
   - Available snapshots listed
   - Snapshot selected (interactive or specified)
   - Snapshot validated

3. **Preview**
   - Snapshot contents listed (first 10 files)
   - Total items counted
   - Target location shown

4. **Confirmation**
   - User prompted to confirm
   - Default is No (safe)

5. **Execution**
   - Restic restore executed
   - Progress shown
   - Success/failure reported

## Error Handling

### Service Not Found
```
Error: Service 'nonexistent' not found in configuration
```

### No Snapshots Available
```
No snapshots found for service 'postgres'
```

### Invalid Snapshot ID
```
Snapshot 'invalid123' not found
```

### Invalid Destination
```
Error: Service 'postgres' does not use destination 'invalid'
Available destinations: local, remote
```

### Restore Failure
```
✗ Restore failed: connection timeout
```

Exit code: 1

## Timeouts

- **List snapshots**: 60 seconds
- **List snapshot files**: 30 seconds
- **Restore operation**: 30 minutes (1800 seconds)

Large restores may need longer timeouts in future versions.

## Best Practices

### 1. Always Use --target for Testing

```bash
# GOOD: Safe restore to temp directory
restic-manager restore --service myapp --target /tmp/restore

# RISKY: In-place restore
restic-manager restore --service myapp
```

### 2. Verify Before Restore

Check snapshot integrity first:

```bash
restic-manager verify --service myapp
restic-manager snapshots --service myapp
restic-manager restore --service myapp --target /tmp/test
```

### 3. Stop Services Before In-Place Restore

```bash
systemctl stop myapp
restic-manager restore --service myapp
systemctl start myapp
```

### 4. Test Restores Regularly

Don't wait for a disaster:

```bash
# Monthly restore test
restic-manager restore --service critical-db --target /tmp/restore-test-$(date +%Y%m%d)
# Verify data integrity
# Clean up
```

### 5. Document Restore Procedures

Create runbooks for each service:

```markdown
# PostgreSQL Restore Procedure
1. Stop PostgreSQL: `systemctl stop postgresql`
2. Backup current data: `mv /var/lib/postgresql /var/lib/postgresql.old`
3. Restore: `restic-manager restore --service postgres`
4. Fix permissions: `chown -R postgres:postgres /var/lib/postgresql`
5. Start PostgreSQL: `systemctl start postgresql`
6. Verify: `psql -U postgres -c "SELECT version();"`
```

## Troubleshooting

### "Restore will overwrite original locations!"

**Problem**: No target directory specified.

**Solution**: Add `--target` flag:
```bash
restic-manager restore --service myapp --target /tmp/restore
```

### Interactive Prompts in Scripts

**Problem**: Script hangs waiting for input.

**Solution**: Specify all options:
```bash
restic-manager restore --service myapp --snapshot latest --target /restore
```

### Permission Denied During Restore

**Problem**: No write permission to target directory.

**Solution**: Run with appropriate permissions:
```bash
sudo restic-manager restore --service myapp --target /var/restore
```

### Partial Restore

To restore only specific files, use `--path`:
```bash
restic-manager restore --service myapp \
  --target /tmp/restore \
  --path /etc/myapp/config.yml
```

## Integration with Other Commands

### Workflow: Verify → List → Restore

```bash
# 1. Verify repository
restic-manager verify --service myapp

# 2. List available snapshots
restic-manager snapshots --service myapp

# 3. Restore specific snapshot
restic-manager restore --service myapp --snapshot abc12345 --target /tmp/restore
```

### Check Status Before Restore

```bash
restic-manager status --service myapp
# Verify backups exist and are recent
restic-manager restore --service myapp --target /tmp/restore
```

## Advanced Examples

### Restore Multiple Paths

```bash
restic-manager restore --service myapp \
  --target /tmp/restore \
  --path /etc/myapp \
  --path /var/lib/myapp/data \
  --path /var/log/myapp
```

### Scripted Disaster Recovery

```bash
#!/bin/bash
set -e

SERVICE="postgres"
TARGET="/var/lib/postgresql"
BACKUP_DIR="${TARGET}.backup.$(date +%Y%m%d_%H%M%S)"

echo "=== Disaster Recovery for $SERVICE ==="

# Stop service
echo "Stopping service..."
systemctl stop postgresql

# Backup current state
if [ -d "$TARGET" ]; then
    echo "Backing up current state to $BACKUP_DIR"
    mv "$TARGET" "$BACKUP_DIR"
fi

# Restore
echo "Restoring from backup..."
restic-manager restore \
  --service "$SERVICE" \
  --destination local \
  --snapshot latest

# Fix permissions
echo "Fixing permissions..."
chown -R postgres:postgres "$TARGET"

# Start service
echo "Starting service..."
systemctl start postgresql

# Verify
echo "Verifying..."
sleep 5
if systemctl is-active --quiet postgresql; then
    echo "✓ Service is running"

    # Run application-specific verification
    sudo -u postgres psql -c "SELECT version();"

    echo "✓ Restore completed successfully"
    echo "Old data backed up to: $BACKUP_DIR"
else
    echo "✗ Service failed to start"
    echo "Rolling back..."
    systemctl stop postgresql
    rm -rf "$TARGET"
    mv "$BACKUP_DIR" "$TARGET"
    systemctl start postgresql
    exit 1
fi
```

## Related Commands

- `restic-manager snapshots --service <name>` - List available snapshots
- `restic-manager status --service <name>` - Check backup status before restore
- `restic-manager verify --service <name>` - Verify repository integrity
- `restic-manager run --service <name>` - Create new backup after restore

## See Also

- [Snapshots Documentation](SNAPSHOTS.md)
- [Status and Verify Documentation](STATUS-VERIFY.md)
- [Main README](README.md)
- [Restic Restore Documentation](https://restic.readthedocs.io/en/latest/050_restore.html)
