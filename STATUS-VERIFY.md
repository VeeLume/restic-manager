# Status and Verify Commands

Documentation for the `status` and `verify` commands in restic-manager.

## Status Command

Shows detailed backup status and health information for services.

### Usage

```bash
# Show overview of all services
restic-manager status

# Show detailed status for a specific service
restic-manager status --service <SERVICE_NAME>

# Short flag
restic-manager status -s <SERVICE_NAME>
```

### Examples

#### Status Overview (All Services)

```bash
restic-manager status
```

Output:
```
=== Backup Status Overview ===

Services configured: 3
Destinations: 2

Services:
  postgres - PostgreSQL database (enabled)
  appwrite - Appwrite backend (enabled)
  immich - Photo management (enabled)
```

#### Detailed Service Status

```bash
restic-manager status --service postgres
```

Output:
```
=== Status for service: postgres ===

Description: PostgreSQL database backup
Enabled: Yes
Schedule: 0 2 * * *
Strategy: Generic
Timeout: 3600 seconds
Targets: local, remote

Destination: local
  Repository: /backup/repos
  Snapshots: 14
  Last Backup: 2025-12-28 10:30:15
  Age: 2 hours ago
  Health: ✓ Healthy (recent backup)
  Repository Size: 2.3 GB

Destination: remote
  Repository: sftp://user@backup-server//backups
  Snapshots: 14
  Last Backup: 2025-12-28 10:35:20
  Age: 2 hours ago
  Health: ✓ Healthy (recent backup)
  Repository Size: 2.1 GB
```

### Health Indicators

The status command uses age-based health indicators:

- **✓ Healthy** - Backup is less than 24 hours old
- **⚠ Warning** - Backup is 1-2 days old
- **✗ Critical** - Backup is over 2 days old
- **✗ No backups found** - Repository has no snapshots

### Information Displayed

For each service and destination:

- **Description**: Service description from config
- **Enabled**: Whether automatic backups are enabled
- **Schedule**: Cron schedule for automated backups
- **Strategy**: Backup strategy (Generic, Appwrite, Immich)
- **Timeout**: Maximum time allowed for backup operations
- **Targets**: List of backup destinations
- **Snapshots**: Total number of snapshots in repository
- **Last Backup**: Timestamp of most recent snapshot
- **Age**: Time since last backup (in hours)
- **Health**: Health indicator based on age
- **Repository Size**: Total size of repository (deduplicated)

### Use Cases

**Monitor Backup Health**

Quickly check if backups are up to date:

```bash
restic-manager status --service critical-db
# Look for "Healthy" status
```

**Verify Backup Coverage**

Ensure backups exist on all destinations:

```bash
restic-manager status --service postgres
# Should show status for both 'local' and 'remote'
```

**Troubleshoot Backup Issues**

Identify services with stale or missing backups:

```bash
restic-manager status
# Shows overview - look for disabled services or old backups
```

**Check Before Restore**

Verify backups exist before attempting restoration:

```bash
restic-manager status --service appwrite
# Check snapshot count and last backup time
```

---

## Verify Command

Verifies the integrity of backup repositories using restic's check functionality.

### Usage

```bash
# Verify all repositories for all enabled services
restic-manager verify

# Verify specific service
restic-manager verify --service <SERVICE_NAME>

# Deep verification (reads all data - much slower)
restic-manager verify --service <SERVICE_NAME> --read-data

# Short flags
restic-manager verify -s <SERVICE_NAME>
```

### Examples

#### Verify All Repositories

```bash
restic-manager verify
```

Output:
```
=== Verifying Repositories ===

Service: postgres
  Destination: local (/backup/repos)
    ✓ Repository structure is OK
    ✓ No errors found

  Destination: remote (sftp://user@backup-server//backups)
    ✓ Repository structure is OK
    ✓ No errors found

Service: appwrite
  Destination: local (/backup/repos)
    ✓ Repository structure is OK
    ✓ No errors found

  Destination: remote (sftp://user@backup-server//backups)
    ✓ Repository structure is OK
    ✓ No errors found

=== Verification Summary ===
Total checks: 4
Passed: 4
Failed: 0

✓ All checks passed!
```

#### Verify Specific Service

```bash
restic-manager verify --service postgres
```

Output:
```
=== Verifying Repositories ===

Service: postgres
  Destination: local (/backup/repos)
    ✓ Repository structure is OK
    ✓ No errors found

  Destination: remote (sftp://user@backup-server//backups)
    ✓ Repository structure is OK
    ✓ No errors found

=== Verification Summary ===
Total checks: 2
Passed: 2
Failed: 0

✓ All checks passed!
```

#### Deep Verification

```bash
restic-manager verify --service postgres --read-data
```

Output:
```
=== Verifying Repositories ===

⚠ Deep verification enabled (this will take longer)

Service: postgres
  Destination: local (/backup/repos)
    ✓ Repository structure is OK
    ✓ No errors found

  Destination: remote (sftp://user@backup-server//backups)
    ✓ Repository structure is OK
    ✓ No errors found

=== Verification Summary ===
Total checks: 2
Passed: 2
Failed: 0

✓ All checks passed!
```

### Verification Levels

**Standard Verification (default)**
- Checks repository structure
- Verifies pack files and indexes
- Fast (usually < 1 minute per repository)
- Timeout: 5 minutes

**Deep Verification (--read-data)**
- Reads and verifies all data blocks
- Ensures no corruption in stored data
- Much slower (can take hours for large repositories)
- Timeout: 30 minutes

### Error Handling

#### Repository Errors

If errors are found:

```
=== Verifying Repositories ===

Service: postgres
  Destination: local (/backup/repos)
    ✗ Check completed with warnings/errors
    Output: error: pack abc123 is corrupted

=== Verification Summary ===
Total checks: 1
Passed: 0
Failed: 1

✗ Some checks failed. Please review the errors above.
```

Exit code: 1 (failure)

#### Connection Failures

```
Service: postgres
  Destination: remote (sftp://user@backup-server//backups)
    ✗ Check failed: connection refused

=== Verification Summary ===
Total checks: 1
Passed: 0
Failed: 1

✗ Some checks failed. Please review the errors above.
```

### Timeouts

- **Standard check**: 5 minutes per repository
- **Deep check**: 30 minutes per repository

If verification times out, increase timeouts in future versions or run checks manually with restic.

### Scheduling Regular Verification

While restic-manager doesn't automatically schedule verification, you can use cron:

```bash
# Weekly verification of all repositories (every Sunday at 3 AM)
0 3 * * 0 /path/to/restic-manager verify >> /var/log/restic-verify.log 2>&1

# Monthly deep verification (first Sunday of month at 4 AM)
0 4 1-7 * 0 /path/to/restic-manager verify --read-data >> /var/log/restic-verify-deep.log 2>&1
```

### Best Practices

1. **Run regular checks**: Verify repositories at least weekly
2. **Use standard checks**: Deep verification only when investigating issues
3. **Monitor exit codes**: Failed verification should trigger alerts
4. **Check before restore**: Always verify repository before restoring
5. **After hardware issues**: Run deep verification if storage had problems

### Integration with Other Commands

- **status**: Check backup age before verification
- **snapshots**: List snapshots after successful verification
- **restore**: Verify repository before attempting restore

### Exit Codes

- **0**: All checks passed
- **1**: One or more checks failed

## Technical Details

### What Gets Verified

**Standard Check**
- Repository structure
- Pack file integrity
- Index consistency
- Snapshot metadata

**Deep Check (--read-data)**
- All of the above, plus:
- All data blocks are readable
- Data checksums are correct
- Complete data integrity validation

### Restic Integration

Both commands use restic under the hood:

- `status` uses: `restic snapshots --json`, `restic stats`
- `verify` uses: `restic check`, `restic check --read-data`

## Related Commands

- `restic-manager snapshots --service <name>` - List available snapshots
- `restic-manager run --service <name>` - Create new backup
- `restic-manager restore --service <name>` - Restore from backup

## See Also

- [Snapshots Documentation](SNAPSHOTS.md)
- [Main README](README.md)
- [TODO List](TODO.md)
- [Restic Check Documentation](https://restic.readthedocs.io/en/latest/075_scripting.html#checking-integrity-and-consistency)
