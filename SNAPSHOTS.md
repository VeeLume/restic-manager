# Snapshots Command

The `snapshots` command lists all available backups for a service.

## Usage

```bash
# List snapshots for a service (all destinations)
restic-manager snapshots --service <SERVICE_NAME>

# List snapshots for specific destination
restic-manager snapshots --service <SERVICE_NAME> --destination <DESTINATION_NAME>

# Short flags
restic-manager snapshots -s <SERVICE_NAME> -d <DESTINATION_NAME>
```

## Examples

### List All Snapshots for a Service

```bash
restic-manager snapshots --service postgres
```

Output:
```
=== Snapshots for service: postgres ===

Destination: local
Repository: /backup/repos

  ID         Date                 Hostname
  --------------------------------------------------
  abc12345   2025-12-28 10:30:15  server01
  def67890   2025-12-27 10:30:15  server01
  ghi24680   2025-12-26 10:30:15  server01

  Total: 3 snapshots
  Repository size: 2.3 GB

Destination: remote
Repository: sftp://user@backup-server//backups

  ID         Date                 Hostname
  --------------------------------------------------
  jkl13579   2025-12-28 10:35:20  server01
  mno97531   2025-12-27 10:35:20  server01

  Total: 2 snapshots
  Repository size: 2.1 GB
```

### List Snapshots for Specific Destination

```bash
restic-manager snapshots --service postgres --destination local
```

Output:
```
=== Snapshots for service: postgres ===

Destination: local
Repository: /backup/repos

  ID         Date                 Hostname
  --------------------------------------------------
  abc12345   2025-12-28 10:30:15  server01
  def67890   2025-12-27 10:30:15  server01
  ghi24680   2025-12-26 10:30:15  server01

  Total: 3 snapshots
  Repository size: 2.3 GB
```

## Output Format

The command displays the following information for each snapshot:

- **ID**: Short snapshot ID (first 8 characters)
- **Date**: Timestamp when the snapshot was created
- **Hostname**: Name of the host where the backup was created
- **Total**: Total number of snapshots
- **Repository size**: Total size of the repository (deduplicated)

## Error Handling

### Service Not Found

```bash
restic-manager snapshots --service nonexistent
```

Output:
```
Error: Service 'nonexistent' not found in configuration
```

### Invalid Destination

```bash
restic-manager snapshots --service postgres --destination invalid
```

Output:
```
Error: Service 'postgres' does not use destination 'invalid'
Available destinations: local, remote
```

### No Snapshots

If no snapshots exist for a service:

```
=== Snapshots for service: postgres ===

Destination: local
Repository: /backup/repos

  No snapshots found.
```

### Connection Errors

If the repository cannot be accessed:

```
=== Snapshots for service: postgres ===

Destination: remote
Repository: sftp://user@backup-server//backups

  ✗ Failed to list snapshots: connection refused
```

## Use Cases

### Verify Backups Exist

Check that backups are being created successfully:

```bash
restic-manager snapshots --service appwrite
```

### Find Snapshot to Restore

List snapshots to find the right one for restoration:

```bash
restic-manager snapshots --service immich
# Note the snapshot ID
# Use it with restore command (when implemented)
```

### Check Backup Coverage

Verify backups exist on all destinations:

```bash
restic-manager snapshots --service critical-service
# Should show snapshots for both 'local' and 'remote'
```

### Monitor Repository Size

Check how much space backups are using:

```bash
restic-manager snapshots --service postgres
# Look at "Repository size" in output
```

## Integration with Other Commands

The snapshots command complements other restic-manager commands:

- **run**: Creates new snapshots
- **restore**: Restores from snapshots (to be implemented)
- **verify**: Verifies snapshot integrity (to be implemented)
- **status**: Shows backup health including latest snapshot (to be implemented)

## Technical Details

### How It Works

1. Loads service configuration
2. Identifies all backup destinations for the service
3. For each destination:
   - Builds repository URL
   - Queries restic with `restic snapshots --json`
   - Parses JSON output
   - Queries repository stats with `restic stats`
   - Formats and displays results

### Timeout

The command has a 60-second timeout for listing snapshots and a 30-second timeout for getting stats. If your repository is very large or the network is slow, these timeouts may need adjustment.

### JSON Output (Future)

A `--json` flag could be added to output raw snapshot data for scripting:

```bash
restic-manager snapshots --service postgres --json
```

This would output the raw restic JSON format for programmatic consumption.

## Related Commands

- `restic-manager run --service <name>` - Create new backup
- `restic-manager status --service <name>` - Show backup status
- `restic-manager restore --service <name>` - Restore from backup
- `restic-manager verify --service <name>` - Verify backup integrity

## See Also

- [Restic Documentation - Snapshots](https://restic.readthedocs.io/en/latest/045_working_with_repos.html#listing-all-snapshots)
- [Main README](README.md)
- [TODO List](TODO.md)
