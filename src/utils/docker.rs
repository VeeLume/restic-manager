//! Docker utilities for volume backup and restore

use super::command::run_command_stdout;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::time::Duration;
use tracing::info;

/// List all Docker volumes
pub fn list_volumes(timeout: Duration) -> Result<Vec<String>> {
    let output = run_command_stdout(
        "docker",
        &["volume", "ls", "--format", "{{.Name}}"],
        None,
        Some(timeout),
    )?;

    Ok(output.lines().map(|s| s.to_string()).collect())
}

/// Check if a Docker volume exists
pub fn volume_exists(volume_name: &str, timeout: Duration) -> Result<bool> {
    let volumes = list_volumes(timeout)?;
    Ok(volumes.iter().any(|v| v == volume_name))
}

/// Archive a Docker volume to a tar.gz file
/// Uses a temporary Alpine container to access the volume
pub fn archive_volume(
    volume_name: &str,
    output_path: &Path,
    timeout: Duration,
) -> Result<()> {
    info!("Archiving Docker volume: {} to {:?}", volume_name, output_path);

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .context(format!("Failed to create directory: {:?}", parent))?;
    }

    // Use docker run to mount volume and create archive
    let output_dir = output_path.parent().unwrap_or(Path::new("."));
    let output_file = output_path
        .file_name()
        .context("Invalid output path")?
        .to_str()
        .context("Output path is not valid UTF-8")?;

    let volume_mount = format!("{}:/data", volume_name);
    let backup_mount = format!("{}:/backup", output_dir.display());
    let output_arg = format!("/backup/{}", output_file);

    let args = vec![
        "run",
        "--rm",
        "-v",
        &volume_mount,
        "-v",
        &backup_mount,
        "alpine",
        "tar",
        "czf",
        &output_arg,
        "-C",
        "/data",
        ".",
    ];

    let mut cmd = std::process::Command::new("docker");
    for arg in &args {
        cmd.arg(arg);
    }

    let output = tokio::runtime::Handle::current().block_on(async {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::from(cmd).output(),
        )
        .await;

        match result {
            Ok(output) => output.context("Failed to execute docker run"),
            Err(_) => Err(anyhow::anyhow!("Volume archiving timed out")),
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to archive volume {}: {}", volume_name, stderr);
    }

    info!("Successfully archived volume: {}", volume_name);
    Ok(())
}

/// Extract a Docker volume from a tar.gz file
/// Uses a temporary Alpine container to restore the volume
pub fn restore_volume(
    volume_name: &str,
    archive_path: &Path,
    timeout: Duration,
) -> Result<()> {
    info!("Restoring Docker volume: {} from {:?}", volume_name, archive_path);

    if !archive_path.exists() {
        anyhow::bail!("Archive file does not exist: {:?}", archive_path);
    }

    let archive_dir = archive_path.parent().unwrap_or(Path::new("."));
    let archive_file = archive_path
        .file_name()
        .context("Invalid archive path")?
        .to_str()
        .context("Archive path is not valid UTF-8")?;

    let volume_mount = format!("{}:/data", volume_name);
    let backup_mount = format!("{}:/backup", archive_dir.display());
    let archive_arg = format!("/backup/{}", archive_file);

    let args = vec![
        "run",
        "--rm",
        "-v",
        &volume_mount,
        "-v",
        &backup_mount,
        "alpine",
        "tar",
        "xzf",
        &archive_arg,
        "-C",
        "/data",
    ];

    let mut cmd = std::process::Command::new("docker");
    for arg in &args {
        cmd.arg(arg);
    }

    let output = tokio::runtime::Handle::current().block_on(async {
        let result = tokio::time::timeout(
            timeout,
            tokio::process::Command::from(cmd).output(),
        )
        .await;

        match result {
            Ok(output) => output.context("Failed to execute docker run"),
            Err(_) => Err(anyhow::anyhow!("Volume restoration timed out")),
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to restore volume {}: {}", volume_name, stderr);
    }

    info!("Successfully restored volume: {}", volume_name);
    Ok(())
}

/// Get the size of a Docker volume in bytes
pub fn get_volume_size(volume_name: &str, timeout: Duration) -> Result<u64> {
    let volume_mount = format!("{}:/data", volume_name);

    let args = vec![
        "run",
        "--rm",
        "-v",
        &volume_mount,
        "alpine",
        "du",
        "-sb",
        "/data",
    ];

    let output = run_command_stdout("docker", &args, None, Some(timeout))?;

    // Parse output: "12345\t/data"
    let size_str = output
        .split_whitespace()
        .next()
        .context("Failed to parse volume size")?;

    size_str
        .parse::<u64>()
        .context("Failed to parse volume size as number")
}
