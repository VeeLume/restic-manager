# Deployment Checklist for Valerie's Server

## Configuration Summary

- **Server User**: valerie
- **Password File**: `/home/valerie/restic_password`
- **Docker Base**: `/home/valerie/docker`
- **Destinations**:
  - Home Raspberry Pi (SFTP): `home.veelume.icu`
  - Hetzner Storage Box (SFTP port 23): `u486657.your-storagebox.de:23`

## Services Configured

### Daily Backups (Important)
- ✅ **Immich** (3 AM) - Photo library + PostgreSQL database
- ✅ **Shomu Discord Bot** (4 AM) - Guild monitoring bot with logs

### Weekly Backups (Casual)
- ✅ **PZ Discord Bot** (Sunday 5 AM)
- ✅ **Zomboid Server** (Sunday 5 AM)
- ✅ **Valheim Server** (Sunday 5 AM)
- ✅ **Enshrouded Server** (Sunday 5 AM)

## Pre-Deployment Checklist

### 1. SSH Keys Setup

You need SSH keys for both SFTP destinations:

```bash
# On your production server (via SSH)

# 1. Generate SSH key if not exists
ssh-keygen -t ed25519 -f ~/.ssh/id_ed25519 -N ""

# 2. Copy to Home Raspberry Pi
ssh-copy-id -i ~/.ssh/id_ed25519.pub valerie@home.veelume.icu

# 3. Copy to Hetzner Storage Box (port 23)
ssh-copy-id -i ~/.ssh/id_ed25519.pub -p 23 u486657@u486657.your-storagebox.de

# 4. Test connections
ssh valerie@home.veelume.icu "echo 'Home connection OK'"
ssh -p 23 u486657@u486657.your-storagebox.de "echo 'Hetzner connection OK'"
```

### 2. Verify Docker Container Names

The config assumes these container names - verify they match:

```bash
# On your server
docker ps --format "table {{.Names}}\t{{.Image}}"

# Should include:
# - immich_postgres (for Immich database)
# - immich_server (optional, for stop/start during backup)
```

If your container names are different, update in `production-config.toml`:
- Line 82: `immich_postgres` → your actual container name
- Lines 74-75, 108-109: `immich_server` → your actual container name (if using)

### 3. Verify Docker Volume Names

Check your Docker volumes match the config:

```bash
# On your server
docker volume ls

# Should include:
# - shomu-discord-bot_bot_data
# - pz-discord-bot_bot_data
```

If different, update in `production-config.toml`:
- Line 125: Volume name for shomu-discord-bot
- Line 139: Volume name for pz-discord-bot

### 4. Verify Directory Structure

Check Immich upload directories exist:

```bash
# On your server
ls -la /home/valerie/docker/immich/upload/

# Should show:
# - library/
# - upload/
# - profile/
```

### 5. Password File

You need to create the restic password file:

```bash
# On your server (will be created during deployment)
# Just choose a strong password and remember it!
```

## Deployment Steps

### Option A: Automated Deployment (Recommended)

```bash
# In WSL Ubuntu
./deploy.sh
```

Follow the prompts:
1. Enter server hostname (your production server IP/hostname)
2. Enter username: `valerie`
3. Enter SSH port: `22` (default)
4. Enter restic password (choose strong password!)
5. Confirm to run setup
6. Confirm to test backup (optional but recommended)

### Option B: Manual Deployment

See the main deployment guide in the README.md or run:
```bash
cat README.md | grep -A 50 "Manual Deployment"
```

## Post-Deployment Verification

### 1. Check Installation

```bash
# SSH to your server
ssh valerie@your-server

# Verify binary
restic-manager --version

# Validate config
restic-manager --config /etc/restic-manager/config.toml validate

# List services
restic-manager list
```

### 2. Verify Cron Jobs

```bash
crontab -l | grep "Restic Manager"

# Should show 6 services:
# - immich (0 3 * * *)
# - shomu-discord-bot (0 4 * * *)
# - pz-discord-bot, zomboid, valheim, enshrouded (0 5 * * 0)
```

### 3. Test Immich Backup

```bash
# Test Immich backup manually
sudo restic-manager run --service immich

# This should:
# 1. Dump PostgreSQL database
# 2. Backup database dump + photo library
# 3. Upload to both home and hetzner
# 4. Clean up dump file

# Check status
restic-manager status --service immich

# List snapshots
restic-manager snapshots --service immich
```

### 4. Test Game Server Backup

```bash
# Test one of the game servers
sudo restic-manager run --service zomboid

# Check it worked
restic-manager snapshots --service zomboid
```

### 5. Verify SFTP Destinations

```bash
# Check both destinations have repositories

# Home
restic-manager snapshots --service immich --destination home

# Hetzner
restic-manager snapshots --service immich --destination hetzner
```

## Troubleshooting

### Issue: Container name not found

```bash
# Find actual container names
docker ps --format "{{.Names}}"

# Update production-config.toml line 82 with correct name
nano /etc/restic-manager/config.toml
```

### Issue: SFTP connection failed

```bash
# Test SSH connections manually
ssh valerie@home.veelume.icu
ssh -p 23 u486657@u486657.your-storagebox.de

# If fails, set up SSH keys (see Pre-Deployment step 1)
```

### Issue: Permission denied on photo library

```bash
# Check permissions
ls -la /home/valerie/docker/immich/upload/

# Ensure restic-manager can read
# Run backups as valerie user or use sudo
```

### Issue: Database dump fails

```bash
# Test manually
docker exec -t immich_postgres pg_dumpall --clean --if-exists --username=postgres | gzip > /tmp/test-dump.sql.gz

# Check file created
ls -lh /tmp/test-dump.sql.gz

# Clean up
rm /tmp/test-dump.sql.gz
```

## Important Notes

### Immich Backup Safety

The Immich docs recommend stopping the immich_server during backups to prevent sync issues.

**If you want this extra safety:**
1. Uncomment lines 73-77 in production-config.toml (stop server)
2. Uncomment lines 107-112 in production-config.toml (restart server)

**Trade-off:**
- Safer: Database and files guaranteed in sync
- Downside: ~5-15 min downtime during backup (3 AM, so probably fine)

### Backup Schedule

Current schedule staggers backups:
- 3 AM: Immich (important, large)
- 4 AM: Shomu bot (important, small)
- 5 AM Sunday: Game servers (casual, weekly)

This prevents multiple large backups running simultaneously.

### Retention Policy

Current settings:
- **Important services**: 6 daily, 3 weekly, 1 monthly (both destinations)
- **Casual services**: 3 daily, 2 weekly, 0 monthly (home only)

Adjust in production-config.toml if needed.

## Monitoring

### Check Backup Logs

```bash
# View logs
tail -f /var/log/restic-manager/*.log

# Check specific service
tail -f /var/log/restic-manager/immich.log

# View errors only
grep -i error /var/log/restic-manager/*.log
```

### Check Backup Status

```bash
# All services
restic-manager status

# Specific service
restic-manager status --service immich

# Shows:
# - Last backup time
# - Number of snapshots
# - Repository size
# - Health status (✓/⚠/✗)
```

### Verify Repositories

```bash
# Quick check
restic-manager verify

# Deep check (reads all data - slow!)
restic-manager verify --service immich --read-data
```

## Next Steps After Deployment

1. ✅ Wait for first scheduled backup (or run manually)
2. ✅ Verify snapshots created on both destinations
3. ✅ Test restore to temporary location
4. ✅ Set up monitoring/alerts (optional)
5. ✅ Document restore procedure for team
6. ✅ Add Appwrite when ready (template in config)

## Emergency Restore

If you need to restore Immich:

```bash
# Interactive restore (safe - to temp location)
restic-manager restore --service immich --target /tmp/immich-restore

# Will prompt to select snapshot and confirm
# Then you can manually restore from /tmp/immich-restore
```

For detailed restore procedure, see RESTORE.md documentation.
