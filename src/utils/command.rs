//! Utilities for running commands with proper error handling and timeouts

use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Duration;
use tracing::{debug, error, info};

/// Run a command with optional timeout
pub fn run_command(
    program: &str,
    args: &[&str],
    working_dir: Option<&Path>,
    timeout: Option<Duration>,
) -> Result<Output> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    debug!("Running command: {} {}", program, args.join(" "));

    let output = if let Some(timeout_duration) = timeout {
        // Use tokio for timeout support
        let handle = tokio::runtime::Handle::current();
        handle.block_on(async {
            let result = tokio::time::timeout(timeout_duration, tokio::process::Command::from(cmd).output())
                .await;

            match result {
                Ok(output) => output.context(format!("Failed to execute {}", program)),
                Err(_) => Err(anyhow::anyhow!("Command timed out after {:?}", timeout_duration)),
            }
        })?
    } else {
        cmd.output()
            .context(format!("Failed to execute {}", program))?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Command failed: {} {}", program, args.join(" "));
        error!("Stderr: {}", stderr);
        anyhow::bail!(
            "Command failed with exit code {:?}: {}",
            output.status.code(),
            stderr
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        debug!("Command output: {}", stdout);
    }

    Ok(output)
}

/// Run a command and return stdout as string
pub fn run_command_stdout(
    program: &str,
    args: &[&str],
    working_dir: Option<&Path>,
    timeout: Option<Duration>,
) -> Result<String> {
    let output = run_command(program, args, working_dir, timeout)?;
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Run a shell command (useful for hooks)
pub fn run_shell_command(
    command: &str,
    working_dir: Option<&Path>,
    timeout: Option<Duration>,
) -> Result<Output> {
    info!("Running shell command: {}", command);

    #[cfg(unix)]
    let (shell, flag) = ("sh", "-c");

    #[cfg(windows)]
    let (shell, flag) = ("cmd", "/C");

    run_command(shell, &[flag, command], working_dir, timeout)
}
