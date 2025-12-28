# Restic Binary Management

## Overview

Restic-manager can automatically download and manage the restic binary for you, or use a system-installed version if you prefer.

## Default Behavior (Managed Binary)

By default, restic-manager uses a **managed local binary** that it downloads and maintains.

### Setup

```bash
# Download and install restic locally
restic-manager setup-restic
```

This downloads restic from GitHub and stores it at:
- **Unix**: `~/.restic-manager/bin/restic`
- **Windows**: `%LOCALAPPDATA%\restic-manager\bin\restic.exe`

### Configuration

The default configuration (no changes needed):

```toml
[global]
# ... other settings ...
use_system_restic = false  # Default - uses managed binary
```

### Benefits

- ✅ **Zero dependencies** - No need to install restic separately
- ✅ **Isolated** - Doesn't interfere with system restic
- ✅ **Consistent** - Everyone uses the same version
- ✅ **Easy updates** - `restic-manager update-restic`

## Using System Restic (Opt-in)

If you prefer to use a system-installed restic from PATH, you can opt-in.

### Configuration Method

Edit your `backup-config.toml`:

```toml
[global]
# ... other settings ...
use_system_restic = true  # Use restic from PATH
```

### CLI Override Method

Use the `--use-system-restic` flag:

```bash
# Use system restic for this command only
restic-manager --use-system-restic run

# CLI flag overrides config setting
restic-manager --use-system-restic --config backup-config.toml run
```

### Prerequisites

When `use_system_restic = true`, you must have restic installed and in PATH:

```bash
# Check if restic is available
restic version

# If not found, install it:
# Windows (Chocolatey)
choco install restic

# Windows (Scoop)
scoop install restic

# Linux (Debian/Ubuntu)
apt install restic

# macOS (Homebrew)
brew install restic
```

## Priority Order

Settings are resolved in this order (later overrides earlier):

1. **Config file**: `use_system_restic` in `backup-config.toml`
2. **CLI flag**: `--use-system-restic` command-line argument

## Commands

### Check Which Binary Is Being Used

```bash
restic-manager restic-version
```

Output when using managed binary:
```
Restic version: restic 0.17.3
Binary location: /home/user/.restic-manager/bin/restic
Source: Managed binary (use_system_restic = false)
```

Output when using system binary:
```
Restic version: restic 0.17.3
Binary location: restic
Source: System PATH (use_system_restic = true)
```

### Setup Managed Binary

```bash
# Download and install managed restic
restic-manager setup-restic
```

Always downloads the managed binary, regardless of system restic availability.

### Update Restic

```bash
# Update managed binary (if use_system_restic = false)
restic-manager update-restic

# Update system binary (if use_system_restic = true)
restic-manager update-restic
```

Uses `restic self-update` on whichever binary is configured.

## Error Messages

### Managed Binary Not Found

```
⚠️  Restic binary not found!

Restic is required for backup operations.
Run the following command to download restic:

  restic-manager setup-restic

Or set use_system_restic = true in config to use system restic.
```

**Solution**: Run `restic-manager setup-restic`

### System Binary Not Found

```
⚠️  System restic not found in PATH!

You have use_system_restic enabled, but restic is not installed.
Either:
  1. Install restic system-wide, or
  2. Run: restic-manager setup-restic
     and set use_system_restic = false
```

**Solutions**:
1. Install restic system-wide, or
2. Set `use_system_restic = false` and run `setup-restic`

## Use Cases

### Recommended: Managed Binary

**When to use:**
- ✅ Default choice for most users
- ✅ You don't want to install restic manually
- ✅ You want consistent versions across machines
- ✅ You prefer isolated, per-user installations
- ✅ You're distributing to users without restic

**Example workflow:**
```bash
git clone <repo>
cd restic-manager
cargo build --release
./target/release/restic-manager setup-restic
./target/release/restic-manager run
```

### Alternative: System Binary

**When to use:**
- ⚙️ You already have restic installed system-wide
- ⚙️ You want to use a specific restic version
- ⚙️ You're managing restic through your package manager
- ⚙️ You want centralized binary management

**Example workflow:**
```bash
# Install restic system-wide
brew install restic  # or apt/choco/scoop

# Configure restic-manager
echo "use_system_restic = true" >> backup-config.toml

# Run backups
restic-manager run
```

## Troubleshooting

### Q: Can I switch between managed and system restic?

**A:** Yes! Just change the `use_system_restic` setting or use the CLI flag.

```bash
# Switch to system restic
echo "use_system_restic = true" >> backup-config.toml

# Or use CLI override
restic-manager --use-system-restic run
```

### Q: What if I have both installed?

**A:** The `use_system_restic` setting determines which is used:
- `false` (default): Uses managed binary
- `true`: Uses system binary from PATH

### Q: How do I check which binary is active?

**A:** Run:
```bash
restic-manager restic-version
```

This shows the version, path, and source.

### Q: Can I use different restic versions for different configs?

**A:** Yes! Each config file can have its own `use_system_restic` setting. The managed binary is per-user (same for all configs), but system binary can vary.

### Q: Where is the managed binary stored?

**A:**
- **Linux/macOS**: `~/.restic-manager/bin/restic`
- **Windows**: `%LOCALAPPDATA%\restic-manager\bin\restic.exe`

### Q: How do I uninstall the managed binary?

**A:** Delete the directory:

```bash
# Linux/macOS
rm -rf ~/.restic-manager

# Windows
rmdir /s %LOCALAPPDATA%\restic-manager
```

## Best Practices

1. **Stick with managed binary** unless you have a specific reason to use system restic
2. **Document your choice** in team documentation if using system restic
3. **Version control your config** including the `use_system_restic` setting
4. **Test after changes** when switching between managed/system binary
5. **Keep updated** - run `update-restic` periodically

## Implementation Details

### How It Works

1. **Config/CLI parsed**: Determines `use_system_restic` setting
2. **Global flag set**: Single source of truth for all restic operations
3. **Binary selection**:
   - If `use_system_restic = false`: Use managed binary at `~/.restic-manager/bin/restic`
   - If `use_system_restic = true`: Use `restic` from PATH
4. **Validation**: Before running backups, checks if chosen binary exists
5. **Execution**: All restic commands use the selected binary

### Code Flow

```
main.rs
  ├─> Parse CLI args (--use-system-restic)
  ├─> Load config (use_system_restic setting)
  ├─> Determine effective setting (CLI overrides config)
  ├─> Set global flag (utils::restic::set_use_system_restic)
  └─> Validate binary exists
        ├─> If not found: Show helpful error
        └─> If found: Proceed with backup

restic.rs (backup operations)
  └─> get_restic_binary()
        └─> Uses global flag to determine which binary
```

## Summary

| Aspect | Managed Binary (Default) | System Binary (Opt-in) |
|--------|-------------------------|------------------------|
| Setup | `restic-manager setup-restic` | `apt/brew/choco install restic` |
| Config | `use_system_restic = false` | `use_system_restic = true` |
| Location | `~/.restic-manager/bin/` | System PATH |
| Update | `restic-manager update-restic` | Package manager or `restic-manager update-restic` |
| Isolation | Per-user, isolated | System-wide, shared |
| Version control | Managed by restic-manager | Managed externally |
| Recommended for | Most users | Users with existing restic |
