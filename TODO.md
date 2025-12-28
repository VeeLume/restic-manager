# TODO List

## Remaining Features

### 1. Restore Command ✅ COMPLETED
**Priority: High**

Implement interactive restoration from restic snapshots.

**Status:** ✅ Implemented

See [RESTORE.md](RESTORE.md) for detailed documentation.

**Requirements:**
- List available snapshots for a service
- Interactive selection (or specify snapshot ID)
- Preview what will be restored
- Confirm before restoring
- Support for:
  - Full service restoration
  - Selective file restoration
  - Restore to different location (optional)

**Files to create/modify:**
- `src/managers/restore.rs` - Restoration logic
- Update `src/main.rs` - Wire up restore command
- Update CLI help text

**Example usage:**
```bash
# Interactive restore
restic-manager restore --service postgres

# Restore specific snapshot
restic-manager restore --service postgres --snapshot abc123

# Restore to different location
restic-manager restore --service postgres --target /tmp/restore
```

---

### 2. Status Command ✅ COMPLETED
**Priority: Medium**

Show backup status and health for a single service.

**Status:** ✅ Implemented

See [STATUS-VERIFY.md](STATUS-VERIFY.md) for detailed documentation.

**Requirements:**
- Last backup time
- Number of snapshots
- Repository size
- Next scheduled backup (from cron schedule)
- Health indicator (green/yellow/red based on age)
- Show for single service or all services

**Files to create/modify:**
- `src/managers/status.rs` - Status reporting logic
- Update `src/main.rs` - Enhance status command
- Add restic query functions to `src/utils/restic.rs`

**Example usage:**
```bash
# Status for one service
restic-manager status --service postgres

# Status for all services (already implemented as default)
restic-manager status
```

**Example output:**
```
Service: postgres
  Last Backup: 2 hours ago (2025-12-28 10:30:15)
  Snapshots: 14
  Repository Size: 2.3 GB
  Next Backup: in 22 hours (2025-12-29 02:00:00)
  Health: ✓ Healthy
```

---

### 3. Snapshots Command ✅ COMPLETED
**Priority: Medium**

List available snapshots for a service.

**Status:** ✅ Implemented

See [SNAPSHOTS.md](SNAPSHOTS.md) for detailed documentation.

**Requirements:**
- Query restic for snapshot list
- Format output nicely
- Show: ID, timestamp, tags, size
- Support filtering by date
- Support multiple destinations

**Files to create/modify:**
- Update `src/utils/restic.rs` - Add `list_snapshots()` function
- Update `src/main.rs` - Implement snapshots command logic

**Example usage:**
```bash
# List snapshots for service
restic-manager snapshots --service postgres

# List snapshots for specific destination
restic-manager snapshots --service postgres --destination home
```

**Example output:**
```
Snapshots for service 'postgres' at destination 'local':

ID        Date                 Size      Tags
--------  -------------------  --------  --------
abc123    2025-12-28 10:30:15  1.2 GB    postgres
def456    2025-12-27 10:30:15  1.1 GB    postgres
ghi789    2025-12-26 10:30:15  1.0 GB    postgres

Total: 3 snapshots
Repository size: 2.3 GB (after deduplication)
```

---

### 4. Setup Command ✅ COMPLETED
**Priority: High**

Initialize directories and configure cron jobs for automated backups.

**Status:** ✅ Implemented

See [SETUP.md](SETUP.md) for detailed documentation.

**Requirements:**
- Create necessary directories:
  - Log directory (from config)
  - Backup data directories
  - Initialize restic repositories on all destinations
- Install cron jobs for each service
- Validate cron syntax
- Support for:
  - `--dry-run` - show what would be done
  - `--cron-only` - only setup cron, skip dirs
  - `--dirs-only` - only setup dirs, skip cron

**Files created/modified:**
- `src/utils/cron.rs` - Cron job management ✅
- Update `src/utils/mod.rs` - Added cron module ✅
- Update `src/main.rs` - Implemented setup command ✅

**Example usage:**
```bash
# Full setup
restic-manager setup

# Dry run (show what would happen)
restic-manager setup --dry-run

# Only setup cron
restic-manager setup --cron-only

# Only initialize directories
restic-manager setup --dirs-only
```

**Example output:**
```
Setting up restic-manager...

[1/4] Creating directories...
  ✓ Created /home/user/logs
  ✓ Created /home/user/docker

[2/4] Initializing restic repositories...
  ✓ Initialized repository at home (sftp://...)
  ✓ Initialized repository at hetzner (sftp://...)

[3/4] Installing cron jobs...
  ✓ Added job for 'postgres' (0 2 * * *)
  ✓ Added job for 'appwrite' (0 3 * * *)

[4/4] Verifying setup...
  ✓ All cron jobs installed
  ✓ All directories accessible
  ✓ All repositories initialized

Setup complete! Backups will run according to schedule.

View scheduled jobs:
  crontab -l

Test a backup manually:
  restic-manager run --service postgres
```

**Cron job format:**
```bash
# Restic Manager - Service: postgres
0 2 * * * /path/to/restic-manager --config /path/to/config.toml run --service postgres >> /var/log/restic-manager/cron.log 2>&1
```

---

## Implementation Order

Suggested order based on dependencies:

1. **Snapshots Command** (Foundation for Status and Restore)
2. **Status Command** (Uses Snapshots)
3. **Restore Command** (Uses Snapshots)
4. **Setup Command** (Can be done independently)

---

## Additional Nice-to-Have Features

### 5. Verify Command ✅ COMPLETED
**Priority: Medium**

Verify repository integrity.

**Status:** ✅ Implemented

See [STATUS-VERIFY.md](STATUS-VERIFY.md) for detailed documentation.

**Requirements:**
- Run `restic check` on repositories
- Report integrity issues
- Support `--read-data` for deep verification
- Check specific service or all services
- Show detailed error messages

**Files to create/modify:**
- Update `src/utils/restic.rs` - Add `check_repository()` function
- Update `src/main.rs` - Implement verify command logic

**Example usage:**
```bash
# Verify all repositories
restic-manager verify

# Verify specific service
restic-manager verify --service postgres

# Deep verification (slower, reads all data)
restic-manager verify --service postgres --read-data
```

**Example output:**
```
Verifying repositories...

Service: postgres
  Destination: local (/backup/repos/postgres)
    ✓ Repository structure is OK
    ✓ No errors found

  Destination: remote (sftp://...)
    ✓ Repository structure is OK
    ✓ No errors found

All checks passed!
```

---

### 6. Verify Command Enhancement

### 6. Discord Notifications
Already in config, needs implementation:
- Send webhook on backup completion
- Different colors for success/warning/failure
- Rate limiting (cache in JSON file)
- Configurable events to notify on

### 7. Web UI (Future)
Optional web interface:
- View backup status
- Browse snapshots
- Trigger manual backups
- View logs
- Configuration editor

---

## Testing Checklist

For each feature, ensure:
- [ ] Works in integration test
- [ ] Works in container test
- [ ] Documented in README
- [ ] Help text added to CLI
- [ ] Error handling comprehensive
- [ ] Cross-platform compatible (Windows/Linux/macOS)
- [ ] Logging added with appropriate levels
- [ ] Example usage in docs
