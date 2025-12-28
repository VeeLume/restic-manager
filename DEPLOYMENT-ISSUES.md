# Deployment Session Summary - 2025-12-28

## Attempted Deployment

**Goal**: Deploy restic-manager from Windows 11/WSL Ubuntu to production Ubuntu server

**Server Details**:
- OS: Ubuntu 24.04.3 LTS
- User: valerie
- Destinations: 2 SFTP (home.veelume.icu, Hetzner Storage Box)
- Services: 6 (immich, shomu-discord-bot, 4 game servers)

## Issues Encountered

### 1. Build Issues in WSL

**Problem**: OpenSSL dependency errors when building in WSL
```
error: failed to run custom build command for `openssl-sys v0.9.111`
Could not find directory of OpenSSL installation
```

**Attempted Fix**: User had already switched reqwest to rustls-tls, but other dependencies still pulled in OpenSSL

**Resolution Needed**: Install build dependencies in WSL:
```bash
sudo apt-get update && sudo apt-get install -y pkg-config libssl-dev
```

**Status**: ⚠️ Not resolved - WSL sudo disabled

### 2. Deployment Script Issues

**Problem**: SSH + sudo + heredoc don't work well together
```bash
ssh user@server << 'EOF'
sudo command  # This fails - no terminal for password
EOF
```

**Error**: `sudo: a terminal is required to read the password`

**Attempted Fixes**:
- Added `-t` flag to SSH for pseudo-terminal allocation
- Created separate setup script on server
- Used `sudo -S` to read password from stdin

**Resolution**: Created `deploy-simple.sh` that copies files and creates a setup script on server to run manually

**Status**: ✅ Worked around

### 3. Configuration Format Error

**Problem**: Config used TOML array syntax `[[services]]` but code expects map syntax `[services.name]`

**Error**:
```
TOML parse error at line 59, column 1
invalid type: sequence, expected a map
```

**Fix**: Changed from:
```toml
[[services]]
name = "immich"
```

To:
```toml
[services.immich]
```

**Status**: ✅ Fixed

### 4. Restic Installer Broken

**Problem**: Wrong download URL in restic installer
```
Downloading from: https://github.com/restic/restic/releases/latest/download/restic_linux_amd64.bz2
Error: HTTP 404 Not Found
```

**Actual URL format**: `https://github.com/restic/restic/releases/download/v0.18.1/restic_0.18.1_linux_amd64.bz2`

**Issue**: The installer tries to use `/latest/download/` which doesn't include version numbers in filenames, but GitHub's actual URLs include version numbers.

**Workaround**: Manual installation:
```bash
wget https://github.com/restic/restic/releases/download/v0.18.1/restic_0.18.1_linux_amd64.bz2
bunzip2 restic_0.18.1_linux_amd64.bz2
sudo mv restic_0.18.1_linux_amd64 /usr/local/bin/restic
sudo chmod +x /usr/local/bin/restic
```

**Status**: ⚠️ Workaround applied, code needs fix

**Code Location**: `src/utils/restic_installer.rs` lines 87-150

### 5. Async Runtime Panic

**Problem**: Nested async runtime error during repository initialization

**Error**:
```
thread 'main' panicked at src/utils/restic.rs:66:52:
Cannot start a runtime from within a runtime. This happens because a function
(like `block_on`) attempted to block the current thread while the thread is
being used to drive asynchronous tasks.
```

**Root Cause**: The restic installer uses tokio async (`async fn ensure_restic`) but is being called from a sync context that's already in a tokio runtime.

**Code Location**: `src/utils/restic.rs:66` and `src/utils/restic_installer.rs:87`

**Status**: ❌ Critical bug - blocks deployment

### 6. System vs User Deployment

**Problem**: System deployment requires sudo for:
- `/usr/local/bin/restic-manager` (binary)
- `/etc/restic-manager/config.toml` (config)
- `/root/.restic_password` (password file - but config uses `/home/valerie/restic_password`)
- `/var/log/restic-manager/` (logs)

**User's Preference**: User-level deployment without sudo
- Binary in `~/.local/bin/` or `~/bin/`
- Config in `~/.config/restic-manager/`
- Logs in `~/.local/log/restic-manager/` or `~/logs/`
- Password in `~/restic_password`

**Status**: ⚠️ Need to redesign deployment approach

## Current Status

**What Works**:
- ✅ Binary builds successfully (6.1 MB)
- ✅ Configuration validates correctly
- ✅ File transfer to server works
- ✅ Restic binary installed manually

**What's Blocked**:
- ❌ Restic installer (404 + async runtime panic)
- ❌ Repository initialization (async runtime panic)
- ❌ System-level deployment too complex
- ❌ Setup command can't complete

## Recommendations for Next Session

### 1. Fix Async Runtime Issue (CRITICAL)

**Problem**: `src/utils/restic_installer.rs` uses `async fn` but is called from sync context

**Solutions**:
- **Option A**: Make restic installer synchronous (use `reqwest::blocking`)
- **Option B**: Don't call restic installer from setup command (assume restic is already installed)
- **Option C**: Restructure to avoid nested runtimes

**Recommended**: Option A - Make installer synchronous
```rust
// In src/utils/restic_installer.rs
// Change from:
pub async fn ensure_restic(use_system: bool) -> Result<PathBuf>

// To:
pub fn ensure_restic(use_system: bool) -> Result<PathBuf>

// Use reqwest::blocking instead of tokio
```

### 2. Fix Restic Download URL

**Problem**: URL format is wrong

**Fix in `src/utils/restic_installer.rs`**:
```rust
// Current (broken):
let url = format!("https://github.com/restic/restic/releases/latest/download/restic_{os}_{arch}.bz2");

// Should be:
// 1. Get latest version from GitHub API
let latest_url = "https://api.github.com/repos/restic/restic/releases/latest";
let version = get_latest_version(latest_url)?; // e.g., "v0.18.1"

// 2. Build download URL with version
let url = format!("https://github.com/restic/restic/releases/download/{version}/restic_{version}_{os}_{arch}.bz2");
```

### 3. Support User-Level Deployment

**Change default paths**:

Instead of:
- `/usr/local/bin/restic-manager` → `~/.local/bin/restic-manager`
- `/etc/restic-manager/config.toml` → `~/.config/restic-manager/config.toml`
- `/var/log/restic-manager/` → `~/.local/log/restic-manager/`

**Add to config**:
```toml
[global]
# Allow user to specify install location
install_directory = "/home/valerie/.local/bin"
config_directory = "/home/valerie/.config/restic-manager"
log_directory = "/home/valerie/.local/log/restic-manager"
```

**Benefits**:
- No sudo required
- Easier deployment
- User controls everything
- Can still use system-level if needed

### 4. Simplify Setup Command

Current setup does too much:
1. Create directories
2. Initialize repositories (← this is where it fails)
3. Install cron jobs

**Split into separate steps**:
```bash
# Step 1: Create directories only
restic-manager setup --dirs-only

# Step 2: Install restic separately
restic-manager ensure-restic  # or use system restic

# Step 3: Initialize repositories
restic-manager init-repos

# Step 4: Setup cron
restic-manager setup --cron-only
```

### 5. Better Error Messages

Instead of async runtime panic, detect and show helpful error:
```rust
if tokio::runtime::Handle::try_current().is_ok() {
    return Err(anyhow!("Restic installer cannot be called from async context. Please install restic manually or use system restic."));
}
```

## Suggested Architecture Changes

### Make restic_installer.rs Synchronous

```rust
// src/utils/restic_installer.rs

use reqwest::blocking::Client; // Instead of async

pub fn download_restic(os: &str, arch: &str) -> Result<PathBuf> {
    let client = Client::new();

    // Get latest release from GitHub API
    let releases_url = "https://api.github.com/repos/restic/restic/releases/latest";
    let response = client.get(releases_url)
        .header("User-Agent", "restic-manager")
        .send()?;

    let release: serde_json::Value = response.json()?;
    let version = release["tag_name"].as_str()
        .ok_or_else(|| anyhow!("Could not get version"))?;

    // Build download URL
    let filename = format!("restic_{version}_{os}_{arch}.bz2");
    let url = format!("https://github.com/restic/restic/releases/download/{version}/{filename}");

    // Download and extract...
}
```

### User-Level Deployment Script

```bash
#!/bin/bash
# deploy-user.sh - No sudo required

# Paths
BIN_DIR="$HOME/.local/bin"
CONFIG_DIR="$HOME/.config/restic-manager"
LOG_DIR="$HOME/.local/log/restic-manager"

# Create directories
mkdir -p "$BIN_DIR" "$CONFIG_DIR" "$LOG_DIR"

# Copy binary
cp target/release/restic-manager "$BIN_DIR/"
chmod +x "$BIN_DIR/restic-manager"

# Copy config
cp production-config.toml "$CONFIG_DIR/config.toml"

# Add to PATH
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc

# Create password file
echo "your-password" > ~/restic_password
chmod 600 ~/restic_password

# Install restic (system or manual)
# User can: apt install restic, or download manually

# Setup cron (user crontab, no sudo)
crontab -l > /tmp/mycron 2>/dev/null || true
echo "0 3 * * * $HOME/.local/bin/restic-manager --config $CONFIG_DIR/config.toml run --service immich >> $LOG_DIR/immich.log 2>&1" >> /tmp/mycron
crontab /tmp/mycron
rm /tmp/mycron
```

## Configuration for User Deployment

```toml
# production-config.toml
[global]
restic_password_file = "/home/valerie/restic_password"
docker_base = "/home/valerie/docker"
log_directory = "/home/valerie/.local/log/restic-manager"
use_system_restic = true  # Use system restic (apt install restic)

# Rest of config unchanged...
```

## Testing Strategy

After fixes, test in this order:

1. **Build Test**:
   ```bash
   cargo build --release
   ./target/release/restic-manager --version
   ```

2. **Restic Installer Test** (after fixing):
   ```bash
   ./target/release/restic-manager setup-restic
   ```

3. **Config Validation**:
   ```bash
   ./target/release/restic-manager --config production-config.toml validate
   ```

4. **Directory Setup**:
   ```bash
   ./target/release/restic-manager --config production-config.toml setup --dirs-only
   ```

5. **Repository Init** (after fixing async issue):
   ```bash
   ./target/release/restic-manager --config production-config.toml init-repos
   ```

6. **Manual Backup Test**:
   ```bash
   ./target/release/restic-manager --config production-config.toml run --service immich
   ```

7. **Cron Setup**:
   ```bash
   ./target/release/restic-manager --config production-config.toml setup --cron-only
   ```

## Files That Need Changes

### Critical (Blocking Deployment):
1. `src/utils/restic_installer.rs` - Fix async + URL
2. `src/utils/restic.rs:66` - Don't call async from sync context
3. `src/main.rs` - Handle setup command errors better

### Important (Better UX):
4. `deploy.sh` / `deploy-simple.sh` - User-level deployment
5. `production-config.toml` - User paths by default
6. `README.md` - Update deployment guide

### Nice to Have:
7. `src/utils/cron.rs` - User crontab vs system crontab
8. Error messages - More helpful guidance

## Summary

The deployment was blocked by:
1. ❌ Async runtime panic (critical)
2. ❌ Broken restic download URL
3. ⚠️ System deployment complexity (sudo issues)

**Next steps**:
1. Fix async runtime issue (make restic_installer synchronous)
2. Fix download URL (use versioned URLs)
3. Implement user-level deployment (no sudo)
4. Test thoroughly before deploying

**Recommendation**: Focus on user-level deployment first, which avoids most sudo issues and is simpler for the user to manage.
