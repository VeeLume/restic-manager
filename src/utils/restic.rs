//! Restic subprocess utilities

use super::restic_installer;
use crate::config::{Destination, RetentionPolicy};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{info, warn};

use std::sync::OnceLock;

/// Global flag for using system restic
static USE_SYSTEM_RESTIC: OnceLock<bool> = OnceLock::new();

/// Set whether to use system restic
pub fn set_use_system_restic(value: bool) {
    USE_SYSTEM_RESTIC.set(value).ok();
}

/// Get the restic binary path
fn get_restic_binary() -> String {
    let use_system = USE_SYSTEM_RESTIC.get().copied().unwrap_or(false);
    restic_installer::get_restic_command(use_system)
}

/// Environment variables for restic
pub struct ResticEnv {
    vars: HashMap<String, String>,
}

impl ResticEnv {
    /// Create new ResticEnv with password file and repository
    pub fn new(password_file: &Path, repository_url: &str) -> Self {
        let mut vars = HashMap::new();
        vars.insert(
            "RESTIC_PASSWORD_FILE".to_string(),
            password_file.display().to_string(),
        );
        vars.insert("RESTIC_REPOSITORY".to_string(), repository_url.to_string());
        Self { vars }
    }

    /// Add custom environment variable
    pub fn add(&mut self, key: String, value: String) {
        self.vars.insert(key, value);
    }

    /// Get all environment variables
    pub fn vars(&self) -> &HashMap<String, String> {
        &self.vars
    }
}

/// Initialize a restic repository if it doesn't exist
pub fn init_repository(env: &ResticEnv, timeout: Duration) -> Result<()> {
    info!("Initializing restic repository...");

    let restic_bin = get_restic_binary();
    let mut cmd = std::process::Command::new(&restic_bin);
    cmd.arg("init");
    for (key, value) in env.vars() {
        cmd.env(key, value);
    }

    let output = tokio::runtime::Handle::current().block_on(async {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::from(cmd).output(),
        )
        .await;

        match result {
            Ok(output) => output.context("Failed to execute restic init"),
            Err(_) => Err(anyhow::anyhow!("Repository initialization timed out")),
        }
    })?;

    // Repository might already exist - that's okay
    if output.status.success() {
        info!("Repository initialized successfully");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already initialized") || stderr.contains("already exists") {
            info!("Repository already initialized");
            Ok(())
        } else {
            anyhow::bail!("Failed to initialize repository: {}", stderr)
        }
    }
}

/// Backup files to restic repository
pub fn backup(
    env: &ResticEnv,
    paths: &[PathBuf],
    excludes: &[String],
    timeout: Duration,
) -> Result<()> {
    if paths.is_empty() {
        warn!("No paths to backup");
        return Ok(());
    }

    info!("Starting restic backup for {} paths", paths.len());

    let mut args = vec!["backup".to_string()];

    // Add paths
    for path in paths {
        args.push(path.display().to_string());
    }

    // Add excludes
    for exclude in excludes {
        args.push("--exclude".to_string());
        args.push(exclude.clone());
    }

    // Always exclude cache directories
    args.push("--exclude-caches".to_string());

    let restic_bin = get_restic_binary();
    let mut cmd = std::process::Command::new(&restic_bin);
    for arg in &args {
        cmd.arg(arg);
    }
    for (key, value) in env.vars() {
        cmd.env(key, value);
    }

    let output = tokio::runtime::Handle::current().block_on(async {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::from(cmd).output(),
        )
        .await;

        match result {
            Ok(output) => output.context("Failed to execute restic backup"),
            Err(_) => Err(anyhow::anyhow!("Backup timed out")),
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Backup failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("Backup completed successfully");
    println!("{}", stdout);

    Ok(())
}

/// Apply retention policy to repository
pub fn apply_retention(
    env: &ResticEnv,
    retention: &RetentionPolicy,
    timeout: Duration,
) -> Result<()> {
    info!("Applying retention policy...");

    let daily_str = retention.daily.to_string();
    let weekly_str = retention.weekly.to_string();
    let monthly_str = retention.monthly.to_string();
    let yearly_str = retention.yearly.to_string();

    let args = vec![
        "forget",
        "--prune",
        "--keep-daily",
        &daily_str,
        "--keep-weekly",
        &weekly_str,
        "--keep-monthly",
        &monthly_str,
        "--keep-yearly",
        &yearly_str,
    ];

    let restic_bin = get_restic_binary();
    let mut cmd = std::process::Command::new(&restic_bin);
    for arg in &args {
        cmd.arg(arg);
    }
    for (key, value) in env.vars() {
        cmd.env(key, value);
    }

    let output = tokio::runtime::Handle::current().block_on(async {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::from(cmd).output(),
        )
        .await;

        match result {
            Ok(output) => output.context("Failed to execute restic forget"),
            Err(_) => Err(anyhow::anyhow!("Retention policy application timed out")),
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Failed to apply retention policy: {}", stderr);
        // Don't fail the entire backup if retention fails
    } else {
        info!("Retention policy applied successfully");
    }

    Ok(())
}

/// Unlock repository (useful after failures)
pub fn unlock_repository(env: &ResticEnv, timeout: Duration) -> Result<()> {
    info!("Unlocking restic repository...");

    let mut cmd = std::process::Command::new("restic");
    cmd.arg("unlock");
    for (key, value) in env.vars() {
        cmd.env(key, value);
    }

    let output = tokio::runtime::Handle::current().block_on(async {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::from(cmd).output(),
        )
        .await;

        match result {
            Ok(output) => output.context("Failed to execute restic unlock"),
            Err(_) => Err(anyhow::anyhow!("Unlock timed out")),
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Failed to unlock repository: {}", stderr);
        // Don't fail - repository might not be locked
    } else {
        info!("Repository unlocked successfully");
    }

    Ok(())
}

/// Build repository URL for a destination and service
pub fn build_repository_url(destination: &Destination, service_name: &str, suffix: Option<&str>) -> String {
    let base_url = &destination.url;
    let repo_name = if let Some(sfx) = suffix {
        format!("{}{}", service_name, sfx)
    } else {
        service_name.to_string()
    };

    // Append service name to URL
    if base_url.ends_with('/') {
        format!("{}{}", base_url, repo_name)
    } else {
        format!("{}/{}", base_url, repo_name)
    }
}

/// Snapshot information
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub id: String,
    pub short_id: String,
    pub time: String,
    pub hostname: String,
    pub paths: Vec<String>,
}

/// List snapshots in a repository
pub fn list_snapshots(env: &ResticEnv, timeout: Duration) -> Result<Vec<Snapshot>> {
    info!("Listing snapshots from repository...");

    let restic_bin = get_restic_binary();
    let mut cmd = std::process::Command::new(&restic_bin);
    cmd.arg("snapshots")
        .arg("--json");

    for (key, value) in env.vars() {
        cmd.env(key, value);
    }

    let output = tokio::runtime::Handle::current().block_on(async {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::from(cmd).output(),
        )
        .await;

        match result {
            Ok(output) => output.context("Failed to execute restic snapshots"),
            Err(_) => Err(anyhow::anyhow!("Listing snapshots timed out")),
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to list snapshots: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON output
    let snapshots_json: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .context("Failed to parse snapshots JSON")?;

    let mut snapshots = Vec::new();
    for snapshot in snapshots_json {
        let id = snapshot["id"].as_str().unwrap_or("").to_string();
        let short_id = snapshot["short_id"].as_str().unwrap_or("").to_string();
        let time = snapshot["time"].as_str().unwrap_or("").to_string();
        let hostname = snapshot["hostname"].as_str().unwrap_or("").to_string();

        let paths = snapshot["paths"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        snapshots.push(Snapshot {
            id,
            short_id,
            time,
            hostname,
            paths,
        });
    }

    info!("Found {} snapshots", snapshots.len());
    Ok(snapshots)
}

/// Get repository stats
pub fn get_stats(env: &ResticEnv, timeout: Duration) -> Result<String> {
    info!("Getting repository statistics...");

    let restic_bin = get_restic_binary();
    let mut cmd = std::process::Command::new(&restic_bin);
    cmd.arg("stats")
        .arg("--mode")
        .arg("restore-size");

    for (key, value) in env.vars() {
        cmd.env(key, value);
    }

    let output = tokio::runtime::Handle::current().block_on(async {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::from(cmd).output(),
        )
        .await;

        match result {
            Ok(output) => output.context("Failed to execute restic stats"),
            Err(_) => Err(anyhow::anyhow!("Getting stats timed out")),
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Failed to get repository stats: {}", stderr);
        return Ok("Unknown".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Extract total size from output
    for line in stdout.lines() {
        if line.contains("Total Size:") {
            let size = line.split(':').nth(1).unwrap_or("Unknown").trim();
            return Ok(size.to_string());
        }
    }

    Ok("Unknown".to_string())
}

/// Check repository integrity
pub fn check_repository(env: &ResticEnv, read_data: bool, timeout: Duration) -> Result<String> {
    info!("Checking repository integrity...");

    let restic_bin = get_restic_binary();
    let mut cmd = std::process::Command::new(&restic_bin);
    cmd.arg("check");

    if read_data {
        cmd.arg("--read-data");
        info!("Deep verification enabled (this may take a while)");
    }

    for (key, value) in env.vars() {
        cmd.env(key, value);
    }

    let output = tokio::runtime::Handle::current().block_on(async {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::from(cmd).output(),
        )
        .await;

        match result {
            Ok(output) => output.context("Failed to execute restic check"),
            Err(_) => Err(anyhow::anyhow!("Repository check timed out")),
        }
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        anyhow::bail!("Repository check failed:\n{}\n{}", stdout, stderr);
    }

    // Combine stdout and stderr for complete output
    let full_output = format!("{}{}", stdout, stderr);

    info!("Repository check completed successfully");
    Ok(full_output)
}

/// Get the latest snapshot for a repository
pub fn get_latest_snapshot(env: &ResticEnv, timeout: Duration) -> Result<Option<Snapshot>> {
    let snapshots = list_snapshots(env, timeout)?;

    // Snapshots are returned in chronological order, last one is most recent
    Ok(snapshots.into_iter().last())
}

/// Count snapshots in a repository
pub fn count_snapshots(env: &ResticEnv, timeout: Duration) -> Result<usize> {
    let snapshots = list_snapshots(env, timeout)?;
    Ok(snapshots.len())
}

/// Restore from a snapshot
pub fn restore_snapshot(
    env: &ResticEnv,
    snapshot_id: &str,
    target_dir: Option<&str>,
    include_paths: &[String],
    timeout: Duration,
) -> Result<()> {
    info!("Restoring from snapshot: {}", snapshot_id);

    let restic_bin = get_restic_binary();
    let mut cmd = std::process::Command::new(&restic_bin);
    cmd.arg("restore")
        .arg(snapshot_id);

    // Add target directory if specified
    if let Some(target) = target_dir {
        cmd.arg("--target").arg(target);
    }

    // Add specific paths to restore if specified
    for path in include_paths {
        cmd.arg("--include").arg(path);
    }

    for (key, value) in env.vars() {
        cmd.env(key, value);
    }

    let output = tokio::runtime::Handle::current().block_on(async {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::from(cmd).output(),
        )
        .await;

        match result {
            Ok(output) => output.context("Failed to execute restic restore"),
            Err(_) => Err(anyhow::anyhow!("Restore timed out")),
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Restore failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    info!("Restore completed successfully");
    println!("{}", stdout);

    Ok(())
}

/// List files in a snapshot
pub fn list_snapshot_files(
    env: &ResticEnv,
    snapshot_id: &str,
    timeout: Duration,
) -> Result<Vec<String>> {
    info!("Listing files in snapshot: {}", snapshot_id);

    let restic_bin = get_restic_binary();
    let mut cmd = std::process::Command::new(&restic_bin);
    cmd.arg("ls")
        .arg(snapshot_id)
        .arg("--long");

    for (key, value) in env.vars() {
        cmd.env(key, value);
    }

    let output = tokio::runtime::Handle::current().block_on(async {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::from(cmd).output(),
        )
        .await;

        match result {
            Ok(output) => output.context("Failed to execute restic ls"),
            Err(_) => Err(anyhow::anyhow!("Listing files timed out")),
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to list files: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_repository_url_with_trailing_slash() {
        let destination = Destination {
            dest_type: crate::config::DestinationType::Sftp,
            url: "sftp://user@host/backups/".to_string(),
            description: "Test destination".to_string(),
        };

        let url = build_repository_url(&destination, "postgres", None);
        assert_eq!(url, "sftp://user@host/backups/postgres");
    }

    #[test]
    fn test_build_repository_url_without_trailing_slash() {
        let destination = Destination {
            dest_type: crate::config::DestinationType::Sftp,
            url: "sftp://user@host/backups".to_string(),
            description: "Test destination".to_string(),
        };

        let url = build_repository_url(&destination, "postgres", None);
        assert_eq!(url, "sftp://user@host/backups/postgres");
    }

    #[test]
    fn test_build_repository_url_with_suffix() {
        let destination = Destination {
            dest_type: crate::config::DestinationType::Local,
            url: "/tmp/backups".to_string(),
            description: "Test destination".to_string(),
        };

        let url = build_repository_url(&destination, "postgres", Some("-prod"));
        assert_eq!(url, "/tmp/backups/postgres-prod");
    }
}
