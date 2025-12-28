//! Cron job management utilities

use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;
use std::process::Command;
use tracing::{info, warn};

/// Get the path to the restic-manager binary
pub fn get_binary_path() -> Result<PathBuf> {
    env::current_exe().context("Failed to get current executable path")
}

/// Get the current crontab
pub fn get_crontab() -> Result<String> {
    let output = Command::new("crontab")
        .arg("-l")
        .output()
        .context("Failed to execute crontab -l")?;

    if !output.status.success() {
        // Empty crontab returns non-zero, check stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("no crontab") {
            return Ok(String::new());
        }
        anyhow::bail!("Failed to read crontab: {}", stderr);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Set the crontab content
pub fn set_crontab(content: &str) -> Result<()> {
    use std::io::Write;

    let mut child = Command::new("crontab")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn crontab")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes())
            .context("Failed to write to crontab stdin")?;
    } else {
        anyhow::bail!("Failed to open crontab stdin");
    }

    let output = child.wait_with_output()
        .context("Failed to wait for crontab")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Crontab command failed: {}", stderr);
    }

    info!("Crontab updated successfully");
    Ok(())
}

/// Add a cron job for a service
pub fn add_cron_job(
    service_name: &str,
    schedule: &str,
    config_path: &PathBuf,
    dry_run: bool,
) -> Result<()> {
    let binary_path = get_binary_path()?;
    let log_file = format!("/var/log/restic-manager/{}.log", service_name);

    // Build the cron command
    let cron_command = format!(
        "{} --config {} run --service {} >> {} 2>&1",
        binary_path.display(),
        config_path.display(),
        service_name,
        log_file
    );

    // Build the cron entry
    let cron_entry = format!(
        "# Restic Manager - Service: {}\n{} {}",
        service_name, schedule, cron_command
    );

    if dry_run {
        println!("  [DRY RUN] Would add cron job:");
        println!("    {}", cron_entry.replace('\n', "\n    "));
        return Ok(());
    }

    // Get existing crontab
    let existing = get_crontab()?;

    // Check if job already exists
    let marker = format!("# Restic Manager - Service: {}", service_name);
    if existing.contains(&marker) {
        warn!("Cron job for service '{}' already exists, updating...", service_name);

        // Remove old entry
        let lines: Vec<&str> = existing.lines().collect();
        let mut new_lines = Vec::new();
        let mut skip_next = false;

        for line in lines {
            if line.contains(&marker) {
                skip_next = true;
                continue;
            }
            if skip_next {
                skip_next = false;
                continue;
            }
            new_lines.push(line);
        }

        // Add new entry
        new_lines.push(&cron_entry);

        let new_content = new_lines.join("\n") + "\n";
        set_crontab(&new_content)?;
    } else {
        // Add new entry
        let new_content = if existing.is_empty() {
            cron_entry + "\n"
        } else {
            existing + "\n" + &cron_entry + "\n"
        };

        set_crontab(&new_content)?;
    }

    info!("Added cron job for service: {}", service_name);
    Ok(())
}

/// Remove cron job for a service
pub fn remove_cron_job(service_name: &str) -> Result<()> {
    let existing = get_crontab()?;
    let marker = format!("# Restic Manager - Service: {}", service_name);

    if !existing.contains(&marker) {
        warn!("No cron job found for service '{}'", service_name);
        return Ok(());
    }

    // Remove the entry
    let lines: Vec<&str> = existing.lines().collect();
    let mut new_lines = Vec::new();
    let mut skip_next = false;

    for line in lines {
        if line.contains(&marker) {
            skip_next = true;
            continue;
        }
        if skip_next {
            skip_next = false;
            continue;
        }
        new_lines.push(line);
    }

    let new_content = new_lines.join("\n") + "\n";
    set_crontab(&new_content)?;

    info!("Removed cron job for service: {}", service_name);
    Ok(())
}

/// Validate cron schedule syntax
pub fn validate_cron_schedule(schedule: &str) -> bool {
    // Basic validation: should have 5 fields
    schedule.split_whitespace().count() == 5
}

/// List all restic-manager cron jobs
pub fn list_cron_jobs() -> Result<Vec<String>> {
    let existing = get_crontab()?;
    let mut jobs = Vec::new();

    for line in existing.lines() {
        if line.contains("# Restic Manager - Service:") {
            jobs.push(line.to_string());
        }
    }

    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_cron_schedule() {
        assert!(validate_cron_schedule("0 2 * * *"));
        assert!(validate_cron_schedule("*/5 * * * *"));
        assert!(validate_cron_schedule("0 0 1 * *"));
        assert!(!validate_cron_schedule("invalid"));
        assert!(!validate_cron_schedule("0 2 * *"));
        assert!(!validate_cron_schedule("0 2 * * * *"));
    }
}
