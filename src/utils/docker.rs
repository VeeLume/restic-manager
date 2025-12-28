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

    // Thread-based timeout implementation
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = cmd.output();
        let _ = tx.send(result);
    });

    let output = match rx.recv_timeout(timeout) {
        Ok(result) => result.context("Failed to execute docker run")?,
        Err(_) => anyhow::bail!("Volume archiving timed out"),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to archive volume {}: {}", volume_name, stderr);
    }

    info!("Successfully archived volume: {}", volume_name);
    Ok(())
}

/// Extract a Docker volume from a tar.gz file
/// Uses a temporary Alpine container to restore the volume
#[allow(dead_code)]
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

    // Thread-based timeout implementation
    let (tx, rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = cmd.output();
        let _ = tx.send(result);
    });

    let output = match rx.recv_timeout(timeout) {
        Ok(result) => result.context("Failed to execute docker run")?,
        Err(_) => anyhow::bail!("Volume restoration timed out"),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to restore volume {}: {}", volume_name, stderr);
    }

    info!("Successfully restored volume: {}", volume_name);
    Ok(())
}

/// Get the size of a Docker volume in bytes
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Note: Most of these tests require Docker to be running
    // They are integration tests rather than pure unit tests

    #[test]
    #[ignore] // Requires Docker
    fn test_list_volumes_returns_vector() {
        let timeout = Duration::from_secs(10);
        let result = list_volumes(timeout);
        assert!(result.is_ok(), "Should list volumes successfully");
        // Result might be empty, but should be a valid Vec
        let _volumes = result.unwrap();
    }

    #[test]
    #[ignore] // Requires Docker - tests timeout handling
    fn test_volume_exists_with_timeout() {
        let timeout = Duration::from_millis(1); // Very short timeout
        let result = volume_exists("nonexistent-test-volume", timeout);
        // This might fail due to timeout or succeed if Docker is fast enough
        // Either way, it should not panic
        let _ = result;
    }

    #[test]
    #[ignore] // Requires Docker
    fn test_volume_exists_nonexistent_volume() {
        let timeout = Duration::from_secs(10);
        let result = volume_exists("restic-test-nonexistent-volume-12345", timeout);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    #[ignore] // Requires Docker - full integration test
    fn test_archive_and_restore_volume_workflow() {
        let timeout = Duration::from_secs(60);
        let temp_dir = TempDir::new().unwrap();
        let volume_name = "restic-test-volume";
        
        // Create a test volume
        let create_result = std::process::Command::new("docker")
            .args(&["volume", "create", volume_name])
            .output();
        
        if create_result.is_err() {
            println!("Skipping test: Docker not available");
            return;
        }

        // Add some test data to the volume
        let write_result = std::process::Command::new("docker")
            .args(&[
                "run", "--rm",
                "-v", &format!("{}:/data", volume_name),
                "alpine",
                "sh", "-c",
                "echo 'test data' > /data/test.txt && echo 'more data' > /data/test2.txt"
            ])
            .output();
        
        assert!(write_result.is_ok());

        // Archive the volume
        let archive_path = temp_dir.path().join("test-volume.tar.gz");
        let archive_result = archive_volume(volume_name, &archive_path, timeout);
        assert!(archive_result.is_ok(), "Should archive volume successfully");
        assert!(archive_path.exists(), "Archive file should exist");

        // Create a new volume for restoration
        let restore_volume_name = "restic-test-volume-restore";
        let _ = std::process::Command::new("docker")
            .args(&["volume", "create", restore_volume_name])
            .output();

        // Restore to the new volume
        let restore_result = restore_volume(restore_volume_name, &archive_path, timeout);
        assert!(restore_result.is_ok(), "Should restore volume successfully");

        // Verify the restored data
        let verify_result = std::process::Command::new("docker")
            .args(&[
                "run", "--rm",
                "-v", &format!("{}:/data", restore_volume_name),
                "alpine",
                "cat", "/data/test.txt"
            ])
            .output();
        
        if let Ok(output) = verify_result {
            let content = String::from_utf8_lossy(&output.stdout);
            assert!(content.contains("test data"));
        }

        // Cleanup
        let _ = std::process::Command::new("docker")
            .args(&["volume", "rm", volume_name])
            .output();
        let _ = std::process::Command::new("docker")
            .args(&["volume", "rm", restore_volume_name])
            .output();
    }

    #[test]
    #[ignore] // Requires Docker
    fn test_get_volume_size() {
        let timeout = Duration::from_secs(30);
        let volume_name = "restic-test-volume-size";
        
        // Create a test volume
        let create_result = std::process::Command::new("docker")
            .args(&["volume", "create", volume_name])
            .output();
        
        if create_result.is_err() {
            println!("Skipping test: Docker not available");
            return;
        }

        // Add some data
        let _ = std::process::Command::new("docker")
            .args(&[
                "run", "--rm",
                "-v", &format!("{}:/data", volume_name),
                "alpine",
                "sh", "-c",
                "dd if=/dev/zero of=/data/testfile bs=1024 count=100"
            ])
            .output();

        // Get size
        let size_result = get_volume_size(volume_name, timeout);
        assert!(size_result.is_ok(), "Should get volume size successfully");
        let size = size_result.unwrap();
        assert!(size > 0, "Volume size should be greater than 0");
        
        // Cleanup
        let _ = std::process::Command::new("docker")
            .args(&["volume", "rm", volume_name])
            .output();
    }

    #[test]
    #[ignore] // Requires Docker - tests directory creation
    fn test_archive_volume_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested").join("deep").join("test.tar.gz");
        let timeout = Duration::from_secs(30);
        
        // This will fail because volume doesn't exist, but it should create directories first
        let result = archive_volume("nonexistent-volume", &nested_path, timeout);
        
        // Should fail due to Docker error, but parent directories should be created
        assert!(result.is_err());
        assert!(nested_path.parent().unwrap().exists(), "Parent directories should be created");
    }

    #[test]
    fn test_restore_volume_nonexistent_archive() {
        let timeout = Duration::from_secs(10);
        let nonexistent_path = std::path::PathBuf::from("/nonexistent/path/archive.tar.gz");
        
        let result = restore_volume("test-volume", &nonexistent_path, timeout);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    #[ignore] // Requires Docker - tests timeout handling
    fn test_list_volumes_timeout() {
        let timeout = Duration::from_nanos(1); // Impossibly short timeout
        let result = list_volumes(timeout);
        // Should either timeout or succeed very fast
        // We're just checking it handles timeouts gracefully
        let _ = result;
    }
}
